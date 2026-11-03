use super::{available_width, draw_parts, item_rect, register_item, widget_id};
use crate::id::split_label;
use crate::ui::{hover_layer_gate, hovered, point_in_rect, Ui, ValidationState};

pub(crate) fn validation_message(ui: &mut Ui, state: ValidationState, message: &str) {
    let color = ui.theme_color(match state {
        ValidationState::Success => crate::theme::ThemeRole::Success,
        ValidationState::Warning => crate::theme::ThemeRole::Warning,
        ValidationState::Error => crate::theme::ThemeRole::Danger,
    });
    let ctx = &mut *ui.ctx;
    let face = ctx.active_font();
    let size = ctx.font.measure_in(face, message);
    let width = available_width(ctx).max(0.0);
    let rect = item_rect(ctx, [width, size[1].max(14.0)]);
    let (font, draw) = draw_parts(ctx);
    draw.add_filled_rect(
        [rect[0], rect[1] + 2.0],
        [rect[0] + 2.0, rect[3] - 2.0],
        color,
    );
    draw.add_text(font, face, [rect[0] + 8.0, rect[1]], message, color);
}

pub(crate) fn nav_row(ui: &mut Ui, label: &str, selected: &mut usize, index: usize) -> bool {
    let ctx = &mut *ui.ctx;
    let id = widget_id(ctx, label);
    let display = split_label(label).0;
    let style = ctx.style;
    let face = ctx.active_font();
    let height = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
    let width = available_width(ctx).max(0.0);
    let rect = super::interactive_rect(ctx, [width, height]);
    super::register_focusable_item(ctx, id, rect);
    let inside = point_in_rect(ctx.input.mouse_pos, ctx.target_rect(rect)) && hover_layer_gate(ctx);
    let down = ctx.input.mouse_down[0];
    if inside && down && !ctx.prev_mouse_down[0] {
        ctx.set_active(id);
    }
    let held = ctx.active == Some(id);
    let clicked = (held && !down && inside) || ctx.keyboard_activate == Some(id);
    if held && !down {
        ctx.clear_active();
    }
    let changed = clicked && *selected != index;
    if changed {
        *selected = index;
    }
    let over = hovered(ctx, rect);
    if over {
        ctx.hot = Some(id);
    }
    ctx.id_stack.semantics.describe(
        id,
        crate::accessibility::AccessibilityRole::Tab,
        display,
        if *selected == index {
            crate::accessibility::AccessibilityState::SELECTED
        } else {
            0
        },
        3,
    );
    let selection = ctx.motion_tween(
        id,
        crate::animation::INDICATOR,
        if *selected == index { 1.0 } else { 0.0 },
        if *selected == index { 1.0 } else { 0.0 },
        ctx.motion.indicator,
        ctx.motion.easing,
    );
    let highlight = ctx.motion_tween(
        id,
        crate::animation::INDICATOR_WIDTH,
        if *selected == index || over { 1.0 } else { 0.0 },
        if *selected == index || over { 1.0 } else { 0.0 },
        ctx.motion.hover,
        ctx.motion.easing,
    );
    let animated = ctx.motion.active();
    let (font, draw) = draw_parts(ctx);
    if highlight > 0.0 {
        let mut bg = if *selected == index {
            style.widget_bg_active
        } else {
            style.widget_bg_hovered
        };
        bg[3] = if animated {
            (bg[3] as f32 * highlight * 0.25 + 0.5) as u8
        } else {
            bg[3] / 4
        };
        draw.add_filled_rect([rect[0], rect[1]], [rect[2], rect[3]], bg);
    }
    if selection > 0.0 {
        let mid = (rect[1] + rect[3]) * 0.5;
        let half = ((rect[3] - rect[1]) * 0.5 - 3.0) * selection;
        let mut color = style.widget_bg_active;
        color[3] = (color[3] as f32 * selection + 0.5) as u8;
        draw.add_filled_rect([rect[0], mid - half], [rect[0] + 2.0, mid + half], color);
    }
    draw.add_text(
        font,
        face,
        [rect[0] + 10.0, rect[1] + style.frame_padding[1]],
        display,
        style.text,
    );
    changed
}

