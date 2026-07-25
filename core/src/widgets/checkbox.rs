use super::{draw_border, lighten, BORDER_LIGHTEN};
use crate::id::split_label;
use crate::ui::{hovered, point_in_rect, Ui};

// A square, a check, a bool flipped in place. The simplest widget we ship, and still the one hosts will hand garbage bytes to through the FFI.
pub(crate) fn checkbox(ui: &mut Ui, label: &str, value: &mut bool) -> bool {
    let ctx = &mut *ui.ctx;
    let id = super::widget_id(ctx, label);
    let display = split_label(label).0;
    let style = ctx.style;
    let side = ctx.font.line_height();
    let text_size = ctx.font.measure(display);
    let size = if display.is_empty() {
        [side, side]
    } else {
        [side + style.item_spacing[0] + text_size[0], side]
    };
    let rect = super::item_rect(ctx, size);
    super::register_item(ctx, id, rect);
    let box_rect = [rect[0], rect[1], rect[0] + side, rect[3]];

    let mouse = ctx.input.mouse_pos;
    let inside = point_in_rect(mouse, rect);
    let left_down = ctx.input.mouse_down[0];
    let left_pressed = left_down && !ctx.prev_mouse_down[0];
    if inside && left_pressed {
        ctx.set_active(id);
    }
    let held = ctx.active == Some(id);
    let toggled = held && !left_down && inside;
    if held && !left_down {
        ctx.clear_active();
    }
    if toggled {
        *value = !*value;
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

    let (font, draw_list) = super::draw_parts(ctx);
    super::chrome_fill(font, draw_list, style.corner_rounding, box_rect, bg);
    if style.border_size > 0.0 {
        draw_border(
            font,
            draw_list,
            style.corner_rounding,
            box_rect,
            style.border_size,
            lighten(bg, BORDER_LIGHTEN),
        );
    }
    if *value {
        let inset = side / 4.0;
        draw_list.add_filled_rect(
            [box_rect[0] + inset, box_rect[1] + inset],
            [box_rect[2] - inset, box_rect[3] - inset],
            style.text,
        );
    }
    if !display.is_empty() {
        let text_pos = [
            box_rect[2] + style.item_spacing[0],
            rect[1] + (side - text_size[1]) / 2.0,
        ];
        draw_list.add_text(font, text_pos, display, style.text);
    }
    toggled
}

#[cfg(test)]
mod tests {
    use crate::context::Context;
    use crate::input::InputState;
    use crate::ui::{Ui, TITLE_BAR_TAG};
    use crate::widgets::test_support::*;

    const AWAY: [f32; 2] = [-50.0, -50.0];

    fn settle_on(ctx: &mut crate::context::Context, value: &mut bool) -> ([f32; 4], [f32; 4]) {
        let mut box_rect = [0.0; 4];
        let mut item_rect = [0.0; 4];
        frame(ctx, AWAY, false, |ui| {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let side = ui.ctx.font.line_height();
            let style = ui.ctx.style;
            let text_w = ui.ctx.font.measure("On")[0];
            let width = side + style.item_spacing[0] + text_w;
            box_rect = [pos[0], pos[1], pos[0] + side, pos[1] + side];
            item_rect = [pos[0], pos[1], pos[0] + width, pos[1] + side];
            ui.checkbox("On", value);
        });
        (box_rect, item_rect)
    }

    fn center(rect: [f32; 4]) -> [f32; 2] {
        [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
    }

    fn check_rect(box_rect: [f32; 4]) -> [f32; 4] {
        let side = box_rect[2] - box_rect[0];
        let inset = side / 4.0;
        [
            box_rect[0] + inset,
            box_rect[1] + inset,
            box_rect[2] - inset,
            box_rect[3] - inset,
        ]
    }

    fn chrome_id(title: &str) -> crate::id::Id {
        let mut stack = crate::id::IdStack::new();
        stack.push(title);
        stack.id(TITLE_BAR_TAG)
    }

    fn frame_w_b(ctx: &mut Context, mouse_pos: [f32; 2], left_down: bool, f: impl FnOnce(&mut Ui)) {
        ctx.set_input(InputState {
            mouse_pos,
            mouse_down: [left_down, false, false],
            ..Default::default()
        });
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("W", f);
            ui.window("B", |_| {});
        }
        ctx.render();
    }

    #[test]
    fn title_bar_drag_over_a_checkbox_does_not_hover_it() {
        let mut ctx = test_context();
        let mut value = false;
        let mut box_rect = [0.0; 4];
        frame_w_b(&mut ctx, AWAY, false, |ui| {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let side = ui.ctx.font.line_height();
            box_rect = [pos[0], pos[1], pos[0] + side, pos[1] + side];
            ui.checkbox("On", &mut value);
        });

        let tb = ctx.font.line_height() + 2.0 * ctx.style.frame_padding[1];
        let b_pos = ctx.windows[1].pos;
        let grab = [b_pos[0] + 20.0, b_pos[1] + tb / 2.0];
        frame_w_b(&mut ctx, grab, true, |ui| {
            ui.checkbox("On", &mut value);
        });
        assert_eq!(
            ctx.active,
            Some(chrome_id("B")),
            "the title bar holds the capture during the drag"
        );

        let over = center(box_rect);
        let mut toggled = true;
        frame_w_b(&mut ctx, over, true, |ui| {
            toggled = ui.checkbox("On", &mut value);
        });
        assert!(!toggled);
        assert_eq!(ctx.active, Some(chrome_id("B")));
        assert_eq!(
            ctx.hot, None,
            "no widget is hovered while a window is dragged"
        );
        assert_eq!(
            fill_color_at(&ctx, box_rect),
            Some(ctx.style.widget_bg),
            "the checkbox stays idle-colored under a dragged window"
        );

        frame_w_b(&mut ctx, over, false, |ui| {
            ui.checkbox("On", &mut value);
        });
        assert_eq!(ctx.active, None);
        ctx.windows[1].pos = b_pos;
        frame_w_b(&mut ctx, over, false, |ui| {
            ui.checkbox("On", &mut value);
        });
        assert_eq!(ctx.hot, Some(widget_id_in_w("On")));
        assert_eq!(
            fill_color_at(&ctx, box_rect),
            Some(ctx.style.widget_bg_hovered)
        );
    }

    #[test]
    fn unchecked_checkbox_emits_idle_box_and_label_without_check_mark() {
        let mut ctx = test_context();
        let mut value = false;
        let (box_rect, item_rect) = settle_on(&mut ctx, &mut value);

        assert!(!value);
        assert_eq!(ctx.hot, None);
        assert_eq!(fill_color_at(&ctx, box_rect), Some(ctx.style.widget_bg));
        assert_eq!(fill_color_at(&ctx, check_rect(box_rect)), None);
        let min_x = glyph_min_x_in(&ctx, item_rect).expect("checkbox must emit label glyphs");
        assert!(
            min_x >= box_rect[2] + ctx.style.item_spacing[0] - 1e-4,
            "label min x {min_x} must start right of the box plus spacing"
        );
    }

    #[test]
    fn hovered_checkbox_emits_hover_color_and_sets_hot() {
        let mut ctx = test_context();
        let mut value = false;
        let (_, item_rect) = settle_on(&mut ctx, &mut value);
        let mut toggled = true;
        let mut box_rect = [0.0; 4];
        frame(&mut ctx, center(item_rect), false, |ui| {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let side = ui.ctx.font.line_height();
            box_rect = [pos[0], pos[1], pos[0] + side, pos[1] + side];
            toggled = ui.checkbox("On", &mut value);
        });
        assert!(!toggled, "hover without a press is not a toggle");
        assert!(!value);
        assert_eq!(ctx.hot, Some(widget_id_in_w("On")));
        assert_eq!(
            fill_color_at(&ctx, box_rect),
            Some(ctx.style.widget_bg_hovered)
        );
    }

    #[test]
    fn click_toggles_value_and_returns_true_on_the_release_frame_only() {
        let mut ctx = test_context();
        let mut value = false;
        let (box_rect, _) = settle_on(&mut ctx, &mut value);
        let at = center(box_rect);
        let mut toggled = true;

        frame(&mut ctx, at, true, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        assert!(!toggled, "the press frame must not toggle");
        assert!(!value);
        assert_eq!(ctx.active, Some(widget_id_in_w("On")));
        assert_eq!(
            fill_color_at(&ctx, box_rect),
            Some(ctx.style.widget_bg_active)
        );

        frame(&mut ctx, at, false, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        assert!(toggled, "the release frame inside must toggle");
        assert!(value);
        assert_eq!(ctx.active, None);

        frame(&mut ctx, at, false, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        assert!(
            !toggled,
            "the toggle must not repeat after the release frame"
        );
        assert!(value);
        assert_eq!(
            fill_color_at(&ctx, check_rect(box_rect)),
            Some(ctx.style.text),
            "a checked checkbox must emit its check mark"
        );
    }

    #[test]
    fn second_click_toggles_back() {
        let mut ctx = test_context();
        let mut value = false;
        let (box_rect, _) = settle_on(&mut ctx, &mut value);
        let at = center(box_rect);
        let mut toggled = false;

        frame(&mut ctx, at, true, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        frame(&mut ctx, at, false, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        assert!(toggled && value);

        frame(&mut ctx, at, true, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        frame(&mut ctx, at, false, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        assert!(toggled && !value, "the second click must toggle back off");
        assert_eq!(fill_color_at(&ctx, check_rect(box_rect)), None);
    }

    #[test]
    fn press_and_release_away_from_the_widget_does_not_toggle() {
        let mut ctx = test_context();
        let mut value = false;
        let _ = settle_on(&mut ctx, &mut value);
        let mut toggled = true;

        frame(&mut ctx, AWAY, true, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        assert!(!toggled);
        assert_eq!(ctx.active, None);
        frame(&mut ctx, AWAY, false, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        assert!(!toggled);
        assert!(!value, "a click off the widget must not toggle it");
    }

    #[test]
    fn press_inside_release_outside_does_not_toggle() {
        let mut ctx = test_context();
        let mut value = false;
        let (box_rect, _) = settle_on(&mut ctx, &mut value);
        let mut toggled = true;

        frame(&mut ctx, center(box_rect), true, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        assert!(!toggled);
        assert_eq!(ctx.active, Some(widget_id_in_w("On")));

        let outside = [box_rect[0] - 30.0, box_rect[1]];
        frame(&mut ctx, outside, false, |ui| {
            toggled = ui.checkbox("On", &mut value)
        });
        assert!(!toggled, "release outside the rect is not a toggle");
        assert!(!value);
        assert_eq!(ctx.active, None, "capture must drop on release either way");
    }
}
