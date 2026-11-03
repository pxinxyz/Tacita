use super::{chrome_fill, draw_border, lighten, BORDER_LIGHTEN};
use crate::font::ShapeKey;
use crate::id::{split_label, Id};
use crate::ui::{hover_layer_gate, hovered, point_in_rect, Ui};

const PILL_WIDTH: f32 = 34.0;
const PILL_HEIGHT: f32 = 18.0;
const DOT_RADIUS: f32 = 7.0;

pub(crate) fn pill_toggle(ui: &mut Ui, label: &str, value: &mut bool) -> bool {
    let ctx = &mut *ui.ctx;
    let id = super::widget_id(ctx, label);
    let display = split_label(label).0;
    let style = ctx.style;
    let face = ctx.active_font();
    let line_height = ctx.font.line_height_in(face);
    let text_size = ctx.font.measure_in(face, display);
    let height = line_height.max(PILL_HEIGHT);
    let width = PILL_WIDTH
        + if display.is_empty() {
            0.0
        } else {
            style.item_spacing[0] + text_size[0]
        };
    let rect = super::interactive_rect(ctx, [width, height]);
    super::register_focusable_item(ctx, id, rect);

    let inside = point_in_rect(ctx.input.mouse_pos, ctx.target_rect(rect));
    let down = ctx.input.mouse_down[0];
    if inside && hover_layer_gate(ctx) && down && !ctx.prev_mouse_down[0] {
        ctx.set_active(id);
    }
    let held = ctx.active == Some(id);
    let changed =
        (held && !down && inside && hover_layer_gate(ctx)) || ctx.keyboard_activate == Some(id);
    if held && !down {
        ctx.clear_active();
    }
    if changed {
        *value = !*value;
    }
    let over = hovered(ctx, rect);
    if over {
        ctx.hot = Some(id);
    }

    ctx.id_stack.semantics.describe(
        id,
        crate::accessibility::AccessibilityRole::CheckBox,
        if *value { "on" } else { "off" },
        if *value {
            crate::accessibility::AccessibilityState::CHECKED
        } else {
            0
        },
        3,
    );
    let track = [
        rect[0],
        rect[1] + (height - PILL_HEIGHT) * 0.5,
        rect[0] + PILL_WIDTH,
        rect[1] + (height + PILL_HEIGHT) * 0.5,
    ];
    let track_color = if *value {
        if over || (ctx.active == Some(id) && inside) {
            lighten(style.widget_bg_active, 24)
        } else {
            style.widget_bg_active
        }
    } else if over || (ctx.active == Some(id) && inside) {
        lighten(style.widget_bg, 24)
    } else {
        lighten(style.widget_bg, 12)
    };
    let track_color = ctx.widget_color(id, track_color);
    let amount = ctx.motion_tween(
        id,
        crate::animation::INDICATOR,
        if *value { 1.0 } else { 0.0 },
        if *value { 1.0 } else { 0.0 },
        ctx.motion.indicator,
        ctx.motion.easing,
    );
    let dot = [
        track[0] + DOT_RADIUS + 2.0 + (PILL_WIDTH - 2.0 * (DOT_RADIUS + 2.0)) * amount,
        (track[1] + track[3]) * 0.5,
    ];
    let (font, draw_list) = super::draw_parts(ctx);
    chrome_fill(font, draw_list, PILL_HEIGHT * 0.5, track, track_color);
    if !super::radio::circle_mask(font, draw_list, ShapeKey::disk(DOT_RADIUS), dot, style.text) {
        draw_list.add_filled_circle(dot, DOT_RADIUS, style.text, false);
    }
    if !display.is_empty() {
        draw_list.add_text(
            font,
            face,
            [
                track[2] + style.item_spacing[0],
                rect[1] + (height - text_size[1]) * 0.5,
            ],
            display,
            style.text,
        );
    }
    changed
}

fn segment_id(ctx: &mut crate::context::Context, group: Id, index: usize) -> Id {
    ctx.id_stack.push_id(group);
    let id = ctx.id_stack.id_index(index);
    ctx.id_stack.pop();
    id
}

