mod compositor;
mod offscreen;
mod rendering_options;
mod timing;
pub use compositor::{BackdropCapture, PostProcessor, RenderTexture};
pub use tacita::rendering::{
    AlphaMode, ColorMode, DrawOptimization, RenderStats, TextureEncoding, UploadStrategy,
};
mod shader;
mod texture;

use std::fmt;

use tacita::draw::{DrawData, DrawVert, TextureId};
use texture::TextureSet;
use windows::core::PCSTR;
use windows::Win32::Foundation::{E_UNEXPECTED, HMODULE, RECT};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
    D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11BlendState, ID3D11Buffer, ID3D11DepthStencilState, ID3D11InputLayout,
    ID3D11PixelShader, ID3D11RasterizerState, ID3D11RenderTargetView, ID3D11SamplerState,
    ID3D11VertexShader, D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_INDEX_BUFFER,
    D3D11_BIND_VERTEX_BUFFER, D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE,
    D3D11_BLEND_OP_ADD, D3D11_BLEND_SRC_ALPHA, D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL,
    D3D11_COMPARISON_ALWAYS, D3D11_CPU_ACCESS_WRITE, D3D11_CREATE_DEVICE_FLAG, D3D11_CULL_NONE,
    D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ALL, D3D11_FILL_SOLID,
    D3D11_FILTER_MIN_MAG_MIP_POINT, D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_WRITE_DISCARD, D3D11_RASTERIZER_DESC,
    D3D11_RENDER_TARGET_BLEND_DESC, D3D11_SAMPLER_DESC, D3D11_SDK_VERSION,
    D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_USAGE_DYNAMIC, D3D11_VIEWPORT,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32_UINT, DXGI_FORMAT_R8G8B8A8_UNORM,
};

pub use offscreen::OffscreenTarget;
pub use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device as Device, ID3D11DeviceContext as Context,
};

const FEATURE_LEVELS: [D3D_FEATURE_LEVEL; 2] = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];

const VERTEX_STRIDE: u32 = 20;

const INITIAL_VERTEX_CAPACITY: u32 = 4096;

const INITIAL_INDEX_CAPACITY: u32 = 8192;

const INPUT_LAYOUT: [D3D11_INPUT_ELEMENT_DESC; 3] = [
    D3D11_INPUT_ELEMENT_DESC {
        SemanticName: PCSTR::from_raw(c"POSITION".as_ptr().cast()),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: 0,
        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
    D3D11_INPUT_ELEMENT_DESC {
        SemanticName: PCSTR::from_raw(c"TEXCOORD".as_ptr().cast()),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: 8,
        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
    D3D11_INPUT_ELEMENT_DESC {
        SemanticName: PCSTR::from_raw(c"COLOR".as_ptr().cast()),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        InputSlot: 0,
        AlignedByteOffset: 16,
        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
];

#[derive(Debug)]
pub enum BackendError {
    DeviceCreation(windows::core::Error),
    ShaderCompile(String),
    TextureCreation(windows::core::Error),
    RenderTargetViewCreation(windows::core::Error),
    BufferCreation(windows::core::Error),
    StateCreation(&'static str, windows::core::Error),
    MapFailed(windows::core::Error),
    Readback(String),
    ReservedTextureId(u64),
    InvalidTextureData(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::DeviceCreation(e) => write!(f, "D3D11 device creation failed: {e}"),
            BackendError::ShaderCompile(msg) => write!(f, "shader compilation failed: {msg}"),
            BackendError::TextureCreation(e) => write!(f, "texture creation failed: {e}"),
            BackendError::RenderTargetViewCreation(e) => {
                write!(f, "render target view creation failed: {e}")
            }
            BackendError::BufferCreation(e) => write!(f, "buffer creation failed: {e}"),
            BackendError::StateCreation(what, e) => write!(f, "{what} creation failed: {e}"),
            BackendError::MapFailed(e) => write!(f, "map failed: {e}"),
            BackendError::Readback(msg) => write!(f, "readback failed: {msg}"),
            BackendError::ReservedTextureId(id) => write!(
                f,
                "texture id {id} is reserved for the built-in atlas (FONT = 0, WHITE = 1)"
            ),
            BackendError::InvalidTextureData(msg) => write!(f, "invalid texture data: {msg}"),
        }
    }
}

impl std::error::Error for BackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BackendError::DeviceCreation(e)
            | BackendError::TextureCreation(e)
            | BackendError::RenderTargetViewCreation(e)
            | BackendError::BufferCreation(e)
            | BackendError::StateCreation(_, e)
            | BackendError::MapFailed(e) => Some(e),
            BackendError::ShaderCompile(_)
            | BackendError::Readback(_)
            | BackendError::ReservedTextureId(_)
            | BackendError::InvalidTextureData(_) => None,
        }
    }
}

