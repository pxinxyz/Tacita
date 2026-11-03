mod filter;
mod sdf;
mod stencil;
mod surface;
#[cfg(test)]
mod tests;
mod uniform;
pub use crate::animation::{GradientSpread, GradientStop};
use crate::{
    draw::TextureId,
    id::Id,
    shapes::{BorderPlacement, RoundedRect},
    ui::Ui,
};
pub use filter::{
    Blur, BlurMethod, FilterKind, FilterPacket, FilterPlan, FilterStep, ImageFilter,
    MAX_FILTER_PASSES,
};
pub use sdf::{RevealDirection, RevealMask, SdfKind, SdfShape, SdfStyle};
pub use stencil::StencilScope;
pub use surface::*;

pub const MAX_PAINT_STOPS: usize = 16;
pub const MAX_MASKS: usize = 4;
pub const EFFECT_UNIFORM_SIZE: usize = 1024;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientKind {
    Linear = 1,
    Radial = 2,
    Conic = 3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientPaint {
    pub kind: GradientKind,
    pub origin: [f32; 2],
    pub vector: [f32; 2],
    pub spread: GradientSpread,
    pub phase: f32,
}
impl Default for GradientPaint {
    fn default() -> Self {
        Self {
            kind: GradientKind::Linear,
            origin: [0.0, 0.5],
            vector: [1.0, 0.0],
            spread: GradientSpread::Clamp,
            phase: 0.0,
        }
    }
}
impl GradientPaint {
    pub fn is_valid(self) -> bool {
        self.origin
            .iter()
            .chain(self.vector.iter())
            .all(|v| v.is_finite() && v.abs() <= 4096.0)
            && self.phase.is_finite()
            && self.phase.abs() <= 1_000_000.0
            && self.vector.iter().map(|v| v * v).sum::<f32>() >= 0.000001
            && (self.kind != GradientKind::Radial || self.vector.iter().all(|v| *v >= 0.0001))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlphaMask {
    pub texture: TextureId,
    pub rect: [f32; 4],
    pub uv: [f32; 4],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientMask {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub opacity: [f32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectError {
    InvalidInput,
    ScopeLimit,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MaskOp {
    pub texture: u64,
    pub kind: u32,
    pub reserved: u32,
    pub rect: [f32; 4],
    pub params: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientFill {
    pub rect: [f32; 4],
    pub origin: [f32; 2],
    pub vector: [f32; 2],
    pub kind: u32,
    pub spread: u32,
    pub phase: f32,
    pub stop_count: u32,
    pub stops: [GradientStop; MAX_PAINT_STOPS],
}
impl Default for GradientFill {
    fn default() -> Self {
        Self {
            rect: [0.0; 4],
            origin: [0.0; 2],
            vector: [0.0; 2],
            kind: 0,
            spread: 0,
            phase: 0.0,
            stop_count: 0,
            stops: [GradientStop {
                position: 0.0,
                color: [0; 4],
            }; MAX_PAINT_STOPS],
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MaskStack {
    pub ops: [MaskOp; MAX_MASKS],
    pub count: u32,
    pub reserved: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawEffects {
    pub gradient: GradientFill,
    pub masks: MaskStack,
    pub sdf: SdfShape,
    pub surface: SurfaceStyle,
    pub filter: ImageFilter,
    pub stencil: [u32; 2],
}
impl DrawEffects {
    pub(crate) fn identity(self) -> bool {
        self.gradient.kind == 0
            && self.masks.count == 0
            && self.sdf.parameters[0] == 0.0
            && self.surface.flags == 0
            && self.filter == ImageFilter::default()
            && self.stencil == [0; 2]
    }
    pub fn mask_texture(self, index: usize) -> Option<TextureId> {
        if index == 3 && self.filter.kind == FilterKind::BloomCompose {
            return Some(TextureId(self.filter.secondary));
        }
        self.masks
            .ops
            .get(index)
            .filter(|op| index < self.masks.count as usize && op.kind == 1)
            .map(|op| TextureId(op.texture))
    }
}

pub struct MaskScope {
    stencil_depth: usize,
    previous: DrawEffects,
    window: Option<Id>,
    popup: bool,
    popup_id: Option<Id>,
    depth: usize,
}
fn coordinates(values: &[f32]) -> bool {
    values
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
}
impl Ui<'_> {
    pub fn draw_shape_gradient(
        &mut self,
        shape: RoundedRect,
        stops: &[GradientStop],
        paint: GradientPaint,
    ) -> Result<(), EffectError> {
        self.draw_shape_gradient_with(shape, stops.len(), |i| stops[i], paint)
    }
    pub fn draw_shape_gradient_with(
        &mut self,
        shape: RoundedRect,
        count: usize,
        get: impl Fn(usize) -> GradientStop,
        paint: GradientPaint,
    ) -> Result<(), EffectError> {
        let gradient = self.paint_record(shape, count, get, paint)?;
        if self.ctx.current_window.is_none() {
            return Ok(());
        }
        let draw = crate::widgets::draw_parts(self.ctx).1;
        let previous = draw.effects_state();
        draw.set_effects(DrawEffects {
            gradient,
            ..previous
        });
        let result = self
            .draw_rounded_rect(shape, [255; 4])
            .map_err(|_| EffectError::InvalidInput);
        crate::widgets::draw_parts(self.ctx).1.set_effects(previous);
        result
    }
    pub fn draw_gradient_border(
        &mut self,
        shape: RoundedRect,
        stops: &[GradientStop],
        paint: GradientPaint,
        width: f32,
        placement: BorderPlacement,
    ) -> Result<(), EffectError> {
        self.draw_gradient_border_with(shape, stops.len(), |i| stops[i], paint, width, placement)
    }
    pub fn draw_gradient_border_with(
        &mut self,
        shape: RoundedRect,
        count: usize,
        get: impl Fn(usize) -> GradientStop,
        paint: GradientPaint,
        width: f32,
        placement: BorderPlacement,
    ) -> Result<(), EffectError> {
        let gradient = self.paint_record(shape, count, get, paint)?;
        if !width.is_finite() || !(0.0..=4096.0).contains(&width) {
            return Err(EffectError::InvalidInput);
        }
        if self.ctx.current_window.is_none() {
            return Ok(());
        }
        let draw = crate::widgets::draw_parts(self.ctx).1;
        let previous = draw.effects_state();
        draw.set_effects(DrawEffects {
            gradient,
            ..previous
        });
        let result = self
            .draw_rounded_border(shape, [255; 4], width, placement)
            .map_err(|_| EffectError::InvalidInput);
        crate::widgets::draw_parts(self.ctx).1.set_effects(previous);
        result
    }
    fn paint_record(
        &self,
        shape: RoundedRect,
        count: usize,
        get: impl Fn(usize) -> GradientStop,
        paint: GradientPaint,
    ) -> Result<GradientFill, EffectError> {
        let shape =
            RoundedRect::new(shape.rect, shape.radii).map_err(|_| EffectError::InvalidInput)?;
        if !paint.is_valid() || !(2..=MAX_PAINT_STOPS).contains(&count) {
            return Err(EffectError::InvalidInput);
        }
        let mut record = GradientFill {
            rect: shape.rect,
            origin: paint.origin,
            vector: paint.vector,
            kind: paint.kind as u32,
            spread: match paint.spread {
                GradientSpread::Clamp => 0,
                GradientSpread::Repeat => 1,
                GradientSpread::Mirror => 2,
            },
            phase: paint.phase,
            stop_count: count as u32,
            ..Default::default()
        };
        let mut previous = -1.0;
        for i in 0..count {
            let stop = get(i);
            if !stop.position.is_finite()
                || !(0.0..=1.0).contains(&stop.position)
                || stop.position <= previous
            {
                return Err(EffectError::InvalidInput);
            }
            previous = stop.position;
            record.stops[i] = stop;
        }
        if record.stops[0].position != 0.0 || previous != 1.0 {
            return Err(EffectError::InvalidInput);
        }
        Ok(record)
    }
    pub fn begin_alpha_mask(&mut self, mask: AlphaMask) -> Result<MaskScope, EffectError> {
        if !coordinates(&mask.rect)
            || mask.rect[0] >= mask.rect[2]
            || mask.rect[1] >= mask.rect[3]
            || !mask
                .uv
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        {
            return Err(EffectError::InvalidInput);
        }
        self.begin_mask(MaskOp {
            texture: mask.texture.0,
            kind: 1,
            rect: mask.rect,
            params: mask.uv,
            ..Default::default()
        })
    }
    pub fn begin_gradient_mask(&mut self, mask: GradientMask) -> Result<MaskScope, EffectError> {
        let line = [mask.start[0], mask.start[1], mask.end[0], mask.end[1]];
        let delta = [mask.end[0] - mask.start[0], mask.end[1] - mask.start[1]];
        if !coordinates(&line)
            || delta.iter().map(|v| v * v).sum::<f32>() < 0.000001
            || !mask
                .opacity
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        {
            return Err(EffectError::InvalidInput);
        }
        self.begin_mask(MaskOp {
            kind: 2,
            rect: line,
            params: [mask.opacity[0], mask.opacity[1], 0.0, 0.0],
            ..Default::default()
        })
    }
    fn begin_mask(&mut self, op: MaskOp) -> Result<MaskScope, EffectError> {
        assert!(
            self.ctx.current_window.is_some(),
            "mask requires an open window"
        );
        let draw = crate::widgets::draw_parts(self.ctx).1;
        let previous = draw.effects_state();
        if previous.masks.count as usize >= MAX_MASKS {
            return Err(EffectError::ScopeLimit);
        }
        let mut next = previous;
        next.masks.ops[next.masks.count as usize] = op;
        next.masks.count += 1;
        draw.set_effects(next);
        self.ctx.mask_depth += 1;
        Ok(MaskScope {
            stencil_depth: self.ctx.stencil_depth,
            previous,
            window: self.ctx.current_window,
            popup: self.ctx.popup_scope_active,
            popup_id: self.ctx.routing.scope_id(),
            depth: self.ctx.mask_depth,
        })
    }
    pub fn end_mask(&mut self, scope: MaskScope) {
        assert_eq!(
            scope.stencil_depth, self.ctx.stencil_depth,
            "mask crossed a stencil scope"
        );
        assert_eq!(scope.depth, self.ctx.mask_depth, "mask end crossed a scope");
        assert_eq!(
            scope.window, self.ctx.current_window,
            "mask crossed a window"
        );
        assert_eq!(
            scope.popup, self.ctx.popup_scope_active,
            "mask crossed a popup"
        );
        assert_eq!(
            scope.popup_id,
            self.ctx.routing.scope_id(),
            "mask crossed a popup level"
        );
        crate::widgets::draw_parts(self.ctx)
            .1
            .set_effects(scope.previous);
        self.ctx.mask_depth -= 1;
    }
    pub fn alpha_mask(
        &mut self,
        mask: AlphaMask,
        body: impl FnOnce(&mut Ui),
    ) -> Result<(), EffectError> {
        let scope = self.begin_alpha_mask(mask)?;
        body(self);
        self.end_mask(scope);
        Ok(())
    }
    pub fn gradient_mask(
        &mut self,
        mask: GradientMask,
        body: impl FnOnce(&mut Ui),
    ) -> Result<(), EffectError> {
        let scope = self.begin_gradient_mask(mask)?;
        body(self);
        self.end_mask(scope);
        Ok(())
    }
}