fn segment_width(
    font: &mut crate::font::Font,
    face: crate::font::FontId,
    text: &str,
    pad_x: f32,
) -> f32 {
    (font.measure_in(face, split_label(text).0)[0] + 2.0 * pad_x).max(44.0)
}

pub(crate) fn segmented_control(
    ui: &mut Ui,
    label: &str,
    items: &[&str],
    selected: &mut usize,
) -> bool {
    let ctx = &mut *ui.ctx;
    let group_id = super::widget_id(ctx, label);
    let display = split_label(label).0;
    let style = ctx.style;
    let face = ctx.active_font();
    let label_size = ctx.font.measure_in(face, display);
    let height = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
    let label_width = if display.is_empty() {
        0.0
    } else {
        label_size[0] + style.item_spacing[0]
    };
    let mut segments_width = 0.0;
    for item in items {
        segments_width += segment_width(&mut ctx.font, face, item, style.frame_padding[0]);
    }
    let rect = super::interactive_rect(ctx, [label_width + segments_width, height]);
    if items.is_empty() {
        super::register_item(ctx, group_id, rect);
    } else {
        super::register_focusable_item(ctx, group_id, rect);
    }
    let panel = [rect[0] + label_width, rect[1], rect[2], rect[3]];

    let mouse = ctx.input.mouse_pos;
    let down = ctx.input.mouse_down[0];
    let press = down && !ctx.prev_mouse_down[0] && hover_layer_gate(ctx);
    let release = !down && hover_layer_gate(ctx);
    let mut x = panel[0];
    let mut changed = false;
    let mut pressed_index = None;
    for (index, item) in items.iter().enumerate() {
        let width = segment_width(&mut ctx.font, face, item, style.frame_padding[0]);
        let segment = [x, rect[1], x + width, rect[3]];
        let inside = point_in_rect(mouse, segment);
        let id = segment_id(ctx, group_id, index);
        if inside && press {
            ctx.set_active(id);
        }
        if ctx.active == Some(id) && !down {
            if inside && release && *selected != index {
                *selected = index;
                changed = true;
            }
            ctx.clear_active();
        }
        if ctx.active == Some(id) && inside {
            pressed_index = Some(index);
        }
        x += width;
    }
    if ctx.keyboard_activate == Some(group_id) && !items.is_empty() {
        *selected = (*selected).wrapping_add(1) % items.len();
        changed = true;
    }

    ctx.id_stack.semantics.describe(
        group_id,
        crate::accessibility::AccessibilityRole::ComboBox,
        items.get(*selected).copied().unwrap_or(""),
        0,
        if items.is_empty() { 0 } else { 3 },
    );
    let over_group = hovered(ctx, panel);
    if over_group {
        ctx.hot = Some(group_id);
    }
    let animated = ctx.motion.active();
    let mut selected_x = 0.0;
    let mut selected_width = 0.0;
    for (index, item) in items.iter().enumerate() {
        let width = segment_width(&mut ctx.font, face, item, style.frame_padding[0]);
        if index == *selected {
            selected_width = width;
            break;
        }
        selected_x += width;
    }
    let indicator_x = ctx.motion_tween(
        group_id,
        crate::animation::INDICATOR,
        selected_x,
        selected_x,
        ctx.motion.indicator,
        ctx.motion.easing,
    );
    let indicator_width = ctx.motion_tween(
        group_id,
        crate::animation::INDICATOR_WIDTH,
        selected_width,
        selected_width,
        ctx.motion.indicator,
        ctx.motion.easing,
    );
    if !items.is_empty() {
        {
            let (font, draw_list) = super::draw_parts(ctx);
            chrome_fill(
                font,
                draw_list,
                style.corner_rounding,
                panel,
                style.widget_bg,
            );
            if animated && *selected < items.len() {
                draw_list.push_clip_rect(panel);
                let inset = style.border_size.max(0.0);
                if indicator_width > 2.0 * inset && height > 2.0 * inset {
                    chrome_fill(
                        font,
                        draw_list,
                        style.corner_rounding,
                        [
                            panel[0] + indicator_x + inset,
                            panel[1] + inset,
                            panel[0] + indicator_x + indicator_width - inset,
                            panel[3] - inset,
                        ],
                        style.widget_bg_active,
                    );
                }
                draw_list.pop_clip_rect();
            }
        }
        let mut x = panel[0];
        for (index, item) in items.iter().enumerate() {
            let text = split_label(item).0;
            let text_size = ctx.font.measure_in(face, text);
            let width = (text_size[0] + 2.0 * style.frame_padding[0]).max(44.0);
            let segment = [x, rect[1], x + width, rect[3]];
            let inside = point_in_rect(mouse, segment);
            let active = *selected == index;
            let pressed = pressed_index == Some(index);
            let fill = if active && !animated {
                Some(style.widget_bg_active)
            } else if !active && ((over_group && inside) || pressed) {
                Some(style.widget_bg_hovered)
            } else {
                None
            };
            let fill = if animated {
                let id = segment_id(ctx, group_id, index);
                let target = fill.unwrap_or([
                    style.widget_bg_hovered[0],
                    style.widget_bg_hovered[1],
                    style.widget_bg_hovered[2],
                    0,
                ]);
                let color = ctx.widget_color(id, target);
                (color[3] > 0).then_some(color)
            } else {
                fill
            };
            let (font, draw_list) = super::draw_parts(ctx);
            if let Some(color) = fill {
                let inset = style.border_size.max(0.0);
                if segment[2] - segment[0] > 2.0 * inset && height > 2.0 * inset {
                    chrome_fill(
                        font,
                        draw_list,
                        style.corner_rounding,
                        [
                            segment[0] + inset,
                            segment[1] + inset,
                            segment[2] - inset,
                            segment[3] - inset,
                        ],
                        color,
                    );
                }
            }
            if index > 0 && style.border_size > 0.0 {
                draw_list.add_filled_rect(
                    [x, rect[1] + style.border_size],
                    [x + style.border_size, rect[3] - style.border_size],
                    lighten(style.widget_bg, BORDER_LIGHTEN),
                );
            }
            draw_list.add_text(
                font,
                face,
                [
                    x + (width - text_size[0]) * 0.5,
                    rect[1] + (height - text_size[1]) * 0.5,
                ],
                text,
                if active {
                    style.text
                } else {
                    [style.text[0], style.text[1], style.text[2], 170]
                },
            );
            x += width;
        }
        if style.border_size > 0.0 {
            let (font, draw_list) = super::draw_parts(ctx);
            draw_border(
                font,
                draw_list,
                style.corner_rounding,
                panel,
                style.border_size,
                lighten(style.widget_bg, BORDER_LIGHTEN),
            );
        }
    }
    if !display.is_empty() {
        let (font, draw_list) = super::draw_parts(ctx);
        draw_list.add_text(
            font,
            face,
            [rect[0], rect[1] + (height - label_size[1]) * 0.5],
            display,
            style.text,
        );
    }
    changed
}