struct Pipeline {
    quad_vertex: ID3D11VertexShader,
    managed_quad_vertex: ID3D11VertexShader,
    quad_layout: ID3D11InputLayout,
    quad_buffer: ID3D11Buffer,
    quad_capacity: u32,
    premult_blend: ID3D11BlendState,
    vertex_shader: ID3D11VertexShader,
    pixel_shader: ID3D11PixelShader,
    managed_vertex: ID3D11VertexShader,
    managed_pixel: ID3D11PixelShader,
    input_layout: ID3D11InputLayout,
    constant_buffer: ID3D11Buffer,
    effects_buffer: ID3D11Buffer,
    stencil: compositor::Stencil,
    blend_state: ID3D11BlendState,
    rasterizer_state: ID3D11RasterizerState,
    depth_stencil_state: ID3D11DepthStencilState,
    sampler: ID3D11SamplerState,
    vertex_buffer: ID3D11Buffer,
    vertex_capacity: u32,
    index_buffer: ID3D11Buffer,
    index_capacity: u32,
}

pub struct D3d11Backend {
    optimization: DrawOptimization,
    plan: tacita::rendering::CommandPlan,
    alpha_mode: AlphaMode,
    texture_alpha: Vec<(TextureId, AlphaMode)>,
    color_mode: ColorMode,
    scale_factor: f32,
    timer: timing::GpuTimer,
    vertex_pool: Vec<(u32, ID3D11Buffer)>,
    index_pool: Vec<(u32, ID3D11Buffer)>,
    stats: RenderStats,
    device: Device,
    context: Context,
    pipeline: Pipeline,
    textures: TextureSet,
}

impl D3d11Backend {
    pub fn new(device: &Device, context: &Context) -> Result<Self, BackendError> {
        Ok(Self {
            device: device.clone(),
            context: context.clone(),
            color_mode: ColorMode::Legacy,
            optimization: DrawOptimization::default(),
            plan: Default::default(),
            alpha_mode: AlphaMode::Straight,
            texture_alpha: Vec::new(),
            scale_factor: 1.,
            timer: timing::GpuTimer::new(device)?,
            vertex_pool: Vec::new(),
            index_pool: Vec::new(),
            stats: RenderStats {
                allocations: 3,
                ..Default::default()
            },
            pipeline: create_pipeline(
                device,
                shader::VERTEX_SHADER_SOURCE,
                shader::PIXEL_SHADER_SOURCE,
            )?,
            textures: TextureSet::new(device)?,
        })
    }

