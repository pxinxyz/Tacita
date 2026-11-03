use super::*;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Texture2D, D3D11_INPUT_PER_INSTANCE_DATA, D3D11_TEXTURE2D_DESC,
};

impl D3d11Backend {
    pub fn set_draw_optimization(&mut self, options: DrawOptimization) {
        self.optimization = options;
    }
    pub fn draw_optimization(&self) -> DrawOptimization {
        self.optimization
    }
    pub fn set_alpha_mode(&mut self, mode: AlphaMode) {
        self.alpha_mode = mode;
    }
    pub fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
    pub fn set_texture_alpha_mode(
        &mut self,
        id: TextureId,
        mode: AlphaMode,
    ) -> Result<(), BackendError> {
        if id.0 < 2 || self.textures.resource_for(id).is_none() {
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
    pub fn texture_alpha_mode(&self, id: TextureId) -> AlphaMode {
        self.texture_alpha
            .iter()
            .find(|(key, _)| *key == id)
            .map_or(AlphaMode::Straight, |(_, mode)| *mode)
    }
    pub fn shader_sources() -> (&'static str, &'static str) {
        (shader::VERTEX_SHADER_SOURCE, shader::PIXEL_SHADER_SOURCE)
    }
    pub fn reload_shaders(&mut self, vertex: &str, pixel: &str) -> Result<(), BackendError> {
        if vertex.len() > 1024 * 1024 || pixel.len() > 1024 * 1024 {
            return Err(BackendError::ShaderCompile("source exceeds 1 MiB".into()));
        }
        shader::interface::validate(vertex, pixel)?;
        let mut candidate = create_pipeline(&self.device, vertex, pixel)?;
        core::mem::swap(
            &mut candidate.vertex_buffer,
            &mut self.pipeline.vertex_buffer,
        );
        core::mem::swap(
            &mut candidate.vertex_capacity,
            &mut self.pipeline.vertex_capacity,
        );
        core::mem::swap(&mut candidate.index_buffer, &mut self.pipeline.index_buffer);
        core::mem::swap(
            &mut candidate.index_capacity,
            &mut self.pipeline.index_capacity,
        );
        core::mem::swap(&mut candidate.quad_buffer, &mut self.pipeline.quad_buffer);
        core::mem::swap(
            &mut candidate.quad_capacity,
            &mut self.pipeline.quad_capacity,
        );
        self.pipeline = candidate;
        Ok(())
    }
    pub(super) fn upload_instances(&mut self) -> Result<(), BackendError> {
        let count = self.plan.instances().len();
        if count == 0 {
            return Ok(());
        }
        let bytes = count
            .checked_mul(80)
            .filter(|v| *v <= 64 * 1024 * 1024)
            .ok_or_else(|| {
                BackendError::InvalidTextureData("instance buffer exceeds 64 MiB".into())
            })?;
        if count > self.pipeline.quad_capacity as usize {
            let capacity = (count as u32)
                .next_power_of_two()
                .min(64 * 1024 * 1024 / 80);
            let buffer =
                create_dynamic_buffer(&self.device, capacity * 80, D3D11_BIND_VERTEX_BUFFER)?;
            self.pipeline.quad_buffer = buffer;
            self.pipeline.quad_capacity = capacity;
            self.stats.allocations += 1;
        }
        unsafe {
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.context
                .Map(
                    &self.pipeline.quad_buffer,
                    0,
                    D3D11_MAP_WRITE_DISCARD,
                    0,
                    Some(&mut mapped),
                )
                .map_err(BackendError::MapFailed)?;
            std::ptr::copy_nonoverlapping(
                self.plan.instances().as_ptr().cast::<u8>(),
                mapped.pData.cast(),
                bytes,
            );
            self.context.Unmap(&self.pipeline.quad_buffer, 0);
        }
        Ok(())
    }
}

pub(super) fn target_samples(target: &ID3D11RenderTargetView) -> Result<u32, BackendError> {
    let texture: ID3D11Texture2D = unsafe { target.GetResource() }
        .map_err(BackendError::TextureCreation)?
        .cast()
        .map_err(BackendError::TextureCreation)?;
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe {
        texture.GetDesc(&mut desc);
    }
    Ok(desc.SampleDesc.Count)
}
pub(super) fn quad_shader(
    device: &Device,
    source: &str,
    managed: bool,
) -> Result<ID3D11VertexShader, BackendError> {
    let source = format!("#define GUI_MANAGED_SRGB {}\n{source}", u8::from(managed));
    let blob = shader::compile(&source, c"quad_main", shader::VS_TARGET)?;
    let bytes = unsafe {
        std::slice::from_raw_parts(blob.GetBufferPointer().cast::<u8>(), blob.GetBufferSize())
    };
    let mut shader = None;
    unsafe { device.CreateVertexShader(bytes, None, Some(&mut shader)) }
        .map_err(|e| BackendError::StateCreation("quad vertex shader", e))?;
    shader.ok_or_else(|| BackendError::ShaderCompile("missing quad vertex shader".into()))
}
pub(super) fn quad_layout(
    device: &Device,
    source: &str,
) -> Result<ID3D11InputLayout, BackendError> {
    let blob = shader::compile(source, c"quad_main", shader::VS_TARGET)?;
    let elements: [D3D11_INPUT_ELEMENT_DESC; 12] = core::array::from_fn(|i| {
        let mut item = INPUT_LAYOUT[i % 3];
        item.SemanticIndex = (i / 3) as u32;
        item.AlignedByteOffset += (i / 3) as u32 * 20;
        item.InputSlotClass = D3D11_INPUT_PER_INSTANCE_DATA;
        item.InstanceDataStepRate = 1;
        item
    });
    let bytes = unsafe {
        std::slice::from_raw_parts(blob.GetBufferPointer().cast::<u8>(), blob.GetBufferSize())
    };
    let mut layout = None;
    unsafe { device.CreateInputLayout(&elements, bytes, Some(&mut layout)) }
        .map_err(|e| BackendError::StateCreation("quad input layout", e))?;
    layout.ok_or_else(|| BackendError::ShaderCompile("missing quad input layout".into()))
}
