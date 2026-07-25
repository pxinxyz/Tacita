use std::collections::HashMap;

use restricted_gui::draw::TextureId;
use windows::Win32::Foundation::E_UNEXPECTED;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11ShaderResourceView, ID3D11Texture2D,
    D3D11_BIND_SHADER_RESOURCE, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE,
    D3D11_USAGE_DEFAULT, D3D11_USAGE_IMMUTABLE,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};

use crate::BackendError;

struct Texture {
    texture: ID3D11Texture2D,
    srv: ID3D11ShaderResourceView,
    width: u32,
    height: u32,
}

fn create_texture(
    device: &ID3D11Device,
    rgba: &[u8],
    width: u32,
    height: u32,
    usage: D3D11_USAGE,
) -> Result<Texture, BackendError> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: usage,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };
    let data = D3D11_SUBRESOURCE_DATA {
        pSysMem: rgba.as_ptr().cast(),
        SysMemPitch: width * 4,
        SysMemSlicePitch: 0,
    };
    let mut texture = None;
    unsafe { device.CreateTexture2D(&desc, Some(&data), Some(&mut texture)) }
        .map_err(BackendError::TextureCreation)?;
    let texture = texture.ok_or_else(|| {
        BackendError::TextureCreation(windows::core::Error::from_hresult(E_UNEXPECTED))
    })?;

    let mut srv = None;
    unsafe { device.CreateShaderResourceView(&texture, None, Some(&mut srv)) }
        .map_err(|e| BackendError::StateCreation("shader resource view", e))?;
    let srv = srv.ok_or_else(|| {
        BackendError::StateCreation(
            "shader resource view",
            windows::core::Error::from_hresult(E_UNEXPECTED),
        )
    })?;

    Ok(Texture {
        texture,
        srv,
        width,
        height,
    })
}

pub(crate) struct TextureSet {
    atlas: Texture,
    host: HashMap<u64, Texture>,
}

impl TextureSet {
    pub(crate) fn new(device: &ID3D11Device) -> Result<Self, BackendError> {
        let atlas = create_texture(device, &[255, 255, 255, 255], 1, 1, D3D11_USAGE_DEFAULT)?;
        Ok(Self {
            atlas,
            host: HashMap::new(),
        })
    }

    // Atlas re-upload happens only when the generation counter bumps. Blindly re-uploading every frame is how you benchmark 12 FPS and wonder why.
    pub(crate) fn upload_atlas(
        &mut self,
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), BackendError> {
        let expected = width as usize * height as usize;
        if width == 0 || height == 0 || pixels.len() != expected {
            return Err(BackendError::InvalidTextureData(format!(
                "atlas pixel data is {} bytes, expected width * height = \
                 {width} * {height} = {expected}",
                pixels.len()
            )));
        }
        let mut rgba = Vec::with_capacity(expected * 4);
        for &a in pixels {
            rgba.extend_from_slice(&[255, 255, 255, a]);
        }
        if self.atlas.width == width && self.atlas.height == height {
            unsafe {
                context.UpdateSubresource(
                    &self.atlas.texture,
                    0,
                    None,
                    rgba.as_ptr().cast(),
                    width * 4,
                    0,
                );
            }
        } else {
            self.atlas = create_texture(device, &rgba, width, height, D3D11_USAGE_DEFAULT)?;
        }
        Ok(())
    }

    pub(crate) fn add_host_texture(
        &mut self,
        device: &ID3D11Device,
        id: TextureId,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), BackendError> {
        if id == TextureId::FONT || id == TextureId::WHITE {
            return Err(BackendError::ReservedTextureId(id.0));
        }
        let expected = width as usize * height as usize * 4;
        if width == 0 || height == 0 || rgba.len() != expected {
            return Err(BackendError::InvalidTextureData(format!(
                "host texture data is {} bytes, expected width * height * 4 = \
                 {width} * {height} * 4 = {expected}",
                rgba.len()
            )));
        }
        let texture = create_texture(device, rgba, width, height, D3D11_USAGE_IMMUTABLE)?;
        self.host.insert(id.0, texture);
        Ok(())
    }

    pub(crate) fn srv_for(&self, id: TextureId) -> Option<&ID3D11ShaderResourceView> {
        match id {
            TextureId::FONT | TextureId::WHITE => Some(&self.atlas.srv),
            _ => self.host.get(&id.0).map(|t| &t.srv),
        }
    }
}