    pub fn create_device() -> Result<(Self, Device, Context), BackendError> {
        let (device, context) = match create_device_with(&FEATURE_LEVELS) {
            Ok(pair) => pair,
            Err(_) => create_device_with(&FEATURE_LEVELS[1..])?,
        };
        let backend = Self::new(&device, &context)?;
        Ok((backend, device, context))
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn upload_atlas(
        &mut self,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), BackendError> {
        self.textures
            .upload_atlas(&self.device, &self.context, pixels, width, height)
    }

    pub fn add_host_texture(
        &mut self,
        id: TextureId,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), BackendError> {
        self.textures
            .add_host_texture(&self.device, id, rgba, width, height)?;
        self.texture_alpha.retain(|(key, _)| *key != id);
        Ok(())
    }

    pub fn add_gpu_texture(
        &mut self,
        id: TextureId,
        texture: &windows::Win32::Graphics::Direct3D11::ID3D11Texture2D,
    ) -> Result<(), BackendError> {
        self.textures.add_gpu_texture(&self.device, id, texture)?;
        self.texture_alpha.retain(|(key, _)| *key != id);
        Ok(())
    }

    pub fn set_scale_factor(&mut self, scale: f32) -> Result<(), BackendError> {
        if !scale.is_finite() || scale <= 0. || scale > 8. {
            return Err(BackendError::InvalidTextureData(
                "invalid pixel density".into(),
            ));
        }
        self.scale_factor = scale;
        Ok(())
    }
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub fn set_color_mode(&mut self, mode: ColorMode) {
        self.color_mode = mode;
    }
    pub fn color_mode(&self) -> ColorMode {
        self.color_mode
    }
    pub fn upload_color_font_atlas(
        &mut self,
        font: &tacita::font::Font,
    ) -> Result<(), BackendError> {
        let Some(id) = font.color_atlas_texture() else {
            return Ok(());
        };
        let (rgba, w, h) = font.color_atlas_pixels();
        if rgba.is_empty() {
            return Ok(());
        }
        let encoding = if self.color_mode == ColorMode::Srgb {
            TextureEncoding::Srgb
        } else {
            TextureEncoding::Linear
        };
        self.add_host_texture_encoded(id, rgba, w, h, encoding)
    }
    pub fn add_host_texture_encoded(
        &mut self,
        id: TextureId,
        rgba: &[u8],
        width: u32,
        height: u32,
        encoding: TextureEncoding,
    ) -> Result<(), BackendError> {
        self.textures
            .add_host_texture_encoded(&self.device, id, rgba, width, height, encoding)?;
        self.texture_alpha.retain(|(key, _)| *key != id);
        Ok(())
    }

    pub fn render(
        &mut self,
        draw_data: &DrawData,
        target: &ID3D11RenderTargetView,
        viewport_size: [f32; 2],
    ) -> Result<(), BackendError> {
        let [width, height] = viewport_size;
        if width <= 0.0 || height <= 0.0 || draw_data.vertices.is_empty() {
            return Ok(());
        }

        if self.optimization.enabled() {
            self.plan
                .prepare(draw_data, self.optimization, self.scale_factor)
                .map_err(|e| BackendError::InvalidTextureData(format!("draw plan: {e:?}")))?;
        }
        let triangles = !self.optimization.enabled()
            || self.plan.draws().iter().any(|d| {
                matches!(
                    d.geometry,
                    tacita::rendering::GeometryRange::Triangles { .. }
                )
            });
        self.ensure_buffer_capacity(
            draw_data.vertices.len() as u32,
            draw_data.indices.len() as u32,
        )?;
        if triangles {
            self.upload_geometry(draw_data)?;
        }
        if self.optimization.enabled() {
            self.upload_instances()?;
        }
        self.write_projection(width / self.scale_factor, height / self.scale_factor)?;
        self.bind_pipeline(target, width, height);

        self.stats.uploaded_bytes = (if triangles {
            draw_data.vertices.len() * 20 + draw_data.indices.len() * 4
        } else {
            0
        }) as u64
            + if self.optimization.enabled() {
                self.plan.instances().len() as u64 * 80
            } else {
                0
            };
        self.stats.draw_calls = 0;
        self.timer.begin(&self.context);
        let result = (|| {
            let stencil_active = draw_data.effects.iter().any(|e| e.stencil != [0; 2]);
            if stencil_active {
                let view = self.pipeline.stencil.view(
                    &self.device,
                    [width.ceil() as u32, height.ceil() as u32],
                    rendering_options::target_samples(target)?,
                )?;
                unsafe {
                    self.context.ClearDepthStencilView(
                        &view,
                        windows::Win32::Graphics::Direct3D11::D3D11_CLEAR_STENCIL.0,
                        1.,
                        0,
                    );
                    self.context
                        .OMSetRenderTargets(Some(&[Some(target.clone())]), &view);
                }
            }
            let mut bound_effects = None;
            let mut bound_mode = None;
            'commands: for (cmd, geometry) in tacita::rendering::DrawCalls::new(
                draw_data,
                self.optimization.enabled().then_some(&self.plan),
            ) {
                if cmd.elem_count == 0 {
                    continue;
                }
                let Some(scissor) =
                    scissor_rect(cmd.clip_rect.map(|v| v * self.scale_factor), width, height)
                else {
                    continue;
                };
                let Some(srv) = self.textures.srv_for(cmd.texture).cloned() else {
                    eprintln!(
                        "tacita-d3d11: skipping draw command: host texture id {} is not \
                     registered (register it with `add_host_texture`; backend contract §7)",
                        cmd.texture.0
                    );
                    continue;
                };
                let effects = if cmd.effects == 0 {
                    tacita::effects::DrawEffects::default()
                } else if let Some(value) = draw_data.effects.get(cmd.effects as usize - 1) {
                    *value
                } else {
                    eprintln!("[d3d11] skipping invalid effect index {}", cmd.effects);
                    continue;
                };
                if effects.stencil[0] > 8 || effects.stencil[1] > 2 {
                    continue;
                }
                let mode = [
                    effects.stencil[0],
                    effects.stencil[1],
                    effects.filter.overwrite,
                ];
                if bound_mode != Some(mode) {
                    unsafe {
                        let state = match effects.stencil[1] {
                            1 => &self.pipeline.stencil.push,
                            2 => &self.pipeline.stencil.pop,
                            _ => &self.pipeline.stencil.test,
                        };
                        self.context.OMSetDepthStencilState(
                            if stencil_active {
                                state
                            } else {
                                &self.pipeline.depth_stencil_state
                            },
                            effects.stencil[0],
                        );
                        let blend = if effects.stencil[1] != 0 {
                            &self.pipeline.stencil.stamp
                        } else if effects.filter.overwrite != 0 {
                            &self.pipeline.stencil.overwrite
                        } else {
                            if self.alpha_mode == AlphaMode::Premultiplied {
                                &self.pipeline.premult_blend
                            } else {
                                &self.pipeline.blend_state
                            }
                        };
                        self.context
                            .OMSetBlendState(blend, Some(&[0.; 4]), u32::MAX);
                    }
                    bound_mode = Some(mode);
                }
                let flags = tacita::rendering::alpha_flags(
                    self.texture_alpha_mode(cmd.texture),
                    self.alpha_mode,
                    self.texture_alpha_mode(effects.mask_texture(3).unwrap_or(TextureId::WHITE)),
                );
                if bound_effects != Some((cmd.effects, flags)) {
                    let mut masks = core::array::from_fn::<_, 4, _>(|_| None);
                    for (i, slot) in masks.iter_mut().enumerate() {
                        let id = effects.mask_texture(i).unwrap_or(TextureId::WHITE);
                        let Some(view) = self.textures.srv_for(id).cloned() else {
                            eprintln!("[d3d11] skipping unregistered mask texture {id:?}");
                            continue 'commands;
                        };
                        *slot = Some(view);
                    }
                    self.write_effects(effects, flags)?;
                    unsafe {
                        self.context.PSSetConstantBuffers(
                            1,
                            Some(&[Some(self.pipeline.effects_buffer.clone())]),
                        );
                        self.context.PSSetShaderResources(1, Some(&masks));
                    }
                    bound_effects = Some((cmd.effects, flags));
                }
                unsafe {
                    self.context.RSSetScissorRects(Some(&[scissor]));
                    self.context.PSSetShaderResources(0, Some(&[Some(srv)]));
                    match geometry {
                        tacita::rendering::GeometryRange::Triangles {
                            first_index,
                            index_count,
                        } => {
                            self.context.IASetInputLayout(&self.pipeline.input_layout);
                            self.context.VSSetShader(
                                if self.color_mode == ColorMode::Srgb {
                                    &self.pipeline.managed_vertex
                                } else {
                                    &self.pipeline.vertex_shader
                                },
                                None,
                            );
                            self.context.IASetVertexBuffers(
                                0,
                                1,
                                Some([Some(self.pipeline.vertex_buffer.clone())].as_ptr()),
                                Some(&VERTEX_STRIDE),
                                Some(&0),
                            );
                            self.context.DrawIndexed(index_count, first_index, 0);
                        }
                        tacita::rendering::GeometryRange::Quads {
                            first_instance,
                            instance_count,
                        } => {
                            self.context.IASetInputLayout(&self.pipeline.quad_layout);
                            self.context.VSSetShader(
                                if self.color_mode == ColorMode::Srgb {
                                    &self.pipeline.managed_quad_vertex
                                } else {
                                    &self.pipeline.quad_vertex
                                },
                                None,
                            );
                            self.context.IASetVertexBuffers(
                                0,
                                1,
                                Some([Some(self.pipeline.quad_buffer.clone())].as_ptr()),
                                Some(&80),
                                Some(&0),
                            );
                            self.context
                                .DrawInstanced(6, instance_count, 0, first_instance);
                        }
                    }
                    self.stats.draw_calls += 1;
                }
            }
            Ok(())
        })();
        self.timer.end(&self.context);
        result
    }

