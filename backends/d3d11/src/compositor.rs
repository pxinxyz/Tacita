use crate::{BackendError, D3d11Backend};
use restricted_gui::{
    draw::TextureId,
    effects::{FilterPlan, MAX_FILTER_PASSES},
};
use windows::Win32::Graphics::{Direct3D11::*, Dxgi::Common::*};

pub(crate) fn required<T>(value: Option<T>) -> Result<T, BackendError> {
    value.ok_or_else(|| {
        BackendError::TextureCreation(windows::core::Error::from_hresult(
            windows::Win32::Foundation::E_UNEXPECTED,
        ))
    })
}

struct Target {
    texture: ID3D11Texture2D,
    view: ID3D11RenderTargetView,
    size: [u32; 2],
    format: DXGI_FORMAT,
    samples: u32,
}
impl Target {
    fn new(
        device: &ID3D11Device,
        size: [u32; 2],
        format: DXGI_FORMAT,
    ) -> Result<Self, BackendError> {
        Self::new_samples(device, size, format, 1)
    }
    fn new_samples(
        device: &ID3D11Device,
        size: [u32; 2],
        format: DXGI_FORMAT,
        samples: u32,
    ) -> Result<Self, BackendError> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: size[0],
            Height: size[1],
            MipLevels: 1,
            ArraySize: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: samples,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_RENDER_TARGET.0
                | if samples == 1 {
                    D3D11_BIND_SHADER_RESOURCE.0
                } else {
                    0
                }) as u32,
            ..Default::default()
        };
        let mut texture = None;
        unsafe { device.CreateTexture2D(&desc, None, Some(&mut texture)) }
            .map_err(BackendError::TextureCreation)?;
        let texture = required(texture)?;
        let mut view = None;
        unsafe { device.CreateRenderTargetView(&texture, None, Some(&mut view)) }
            .map_err(BackendError::RenderTargetViewCreation)?;
        Ok(Self {
            texture,
            view: required(view)?,
            size,
            format,
            samples,
        })
    }
}

pub struct BackdropCapture {
    id: TextureId,
    target: Option<Target>,
}
impl BackdropCapture {
    pub fn new(id: TextureId) -> Result<Self, BackendError> {
        if id.0 < 2 {
            return Err(BackendError::ReservedTextureId(id.0));
        }
        Ok(Self { id, target: None })
    }
    pub fn capture(
        &mut self,
        backend: &mut D3d11Backend,
        source: &ID3D11Texture2D,
    ) -> Result<TextureId, BackendError> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe {
            source.GetDesc(&mut desc);
        }
        if desc.SampleDesc.Count != 1
            || desc.ArraySize != 1
            || desc.MipLevels != 1
            || !matches!(
                desc.Format,
                DXGI_FORMAT_R8G8B8A8_UNORM | DXGI_FORMAT_B8G8R8A8_UNORM
            )
        {
            return Err(BackendError::InvalidTextureData(
                "capture requires single-sample RGBA8/BGRA8 UNORM".into(),
            ));
        }
        let owner = unsafe { source.GetDevice() }.map_err(BackendError::TextureCreation)?;
        if owner != backend.device || self.target.as_ref().is_some_and(|t| t.texture == *source) {
            return Err(BackendError::InvalidTextureData(
                "capture device mismatch or source/output alias".into(),
            ));
        }
        let size = [desc.Width, desc.Height];
        if self
            .target
            .as_ref()
            .is_none_or(|t| t.size != size || t.format != desc.Format)
        {
            let target = Target::new(&backend.device, size, desc.Format)?;
            backend.add_gpu_texture(self.id, &target.texture)?;
            self.target = Some(target);
        }
        if let Some(target) = &self.target {
            unsafe {
                backend
                    .context
                    .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
                backend.context.OMSetRenderTargets(None, None);
                backend.context.CopyResource(&target.texture, source);
            }
        }
        Ok(self.id)
    }
}

