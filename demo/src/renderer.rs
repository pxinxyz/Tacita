use std::sync::Arc;

use restricted_gui::draw::{DrawData, TextureId};
use winit::window::Window;

const SHADER: &str = r"
struct Uniforms {
    proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_smp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) col: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) col: vec4<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.pos = uniforms.proj * vec4<f32>(in.pos, 0.0, 1.0);
    out.uv = in.uv;
    out.col = in.col;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.col * textureSample(atlas_tex, atlas_smp, in.uv);
}
";

const VERTEX_SIZE: u64 = 20;

pub struct Renderer {
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    proj_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
    atlas_texture: wgpu::Texture,
    atlas_size: (u32, u32),
    atlas_generation: u64,
    host_textures: Vec<(u64, wgpu::BindGroup)>,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: u64,
    index_buffer: wgpu::Buffer,
    index_capacity: u64,
}

impl Renderer {
    pub async fn new(
        window: Arc<Window>,
        pixels: &[u8],
        w: u32,
        h: u32,
        generation: u64,
    ) -> Result<Renderer, String> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| format!("failed to create surface: {e}"))?;

        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
        {
            Ok(adapter) => adapter,
            Err(e) => {
                eprintln!("[demo] default adapter request failed ({e}); retrying with force_fallback_adapter");
                instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: true,
                        ..Default::default()
                    })
                    .await
                    .map_err(|e2| format!("no suitable GPU adapter: {e2}"))?
            }
        };
        let info = adapter.get_info();
        eprintln!(
            "[demo] adapter: {} ({:?}, {:?})",
            info.name, info.backend, info.device_type
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|e| format!("failed to create device: {e}"))?;

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = *caps
            .formats
            .iter()
            // Non-sRGB target so alpha blending executes in linear space. Blend in sRGB and your 1px AA fringe turns into a bruise. Ask the rounded corners.
            .find(|f| !f.is_srgb())
            .unwrap_or(&caps.formats[0]);
        eprintln!("[demo] surface format: {format:?}");
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let proj_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gui projection"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&proj_buffer, 0, &proj_matrix(config.width, config.height));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gui bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gui atlas sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gui shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gui pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gui pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: VERTEX_SIZE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Unorm8x4,
                            offset: 16,
                            shader_location: 2,
                        },
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let atlas_texture = create_rgba_texture(&device, "gui font atlas", w, h);
        let bind_group = create_bind_group(
            &device,
            &bind_group_layout,
            &proj_buffer,
            &atlas_texture,
            &sampler,
        );
        write_atlas(&queue, &atlas_texture, pixels, w, h);

        let vertex_capacity = 4096 * VERTEX_SIZE;
        let index_capacity = 8192 * 4;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gui vertices"),
            size: vertex_capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gui indices"),
            size: index_capacity,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Renderer {
            instance,
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            bind_group_layout,
            bind_group,
            proj_buffer,
            sampler,
            atlas_texture,
            atlas_size: (w, h),
            atlas_generation: generation,
            host_textures: Vec::new(),
            vertex_buffer,
            vertex_capacity,
            index_buffer,
            index_capacity,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.queue
            .write_buffer(&self.proj_buffer, 0, &proj_matrix(width, height));
    }

    pub fn sync_atlas(&mut self, pixels: &[u8], w: u32, h: u32, generation: u64) {
        if generation == self.atlas_generation {
            return;
        }
        if (w, h) != self.atlas_size {
            self.atlas_texture = create_rgba_texture(&self.device, "gui font atlas", w, h);
            self.bind_group = create_bind_group(
                &self.device,
                &self.bind_group_layout,
                &self.proj_buffer,
                &self.atlas_texture,
                &self.sampler,
            );
            self.atlas_size = (w, h);
            eprintln!("[demo] atlas grew to {w}x{h} (generation {generation})");
        }
        write_atlas(&self.queue, &self.atlas_texture, pixels, w, h);
        self.atlas_generation = generation;
    }

    pub fn add_host_texture(&mut self, id: TextureId, rgba: &[u8], w: u32, h: u32) {
        assert!(
            id != TextureId::FONT && id != TextureId::WHITE,
            "TextureId {id:?} is reserved for the built-in atlas"
        );
        assert_eq!(
            rgba.len(),
            (w * h * 4) as usize,
            "host texture data must be w*h*4 RGBA bytes"
        );
        assert_eq!(
            w % 64,
            0,
            "host texture width {w} must be a multiple of 64 px: the upload \
             does no row padding, so 4*w must satisfy write_texture's \
             256-byte row alignment"
        );
        let texture = create_rgba_texture(&self.device, "gui host texture", w, h);
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        let bind_group = create_bind_group(
            &self.device,
            &self.bind_group_layout,
            &self.proj_buffer,
            &texture,
            &self.sampler,
        );
        self.host_textures.retain(|(k, _)| *k != id.0);
        self.host_textures.push((id.0, bind_group));
    }

    fn bind_group_for(&self, id: TextureId) -> Option<&wgpu::BindGroup> {
        if id == TextureId::FONT || id == TextureId::WHITE {
            return Some(&self.bind_group);
        }
        self.host_textures
            .iter()
            .find(|(k, _)| *k == id.0)
            .map(|(_, bg)| bg)
    }

    pub fn render(&mut self, draw: &DrawData) {
        if self.config.width == 0 || self.config.height == 0 {
            return;
        }

        let vertex_bytes = pack_vertices(draw);
        let index_bytes = pack_indices(draw);
        if vertex_bytes.len() as u64 > self.vertex_capacity {
            self.vertex_capacity = (vertex_bytes.len() as u64).next_power_of_two();
            self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gui vertices"),
                size: self.vertex_capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if index_bytes.len() as u64 > self.index_capacity {
            self.index_capacity = (index_bytes.len() as u64).next_power_of_two();
            self.index_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gui indices"),
                size: self.index_capacity,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.queue
            .write_buffer(&self.vertex_buffer, 0, &vertex_bytes);
        self.queue.write_buffer(&self.index_buffer, 0, &index_bytes);

        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                return;
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                drop(t);
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                if let Ok(surface) = self.instance.create_surface(self.window.clone()) {
                    self.surface = surface;
                    self.surface.configure(&self.device, &self.config);
                }
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("[demo] surface validation error; skipping frame");
                return;
            }
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("gui") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 17.0 / 255.0,
                            g: 17.0 / 255.0,
                            b: 27.0 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            let (sw, sh) = (self.config.width as f32, self.config.height as f32);
            let mut index_offset = 0u32;
            for cmd in draw.commands {
                let x0 = cmd.clip_rect[0].clamp(0.0, sw);
                let y0 = cmd.clip_rect[1].clamp(0.0, sh);
                let x1 = cmd.clip_rect[2].clamp(0.0, sw);
                let y1 = cmd.clip_rect[3].clamp(0.0, sh);
                if x1 > x0 && y1 > y0 && cmd.elem_count > 0 {
                    let Some(bind_group) = self.bind_group_for(cmd.texture) else {
                        eprintln!(
                            "[demo] skipping draw command with unregistered texture {:?}",
                            cmd.texture
                        );
                        index_offset += cmd.elem_count;
                        continue;
                    };
                    pass.set_bind_group(0, bind_group, &[]);
                    let (sx, sy) = (x0.floor() as u32, y0.floor() as u32);
                    let (sx1, sy1) = (x1.ceil() as u32, y1.ceil() as u32);
                    pass.set_scissor_rect(sx, sy, sx1 - sx, sy1 - sy);
                    pass.draw_indexed(index_offset..index_offset + cmd.elem_count, 0, 0..1);
                }
                index_offset += cmd.elem_count;
            }
        }
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(surface_texture);
    }
}

