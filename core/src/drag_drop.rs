use crate::id::Id;
use crate::ui::{hover_layer_gate, point_in_rect, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragPayload {
    pub type_id: u64,
    pub len: u8,
    pub bytes: [u8; 32],
}

impl DragPayload {
    pub fn new(type_id: u64, bytes: &[u8]) -> Option<Self> {
        if bytes.len() > 32 {
            return None;
        }
        let mut payload = Self {
            type_id,
            len: bytes.len() as u8,
            bytes: [0; 32],
        };
        payload.bytes[..bytes.len()].copy_from_slice(bytes);
        Some(payload)
    }

    pub fn data(&self) -> &[u8] {
        &self.bytes[..(self.len as usize).min(32)]
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DragState {
    pub source: Id,
    pub origin: [f32; 2],
    pub payload: DragPayload,
    pub started: bool,
}

impl Ui<'_> {
    pub fn drag_source(&mut self, id: Id, rect: [f32; 4], payload: DragPayload) -> bool {
        let ctx = &mut *self.ctx;
        if ctx.disabled_depth > 0 || payload.len > 32 {
            return false;
        }
        let mouse = ctx.input.mouse_pos;
        let pressed = ctx.input.mouse_down[0] && !ctx.prev_mouse_down[0];
        if pressed && point_in_rect(mouse, rect) && hover_layer_gate(ctx) {
            ctx.drag_state = Some(DragState {
                source: id,
                origin: mouse,
                payload,
                started: false,
            });
            ctx.set_active(id);
        }
        let Some(state) = ctx.drag_state.as_mut() else {
            return false;
        };
        if state.source != id {
            return false;
        }
        if ctx.input.mouse_down[0] {
            let dx = mouse[0] - state.origin[0];
            let dy = mouse[1] - state.origin[1];
            state.started |= dx * dx + dy * dy >= 16.0;
        }
        state.started
    }

    pub fn drag_target(&mut self, rect: [f32; 4], type_id: u64) -> Option<DragPayload> {
        let ctx = &mut *self.ctx;
        if ctx.disabled_depth > 0
            || ctx.input.mouse_down[0]
            || !ctx.prev_mouse_down[0]
            || !point_in_rect(ctx.input.mouse_pos, rect)
            || !hover_layer_gate(ctx)
        {
            return None;
        }
        let state = ctx.drag_state?;
        if !state.started || state.payload.type_id != type_id {
            return None;
        }
        ctx.drag_state = None;
        if ctx.active == Some(state.source) {
            ctx.clear_active();
        }
        Some(state.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::test_support::{frame, test_context};

    #[test]
    fn payload_preserves_type_and_bytes_without_borrowing_source() {
        let source = [1u8, 2, 3, 4];
        let payload = DragPayload::new(7, &source).unwrap();
        assert_eq!(payload.type_id, 7);
        assert_eq!(payload.data(), &source);
        assert!(DragPayload::new(7, &[0; 33]).is_none());
    }

    #[test]
    fn drag_requires_threshold_and_target_accepts_once_on_release() {
        let mut ctx = test_context();
        ctx.set_window_rect("W", [0.0, 0.0], [400.0, 300.0]);
        let payload = DragPayload::new(9, &[42]).unwrap();
        fn step(
            ctx: &mut crate::context::Context,
            mouse: [f32; 2],
            down: bool,
            payload: DragPayload,
        ) -> ([f32; 4], [f32; 4], Option<DragPayload>) {
            let mut source = [0.0; 4];
            let mut target = [0.0; 4];
            let mut accepted = None;
            frame(ctx, mouse, down, |ui| {
                let id = ui.widget_id("source");
                source = ui.allocate([80.0, 24.0]);
                ui.drag_source(id, source, payload);
                target = ui.allocate([80.0, 24.0]);
                accepted = ui.drag_target(target, 9);
            });
            (source, target, accepted)
        }
        let (source, target, _) = step(&mut ctx, [-50.0, -50.0], false, payload);
        let start = [(source[0] + source[2]) * 0.5, (source[1] + source[3]) * 0.5];
        let end = [(target[0] + target[2]) * 0.5, (target[1] + target[3]) * 0.5];
        assert!(step(&mut ctx, start, true, payload).2.is_none());
        assert!(step(&mut ctx, end, true, payload).2.is_none());
        assert_eq!(step(&mut ctx, end, false, payload).2, Some(payload));
        assert!(step(&mut ctx, end, false, payload).2.is_none());
    }
}