pub(crate) fn breadcrumbs(ui: &mut Ui, label: &str, items: &[&str], selected: &mut usize) -> bool {
    let ctx = &mut *ui.ctx;
    let group = widget_id(ctx, label);
    let style = ctx.style;
    let face = ctx.active_font();
    let height = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
    let width = available_width(ctx).max(0.0);
    let rect = super::interactive_rect(ctx, [width, height]);
    draw_parts(ctx).1.push_clip_rect(rect);
    ctx.id_stack.push_id(group);
    let mut x = rect[0];
    let mut changed = false;
    for (index, item) in items.iter().enumerate() {
        let id = ctx.id_stack.id_index(index);
        let width = ctx.font.measure_in(face, item)[0] + 2.0 * style.frame_padding[0];
        let segment = [x, rect[1], (x + width).min(rect[2]), rect[3]];
        if segment[2] <= segment[0] {
            break;
        }
        register_item(ctx, id, segment);
        let inside = point_in_rect(ctx.input.mouse_pos, segment) && hover_layer_gate(ctx);
        let down = ctx.input.mouse_down[0];
        if inside && down && !ctx.prev_mouse_down[0] {
            ctx.set_active(id);
        }
        let held = ctx.active == Some(id);
        let clicked = held && !down && inside;
        if held && !down {
            ctx.clear_active();
        }
        if clicked && *selected != index {
            *selected = index;
            changed = true;
        }
        let over = hovered(ctx, segment);
        if over {
            ctx.hot = Some(id);
        }
        let (font, draw) = draw_parts(ctx);
        let mut color = style.text;
        if index != *selected && !over {
            color[3] /= 2;
        }
        draw.add_text(
            font,
            face,
            [x + style.frame_padding[0], rect[1] + style.frame_padding[1]],
            item,
            color,
        );
        x += width;
        if index + 1 < items.len() && x < rect[2] {
            let (font, draw) = draw_parts(ctx);
            draw.add_text(
                font,
                face,
                [x + 2.0, rect[1] + style.frame_padding[1]],
                "/",
                style.text,
            );
            x += ctx.font.measure_in(face, "/")[0] + 8.0;
        }
    }
    ctx.id_stack.pop();
    draw_parts(ctx).1.pop_clip_rect();
    changed
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{frame, test_context};

    #[test]
    fn disabling_a_section_releases_its_capture_and_mutes_its_controls() {
        let mut ctx = test_context();
        let mut enabled = true;
        let mut value = false;
        let mut rect = [0.0; 4];
        frame(&mut ctx, [-100.0; 2], false, |ui| {
            ui.disabled("section", !enabled, |ui| {
                ui.checkbox("option", &mut value);
                rect = ui.ctx.last_item.unwrap().1;
            });
        });
        let mouse = [(rect[0] + rect[2]) * 0.5, (rect[1] + rect[3]) * 0.5];
        frame(&mut ctx, mouse, true, |ui| {
            ui.disabled("section", !enabled, |ui| {
                ui.checkbox("option", &mut value);
            })
        });
        assert!(ctx.active.is_some());
        enabled = false;
        frame(&mut ctx, mouse, false, |ui| {
            ui.disabled("section", !enabled, |ui| {
                ui.checkbox("option", &mut value);
            })
        });
        assert!(ctx.active.is_none());
        assert!(!value);
        assert_eq!(ctx.disabled_depth, 0);
    }

    #[test]
    fn vertical_row_changes_selection_only_on_release_inside() {
        let mut ctx = test_context();
        let mut selected = 0;
        let mut rect = [0.0; 4];
        frame(&mut ctx, [-100.0; 2], false, |ui| {
            ui.nav_row("first", &mut selected, 0);
            ui.nav_row("second", &mut selected, 1);
            rect = ui.ctx.last_item.unwrap().1;
        });
        let mouse = [(rect[0] + rect[2]) * 0.5, (rect[1] + rect[3]) * 0.5];
        let mut changed = false;
        frame(&mut ctx, mouse, true, |ui| {
            ui.nav_row("first", &mut selected, 0);
            changed = ui.nav_row("second", &mut selected, 1);
        });
        assert!(!changed);
        frame(&mut ctx, mouse, false, |ui| {
            ui.nav_row("first", &mut selected, 0);
            changed = ui.nav_row("second", &mut selected, 1);
        });
        assert!(changed);
        assert_eq!(selected, 1);
    }

    #[test]
    fn preview_canvas_returns_a_clipped_content_rect() {
        let mut ctx = test_context();
        let mut rect = [0.0; 4];
        frame(&mut ctx, [-100.0; 2], false, |ui| {
            ui.preview_canvas("preview", [120.0, 70.0], |ui, content| {
                rect = content;
                ui.draw_rect_filled([content[0], content[1]], [content[2], content[3]], [255; 4]);
            });
        });
        assert!(rect[0] < rect[2] && rect[1] < rect[3]);
        assert!(rect[2] - rect[0] <= 120.0 && rect[3] - rect[1] <= 70.0);
    }
}