fn proj_matrix(width: u32, height: u32) -> [u8; 64] {
    let (w, h) = (width as f32, height as f32);
    let cols: [[f32; 4]; 4] = [
        [2.0 / w, 0.0, 0.0, 0.0],
        [0.0, -2.0 / h, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0, 1.0],
    ];
    let mut bytes = [0u8; 64];
    for (i, f) in cols.into_iter().flatten().enumerate() {
        bytes[i * 4..i * 4 + 4].copy_from_slice(&f.to_ne_bytes());
    }
    bytes
}

fn pack_vertices(draw: &DrawData) -> Vec<u8> {
    let mut out = Vec::with_capacity(draw.vertices.len() * VERTEX_SIZE as usize);
    for v in draw.vertices {
        for f in [v.pos[0], v.pos[1], v.uv[0], v.uv[1]] {
            out.extend_from_slice(&f.to_ne_bytes());
        }
        out.extend_from_slice(&v.col);
    }
    out
}

fn pack_indices(draw: &DrawData) -> Vec<u8> {
    let mut out = Vec::with_capacity(draw.indices.len() * 4);
    for i in draw.indices {
        out.extend_from_slice(&i.to_ne_bytes());
    }
    out
}

fn create_rgba_texture(device: &wgpu::Device, label: &str, w: u32, h: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    proj_buffer: &wgpu::Buffer,
    texture: &wgpu::Texture,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("gui bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: proj_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn write_atlas(queue: &wgpu::Queue, texture: &wgpu::Texture, pixels: &[u8], w: u32, h: u32) {
    let mut rgba = Vec::with_capacity(pixels.len() * 4);
    for &a in pixels {
        rgba.extend_from_slice(&[255, 255, 255, a]);
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * w),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
}
