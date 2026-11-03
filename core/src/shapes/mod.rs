pub(crate) mod clipping;
pub(crate) mod geometry;
mod mesh;
#[cfg(test)]
mod tests;
use crate::{id::Id, panels::PanelCellScope, ui::Ui};
pub use geometry::{BorderPlacement, RoundedRect, Shadow, ShapeError};

#[derive(Clone, Copy)]
pub(crate) struct ClipEntry {
    pub shape: RoundedRect,
    pub id: Id,
    pub popup: bool,
    pub popup_id: Option<Id>,
    pub window: Option<Id>,
}
pub struct RoundedClipScope {
    region: PanelCellScope,
    mark: [usize; 2],
    entry: ClipEntry,
    depth: usize,
}
impl Ui<'_> {
    pub fn draw_rounded_rect(
        &mut self,
        shape: RoundedRect,
        color: [u8; 4],
    ) -> Result<(), ShapeError> {
        let shape = shape.canonical()?;
        if self.ctx.current_window.is_none() {
            return Ok(());
        }
        let scale = self.ctx.font.raster_scale();
        let aa = self.ctx.style.anti_aliasing;
        mesh::fill(
            crate::widgets::draw_parts(self.ctx).1,
            shape,
            scale,
            aa,
            color,
        );
        Ok(())
    }
    pub fn draw_rounded_border(
        &mut self,
        shape: RoundedRect,
        color: [u8; 4],
        width: f32,
        placement: BorderPlacement,
    ) -> Result<(), ShapeError> {
        let shape = shape.canonical()?;
        if !width.is_finite() || !(0.0..=4096.0).contains(&width) {
            return Err(ShapeError::InvalidGeometry);
        }
        if self.ctx.current_window.is_none() || width == 0.0 {
            return Ok(());
        }
        let scale = self.ctx.font.raster_scale();
        let aa = self.ctx.style.anti_aliasing;
        mesh::border(
            crate::widgets::draw_parts(self.ctx).1,
            shape,
            scale,
            aa,
            color,
            width,
            placement,
        );
        Ok(())
    }
    pub fn draw_shadow(&mut self, shape: RoundedRect, shadow: Shadow) -> Result<(), ShapeError> {
        let shape = shape.canonical()?;
        if !shadow.is_valid() {
            return Err(ShapeError::InvalidGeometry);
        }
        if self.ctx.current_window.is_none() {
            return Ok(());
        }
        let scale = self.ctx.font.raster_scale();
        mesh::shadow(crate::widgets::draw_parts(self.ctx).1, shape, scale, shadow);
        Ok(())
    }
    pub fn draw_glow(
        &mut self,
        shape: RoundedRect,
        color: [u8; 4],
        reach: f32,
    ) -> Result<(), ShapeError> {
        self.draw_shadow(
            shape,
            Shadow {
                offset: [0.0; 2],
                spread: 0.0,
                blur: reach,
                color,
                inset: false,
            },
        )
    }
    pub fn begin_rounded_clip(
        &mut self,
        label: &str,
        shape: RoundedRect,
    ) -> Result<RoundedClipScope, ShapeError> {
        let shape = shape.canonical()?;
        if self.ctx.rounded_clips.len() >= 16 {
            return Err(ShapeError::ScopeLimit);
        }
        let entry = ClipEntry {
            shape,
            id: self.widget_id(label),
            popup: self.ctx.popup_scope_active,
            popup_id: self.ctx.routing.scope_id(),
            window: self.ctx.current_window,
        };
        self.ctx.id_stack.push_id(entry.id);
        let region = crate::panels::begin_cell(self, shape.rect, 0x52434c50, 0);
        let outside = !shape.contains(self.ctx.input.mouse_pos);
        if outside {
            self.ctx.prev_mouse_down[0] = true;
            if !self.ctx.input.mouse_down[0] && self.ctx.rounded_capture_path.contains(&entry.id) {
                self.ctx.clear_active();
            }
        }
        let mark = crate::widgets::draw_parts(self.ctx).1.paint_mark();
        self.ctx.rounded_clips.push(entry);
        Ok(RoundedClipScope {
            region,
            mark,
            entry,
            depth: self.ctx.rounded_clips.len(),
        })
    }
    pub fn end_rounded_clip(&mut self, scope: RoundedClipScope) {
        assert_eq!(
            self.ctx.rounded_clips.len(),
            scope.depth,
            "rounded clip end crossed a scope"
        );
        assert_eq!(
            self.ctx.current_window, scope.entry.window,
            "rounded clip crossed a window"
        );
        assert_eq!(
            self.ctx.popup_scope_active, scope.entry.popup,
            "rounded clip crossed a popup"
        );
        assert_eq!(
            self.ctx.routing.scope_id(),
            scope.entry.popup_id,
            "rounded clip crossed a popup level"
        );
        self.ctx.rounded_clips.pop();
        let scale = self.ctx.font.raster_scale();
        crate::widgets::draw_parts(self.ctx).1.clip_rounded_tail(
            scope.mark,
            scope.entry.shape,
            scale,
        );
        self.end_panel_cell(scope.region);
        self.ctx.id_stack.pop();
    }
    pub fn rounded_clip(
        &mut self,
        label: &str,
        shape: RoundedRect,
        body: impl FnOnce(&mut Ui),
    ) -> Result<(), ShapeError> {
        let scope = self.begin_rounded_clip(label, shape)?;
        body(self);
        self.end_rounded_clip(scope);
        Ok(())
    }
}
