use super::*;

pub(super) struct RasterBundle {
    surface: [[wgpu::RenderPipeline; 6]; 2],
    pub offscreen: [[[wgpu::RenderPipeline; 6]; 2]; 2],
    image: wgpu::RenderPipeline,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_variants_compile_and_shader_errors_are_returned() {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(
                wgpu::InstanceDescriptor::new_without_display_handle_from_env(),
            );
            let adapter = instance.request_adapter(&Default::default()).await.unwrap();
            let (device, _) = adapter.request_device(&Default::default()).await.unwrap();
            let base = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("test projection and texture"),
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
            let effects = crate::effects_renderer::EffectsRenderer::layout(&device);
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("test GUI pipeline"),
                bind_group_layouts: &[Some(&base), Some(&effects)],
                immediate_size: 0,
            });
            Renderer::build_raster(
                &device,
                &layout,
                wgpu::TextureFormat::Bgra8Unorm,
                Renderer::builtin_shader(),
                1,
                AlphaMode::Straight,
            )
            .await
            .unwrap();
            let invalid = format!("{}\ninvalid shader syntax", Renderer::builtin_shader());
            assert!(Renderer::build_raster(
                &device,
                &layout,
                wgpu::TextureFormat::Bgra8Unorm,
                &invalid,
                1,
                AlphaMode::Straight
            )
            .await
            .is_err());
        });
    }
}
pub(super) fn instance_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GUI quad instances"),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
impl Renderer {
    pub async fn configure_rendering(
        &mut self,
        samples: u32,
        alpha: AlphaMode,
        optimization: DrawOptimization,
    ) -> Result<(), String> {
        if ![1, 2, 4, 8]
            .into_iter()
            .zip(self.supported_samples)
            .any(|(count, available)| count == samples && available)
        {
            return Err(format!(
                "{samples}x MSAA is unavailable for the color/stencil pair"
            ));
        }
        if self.sample_count != samples
            || self.alpha_mode != alpha
            || (optimization.instance_quads && self.raster.is_none())
        {
            let candidate = Self::build_raster(
                &self.device,
                &self.pipeline_layout,
                self.config.format,
                &self.shader_source,
                samples,
                alpha,
            )
            .await?;
            self.raster = Some(candidate);
        }
        self.sample_count = samples;
        self.alpha_mode = alpha;
        self.optimization = optimization;
        Ok(())
    }
    async fn build_raster(
        device: &wgpu::Device,
        pipeline_layout: &wgpu::PipelineLayout,
        surface_format: wgpu::TextureFormat,
        source: &str,
        samples: u32,
        alpha: AlphaMode,
    ) -> Result<RasterBundle, String> {
        if source.len() > 1024 * 1024 {
            return Err("shader source exceeds 1 MiB".into());
        }
        if source.matches("const GUI_MANAGED_SRGB:bool=false;").count() != 1 {
            return Err("shader must retain the renderer's GUI_MANAGED_SRGB declaration".into());
        }
        let started = std::time::Instant::now();
        eprintln!("[renderer] building raster variants: {samples}x, {alpha:?}");
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("reload GUI shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let managed = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("reload managed GUI shader"),
            source: wgpu::ShaderSource::Wgsl(
                source
                    .replace(
                        "const GUI_MANAGED_SRGB:bool=false;",
                        "const GUI_MANAGED_SRGB:bool=true;",
                    )
                    .into(),
            ),
        });
        let pipeline = |shader, format, mode, count, instanced| {
            eprintln!(
                "[renderer] pipeline: {format:?}, mode {mode}, {count}x, instanced {instanced}"
            );
            create_raster_pipeline(
                device,
                pipeline_layout,
                shader,
                format,
                mode,
                count,
                alpha,
                instanced,
            )
        };
        let surface = core::array::from_fn(|instance| {
            core::array::from_fn(|mode| {
                pipeline(&shader, surface_format, mode, samples, instance != 0)
            })
        });
        let offscreen = core::array::from_fn(|color| {
            core::array::from_fn(|instance| {
                core::array::from_fn(|mode| {
                    pipeline(
                        if color == 0 { &shader } else { &managed },
                        if color == 0 {
                            wgpu::TextureFormat::Rgba8Unorm
                        } else {
                            wgpu::TextureFormat::Rgba8UnormSrgb
                        },
                        mode,
                        1,
                        instance != 0,
                    )
                })
            })
        });
        let image = create_effect_pipeline(
            device,
            pipeline_layout,
            &shader,
            wgpu::TextureFormat::Rgba8Unorm,
            1,
        );
        if let Some(error) = scope.pop().await {
            eprintln!(
                "[renderer] raster rejected after {:?}: {error}",
                started.elapsed()
            );
            return Err(error.to_string());
        }
        eprintln!("[renderer] raster ready after {:?}", started.elapsed());
        Ok(RasterBundle {
            surface,
            offscreen,
            image,
        })
    }
    pub async fn reload_shader(&mut self, source: &str) -> Result<(), String> {
        let candidate = Self::build_raster(
            &self.device,
            &self.pipeline_layout,
            self.config.format,
            source,
            self.sample_count,
            self.alpha_mode,
        )
        .await?;
        self.image_pipeline = candidate.image.clone();
        self.raster = Some(candidate);
        self.shader_source = source.into();
        Ok(())
    }
    pub fn builtin_shader() -> &'static str {
        SHADER
    }
    pub fn supported_samples(&self) -> [bool; 4] {
        self.supported_samples
    }
    pub fn optimization_stats(&self) -> (u32, usize) {
        if self.optimization.enabled() {
            (self.plan.reordered_commands(), self.plan.instances().len())
        } else {
            (0, 0)
        }
    }
    pub fn set_texture_alpha_mode(&mut self, id: TextureId, mode: AlphaMode) -> Result<(), String> {
        if id.0 < 2 || self.bind_group_for(id).is_none() {
            return Err("alpha mode needs a registered host texture".into());
        }
        self.texture_alpha.retain(|(key, _)| *key != id);
        if mode != AlphaMode::Straight {
            self.texture_alpha.push((id, mode));
        }
        Ok(())
    }
    fn texture_alpha_mode(&self, id: TextureId) -> AlphaMode {
        let id = self
            .preview_aliases
            .iter()
            .find(|(alias, _)| *alias == id)
            .map_or(id, |(_, source)| *source);
        self.texture_alpha
            .iter()
            .find(|(key, _)| *key == id)
            .map_or(AlphaMode::Straight, |(_, mode)| *mode)
    }
    pub(super) fn command_alpha(
        &self,
        texture: TextureId,
        effect: restricted_gui::effects::DrawEffects,
    ) -> u32 {
        restricted_gui::rendering::alpha_flags(
            self.texture_alpha_mode(texture),
            self.alpha_mode,
            self.texture_alpha_mode(effect.mask_texture(3).unwrap_or(TextureId::WHITE)),
        )
    }
    pub(super) fn needs_alpha(&self, draw: &DrawData) -> bool {
        self.alpha_mode != AlphaMode::Straight
            || draw
                .commands
                .iter()
                .any(|cmd| self.texture_alpha_mode(cmd.texture) != AlphaMode::Straight)
            || draw.effects.iter().any(|e| {
                self.texture_alpha_mode(e.mask_texture(3).unwrap_or(TextureId::WHITE))
                    != AlphaMode::Straight
            })
    }
    pub(super) fn surface_pipeline(&self, instanced: bool, mode: usize) -> &wgpu::RenderPipeline {
        if let Some(bundle) = &self.raster {
            &bundle.surface[usize::from(instanced)][mode]
        } else if mode == 0 {
            &self.pipeline
        } else {
            &self.variants[mode - 1]
        }
    }
    pub(super) fn prepare_plan(&mut self, draw: &DrawData, scale: f32) -> Result<(), String> {
        if !self.optimization.enabled() {
            return Ok(());
        }
        self.plan
            .prepare(draw, self.optimization, scale)
            .map_err(|e| format!("draw plan: {e:?}"))?;
        let bytes = core::mem::size_of_val(self.plan.instances()) as u64;
        if bytes > 64 * 1024 * 1024 {
            return Err("quad instance buffer exceeds 64 MiB".into());
        }
        if bytes > self.instance_capacity {
            self.instance_capacity = bytes.next_power_of_two().min(64 * 1024 * 1024);
            self.instance_buffer = instance_buffer(&self.device, self.instance_capacity);
            self.stats.allocations += 1;
        }
        if bytes != 0 {
            let upload = unsafe {
                std::slice::from_raw_parts(
                    self.plan.instances().as_ptr().cast::<u8>(),
                    bytes as usize,
                )
            };
            self.queue.write_buffer(&self.instance_buffer, 0, upload);
        }
        Ok(())
    }
    pub(super) fn ensure_msaa(&mut self) {
        if self.sample_count == 1 {
            self.msaa = None;
            return;
        }
        if self.msaa.as_ref().is_none_or(|(t, _)| {
            t.width() != self.config.width
                || t.height() != self.config.height
                || t.sample_count() != self.sample_count
        }) {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("GUI multisample color"),
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: self.sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            self.msaa = Some((texture, view));
        }
    }
}
