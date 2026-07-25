use windows::core::PCSTR;
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::ID3DBlob;

use crate::BackendError;

// Shader source embedded as a string and compiled at startup. Runtime shader compilation is normally a sin; here it is one less file to lose inside an injected process.
pub(crate) const VS_ENTRY: &std::ffi::CStr = c"main";
pub(crate) const VS_TARGET: &std::ffi::CStr = c"vs_4_0";
pub(crate) const PS_ENTRY: &std::ffi::CStr = c"main";
pub(crate) const PS_TARGET: &std::ffi::CStr = c"ps_4_0";

pub(crate) const VERTEX_SHADER_SOURCE: &str = r"
cbuffer vertexBuffer : register(b0)
{
    float4x4 ProjectionMatrix;
};

struct VS_INPUT
{
    float2 pos : POSITION;
    float2 uv  : TEXCOORD0;
    float4 col : COLOR0;
};

struct PS_INPUT
{
    float4 pos : SV_POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
};

PS_INPUT main(VS_INPUT input)
{
    PS_INPUT output;
    output.pos = mul(ProjectionMatrix, float4(input.pos.xy, 0.0f, 1.0f));
    output.col = input.col;
    output.uv  = input.uv;
    return output;
}
";

pub(crate) const PIXEL_SHADER_SOURCE: &str = r"
struct PS_INPUT
{
    float4 pos : SV_POSITION;
    float4 col : COLOR0;
    float2 uv  : TEXCOORD0;
};

sampler sampler0 : register(s0);
Texture2D texture0 : register(t0);

float4 main(PS_INPUT input) : SV_Target
{
    return input.col * texture0.Sample(sampler0, input.uv);
}
";

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
            let detail = shader_error_text(errors);
            if detail.is_empty() {
                Err(BackendError::ShaderCompile(e.to_string()))
            } else {
                Err(BackendError::ShaderCompile(detail))
            }
        }
    }
}

fn shader_error_text(errors: Option<ID3DBlob>) -> String {
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