pub struct PostProcessor {
    ids: [TextureId; MAX_FILTER_PASSES],
    targets: Vec<Option<Target>>,
}
impl PostProcessor {
    pub fn new(first: TextureId) -> Result<Self, BackendError> {
        if first.0 < 2 || first.0.checked_add(MAX_FILTER_PASSES as u64 - 1).is_none() {
            return Err(BackendError::InvalidTextureData(
                "invalid filter texture ID range".into(),
            ));
        }
        Ok(Self {
            ids: core::array::from_fn(|i| TextureId(first.0 + i as u64)),
            targets: (0..MAX_FILTER_PASSES).map(|_| None).collect(),
        })
    }
    pub fn process(
        &mut self,
        backend: &mut D3d11Backend,
        source: TextureId,
        plan: &FilterPlan,
    ) -> Result<TextureId, BackendError> {
        if self.ids.contains(&source)
            || backend.textures.size_for(source) != Some(plan.input_size())
        {
            return Err(BackendError::InvalidTextureData(
                "filter source dimensions/ID do not match the plan".into(),
            ));
        }
        for (i, step) in plan.steps().iter().enumerate() {
            if self.targets[i].as_ref().is_none_or(|t| t.size != step.size) {
                let target = Target::new(&backend.device, step.size, DXGI_FORMAT_R8G8B8A8_UNORM)?;
                backend.add_gpu_texture(self.ids[i], &target.texture)?;
                self.targets[i] = Some(target);
            }
            let packet = plan
                .packet(i, source, &self.ids)
                .map_err(|e| BackendError::InvalidTextureData(format!("{e:?}")))?;
            if let Some(target) = &self.targets[i] {
                unsafe {
                    backend
                        .context
                        .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
                }
                let previous_scale = backend.scale_factor;
                let previous_mode = backend.color_mode;
                let previous_alpha = backend.alpha_mode();
                backend.scale_factor = 1.;
                backend.color_mode = crate::ColorMode::Legacy;
                backend.set_alpha_mode(crate::AlphaMode::Straight);
                let result = backend.render(
                    &packet.draw_data(),
                    &target.view,
                    [step.size[0] as f32, step.size[1] as f32],
                );
                backend.scale_factor = previous_scale;
                backend.color_mode = previous_mode;
                backend.set_alpha_mode(previous_alpha);
                result?;
                backend.set_texture_alpha_mode(self.ids[i], crate::AlphaMode::Straight)?;
            }
        }
        Ok(self.ids[plan.steps().len() - 1])
    }
}

pub(crate) struct Stencil {
    pub test: ID3D11DepthStencilState,
    pub push: ID3D11DepthStencilState,
    pub pop: ID3D11DepthStencilState,
    pub overwrite: ID3D11BlendState,
    pub stamp: ID3D11BlendState,
    target: Option<([u32; 2], u32, ID3D11DepthStencilView)>,
}
impl Stencil {
    pub fn new(device: &ID3D11Device) -> Result<Self, BackendError> {
        let state = |operation| {
            let face = D3D11_DEPTH_STENCILOP_DESC {
                StencilFailOp: D3D11_STENCIL_OP_KEEP,
                StencilDepthFailOp: D3D11_STENCIL_OP_KEEP,
                StencilPassOp: operation,
                StencilFunc: D3D11_COMPARISON_EQUAL,
            };
            let desc = D3D11_DEPTH_STENCIL_DESC {
                DepthEnable: false.into(),
                DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ZERO,
                DepthFunc: D3D11_COMPARISON_ALWAYS,
                StencilEnable: true.into(),
                StencilReadMask: 255,
                StencilWriteMask: if operation == D3D11_STENCIL_OP_KEEP {
                    0
                } else {
                    255
                },
                FrontFace: face,
                BackFace: face,
            };
            let mut value = None;
            unsafe { device.CreateDepthStencilState(&desc, Some(&mut value)) }
                .map_err(|e| BackendError::StateCreation("stencil", e))?;
            required(value)
        };
        let blend = |mask| {
            let mut desc = D3D11_BLEND_DESC::default();
            desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: false.into(),
                SrcBlend: D3D11_BLEND_ONE,
                DestBlend: D3D11_BLEND_ZERO,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_ZERO,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: mask,
            };
            let mut value = None;
            unsafe { device.CreateBlendState(&desc, Some(&mut value)) }
                .map_err(|e| BackendError::StateCreation("overwrite blend", e))?;
            required(value)
        };
        Ok(Self {
            test: state(D3D11_STENCIL_OP_KEEP)?,
            push: state(D3D11_STENCIL_OP_INCR_SAT)?,
            pop: state(D3D11_STENCIL_OP_DECR_SAT)?,
            overwrite: blend(15)?,
            stamp: blend(0)?,
            target: None,
        })
    }
    pub fn view(
        &mut self,
        device: &ID3D11Device,
        size: [u32; 2],
        samples: u32,
    ) -> Result<ID3D11DepthStencilView, BackendError> {
        if self
            .target
            .as_ref()
            .is_none_or(|(old, count, _)| *old != size || *count != samples)
        {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: size[0],
                Height: size[1],
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_D24_UNORM_S8_UINT,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: samples,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_DEPTH_STENCIL.0 as u32,
                ..Default::default()
            };
            let mut texture = None;
            unsafe { device.CreateTexture2D(&desc, None, Some(&mut texture)) }
                .map_err(BackendError::TextureCreation)?;
            let texture = required(texture)?;
            let mut view = None;
            unsafe { device.CreateDepthStencilView(&texture, None, Some(&mut view)) }
                .map_err(|e| BackendError::StateCreation("stencil view", e))?;
            self.target = Some((size, samples, required(view)?));
        }
        self.target
            .as_ref()
            .map(|(_, _, v)| v.clone())
            .ok_or_else(|| BackendError::InvalidTextureData("missing stencil target".into()))
    }
}

