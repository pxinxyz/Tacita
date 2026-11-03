use super::{
    DrawEffects, EffectError, GradientFill, GradientPaint, GradientStop, MaskOp, MaskScope,
};
use crate::{
    shapes::{BorderPlacement, RoundedRect},
    ui::Ui,
};

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SdfKind {
    #[default]
    Fill = 1,
    Border = 2,
    Glow = 3,
    EdgeGlow = 4,
    Highlight = 5,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SdfStyle {
    pub kind: SdfKind,
    pub width: f32,
    pub placement: BorderPlacement,
    pub reach: f32,
    pub light_direction: [f32; 2],
    pub light_power: f32,
}
impl Default for SdfStyle {
    fn default() -> Self {
        Self {
            kind: SdfKind::Fill,
            width: 1.0,
            placement: BorderPlacement::Inner,
            reach: 12.0,
            light_direction: [-1.0, -1.0],
            light_power: 2.0,
        }
    }
}
impl SdfStyle {
    pub fn is_valid(self) -> bool {
        [self.width, self.reach]
            .iter()
            .all(|v| v.is_finite() && (0.0..=4096.0).contains(v))
            && self
                .light_direction
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 4096.0)
            && self.light_direction.iter().map(|v| v * v).sum::<f32>() >= 0.000001
            && self.light_power.is_finite()
            && (0.25..=64.0).contains(&self.light_power)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SdfShape {
    pub rect: [f32; 4],
    pub radii: [f32; 4],
    pub parameters: [f32; 4],
    pub light: [f32; 4],
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealDirection {
    LeftToRight = 3,
    RightToLeft = 4,
    TopToBottom = 5,
    BottomToTop = 6,
    Radial = 7,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RevealMask {
    pub rect: [f32; 4],
    pub direction: RevealDirection,
    pub progress: f32,
    pub softness: f32,
    pub inverted: bool,
}
impl RevealMask {
    pub fn is_valid(self) -> bool {
        RoundedRect::new(self.rect, [0.0; 4]).is_ok()
            && [self.progress, self.softness]
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    }
}

impl Ui<'_> {
    pub fn draw_sdf_shape(
        &mut self,
        shape: RoundedRect,
        color: [u8; 4],
        style: SdfStyle,
    ) -> Result<(), EffectError> {
        self.emit_sdf(shape, color, style, GradientFill::default())
    }
    pub fn draw_sdf_shape_gradient(
        &mut self,
        shape: RoundedRect,
        stops: &[GradientStop],
        paint: GradientPaint,
        style: SdfStyle,
    ) -> Result<(), EffectError> {
        self.draw_sdf_shape_gradient_with(shape, stops.len(), |i| stops[i], paint, style)
    }
    pub fn draw_sdf_shape_gradient_with(
        &mut self,
        shape: RoundedRect,
        count: usize,
        get: impl Fn(usize) -> GradientStop,
        paint: GradientPaint,
        style: SdfStyle,
    ) -> Result<(), EffectError> {
        if !style.is_valid() {
            return Err(EffectError::InvalidInput);
        }
        let gradient = self.paint_record(shape, count, get, paint)?;
        self.emit_sdf(shape, [255; 4], style, gradient)
    }
    fn emit_sdf(
        &mut self,
        shape: RoundedRect,
        color: [u8; 4],
        style: SdfStyle,
        gradient: GradientFill,
    ) -> Result<(), EffectError> {
        let shape =
            RoundedRect::new(shape.rect, shape.radii).map_err(|_| EffectError::InvalidInput)?;
        if !style.is_valid() {
            return Err(EffectError::InvalidInput);
        }
        if self.ctx.current_window.is_none()
            || color[3] == 0
            || (matches!(style.kind, SdfKind::Border | SdfKind::Highlight) && style.width == 0.0)
            || (matches!(style.kind, SdfKind::Glow | SdfKind::EdgeGlow) && style.reach == 0.0)
        {
            return Ok(());
        }
        let light_len = crate::draw::sqrt_approx(style.light_direction.iter().map(|v| v * v).sum());
        let sdf = SdfShape {
            rect: shape.rect,
            radii: shape.radii,
            parameters: [
                style.kind as u32 as f32,
                style.width,
                style.placement as u32 as f32,
                style.reach,
            ],
            light: [
                style.light_direction[0] / light_len,
                style.light_direction[1] / light_len,
                style.light_power,
                if self.ctx.style.anti_aliasing {
                    1.0
                } else {
                    0.0
                },
            ],
        };
        let expansion = match style.kind {
            SdfKind::Glow | SdfKind::EdgeGlow => style.reach * 6.0,
            SdfKind::Border => match style.placement {
                BorderPlacement::Inner => 0.0,
                BorderPlacement::Center => style.width * 0.5,
                BorderPlacement::Outer => style.width,
            },
            _ => 0.0,
        } + 2.0 / self.ctx.font.raster_scale();
        let draw = crate::widgets::draw_parts(self.ctx).1;
        let previous = draw.effects_state();
        draw.set_effects(DrawEffects {
            gradient,
            sdf,
            ..previous
        });
        draw.add_filled_rect(
            [shape.rect[0] - expansion, shape.rect[1] - expansion],
            [shape.rect[2] + expansion, shape.rect[3] + expansion],
            color,
        );
        draw.set_effects(previous);
        Ok(())
    }
    pub fn begin_reveal_mask(&mut self, mask: RevealMask) -> Result<MaskScope, EffectError> {
        if !mask.is_valid() {
            return Err(EffectError::InvalidInput);
        }
        self.begin_mask(MaskOp {
            kind: mask.direction as u32,
            rect: mask.rect,
            params: [
                mask.progress,
                mask.softness,
                if mask.inverted { 1.0 } else { 0.0 },
                0.0,
            ],
            ..Default::default()
        })
    }
    pub fn reveal_mask(
        &mut self,
        mask: RevealMask,
        body: impl FnOnce(&mut Ui),
    ) -> Result<(), EffectError> {
        let scope = self.begin_reveal_mask(mask)?;
        body(self);
        self.end_mask(scope);
        Ok(())
    }
}
