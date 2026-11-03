use super::{draw_border, lighten, slider::round_f64_to_i32, FmtBuf, BORDER_LIGHTEN};
use crate::id::{split_label, Id};
use crate::ui::{hover_layer_gate, hovered, point_in_rect, Ui};
use core::fmt::Write;

pub(crate) fn range_slider_float(
    ui: &mut Ui,
    label: &str,
    lower: &mut f32,
    upper: &mut f32,
    range: core::ops::RangeInclusive<f32>,
) -> bool {
    let mut lo = f64::from(*lower);
    let mut hi = f64::from(*upper);
    let changed = range_slider_impl(
        ui,
        label,
        &mut lo,
        &mut hi,
        f64::from(*range.start()),
        f64::from(*range.end()),
        false,
    );
    let next = (lo as f32, hi as f32);
    let changed = changed && next != (*lower, *upper);
    if changed {
        *lower = next.0;
        *upper = next.1;
    }
    changed
}

pub(crate) fn range_slider_int(
    ui: &mut Ui,
    label: &str,
    lower: &mut i32,
    upper: &mut i32,
    range: core::ops::RangeInclusive<i32>,
) -> bool {
    let mut lo = f64::from(*lower);
    let mut hi = f64::from(*upper);
    let changed = range_slider_impl(
        ui,
        label,
        &mut lo,
        &mut hi,
        f64::from(*range.start()),
        f64::from(*range.end()),
        true,
    );
    let next = (round_f64_to_i32(lo), round_f64_to_i32(hi));
    let changed = changed && next != (*lower, *upper);
    if changed {
        *lower = next.0;
        *upper = next.1;
    }
    changed
}

fn handle_ids(ctx: &mut crate::context::Context, group: Id) -> (Id, Id) {
    ctx.id_stack.push_id(group);
    let ids = (ctx.id_stack.id_index(0), ctx.id_stack.id_index(1));
    ctx.id_stack.pop();
    ids
}

