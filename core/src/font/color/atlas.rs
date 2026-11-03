use super::{Bitmap, ColorFontError};
use alloc::{vec, vec::Vec};

#[derive(Clone, Copy)]
pub(crate) struct Entry {
    pub rect: [u32; 4],
    pub origin: [i32; 2],
}
#[derive(Clone, Default)]
pub(crate) struct Atlas {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub generation: u64,
    x: u32,
    y: u32,
    row: u32,
}
impl Atlas {
    pub fn insert(&mut self, bitmap: &Bitmap) -> Result<Entry, ColorFontError> {
        let w = bitmap.width + 2;
        let h = bitmap.height + 2;
        let width = 1024;
        let height = 1024;
        let mut x = self.x;
        let mut y = self.y;
        let mut row = self.row;
        if w > width || h > height {
            return Err(ColorFontError::AtlasFull);
        }
        if x + w > width {
            x = 0;
            y += row;
            row = 0;
        }
        if y + h > height {
            return Err(ColorFontError::AtlasFull);
        }
        if width != self.width || height != self.height {
            let mut pixels = vec![0; (width * height * 4) as usize];
            for r in 0..self.height as usize {
                let src = r * self.width as usize * 4;
                let dst = r * width as usize * 4;
                pixels[dst..dst + self.width as usize * 4]
                    .copy_from_slice(&self.pixels[src..src + self.width as usize * 4]);
            }
            self.pixels = pixels;
            self.width = width;
            self.height = height;
        }
        for r in 0..bitmap.height as usize {
            let src = r * bitmap.width as usize * 4;
            let dst = (((y + 1 + r as u32) * width + x + 1) * 4) as usize;
            self.pixels[dst..dst + bitmap.width as usize * 4]
                .copy_from_slice(&bitmap.pixels[src..src + bitmap.width as usize * 4]);
        }
        self.x = x + w;
        self.y = y;
        self.row = row.max(h);
        self.generation = self.generation.wrapping_add(1);
        Ok(Entry {
            rect: [x + 1, y + 1, bitmap.width, bitmap.height],
            origin: bitmap.origin,
        })
    }
}
