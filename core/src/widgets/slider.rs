use super::{draw_border, lighten, FmtBuf, BORDER_LIGHTEN};
use crate::id::split_label;
use crate::ui::{hovered, point_in_rect, Ui};
use core::fmt::Write;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SliderAppearance {
    Grab,
    Filled,
}

pub(crate) fn slider_float(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: core::ops::RangeInclusive<f32>,
) -> bool {
    slider_impl(
        ui,
        label,
        value,
        *range.start(),
        *range.end(),
        false,
        SliderAppearance::Grab,
    )
}

pub(crate) fn filled_slider_float(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: core::ops::RangeInclusive<f32>,
) -> bool {
    slider_impl(
        ui,
        label,
        value,
        *range.start(),
        *range.end(),
        false,
        SliderAppearance::Filled,
    )
}

pub(crate) fn slider_int(
    ui: &mut Ui,
    label: &str,
    value: &mut i32,
    range: core::ops::RangeInclusive<i32>,
) -> bool {
    slider_int_impl(ui, label, value, range, SliderAppearance::Grab)
}

pub(crate) fn filled_slider_int(
    ui: &mut Ui,
    label: &str,
    value: &mut i32,
    range: core::ops::RangeInclusive<i32>,
) -> bool {
    slider_int_impl(ui, label, value, range, SliderAppearance::Filled)
}

fn slider_int_impl(
    ui: &mut Ui,
    label: &str,
    value: &mut i32,
    range: core::ops::RangeInclusive<i32>,
    appearance: SliderAppearance,
) -> bool {
    let mut v = *value as f32;
    let changed = slider_impl(
        ui,
        label,
        &mut v,
        *range.start() as f32,
        *range.end() as f32,
        true,
        appearance,
    );
    if changed {
        *value = v as i32;
    }
    changed
}

