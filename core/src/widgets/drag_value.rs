use super::{draw_border, lighten, slider::round_f64_to_i32, FmtBuf, BORDER_LIGHTEN};
use crate::context::WidgetState;
use crate::id::split_label;
use crate::ui::{hover_layer_gate, hovered, point_in_rect, Ui};
use core::fmt::Write;

pub(crate) fn drag_float(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    speed: f32,
    range: core::ops::RangeInclusive<f32>,
) -> bool {
    let mut v = f64::from(*value);
    let changed = drag_impl(
        ui,
        label,
        &mut v,
        speed,
        f64::from(*range.start()),
        f64::from(*range.end()),
        false,
    );
    let next = v as f32;
    let changed = changed && next != *value;
    if changed {
        *value = next;
    }
    changed
}

pub(crate) fn drag_int(
    ui: &mut Ui,
    label: &str,
    value: &mut i32,
    speed: f32,
    range: core::ops::RangeInclusive<i32>,
) -> bool {
    let mut v = f64::from(*value);
    let changed = drag_impl(
        ui,
        label,
        &mut v,
        speed,
        f64::from(*range.start()),
        f64::from(*range.end()),
        true,
    );
    let next = round_f64_to_i32(v);
    let changed = changed && next != *value;
    if changed {
        *value = next;
    }
    changed
}

fn drag_impl(
    ui: &mut Ui,
    label: &str,
    value: &mut f64,
    speed: f32,
    start: f64,
    end: f64,
    integer: bool,
) -> bool {
    let ctx = &mut *ui.ctx;
    let id = super::widget_id(ctx, label);
    let display = split_label(label).0;
    let style = ctx.style;
    let face = ctx.active_font();
    let text_size = ctx.font.measure_in(face, display);
    let height = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
    let width = super::available_width(ctx).max(0.0);
    let rect = super::interactive_rect(ctx, [width, height]);
    super::register_item(ctx, id, rect);
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let mouse = ctx.input.mouse_pos;
    let inside = point_in_rect(mouse, ctx.target_rect(rect));
    let down = ctx.input.mouse_down[0];
    if inside && hover_layer_gate(ctx) && down && !ctx.prev_mouse_down[0] {
        ctx.set_active(id);
        ctx.widget_states.insert(
            id,
            WidgetState::DragValue {
                start_x: mouse[0],
                start_value: *value,
            },
        );
    }
    let active = ctx.active == Some(id);
    let mut changed = false;
    if let Some(next) = ctx.semantic_number(id) {
        let next = if integer {
            f64::from(round_f64_to_i32(next))
        } else {
            next
        }
        .clamp(start, end);
        if next != *value {
            *value = next;
            changed = true;
        }
    }
    if active && down && mouse[0].is_finite() {
        if let Some(WidgetState::DragValue {
            start_x,
            start_value,
        }) = ctx.widget_states.get(&id)
        {
            let speed = if speed.is_finite() { speed } else { 0.0 };
            let mapped = (*start_value + f64::from(mouse[0] - *start_x) * f64::from(speed))
                .clamp(start, end);
            let mapped = if integer {
                f64::from(round_f64_to_i32(mapped))
            } else {
                mapped
            };
            if mapped != *value {
                *value = mapped;
                changed = true;
            }
        }
    }
    if active && !down {
        ctx.clear_active();
    }
    let over = hovered(ctx, rect);
    if over {
        ctx.hot = Some(id);
    }
    let base = if active && down {
        lighten(style.widget_bg, 20)
    } else if over {
        lighten(style.widget_bg, 10)
    } else {
        style.widget_bg
    };
    let base = ctx.widget_color(id, base);
    ctx.request_cursor(
        id,
        rect,
        crate::host_requests::CursorShape::ResizeHorizontal,
    );
    if ctx.accessibility_enabled() {
        use core::fmt::Write;
        let mut text = super::FmtBuf::<64>::new();
        let _ = write!(text, "{:.2}", *value);
        ctx.id_stack.semantics.describe(
            id,
            crate::accessibility::AccessibilityRole::Slider,
            text.as_str(),
            0,
            crate::accessibility::AccessibilityActions::SET_NUMBER,
        );
    }
    let (font, draw_list) = super::draw_parts(ctx);
    super::chrome_fill(font, draw_list, style.corner_rounding, rect, base);
    if (active && down) || over {
        let bar_w = 2.0_f32.min(width);
        if bar_w > 0.0 {
            draw_list.add_filled_rect(
                [rect[0], rect[1]],
                [rect[0] + bar_w, rect[3]],
                if active && down {
                    style.widget_bg_active
                } else {
                    style.widget_bg_hovered
                },
            );
        }
    }
    if style.border_size > 0.0 {
        draw_border(
            font,
            draw_list,
            style.corner_rounding,
            rect,
            style.border_size,
            lighten(base, BORDER_LIGHTEN),
        );
    }
    let mut number = FmtBuf::<64>::new();
    if integer {
        write!(number, "{}", round_f64_to_i32(*value))
    } else {
        write!(number, "{:.2}", *value as f32)
    }
    .expect("drag value fits the stack buffer");
    let number = number.as_str();
    let number_size = font.measure_in(face, number);
    let pad = style.frame_padding[0].max(0.0);
    if width > 2.0 * pad {
        let number_x = rect[2] - pad - number_size[0];
        draw_list.push_clip_rect([rect[0] + pad, rect[1], rect[2] - pad, rect[3]]);
        draw_list.add_text(
            font,
            face,
            [number_x, rect[1] + (height - number_size[1]) * 0.5],
            number,
            style.text,
        );
        if !display.is_empty() && rect[0] + pad + text_size[0] + style.item_spacing[0] <= number_x {
            draw_list.add_text(
                font,
                face,
                [rect[0] + pad, rect[1] + (height - text_size[1]) * 0.5],
                display,
                style.text,
            );
        }
        draw_list.pop_clip_rect();
    }
    changed
}

