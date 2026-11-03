use std::path::PathBuf;
use std::time::Duration;

pub struct FrameCapture {
    buffer: wgpu::Buffer,
    path: PathBuf,
    width: u32,
    height: u32,
    stride: u32,
    bgra: bool,
}

impl FrameCapture {
    pub fn encode(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        path: PathBuf,
    ) -> Self {
        let width = texture.width();
        let height = texture.height();
        let stride = (width * 4).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame capture"),
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        Self {
            buffer,
            path,
            width,
            height,
            stride,
            bgra: matches!(
                texture.format(),
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            ),
        }
    }

    pub fn write(self, device: &wgpu::Device) -> Result<(), String> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(5)),
            })
            .map_err(|e| e.to_string())?;
        receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        let data = self
            .buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|e| e.to_string())?;
        let mut pixels = Vec::with_capacity((self.width * self.height * 4) as usize);
        for row in data.chunks_exact(self.stride as usize) {
            pixels.extend_from_slice(&row[..self.width as usize * 4]);
        }
        drop(data);
        self.buffer.unmap();
        if self.bgra {
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
        }
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = std::fs::File::create(&self.path).map_err(|e| e.to_string())?;
        let mut encoder = png::Encoder::new(file, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(&pixels)
            .map_err(|e| e.to_string())?;
        eprintln!("[capture] saved {}", self.path.display());
        Ok(())
    }
}