fn range_slider_impl(
    ui: &mut Ui,
    label: &str,
    lower: &mut f64,
    upper: &mut f64,
    start: f64,
    end: f64,
    integer: bool,
) -> bool {
    let ctx = &mut *ui.ctx;
    let group_id = super::widget_id(ctx, label);
    let (lower_id, upper_id) = handle_ids(ctx, group_id);
    let display = split_label(label).0;
    let style = ctx.style;
    let face = ctx.active_font();
    let text_size = ctx.font.measure_in(face, display);
    let height = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
    let width = super::available_width(ctx).max(0.0);
    let rect = super::interactive_rect(ctx, [width, height]);
    super::register_item(ctx, group_id, rect);
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let span = end - start;
    let shown_lower = (*lower).min(*upper).clamp(start, end);
    let shown_upper = (*lower).max(*upper).clamp(start, end);
    let to_x = |value: f64| -> f32 {
        if span > 0.0 {
            rect[0] + (((value - start) / span) as f32) * width
        } else {
            rect[0]
        }
    };
    let mouse = ctx.input.mouse_pos;
    let inside = point_in_rect(mouse, ctx.target_rect(rect));
    let down = ctx.input.mouse_down[0];
    if inside && hover_layer_gate(ctx) && down && !ctx.prev_mouse_down[0] {
        let lo_x = to_x(shown_lower);
        let hi_x = to_x(shown_upper);
        let choose_lower = if lo_x == hi_x {
            mouse[0] <= lo_x
        } else {
            (mouse[0] - lo_x).abs() <= (mouse[0] - hi_x).abs()
        };
        ctx.set_active(if choose_lower { lower_id } else { upper_id });
    }
    let editing_lower = ctx.active == Some(lower_id);
    let editing_upper = ctx.active == Some(upper_id);
    let mut changed = false;
    if (editing_lower || editing_upper) && down {
        let old = (*lower, *upper);
        *lower = shown_lower;
        *upper = shown_upper;
        let t = if width > 0.0 {
            ((mouse[0] - rect[0]) / width).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mapped = start + f64::from(t) * span;
        let mapped = if integer {
            f64::from(round_f64_to_i32(mapped))
        } else {
            mapped
        };
        if editing_lower {
            *lower = mapped.clamp(start, *upper);
        } else {
            *upper = mapped.clamp(*lower, end);
        }
        changed = old != (*lower, *upper);
    }
    if (editing_lower || editing_upper) && !down {
        ctx.clear_active();
    }
    let over = hovered(ctx, rect);
    if over {
        ctx.hot = Some(group_id);
    }

    let lo = (*lower).min(*upper).clamp(start, end);
    let hi = (*lower).max(*upper).clamp(start, end);
    let lo_x = to_x(lo);
    let hi_x = to_x(hi);
    let grab_w = style.slider_grab_size.max(10.0).min(width);
    let grab_left =
        |center: f32| -> f32 { (center - grab_w * 0.5).clamp(rect[0], rect[2] - grab_w) };
    let lower_color = if editing_lower && down {
        style.widget_bg_hovered
    } else {
        style.widget_bg_active
    };
    let upper_color = if editing_upper && down {
        style.widget_bg_hovered
    } else {
        style.widget_bg_active
    };
    let lower_color = ctx.widget_color(lower_id, lower_color);
    let upper_color = ctx.widget_color(upper_id, upper_color);
    let (font, draw_list) = super::draw_parts(ctx);
    super::chrome_fill(
        font,
        draw_list,
        style.corner_rounding,
        rect,
        style.widget_bg,
    );
    if hi_x > lo_x {
        let mut color = [0u8; 4];
        for (channel, value) in color.iter_mut().enumerate() {
            *value = ((u16::from(style.widget_bg[channel])
                + u16::from(style.widget_bg_active[channel]))
                / 2) as u8;
        }
        super::chrome_fill(
            font,
            draw_list,
            style.corner_rounding,
            [lo_x, rect[1], hi_x, rect[3]],
            color,
        );
    }
    if grab_w > 0.0 {
        super::chrome_fill(
            font,
            draw_list,
            style.corner_rounding,
            [grab_left(lo_x), rect[1], grab_left(lo_x) + grab_w, rect[3]],
            lower_color,
        );
        super::chrome_fill(
            font,
            draw_list,
            style.corner_rounding,
            [grab_left(hi_x), rect[1], grab_left(hi_x) + grab_w, rect[3]],
            upper_color,
        );
    }
    if style.border_size > 0.0 {
        draw_border(
            font,
            draw_list,
            style.corner_rounding,
            rect,
            style.border_size,
            lighten(style.widget_bg, BORDER_LIGHTEN),
        );
    }
    let mut values = FmtBuf::<128>::new();
    if integer {
        write!(
            values,
            "{} .. {}",
            round_f64_to_i32(lo),
            round_f64_to_i32(hi)
        )
    } else {
        write!(values, "{:.2} .. {:.2}", lo, hi)
    }
    .expect("range values fit the stack buffer");
    let values = values.as_str();
    let values_size = font.measure_in(face, values);
    let pad = style.frame_padding[0].max(0.0);
    if width > 2.0 * pad {
        let values_x = rect[2] - pad - values_size[0];
        draw_list.push_clip_rect([rect[0] + pad, rect[1], rect[2] - pad, rect[3]]);
        draw_list.add_text(
            font,
            face,
            [values_x, rect[1] + (height - values_size[1]) * 0.5],
            values,
            style.text,
        );
        if !display.is_empty() && rect[0] + pad + text_size[0] + style.item_spacing[0] <= values_x {
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
    fn float_handles_capture_nearest_and_do_not_cross() {
        let mut ctx = test_context();
        let (mut lower, mut upper) = (20.0, 80.0);
        let mut rect = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            ui.range_slider_float("Range", &mut lower, &mut upper, 0.0..=100.0);
            rect = ui.ctx.last_item.unwrap().1;
        });
        let point = |fraction: f32| {
            [
                rect[0] + (rect[2] - rect[0]) * fraction,
                (rect[1] + rect[3]) * 0.5,
            ]
        };
        frame(&mut ctx, point(0.20), true, |ui| {
            ui.range_slider_float("Range", &mut lower, &mut upper, 0.0..=100.0);
        });
        let mut changed = false;
        frame(&mut ctx, point(0.95), true, |ui| {
            changed = ui.range_slider_float("Range", &mut lower, &mut upper, 0.0..=100.0);
        });
        assert!(changed);
        assert!((lower - 80.0).abs() < 0.01 && (upper - 80.0).abs() < 0.01);
        frame(&mut ctx, AWAY, false, |ui| {
            changed = ui.range_slider_float("Range", &mut lower, &mut upper, 0.0..=100.0);
        });
        assert!(!changed);
        frame(&mut ctx, point(0.85), true, |ui| {
            ui.range_slider_float("Range", &mut lower, &mut upper, 0.0..=100.0);
        });
        frame(&mut ctx, point(0.95), true, |ui| {
            changed = ui.range_slider_float("Range", &mut lower, &mut upper, 0.0..=100.0);
        });
        assert!(changed);
        assert!(upper > lower && upper > 90.0);
    }

    #[test]
    fn integer_range_preserves_large_idle_values_and_rounds_drag() {
        let mut ctx = test_context();
        let (mut lower, mut upper) = (2_000_000_020, 2_000_000_080);
        let mut rect = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            assert!(!ui.range_slider_int(
                "Large",
                &mut lower,
                &mut upper,
                2_000_000_000..=2_000_000_100
            ));
            rect = ui.ctx.last_item.unwrap().1;
        });
        assert_eq!((lower, upper), (2_000_000_020, 2_000_000_080));
        let at = |fraction: f32| {
            [
                rect[0] + (rect[2] - rect[0]) * fraction,
                (rect[1] + rect[3]) * 0.5,
            ]
        };
        frame(&mut ctx, at(0.2), true, |ui| {
            ui.range_slider_int(
                "Large",
                &mut lower,
                &mut upper,
                2_000_000_000..=2_000_000_100,
            );
        });
        frame(&mut ctx, at(0.3), true, |ui| {
            assert!(ui.range_slider_int(
                "Large",
                &mut lower,
                &mut upper,
                2_000_000_000..=2_000_000_100
            ));
        });
        assert!((2_000_000_029..=2_000_000_031).contains(&lower));
        assert_eq!(upper, 2_000_000_080);
    }
}
