use super::{draw_border, lighten, BORDER_LIGHTEN};
use crate::id::split_label;
use crate::ui::{hovered, point_in_rect, Ui};

pub(crate) fn button(ui: &mut Ui, label: &str) -> bool {
    button_impl(ui, label, None)
}

pub(crate) fn icon_button(ui: &mut Ui, label: &str, side: f32) -> bool {
    button_impl(ui, label, Some(side))
}

fn button_impl(ui: &mut Ui, label: &str, square_side: Option<f32>) -> bool {
    let ctx = &mut *ui.ctx;
    let id = super::widget_id(ctx, label);
    let display = split_label(label).0;
    let style = ctx.style;
    let face = ctx.active_font();
    let text_size = ctx.font.measure_in(face, display);
    let natural = [
        text_size[0] + 2.0 * style.frame_padding[0],
        text_size[1] + 2.0 * style.frame_padding[1],
    ];
    let size = if let Some(side) = square_side {
        let side = if side.is_finite() { side.max(0.0) } else { 0.0 };
        let side = side.max(natural[0]).max(natural[1]);
        [side, side]
    } else {
        natural
    };
    let rect = super::interactive_rect(ctx, size);
    super::register_focusable_item(ctx, id, rect);

    let mouse = ctx.input.mouse_pos;
    let inside = point_in_rect(mouse, ctx.target_rect(rect));
    let left_down = ctx.input.mouse_down[0];
    let left_pressed = left_down && !ctx.prev_mouse_down[0];
    if inside && left_pressed {
        ctx.set_active(id);
    }
    let held = ctx.active == Some(id);
    let clicked = (held && !left_down && inside) || ctx.keyboard_activate == Some(id);
    if held && !left_down {
        ctx.clear_active();
    }

    let hovered = hovered(ctx, rect);
    if hovered {
        ctx.hot = Some(id);
    }
    let active = ctx.active == Some(id);
    let bg = if active && inside {
        style.widget_bg_active
    } else if hovered {
        style.widget_bg_hovered
    } else {
        style.widget_bg
    };

    let bg = ctx.widget_color(id, bg);
    let (font, draw_list) = super::draw_parts(ctx);
    super::chrome_fill(font, draw_list, style.corner_rounding, rect, bg);
    if style.border_size > 0.0 {
        draw_border(
            font,
            draw_list,
            style.corner_rounding,
            rect,
            style.border_size,
            lighten(bg, BORDER_LIGHTEN),
        );
    }
    let text_pos = [
        rect[0] + (size[0] - text_size[0]) / 2.0,
        rect[1] + (size[1] - text_size[1]) / 2.0,
    ];
    draw_list.add_text(font, face, text_pos, display, style.text);
    clicked
}

#[cfg(test)]
mod tests {
    use crate::widgets::test_support::*;

    const AWAY: [f32; 2] = [-50.0, -50.0];

    fn settle_ok(ctx: &mut crate::context::Context) -> [f32; 4] {
        let mut rect = [0.0; 4];
        frame(ctx, AWAY, false, |ui| {
            rect = next_text_rect(ui, "OK");
            ui.button("OK");
        });
        rect
    }

