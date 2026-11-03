use crate::{
    AlphaMask, BorderPlacement, EffectError, GradientMask, GradientPaint, GradientStop, Gui,
    RevealMask, RoundedRect, Scope, SdfStyle,
};
impl Gui {
    pub fn draw_sdf_shape(
        &mut self,
        shape: RoundedRect,
        color: [u8; 4],
        style: SdfStyle,
    ) -> Result<(), EffectError> {
        if self.emitting() {
            self.ctx.ui().draw_sdf_shape(shape, color, style)
        } else {
            Ok(())
        }
    }
    pub fn draw_sdf_shape_gradient(
        &mut self,
        shape: RoundedRect,
        stops: &[GradientStop],
        paint: GradientPaint,
        style: SdfStyle,
    ) -> Result<(), EffectError> {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_sdf_shape_gradient(shape, stops, paint, style)
        } else {
            Ok(())
        }
    }
    pub fn begin_reveal_mask(&mut self, mask: RevealMask) -> Result<(), EffectError> {
        let scope = if self.emitting() {
            Some(self.ctx.ui().begin_reveal_mask(mask)?)
        } else {
            None
        };
        self.scopes.push(Scope::Mask(scope));
        Ok(())
    }
    pub fn draw_shape_gradient(
        &mut self,
        shape: RoundedRect,
        stops: &[GradientStop],
        paint: GradientPaint,
    ) -> Result<(), EffectError> {
        if self.emitting() {
            self.ctx.ui().draw_shape_gradient(shape, stops, paint)
        } else {
            Ok(())
        }
    }
    pub fn draw_gradient_border(
        &mut self,
        shape: RoundedRect,
        stops: &[GradientStop],
        paint: GradientPaint,
        width: f32,
        placement: BorderPlacement,
    ) -> Result<(), EffectError> {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_gradient_border(shape, stops, paint, width, placement)
        } else {
            Ok(())
        }
    }
    pub fn begin_alpha_mask(&mut self, mask: AlphaMask) -> Result<(), EffectError> {
        let scope = if self.emitting() {
            Some(self.ctx.ui().begin_alpha_mask(mask)?)
        } else {
            None
        };
        self.scopes.push(Scope::Mask(scope));
        Ok(())
    }
    pub fn begin_gradient_mask(&mut self, mask: GradientMask) -> Result<(), EffectError> {
        let scope = if self.emitting() {
            Some(self.ctx.ui().begin_gradient_mask(mask)?)
        } else {
            None
        };
        self.scopes.push(Scope::Mask(scope));
        Ok(())
    }
    pub fn end_mask(&mut self) {
        match self.scopes.pop() {
            Some(Scope::Mask(Some(scope))) => self.ctx.ui().end_mask(scope),
            Some(Scope::Mask(None)) => {}
            _ => panic!("end_mask requires mask scope"),
        }
    }
}

impl Gui {
    pub fn draw_surface(
        &mut self,
        shape: RoundedRect,
        texture: Option<crate::TextureId>,
        uv: [f32; 4],
        color: [u8; 4],
        style: crate::SurfaceStyle,
    ) -> Result<(), EffectError> {
        if self.emitting() {
            self.ctx.ui().draw_surface(shape, texture, uv, color, style)
        } else {
            Ok(())
        }
    }
    pub fn draw_filtered_image(
        &mut self,
        texture: crate::TextureId,
        rect: [f32; 4],
        uv: [f32; 4],
        color: [u8; 4],
        filter: crate::ImageFilter,
    ) -> Result<(), EffectError> {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_filtered_image(texture, rect, uv, color, filter)
        } else {
            Ok(())
        }
    }
    pub fn begin_stencil_clip(
        &mut self,
        label: &str,
        shape: RoundedRect,
    ) -> Result<(), EffectError> {
        let scope = if self.emitting() {
            Some(self.ctx.ui().begin_stencil_clip(label, shape)?)
        } else {
            None
        };
        self.scopes.push(Scope::Stencil(scope));
        Ok(())
    }
    pub fn end_stencil_clip(&mut self) {
        assert!(
            matches!(self.scopes.last(), Some(Scope::Stencil(_))),
            "end_stencil_clip requires stencil scope"
        );
        if let Some(Scope::Stencil(Some(scope))) = self.scopes.pop() {
            self.ctx.ui().end_stencil_clip(scope);
        }
    }
}