pub struct RenderTexture {
    id: TextureId,
    target: Target,
    multisampled: Option<Target>,
}
impl RenderTexture {
    pub fn new(
        backend: &mut D3d11Backend,
        id: TextureId,
        size: [u32; 2],
    ) -> Result<Self, BackendError> {
        Self::new_encoded(backend, id, size, crate::ColorMode::Legacy)
    }
    pub fn new_encoded(
        backend: &mut D3d11Backend,
        id: TextureId,
        size: [u32; 2],
        mode: crate::ColorMode,
    ) -> Result<Self, BackendError> {
        Self::new_multisampled(backend, id, size, mode, 1)
    }
    pub fn new_multisampled(
        backend: &mut D3d11Backend,
        id: TextureId,
        size: [u32; 2],
        mode: crate::ColorMode,
        samples: u32,
    ) -> Result<Self, BackendError> {
        if !restricted_gui::rendering::valid_sample_count(samples) {
            return Err(BackendError::InvalidTextureData(
                "sample count must be 1, 2, 4 or 8".into(),
            ));
        }
        if id.0 < 2 {
            return Err(BackendError::ReservedTextureId(id.0));
        }
        if size.contains(&0) || size.iter().any(|&v| v > 16384) {
            return Err(BackendError::InvalidTextureData(
                "invalid render target dimensions".into(),
            ));
        }
        let format = if mode == crate::ColorMode::Srgb {
            DXGI_FORMAT_R8G8B8A8_UNORM_SRGB
        } else {
            DXGI_FORMAT_R8G8B8A8_UNORM
        };
        if samples > 1
            && (unsafe {
                backend
                    .device
                    .CheckMultisampleQualityLevels(format, samples)
            }
            .map_err(BackendError::TextureCreation)?
                == 0
                || unsafe {
                    backend
                        .device
                        .CheckMultisampleQualityLevels(DXGI_FORMAT_D24_UNORM_S8_UINT, samples)
                }
                .map_err(BackendError::TextureCreation)?
                    == 0)
        {
            return Err(BackendError::InvalidTextureData(
                "unsupported color/stencil sample count".into(),
            ));
        }
        let target = Target::new(&backend.device, size, format)?;
        let multisampled = if samples > 1 {
            Some(Target::new_samples(&backend.device, size, format, samples)?)
        } else {
            None
        };
        backend.add_gpu_texture(id, &target.texture)?;
        Ok(Self {
            id,
            target,
            multisampled,
        })
    }
    pub fn id(&self) -> TextureId {
        self.id
    }
    pub fn size(&self) -> [u32; 2] {
        self.target.size
    }
    pub fn texture(&self) -> &ID3D11Texture2D {
        &self.target.texture
    }
    pub fn sample_count(&self) -> u32 {
        self.multisampled.as_ref().map_or(1, |t| t.samples)
    }
    pub fn render(
        &self,
        backend: &mut D3d11Backend,
        draw: &restricted_gui::draw::DrawData,
        clear: [f32; 4],
    ) -> Result<(), BackendError> {
        let aliases = |id: TextureId| {
            id == self.id || backend.textures.resource_for(id) == Some(&self.target.texture)
        };
        if !clear.iter().all(|v| v.is_finite())
            || draw.commands.iter().any(|c| aliases(c.texture))
            || draw
                .effects
                .iter()
                .any(|e| (0..4).any(|i| e.mask_texture(i).is_some_and(aliases)))
        {
            return Err(BackendError::InvalidTextureData(
                "invalid clear or render target read/write alias".into(),
            ));
        }
        let owner =
            unsafe { self.target.texture.GetDevice() }.map_err(BackendError::TextureCreation)?;
        if owner != backend.device {
            return Err(BackendError::InvalidTextureData(
                "render target device mismatch".into(),
            ));
        }
        if backend.textures.resource_for(self.id) != Some(&self.target.texture) {
            backend.add_gpu_texture(self.id, &self.target.texture)?;
        }
        let clear = backend.alpha_mode().clear_color(clear);
        unsafe {
            backend
                .context
                .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
            backend.context.ClearRenderTargetView(
                &self.multisampled.as_ref().unwrap_or(&self.target).view,
                &clear,
            );
        }
        let previous = backend.color_mode;
        backend.color_mode = if self.target.format == DXGI_FORMAT_R8G8B8A8_UNORM_SRGB {
            crate::ColorMode::Srgb
        } else {
            crate::ColorMode::Legacy
        };
        let result = backend.render(
            draw,
            &self.multisampled.as_ref().unwrap_or(&self.target).view,
            self.target.size.map(|v| v as f32),
        );
        backend.color_mode = previous;
        unsafe {
            backend
                .context
                .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
            backend.context.OMSetRenderTargets(None, None);
        }
        if result.is_ok() {
            if let Some(msaa) = &self.multisampled {
                unsafe {
                    backend.context.ResolveSubresource(
                        &self.target.texture,
                        0,
                        &msaa.texture,
                        0,
                        self.target.format,
                    );
                }
            }
            backend.set_texture_alpha_mode(self.id, backend.alpha_mode())?;
        }
        result
    }
}
