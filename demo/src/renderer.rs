use std::sync::Arc;
use tacita::rendering::{AlphaMode, CommandPlan, DrawCalls, DrawOptimization, GeometryRange};
#[path = "renderer_options.rs"]
mod renderer_options;
use renderer_options::RasterBundle;

use tacita::draw::{DrawData, DrawVert, TextureId};
use winit::window::Window;

use crate::chrome::ChromeMesh;
use crate::cursor::OrbitalCursor;

const SHADER: &str = concat!(
    include_str!("../../backends/shaders/blue_noise.wgsl"),
    include_str!("../../backends/shaders/gui.wgsl"),
    include_str!("../../backends/shaders/surface.wgsl")
);

const VERTEX_SIZE: u64 = core::mem::size_of::<DrawVert>() as u64;

struct RenderTarget {
    id: TextureId,
    texture: wgpu::Texture,
    mode: tacita::rendering::ColorMode,
    stencil: Option<(wgpu::Texture, wgpu::TextureView)>,
}

pub struct Renderer {
    capture_path: Option<std::path::PathBuf>,
    frame_progress: crate::frame_watchdog::Progress,
    optimization: DrawOptimization,
    plan: CommandPlan,
    sample_count: u32,
    alpha_mode: AlphaMode,
    texture_alpha: Vec<(TextureId, AlphaMode)>,
    raster: Option<RasterBundle>,
    pipeline_layout: wgpu::PipelineLayout,
    shader_source: String,
    supported_samples: [bool; 4],
    msaa: Option<(wgpu::Texture, wgpu::TextureView)>,
    instance_buffer: wgpu::Buffer,
    instance_capacity: u64,
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    scale_factor: f32,
    pipeline: wgpu::RenderPipeline,
    variants: [wgpu::RenderPipeline; 5],
    image_pipeline: wgpu::RenderPipeline,
    stencil: Option<(wgpu::Texture, wgpu::TextureView)>,
    gallery: Option<crate::postprocess::Gallery>,
    effects: crate::effects_renderer::EffectsRenderer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    proj_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
    atlas_texture: wgpu::Texture,
    atlas_size: (u32, u32),
    atlas_generation: u64,
    color_atlas_key: Option<(TextureId, u64)>,
    host_textures: Vec<(u64, wgpu::BindGroup)>,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: u64,
    index_buffer: wgpu::Buffer,
    index_capacity: u64,
    vertex_pool: Vec<(wgpu::Buffer, u64)>,
    index_pool: Vec<(wgpu::Buffer, u64)>,
    stats: tacita::rendering::RenderStats,
    timer: Option<crate::gpu_timing::GpuTimer>,
    offscreen_pipelines: [[wgpu::RenderPipeline; 6]; 2],
    targets: Vec<RenderTarget>,
    preview_aliases: Vec<(TextureId, TextureId)>,
}

impl Renderer {
    pub fn set_frame_progress(&mut self, progress: crate::frame_watchdog::Progress) {
        self.frame_progress = progress;
    }
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

