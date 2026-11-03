use crate::{BorderPlacement, Gui, RoundedRect, Scope, Shadow, ShapeError};

impl Gui {
    pub fn draw_rounded_rect(
        &mut self,
        shape: RoundedRect,
        color: [u8; 4],
    ) -> Result<(), ShapeError> {
        if self.emitting() {
            self.ctx.ui().draw_rounded_rect(shape, color)
        } else {
            Ok(())
        }
    }
    pub fn draw_rounded_border(
        &mut self,
        shape: RoundedRect,
        color: [u8; 4],
        width: f32,
        placement: BorderPlacement,
    ) -> Result<(), ShapeError> {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_rounded_border(shape, color, width, placement)
        } else {
            Ok(())
        }
    }
    pub fn draw_shadow(&mut self, shape: RoundedRect, shadow: Shadow) -> Result<(), ShapeError> {
        if self.emitting() {
            self.ctx.ui().draw_shadow(shape, shadow)
        } else {
            Ok(())
        }
    }
    pub fn draw_glow(
        &mut self,
        shape: RoundedRect,
        color: [u8; 4],
        reach: f32,
    ) -> Result<(), ShapeError> {
        if self.emitting() {
            self.ctx.ui().draw_glow(shape, color, reach)
        } else {
            Ok(())
        }
    }
    pub fn begin_rounded_clip(
        &mut self,
        label: &str,
        shape: RoundedRect,
    ) -> Result<(), ShapeError> {
        let scope = if self.emitting() {
            Some(self.ctx.ui().begin_rounded_clip(label, shape)?)
        } else {
            None
        };
        self.scopes.push(Scope::RoundedClip(scope));
        Ok(())
    }
    pub fn end_rounded_clip(&mut self) {
        match self.scopes.pop() {
            Some(Scope::RoundedClip(Some(scope))) => self.ctx.ui().end_rounded_clip(scope),
            Some(Scope::RoundedClip(None)) => {}
            _ => panic!("end_rounded_clip requires rounded clip"),
        }
    }
}
