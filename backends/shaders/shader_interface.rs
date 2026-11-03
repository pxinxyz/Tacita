use crate::{shader, BackendError};
use std::ffi::CStr;
use windows::{
    core::{Interface, PCSTR},
    Win32::Graphics::{
        Direct3D::{Fxc::D3DReflect, ID3DBlob, D3D_SIT_CBUFFER},
        Direct3D11::*,
    },
};
fn failure() -> BackendError {
    BackendError::ShaderCompile("reload changes the renderer resource/uniform interface".into())
}
fn reflect(blob: &ID3DBlob) -> Result<ID3D11ShaderReflection, BackendError> {
    let mut raw = std::ptr::null_mut();
    unsafe {
        D3DReflect(
            blob.GetBufferPointer(),
            blob.GetBufferSize(),
            &ID3D11ShaderReflection::IID,
            &mut raw,
        )
    }
    .map_err(|e| BackendError::ShaderCompile(e.to_string()))?;
    if raw.is_null() {
        return Err(failure());
    }
    Ok(unsafe { ID3D11ShaderReflection::from_raw(raw) })
}
fn names_equal(a: PCSTR, b: PCSTR) -> bool {
    if a.is_null() || b.is_null() {
        return a.is_null() && b.is_null();
    }
    unsafe { CStr::from_ptr(a.as_ptr().cast()) == CStr::from_ptr(b.as_ptr().cast()) }
}
fn types_equal(
    a: ID3D11ShaderReflectionType,
    b: ID3D11ShaderReflectionType,
    depth: u32,
) -> Result<bool, windows::core::Error> {
    if depth > 16 {
        return Ok(false);
    }
    let mut x = D3D11_SHADER_TYPE_DESC::default();
    let mut y = D3D11_SHADER_TYPE_DESC::default();
    unsafe {
        a.GetDesc(&mut x)?;
        b.GetDesc(&mut y)?;
    }
    if x.Class != y.Class
        || x.Type != y.Type
        || x.Rows != y.Rows
        || x.Columns != y.Columns
        || x.Elements != y.Elements
        || x.Members != y.Members
        || x.Offset != y.Offset
        || x.Members > 256
    {
        return Ok(false);
    }
    for i in 0..x.Members {
        let Some(at) = (unsafe { a.GetMemberTypeByIndex(i) }) else {
            return Ok(false);
        };
        let Some(bt) = (unsafe { b.GetMemberTypeByIndex(i) }) else {
            return Ok(false);
        };
        if !names_equal(unsafe { a.GetMemberTypeName(i) }, unsafe {
            b.GetMemberTypeName(i)
        }) || !types_equal(at, bt, depth + 1)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}
fn compare(reference: &ID3DBlob, candidate: &ID3DBlob) -> Result<(), BackendError> {
    let reference = reflect(reference)?;
    let candidate = reflect(candidate)?;
    let mut desc = D3D11_SHADER_DESC::default();
    unsafe { candidate.GetDesc(&mut desc) }
        .map_err(|e| BackendError::ShaderCompile(e.to_string()))?;
    if desc.BoundResources > 128 {
        return Err(failure());
    }
    for index in 0..desc.BoundResources {
        let mut new = D3D11_SHADER_INPUT_BIND_DESC::default();
        let mut old = D3D11_SHADER_INPUT_BIND_DESC::default();
        unsafe { candidate.GetResourceBindingDesc(index, &mut new) }
            .map_err(|e| BackendError::ShaderCompile(e.to_string()))?;
        unsafe { reference.GetResourceBindingDescByName(new.Name, &mut old) }
            .map_err(|_| failure())?;
        if new.Type != old.Type
            || new.BindPoint != old.BindPoint
            || new.BindCount != old.BindCount
            || new.Dimension != old.Dimension
            || new.ReturnType != old.ReturnType
        {
            return Err(failure());
        }
        if new.Type != D3D_SIT_CBUFFER {
            continue;
        }
        let new_buffer =
            unsafe { candidate.GetConstantBufferByName(new.Name) }.ok_or_else(failure)?;
        let old_buffer =
            unsafe { reference.GetConstantBufferByName(old.Name) }.ok_or_else(failure)?;
        let mut n = D3D11_SHADER_BUFFER_DESC::default();
        let mut o = D3D11_SHADER_BUFFER_DESC::default();
        unsafe { new_buffer.GetDesc(&mut n) }
            .map_err(|e| BackendError::ShaderCompile(e.to_string()))?;
        unsafe { old_buffer.GetDesc(&mut o) }
            .map_err(|e| BackendError::ShaderCompile(e.to_string()))?;
        if n.Size != o.Size || n.Variables != o.Variables || n.Type != o.Type || n.Variables > 256 {
            return Err(failure());
        }
        for i in 0..n.Variables {
            let nv = unsafe { new_buffer.GetVariableByIndex(i) }.ok_or_else(failure)?;
            let mut nd = D3D11_SHADER_VARIABLE_DESC::default();
            unsafe { nv.GetDesc(&mut nd) }
                .map_err(|e| BackendError::ShaderCompile(e.to_string()))?;
            let ov = unsafe { old_buffer.GetVariableByName(nd.Name) }.ok_or_else(failure)?;
            let mut od = D3D11_SHADER_VARIABLE_DESC::default();
            unsafe { ov.GetDesc(&mut od) }
                .map_err(|e| BackendError::ShaderCompile(e.to_string()))?;
            if nd.StartOffset != od.StartOffset || nd.Size != od.Size {
                return Err(failure());
            }
            if !types_equal(
                unsafe { nv.GetType() }.ok_or_else(failure)?,
                unsafe { ov.GetType() }.ok_or_else(failure)?,
                0,
            )
            .map_err(|e| BackendError::ShaderCompile(e.to_string()))?
            {
                return Err(failure());
            }
        }
    }
    Ok(())
}
pub(crate) fn validate(vertex: &str, pixel: &str) -> Result<(), BackendError> {
    for managed in [false, true] {
        let old_vertex = format!(
            "#define GUI_MANAGED_SRGB {}\n{}",
            u8::from(managed),
            shader::VERTEX_SHADER_SOURCE
        );
        let new_vertex = format!("#define GUI_MANAGED_SRGB {}\n{vertex}", u8::from(managed));
        for entry in [shader::VS_ENTRY, c"quad_main"] {
            compare(
                &shader::compile(&old_vertex, entry, shader::VS_TARGET)?,
                &shader::compile(&new_vertex, entry, shader::VS_TARGET)?,
            )?;
        }
        let old_pixel = format!(
            "#define GUI_MANAGED_SRGB {}\n{}",
            u8::from(managed),
            shader::PIXEL_SHADER_SOURCE
        );
        let new_pixel = format!("#define GUI_MANAGED_SRGB {}\n{pixel}", u8::from(managed));
        compare(
            &shader::compile(&old_pixel, shader::PS_ENTRY, shader::PS_TARGET)?,
            &shader::compile(&new_pixel, shader::PS_ENTRY, shader::PS_TARGET)?,
        )?;
    }
    Ok(())
}
