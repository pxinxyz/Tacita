use crate::{Gui, Id};
use alloc::string::String;
pub use tacita::accessibility::{
    AccessibilityAction, AccessibilityActionKind, AccessibilityActions, AccessibilityNode,
    AccessibilityRole, AccessibilityState,
};
pub use tacita::diagnostics::DuplicateId;
pub use tacita::host_requests::{CursorShape, HitTargetPolicy};
impl Gui {
    pub fn set_accessibility_enabled(&mut self, on: bool) {
        self.ctx.set_accessibility_enabled(on);
    }
    pub fn accessibility_enabled(&self) -> bool {
        self.ctx.accessibility_enabled()
    }
    pub fn accessibility_count(&self) -> usize {
        self.ctx.accessibility_count()
    }
    pub fn accessibility_node(&self, index: usize) -> Option<AccessibilityNode> {
        self.ctx.accessibility_node(index)
    }
    pub fn accessibility_name(&self, id: Id, out: &mut String) {
        self.ctx.accessibility_name(id, out);
    }
    pub fn accessibility_value(&self, id: Id, out: &mut String) {
        self.ctx.accessibility_value(id, out);
    }
    pub fn accessibility_overflowed(&self) -> bool {
        self.ctx.accessibility_overflowed()
    }
    pub fn submit_accessibility_action(&mut self, action: AccessibilityAction) -> bool {
        self.ctx.submit_accessibility_action(action)
    }
    pub fn accessibility_info(
        &mut self,
        id: Id,
        role: AccessibilityRole,
        value: &str,
        states: u32,
        actions: u32,
    ) {
        if self.emitting() {
            self.ctx
                .ui()
                .accessibility_info(id, role, value, states, actions);
        }
    }
    pub fn set_accessibility_name(&mut self, id: Id, name: &str) {
        if self.emitting() {
            self.ctx.ui().accessibility_name(id, name);
        }
    }
    pub fn set_hit_target_policy(&mut self, p: HitTargetPolicy) {
        self.ctx.set_hit_target_policy(p);
    }
    pub fn hit_target_policy(&self) -> HitTargetPolicy {
        self.ctx.hit_target_policy()
    }
    pub fn hit_target_rect(&mut self, rect: [f32; 4]) -> [f32; 4] {
        self.ctx.ui().hit_target_rect(rect)
    }
    pub fn cursor_shape(&self) -> CursorShape {
        self.ctx.cursor_shape()
    }
    pub fn request_cursor(&mut self, id: Id, rect: [f32; 4], shape: CursorShape) {
        if self.emitting() {
            self.ctx.ui().request_cursor(id, rect, shape);
        }
    }
    pub fn invalidate(&mut self) {
        self.ctx.invalidate();
    }
    pub fn next_redraw_in(&self) -> Option<f32> {
        self.ctx.next_redraw_in()
    }
    pub fn request_redraw_in(&mut self, seconds: f32) {
        self.ctx.request_redraw_in(seconds);
    }
    pub fn set_duplicate_id_diagnostics(&mut self, on: bool) {
        self.ctx.set_duplicate_id_diagnostics(on);
    }
    pub fn duplicate_ids(&self) -> &[DuplicateId] {
        self.ctx.duplicate_ids()
    }
    pub fn duplicate_ids_overflowed(&self) -> bool {
        self.ctx.duplicate_ids_overflowed()
    }
}
