use super::*;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R8G8B8A8_UNORM_SRGB;

#[derive(Clone, Copy)]
pub(super) struct PsoConfig {
    pub samples: u32,
    pub alpha: AlphaMode,
    pub instanced: bool,
}
impl Default for PsoConfig {
    fn default() -> Self {
        Self {
            samples: 1,
            alpha: AlphaMode::Straight,
            instanced: false,
        }
    }
}
pub(super) struct RasterSet {
    pub samples: u32,
    pub alpha: AlphaMode,
    pipelines: [ID3D12PipelineState; 24],
}
impl RasterSet {
    pub(super) fn pipeline(
        &self,
        color: crate::ColorMode,
        instance: bool,
        mode: usize,
    ) -> &ID3D12PipelineState {
        &self.pipelines
            [usize::from(color == crate::ColorMode::Srgb) * 12 + usize::from(instance) * 6 + mode]
    }
}
pub(super) fn texture_alpha(modes: &[(TextureId, AlphaMode)], id: TextureId) -> AlphaMode {
    modes
        .iter()
        .find(|(key, _)| *key == id)
        .map_or(AlphaMode::Straight, |(_, mode)| *mode)
}
pub(super) fn instance_bytes(values: &[tacita::rendering::QuadInstance]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), core::mem::size_of_val(values))
    }
}
fn build_raster(
    device: &Device,
    root: &ID3D12RootSignature,
    sources: (&str, &str),
    samples: u32,
    alpha: AlphaMode,
) -> Result<RasterSet, BackendError> {
    let mut pipelines = Vec::with_capacity(24);
    for managed in [false, true] {
        let vs = format!(
            "#define GUI_MANAGED_SRGB {}\n{}",
            u8::from(managed),
            sources.0
        );
        let ps = format!(
            "#define GUI_MANAGED_SRGB {}\n{}",
            u8::from(managed),
            sources.1
        );
        let pixel = shader::compile(&ps, shader::PS_ENTRY, shader::PS_TARGET)?;
        for instanced in [false, true] {
            let vertex = shader::compile(
                &vs,
                if instanced {
                    c"quad_main"
                } else {
                    shader::VS_ENTRY
                },
                shader::VS_TARGET,
            )?;
            for mode in 0..6 {
                pipelines.push(create_pso(
                    device,
                    root,
                    mode,
                    &vertex,
                    &pixel,
                    managed,
                    PsoConfig {
                        samples,
                        alpha,
                        instanced,
                    },
                )?);
            }
        }
    }
    let pipelines = pipelines
        .try_into()
        .map_err(|_| BackendError::ShaderCompile("incomplete raster set".into()))?;
    Ok(RasterSet {
        samples,
        alpha,
        pipelines,
    })
}
pub(crate) fn check_samples(device: &Device, samples: u32) -> Result<(), BackendError> {
    if !tacita::rendering::valid_sample_count(samples) {
        return Err(BackendError::InvalidTextureData(
            "sample count must be 1, 2, 4 or 8".into(),
        ));
    }
    for format in [
        DXGI_FORMAT_R8G8B8A8_UNORM,
        DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
        DXGI_FORMAT_D24_UNORM_S8_UINT,
    ] {
        let mut quality = D3D12_FEATURE_DATA_MULTISAMPLE_QUALITY_LEVELS {
            Format: format,
            SampleCount: samples,
            ..Default::default()
        };
        unsafe {
            device.CheckFeatureSupport(
                D3D12_FEATURE_MULTISAMPLE_QUALITY_LEVELS,
                (&mut quality as *mut D3D12_FEATURE_DATA_MULTISAMPLE_QUALITY_LEVELS).cast(),
                core::mem::size_of_val(&quality) as u32,
            )
        }
        .map_err(BackendError::TextureCreation)?;
        if quality.NumQualityLevels == 0 {
            return Err(BackendError::InvalidTextureData(
                "unsupported color/stencil sample count".into(),
            ));
        }
    }
    Ok(())
}
impl Pipeline {
    pub(crate) fn draw_optimization(&self) -> DrawOptimization {
        self.optimization
    }
    pub(crate) fn set_draw_optimization(&mut self, options: DrawOptimization) {
        self.optimization = options;
    }
    pub(crate) fn set_alpha_mode(&mut self, mode: AlphaMode) {
        self.alpha_mode = mode;
    }
    pub(crate) fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
    pub(crate) fn set_texture_alpha_mode(
        &mut self,
        id: TextureId,
        mode: AlphaMode,
    ) -> Result<(), BackendError> {
        if id.0 < 2 || self.textures.srv_for(id).is_none() {
            return Err(BackendError::InvalidTextureData(
                "alpha mode needs a registered host texture".into(),
            ));
        }
        self.texture_alpha.retain(|(key, _)| *key != id);
        if mode != AlphaMode::Straight {
            self.texture_alpha.push((id, mode));
        }
        Ok(())
    }
    pub(crate) fn ensure_raster(&mut self, samples: u32) -> Result<usize, BackendError> {
        if let Some(index) = self
            .rasters
            .iter()
            .position(|set| set.samples == samples && set.alpha == self.alpha_mode)
        {
            return Ok(index);
        }
        check_samples(&self.device, samples)?;
        let set = build_raster(
            &self.device,
            &self.root_signature,
            (&self.sources.0, &self.sources.1),
            samples,
            self.alpha_mode,
        )?;
        self.rasters.push(set);
        Ok(self.rasters.len() - 1)
    }
    pub(crate) fn reload_shaders(
        &mut self,
        sync: &FrameSync,
        vertex: &str,
        pixel: &str,
    ) -> Result<(), BackendError> {
        if vertex.len() > 1024 * 1024 || pixel.len() > 1024 * 1024 {
            return Err(BackendError::ShaderCompile("source exceeds 1 MiB".into()));
        }
        shader::interface::validate(vertex, pixel)?;
        let base = build_raster(
            &self.device,
            &self.root_signature,
            (vertex, pixel),
            1,
            AlphaMode::Straight,
        )?;
        let mut sets = Vec::with_capacity(self.rasters.len());
        for old in &self.rasters {
            sets.push(build_raster(
                &self.device,
                &self.root_signature,
                (vertex, pixel),
                old.samples,
                old.alpha,
            )?);
        }
        let fence = sync.last_signaled_value();
        let old = core::mem::replace(&mut self.pso, base.pipelines[0].clone());
        self.retired.push_back(Retired {
            fence,
            kind: RetiredKind::PipelineState(old),
        });
        for (i, old) in self.variants.iter_mut().enumerate() {
            let old = core::mem::replace(old, base.pipelines[i + 1].clone());
            self.retired.push_back(Retired {
                fence,
                kind: RetiredKind::PipelineState(old),
            });
        }
        for (i, old) in self.managed.iter_mut().enumerate() {
            let old = core::mem::replace(old, base.pipelines[i + 12].clone());
            self.retired.push_back(Retired {
                fence,
                kind: RetiredKind::PipelineState(old),
            });
        }
        for old in core::mem::replace(&mut self.rasters, sets) {
            for pso in old.pipelines {
                self.retired.push_back(Retired {
                    fence,
                    kind: RetiredKind::PipelineState(pso),
                });
            }
        }
        self.sources = (vertex.into(), pixel.into());
        Ok(())
    }
}
