use windows::Win32::Foundation::E_UNEXPECTED;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11RenderTargetView, ID3D11Texture2D, D3D11_BIND_RENDER_TARGET, D3D11_CPU_ACCESS_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};

use crate::{BackendError, Context, Device};

pub struct OffscreenTarget {
    texture: ID3D11Texture2D,
    render_target_view: ID3D11RenderTargetView,
    staging: ID3D11Texture2D,
    width: u32,
    height: u32,
}

impl OffscreenTarget {
    pub fn new(device: &Device, width: u32, height: u32) -> Result<Self, BackendError> {
        let mut desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let texture = create_texture(device, &desc)?;

        let mut rtv = None;
        unsafe { device.CreateRenderTargetView(&texture, None, Some(&mut rtv)) }
            .map_err(BackendError::RenderTargetViewCreation)?;
        let render_target_view = rtv.ok_or_else(|| {
            BackendError::RenderTargetViewCreation(windows::core::Error::from_hresult(E_UNEXPECTED))
        })?;

        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        let staging = create_texture(device, &desc)?;

        Ok(Self {
            texture,
            render_target_view,
            staging,
            width,
            height,
        })
    }

    pub fn rtv(&self) -> &ID3D11RenderTargetView {
        &self.render_target_view
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn clear(&self, context: &Context, color: [f32; 4]) {
        unsafe { context.ClearRenderTargetView(&self.render_target_view, &color) };
    }

    pub fn readback(&self, context: &Context) -> Result<Vec<u8>, BackendError> {
        unsafe {
            context.CopyResource(&self.staging, &self.texture);
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            context
                .Map(&self.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .map_err(BackendError::MapFailed)?;

            let row_bytes = self.width as usize * 4;
            let mut out = vec![0u8; row_bytes * self.height as usize];
            let src = mapped.pData as *const u8;
            for row in 0..self.height as usize {
                let src_row = src.add(row * mapped.RowPitch as usize);
                let dst_row = out.as_mut_ptr().add(row * row_bytes);
                std::ptr::copy_nonoverlapping(src_row, dst_row, row_bytes);
            }
            context.Unmap(&self.staging, 0);
            Ok(out)
        }
    }
}

fn create_texture(
    device: &Device,
    desc: &D3D11_TEXTURE2D_DESC,
) -> Result<ID3D11Texture2D, BackendError> {
    let mut texture = None;
    unsafe { device.CreateTexture2D(desc, None, Some(&mut texture)) }
        .map_err(BackendError::TextureCreation)?;
    texture.ok_or_else(|| {
        BackendError::TextureCreation(windows::core::Error::from_hresult(E_UNEXPECTED))
    })
}
