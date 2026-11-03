use super::{ImageError, ImageRegion};
use crate::draw::TextureId;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteId(pub u32);

pub struct SpriteAtlas {
    texture: TextureId,
    size: [u32; 2],
    max_size: u32,
    padding: u32,
    cursor: [u32; 2],
    row_height: u32,
    pixels: Vec<u8>,
    sprites: Vec<[u32; 4]>,
    generation: u64,
}
impl SpriteAtlas {
    pub fn new(
        texture: TextureId,
        initial_size: u32,
        max_size: u32,
        padding: u32,
    ) -> Result<Self, ImageError> {
        if texture.0 < 2
            || !initial_size.is_power_of_two()
            || !max_size.is_power_of_two()
            || initial_size > max_size
            || max_size > 4096
            || padding > 16
        {
            return Err(ImageError::InvalidInput);
        }
        Ok(Self {
            texture,
            size: [initial_size; 2],
            max_size,
            padding,
            cursor: [0; 2],
            row_height: 0,
            pixels: alloc::vec![0; initial_size as usize * initial_size as usize * 4],
            sprites: Vec::new(),
            generation: 1,
        })
    }
    pub fn size(&self) -> [u32; 2] {
        self.size
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn len(&self) -> usize {
        self.sprites.len()
    }
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty()
    }
    pub fn texture(&self) -> TextureId {
        self.texture
    }
    pub fn region(&self, id: SpriteId) -> Option<ImageRegion> {
        self.sprites
            .get(id.0 as usize)
            .map(|&[x, y, w, h]| ImageRegion {
                texture: self.texture,
                uv_min: [
                    x as f32 / self.size[0] as f32,
                    y as f32 / self.size[1] as f32,
                ],
                uv_max: [
                    (x + w) as f32 / self.size[0] as f32,
                    (y + h) as f32 / self.size[1] as f32,
                ],
                size: [w as f32, h as f32],
            })
    }
    pub fn insert(&mut self, rgba: &[u8], size: [u32; 2]) -> Result<SpriteId, ImageError> {
        if size.contains(&0)
            || size.iter().any(|&v| v > self.max_size)
            || rgba.len() != size[0] as usize * size[1] as usize * 4
        {
            return Err(ImageError::InvalidInput);
        }
        if self.sprites.len() >= 4096 {
            return Err(ImageError::AtlasFull);
        }
        let padded = [size[0] + 2 * self.padding, size[1] + 2 * self.padding];
        let mut dimensions = self.size;
        let (position, row_height) = loop {
            let mut position = self.cursor;
            let mut row_height = self.row_height;
            if position[0] + padded[0] > dimensions[0] {
                position = [0, position[1] + row_height];
                row_height = 0;
            }
            if padded[0] <= dimensions[0] && position[1] + padded[1] <= dimensions[1] {
                break (position, row_height.max(padded[1]));
            }
            if dimensions == [self.max_size; 2] {
                return Err(ImageError::AtlasFull);
            }
            let axis = usize::from(dimensions[0] > dimensions[1]);
            dimensions[axis] = (dimensions[axis] * 2).min(self.max_size);
        };
        if dimensions != self.size {
            let mut pixels = alloc::vec![0; dimensions[0] as usize * dimensions[1] as usize * 4];
            for y in 0..self.size[1] as usize {
                let source = y * self.size[0] as usize * 4;
                let target = y * dimensions[0] as usize * 4;
                pixels[target..target + self.size[0] as usize * 4]
                    .copy_from_slice(&self.pixels[source..source + self.size[0] as usize * 4]);
            }
            self.pixels = pixels;
            self.size = dimensions;
        }
        for y in 0..padded[1] {
            for x in 0..padded[0] {
                let source_x = x.saturating_sub(self.padding).min(size[0] - 1);
                let source_y = y.saturating_sub(self.padding).min(size[1] - 1);
                let source = (source_y as usize * size[0] as usize + source_x as usize) * 4;
                let target = ((position[1] + y) as usize * self.size[0] as usize
                    + (position[0] + x) as usize)
                    * 4;
                self.pixels[target..target + 4].copy_from_slice(&rgba[source..source + 4]);
            }
        }
        let id = SpriteId(self.sprites.len() as u32);
        self.sprites.push([
            position[0] + self.padding,
            position[1] + self.padding,
            size[0],
            size[1],
        ]);
        self.cursor = [position[0] + padded[0], position[1]];
        self.row_height = row_height;
        self.generation = self.generation.wrapping_add(1);
        Ok(id)
    }
}