    pub fn upload_strategy(&self) -> UploadStrategy {
        UploadStrategy::DiscardMap
    }
    pub fn stats(&mut self) -> RenderStats {
        self.timer.poll(&self.context);
        let mut stats = self.stats;
        stats.vertex_capacity = self.pipeline.vertex_capacity as u64;
        stats.index_capacity = self.pipeline.index_capacity as u64;
        stats.upload_capacity = stats.vertex_capacity * 20
            + stats.index_capacity * 4
            + self.pipeline.quad_capacity as u64 * 80;
        stats.pool_entries = (self.vertex_pool.len() + self.index_pool.len()) as u64;
        stats.pool_bytes = self
            .vertex_pool
            .iter()
            .map(|(n, _)| *n as u64 * 20)
            .chain(self.index_pool.iter().map(|(n, _)| *n as u64 * 4))
            .sum();
        stats.gpu_ms = self.timer.latest();
        stats
    }
    pub fn reserve_geometry(&mut self, vertices: u32, indices: u32) -> Result<(), BackendError> {
        self.ensure_buffer_capacity(vertices, indices)
    }
    pub fn set_geometry_capacity(
        &mut self,
        vertices: u32,
        indices: u32,
    ) -> Result<(), BackendError> {
        if vertices == 0
            || indices == 0
            || vertices > 64 * 1024 * 1024 / 20
            || indices > 64 * 1024 * 1024 / 4
        {
            return Err(BackendError::InvalidTextureData(
                "geometry capacity outside 64 MiB bound".into(),
            ));
        }
        for (required, current, buffer, pool, stride, bind) in [
            (
                vertices,
                &mut self.pipeline.vertex_capacity,
                &mut self.pipeline.vertex_buffer,
                &mut self.vertex_pool,
                20,
                D3D11_BIND_VERTEX_BUFFER,
            ),
            (
                indices,
                &mut self.pipeline.index_capacity,
                &mut self.pipeline.index_buffer,
                &mut self.index_pool,
                4,
                D3D11_BIND_INDEX_BUFFER,
            ),
        ] {
            if required == *current {
                continue;
            }
            let found = pool
                .iter()
                .enumerate()
                .filter(|(_, item)| item.0 >= required)
                .min_by_key(|(_, item)| item.0)
                .map(|(i, _)| i);
            let (capacity, replacement) = if let Some(index) = found {
                self.stats.reuses += 1;
                pool.swap_remove(index)
            } else {
                self.stats.allocations += 1;
                (
                    required,
                    create_dynamic_buffer(&self.device, required * stride, bind)?,
                )
            };
            let old = std::mem::replace(buffer, replacement);
            let old_capacity = std::mem::replace(current, capacity);
            if pool.len() < 4
                && pool
                    .iter()
                    .map(|(n, _)| *n as u64 * stride as u64)
                    .sum::<u64>()
                    + old_capacity as u64 * stride as u64
                    <= 64 * 1024 * 1024
            {
                pool.push((old_capacity, old));
            }
        }
        Ok(())
    }
    pub fn trim_geometry_pool(&mut self) {
        self.vertex_pool.clear();
        self.index_pool.clear();
    }

