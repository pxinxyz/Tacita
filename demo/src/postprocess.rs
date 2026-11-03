use crate::renderer::{create_bind_group, index_bytes, proj_matrix, vertex_bytes};
use restricted_gui::{
    draw::TextureId,
    effects::{
        Blur, DrawEffects, FilterPacket, FilterPlan, ImageFilter, SurfaceStyle, SURFACE_DISTORT,
        SURFACE_SWEEP,
    },
};
pub const SCENE: TextureId = TextureId(20);
pub const BLURRED: TextureId = TextureId(21);
pub const BLOOM: TextureId = TextureId(22);
const SIZE: [u32; 2] = [512, 256];

pub struct GpuResources<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub base: &'a wgpu::BindGroupLayout,
    pub effects: &'a wgpu::BindGroupLayout,
    pub sampler: &'a wgpu::Sampler,
}

fn texture(device: &wgpu::Device, size: [u32; 2]) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sampleable effect target"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}
fn buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
struct Pass {
    target: wgpu::Texture,
    view: wgpu::TextureView,
    size: [u32; 2],
    base: wgpu::BindGroup,
    effects: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
}
impl Pass {
    fn new(
        resources: &GpuResources<'_>,
        source: &wgpu::Texture,
        original: &wgpu::Texture,
        size: [u32; 2],
    ) -> Self {
        let GpuResources {
            device,
            queue,
            base: base_layout,
            effects: effect_layout,
            sampler,
        } = *resources;
        let target = texture(device, size);
        let view = target.create_view(&Default::default());
        let projection = buffer(device, "pass projection", 64, wgpu::BufferUsages::UNIFORM);
        queue.write_buffer(&projection, 0, &proj_matrix(size[0], size[1], 1.));
        let base = create_bind_group(device, base_layout, &projection, source, sampler);
        let uniform = buffer(device, "pass effects", 1024, wgpu::BufferUsages::UNIFORM);
        let original = original.create_view(&Default::default());
        let effects = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pass effects"),
            layout: effect_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniform,
                        offset: 0,
                        size: wgpu::BufferSize::new(1024),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&original),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&original),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&original),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&original),
                },
            ],
        });
        let vertices = buffer(device, "pass quad", 80, wgpu::BufferUsages::VERTEX);
        let indices = buffer(device, "pass indices", 24, wgpu::BufferUsages::INDEX);
        let packet = FilterPacket::new(size, SCENE, ImageFilter::default())
            .expect("validated pass dimensions");
        let draw = packet.draw_data();
        queue.write_buffer(&vertices, 0, vertex_bytes(draw.vertices));
        queue.write_buffer(&indices, 0, index_bytes(draw.indices));
        Self {
            target,
            view,
            size,
            base,
            effects,
            uniform,
            vertices,
            indices,
        }
    }
    fn encode(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::RenderPipeline,
        effect: DrawEffects,
    ) {
        queue.write_buffer(&self.uniform, 0, &effect.uniform_bytes());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("image filter pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.base, &[]);
        pass.set_bind_group(1, &self.effects, &[0]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..6, 0, 0..1);
    }
}
#[derive(Default)]
struct Chain {
    passes: Vec<Pass>,
}
impl Chain {
    fn prepare(
        &mut self,
        resources: &GpuResources<'_>,
        source: &wgpu::Texture,
        plan: &FilterPlan,
    ) -> bool {
        let changed = self.passes.len() != plan.steps().len()
            || self
                .passes
                .iter()
                .zip(plan.steps())
                .any(|(p, s)| p.size != s.size);
        if changed {
            self.passes.clear();
            for step in plan.steps() {
                let input = if step.source == 0 {
                    source
                } else {
                    &self.passes[step.source - 1].target
                };
                let pass = Pass::new(resources, input, source, step.size);
                self.passes.push(pass);
            }
        }
        changed
    }
    fn encode(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::RenderPipeline,
        plan: &FilterPlan,
    ) {
        for (pass, step) in self.passes.iter().zip(plan.steps()) {
            pass.encode(
                queue,
                encoder,
                pipeline,
                DrawEffects {
                    filter: step.filter,
                    ..Default::default()
                },
            );
        }
    }
}
pub struct Gallery {
    scene: Pass,
    capture: wgpu::Texture,
    blur: Chain,
    bloom: Chain,
    registered: bool,
}
impl Gallery {
    pub fn new(resources: &GpuResources<'_>) -> Self {
        let device = resources.device;
        let queue = resources.queue;
        let fixture = texture(device, SIZE);
        let mut pixels = vec![0; SIZE[0] as usize * SIZE[1] as usize * 4];
        for y in 0..SIZE[1] {
            for x in 0..SIZE[0] {
                let index = ((y * SIZE[0] + x) * 4) as usize;
                let tile = (x / 32 + y / 32) % 2;
                let bright = (x as i32 - 310).pow(2) + (y as i32 - 100).pow(2) < 400
                    || x % 97 < 3
                    || y % 73 < 2;
                pixels[index..index + 4].copy_from_slice(if bright {
                    &[245, 216, 145, 255]
                } else if tile == 0 {
                    &[28, 38, 80, 255]
                } else {
                    &[98, 67, 160, 255]
                });
            }
        }
        queue.write_texture(
            fixture.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE[0] * 4),
                rows_per_image: Some(SIZE[1]),
            },
            fixture.size(),
        );
        let scene = Pass::new(resources, &fixture, &fixture, SIZE);
        Self {
            scene,
            capture: texture(device, SIZE),
            blur: Chain::default(),
            bloom: Chain::default(),
            registered: false,
        }
    }
    pub fn prepare(
        &mut self,
        resources: &GpuResources<'_>,
        pipeline: &wgpu::RenderPipeline,
        phase: f32,
        blur: Blur,
        threshold: f32,
        intensity: f32,
    ) -> Result<Vec<(TextureId, wgpu::Texture)>, restricted_gui::effects::EffectError> {
        let device = resources.device;
        let queue = resources.queue;
        let blur_plan = FilterPlan::blur(SIZE, blur)?;
        let bloom_plan = FilterPlan::bloom(SIZE, blur, threshold, intensity)?;
        let blur_changed = self.blur.prepare(resources, &self.capture, &blur_plan);
        let bloom_changed = self.bloom.prepare(resources, &self.capture, &bloom_plan);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gallery scene / snapshot / blur / bloom"),
        });
        self.scene.encode(
            queue,
            &mut encoder,
            pipeline,
            DrawEffects {
                filter: ImageFilter {
                    overwrite: 1,
                    ..Default::default()
                },
                surface: SurfaceStyle {
                    flags: SURFACE_DISTORT | SURFACE_SWEEP,
                    phase,
                    rect: [0., 0., SIZE[0] as f32, SIZE[1] as f32],
                    distortion: 8.,
                    shimmer: 0.15,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        encoder.copy_texture_to_texture(
            self.scene.target.as_image_copy(),
            self.capture.as_image_copy(),
            self.capture.size(),
        );
        self.blur.encode(queue, &mut encoder, pipeline, &blur_plan);
        self.bloom
            .encode(queue, &mut encoder, pipeline, &bloom_plan);
        queue.submit([encoder.finish()]);
        let mut updates = Vec::new();
        if !self.registered {
            updates.push((SCENE, self.capture.clone()));
        }
        if blur_changed {
            if let Some(pass) = self.blur.passes.last() {
                updates.push((BLURRED, pass.target.clone()));
            }
        }
        if bloom_changed {
            if let Some(pass) = self.bloom.passes.last() {
                updates.push((BLOOM, pass.target.clone()));
            }
        }
        self.registered = true;
        Ok(updates)
    }
}
