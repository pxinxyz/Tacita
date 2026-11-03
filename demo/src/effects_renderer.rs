use restricted_gui::{
    draw::{DrawData, TextureId},
    effects::{DrawEffects, EFFECT_UNIFORM_SIZE},
};

pub struct EffectsRenderer {
    pub layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    capacity: u64,
    stride: u64,
    bank_size: u32,
    atlas: wgpu::TextureView,
    textures: Vec<(u64, wgpu::TextureView)>,
    groups: Vec<([u64; 4], wgpu::BindGroup)>,
    prepared: Vec<usize>,
    upload: Vec<u8>,
}
impl EffectsRenderer {
    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let entries: [wgpu::BindGroupLayoutEntry; 5] =
            core::array::from_fn(|i| wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: if i == 0 {
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(EFFECT_UNIFORM_SIZE as u64),
                    }
                } else {
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    }
                },
                count: None,
            });
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gui effects layout"),
            entries: &entries,
        })
    }
    pub fn new(
        device: &wgpu::Device,
        layout: wgpu::BindGroupLayout,
        atlas: &wgpu::Texture,
    ) -> Self {
        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let stride = (EFFECT_UNIFORM_SIZE as u64).div_ceil(alignment) * alignment;
        let capacity = stride * 64;
        Self {
            layout,
            buffer: Self::buffer(device, capacity),
            capacity,
            stride,
            bank_size: 1,
            atlas: atlas.create_view(&Default::default()),
            textures: Vec::new(),
            groups: Vec::new(),
            prepared: Vec::new(),
            upload: Vec::new(),
        }
    }
    fn buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gui effect records"),
            size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
    pub fn set_atlas(&mut self, atlas: &wgpu::Texture) {
        self.atlas = atlas.create_view(&Default::default());
        self.groups.clear();
    }
    pub fn add_texture(&mut self, id: TextureId, texture: &wgpu::Texture) {
        self.textures.retain(|(key, _)| *key != id.0);
        self.textures
            .push((id.0, texture.create_view(&Default::default())));
        self.groups.clear();
    }
    fn view(&self, id: u64) -> Option<&wgpu::TextureView> {
        if id <= 1 {
            Some(&self.atlas)
        } else {
            self.textures
                .iter()
                .find(|(key, _)| *key == id)
                .map(|(_, view)| view)
        }
    }
    pub fn prepare_alpha(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draw: &DrawData,
        alpha: bool,
    ) {
        self.bank_size = draw.effects.len() as u32 + 1;
        let banks = if alpha { 8 } else { 1 };
        let required = (draw.effects.len() as u64 + 1) * self.stride * banks;
        if required > self.capacity {
            self.capacity = required.next_power_of_two();
            self.buffer = Self::buffer(device, self.capacity);
            self.groups.clear();
        }
        self.prepared.clear();
        self.upload.resize(required as usize, 0);
        for (i, (flags, effect)) in (0..banks)
            .flat_map(|flags| {
                core::iter::once(DrawEffects::default())
                    .chain(draw.effects.iter().copied())
                    .map(move |effect| (flags, effect))
            })
            .enumerate()
        {
            let start = i * self.stride as usize;
            self.upload[start..start + EFFECT_UNIFORM_SIZE]
                .copy_from_slice(&effect.uniform_bytes_with_alpha(flags as u32));
            let keys = core::array::from_fn(|j| effect.mask_texture(j).map_or(0, |id| id.0));
            let group = if let Some(index) = self.groups.iter().position(|(key, _)| *key == keys) {
                index
            } else {
                let views = keys.map(|id| self.view(id));
                let [Some(a), Some(b), Some(c), Some(d)] = views else {
                    eprintln!("[demo] skipping unregistered mask combination {keys:?}");
                    self.prepared.push(usize::MAX);
                    continue;
                };
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("gui effect textures"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &self.buffer,
                                offset: 0,
                                size: wgpu::BufferSize::new(EFFECT_UNIFORM_SIZE as u64),
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(a),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(b),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(c),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::TextureView(d),
                        },
                    ],
                });
                self.groups.push((keys, bind_group));
                self.groups.len() - 1
            };
            self.prepared.push(group);
        }
        queue.write_buffer(&self.buffer, 0, &self.upload);
    }
    pub fn binding_alpha(&self, index: u32, flags: u32) -> Option<(&wgpu::BindGroup, u32)> {
        let index = index.checked_add(flags.checked_mul(self.bank_size)?)?;
        let group = *self.prepared.get(index as usize)?;
        let offset = u32::try_from(index as u64 * self.stride).ok()?;
        self.groups.get(group).map(|(_, binding)| (binding, offset))
    }
}
