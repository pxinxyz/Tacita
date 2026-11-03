use crate::{expect_resource, BackendError, D3d12Backend, Device};
use tacita::{
    draw::TextureId,
    effects::{FilterPlan, MAX_FILTER_PASSES},
};
use windows::Win32::Graphics::{Direct3D12::*, Dxgi::Common::*};

pub(crate) struct Target {
    pub resource: ID3D12Resource,
    pub handle: D3D12_CPU_DESCRIPTOR_HANDLE,
    _heap: ID3D12DescriptorHeap,
    pub size: [u32; 2],
    pub format: DXGI_FORMAT,
    pub samples: u32,
}
impl Target {
    pub fn new(
        device: &Device,
        size: [u32; 2],
        format: DXGI_FORMAT,
        stencil: bool,
    ) -> Result<Self, BackendError> {
        Self::new_samples(device, size, format, stencil, 1)
    }
    pub fn new_samples(
        device: &Device,
        size: [u32; 2],
        format: DXGI_FORMAT,
        stencil: bool,
        samples: u32,
    ) -> Result<Self, BackendError> {
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Width: size[0] as u64,
            Height: size[1],
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: samples,
                Quality: 0,
            },
            Flags: if stencil {
                D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL
            } else {
                D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET
            },
            ..Default::default()
        };
        let heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_DEFAULT,
            ..Default::default()
        };
        let clear = D3D12_CLEAR_VALUE {
            Format: format,
            Anonymous: if stencil {
                D3D12_CLEAR_VALUE_0 {
                    DepthStencil: D3D12_DEPTH_STENCIL_VALUE {
                        Depth: 1.,
                        Stencil: 0,
                    },
                }
            } else {
                D3D12_CLEAR_VALUE_0 { Color: [0.; 4] }
            },
        };
        let mut resource = None;
        unsafe {
            device.CreateCommittedResource(
                &heap,
                D3D12_HEAP_FLAG_NONE,
                &desc,
                if stencil {
                    D3D12_RESOURCE_STATE_DEPTH_WRITE
                } else {
                    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE
                },
                Some(&clear),
                &mut resource,
            )
        }
        .map_err(BackendError::TextureCreation)?;
        let resource = expect_resource(resource, BackendError::TextureCreation)?;
        let heap: ID3D12DescriptorHeap = unsafe {
            device.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                Type: if stencil {
                    D3D12_DESCRIPTOR_HEAP_TYPE_DSV
                } else {
                    D3D12_DESCRIPTOR_HEAP_TYPE_RTV
                },
                NumDescriptors: 1,
                ..Default::default()
            })
        }
        .map_err(BackendError::DescriptorHeapCreation)?;
        let handle = unsafe { heap.GetCPUDescriptorHandleForHeapStart() };
        unsafe {
            if stencil {
                device.CreateDepthStencilView(&resource, None, handle)
            } else {
                device.CreateRenderTargetView(&resource, None, handle)
            }
        }
        Ok(Self {
            resource,
            handle,
            _heap: heap,
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
        backend: &mut D3d12Backend,
        source: &ID3D12Resource,
    ) -> Result<TextureId, BackendError> {
        let desc = unsafe { source.GetDesc() };
        if desc.Flags.0 & D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET.0 == 0
            || desc.Dimension != D3D12_RESOURCE_DIMENSION_TEXTURE2D
            || desc.SampleDesc.Count != 1
            || desc.DepthOrArraySize != 1
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
        let mut owner: Option<Device> = None;
        unsafe { source.GetDevice(&mut owner) }.map_err(BackendError::TextureCreation)?;
        if owner.as_ref() != Some(&backend.device) {
            return Err(BackendError::InvalidTextureData(
                "capture device mismatch".into(),
            ));
        }
        let size = [desc.Width as u32, desc.Height];
        if self
            .target
            .as_ref()
            .is_none_or(|t| t.size != size || t.format != desc.Format)
        {
            let target = Target::new(&backend.device, size, desc.Format, false)?;
            backend.add_gpu_texture(self.id, &target.resource)?;
            self.target = Some(target);
        }
        if let Some(target) = &self.target {
            backend
                .pipeline
                .capture(&mut backend.sync, source, &target.resource)?;
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
        backend: &mut D3d12Backend,
        source: TextureId,
        plan: &FilterPlan,
    ) -> Result<TextureId, BackendError> {
        if self.ids.contains(&source)
            || backend.pipeline.texture_size(source) != Some(plan.input_size())
        {
            return Err(BackendError::InvalidTextureData(
                "filter source dimensions/ID do not match the plan".into(),
            ));
        }
        for (i, step) in plan.steps().iter().enumerate() {
            if self.targets[i].as_ref().is_none_or(|t| t.size != step.size) {
                let target = Target::new(
                    &backend.device,
                    step.size,
                    DXGI_FORMAT_R8G8B8A8_UNORM,
                    false,
                )?;
                backend.add_gpu_texture(self.ids[i], &target.resource)?;
                self.targets[i] = Some(target);
            }
            let packet = plan
                .packet(i, source, &self.ids)
                .map_err(|e| BackendError::InvalidTextureData(format!("{e:?}")))?;
            if let Some(target) = &self.targets[i] {
                let previous_alpha = backend.alpha_mode();
                backend.set_alpha_mode(crate::AlphaMode::Straight);
                let result = backend.pipeline.render(
                    &mut backend.sync,
                    &packet.draw_data(),
                    target.handle,
                    [step.size[0] as f32, step.size[1] as f32],
                    crate::pipeline::RenderOptions {
                        sampled: Some(&target.resource),
                        ..Default::default()
                    },
                );
                backend.set_alpha_mode(previous_alpha);
                result?;
                backend.set_texture_alpha_mode(self.ids[i], crate::AlphaMode::Straight)?;
            }
        }
        Ok(self.ids[plan.steps().len() - 1])
    }
}

pub struct RenderTexture {
    id: TextureId,
    target: Target,
    multisampled: Option<Target>,
}
impl RenderTexture {
    pub fn new(
        backend: &mut D3d12Backend,
        id: TextureId,
        size: [u32; 2],
    ) -> Result<Self, BackendError> {
        Self::new_encoded(backend, id, size, crate::ColorMode::Legacy)
    }
    pub fn new_encoded(
        backend: &mut D3d12Backend,
        id: TextureId,
        size: [u32; 2],
        mode: crate::ColorMode,
    ) -> Result<Self, BackendError> {
        Self::new_multisampled(backend, id, size, mode, 1)
    }
    pub fn new_multisampled(
        backend: &mut D3d12Backend,
        id: TextureId,
        size: [u32; 2],
        mode: crate::ColorMode,
        samples: u32,
    ) -> Result<Self, BackendError> {
        if id.0 < 2 {
            return Err(BackendError::ReservedTextureId(id.0));
        }
        if size.contains(&0) || size.iter().any(|&v| v > 16384) {
            return Err(BackendError::InvalidTextureData(
                "invalid render target dimensions".into(),
            ));
        }
        backend.pipeline.ensure_raster(samples)?;
        let format = if mode == crate::ColorMode::Srgb {
            DXGI_FORMAT_R8G8B8A8_UNORM_SRGB
        } else {
            DXGI_FORMAT_R8G8B8A8_UNORM
        };
        let target = Target::new(&backend.device, size, format, false)?;
        let multisampled = if samples > 1 {
            Some(Target::new_samples(
                &backend.device,
                size,
                format,
                false,
                samples,
            )?)
        } else {
            None
        };
        backend.add_gpu_texture(id, &target.resource)?;
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
    pub fn resource(&self) -> &ID3D12Resource {
        &self.target.resource
    }
    pub fn sample_count(&self) -> u32 {
        self.multisampled.as_ref().map_or(1, |t| t.samples)
    }
    pub fn render(
        &self,
        backend: &mut D3d12Backend,
        draw: &tacita::draw::DrawData,
        clear: [f32; 4],
    ) -> Result<(), BackendError> {
        let aliases =
            |id: TextureId| id == self.id || backend.pipeline.aliases(id, &self.target.resource);
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
        let mut owner: Option<Device> = None;
        unsafe { self.target.resource.GetDevice(&mut owner) }
            .map_err(BackendError::TextureCreation)?;
        if owner.as_ref() != Some(&backend.device) {
            return Err(BackendError::InvalidTextureData(
                "render target device mismatch".into(),
            ));
        }
        if !backend.pipeline.aliases(self.id, &self.target.resource) {
            backend.add_gpu_texture(self.id, &self.target.resource)?;
        }
        let clear = backend.alpha_mode().clear_color(clear);
        backend.pipeline.render(
            &mut backend.sync,
            draw,
            self.multisampled.as_ref().unwrap_or(&self.target).handle,
            self.target.size.map(|v| v as f32),
            crate::pipeline::RenderOptions {
                sampled: Some(&self.multisampled.as_ref().unwrap_or(&self.target).resource),
                clear: Some(clear),
                scale_factor: backend.scale_factor,
                sample_count: self.multisampled.as_ref().map_or(1, |t| t.samples),
                resolve: self
                    .multisampled
                    .as_ref()
                    .map(|_| (&self.target.resource, self.target.format)),
                color_mode: if self.target.format == DXGI_FORMAT_R8G8B8A8_UNORM_SRGB {
                    crate::ColorMode::Srgb
                } else {
                    crate::ColorMode::Legacy
                },
            },
        )?;
        backend.set_texture_alpha_mode(self.id, backend.alpha_mode())?;
        Ok(())
    }
}