    fn ensure_buffer_capacity(
        &mut self,
        vertex_count: u32,
        index_count: u32,
    ) -> Result<(), BackendError> {
        const MAX_VERTICES: u32 = 64 * 1024 * 1024 / 20;
        const MAX_INDICES: u32 = 64 * 1024 * 1024 / 4;
        if vertex_count > MAX_VERTICES || index_count > MAX_INDICES {
            return Err(BackendError::InvalidTextureData(
                "geometry capacity outside 64 MiB bound".into(),
            ));
        }
        if vertex_count > self.pipeline.vertex_capacity
            || index_count > self.pipeline.index_capacity
        {
            self.set_geometry_capacity(
                grow_capacity(self.pipeline.vertex_capacity, vertex_count).min(MAX_VERTICES),
                grow_capacity(self.pipeline.index_capacity, index_count).min(MAX_INDICES),
            )?;
        }
        Ok(())
    }

    fn upload_geometry(&self, draw_data: &DrawData) -> Result<(), BackendError> {
        let vertex_bytes = vertex_bytes(draw_data.vertices);
        let index_bytes = index_bytes(draw_data.indices);
        unsafe {
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.context
                .Map(
                    &self.pipeline.vertex_buffer,
                    0,
                    D3D11_MAP_WRITE_DISCARD,
                    0,
                    Some(&mut mapped),
                )
                .map_err(BackendError::MapFailed)?;
            std::ptr::copy_nonoverlapping(
                vertex_bytes.as_ptr(),
                mapped.pData.cast::<u8>(),
                vertex_bytes.len(),
            );
            self.context.Unmap(&self.pipeline.vertex_buffer, 0);

            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.context
                .Map(
                    &self.pipeline.index_buffer,
                    0,
                    D3D11_MAP_WRITE_DISCARD,
                    0,
                    Some(&mut mapped),
                )
                .map_err(BackendError::MapFailed)?;
            std::ptr::copy_nonoverlapping(
                index_bytes.as_ptr(),
                mapped.pData.cast::<u8>(),
                index_bytes.len(),
            );
            self.context.Unmap(&self.pipeline.index_buffer, 0);
        }
        Ok(())
    }