    fn center(rect: [f32; 4]) -> [f32; 2] {
        [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
    }

    #[test]
    fn unhovered_button_returns_false_and_emits_idle_color() {
        let mut ctx = test_context();
        let mut clicked = true;
        let mut rect = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            rect = next_text_rect(ui, "OK");
            clicked = ui.button("OK");
        });
        assert!(!clicked);
        assert_eq!(ctx.hot, None);
        assert_eq!(fill_color_at(&ctx, rect), Some(ctx.style.widget_bg));
        let glyphs = glyph_colors_in(&ctx, rect);
        assert!(!glyphs.is_empty(), "button must emit its label glyphs");
        assert!(glyphs.iter().all(|&c| c == ctx.style.text));
    }

    #[test]
    fn button_chrome_is_aa_invariant() {
        fn window_vertices(aa: bool) -> alloc::vec::Vec<crate::draw::DrawVert> {
            let mut ctx = test_context();
            ctx.style.anti_aliasing = aa;
            frame(&mut ctx, AWAY, false, |ui| {
                ui.button("OK");
            });
            ctx.windows[0].draw_list.as_draw_data().vertices.to_vec()
        }
        let off = window_vertices(false);
        let on = window_vertices(true);
        assert!(
            on.len() > off.len(),
            "the × fringe must grow the AA emission"
        );
        let mut it = on.iter();
        for v in &off {
            assert!(
                it.by_ref().any(|w| w == v),
                "crisp vertex {v:?} missing from the AA emission"
            );
        }
    }

    #[test]
    fn hovered_button_emits_hover_color_and_sets_hot() {
        let mut ctx = test_context();
        let rect = settle_ok(&mut ctx);
        let mut clicked = true;
        frame(&mut ctx, center(rect), false, |ui| {
            clicked = ui.button("OK");
        });
        assert!(!clicked, "hover without a press is not a click");
        assert_eq!(ctx.hot, Some(widget_id_in_w("OK")));
        assert_eq!(fill_color_at(&ctx, rect), Some(ctx.style.widget_bg_hovered));
    }

    #[test]
    fn press_and_release_inside_clicks_on_the_release_frame_only() {
        let mut ctx = test_context();
        let rect = settle_ok(&mut ctx);
        let at = center(rect);
        let mut clicked = true;

        frame(&mut ctx, at, true, |ui| clicked = ui.button("OK"));
        assert!(!clicked, "the press frame must not report a click");
        assert_eq!(ctx.active, Some(widget_id_in_w("OK")));
        assert_eq!(fill_color_at(&ctx, rect), Some(ctx.style.widget_bg_active));

        frame(&mut ctx, at, false, |ui| clicked = ui.button("OK"));
        assert!(clicked, "the release frame inside must report the click");
        assert_eq!(ctx.active, None);
        assert_eq!(fill_color_at(&ctx, rect), Some(ctx.style.widget_bg_hovered));

        frame(&mut ctx, at, false, |ui| clicked = ui.button("OK"));
        assert!(
            !clicked,
            "the click must not repeat after the release frame"
        );
    }

    #[test]
    fn press_inside_release_outside_does_not_click() {
        let mut ctx = test_context();
        let rect = settle_ok(&mut ctx);
        let mut clicked = true;

        frame(&mut ctx, center(rect), true, |ui| clicked = ui.button("OK"));
        assert!(!clicked);
        assert_eq!(ctx.active, Some(widget_id_in_w("OK")));

        let outside = [rect[0] - 30.0, rect[1]];
        frame(&mut ctx, outside, false, |ui| clicked = ui.button("OK"));
        assert!(!clicked, "release outside the rect is not a click");
        assert_eq!(ctx.active, None, "capture must drop on release either way");
    }

    #[test]
    fn hidden_id_suffixes_distinguish_buttons_but_display_the_same_text() {
        let mut ctx = test_context();
        let mut rect_a = [0.0; 4];
        let mut rect_b = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            rect_a = next_text_rect(ui, "OK");
            ui.button("OK##a");
            rect_b = next_text_rect(ui, "OK");
            ui.button("OK##b");
        });
        assert_ne!(rect_a, rect_b);
        assert_ne!(widget_id_in_w("OK##a"), widget_id_in_w("OK##b"));
        assert!(!glyph_colors_in(&ctx, rect_a).is_empty());
        assert!(!glyph_colors_in(&ctx, rect_b).is_empty());

        let at_a = center(rect_a);
        let mut clicks = (true, true);
        frame(&mut ctx, at_a, true, |ui| {
            clicks.0 = ui.button("OK##a");
            clicks.1 = ui.button("OK##b");
        });
        assert_eq!(ctx.active, Some(widget_id_in_w("OK##a")));
        frame(&mut ctx, at_a, false, |ui| {
            clicks.0 = ui.button("OK##a");
            clicks.1 = ui.button("OK##b");
        });
        assert!(clicks.0, "button a was pressed and released inside");
        assert!(!clicks.1, "button b must not click from a's press");
    }

    #[test]
    fn border_strips_ring_the_rect_in_a_lightened_state_color() {
        let mut ctx = test_context();
        assert!(
            ctx.style.border_size > 0.0,
            "the default style draws borders"
        );
        let rect = settle_ok(&mut ctx);
        let t = ctx.style.border_size;
        let expected = super::lighten(ctx.style.widget_bg, super::BORDER_LIGHTEN);
        let r = ctx
            .style
            .corner_rounding
            .min((rect[2] - rect[0]).min(rect[3] - rect[1]) / 2.0);
        let top = [rect[0] + r, rect[1], rect[2] - r, rect[1] + t];
        let bottom = [rect[0] + r, rect[3] - t, rect[2] - r, rect[3]];
        let left = [rect[0], rect[1] + r, rect[0] + t, rect[3] - r];
        let right = [rect[2] - t, rect[1] + r, rect[2], rect[3] - r];
        for strip in [top, bottom, left, right] {
            assert_eq!(
                fill_color_at(&ctx, strip),
                Some(expected),
                "border strip {strip:?} must be emitted in the lightened state color"
            );
        }
        ctx.style.corner_rounding = 0.0;
        frame(&mut ctx, AWAY, false, |ui| {
            ui.button("OK");
        });
        let top = [rect[0], rect[1], rect[2], rect[1] + t];
        let bottom = [rect[0], rect[3] - t, rect[2], rect[3]];
        let left = [rect[0], rect[1] + t, rect[0] + t, rect[3] - t];
        let right = [rect[2] - t, rect[1] + t, rect[2], rect[3] - t];
        for strip in [top, bottom, left, right] {
            assert_eq!(
                fill_color_at(&ctx, strip),
                Some(expected),
                "square border strip {strip:?} must be emitted in the lightened state color"
            );
        }

        ctx.style.border_size = 0.0;
        frame(&mut ctx, AWAY, false, |ui| {
            ui.button("OK");
        });
        assert_eq!(fill_color_at(&ctx, top), None);
    }

    #[test]
    fn button_text_is_centered_horizontally_in_the_rect() {
        let mut ctx = test_context();
        let rect = settle_ok(&mut ctx);
        let text_size = ctx.font.measure("OK");
        let centered_pen_x = rect[0] + ((rect[2] - rect[0]) - text_size[0]) / 2.0;
        let (_, quad) =
            ctx.font
                .glyph_quad(crate::font::FontId::DEFAULT, 'O', [centered_pen_x, 0.0]);
        let expected_min_x = quad.expect("'O' has ink").min[0];
        let actual_min_x = glyph_min_x_in(&ctx, rect).expect("button label glyphs must be emitted");
        assert!(
            (actual_min_x - expected_min_x).abs() < 1e-4,
            "text min x {actual_min_x} != centered {expected_min_x}"
        );
    }

    #[test]
    fn same_line_places_the_next_button_beside_the_previous_one() {
        let mut ctx = test_context();
        let mut rect_a = [0.0; 4];
        let mut rect_b = [0.0; 4];
        frame(&mut ctx, AWAY, false, |ui| {
            rect_a = next_text_rect(ui, "A");
            ui.button("A");
            ui.same_line();
            rect_b = next_text_rect(ui, "B");
            ui.button("B");
        });
        let spacing = ctx.style.item_spacing;
        assert_eq!(rect_b[1], rect_a[1]);
        assert_eq!(rect_b[0], rect_a[2] + spacing[0]);
    }
}
