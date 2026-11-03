use crate::{context::Context, font::FontError};
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct DisplayMetrics {
    pub physical_size: [u32; 2],
    pub scale: f32,
}
impl DisplayMetrics {
    pub fn is_valid(self) -> bool {
        self.scale.is_finite() && self.scale > 0.0 && self.scale <= 8.0
    }
    pub fn logical_size(self) -> [f32; 2] {
        [
            self.physical_size[0] as f32 / self.scale,
            self.physical_size[1] as f32 / self.scale,
        ]
    }
    pub fn logical_point(self, physical: [f32; 2]) -> [f32; 2] {
        [physical[0] / self.scale, physical[1] / self.scale]
    }
    pub fn physical_rect(self, logical: [f32; 4]) -> [f32; 4] {
        logical.map(|v| v * self.scale)
    }
}
impl Context {
    pub fn update_display_metrics(&mut self, metrics: DisplayMetrics) -> Result<bool, FontError> {
        assert!(!self.frame_active, "display metrics change between frames");
        if !metrics.is_valid() {
            return Err(FontError::InvalidFont("invalid display density"));
        }
        if self.font().raster_scale() == metrics.scale {
            return Ok(false);
        }
        let rebuilt = self.font().rescaled(metrics.scale)?;
        self.replace_font(rebuilt);
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monitor_change_preserves_ids_and_failures_are_atomic() {
        let bytes = include_bytes!("../tests/assets/DejaVuSans.ttf");
        let mut ctx = Context::new(bytes).unwrap();
        let face = ctx.add_font(bytes, 12.).unwrap();
        let svg = ctx
            .add_svg_icon(
                "<svg viewBox='0 0 8 8'><circle cx='4' cy='4' r='3'/></svg>",
                [8.; 2],
            )
            .unwrap();
        let metrics = DisplayMetrics {
            physical_size: [1500, 900],
            scale: 1.5,
        };
        assert_eq!(metrics.logical_size(), [1000., 600.]);
        assert!(ctx.update_display_metrics(metrics).unwrap());
        assert_eq!(ctx.font().face_info(face).unwrap().px_size, 12.);
        assert!(ctx.font().has_svg_icon(svg));
        let generation = ctx.font().atlas_generation();
        assert!(ctx
            .update_display_metrics(DisplayMetrics {
                scale: f32::NAN,
                ..metrics
            })
            .is_err());
        assert_eq!(ctx.font().atlas_generation(), generation);
    }
}