    fn write_projection(&self, width: f32, height: f32) -> Result<(), BackendError> {
        let columns: [f32; 16] = [
            2.0 / width,
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 / height,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            -1.0,
            1.0,
            0.0,
            1.0,
        ];
        unsafe {
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.context
                .Map(
                    &self.pipeline.constant_buffer,
                    0,
                    D3D11_MAP_WRITE_DISCARD,
                    0,
                    Some(&mut mapped),
                )
                .map_err(BackendError::MapFailed)?;
            std::ptr::copy_nonoverlapping(columns.as_ptr(), mapped.pData.cast::<f32>(), 16);
            self.context.Unmap(&self.pipeline.constant_buffer, 0);
        }
        Ok(())
    }

    fn write_effects(
        &self,
        effects: tacita::effects::DrawEffects,
        flags: u32,
    ) -> Result<(), BackendError> {
        let bytes = effects.uniform_bytes_with_alpha(flags);
        unsafe {
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.context
                .Map(
                    &self.pipeline.effects_buffer,
                    0,
                    D3D11_MAP_WRITE_DISCARD,
                    0,
                    Some(&mut mapped),
                )
                .map_err(BackendError::MapFailed)?;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), mapped.pData.cast::<u8>(), bytes.len());
            self.context.Unmap(&self.pipeline.effects_buffer, 0);
        }
        Ok(())
    }

    fn bind_pipeline(&self, target: &ID3D11RenderTargetView, width: f32, height: f32) {
        let viewport = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: width,
            Height: height,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        unsafe {
            self.context.RSSetViewports(Some(&[viewport]));
            self.context
                .OMSetRenderTargets(Some(&[Some(target.clone())]), None);
            self.context.IASetInputLayout(&self.pipeline.input_layout);
            let stride = VERTEX_STRIDE;
            let offset = 0u32;
            self.context.IASetVertexBuffers(
                0,
                1,
                Some(&Some(self.pipeline.vertex_buffer.clone())),
                Some(&stride),
                Some(&offset),
            );
            self.context
                .IASetIndexBuffer(&self.pipeline.index_buffer, DXGI_FORMAT_R32_UINT, 0);
            self.context
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.context.VSSetShader(
                if self.color_mode == ColorMode::Srgb {
                    &self.pipeline.managed_vertex
                } else {
                    &self.pipeline.vertex_shader
                },
                None,
            );
            self.context
                .VSSetConstantBuffers(0, Some(&[Some(self.pipeline.constant_buffer.clone())]));
            self.context.PSSetShader(
                if self.color_mode == ColorMode::Srgb {
                    &self.pipeline.managed_pixel
                } else {
                    &self.pipeline.pixel_shader
                },
                None,
            );
            self.context
                .PSSetSamplers(0, Some(&[Some(self.pipeline.sampler.clone())]));
            self.context.GSSetShader(None, None);
            self.context
                .OMSetBlendState(&self.pipeline.blend_state, Some(&[0.0; 4]), 0xffffffff);
            self.context
                .OMSetDepthStencilState(&self.pipeline.depth_stencil_state, 0);
            self.context.RSSetState(&self.pipeline.rasterizer_state);
        }
    }
}

fn create_device_with(
    feature_levels: &[D3D_FEATURE_LEVEL],
) -> Result<(Device, Context), BackendError> {
    let mut device = None;
    let mut context = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            Some(feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
    }
    .map_err(BackendError::DeviceCreation)?;
    let unexpected =
        || BackendError::DeviceCreation(windows::core::Error::from_hresult(E_UNEXPECTED));
    Ok((
        device.ok_or_else(unexpected)?,
        context.ok_or_else(unexpected)?,
    ))
}

