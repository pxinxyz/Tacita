use windows::core::PCSTR;
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::ID3DBlob;

use crate::BackendError;
#[path = "../../shaders/shader_interface.rs"]
pub(crate) mod interface;

pub(crate) const VS_ENTRY: &std::ffi::CStr = c"main";
pub(crate) const VS_TARGET: &std::ffi::CStr = c"vs_4_0";
pub(crate) const PS_ENTRY: &std::ffi::CStr = c"main";
pub(crate) const PS_TARGET: &std::ffi::CStr = c"ps_4_0";

pub(crate) const VERTEX_SHADER_SOURCE: &str = include_str!("../../shaders/gui.vert.hlsl");

pub(crate) const PIXEL_SHADER_SOURCE: &str = concat!(
    include_str!("../../shaders/blue_noise.hlsl"),
    include_str!("../../shaders/gui.frag.hlsl"),
    include_str!("../../shaders/surface.hlsl")
);

pub(crate) fn compile(
    source: &str,
    entry: &std::ffi::CStr,
    target: &std::ffi::CStr,
) -> Result<ID3DBlob, BackendError> {
    let mut blob = None;
    let mut errors = None;
    let result = unsafe {
        D3DCompile(
            source.as_ptr().cast(),
            source.len(),
            PCSTR::null(),
            None,
            None,
            PCSTR::from_raw(entry.as_ptr().cast()),
            PCSTR::from_raw(target.as_ptr().cast()),
            0,
            0,
            &mut blob,
            Some(&mut errors),
        )
    };
    match result {
        Ok(()) => blob.ok_or_else(|| {
            BackendError::ShaderCompile("D3DCompile succeeded without a blob".to_string())
        }),
        Err(e) => {
            let detail = error_blob_text(errors);
            if detail.is_empty() {
                Err(BackendError::ShaderCompile(e.to_string()))
            } else {
                Err(BackendError::ShaderCompile(detail))
            }
        }
    }
}

pub(crate) fn error_blob_text(errors: Option<ID3DBlob>) -> String {
    let Some(blob) = errors else {
        return String::new();
    };
    unsafe {
        let ptr = blob.GetBufferPointer().cast::<u8>();
        let len = blob.GetBufferSize();
        if ptr.is_null() || len == 0 {
            return String::new();
        }
        let bytes = std::slice::from_raw_parts(ptr, len);
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(len);
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    }
}