        let timing_supported = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        let optional_features = adapter.features()
            & (wgpu::Features::TIMESTAMP_QUERY
                | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: optional_features,
                ..Default::default()
            })
            .await
            .map_err(|e| format!("failed to create device: {e}"))?;

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = *caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .unwrap_or(&caps.formats[0]);
        eprintln!("[demo] surface format: {format:?}");
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | (caps.usages & wgpu::TextureUsages::COPY_SRC),
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
        let scale_factor = window.scale_factor() as f32;
        queue.write_buffer(
            &proj_buffer,
            0,
            &proj_matrix(config.width, config.height, scale_factor),
        );

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

        let effects_layout = crate::effects_renderer::EffectsRenderer::layout(&device);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gui shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gui pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout), Some(&effects_layout)],
            immediate_size: 0,
        });
        let managed_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("managed sRGB shader"),
            source: wgpu::ShaderSource::Wgsl(
                SHADER
                    .replace(
                        "const GUI_MANAGED_SRGB:bool=false;",
                        "const GUI_MANAGED_SRGB:bool=true;",
                    )
                    .into(),
            ),
        });
        let offscreen_pipelines = [
            core::array::from_fn(|i| {
                create_effect_pipeline(
                    &device,
                    &pipeline_layout,
                    &shader,
                    wgpu::TextureFormat::Rgba8Unorm,
                    i,
                )
            }),
            core::array::from_fn(|i| {
                create_effect_pipeline(
                    &device,
                    &pipeline_layout,
                    &managed_shader,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    i,
                )
            }),
        ];
        let timer = timing_supported.then(|| crate::gpu_timing::GpuTimer::new(&device, &queue));
        let pipeline = create_effect_pipeline(&device, &pipeline_layout, &shader, format, 0);
        let variants = core::array::from_fn(|i| {
            create_effect_pipeline(&device, &pipeline_layout, &shader, format, i + 1)
        });
        let image_pipeline = create_effect_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            wgpu::TextureFormat::Rgba8Unorm,
            1,
        );

        let atlas_texture = create_rgba_texture(&device, "gui font atlas", w, h);
        let effects =
            crate::effects_renderer::EffectsRenderer::new(&device, effects_layout, &atlas_texture);
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
            capture_path: None,
            frame_progress: crate::frame_watchdog::Progress::default(),
            instance,
            window,
            surface,
            device: device.clone(),
            queue,
            config,
            scale_factor,
            pipeline,
            variants,
            optimization: DrawOptimization::default(),
            plan: CommandPlan::default(),
            sample_count: 1,
            alpha_mode: AlphaMode::Straight,
            texture_alpha: Vec::new(),
            raster: None,
            pipeline_layout,
            shader_source: SHADER.into(),
            supported_samples: [1, 2, 4, 8].map(|n| {
                adapter
                    .get_texture_format_features(format)
                    .flags
                    .sample_count_supported(n)
                    && adapter
                        .get_texture_format_features(wgpu::TextureFormat::Depth24PlusStencil8)
                        .flags
                        .sample_count_supported(n)
            }),
            msaa: None,
            instance_buffer: renderer_options::instance_buffer(&device, 4096),
            instance_capacity: 4096,
            image_pipeline,
            stencil: None,
            gallery: None,
            effects,
            bind_group_layout,
            bind_group,
            proj_buffer,
            sampler,
            atlas_texture,
            atlas_size: (w, h),
            atlas_generation: generation,
            color_atlas_key: None,
            host_textures: Vec::new(),
            vertex_buffer,
            vertex_capacity,
            index_buffer,
            index_capacity,
            vertex_pool: Vec::new(),
            index_pool: Vec::new(),
            stats: tacita::rendering::RenderStats {
                allocations: 3,
                ..Default::default()
            },
            timer,
            offscreen_pipelines,
            targets: Vec::new(),
            preview_aliases: Vec::new(),
        })
    }

    pub fn capture_next_frame(&mut self, path: std::path::PathBuf) {
        self.capture_path = Some(path);
    }

    pub fn stats(&mut self) -> tacita::rendering::RenderStats {
        if let Some(timer) = &mut self.timer {
            timer.poll(&self.device);
            self.stats.gpu_ms = timer.latest;
        }
        let mut stats = self.stats;
        stats.vertex_capacity = self.vertex_capacity / 20;
        stats.index_capacity = self.index_capacity / 4;
        stats.upload_capacity = self.vertex_capacity + self.index_capacity + self.instance_capacity;
        stats.pool_entries = (self.vertex_pool.len() + self.index_pool.len()) as u64;
        stats.pool_bytes = self
            .vertex_pool
            .iter()
            .chain(&self.index_pool)
            .map(|(_, n)| *n)
            .sum();
        stats
    }
    pub fn timing_supported(&self) -> bool {
        self.timer.is_some()
    }
    pub fn reserve_geometry(&mut self, vertices: u64, indices: u64) {
        if vertices > self.vertex_capacity || indices > self.index_capacity {
            self.set_geometry_capacity(
                vertices.max(self.vertex_capacity),
                indices.max(self.index_capacity),
            );
        }
    }
    pub fn set_geometry_capacity(&mut self, vertices: u64, indices: u64) {
        for (required, current, buffer, pool, usage) in [
            (
                vertices,
                &mut self.vertex_capacity,
                &mut self.vertex_buffer,
                &mut self.vertex_pool,
                wgpu::BufferUsages::VERTEX,
            ),
            (
                indices,
                &mut self.index_capacity,
                &mut self.index_buffer,
                &mut self.index_pool,
                wgpu::BufferUsages::INDEX,
            ),
        ] {
            let required = required.max(4).next_power_of_two();
            assert!(required <= 64 * 1024 * 1024);
            if required == *current {
                continue;
            }
            let found = pool.iter().position(|(_, n)| *n == required);
            let (replacement, capacity) = if let Some(i) = found {
                self.stats.reuses += 1;
                pool.swap_remove(i)
            } else {
                self.stats.allocations += 1;
                (
                    self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("pooled GUI geometry"),
                        size: required,
                        usage: usage | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }),
                    required,
                )
            };
            let old = std::mem::replace(buffer, replacement);
            let old_capacity = std::mem::replace(current, capacity);
            if pool.len() < 4
                && pool.iter().map(|(_, n)| *n).sum::<u64>() + old_capacity <= 64 * 1024 * 1024
            {
                pool.push((old, old_capacity));
            }
        }
    }
    pub fn trim_geometry_pool(&mut self) {
        self.vertex_pool.clear();
        self.index_pool.clear();
    }

    pub fn render_to_texture(
        &mut self,
        id: TextureId,
        size: [u32; 2],
        mode: tacita::rendering::ColorMode,
        scale: f32,
        draw: &DrawData,
        clear: [f64; 4],
    ) -> Result<(), String> {
        use tacita::rendering::ColorMode;
        if !scale.is_finite()
            || scale <= 0.
            || scale > 8.
            || id.0 < 2
            || size.contains(&0)
            || size.iter().any(|&v| v > 8192)
            || !clear.iter().all(|v| v.is_finite())
            || draw
                .commands
                .iter()
                .any(|c| self.target_alias(c.texture, id))
            || draw.effects.iter().any(|e| {
                (0..4).any(|i| {
                    e.mask_texture(i)
                        .is_some_and(|texture| self.target_alias(texture, id))
                })
            })
        {
            return Err("invalid target or read/write feedback".into());
        }
        let mut index = self.targets.iter().position(|t| t.id == id);
        if index.is_none()
            || index.is_some_and(|i| {
                let t = &self.targets[i];
                t.texture.width() != size[0] || t.texture.height() != size[1] || t.mode != mode
            })
        {
            if index.is_none() && self.targets.len() >= 16 {
                return Err("sixteen retained targets already registered".into());
            }
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("GUI render texture"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: if mode == ColorMode::Srgb {
                    wgpu::TextureFormat::Rgba8UnormSrgb
                } else {
                    wgpu::TextureFormat::Rgba8Unorm
                },
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[
                    wgpu::TextureFormat::Rgba8Unorm,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                ],
            });
            let bg = create_bind_group(
                &self.device,
                &self.bind_group_layout,
                &self.proj_buffer,
                &texture,
                &self.sampler,
            );
            self.host_textures.retain(|(key, _)| *key != id.0);
            self.host_textures.push((id.0, bg));
            self.effects.add_texture(id, &texture);
            let target = RenderTarget {
                id,
                texture,
                mode,
                stencil: None,
            };
            if let Some(i) = index {
                self.targets[i] = target;
            } else {
                index = Some(self.targets.len());
                self.targets.push(target);
            }
        }
        let index = index.ok_or("target registration failed")?;
        let stencil_active = draw.effects.iter().any(|e| e.stencil != [0; 2]);
        if stencil_active && self.targets[index].stencil.is_none() {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("render texture stencil"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            self.targets[index].stencil = Some((texture, view));
        }
        self.prepare_plan(draw, scale)?;
        let alpha = self.needs_alpha(draw);
        self.effects
            .prepare_alpha(&self.device, &self.queue, draw, alpha);
        let triangles = !self.optimization.enabled()
            || self
                .plan
                .draws()
                .iter()
                .any(|d| matches!(d.geometry, GeometryRange::Triangles { .. }));
        let vertices = if triangles { draw.vertices } else { &[] };
        let indices = if triangles { draw.indices } else { &[] };
        self.reserve_geometry((vertices.len() * 20) as u64, (indices.len() * 4) as u64);
        self.queue
            .write_buffer(&self.vertex_buffer, 0, vertex_bytes(vertices));
        self.queue
            .write_buffer(&self.index_buffer, 0, index_bytes(indices));
        let clear = if self.alpha_mode == AlphaMode::Premultiplied {
            [
                clear[0] * clear[3],
                clear[1] * clear[3],
                clear[2] * clear[3],
                clear[3],
            ]
        } else {
            clear
        };
        self.queue
            .write_buffer(&self.proj_buffer, 0, &proj_matrix(size[0], size[1], scale));
        let target = &self.targets[index];
        let view = target.texture.create_view(&Default::default());
        let pipelines = &self.offscreen_pipelines[usize::from(mode == ColorMode::Srgb)];
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GUI offscreen"),
            });
        let mut draw_calls = 0;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GUI offscreen"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0],
                            g: clear[1],
                            b: clear[2],
                            a: clear[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: if stencil_active {
                    target.stencil.as_ref().map(|(_, view)| {
                        wgpu::RenderPassDepthStencilAttachment {
                            view,
                            depth_ops: None,
                            stencil_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(0),
                                store: wgpu::StoreOp::Store,
                            }),
                        }
                    })
                } else {
                    None
                },
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            for (cmd, geometry) in
                DrawCalls::new(draw, self.optimization.enabled().then_some(&self.plan))
            {
                if let Some((sx, sy, width, height)) =
                    physical_scissor(cmd.clip_rect, size, scale).filter(|_| cmd.elem_count > 0)
                {
                    let Some(bind_group) = self.bind_group_for(cmd.texture) else {
                        eprintln!(
                            "[demo] skipping draw command with unregistered texture {:?}",
                            cmd.texture
                        );
                        continue;
                    };
                    let effect = if cmd.effects == 0 {
                        tacita::effects::DrawEffects::default()
                    } else if let Some(e) = draw.effects.get(cmd.effects as usize - 1) {
                        *e
                    } else {
                        continue;
                    };
                    if effect.stencil[0] > 8 || effect.stencil[1] > 2 {
                        continue;
                    }
                    let mode = if stencil_active {
                        match effect.stencil[1] {
                            1 => 3,
                            2 => 4,
                            _ => {
                                if effect.filter.overwrite != 0 {
                                    5
                                } else {
                                    2
                                }
                            }
                        }
                    } else if effect.filter.overwrite != 0 {
                        1
                    } else {
                        0
                    };
                    pass.set_pipeline(if let Some(bundle) = &self.raster {
                        &bundle.offscreen[usize::from(target.mode == ColorMode::Srgb)]
                            [usize::from(matches!(geometry, GeometryRange::Quads { .. }))][mode]
                    } else {
                        &pipelines[mode]
                    });
                    pass.set_stencil_reference(effect.stencil[0]);
                    pass.set_bind_group(0, bind_group, &[]);
                    let Some((effects, offset)) = self
                        .effects
                        .binding_alpha(cmd.effects, self.command_alpha(cmd.texture, effect))
                    else {
                        continue;
                    };
                    pass.set_bind_group(1, effects, &[offset]);
                    pass.set_scissor_rect(sx, sy, width, height);
                    match geometry {
                        GeometryRange::Triangles {
                            first_index,
                            index_count,
                        } => {
                            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                            pass.draw_indexed(first_index..first_index + index_count, 0, 0..1);
                        }
                        GeometryRange::Quads {
                            first_instance,
                            instance_count,
                        } => {
                            pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
                            pass.draw(0..6, first_instance..first_instance + instance_count);
                        }
                    }
                    draw_calls += 1;
                }
            }
        }
        self.queue.submit([encoder.finish()]);
        self.queue.write_buffer(
            &self.proj_buffer,
            0,
            &proj_matrix(self.config.width, self.config.height, self.scale_factor),
        );
        self.stats.draw_calls = draw_calls;
        self.set_texture_alpha_mode(id, self.alpha_mode)?;
        Ok(())
    }

    fn target_alias(&self, input: TextureId, output: TextureId) -> bool {
        input == output
            || self
                .preview_aliases
                .iter()
                .any(|&(alias, source)| alias == input && source == output)
    }
    pub fn register_encoded_preview(
        &mut self,
        source: TextureId,
        alias: TextureId,
    ) -> Result<(), String> {
        if alias.0 < 2 || alias == source {
            return Err("invalid preview ID".into());
        }
        let target = self
            .targets
            .iter()
            .find(|t| t.id == source)
            .ok_or("render target missing")?;
        let view = target.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            ..Default::default()
        });
        let bg = create_bind_group_view(
            &self.device,
            &self.bind_group_layout,
            &self.proj_buffer,
            &view,
            &self.sampler,
        );
        self.host_textures.retain(|(id, _)| *id != alias.0);
        self.host_textures.push((alias.0, bg));
        self.preview_aliases.retain(|&(id, _)| id != alias);
        self.preview_aliases.push((alias, source));
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.scale_factor = scale_factor;
        self.surface.configure(&self.device, &self.config);
        self.queue.write_buffer(
            &self.proj_buffer,
            0,
            &proj_matrix(width, height, scale_factor),
        );
    }

    pub fn invalidate_atlas(&mut self) {
        self.atlas_generation = u64::MAX;
        self.color_atlas_key = None;
    }

    pub fn sync_color_atlas(&mut self, font: &tacita::font::Font) {
        let Some(id) = font.color_atlas_texture() else {
            return;
        };
        let key = (id, font.color_atlas_generation());
        if self.color_atlas_key == Some(key) {
            return;
        }
        let (pixels, w, h) = font.color_atlas_pixels();
        if pixels.is_empty() {
            return;
        }
        self.add_host_texture(id, pixels, w, h);
        self.color_atlas_key = Some(key);
    }

    pub fn sync_atlas(&mut self, pixels: &[u8], w: u32, h: u32, generation: u64) {
        if generation == self.atlas_generation {
            return;
        }
        if (w, h) != self.atlas_size {
            self.atlas_texture = create_rgba_texture(&self.device, "gui font atlas", w, h);
            self.effects.set_atlas(&self.atlas_texture);
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
        let row_bytes = (w * 4) as usize;
        let stride = row_bytes.div_ceil(256) * 256;
        let mut padded = Vec::new();
        let upload = if stride == row_bytes {
            rgba
        } else {
            padded.resize(stride * h as usize, 0);
            for (source, destination) in rgba
                .chunks_exact(row_bytes)
                .zip(padded.chunks_exact_mut(stride))
            {
                destination[..row_bytes].copy_from_slice(source);
            }
            &padded
        };
        let texture = create_rgba_texture(&self.device, "gui host texture", w, h);
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            upload,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride as u32),
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
        self.texture_alpha.retain(|(key, _)| *key != id);
        self.effects.add_texture(id, &texture);
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

    pub fn prepare_gallery(
        &mut self,
        phase: f32,
        blur: tacita::effects::Blur,
        threshold: f32,
        intensity: f32,
    ) {
        let resources = crate::postprocess::GpuResources {
            device: &self.device,
            queue: &self.queue,
            base: &self.bind_group_layout,
            effects: &self.effects.layout,
            sampler: &self.sampler,
        };
        let gallery = self
            .gallery
            .get_or_insert_with(|| crate::postprocess::Gallery::new(&resources));
        match gallery.prepare(
            &resources,
            &self.image_pipeline,
            phase,
            blur,
            threshold,
            intensity,
        ) {
            Ok(updates) => {
                for (id, texture) in updates {
                    self.effects.add_texture(id, &texture);
                    let bg = create_bind_group(
                        &self.device,
                        &self.bind_group_layout,
                        &self.proj_buffer,
                        &texture,
                        &self.sampler,
                    );
                    self.host_textures.retain(|(key, _)| *key != id.0);
                    self.host_textures.push((id.0, bg));
                }
            }
            Err(error) => eprintln!("[demo] invalid gallery filters: {error:?}"),
        }
    }

    pub fn render(
        &mut self,
        draw: &DrawData,
        chrome: &ChromeMesh,
        cursor: &OrbitalCursor,
        backdrop: [u8; 4],
    ) {
        if self.config.width == 0 || self.config.height == 0 {
            return;
        }
        self.frame_progress
            .mark(crate::frame_watchdog::Stage::Upload);
        if let Err(error) = self.prepare_plan(draw, self.scale_factor) {
            eprintln!("[demo] {error}");
            return;
        }
        let alpha = self.needs_alpha(draw);
        self.effects
            .prepare_alpha(&self.device, &self.queue, draw, alpha);
        self.ensure_msaa();
        let stencil_active = draw.effects.iter().any(|e| e.stencil != [0; 2]);
        if stencil_active
            && self.stencil.as_ref().is_none_or(|(t, _)| {
                t.width() != self.config.width
                    || t.height() != self.config.height
                    || t.sample_count() != self.sample_count
            })
        {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("GUI stencil"),
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: self.sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            self.stencil = Some((texture, view));
        }

        let cursor_vertex_bytes = vertex_bytes(&cursor.vertices);
        let cursor_index_bytes = index_bytes(&cursor.indices);
        let chrome_vertex_bytes = vertex_bytes(&chrome.vertices);
        let chrome_index_bytes = index_bytes(&chrome.indices);
        let triangles = !self.optimization.enabled()
            || self
                .plan
                .draws()
                .iter()
                .any(|d| matches!(d.geometry, GeometryRange::Triangles { .. }));
        let vertex_bytes = vertex_bytes(if triangles { draw.vertices } else { &[] });
        let index_bytes = index_bytes(if triangles { draw.indices } else { &[] });
        let gui_vertex_count = vertex_bytes.len() / 20;
        let total_vertex_bytes =
            (vertex_bytes.len() + chrome_vertex_bytes.len() + cursor_vertex_bytes.len()) as u64;
        let total_index_bytes =
            (index_bytes.len() + chrome_index_bytes.len() + cursor_index_bytes.len()) as u64;
        self.reserve_geometry(total_vertex_bytes, total_index_bytes);
        self.stats.uploaded_bytes = total_vertex_bytes
            + total_index_bytes
            + if self.optimization.enabled() {
                self.plan.instances().len() as u64 * 80
            } else {
                0
            };
        self.queue
            .write_buffer(&self.vertex_buffer, 0, vertex_bytes);
        self.queue.write_buffer(&self.index_buffer, 0, index_bytes);
        if !chrome.indices.is_empty() {
            self.queue.write_buffer(
                &self.vertex_buffer,
                vertex_bytes.len() as u64,
                chrome_vertex_bytes,
            );
            self.queue.write_buffer(
                &self.index_buffer,
                index_bytes.len() as u64,
                chrome_index_bytes,
            );
        }
        if !cursor.indices.is_empty() {
            self.queue.write_buffer(
                &self.vertex_buffer,
                (vertex_bytes.len() + chrome_vertex_bytes.len()) as u64,
                cursor_vertex_bytes,
            );
            self.queue.write_buffer(
                &self.index_buffer,
                (index_bytes.len() + chrome_index_bytes.len()) as u64,
                cursor_index_bytes,
            );
        }

        self.frame_progress
            .mark(crate::frame_watchdog::Stage::Acquire);
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
        self.frame_progress
            .mark(crate::frame_watchdog::Stage::Record);
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let timing = self
            .timer
            .as_mut()
            .and_then(|timer| timer.begin(&self.device));
        let mut draw_calls = 0;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("gui") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.msaa.as_ref().map_or(&view, |(_, v)| v),
                    depth_slice: None,
                    resolve_target: self.msaa.as_ref().map(|_| &view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: backdrop[0] as f64 / 255.0,
                            g: backdrop[1] as f64 / 255.0,
                            b: backdrop[2] as f64 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: if stencil_active {
                    self.stencil
                        .as_ref()
                        .map(|(_, view)| wgpu::RenderPassDepthStencilAttachment {
                            view,
                            depth_ops: None,
                            stencil_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(0),
                                store: wgpu::StoreOp::Discard,
                            }),
                        })
                } else {
                    None
                },
                timestamp_writes: timing
                    .as_ref()
                    .map(|(i, set)| wgpu::RenderPassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: Some(*i as u32 * 2),
                        end_of_pass_write_index: Some(*i as u32 * 2 + 1),
                    }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(self.surface_pipeline(false, if stencil_active { 2 } else { 0 }));
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_stencil_reference(0);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            for (cmd, geometry) in
                DrawCalls::new(draw, self.optimization.enabled().then_some(&self.plan))
            {
                if let Some((sx, sy, width, height)) = physical_scissor(
                    cmd.clip_rect,
                    [self.config.width, self.config.height],
                    self.scale_factor,
                )
                .filter(|_| cmd.elem_count > 0)
                {
                    let Some(bind_group) = self.bind_group_for(cmd.texture) else {
                        eprintln!(
                            "[demo] skipping draw command with unregistered texture {:?}",
                            cmd.texture
                        );
                        continue;
                    };
                    let effect = if cmd.effects == 0 {
                        tacita::effects::DrawEffects::default()
                    } else if let Some(e) = draw.effects.get(cmd.effects as usize - 1) {
                        *e
                    } else {
                        continue;
                    };
                    if effect.stencil[0] > 8 || effect.stencil[1] > 2 {
                        continue;
                    }
                    let mode = if stencil_active {
                        match effect.stencil[1] {
                            1 => 3,
                            2 => 4,
                            _ => {
                                if effect.filter.overwrite != 0 {
                                    5
                                } else {
                                    2
                                }
                            }
                        }
                    } else if effect.filter.overwrite != 0 {
                        1
                    } else {
                        0
                    };
                    pass.set_pipeline(
                        self.surface_pipeline(
                            matches!(geometry, GeometryRange::Quads { .. }),
                            mode,
                        ),
                    );
                    pass.set_stencil_reference(effect.stencil[0]);
                    pass.set_bind_group(0, bind_group, &[]);
                    let Some((effects, offset)) = self
                        .effects
                        .binding_alpha(cmd.effects, self.command_alpha(cmd.texture, effect))
                    else {
                        continue;
                    };
                    pass.set_bind_group(1, effects, &[offset]);
                    pass.set_scissor_rect(sx, sy, width, height);
                    match geometry {
                        GeometryRange::Triangles {
                            first_index,
                            index_count,
                        } => {
                            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                            pass.draw_indexed(first_index..first_index + index_count, 0, 0..1);
                        }
                        GeometryRange::Quads {
                            first_instance,
                            instance_count,
                        } => {
                            pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
                            pass.draw(0..6, first_instance..first_instance + instance_count);
                        }
                    }
                    draw_calls += 1;
                }
            }
            pass.set_pipeline(self.surface_pipeline(false, if stencil_active { 2 } else { 0 }));
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_stencil_reference(0);
            if !chrome.indices.is_empty() {
                pass.set_bind_group(0, &self.bind_group, &[]);
                if let Some((effects, offset)) = self.effects.binding_alpha(
                    0,
                    if self.alpha_mode == AlphaMode::Premultiplied {
                        2
                    } else {
                        0
                    },
                ) {
                    pass.set_bind_group(1, effects, &[offset]);
                }
                pass.set_scissor_rect(0, 0, self.config.width, self.config.height);
                let first_index = (index_bytes.len() / core::mem::size_of::<u32>()) as u32;
                pass.draw_indexed(
                    first_index..first_index + chrome.indices.len() as u32,
                    gui_vertex_count as i32,
                    0..1,
                );
            }
            if !cursor.indices.is_empty() {
                pass.set_bind_group(0, &self.bind_group, &[]);
                if let Some((effects, offset)) = self.effects.binding_alpha(
                    0,
                    if self.alpha_mode == AlphaMode::Premultiplied {
                        2
                    } else {
                        0
                    },
                ) {
                    pass.set_bind_group(1, effects, &[offset]);
                }
                pass.set_scissor_rect(0, 0, self.config.width, self.config.height);
                let first_index = ((index_bytes.len() + chrome_index_bytes.len())
                    / core::mem::size_of::<u32>()) as u32;
                pass.draw_indexed(
                    first_index..first_index + cursor.indices.len() as u32,
                    (gui_vertex_count + chrome.vertices.len()) as i32,
                    0..1,
                );
            }
        }
        let capture = self.capture_path.take().and_then(|path| {
            if self.config.usage.contains(wgpu::TextureUsages::COPY_SRC) {
                Some(crate::capture::FrameCapture::encode(
                    &self.device,
                    &mut encoder,
                    &surface_texture.texture,
                    path,
                ))
            } else {
                eprintln!("[capture] the surface does not support readback");
                None
            }
        });
        if let (Some(timer), Some((index, _))) = (&self.timer, &timing) {
            timer.resolve(&mut encoder, *index);
        }
        self.frame_progress
            .mark(crate::frame_watchdog::Stage::Submit);
        self.queue.submit([encoder.finish()]);
        if let Some(capture) = capture {
            if let Err(error) = capture.write(&self.device) {
                eprintln!("[capture] {error}");
            }
        }
        if let (Some(timer), Some((index, _))) = (&self.timer, &timing) {
            timer.map(*index);
        }
        self.stats.draw_calls = draw_calls
            + u64::from(!chrome.indices.is_empty())
            + u64::from(!cursor.indices.is_empty());
        self.window.pre_present_notify();
        self.frame_progress
            .mark(crate::frame_watchdog::Stage::Present);
        self.queue.present(surface_texture);
    }
}

