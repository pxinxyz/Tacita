#[cfg(test)]
mod tests;
use crate::{context::Context, id::Id};
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorShape {
    #[default]
    Default,
    Pointer,
    Text,
    ResizeHorizontal,
    ResizeVertical,
    ResizeDiagonal,
    Grab,
    Grabbing,
    NotAllowed,
}
impl CursorShape {
    pub fn from_code(c: u32) -> Option<Self> {
        Some(match c {
            0 => Self::Default,
            1 => Self::Pointer,
            2 => Self::Text,
            3 => Self::ResizeHorizontal,
            4 => Self::ResizeVertical,
            5 => Self::ResizeDiagonal,
            6 => Self::Grab,
            7 => Self::Grabbing,
            8 => Self::NotAllowed,
            _ => return None,
        })
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HitTargetPolicy {
    pub minimum: [f32; 2],
}
impl HitTargetPolicy {
    pub fn is_valid(self) -> bool {
        self.minimum
            .iter()
            .all(|v| v.is_finite() && *v >= 0. && *v <= 256.)
    }
}
#[derive(Default)]
pub(crate) struct HostRequests {
    pub policy: HitTargetPolicy,
    pub cursor: CursorShape,
    cursor_target: Option<[f32; 4]>,
    pub wake: Option<f32>,
    pub invalidated: bool,
}
impl Context {
    pub fn set_hit_target_policy(&mut self, p: HitTargetPolicy) {
        assert!(
            !self.frame_active && p.is_valid(),
            "invalid hit-target policy or active frame"
        );
        if self.host_requests.policy != p {
            self.host_requests.policy = p;
            self.invalidate();
        }
    }
    pub fn hit_target_policy(&self) -> HitTargetPolicy {
        self.host_requests.policy
    }
    pub fn cursor_shape(&self) -> CursorShape {
        self.host_requests.cursor
    }
    pub fn invalidate(&mut self) {
        self.host_requests.invalidated = true;
    }
    pub fn next_redraw_in(&self) -> Option<f32> {
        if self.host_requests.invalidated {
            Some(0.)
        } else {
            self.host_requests.wake
        }
    }
    pub fn request_redraw_in(&mut self, seconds: f32) {
        assert!(
            seconds.is_finite() && seconds >= 0.,
            "invalid wake interval"
        );
        self.host_requests.wake = Some(
            self.host_requests
                .wake
                .map_or(seconds, |old| old.min(seconds)),
        );
    }
    pub(crate) fn reset_host_requests(&mut self) {
        self.host_requests.wake = None;
        self.host_requests.invalidated = false;
        self.host_requests.cursor = CursorShape::Default;
        self.host_requests.cursor_target = None;
    }
    pub(crate) fn finish_host_requests(&mut self) {
        if self.active.is_none()
            && self
                .host_requests
                .cursor_target
                .is_some_and(|rect| !crate::ui::point_in_rect(self.input.mouse_pos, rect))
        {
            self.host_requests.cursor = CursorShape::Default;
        }
        if self.input.mouse_down.iter().any(|b| *b) {
            self.request_redraw_in(1. / 144.);
        }
        if !self.toasts.is_empty() {
            self.request_redraw_in(1. / 144.);
        }
        if self.animation_needs_redraw() {
            self.request_redraw_in(1. / 144.);
        }
        if let Some(caret) = self.text_host.caret {
            if self.focused == Some(caret.owner) {
                self.request_redraw_in(0.25);
            }
        }
        if self.routing.popups.iter().any(|p| p.motion.needs_redraw())
            || self.popup_animation.needs_redraw()
        {
            self.request_redraw_in(1. / 144.);
        }
    }
    pub(crate) fn request_cursor(&mut self, id: Id, rect: [f32; 4], shape: CursorShape) {
        if self.disabled_depth > 0
            || !self.popup_input_allowed()
            || (self.popup_modal && self.open_popup.is_some() && !self.popup_scope_active)
        {
            return;
        }
        if self.active == Some(id) || (self.active.is_none() && crate::ui::hovered(self, rect)) {
            self.host_requests.cursor = shape;
            self.host_requests.cursor_target = Some(self.semantic_clip(self.target_rect(rect)));
        }
    }
    pub(crate) fn target_rect(&self, r: [f32; 4]) -> [f32; 4] {
        let p = self.host_requests.policy.minimum;
        if p == [0.; 2] {
            return r;
        }
        let dx = (p[0] - (r[2] - r[0])).max(0.) * 0.5;
        let dy = (p[1] - (r[3] - r[1])).max(0.) * 0.5;
        self.semantic_clip([r[0] - dx, r[1] - dy, r[2] + dx, r[3] + dy])
    }
}
impl crate::ui::Ui<'_> {
    pub fn request_cursor(&mut self, id: Id, rect: [f32; 4], shape: CursorShape) {
        self.ctx.request_cursor(id, rect, shape);
    }
    pub fn request_redraw_in(&mut self, seconds: f32) {
        self.ctx.request_redraw_in(seconds);
    }
    pub fn hit_target_rect(&self, rect: [f32; 4]) -> [f32; 4] {
        self.ctx.target_rect(rect)
    }
}
