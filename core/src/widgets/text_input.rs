use alloc::string::String;

use super::{draw_border, lighten, BORDER_LIGHTEN};
use crate::context::{TextInputState, WidgetState};
use crate::id::split_label;
use crate::input::{Key, KeyEvent};
use crate::ui::{hovered, point_in_rect, Ui};

const BLINK_PERIOD: f32 = 1.0;

const CURSOR_WIDTH: f32 = 1.0;

pub(crate) fn text_input(ui: &mut Ui, label: &str, buf: &mut String) -> bool {
    text_input_impl(ui, label, buf, true)
}

pub(crate) fn embedded_text_input(ui: &mut Ui, label: &str, buf: &mut String) -> bool {
    text_input_impl(ui, label, buf, false)
}

fn text_input_impl(ui: &mut Ui, label: &str, buf: &mut String, draw_chrome: bool) -> bool {
    if buf.len() > crate::editing::MAX_EDIT_BYTES || buf.contains('\0') {
        return false;
    }
    let ctx = &mut *ui.ctx;
    let id = super::widget_id(ctx, label);
    let display = split_label(label).0;
    let style = ctx.style;
    let face = ctx.active_font();

    let height = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
    let text_size = ctx.font.measure_in(face, display);
    let available = super::available_width(ctx);
    let field_w = if display.is_empty() {
        available
    } else {
        available - text_size[0] - style.item_spacing[0]
    }
    .max(0.0);
    let rect = super::interactive_rect(ctx, [available, height]);
    super::register_focusable_item(ctx, id, rect);
    let field = [rect[0], rect[1], rect[0] + field_w, rect[3]];

    ctx.request_cursor(id, field, crate::host_requests::CursorShape::Text);

    let mouse = ctx.input.mouse_pos;
    let inside = point_in_rect(mouse, ctx.target_rect(field));
    let left_down = ctx.input.mouse_down[0];
    let left_pressed = left_down && !ctx.prev_mouse_down[0];

    let clicked_in = left_pressed && inside;
    if ctx.focused == Some(id) && ctx.active.is_none() && !left_pressed {
        ctx.set_active(id);
    }
    if left_pressed {
        if inside {
            ctx.set_active(id);
        } else if ctx.active == Some(id) {
            ctx.clear_active();
        }
    }

    let hovered = hovered(ctx, field);
    if hovered {
        ctx.hot = Some(id);
    }
    let mut focused = ctx.active == Some(id);

    let had_state = ctx.widget_states.contains_key(&id);
    let (mut editor, mut scroll_x, mut blink, old_hash) = match ctx.widget_states.get_mut(&id) {
        Some(WidgetState::TextInput(s)) => {
            let mut editor = s
                .editor
                .take()
                .unwrap_or_else(|| alloc::boxed::Box::new(crate::editing::TextEditor::default()));
            editor.cursor = s.cursor;
            (editor, s.scroll_x, s.blink, s.text_hash)
        }
        _ => (
            alloc::boxed::Box::new(crate::editing::TextEditor::default()),
            0.,
            0.,
            0,
        ),
    };
    let hash = buf.bytes().fold(14695981039346656037u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(1099511628211)
    });
    core::mem::swap(buf, &mut editor.text);
    if hash != old_hash {
        editor.revision = editor.revision.wrapping_add(1);
        editor.cancel_composition();
    }
    if !had_state {
        editor.cursor = editor.text.len();
        editor.anchor = editor.cursor;
    }
    editor.cursor = crate::editing::model::snap(&editor.text, editor.cursor);
    editor.anchor = crate::editing::model::snap(&editor.text, editor.anchor);
    if focused {
        blink = (blink + ctx.dt) % BLINK_PERIOD;
    } else {
        editor.cancel_composition();
        editor.dragging = false;
    }
    let text_x = field[0] + style.frame_padding[0];
    let inner_w = (field_w - 2. * style.frame_padding[0]).max(0.);
    if clicked_in || focused && editor.dragging && left_down {
        editor.cancel_composition();
        editor.cursor = crate::editing::model::snap(
            &editor.text,
            byte_index_at_x(
                &mut ctx.font,
                face,
                &editor.text,
                mouse[0] - (text_x - scroll_x),
            ),
        );
        if !editor.dragging && !ctx.modifiers.shift {
            editor.anchor = editor.cursor;
        }
        editor.dragging = true;
        blink = 0.;
    }
    if !left_down {
        editor.dragging = false;
    }
    let mut changed = false;
    if focused {
        let mut n = 0;
        while n < ctx.text_host.events.len() {
            let owner = match &ctx.text_host.events[n] {
                crate::editing::Event::Ime { owner, .. }
                | crate::editing::Event::Clipboard { owner, .. } => *owner,
            };
            if owner != id {
                n += 1;
                continue;
            }
            match ctx.text_host.events.remove(n) {
                crate::editing::Event::Ime {
                    kind, text, cursor, ..
                } => match kind {
                    crate::editing::TextEventKind::Preedit
                    | crate::editing::TextEventKind::PreeditHidden
                        if !text.chars().any(crate::editing::model::is_line_break) =>
                    {
                        let _ = editor.set_preedit(&text, cursor);
                        editor.preedit_visible =
                            kind != crate::editing::TextEventKind::PreeditHidden;
                    }
                    crate::editing::TextEventKind::Commit
                        if !text.chars().any(crate::editing::model::is_line_break) =>
                    {
                        changed |= editor.commit(&text).unwrap_or(false)
                    }
                    _ => editor.cancel_composition(),
                },
                crate::editing::Event::Clipboard {
                    action,
                    text,
                    range,
                    revision,
                    ..
                } if editor.revision == revision && editor.selection() == range => {
                    if action == crate::editing::ClipboardAction::Cut {
                        changed |= editor.insert("").unwrap_or(false);
                    } else if action == crate::editing::ClipboardAction::Paste
                        && !text.chars().any(crate::editing::model::is_line_break)
                    {
                        changed |= editor.insert(&text).unwrap_or(false);
                    }
                }
                _ => {}
            }
            blink = 0.;
        }
        let mut modifiers = ctx.routing.initial_modifiers;
        for i in 0..ctx.input.keys.len() {
            match ctx.input.keys[i] {
                KeyEvent::Press(Key::Ctrl) => modifiers.ctrl = true,
                KeyEvent::Release(Key::Ctrl) => modifiers.ctrl = false,
                KeyEvent::Press(Key::Shift) => modifiers.shift = true,
                KeyEvent::Release(Key::Shift) => modifiers.shift = false,
                KeyEvent::Press(Key::Alt) => modifiers.alt = true,
                KeyEvent::Release(Key::Alt) => modifiers.alt = false,
                KeyEvent::Press(Key::A) if modifiers.ctrl => editor.select_all(),
                KeyEvent::Press(key @ (Key::C | Key::X | Key::V)) if modifiers.ctrl => {
                    let action = match key {
                        Key::C => crate::editing::ClipboardAction::Copy,
                        Key::X => crate::editing::ClipboardAction::Cut,
                        _ => crate::editing::ClipboardAction::Paste,
                    };
                    ctx.text_host.request(
                        id,
                        action,
                        editor.selected_text(),
                        editor.selection(),
                        editor.revision,
                    );
                }
                KeyEvent::Char(c)
                    if !c.is_control()
                        && !crate::editing::model::is_line_break(c)
                        && !modifiers.ctrl
                        && !modifiers.alt
                        && !editor.composing() =>
                {
                    let mut bytes = [0; 4];
                    changed |= editor.insert(c.encode_utf8(&mut bytes)).unwrap_or(false);
                }
                KeyEvent::Press(Key::Backspace) if !editor.composing() => {
                    changed |= editor.backspace()
                }
                KeyEvent::Press(Key::Delete) if !editor.composing() => changed |= editor.delete(),
                KeyEvent::Press(Key::Left) if !editor.composing() => {
                    editor.move_logical(false, modifiers.shift)
                }
                KeyEvent::Press(Key::Right) if !editor.composing() => {
                    editor.move_logical(true, modifiers.shift)
                }
                KeyEvent::Press(key @ (Key::Home | Key::End)) if !editor.composing() => {
                    editor.cursor = if key == Key::Home {
                        0
                    } else {
                        editor.text.len()
                    };
                    if !modifiers.shift {
                        editor.anchor = editor.cursor;
                    }
                }
                KeyEvent::Press(Key::Escape) if editor.composing() => editor.cancel_composition(),
                KeyEvent::Press(Key::Enter | Key::Escape) if !editor.composing() => {
                    ctx.clear_active();
                    ctx.focused = None;
                    ctx.focused_in_modal = false;
                    ctx.focused_in_popup = false;
                    focused = false;
                    blink = 0.;
                    break;
                }
                KeyEvent::Release(_) => continue,
                _ => continue,
            }
            blink = 0.;
        }
    }
    let cursor = editor.cursor;
    let display_cursor = if let Some(r) = &editor.composition_range {
        editor.display.clear();
        editor.display.push_str(&editor.text[..r.start]);
        editor.display.push_str(&editor.preedit);
        editor.display.push_str(&editor.text[r.end..]);
        r.start + editor.preedit_cursor.end
    } else {
        cursor
    };
    let rendered = if editor.composing() {
        editor.display.as_str()
    } else {
        editor.text.as_str()
    };
    let cursor_x = ctx.font.measure_in(face, &rendered[..display_cursor])[0];
    if focused {
        if cursor_x - scroll_x > inner_w {
            scroll_x = cursor_x - inner_w;
        }
        if cursor_x - scroll_x < 0.0 {
            scroll_x = cursor_x;
        }
        if scroll_x < 0.0 {
            scroll_x = 0.0;
        }
    }

    if focused {
        ctx.text_host.caret = Some(crate::editing::TextCaret {
            owner: id,
            rect: [
                text_x - scroll_x + cursor_x,
                rect[1] + style.frame_padding[1],
                text_x - scroll_x + cursor_x + 1.,
                rect[1] + style.frame_padding[1] + ctx.font.line_height_in(face),
            ],
            composing: editor.composing(),
        });
    }
    let field_bg = if focused {
        style.widget_bg_active
    } else if hovered {
        style.widget_bg_hovered
    } else {
        style.widget_bg
    };
    let field_bg = if ctx.motion.active() {
        ctx.widget_color(id, field_bg)
    } else {
        style.widget_bg
    };
    let (font, draw_list) = super::draw_parts(ctx);
    if draw_chrome {
        super::chrome_fill(font, draw_list, style.corner_rounding, field, field_bg);
        if style.border_size > 0.0 {
            draw_border(
                font,
                draw_list,
                style.corner_rounding,
                field,
                style.border_size,
                lighten(style.widget_bg, BORDER_LIGHTEN),
            );
        }
    }
    draw_list.push_clip_rect(field);
    let text_pos = [text_x - scroll_x, rect[1] + style.frame_padding[1]];
    let selection = editor
        .composition_range
        .clone()
        .map(|r| r.start..r.start + editor.preedit.len())
        .unwrap_or_else(|| editor.selection());
    if !selection.is_empty() {
        let x0 = text_pos[0] + font.measure_in(face, &rendered[..selection.start])[0];
        let x1 = text_pos[0] + font.measure_in(face, &rendered[..selection.end])[0];
        let y1 = text_pos[1] + font.line_height_in(face);
        if editor.composing() {
            draw_list.add_filled_rect([x0, y1 - 1.], [x1, y1], style.text);
        } else {
            let mut color = style.widget_bg_hovered;
            color[3] = 160;
            draw_list.add_filled_rect([x0, text_pos[1]], [x1, y1], color);
        }
    }
    draw_list.add_text(font, face, text_pos, rendered, style.text);
    if focused
        && blink % BLINK_PERIOD < BLINK_PERIOD / 2.0
        && (!editor.composing() || editor.preedit_visible)
    {
        let cx = text_pos[0] + cursor_x;
        let y0 = rect[1] + style.frame_padding[1];
        draw_list.add_filled_rect(
            [cx, y0],
            [cx + CURSOR_WIDTH, y0 + font.line_height_in(face)],
            style.text,
        );
    }
    draw_list.pop_clip_rect();
    if !display.is_empty() {
        let label_pos = [
            field[2] + style.item_spacing[0],
            rect[1] + (height - text_size[1]) / 2.0,
        ];
        draw_list.add_text(font, face, label_pos, display, style.text);
    }
    let text_hash = editor.text.bytes().fold(14695981039346656037u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(1099511628211)
    });
    core::mem::swap(buf, &mut editor.text);
    ctx.id_stack.semantics.describe(
        id,
        crate::accessibility::AccessibilityRole::TextInput,
        buf,
        0,
        crate::accessibility::AccessibilityActions::FOCUS,
    );
    {
        ctx.widget_states.insert(
            id,
            WidgetState::TextInput(TextInputState {
                cursor,
                scroll_x,
                blink,
                editor: Some(editor),
                text_hash,
            }),
        );
    }
    changed
}