#[cfg(test)]
mod tests {
    use crate::widgets::test_support::{frame, test_context};

    const AWAY: [f32; 2] = [-50.0, -50.0];

    #[test]
    fn float_drag_uses_press_origin_and_clamps() {
        let mut ctx = test_context();
        let mut value = 10.0;
        let mut rect = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            ui.drag_float("Drag", &mut value, 0.25, 0.0..=30.0);
            rect = ui.ctx.last_item.unwrap().1;
        });
        let at = [(rect[0] + rect[2]) * 0.5, (rect[1] + rect[3]) * 0.5];
        frame(&mut ctx, at, true, |ui| {
            assert!(!ui.drag_float("Drag", &mut value, 0.25, 0.0..=30.0));
        });
        frame(&mut ctx, [at[0] + 40.0, at[1]], true, |ui| {
            assert!(ui.drag_float("Drag", &mut value, 0.25, 0.0..=30.0));
        });
        assert_eq!(value, 20.0);
        frame(&mut ctx, [at[0] + 200.0, at[1]], true, |ui| {
            assert!(ui.drag_float("Drag", &mut value, 0.25, 0.0..=30.0));
        });
        assert_eq!(value, 30.0);
        frame(&mut ctx, AWAY, false, |ui| {
            assert!(!ui.drag_float("Drag", &mut value, 0.25, 0.0..=30.0));
        });
    }

    #[test]
    fn integer_drag_keeps_large_values_exact() {
        let mut ctx = test_context();
        let mut value = 2_000_000_001;
        let mut rect = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            assert!(!ui.drag_int("Large", &mut value, 1.0, 2_000_000_000..=2_000_000_100));
            rect = ui.ctx.last_item.unwrap().1;
        });
        assert_eq!(value, 2_000_000_001);
        let at = [(rect[0] + rect[2]) * 0.5, (rect[1] + rect[3]) * 0.5];
        frame(&mut ctx, at, true, |ui| {
            assert!(!ui.drag_int("Large", &mut value, 1.0, 2_000_000_000..=2_000_000_100));
        });
        frame(&mut ctx, [at[0] + 7.0, at[1]], true, |ui| {
            assert!(ui.drag_int("Large", &mut value, 1.0, 2_000_000_000..=2_000_000_100));
        });
        assert_eq!(value, 2_000_000_008);
    }
}