#[cfg(test)]
mod tests {
    use crate::widgets::test_support::{frame, frame_with_input, test_context};

    const AWAY: [f32; 2] = [-50.0, -50.0];

    #[test]
    fn focused_pill_and_segmented_choice_accept_keyboard_activation() {
        use crate::input::{InputState, Key, KeyEvent};
        let mut ctx = test_context();
        let mut pill = false;
        let mut selected = 0;
        frame(&mut ctx, AWAY, false, |ui| {
            ui.pill_toggle("Pill", &mut pill);
            ui.segmented_control("Mode", &["A", "B"], &mut selected);
        });
        frame_with_input(
            &mut ctx,
            0.016,
            InputState {
                keys: &[KeyEvent::Press(Key::Tab)],
                ..Default::default()
            },
            |ui| {
                ui.pill_toggle("Pill", &mut pill);
                ui.segmented_control("Mode", &["A", "B"], &mut selected);
            },
        );
        frame_with_input(
            &mut ctx,
            0.016,
            InputState {
                keys: &[KeyEvent::Press(Key::Space)],
                ..Default::default()
            },
            |ui| {
                assert!(ui.pill_toggle("Pill", &mut pill));
                ui.segmented_control("Mode", &["A", "B"], &mut selected);
            },
        );
        assert!(pill);
        frame_with_input(
            &mut ctx,
            0.016,
            InputState {
                keys: &[KeyEvent::Press(Key::Tab)],
                ..Default::default()
            },
            |ui| {
                ui.pill_toggle("Pill", &mut pill);
                ui.segmented_control("Mode", &["A", "B"], &mut selected);
            },
        );
        frame_with_input(
            &mut ctx,
            0.016,
            InputState {
                keys: &[KeyEvent::Press(Key::Enter)],
                ..Default::default()
            },
            |ui| {
                ui.pill_toggle("Pill", &mut pill);
                assert!(ui.segmented_control("Mode", &["A", "B"], &mut selected));
            },
        );
        assert_eq!(selected, 1);
    }