fn byte_index_at_x(
    font: &mut crate::font::Font,
    face: crate::font::FontId,
    text: &str,
    rel_x: f32,
) -> usize {
    let mut pen = 0.0f32;
    for (i, ch) in text.char_indices() {
        let advance = font.glyph_quad(face, ch, [0.0; 2]).0[0];
        if rel_x < pen + advance / 2.0 {
            return i;
        }
        pen += advance;
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use crate::context::Context;
    use crate::input::{InputState, Key, KeyEvent};
    use crate::ui::Ui;
    use crate::widgets::test_support::*;

    const AWAY: [f32; 2] = [-50.0, -50.0];

    fn keyed(mouse_pos: [f32; 2], left_down: bool, keys: &[KeyEvent]) -> InputState<'_> {
        InputState {
            mouse_pos,
            mouse_down: [left_down, false, false, false, false],
            mouse_wheel: 0.0,
            keys,
            viewport_size: [0.0, 0.0],
        }
    }

    fn field_rect(ui: &mut Ui, display: &str) -> [f32; 4] {
        let pos = ui.ctx.current_window_mut().cursor.next_pos();
        let available = ui.ctx.current_window_mut().cursor.available_width();
        let style = ui.ctx.style;
        let label_w = ui.ctx.font.measure(display)[0];
        let height = ui.ctx.font.line_height() + 2.0 * style.frame_padding[1];
        let field_w = if display.is_empty() {
            available
        } else {
            (available - label_w - style.item_spacing[0]).max(0.0)
        };
        [pos[0], pos[1], pos[0] + field_w, pos[1] + height]
    }

    fn center(r: [f32; 4]) -> [f32; 2] {
        [(r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0]
    }

    fn settle(ctx: &mut Context, label: &str, display: &str, buf: &mut String) -> [f32; 4] {
        let mut field = [0.0; 4];
        frame(ctx, AWAY, false, |ui| {
            field = field_rect(ui, display);
            ui.text_input(label, buf);
        });
        field
    }

    fn click(ctx: &mut Context, label: &str, buf: &mut String, point: [f32; 2]) {
        frame(ctx, point, true, |ui| {
            ui.text_input(label, buf);
        });
        frame(ctx, point, false, |ui| {
            ui.text_input(label, buf);
        });
    }

    fn cursor_rect(ctx: &mut Context, field: [f32; 4], buf: &str, cursor: usize) -> [f32; 4] {
        let style = ctx.style;
        let cursor_x = ctx.font.measure(&buf[..cursor])[0];
        let cx = field[0] + style.frame_padding[0] + cursor_x;
        let y0 = field[1] + style.frame_padding[1];
        [cx, y0, cx + 1.0, y0 + ctx.font.line_height()]
    }

    #[test]
    fn plain_hover_sets_hot_without_a_capture() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "T", "T", &mut buf);
        frame(&mut ctx, center(field), false, |ui| {
            ui.text_input("T", &mut buf);
        });
        assert_eq!(ctx.hot, Some(widget_id_in_w("T")));
        assert_eq!(ctx.active, None, "hover alone must not focus the field");
    }

    #[test]
    fn char_events_append_text_and_report_changed() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "Name", "Name", &mut buf);
        click(&mut ctx, "Name", &mut buf, center(field));
        assert_eq!(ctx.active, Some(widget_id_in_w("Name")));

        let mut changed = false;
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('h'), KeyEvent::Char('i')]),
            |ui| changed = ui.text_input("Name", &mut buf),
        );
        assert!(changed, "a frame with Char events must report a change");
        assert_eq!(buf, "hi");

        frame(&mut ctx, AWAY, false, |ui| {
            changed = ui.text_input("Name", &mut buf)
        });
        assert!(!changed, "a frame with no events must not report a change");
        assert_eq!(buf, "hi");
    }

    #[test]
    fn cursor_movement_and_backspace_delete_the_expected_char() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "T", "T", &mut buf);
        click(&mut ctx, "T", &mut buf, center(field));

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[
                    KeyEvent::Char('a'),
                    KeyEvent::Char('b'),
                    KeyEvent::Char('c'),
                ],
            ),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "abc");

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[KeyEvent::Press(Key::Left), KeyEvent::Press(Key::Backspace)],
            ),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "ac", "Left + Backspace must delete 'b'");

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('X')]),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "aXc");
    }

    #[test]
    fn home_end_and_delete_edit_at_the_cursor() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "T", "T", &mut buf);
        click(&mut ctx, "T", &mut buf, center(field));

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[
                    KeyEvent::Char('a'),
                    KeyEvent::Char('b'),
                    KeyEvent::Char('c'),
                ],
            ),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[KeyEvent::Press(Key::Home), KeyEvent::Press(Key::Delete)],
            ),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "bc", "Home + Delete must remove 'a'");

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[KeyEvent::Press(Key::End), KeyEvent::Char('!')],
            ),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "bc!");

        let mut changed = true;
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[
                    KeyEvent::Release(Key::Backspace),
                    KeyEvent::Release(Key::Delete),
                ],
            ),
            |ui| changed = ui.text_input("T", &mut buf),
        );
        assert!(!changed, "Release events must not edit");
        assert_eq!(buf, "bc!");
    }

    #[test]
    fn click_between_two_chars_sets_the_insertion_point() {
        let mut ctx = test_context();
        let mut buf = String::from("ab");
        let field = settle(&mut ctx, "T", "T", &mut buf);

        let boundary_x = field[0] + ctx.style.frame_padding[0] + ctx.font.measure("a")[0];
        let point = [boundary_x + 0.5, center(field)[1]];
        click(&mut ctx, "T", &mut buf, point);

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('X')]),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "aXb", "the char must land at the clicked position");
    }

    #[test]
    fn focus_handoff_between_two_inputs() {
        let mut ctx = test_context();
        let mut a = String::new();
        let mut b = String::new();
        let (mut fa, mut fb) = ([0.0; 4], [0.0; 4]);
        frame(&mut ctx, AWAY, false, |ui| {
            fa = field_rect(ui, "A");
            ui.text_input("A", &mut a);
            fb = field_rect(ui, "B");
            ui.text_input("B", &mut b);
        });

        frame(&mut ctx, center(fa), true, |ui| {
            ui.text_input("A", &mut a);
            ui.text_input("B", &mut b);
        });
        frame(&mut ctx, center(fa), false, |ui| {
            ui.text_input("A", &mut a);
            ui.text_input("B", &mut b);
        });
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('x')]),
            |ui| {
                ui.text_input("A", &mut a);
                ui.text_input("B", &mut b);
            },
        );
        assert_eq!(a, "x");
        assert_eq!(b, "");
        let a_cursor = cursor_rect(&mut ctx, fa, "x", 1);
        assert_eq!(
            fill_color_at(&ctx, a_cursor),
            Some(ctx.style.text),
            "the focused field must show its cursor"
        );

        frame(&mut ctx, center(fb), true, |ui| {
            ui.text_input("A", &mut a);
            ui.text_input("B", &mut b);
        });
        frame(&mut ctx, center(fb), false, |ui| {
            ui.text_input("A", &mut a);
            ui.text_input("B", &mut b);
        });
        assert_eq!(ctx.active, Some(widget_id_in_w("B")));
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('y')]),
            |ui| {
                ui.text_input("A", &mut a);
                ui.text_input("B", &mut b);
            },
        );
        assert_eq!(a, "x", "the unfocused field must not receive keys");
        assert_eq!(b, "y");
        let a_cursor = cursor_rect(&mut ctx, fa, "x", 1);
        assert_eq!(
            fill_color_at(&ctx, a_cursor),
            None,
            "the unfocused field must not draw a cursor"
        );
    }

    #[test]
    fn unfocused_field_ignores_key_events() {
        let mut ctx = test_context();
        let mut buf = String::from("keep");
        settle(&mut ctx, "T", "T", &mut buf);

        let mut changed = true;
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[KeyEvent::Char('!'), KeyEvent::Press(Key::Backspace)],
            ),
            |ui| changed = ui.text_input("T", &mut buf),
        );
        assert!(!changed, "an unfocused field must not report a change");
        assert_eq!(buf, "keep");
        assert_eq!(ctx.active, None);
    }

    #[test]
    fn enter_and_escape_unfocus() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "T", "T", &mut buf);

        click(&mut ctx, "T", &mut buf, center(field));
        assert_eq!(ctx.active, Some(widget_id_in_w("T")));
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Press(Key::Enter)]),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(ctx.active, None, "Enter must unfocus");
        assert_eq!(ctx.focused, None);

        let mut changed = true;
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('z')]),
            |ui| {
                changed = ui.text_input("T", &mut buf);
            },
        );
        assert!(!changed);
        assert_eq!(buf, "");

        click(&mut ctx, "T", &mut buf, center(field));
        assert_eq!(ctx.active, Some(widget_id_in_w("T")));
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Press(Key::Escape)]),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(ctx.active, None, "Escape must unfocus");
        assert_eq!(ctx.focused, None);
    }

    #[test]
    fn clicking_elsewhere_unfocuses() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "T", "T", &mut buf);
        click(&mut ctx, "T", &mut buf, center(field));
        assert_eq!(ctx.active, Some(widget_id_in_w("T")));

        frame(&mut ctx, AWAY, true, |ui| {
            ui.text_input("T", &mut buf);
        });
        assert_eq!(ctx.active, None, "a click outside the field must unfocus");
        frame(&mut ctx, AWAY, false, |ui| {
            ui.text_input("T", &mut buf);
        });
        assert_eq!(ctx.active, None);
    }

    #[test]
    fn multibyte_char_keeps_cursor_on_char_boundaries() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "T", "T", &mut buf);
        click(&mut ctx, "T", &mut buf, center(field));

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('é'), KeyEvent::Char('a')]),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "éa");

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[KeyEvent::Press(Key::Left), KeyEvent::Press(Key::Backspace)],
            ),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "a", "Backspace must remove the whole multi-byte char");

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('X')]),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "Xa");
    }

    #[test]
    fn control_char_events_are_rejected() {
        let mut ctx = test_context();
        let mut buf = String::from("ab");
        let field = settle(&mut ctx, "T", "T", &mut buf);
        click(&mut ctx, "T", &mut buf, center(field));

        let mut changed = true;
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[
                    KeyEvent::Char('\n'),
                    KeyEvent::Char('\r'),
                    KeyEvent::Char('\t'),
                ],
            ),
            |ui| changed = ui.text_input("T", &mut buf),
        );
        assert!(!changed, "control chars must not report a change");
        assert_eq!(buf, "ab");

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('\t'), KeyEvent::Char('c')]),
            |ui| changed = ui.text_input("T", &mut buf),
        );
        assert!(changed);
        assert_eq!(buf, "abc");
    }

    #[test]
    fn host_mutation_between_frames_clamps_cursor_to_a_boundary() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "T", "T", &mut buf);
        click(&mut ctx, "T", &mut buf, center(field));

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[
                    KeyEvent::Char('a'),
                    KeyEvent::Char('b'),
                    KeyEvent::Char('c'),
                    KeyEvent::Char('d'),
                ],
            ),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "abcd");

        buf.truncate(2);
        let mut changed = true;
        frame(&mut ctx, AWAY, false, |ui| {
            changed = ui.text_input("T", &mut buf)
        });
        assert!(!changed, "clamping the cursor is not a buffer change");
        assert_eq!(buf, "ab");

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('X')]),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "abX");

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(
                AWAY,
                false,
                &[KeyEvent::Press(Key::Home), KeyEvent::Press(Key::Right)],
            ),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        buf.clear();
        buf.push('é');
        frame(&mut ctx, AWAY, false, |ui| {
            changed = ui.text_input("T", &mut buf)
        });
        assert!(!changed);
        assert_eq!(buf, "é");

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('Z')]),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "Zé", "the clamped cursor must sit before 'é'");
    }

    #[test]
    fn cursor_blinks_with_accumulated_dt_and_solidifies_on_key_press() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "T", "T", &mut buf);
        click(&mut ctx, "T", &mut buf, center(field));

        let solid = cursor_rect(&mut ctx, field, "", 0);
        assert_eq!(
            fill_color_at(&ctx, solid),
            Some(ctx.style.text),
            "the cursor must be solid right after focusing"
        );

        frame_with_input(&mut ctx, 0.6, keyed(AWAY, false, &[]), |ui| {
            ui.text_input("T", &mut buf);
        });
        assert_eq!(
            fill_color_at(&ctx, solid),
            None,
            "the cursor must hide in the second half of the blink period"
        );

        frame_with_input(&mut ctx, 0.5, keyed(AWAY, false, &[]), |ui| {
            ui.text_input("T", &mut buf);
        });
        assert_eq!(
            fill_color_at(&ctx, solid),
            Some(ctx.style.text),
            "the cursor must reappear when the phase wraps"
        );

        frame_with_input(&mut ctx, 0.6, keyed(AWAY, false, &[]), |ui| {
            ui.text_input("T", &mut buf);
        });
        assert_eq!(fill_color_at(&ctx, solid), None);
        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('z')]),
            |ui| {
                ui.text_input("T", &mut buf);
            },
        );
        assert_eq!(buf, "z");
        let solid_z = cursor_rect(&mut ctx, field, "z", 1);
        assert_eq!(
            fill_color_at(&ctx, solid_z),
            Some(ctx.style.text),
            "a key press must reset the cursor to solid"
        );
    }

    #[test]
    fn long_text_scrolls_to_keep_the_cursor_visible() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "##t", "", &mut buf);
        click(&mut ctx, "##t", &mut buf, center(field));

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('a'); 40]),
            |ui| {
                ui.text_input("##t", &mut buf);
            },
        );
        assert_eq!(buf.len(), 40);

        let style = ctx.style;
        let cursor_x = ctx.font.measure(&buf)[0];
        let inner_w = (field[2] - field[0] - 2.0 * style.frame_padding[0]).max(0.0);
        assert!(cursor_x > inner_w, "the test text must overflow the field");
        let scroll_x = cursor_x - inner_w;
        let cx = field[0] + style.frame_padding[0] - scroll_x + cursor_x;
        let y0 = field[1] + style.frame_padding[1];
        let cursor = [cx, y0, cx + 1.0, y0 + ctx.font.line_height()];
        assert_eq!(
            fill_color_at(&ctx, cursor),
            Some(style.text),
            "the cursor must stay visible at the field's inner right edge"
        );
        assert!(
            cx <= field[2] - style.frame_padding[0] + 1e-4,
            "the cursor must not pass the field's inner right edge"
        );

        let state = ctx
            .widget_states
            .get(&widget_id_in_w("##t"))
            .expect("a focused field must have state");
        let crate::context::WidgetState::TextInput(s) = state else {
            panic!("a text input's state must be the TextInput variant");
        };
        assert!(
            s.scroll_x > 0.0,
            "overflowing text must scroll to keep the cursor visible"
        );
    }

    #[test]
    fn home_on_overflowed_text_scrolls_back_to_the_left_edge() {
        let mut ctx = test_context();
        let mut buf = String::new();
        let field = settle(&mut ctx, "##t", "", &mut buf);
        click(&mut ctx, "##t", &mut buf, center(field));

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Char('a'); 40]),
            |ui| {
                ui.text_input("##t", &mut buf);
            },
        );
        let crate::context::WidgetState::TextInput(before) = ctx
            .widget_states
            .get(&widget_id_in_w("##t"))
            .expect("a focused field must have state")
        else {
            panic!("a text input's state must be the TextInput variant");
        };
        assert!(
            before.scroll_x > 0.0,
            "the test text must overflow the field"
        );

        frame_with_input(
            &mut ctx,
            0.016,
            keyed(AWAY, false, &[KeyEvent::Press(Key::Home)]),
            |ui| {
                ui.text_input("##t", &mut buf);
            },
        );
        let crate::context::WidgetState::TextInput(after) = ctx
            .widget_states
            .get(&widget_id_in_w("##t"))
            .expect("a focused field must have state")
        else {
            panic!("a text input's state must be the TextInput variant");
        };
        assert_eq!(after.cursor, 0);
        assert_eq!(
            after.scroll_x, 0.0,
            "Home must scroll an overflowed field back to the left edge"
        );
    }
}
