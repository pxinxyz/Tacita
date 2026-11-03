use super::DataError;
use crate::{
    id::Id,
    input::{Key, KeyEvent},
    ui::Ui,
};

pub(super) const SEPARATION: f32 = 0.0001;
pub(super) struct Canvas {
    pub id: Id,
    pub rect: [f32; 4],
    outer: [f32; 4],
    pub visible: bool,
}
impl Canvas {
    pub fn begin(
        ui: &mut Ui,
        label: &str,
        size: [f32; 2],
        axes: bool,
        interactive: bool,
    ) -> Result<Self, DataError> {
        if !size.iter().all(|v| v.is_finite())
            || !(80.0..=4096.0).contains(&size[0])
            || !(50.0..=2048.0).contains(&size[1])
        {
            return Err(DataError::InvalidInput);
        }
        let display = crate::id::split_label(label).0;
        if !display.is_empty() {
            ui.label(display);
        }
        let id = ui.widget_id(label);
        let outer = ui.allocate(size);
        if interactive {
            crate::widgets::register_focusable_item(ui.ctx, id, outer);
        } else {
            crate::widgets::register_item(ui.ctx, id, outer);
        }
        let parent = crate::widgets::draw_parts(ui.ctx).1.current_clip();
        let shown = crate::draw::intersect_rects(outer, parent);
        let visible = shown[2] > shown[0] && shown[3] > shown[1];
        let rect = [
            outer[0] + if axes { 42. } else { 8. },
            outer[1] + 8.,
            outer[2] - 8.,
            outer[3] - if axes { 24. } else { 8. },
        ];
        crate::widgets::draw_parts(ui.ctx).1.push_clip_rect(outer);
        if visible {
            let style = ui.ctx.style;
            ui.draw_rect_filled([outer[0], outer[1]], [outer[2], outer[3]], style.widget_bg);
            ui.draw_rect_outline(
                [outer[0], outer[1]],
                [outer[2], outer[3]],
                style.scrollbar_bg,
                1.,
            );
            if axes {
                for i in 0..=4 {
                    let t = i as f32 / 4.;
                    let x = rect[0] + t * (rect[2] - rect[0]);
                    let y = rect[1] + t * (rect[3] - rect[1]);
                    ui.draw_line([x, rect[1]], [x, rect[3]], style.scrollbar_bg, 1.);
                    ui.draw_line([rect[0], y], [rect[2], y], style.scrollbar_bg, 1.);
                }
            }
        }
        Ok(Self {
            id,
            rect,
            outer,
            visible,
        })
    }
    pub fn close(self, ui: &mut Ui) {
        crate::widgets::draw_parts(ui.ctx).1.pop_clip_rect();
    }
    pub fn point(&self, x: f32, y: f32) -> [f32; 2] {
        [
            self.rect[0] + x * (self.rect[2] - self.rect[0]),
            self.rect[3] - y * (self.rect[3] - self.rect[1]),
        ]
    }
    pub fn value(&self, p: [f32; 2]) -> [f32; 2] {
        [
            ((p[0] - self.rect[0]) / (self.rect[2] - self.rect[0])).clamp(0., 1.),
            ((self.rect[3] - p[1]) / (self.rect[3] - self.rect[1])).clamp(0., 1.),
        ]
    }
    pub fn pressed(&self, ui: &Ui, button: usize) -> bool {
        self.visible
            && ui.ctx.disabled_depth == 0
            && ui.mouse_pressed(button)
            && crate::ui::point_in_rect(ui.mouse_pos(), self.outer)
            && crate::ui::hover_layer_gate(ui.ctx)
    }
    pub fn key(&self, ui: &Ui, key: Key) -> bool {
        ui.ctx.disabled_depth == 0
            && ui.ctx.popup_input_allowed()
            && ui.is_focused(self.id)
            && ui.ctx.input.keys.contains(&KeyEvent::Press(key))
    }
}
pub(super) fn distance_squared(a: [f32; 2], b: [f32; 2]) -> f32 {
    let x = a[0] - b[0];
    let y = a[1] - b[1];
    x * x + y * y
}
pub(super) fn marker(ui: &mut Ui, p: [f32; 2], color: [u8; 4], selected: bool) {
    let r = if selected { 5. } else { 4. };
    let edge = ui.ctx.style.window_bg;
    ui.draw_rect_filled(
        [p[0] - r - 1., p[1] - r - 1.],
        [p[0] + r + 1., p[1] + r + 1.],
        edge,
    );
    ui.draw_rect_filled([p[0] - r, p[1] - r], [p[0] + r, p[1] + r], color);
}