fn physical_scissor(clip: [f32; 4], surface: [u32; 2], scale: f32) -> Option<(u32, u32, u32, u32)> {
    let [sw, sh] = [surface[0] as f32, surface[1] as f32];
    let x0 = (clip[0] * scale).clamp(0.0, sw);
    let y0 = (clip[1] * scale).clamp(0.0, sh);
    let x1 = (clip[2] * scale).clamp(0.0, sw);
    let y1 = (clip[3] * scale).clamp(0.0, sh);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let (sx, sy) = (x0.floor() as u32, y0.floor() as u32);
    Some((sx, sy, x1.ceil() as u32 - sx, y1.ceil() as u32 - sy))
}

pub(crate) fn proj_matrix(width: u32, height: u32, scale_factor: f32) -> [u8; 64] {
    let (w, h) = (width as f32 / scale_factor, height as f32 / scale_factor);
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

pub(crate) fn vertex_bytes(vertices: &[DrawVert]) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            vertices.as_ptr().cast::<u8>(),
            core::mem::size_of_val(vertices),
        )
    }
}

pub(crate) fn index_bytes(indices: &[u32]) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            indices.as_ptr().cast::<u8>(),
            core::mem::size_of_val(indices),
        )
    }
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

pub(crate) fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    proj_buffer: &wgpu::Buffer,
    texture: &wgpu::Texture,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    let view = texture.create_view(&Default::default());
    create_bind_group_view(device, layout, proj_buffer, &view, sampler)
}

