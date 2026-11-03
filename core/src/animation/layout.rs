use super::Easing;
use crate::panels::PanelCellScope;
use crate::ui::Ui;

pub struct AnimatedLayoutScope {
    cell: PanelCellScope,
}

pub fn valid_rect(rect: [f32; 4]) -> bool {
    rect.iter().all(|v| v.is_finite())
        && rect[2] >= rect[0]
        && rect[3] >= rect[1]
        && (rect[2] - rect[0]).is_finite()
        && (rect[3] - rect[1]).is_finite()
}

impl Ui<'_> {
    pub fn animated_layout(
        &mut self,
        label: &str,
        target: [f32; 4],
        duration: f32,
        easing: Easing,
        body: impl FnOnce(&mut Ui, [f32; 4]),
    ) {
        let (scope, rect) = self.begin_animated_layout(label, target, duration, easing);
        body(self, rect);
        self.end_animated_layout(scope);
    }
    pub fn begin_animated_layout(
        &mut self,
        label: &str,
        target: [f32; 4],
        duration: f32,
        easing: Easing,
    ) -> (AnimatedLayoutScope, [f32; 4]) {
        let id = self.widget_id(label);
        let rect = self.ctx.animate_rect(id, target, target, duration, easing);
        self.ctx.id_stack.push_id(id);
        let cell = crate::panels::begin_cell(self, rect, 0, 0);
        (AnimatedLayoutScope { cell }, rect)
    }
    pub fn end_animated_layout(&mut self, scope: AnimatedLayoutScope) {
        self.end_panel_cell(scope.cell);
        self.ctx.id_stack.pop();
    }
    pub fn animate_rect(
        &mut self,
        id: crate::id::Id,
        initial: [f32; 4],
        target: [f32; 4],
        duration: f32,
        easing: Easing,
    ) -> [f32; 4] {
        self.ctx.animate_rect(id, initial, target, duration, easing)
    }
}

impl crate::context::Context {
    pub fn animate_rect(
        &mut self,
        id: crate::id::Id,
        initial: [f32; 4],
        target: [f32; 4],
        duration: f32,
        easing: Easing,
    ) -> [f32; 4] {
        assert!(
            self.frame_active && valid_rect(initial) && valid_rect(target),
            "invalid animated rectangle"
        );
        self.motion_rect(id, initial, target, duration, easing)
    }
}
