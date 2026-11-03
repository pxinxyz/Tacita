use super::{lighten, BORDER_LIGHTEN};
use crate::draw::{DrawList, TextureId};
use crate::font::{Font, ShapeKey};
use crate::id::split_label;
use crate::ui::{hover_layer_gate, hovered, point_in_rect, Ui};

pub(crate) fn radio_button(ui: &mut Ui, label: &str, selected: &mut usize, index: usize) -> bool {
    let ctx = &mut *ui.ctx;
    let id = super::widget_id(ctx, label);
    let display = split_label(label).0;
    let style = ctx.style;
    let face = ctx.active_font();
    let side = ctx.font.line_height_in(face);
    let text_size = ctx.font.measure_in(face, display);
    let width = if display.is_empty() {
        side
    } else {
        side + style.item_spacing[0] + text_size[0]
    };
    let rect = super::interactive_rect(ctx, [width, side]);
    super::register_focusable_item(ctx, id, rect);
    let marker = [rect[0], rect[1], rect[0] + side, rect[3]];

    let mouse = ctx.input.mouse_pos;
    let inside = point_in_rect(mouse, ctx.target_rect(rect));
    let down = ctx.input.mouse_down[0];
    if inside && hover_layer_gate(ctx) && down && !ctx.prev_mouse_down[0] {
        ctx.set_active(id);
    }
    let held = ctx.active == Some(id);
    let clicked =
        (held && !down && inside && hover_layer_gate(ctx)) || ctx.keyboard_activate == Some(id);
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
        crate::accessibility::AccessibilityRole::RadioButton,
        if *selected == index { "selected" } else { "" },
        if *selected == index {
            crate::accessibility::AccessibilityState::CHECKED
        } else {
            0
        },
        3,
    );
    let bg = if ctx.active == Some(id) && inside {
        style.widget_bg_active
    } else if over {
        style.widget_bg_hovered
    } else {
        style.widget_bg
    };

    let bg = ctx.widget_color(id, bg);
    let selection = ctx.motion_tween(
        id,
        crate::animation::INDICATOR,
        if *selected == index { 1.0 } else { 0.0 },
        if *selected == index { 1.0 } else { 0.0 },
        ctx.motion.indicator,
        ctx.motion.easing,
    );
    let mut dot_color = style.text;
    dot_color[3] = (dot_color[3] as f32 * selection + 0.5) as u8;
    let center = [(marker[0] + marker[2]) * 0.5, (marker[1] + marker[3]) * 0.5];
    let radius = side * 0.5;
    let (font, draw_list) = super::draw_parts(ctx);
    if !circle_mask(font, draw_list, ShapeKey::disk(radius), center, bg) {
        draw_list.add_filled_circle(center, radius, bg, false);
    }
    if style.border_size > 0.0 {
        let thickness = style.border_size.min(side);
        let border_color = lighten(bg, BORDER_LIGHTEN);
        if thickness >= side {
            if !circle_mask(
                font,
                draw_list,
                ShapeKey::disk(radius),
                center,
                border_color,
            ) {
                draw_list.add_filled_circle(center, radius, border_color, false);
            }
        } else {
            let centerline = radius - thickness * 0.5;
            if !circle_mask(
                font,
                draw_list,
                ShapeKey::ring(centerline, thickness),
                center,
                border_color,
            ) {
                draw_list.add_circle_outline(center, centerline, border_color, thickness, false);
            }
        }
    }
    if selection > 0.0 {
        let dot_radius = side / 6.0;
        if !circle_mask(
            font,
            draw_list,
            ShapeKey::disk(dot_radius),
            center,
            dot_color,
        ) {
            draw_list.add_filled_circle(center, dot_radius, dot_color, false);
        }
    }
    if !display.is_empty() {
        draw_list.add_text(
            font,
            face,
            [
                marker[2] + style.item_spacing[0],
                rect[1] + (side - text_size[1]) / 2.0,
            ],
            display,
            style.text,
        );
    }
    changed
}

pub(crate) fn circle_mask(
    font: &mut Font,
    draw_list: &mut DrawList,
    key: ShapeKey,
    center: [f32; 2],
    color: [u8; 4],
) -> bool {
    let Some(mask) = font.shape_mask(key) else {
        return false;
    };
    let half = mask.rect[2] as f32 / (2.0 * mask.raster_scale);
    draw_list.add_textured_quad(
        TextureId::FONT,
        [center[0] - half, center[1] - half],
        [center[0] + half, center[1] + half],
        mask.uv_min,
        mask.uv_max,
        color,
    );
    true
}