fn create_pipeline(
    device: &Device,
    vertex_source: &str,
    pixel_source: &str,
) -> Result<Pipeline, BackendError> {
    let vs_blob = shader::compile(vertex_source, shader::VS_ENTRY, shader::VS_TARGET)?;
    let ps_blob = shader::compile(pixel_source, shader::PS_ENTRY, shader::PS_TARGET)?;
    let managed_vs = shader::compile(
        &format!("#define GUI_MANAGED_SRGB 1\n{}", vertex_source),
        shader::VS_ENTRY,
        shader::VS_TARGET,
    )?;
    let managed_ps = shader::compile(
        &format!("#define GUI_MANAGED_SRGB 1\n{}", pixel_source),
        shader::PS_ENTRY,
        shader::PS_TARGET,
    )?;
    let mut managed_vertex = None;
    let mut managed_pixel = None;
    unsafe {
        device.CreateVertexShader(
            std::slice::from_raw_parts(
                managed_vs.GetBufferPointer().cast::<u8>(),
                managed_vs.GetBufferSize(),
            ),
            None,
            Some(&mut managed_vertex),
        )
    }
    .map_err(|e| BackendError::StateCreation("managed vertex shader", e))?;
    unsafe {
        device.CreatePixelShader(
            std::slice::from_raw_parts(
                managed_ps.GetBufferPointer().cast::<u8>(),
                managed_ps.GetBufferSize(),
            ),
            None,
            Some(&mut managed_pixel),
        )
    }
    .map_err(|e| BackendError::StateCreation("managed pixel shader", e))?;
    let managed_vertex = compositor::required(managed_vertex)?;
    let managed_pixel = compositor::required(managed_pixel)?;
    let vs_bytecode = unsafe {
        std::slice::from_raw_parts(
            vs_blob.GetBufferPointer().cast::<u8>(),
            vs_blob.GetBufferSize(),
        )
    };
    let ps_bytecode = unsafe {
        std::slice::from_raw_parts(
            ps_blob.GetBufferPointer().cast::<u8>(),
            ps_blob.GetBufferSize(),
        )
    };

    let mut vertex_shader = None;
    unsafe { device.CreateVertexShader(vs_bytecode, None, Some(&mut vertex_shader)) }
        .map_err(|e| BackendError::StateCreation("vertex shader", e))?;
    let vertex_shader = vertex_shader.ok_or_else(|| {
        BackendError::StateCreation(
            "vertex shader",
            windows::core::Error::from_hresult(E_UNEXPECTED),
        )
    })?;

    let mut input_layout = None;
    unsafe { device.CreateInputLayout(&INPUT_LAYOUT, vs_bytecode, Some(&mut input_layout)) }
        .map_err(|e| BackendError::StateCreation("input layout", e))?;
    let input_layout = input_layout.ok_or_else(|| {
        BackendError::StateCreation(
            "input layout",
            windows::core::Error::from_hresult(E_UNEXPECTED),
        )
    })?;

    let mut pixel_shader = None;
    unsafe { device.CreatePixelShader(ps_bytecode, None, Some(&mut pixel_shader)) }
        .map_err(|e| BackendError::StateCreation("pixel shader", e))?;
    let pixel_shader = pixel_shader.ok_or_else(|| {
        BackendError::StateCreation(
            "pixel shader",
            windows::core::Error::from_hresult(E_UNEXPECTED),
        )
    })?;

    let constant_buffer = create_dynamic_buffer(device, 64, D3D11_BIND_CONSTANT_BUFFER)?;
    let effects_buffer = create_dynamic_buffer(
        device,
        tacita::effects::EFFECT_UNIFORM_SIZE as u32,
        D3D11_BIND_CONSTANT_BUFFER,
    )?;

    let mut blend_desc = D3D11_BLEND_DESC::default();
    blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
        BlendEnable: true.into(),
        SrcBlend: D3D11_BLEND_SRC_ALPHA,
        DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
        BlendOp: D3D11_BLEND_OP_ADD,
        SrcBlendAlpha: D3D11_BLEND_ONE,
        DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
        BlendOpAlpha: D3D11_BLEND_OP_ADD,
        RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
    };
    let blend_state = create_state(device, &blend_desc, "blend state", |d, desc, out| unsafe {
        d.CreateBlendState(desc, out)
    })?;
    blend_desc.RenderTarget[0].SrcBlend = D3D11_BLEND_ONE;
    let premult_blend = create_state(
        device,
        &blend_desc,
        "premultiplied blend",
        |d, desc, out| unsafe { d.CreateBlendState(desc, out) },
    )?;

    let rasterizer_desc = D3D11_RASTERIZER_DESC {
        FillMode: D3D11_FILL_SOLID,
        CullMode: D3D11_CULL_NONE,
        ScissorEnable: true.into(),
        DepthClipEnable: true.into(),
        MultisampleEnable: true.into(),
        ..Default::default()
    };
    let rasterizer_state = create_state(
        device,
        &rasterizer_desc,
        "rasterizer state",
        |d, desc, out| unsafe { d.CreateRasterizerState(desc, out) },
    )?;

    let depth_stencil_desc = D3D11_DEPTH_STENCIL_DESC {
        DepthEnable: false.into(),
        DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ALL,
        DepthFunc: D3D11_COMPARISON_ALWAYS,
        StencilEnable: false.into(),
        ..Default::default()
    };
    let depth_stencil_state = create_state(
        device,
        &depth_stencil_desc,
        "depth-stencil state",
        |d, desc, out| unsafe { d.CreateDepthStencilState(desc, out) },
    )?;

    let sampler_desc = D3D11_SAMPLER_DESC {
        Filter: D3D11_FILTER_MIN_MAG_MIP_POINT,
        AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
        AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
        AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
        ComparisonFunc: D3D11_COMPARISON_ALWAYS,
        ..Default::default()
    };
    let sampler = create_state(
        device,
        &sampler_desc,
        "sampler state",
        |d, desc, out| unsafe { d.CreateSamplerState(desc, out) },
    )?;

    let vertex_buffer = create_dynamic_buffer(
        device,
        INITIAL_VERTEX_CAPACITY * VERTEX_STRIDE,
        D3D11_BIND_VERTEX_BUFFER,
    )?;
    let index_buffer =
        create_dynamic_buffer(device, INITIAL_INDEX_CAPACITY * 4, D3D11_BIND_INDEX_BUFFER)?;

    Ok(Pipeline {
        quad_vertex: rendering_options::quad_shader(device, vertex_source, false)?,
        managed_quad_vertex: rendering_options::quad_shader(device, vertex_source, true)?,
        quad_layout: rendering_options::quad_layout(device, vertex_source)?,
        quad_buffer: create_dynamic_buffer(device, 80 * 1024, D3D11_BIND_VERTEX_BUFFER)?,
        quad_capacity: 1024,
        premult_blend,
        vertex_shader,
        pixel_shader,
        managed_vertex,
        managed_pixel,
        input_layout,
        constant_buffer,
        effects_buffer,
        stencil: compositor::Stencil::new(device)?,
        blend_state,
        rasterizer_state,
        depth_stencil_state,
        sampler,
        vertex_buffer,
        vertex_capacity: INITIAL_VERTEX_CAPACITY,
        index_buffer,
        index_capacity: INITIAL_INDEX_CAPACITY,
    })
}

