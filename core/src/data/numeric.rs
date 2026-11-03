use super::{click, DataError};
use crate::{id::split_label, ui::Ui};
use alloc::string::String;
use core::fmt::Write;

#[derive(Default, Debug)]
pub struct NumericEdit {
    pub(crate) text: String,
    focused: bool,
    invalid: bool,
    original: f64,
}

impl NumericEdit {
    pub fn invalid(&self) -> bool {
        self.invalid
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    fn format_float(&mut self, value: f32) {
        self.text.clear();
        let _ = write!(self.text, "{}", value);
        self.invalid = false;
    }
    fn format_int(&mut self, value: i32) {
        self.text.clear();
        let _ = write!(self.text, "{}", value);
        self.invalid = false;
    }
    pub(crate) fn commit_float(&mut self, value: &mut f32, min: f32, max: f32) -> bool {
        let Ok(parsed) = self.text.trim().parse::<f32>() else {
            self.invalid = true;
            return false;
        };
        if !parsed.is_finite() {
            self.invalid = true;
            return false;
        }
        let next = parsed.clamp(min, max);
        let changed = next != *value;
        *value = next;
        self.format_float(next);
        changed
    }
    pub(crate) fn commit_int(&mut self, value: &mut i32, min: i32, max: i32) -> bool {
        let Ok(parsed) = self.text.trim().parse::<i32>() else {
            self.invalid = true;
            return false;
        };
        let next = parsed.clamp(min, max);
        let changed = next != *value;
        *value = next;
        self.format_int(next);
        changed
    }
}
struct EditResult {
    commit: bool,
    cancel: bool,
    step: i32,
}
fn field(ui: &mut Ui<'_>, label: &str, edit: &mut NumericEdit, value: f64) -> EditResult {
    let style = ui.ctx.style;
    let height = ui.ctx.font.line_height_in(ui.ctx.active_font()) + 2. * style.frame_padding[1];
    let width = crate::widgets::available_width(ui.ctx);
    let rect = ui.allocate([width, height]);
    let label_w = ui.measure_text(split_label(label).0)[0];
    let available = (rect[2] - rect[0]).max(0.);
    let label_w = (label_w + style.item_spacing[0]).min(available * 0.4);
    let button_w = height.min(available * 0.2);
    let id = ui.widget_id(label);
    ui.ctx.id_stack.push_id(id);
    ui.draw_text_at(
        [rect[0], rect[1] + style.frame_padding[1]],
        split_label(label).0,
        style.text,
    );
    let field_rect = [
        rect[0] + label_w,
        rect[1],
        (rect[2] - button_w * 2.).max(rect[0] + label_w),
        rect[3],
    ];
    let previous_left = ui.ctx.prev_mouse_down[0];
    let cell = crate::panels::begin_cell(ui, field_rect, 0, 0);
    if !crate::ui::point_in_rect(ui.mouse_pos(), field_rect) {
        ui.ctx.prev_mouse_down[0] = previous_left;
    }
    let field_id = ui.widget_id("##numeric_value");
    let was = edit.focused;
    let started = was
        || ui.is_focused(field_id)
        || ui.is_captured(field_id)
        || ui.mouse_pressed(0)
            && crate::ui::point_in_rect(ui.mouse_pos(), field_rect)
            && crate::ui::hover_layer_gate(ui.ctx);
    if started && !was {
        edit.original = value;
    }
    let cancel = started
        && ui
            .ctx
            .input
            .keys
            .contains(&crate::input::KeyEvent::Press(crate::input::Key::Escape));
    ui.text_input("##numeric_value", &mut edit.text);
    let focused = ui.is_captured(field_id);
    if focused && !was {
        edit.original = value;
    }
    let enter = ui
        .ctx
        .input
        .keys
        .contains(&crate::input::KeyEvent::Press(crate::input::Key::Enter));
    let commit = (started && !focused || focused && enter) && !cancel && ui.ctx.disabled_depth == 0;
    edit.focused = focused;
    ui.end_panel_cell(cell);
    if edit.invalid {
        let color = ui.ctx.theme_color(crate::theme::ThemeRole::Danger);
        crate::widgets::draw_parts(ui.ctx).1.add_filled_rect(
            [field_rect[0], field_rect[3] - 2.],
            [field_rect[2], field_rect[3]],
            color,
        );
    }
    let mut step = 0;
    for (i, text) in ["-", "+"].iter().enumerate() {
        let r = [
            rect[2] - button_w * (2 - i) as f32,
            rect[1],
            rect[2] - button_w * (1 - i) as f32,
            rect[3],
        ];
        ui.ctx.id_stack.push_id(crate::id::Id((i + 1) as u64));
        let id = ui.widget_id("##step");
        ui.ctx.id_stack.pop();
        if click(ui, id, r) {
            step = if i == 0 { -1 } else { 1 };
        }
        let color = if ui.is_captured(id) {
            style.widget_bg_active
        } else if ui.ctx.hot == Some(id) {
            style.widget_bg_hovered
        } else {
            style.widget_bg
        };
        let color = ui.ctx.widget_color(id, color);
        let (font, draw) = crate::widgets::draw_parts(ui.ctx);
        crate::widgets::chrome_fill(font, draw, 0., r, color);
        let text_w = ui.measure_text(text)[0];
        ui.draw_text_at(
            [
                r[0] + (button_w - text_w) * 0.5,
                r[1] + style.frame_padding[1],
            ],
            text,
            style.text,
        );
    }
    ui.ctx.id_stack.pop();
    EditResult {
        commit,
        cancel,
        step,
    }
}
impl Ui<'_> {
    pub fn input_float(
        &mut self,
        label: &str,
        value: &mut f32,
        edit: &mut NumericEdit,
        step: f32,
        min: f32,
        max: f32,
    ) -> Result<bool, DataError> {
        if ![step, min, max, *value].iter().all(|v| v.is_finite()) || step <= 0. || min > max {
            return Err(DataError::InvalidInput);
        }
        if !edit.focused && !edit.invalid {
            edit.format_float(*value);
        }
        let action = field(self, label, edit, *value as f64);
        if action.cancel {
            edit.format_float(edit.original as f32);
            return Ok(false);
        }
        let mut changed = if action.commit {
            edit.commit_float(value, min, max)
        } else {
            false
        };
        if action.step != 0 {
            let next = (*value as f64 + action.step as f64 * step as f64)
                .clamp(min as f64, max as f64) as f32;
            changed |= next != *value;
            *value = next;
            edit.format_float(next);
            edit.focused = false;
        }
        Ok(changed)
    }
    pub fn input_int(
        &mut self,
        label: &str,
        value: &mut i32,
        edit: &mut NumericEdit,
        step: i32,
        min: i32,
        max: i32,
    ) -> Result<bool, DataError> {
        if step <= 0 || min > max {
            return Err(DataError::InvalidInput);
        }
        if !edit.focused && !edit.invalid {
            edit.format_int(*value);
        }
        let action = field(self, label, edit, *value as f64);
        if action.cancel {
            edit.format_int(edit.original as i32);
            return Ok(false);
        }
        let mut changed = if action.commit {
            edit.commit_int(value, min, max)
        } else {
            false
        };
        if action.step != 0 {
            let next = (*value as i64 + action.step as i64 * step as i64)
                .clamp(min as i64, max as i64) as i32;
            changed |= next != *value;
            *value = next;
            edit.format_int(next);
            edit.focused = false;
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        context::Context,
        input::{InputState, Key, KeyEvent},
        ScrollFlags,
    };
    const FONT: &[u8] = include_bytes!("../../tests/assets/DejaVuSans.ttf");

    #[test]
    fn numeric_focus_and_enter_in_one_frame_commits_and_escape_cancels() {
        let mut ctx = Context::new(FONT).unwrap();
        ctx.set_window_rect("W", [0., 0.], [400., 300.]);
        let mut edit = NumericEdit::default();
        let mut value = 1;
        let frame = |ctx: &mut Context,
                     edit: &mut NumericEdit,
                     value: &mut i32,
                     keys: &[KeyEvent],
                     focus: Option<crate::id::Id>| {
            ctx.set_input(InputState {
                keys,
                ..InputState::default()
            });
            ctx.new_frame(0.016);
            if let Some(id) = focus {
                ctx.focused = Some(id);
            }
            let mut changed = false;
            ctx.ui().window("W", ScrollFlags::NONE, |ui| {
                changed = ui.input_int("N", value, edit, 1, 0, 100).unwrap();
            });
            let id = ctx.focus_order[0];
            ctx.render();
            (id, changed)
        };
        let (id, _) = frame(&mut ctx, &mut edit, &mut value, &[], None);
        let (_, changed) = frame(
            &mut ctx,
            &mut edit,
            &mut value,
            &[
                KeyEvent::Press(Key::Home),
                KeyEvent::Press(Key::Delete),
                KeyEvent::Char('4'),
                KeyEvent::Char('2'),
                KeyEvent::Press(Key::Enter),
            ],
            Some(id),
        );
        assert!(changed);
        assert_eq!(value, 42);
        frame(
            &mut ctx,
            &mut edit,
            &mut value,
            &[
                KeyEvent::Press(Key::Home),
                KeyEvent::Press(Key::Delete),
                KeyEvent::Press(Key::Delete),
                KeyEvent::Char('-'),
            ],
            Some(id),
        );
        assert_eq!(edit.text(), "-");
        assert_eq!(value, 42);
        let (_, changed) = frame(
            &mut ctx,
            &mut edit,
            &mut value,
            &[KeyEvent::Press(Key::Escape)],
            None,
        );
        assert!(!changed);
        assert_eq!(value, 42);
        assert_eq!(edit.text(), "42");
    }
}