    #[test]
    fn pill_changes_only_on_completed_click() {
        let mut ctx = test_context();
        let mut value = false;
        let mut rect = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            ui.pill_toggle("Pill", &mut value);
            rect = ui.ctx.last_item.unwrap().1;
        });
        let at = [(rect[0] + rect[2]) * 0.5, (rect[1] + rect[3]) * 0.5];
        let mut changed = false;
        frame(&mut ctx, at, true, |ui| {
            changed = ui.pill_toggle("Pill", &mut value)
        });
        assert!(!changed && !value);
        frame(&mut ctx, AWAY, false, |ui| {
            changed = ui.pill_toggle("Pill", &mut value)
        });
        assert!(!changed && !value);
        frame(&mut ctx, at, true, |ui| {
            changed = ui.pill_toggle("Pill", &mut value)
        });
        assert!(!changed);
        frame(&mut ctx, at, false, |ui| {
            changed = ui.pill_toggle("Pill", &mut value)
        });
        assert!(changed && value);
        frame(&mut ctx, at, false, |ui| {
            changed = ui.pill_toggle("Pill", &mut value)
        });
        assert!(!changed && value);
    }

    #[test]
    fn segment_capture_stays_with_pressed_choice() {
        let mut ctx = test_context();
        let mut selected = 0;
        let items = ["A", "B", "C"];
        let mut rect = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            ui.segmented_control("Mode", &items, &mut selected);
            rect = ui.ctx.last_item.unwrap().1;
        });
        let label_width = ctx.font.measure("Mode")[0] + ctx.style.item_spacing[0];
        let a = [rect[0] + label_width + 22.0, (rect[1] + rect[3]) * 0.5];
        let b = [a[0] + 44.0, a[1]];
        let mut changed = false;
        frame(&mut ctx, b, true, |ui| {
            changed = ui.segmented_control("Mode", &items, &mut selected)
        });
        assert!(!changed && selected == 0);
        frame(&mut ctx, a, false, |ui| {
            changed = ui.segmented_control("Mode", &items, &mut selected)
        });
        assert!(!changed && selected == 0);
        frame(&mut ctx, b, true, |ui| {
            changed = ui.segmented_control("Mode", &items, &mut selected)
        });
        frame(&mut ctx, b, false, |ui| {
            changed = ui.segmented_control("Mode", &items, &mut selected)
        });
        assert!(changed && selected == 1);
        frame(&mut ctx, b, true, |ui| {
            changed = ui.segmented_control("Mode", &items, &mut selected)
        });
        frame(&mut ctx, b, false, |ui| {
            changed = ui.segmented_control("Mode", &items, &mut selected)
        });
        assert!(!changed && selected == 1);
    }

    #[test]
    fn empty_segments_are_inert() {
        let mut ctx = test_context();
        let mut selected = 4;
        frame(&mut ctx, AWAY, false, |ui| {
            assert!(!ui.segmented_control("Mode", &[], &mut selected));
        });
        assert_eq!(selected, 4);
    }
}