fn slider_impl(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    start: f32,
    end: f32,
    integer: bool,
    appearance: SliderAppearance,
) -> bool {
    let ctx = &mut *ui.ctx;
    let id = super::widget_id(ctx, label);
    let display = split_label(label).0;
    let style = ctx.style;
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    let face = ctx.active_font();
    let height = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
    let text_size = ctx.font.measure_in(face, display);
    let available = super::available_width(ctx);
    let track_w = if appearance == SliderAppearance::Filled || display.is_empty() {
        available
    } else {
        available - text_size[0] - style.item_spacing[0]
    }
    .max(0.0);
    let rect = super::interactive_rect(ctx, [available, height]);
    super::register_item(ctx, id, rect);
    let track = [rect[0], rect[1], rect[0] + track_w, rect[3]];

    let mouse = ctx.input.mouse_pos;
    let inside = point_in_rect(mouse, ctx.target_rect(track));
    let left_down = ctx.input.mouse_down[0];
    let left_pressed = left_down && !ctx.prev_mouse_down[0];
    if inside && left_pressed {
        ctx.set_active(id);
    }
    let held = ctx.active == Some(id);
    let mut changed = false;
    if let Some(number) = ctx.semantic_number(id) {
        let mut next = (number as f32).clamp(start, end);
        if integer {
            next = round_half_away_from_zero(next);
        }
        if next != *value {
            *value = next;
            changed = true;
        }
    }
    if held && left_down {
        let span = end - start;
        let t = if track_w > 0.0 {
            ((mouse[0] - track[0]) / track_w).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mut new_value = start + t * span;
        if integer {
            new_value = round_half_away_from_zero(new_value);
        }
        let new_value = new_value.clamp(start, end);
        if new_value != *value {
            *value = new_value;
            changed = true;
        }
    }
    if held && !left_down {
        ctx.clear_active();
    }

    let hovered = hovered(ctx, track);
    if hovered {
        ctx.hot = Some(id);
    }

    let active = ctx.active == Some(id);
    let span = end - start;
    let t_cur = if span > 0.0 {
        ((*value - start) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let grab_w = style.slider_grab_size.min(track_w);
    let grab_x = (track[0] + t_cur * track_w - grab_w / 2.0).clamp(track[0], track[2] - grab_w);
    let grab_bg = if active {
        style.widget_bg_active
    } else if hovered {
        style.widget_bg_hovered
    } else {
        style.widget_bg
    };

    let grab_bg = ctx.widget_color(id, grab_bg);
    ctx.request_cursor(
        id,
        track,
        crate::host_requests::CursorShape::ResizeHorizontal,
    );
    if ctx.accessibility_enabled() {
        use core::fmt::Write;
        let mut semantic_value = super::FmtBuf::<64>::new();
        let _ = write!(semantic_value, "{:.2}", *value);
        ctx.id_stack.semantics.describe(
            id,
            crate::accessibility::AccessibilityRole::Slider,
            semantic_value.as_str(),
            0,
            crate::accessibility::AccessibilityActions::SET_NUMBER,
        );
    }
    let (font, draw_list) = super::draw_parts(ctx);
    super::chrome_fill(
        font,
        draw_list,
        style.corner_rounding,
        track,
        style.widget_bg,
    );
    if appearance == SliderAppearance::Grab {
        super::chrome_fill(
            font,
            draw_list,
            style.corner_rounding,
            [grab_x, track[1], grab_x + grab_w, track[3]],
            grab_bg,
        );
    } else if t_cur > 0.0 && track_w > 0.0 {
        let accent = if hovered {
            style.widget_bg_hovered
        } else {
            style.widget_bg_active
        };
        let weight: u16 = if active {
            4
        } else if hovered {
            3
        } else {
            2
        };
        let mut fill_color = [0; 4];
        for channel in 0..4 {
            fill_color[channel] = ((u16::from(style.widget_bg[channel]) * (4 - weight)
                + u16::from(accent[channel]) * weight)
                / 4) as u8;
        }
        super::chrome_fill(
            font,
            draw_list,
            style.corner_rounding,
            [track[0], track[1], track[0] + t_cur * track_w, track[3]],
            fill_color,
        );
    }
    if style.border_size > 0.0 {
        draw_border(
            font,
            draw_list,
            style.corner_rounding,
            track,
            style.border_size,
            lighten(style.widget_bg, BORDER_LIGHTEN),
        );
    }
    let mut value_text = FmtBuf::<64>::new();
    if integer {
        write!(value_text, "{}", *value as i32)
    } else {
        write!(value_text, "{:.2}", *value)
    }
    .expect("slider value text always fits 64 bytes");
    let value_text = value_text.as_str();
    let value_size = font.measure_in(face, value_text);
    if appearance == SliderAppearance::Filled {
        let pad = style.frame_padding[0].max(0.0);
        if track_w > 2.0 * pad {
            draw_list.push_clip_rect([track[0] + pad, track[1], track[2] - pad, track[3]]);
            let value_x = track[2] - pad - value_size[0];
            draw_list.add_text(
                font,
                face,
                [value_x, rect[1] + (height - value_size[1]) / 2.0],
                value_text,
                style.text,
            );
            if !display.is_empty()
                && track[0] + pad + text_size[0] + style.item_spacing[0] <= value_x
            {
                draw_list.add_text(
                    font,
                    face,
                    [track[0] + pad, rect[1] + (height - text_size[1]) / 2.0],
                    display,
                    style.text,
                );
            }
            draw_list.pop_clip_rect();
        }
    } else {
        let value_pos = [
            track[0] + (track_w - value_size[0]) / 2.0,
            rect[1] + (height - value_size[1]) / 2.0,
        ];
        draw_list.add_text(font, face, value_pos, value_text, style.text);
        if !display.is_empty() {
            let label_pos = [
                track[2] + style.item_spacing[0],
                rect[1] + (height - text_size[1]) / 2.0,
            ];
            draw_list.add_text(font, face, label_pos, display, style.text);
        }
    }
    changed
}

fn round_half_away_from_zero(x: f32) -> f32 {
    if x >= 0.0 {
        (x + 0.5) as i32 as f32
    } else {
        (x - 0.5) as i32 as f32
    }
}

pub(crate) fn round_f64_to_i32(x: f64) -> i32 {
    let rounded = if x >= 0.0 {
        (x + 0.5) as i64
    } else {
        (x - 0.5) as i64
    };
    rounded.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

#[cfg(test)]
mod tests {
    use crate::ui::Ui;
    use crate::widgets::test_support::*;
    use alloc::vec::Vec;

    const AWAY: [f32; 2] = [-50.0, -50.0];

    fn track_rect(ui: &mut Ui, display: &str) -> [f32; 4] {
        let pos = ui.ctx.current_window_mut().cursor.next_pos();
        let available = ui.ctx.current_window_mut().cursor.available_width();
        let style = ui.ctx.style;
        let text_w = ui.ctx.font.measure(display)[0];
        let height = ui.ctx.font.line_height() + 2.0 * style.frame_padding[1];
        let track_w = (available - text_w - style.item_spacing[0]).max(0.0);
        [pos[0], pos[1], pos[0] + track_w, pos[1] + height]
    }

    fn center(track: [f32; 4]) -> [f32; 2] {
        [(track[0] + track[2]) / 2.0, (track[1] + track[3]) / 2.0]
    }

    fn settle_vol(ctx: &mut crate::context::Context, value: &mut f32) -> [f32; 4] {
        let mut track = [0.0; 4];
        frame(ctx, AWAY, false, |ui| {
            track = track_rect(ui, "Vol");
            ui.slider_float("Vol", value, 0.0..=10.0);
        });
        track
    }

    #[test]
    fn filled_slider_uses_full_row_and_bounds_its_fill() {
        let mut ctx = test_context();
        let mut value = 0.0;
        let mut track = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let width = ui.ctx.current_window_mut().cursor.available_width();
            let height = ui.ctx.font.line_height() + 2.0 * ui.ctx.style.frame_padding[1];
            track = [pos[0], pos[1], pos[0] + width, pos[1] + height];
            assert!(!ui.filled_slider_float("Fill", &mut value, 0.0..=10.0));
            assert_eq!(ui.ctx.last_item.unwrap().1, track);
        });
        let mid = center(track);
        let mut changed = false;
        frame(&mut ctx, mid, true, |ui| {
            changed = ui.filled_slider_float("Fill", &mut value, 0.0..=10.0)
        });
        assert!(changed);
        assert!((value - 5.0).abs() < 1e-3);
        let bg = ctx.style.widget_bg;
        let accent = ctx.style.widget_bg_active;
        let fill_color = [accent[0], accent[1], accent[2], accent[3]];
        let data = ctx.windows[0].draw_list.as_draw_data();
        let fill_vertices: Vec<_> = data
            .vertices
            .iter()
            .filter(|vertex| vertex.col == fill_color)
            .collect();
        assert!(!fill_vertices.is_empty());
        assert!(fill_vertices.iter().all(|vertex| vertex.pos[0] >= track[0]
            && vertex.pos[0] <= mid[0]
            && vertex.pos[1] >= track[1]
            && vertex.pos[1] <= track[3]));
        assert_ne!(bg, fill_color);
        frame(&mut ctx, [track[2] + 100.0, mid[1]], true, |ui| {
            changed = ui.filled_slider_float("Fill", &mut value, 0.0..=10.0)
        });
        assert!(changed);
        assert_eq!(value, 10.0);
    }

    #[test]
    fn filled_integer_slider_preserves_idle_host_value() {
        let mut ctx = test_context();
        let mut value = 7;
        frame(&mut ctx, AWAY, false, |ui| {
            assert!(!ui.filled_slider_int("Count", &mut value, 0..=10));
        });
        assert_eq!(value, 7);
    }

    #[test]
    fn drag_to_track_midpoint_sets_range_midpoint() {
        let mut ctx = test_context();
        let mut value = 0.0;
        let track = settle_vol(&mut ctx, &mut value);
        let mid = center(track);
        let mut changed = false;

        frame(&mut ctx, mid, true, |ui| {
            changed = ui.slider_float("Vol", &mut value, 0.0..=10.0)
        });
        assert!(changed, "dragging to a new position must report a change");
        assert!(
            (value - 5.0).abs() < 1e-3,
            "midpoint drag must set ≈ the range midpoint, got {value}"
        );
        assert_eq!(ctx.active, Some(widget_id_in_w("Vol")));

        frame(&mut ctx, mid, true, |ui| {
            changed = ui.slider_float("Vol", &mut value, 0.0..=10.0)
        });
        assert!(!changed, "holding still must not report a change");

        frame(&mut ctx, mid, false, |ui| {
            changed = ui.slider_float("Vol", &mut value, 0.0..=10.0)
        });
        assert!(!changed, "the release frame does not change the value");
        assert!((value - 5.0).abs() < 1e-3);
        assert_eq!(ctx.active, None);
    }

    #[test]
    fn slider_inside_popup_sizes_to_the_popup_and_drags_to_the_right_value() {
        let mut ctx = test_context();
        let popup_rect = [400.0, 300.0, 600.0, 460.0];
        let mut value = 0.0_f32;
        let mut track = [0.0; 4];

        frame(&mut ctx, AWAY, false, |ui| {
            let popup_id = ui.widget_id("##popup");
            ui.open_popup(popup_id);
            ui.popup(popup_id, popup_rect, |ui| {
                let pos = ui.ctx.overlay_cursor.next_pos();
                let style = ui.ctx.style;
                let height = ui.ctx.font.line_height() + 2.0 * style.frame_padding[1];
                let width = popup_rect[2] - popup_rect[0] - 2.0 * style.window_padding[0];
                track = [pos[0], pos[1], pos[0] + width, pos[1] + height];
                ui.slider_float("##s", &mut value, 0.0..=100.0);
            });
        });
        let data = ctx.overlay.as_draw_data();
        assert!(!data.vertices.is_empty());
        for v in data.vertices {
            assert!(
                v.pos[0] >= popup_rect[0]
                    && v.pos[0] <= popup_rect[2]
                    && v.pos[1] >= popup_rect[1]
                    && v.pos[1] <= popup_rect[3],
                "overlay vertex {:?} escapes the popup rect {popup_rect:?}",
                v.pos
            );
        }

        let mid = center(track);
        let mut changed = false;
        frame(&mut ctx, mid, true, |ui| {
            let popup_id = ui.widget_id("##popup");
            ui.popup(popup_id, popup_rect, |ui| {
                changed = ui.slider_float("##s", &mut value, 0.0..=100.0);
            });
        });
        assert!(
            changed,
            "the press frame already applies the clicked position"
        );
        assert!(
            (value - 50.0).abs() < 0.5,
            "drag to track midpoint must set ≈ the range midpoint, got {value}"
        );
    }

    #[test]
    fn drag_beyond_track_ends_clamps_to_range_ends() {
        let mut ctx = test_context();
        let mut value = 5.0;
        let track = settle_vol(&mut ctx, &mut value);
        let mid = center(track);

        frame(&mut ctx, mid, true, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
        });
        assert_eq!(ctx.active, Some(widget_id_in_w("Vol")));

        frame(&mut ctx, [track[2] + 100.0, mid[1]], true, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
        });
        assert_eq!(
            value, 10.0,
            "past the right end must clamp to the range end"
        );

        frame(&mut ctx, [track[0] - 100.0, mid[1]], true, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
        });
        assert_eq!(
            value, 0.0,
            "past the left end must clamp to the range start"
        );

        frame(&mut ctx, AWAY, false, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
        });
        assert_eq!(ctx.active, None);
    }

    #[test]
    fn drag_continues_while_active_when_mouse_leaves_track_vertically() {
        let mut ctx = test_context();
        let mut value = 0.0;
        let track = settle_vol(&mut ctx, &mut value);
        let mid = center(track);

        frame(&mut ctx, mid, true, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
        });
        assert_eq!(ctx.active, Some(widget_id_in_w("Vol")));

        let off_track = [track[0] + 0.75 * (track[2] - track[0]), track[3] + 150.0];
        let mut changed = false;
        frame(&mut ctx, off_track, true, |ui| {
            changed = ui.slider_float("Vol", &mut value, 0.0..=10.0)
        });
        assert!(changed);
        assert!(
            (value - 7.5).abs() < 1e-2,
            "active capture must follow the mouse x off-track, got {value}"
        );
        assert_eq!(
            ctx.active,
            Some(widget_id_in_w("Vol")),
            "capture must survive leaving the track"
        );

        frame(&mut ctx, off_track, false, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
        });
        assert_eq!(ctx.active, None);
        assert!((value - 7.5).abs() < 1e-2);
    }

    #[test]
    fn slider_drag_over_a_button_does_not_hover_the_button() {
        let mut ctx = test_context();
        let mut value = 0.0;
        let (mut track, mut btn) = ([0.0; 4], [0.0; 4]);
        frame(&mut ctx, AWAY, false, |ui| {
            track = track_rect(ui, "Vol");
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
            btn = next_text_rect(ui, "OK");
            ui.button("OK");
        });

        frame(&mut ctx, center(track), true, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
            ui.button("OK");
        });
        assert_eq!(ctx.active, Some(widget_id_in_w("Vol")));

        let mut clicked = true;
        frame(&mut ctx, center(btn), true, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
            clicked = ui.button("OK");
        });
        assert!(!clicked);
        assert_eq!(
            ctx.active,
            Some(widget_id_in_w("Vol")),
            "the slider keeps its capture"
        );
        assert_eq!(ctx.hot, None, "no widget is hovered mid-drag");
        assert_eq!(
            fill_color_at(&ctx, btn),
            Some(ctx.style.widget_bg),
            "the button stays idle-colored while the slider is dragged over it"
        );

        frame(&mut ctx, center(btn), false, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
            clicked = ui.button("OK");
        });
        assert!(
            !clicked,
            "release over the button without a press on it is not a click"
        );
        assert_eq!(ctx.active, None);

        frame(&mut ctx, center(btn), false, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
            ui.button("OK");
        });
        assert_eq!(ctx.hot, Some(widget_id_in_w("OK")));
        assert_eq!(fill_color_at(&ctx, btn), Some(ctx.style.widget_bg_hovered));
    }

    #[test]
    fn slider_int_rounds_to_integers_only() {
        let mut ctx = test_context();
        let mut value = 0i32;
        let mut track = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            track = track_rect(ui, "N");
            ui.slider_int("N", &mut value, 0..=10);
        });
        let y = (track[1] + track[3]) / 2.0;

        let x = track[0] + 0.34 * (track[2] - track[0]);
        frame(&mut ctx, [x, y], true, |ui| {
            ui.slider_int("N", &mut value, 0..=10);
        });
        assert_eq!(value, 3, "3.4 must round to 3");

        let x = track[0] + 0.36 * (track[2] - track[0]);
        let mut changed = false;
        frame(&mut ctx, [x, y], true, |ui| {
            changed = ui.slider_int("N", &mut value, 0..=10)
        });
        assert!(changed);
        assert_eq!(value, 4, "3.6 must round to 4");

        frame(&mut ctx, [x, y], false, |ui| {
            ui.slider_int("N", &mut value, 0..=10);
        });
    }

    #[test]
    fn slider_int_leaves_unrepresentable_values_untouched_without_interaction() {
        let mut ctx = test_context();
        let mut value = (1 << 24) + 1;
        let mut changed = true;
        frame(&mut ctx, AWAY, false, |ui| {
            changed = ui.slider_int("Big", &mut value, 0..=20_000_000);
        });
        assert!(
            !changed,
            "an interaction-free frame must not report a change"
        );
        assert_eq!(
            value,
            (1 << 24) + 1,
            "an interaction-free frame must not touch the value"
        );
    }

    #[test]
    fn slider_int_rounds_half_away_from_zero_over_negative_ranges() {
        let mut ctx = test_context();
        let mut value = 0i32;
        let mut track = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            track = track_rect(ui, "Neg");
            ui.slider_int("Neg", &mut value, -10..=0);
        });
        let y = (track[1] + track[3]) / 2.0;

        let x = track[0] + 0.36 * (track[2] - track[0]);
        frame(&mut ctx, [x, y], true, |ui| {
            ui.slider_int("Neg", &mut value, -10..=0);
        });
        assert_eq!(value, -6, "-6.4 must round to -6");

        let x = track[0] + 0.34 * (track[2] - track[0]);
        let mut changed = false;
        frame(&mut ctx, [x, y], true, |ui| {
            changed = ui.slider_int("Neg", &mut value, -10..=0);
        });
        assert!(changed);
        assert_eq!(value, -7, "-6.6 must round to -7");

        frame(&mut ctx, [x, y], false, |ui| {
            ui.slider_int("Neg", &mut value, -10..=0);
        });
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn reversed_range_swaps_instead_of_panicking() {
        let mut ctx = test_context();
        let mut value = 5i32;
        let mut track = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            track = track_rect(ui, "Rev");
            ui.slider_int("Rev", &mut value, 10..=0);
        });
        let y = (track[1] + track[3]) / 2.0;

        frame(&mut ctx, [track[0], y], true, |ui| {
            ui.slider_int("Rev", &mut value, 10..=0);
        });
        assert_eq!(value, 0, "the left end of the swapped range must be 0");
        frame(&mut ctx, [track[2], y], true, |ui| {
            ui.slider_int("Rev", &mut value, 10..=0);
        });
        assert_eq!(value, 10, "the right end of the swapped range must be 10");
        frame(&mut ctx, AWAY, false, |ui| {
            ui.slider_int("Rev", &mut value, 10..=0);
        });
        assert_eq!(ctx.active, None);
    }

    #[test]
    fn changed_flag_is_true_exactly_on_value_change_frames() {
        let mut ctx = test_context();
        let mut value = 0.0;
        let track = settle_vol(&mut ctx, &mut value);
        let y = (track[1] + track[3]) / 2.0;
        let mut changed = true;

        frame(&mut ctx, center(track), false, |ui| {
            changed = ui.slider_float("Vol", &mut value, 0.0..=10.0)
        });
        assert!(!changed, "hover without a press must not report a change");
        assert_eq!(ctx.hot, Some(widget_id_in_w("Vol")));

        frame(&mut ctx, [track[0], y], true, |ui| {
            changed = ui.slider_float("Vol", &mut value, 0.0..=10.0)
        });
        assert!(
            !changed,
            "a press that moves nothing must not report a change"
        );
        assert_eq!(ctx.active, Some(widget_id_in_w("Vol")));

        let mid = center(track);
        frame(&mut ctx, mid, true, |ui| {
            changed = ui.slider_float("Vol", &mut value, 0.0..=10.0)
        });
        assert!(changed);

        frame(&mut ctx, mid, true, |ui| {
            changed = ui.slider_float("Vol", &mut value, 0.0..=10.0)
        });
        assert!(!changed, "holding still must not report a change");

        frame(&mut ctx, mid, false, |ui| {
            changed = ui.slider_float("Vol", &mut value, 0.0..=10.0)
        });
        assert!(!changed);
        assert_eq!(ctx.active, None);
    }

    #[test]
    fn value_text_is_centered_on_the_track_and_label_sits_right_of_it() {
        let mut ctx = test_context();
        let mut value = 5.0;
        let track = settle_vol(&mut ctx, &mut value);

        let value_size = ctx.font.measure("5.00");
        let pen_x = track[0] + ((track[2] - track[0]) - value_size[0]) / 2.0;
        let (_, quad) = ctx
            .font
            .glyph_quad(crate::font::FontId::DEFAULT, '5', [pen_x, 0.0]);
        let expected_min_x = quad.expect("'5' has ink").min[0];
        let actual_min_x =
            glyph_min_x_in(&ctx, track).expect("slider must emit its value text on the track");
        assert!(
            (actual_min_x - expected_min_x).abs() < 1e-4,
            "value text min x {actual_min_x} != centered {expected_min_x}"
        );

        let style = ctx.style;
        let label_area = [track[2], track[1], track[2] + 200.0, track[3]];
        let label_min_x =
            glyph_min_x_in(&ctx, label_area).expect("slider must emit its label glyphs");
        assert!(
            label_min_x >= track[2] + style.item_spacing[0] - 1e-4,
            "label min x {label_min_x} must start right of the track plus spacing"
        );
    }

    #[test]
    fn grab_uses_state_colors_and_sits_at_the_value_position() {
        let mut ctx = test_context();
        let mut value = 5.0;
        let track = settle_vol(&mut ctx, &mut value);
        let style = ctx.style;

        let grab_w = style.slider_grab_size.min(track[2] - track[0]);
        let t = (value / 10.0f32).clamp(0.0, 1.0);
        let grab_x = (track[0] + t * (track[2] - track[0]) - grab_w / 2.0)
            .clamp(track[0], track[2] - grab_w);
        let grab = [grab_x, track[1], grab_x + grab_w, track[3]];
        assert_eq!(
            fill_color_at(&ctx, grab),
            Some(style.widget_bg),
            "idle grab"
        );

        frame(&mut ctx, center(track), false, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
        });
        assert_eq!(
            fill_color_at(&ctx, grab),
            Some(style.widget_bg_hovered),
            "hovered grab"
        );

        frame(&mut ctx, center(track), true, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
        });
        frame(&mut ctx, [center(track)[0], track[3] + 120.0], true, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
        });
        assert_eq!(
            fill_color_at(&ctx, grab),
            Some(style.widget_bg_active),
            "grab must use the active color while captured"
        );
        frame(&mut ctx, AWAY, false, |ui| {
            ui.slider_float("Vol", &mut value, 0.0..=10.0);
        });
    }
}