fn create_state<T, D>(
    device: &Device,
    desc: &D,
    what: &'static str,
    create: impl FnOnce(&Device, *const D, Option<*mut Option<T>>) -> windows::core::Result<()>,
) -> Result<T, BackendError> {
    let mut out = None;
    create(device, desc, Some(&mut out)).map_err(|e| BackendError::StateCreation(what, e))?;
    out.ok_or_else(|| {
        BackendError::StateCreation(what, windows::core::Error::from_hresult(E_UNEXPECTED))
    })
}

fn create_dynamic_buffer(
    device: &Device,
    byte_width: u32,
    bind_flag: windows::Win32::Graphics::Direct3D11::D3D11_BIND_FLAG,
) -> Result<ID3D11Buffer, BackendError> {
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: byte_width,
        Usage: D3D11_USAGE_DYNAMIC,
        BindFlags: bind_flag.0 as u32,
        CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
        MiscFlags: 0,
        StructureByteStride: 0,
    };
    let mut buffer = None;
    unsafe { device.CreateBuffer(&desc, None, Some(&mut buffer)) }
        .map_err(BackendError::BufferCreation)?;
    buffer.ok_or_else(|| {
        BackendError::BufferCreation(windows::core::Error::from_hresult(E_UNEXPECTED))
    })
}

fn grow_capacity(current: u32, needed: u32) -> u32 {
    let mut capacity = current.max(1);
    while capacity < needed {
        capacity *= 2;
    }
    capacity
}

fn scissor_rect(clip_rect: [f32; 4], width: f32, height: f32) -> Option<RECT> {
    let min_x = clip_rect[0].clamp(0.0, width);
    let min_y = clip_rect[1].clamp(0.0, height);
    let max_x = clip_rect[2].clamp(0.0, width);
    let max_y = clip_rect[3].clamp(0.0, height);
    if max_x <= min_x || max_y <= min_y {
        return None;
    }
    Some(RECT {
        left: min_x.floor() as i32,
        top: min_y.floor() as i32,
        right: max_x.ceil() as i32,
        bottom: max_y.ceil() as i32,
    })
}

fn vertex_bytes(vertices: &[DrawVert]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            vertices.as_ptr().cast::<u8>(),
            std::mem::size_of_val(vertices),
        )
    }
}

fn index_bytes(indices: &[u32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            indices.as_ptr().cast::<u8>(),
            std::mem::size_of_val(indices),
        )
    }
}
