mod atlas;
use crate::{
    draw::TextureId,
    ui::{hovered, point_in_rect, Ui},
    widgets,
};
pub use atlas::{SpriteAtlas, SpriteId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    InvalidInput,
    AtlasFull,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct ImageRegion {
    pub texture: TextureId,
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub size: [f32; 2],
}
impl ImageRegion {
    pub fn is_valid(&self) -> bool {
        self.uv_min
            .iter()
            .chain(&self.uv_max)
            .all(|v| v.is_finite())
            && self
                .size
                .iter()
                .all(|v| v.is_finite() && *v > 0. && *v <= 16384.)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ImageFit {
    Stretch = 0,
    Contain = 1,
    Cover = 2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct NineSlice {
    pub source: [f32; 4],
    pub destination: [f32; 4],
}

fn insets(mut values: [f32; 4], size: [f32; 2]) -> [f32; 4] {
    for axis in 0..2 {
        let sum = values[axis] + values[axis + 2];
        if sum > size[axis] && sum > 0. {
            let ratio = size[axis] / sum;
            values[axis] *= ratio;
            values[axis + 2] *= ratio;
        }
    }
    values
}
fn fitted(mut region: ImageRegion, mut rect: [f32; 4], fit: ImageFit) -> ([f32; 4], ImageRegion) {
    let size = [rect[2] - rect[0], rect[3] - rect[1]];
    if size[0] <= 0. || size[1] <= 0. {
        return (rect, region);
    }
    let ratio = [size[0] / region.size[0], size[1] / region.size[1]];
    if fit == ImageFit::Contain {
        let scale = ratio[0].min(ratio[1]);
        for axis in 0..2 {
            let gap = (size[axis] - region.size[axis] * scale) * 0.5;
            rect[axis] += gap;
            rect[axis + 2] -= gap;
        }
    } else if fit == ImageFit::Cover {
        let scale = ratio[0].max(ratio[1]);
        for (axis, &extent) in size.iter().enumerate() {
            let gap = (1. - extent / (region.size[axis] * scale)) * 0.5;
            let uv = (region.uv_max[axis] - region.uv_min[axis]) * gap;
            region.uv_min[axis] += uv;
            region.uv_max[axis] -= uv;
        }
    }
    (rect, region)
}
fn image_size(region: ImageRegion, mut size: [f32; 2]) -> Result<[f32; 2], ImageError> {
    if !region.is_valid()
        || !size
            .iter()
            .all(|v| v.is_finite() && *v >= 0. && *v <= 16384.)
    {
        return Err(ImageError::InvalidInput);
    }
    if size == [0.; 2] {
        size = region.size;
    } else if size[0] == 0. {
        size[0] = size[1] * region.size[0] / region.size[1];
    } else if size[1] == 0. {
        size[1] = size[0] * region.size[1] / region.size[0];
    }
    if size.iter().any(|v| !v.is_finite() || *v > 16384.) {
        return Err(ImageError::InvalidInput);
    }
    Ok(size)
}
impl Ui<'_> {
    pub fn image(
        &mut self,
        region: ImageRegion,
        size: [f32; 2],
        fit: ImageFit,
        tint: [u8; 4],
    ) -> Result<[f32; 4], ImageError> {
        let size = image_size(region, size)?;
        let rect = widgets::item_rect(self.ctx, size);
        let (paint, region) = fitted(region, rect, fit);
        self.draw_image(
            region.texture,
            [paint[0], paint[1]],
            [paint[2], paint[3]],
            region.uv_min,
            region.uv_max,
            tint,
        );
        Ok(rect)
    }
    pub fn image_button(
        &mut self,
        label: &str,
        region: ImageRegion,
        size: [f32; 2],
        fit: ImageFit,
        tint: [u8; 4],
    ) -> Result<bool, ImageError> {
        let size = image_size(region, size)?;
        let id = widgets::widget_id(self.ctx, label);
        let rect = widgets::item_rect(self.ctx, size);
        widgets::register_focusable_item(self.ctx, id, rect);
        let inside =
            point_in_rect(self.ctx.input.mouse_pos, rect) && crate::ui::hover_layer_gate(self.ctx);
        let down = self.ctx.input.mouse_down[0];
        if inside && down && !self.ctx.prev_mouse_down[0] && crate::ui::hover_layer_gate(self.ctx) {
            self.ctx.set_active(id);
        }
        let held = self.ctx.active == Some(id);
        let clicked = held && !down && inside || self.ctx.keyboard_activate == Some(id);
        if held && !down {
            self.ctx.clear_active();
        }
        if hovered(self.ctx, rect) {
            self.ctx.hot = Some(id);
        }
        let (paint, region) = fitted(region, rect, fit);
        self.draw_image(
            region.texture,
            [paint[0], paint[1]],
            [paint[2], paint[3]],
            region.uv_min,
            region.uv_max,
            tint,
        );
        Ok(clicked)
    }
    pub fn draw_nine_slice(
        &mut self,
        region: ImageRegion,
        rect: [f32; 4],
        slices: NineSlice,
        tint: [u8; 4],
    ) -> Result<(), ImageError> {
        if !region.is_valid()
            || !rect.iter().all(|v| v.is_finite())
            || rect[2] < rect[0]
            || rect[3] < rect[1]
            || !slices
                .source
                .iter()
                .chain(&slices.destination)
                .all(|v| v.is_finite() && *v >= 0. && *v <= 16384.)
        {
            return Err(ImageError::InvalidInput);
        }
        let source = insets(slices.source, region.size);
        let destination = insets(slices.destination, [rect[2] - rect[0], rect[3] - rect[1]]);
        let xs = [
            rect[0],
            rect[0] + destination[0],
            rect[2] - destination[2],
            rect[2],
        ];
        let ys = [
            rect[1],
            rect[1] + destination[1],
            rect[3] - destination[3],
            rect[3],
        ];
        let du = region.uv_max[0] - region.uv_min[0];
        let dv = region.uv_max[1] - region.uv_min[1];
        let us = [
            region.uv_min[0],
            region.uv_min[0] + du * source[0] / region.size[0],
            region.uv_max[0] - du * source[2] / region.size[0],
            region.uv_max[0],
        ];
        let vs = [
            region.uv_min[1],
            region.uv_min[1] + dv * source[1] / region.size[1],
            region.uv_max[1] - dv * source[3] / region.size[1],
            region.uv_max[1],
        ];
        for y in 0..3 {
            for x in 0..3 {
                self.draw_image(
                    region.texture,
                    [xs[x], ys[y]],
                    [xs[x + 1], ys[y + 1]],
                    [us[x], vs[y]],
                    [us[x + 1], vs[y + 1]],
                    tint,
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