fn create_bind_group_view(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    proj_buffer: &wgpu::Buffer,
    view: &wgpu::TextureView,
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
                resource: wgpu::BindingResource::TextureView(view),
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

fn create_effect_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    mode: usize,
) -> wgpu::RenderPipeline {
    create_raster_pipeline(
        device,
        pipeline_layout,
        shader,
        format,
        mode,
        1,
        AlphaMode::Straight,
        false,
    )
}
#[allow(clippy::too_many_arguments)]
fn create_raster_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    mode: usize,
    samples: u32,
    alpha: AlphaMode,
    instanced: bool,
) -> wgpu::RenderPipeline {
    let attributes: [wgpu::VertexAttribute; 12] = core::array::from_fn(|i| wgpu::VertexAttribute {
        format: if i % 3 == 2 {
            wgpu::VertexFormat::Unorm8x4
        } else {
            wgpu::VertexFormat::Float32x2
        },
        offset: (i / 3 * 20 + [0, 8, 16][i % 3]) as u64,
        shader_location: i as u32,
    });
    let face = wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: match mode {
            3 => wgpu::StencilOperation::IncrementClamp,
            4 => wgpu::StencilOperation::DecrementClamp,
            _ => wgpu::StencilOperation::Keep,
        },
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gui pipeline"),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(if instanced { "vs_quad" } else { "vs_main" }),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: if instanced { 80 } else { VERTEX_SIZE },
                step_mode: if instanced {
                    wgpu::VertexStepMode::Instance
                } else {
                    wgpu::VertexStepMode::Vertex
                },
                attributes: &attributes[..if instanced { 12 } else { 3 }],
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: if mode == 1 || mode == 3 || mode == 4 || mode == 5 {
                    None
                } else {
                    Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: if alpha == AlphaMode::Premultiplied {
                                wgpu::BlendFactor::One
                            } else {
                                wgpu::BlendFactor::SrcAlpha
                            },
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    })
                },
                write_mask: if mode == 3 || mode == 4 {
                    wgpu::ColorWrites::empty()
                } else {
                    wgpu::ColorWrites::ALL
                },
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: if mode >= 2 {
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState {
                    front: face,
                    back: face,
                    read_mask: 255,
                    write_mask: if mode == 3 || mode == 4 { 255 } else { 0 },
                },
                bias: Default::default(),
            })
        } else {
            None
        },
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod dpi_tests {
    use super::{physical_scissor, proj_matrix};

    #[test]
    fn logical_coordinates_map_to_physical_surface_and_clip() {
        let bytes = proj_matrix(1500, 900, 1.5);
        let floats: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        assert_eq!(floats[0], 2.0 / 1000.0);
        assert_eq!(floats[5], -2.0 / 600.0);
        assert_eq!(
            physical_scissor([10.25, 20.0, 30.4, 40.2], [1500, 900], 1.5),
            Some((15, 30, 31, 31))
        );
        assert_eq!(
            physical_scissor([-5.0, -8.0, 2000.0, 1000.0], [1500, 900], 1.5),
            Some((0, 0, 1500, 900))
        );
        assert_eq!(
            physical_scissor([100.0, 100.0, 100.0, 120.0], [1500, 900], 1.5),
            None
        );
    }
}
