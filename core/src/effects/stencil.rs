use super::{DrawEffects, EffectError, SdfShape};
use crate::{
    panels::PanelCellScope,
    shapes::{ClipEntry, RoundedRect},
    ui::Ui,
};

pub struct StencilScope {
    previous: DrawEffects,
    entry: ClipEntry,
    region: PanelCellScope,
    depth: usize,
    clip_depth: usize,
    mask_depth: usize,
}
fn stamp(draw: &mut crate::draw::DrawList, shape: RoundedRect, reference: u32, op: u32) {
    let previous = draw.effects_state();
    draw.set_effects(DrawEffects {
        sdf: SdfShape {
            rect: shape.rect,
            radii: shape.radii,
            parameters: [1., 0., 0., 0.],
            light: [0.; 4],
        },
        stencil: [reference, op],
        ..Default::default()
    });
    draw.add_filled_rect(
        [shape.rect[0], shape.rect[1]],
        [shape.rect[2], shape.rect[3]],
        [255; 4],
    );
    draw.set_effects(previous);
}
impl Ui<'_> {
    pub fn begin_stencil_clip(
        &mut self,
        label: &str,
        shape: RoundedRect,
    ) -> Result<StencilScope, EffectError> {
        let shape = shape.canonical().map_err(|_| EffectError::InvalidInput)?;
        if self.ctx.current_window.is_none() {
            return Err(EffectError::InvalidInput);
        }
        let previous = crate::widgets::draw_parts(self.ctx).1.effects_state();
        if previous.stencil[0] >= 8 || self.ctx.rounded_clips.len() >= 16 {
            return Err(EffectError::ScopeLimit);
        }
        let entry = ClipEntry {
            shape,
            id: self.widget_id(label),
            popup: self.ctx.popup_scope_active,
            popup_id: self.ctx.routing.scope_id(),
            window: self.ctx.current_window,
        };
        self.ctx.id_stack.push_id(entry.id);
        let region = crate::panels::begin_cell(self, shape.rect, 0x53544e43, 0);
        if !shape.contains(self.ctx.input.mouse_pos) {
            self.ctx.prev_mouse_down[0] = true;
            if !self.ctx.input.mouse_down[0] && self.ctx.rounded_capture_path.contains(&entry.id) {
                self.ctx.clear_active();
            }
        }
        let draw = crate::widgets::draw_parts(self.ctx).1;
        stamp(draw, shape, previous.stencil[0], 1);
        draw.set_effects(DrawEffects {
            stencil: [previous.stencil[0] + 1, 0],
            ..previous
        });
        self.ctx.rounded_clips.push(entry);
        self.ctx.stencil_depth += 1;
        let clip_depth = crate::widgets::draw_parts(self.ctx).1.clip_depth();
        Ok(StencilScope {
            previous,
            entry,
            region,
            depth: self.ctx.rounded_clips.len(),
            clip_depth,
            mask_depth: self.ctx.mask_depth,
        })
    }
    pub fn end_stencil_clip(&mut self, scope: StencilScope) {
        assert_eq!(
            self.ctx.rounded_clips.len(),
            scope.depth,
            "stencil end crossed a clip"
        );
        assert_eq!(
            self.ctx.current_window, scope.entry.window,
            "stencil crossed a window"
        );
        assert_eq!(
            self.ctx.routing.scope_id(),
            scope.entry.popup_id,
            "stencil crossed a popup"
        );
        assert_eq!(
            self.ctx.mask_depth, scope.mask_depth,
            "stencil crossed a mask scope"
        );
        assert_eq!(
            crate::widgets::draw_parts(self.ctx).1.clip_depth(),
            scope.clip_depth,
            "stencil crossed a layout scope"
        );
        assert_eq!(
            self.ctx.rounded_clips.last().map(|e| e.id),
            Some(scope.entry.id),
            "stencil clip order mismatch"
        );
        self.ctx.rounded_clips.pop();
        self.ctx.stencil_depth -= 1;
        let draw = crate::widgets::draw_parts(self.ctx).1;
        stamp(draw, scope.entry.shape, scope.previous.stencil[0] + 1, 2);
        draw.set_effects(scope.previous);
        self.end_panel_cell(scope.region);
        self.ctx.id_stack.pop();
    }
    pub fn stencil_clip(
        &mut self,
        label: &str,
        shape: RoundedRect,
        body: impl FnOnce(&mut Ui),
    ) -> Result<(), EffectError> {
        let scope = self.begin_stencil_clip(label, shape)?;
        body(self);
        self.end_stencil_clip(scope);
        Ok(())
    }
}
