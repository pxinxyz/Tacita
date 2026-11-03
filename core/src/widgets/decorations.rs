use super::{chrome_fill, draw_border, lighten, BORDER_LIGHTEN};
use crate::id::split_label;
use crate::ui::{draw_checkerboard, Ui};

pub(crate) fn separator(ui: &mut Ui) {
    let ctx = &mut *ui.ctx;
    let style = ctx.style;
    let thickness = style.border_size.max(1.0);
    let width = super::available_width(ctx).max(0.0);
    let rect = super::item_rect(ctx, [width, thickness]);
    let (_, draw_list) = super::draw_parts(ctx);
    draw_list.add_filled_rect(
        [rect[0], rect[1]],
        [rect[2], rect[1] + thickness],
        lighten(style.widget_bg, BORDER_LIGHTEN),
    );
}

pub(crate) fn color_swatch(ui: &mut Ui, label: &str, color: [u8; 4], size: [f32; 2]) {
    let ctx = &mut *ui.ctx;
    let style = ctx.style;
    let display = split_label(label).0;
    let face = ctx.active_font();
    let text = ctx.font.measure_in(face, display);
    let swatch_size = size.map(|axis| if axis.is_finite() { axis.max(0.0) } else { 0.0 });
    let gap = if display.is_empty() {
        0.0
    } else {
        style.item_spacing[0]
    };
    let item_size = [swatch_size[0] + gap + text[0], swatch_size[1].max(text[1])];
    let rect = super::item_rect(ctx, item_size);
    let swatch = [
        rect[0],
        rect[1] + (item_size[1] - swatch_size[1]) / 2.0,
        rect[0] + swatch_size[0],
        rect[1] + (item_size[1] + swatch_size[1]) / 2.0,
    ];
    let (font, draw_list) = super::draw_parts(ctx);
    draw_checkerboard(draw_list, swatch);
    draw_list.add_filled_rect([swatch[0], swatch[1]], [swatch[2], swatch[3]], color);
    if style.border_size > 0.0 {
        draw_border(
            font,
            draw_list,
            0.0,
            swatch,
            style.border_size,
            lighten(style.widget_bg, BORDER_LIGHTEN),
        );
    }
    if !display.is_empty() {
        draw_list.add_text(
            font,
            face,
            [swatch[2] + gap, rect[1] + (item_size[1] - text[1]) / 2.0],
            display,
            style.text,
        );
    }
}

pub(crate) fn status_badge(ui: &mut Ui, label: &str, color: [u8; 4]) {
    let ctx = &mut *ui.ctx;
    let style = ctx.style;
    let display = split_label(label).0;
    let face = ctx.active_font();
    let text = ctx.font.measure_in(face, display);
    let dot = 6.0;
    let gap = if display.is_empty() {
        0.0
    } else {
        style.frame_padding[0]
    };
    let size = [
        2.0 * style.frame_padding[0] + dot + gap + text[0],
        (text[1] + 2.0 * style.frame_padding[1]).max(dot + 2.0 * style.frame_padding[1]),
    ];
    let rect = super::item_rect(ctx, size);
    let backdrop = [
        color[0],
        color[1],
        color[2],
        ((color[3] as u16 * 38) / 255) as u8,
    ];
    let edge = [
        color[0],
        color[1],
        color[2],
        ((color[3] as u16 * 96) / 255) as u8,
    ];
    let (font, draw_list) = super::draw_parts(ctx);
    chrome_fill(font, draw_list, style.corner_rounding, rect, backdrop);
    if style.border_size > 0.0 {
        draw_border(
            font,
            draw_list,
            style.corner_rounding,
            rect,
            style.border_size,
            edge,
        );
    }
    let dot_x = rect[0] + style.frame_padding[0];
    let dot_y = rect[1] + (size[1] - dot) / 2.0;
    draw_list.add_filled_rect([dot_x, dot_y], [dot_x + dot, dot_y + dot], color);
    if !display.is_empty() {
        draw_list.add_text(
            font,
            face,
            [dot_x + dot + gap, rect[1] + (size[1] - text[1]) / 2.0],
            display,
            style.text,
        );
    }
}

pub(crate) fn hotkey_hint(ui: &mut Ui, label: &str) {
    let ctx = &mut *ui.ctx;
    let style = ctx.style;
    let display = split_label(label).0;
    let face = ctx.active_font();
    let left = "[ ";
    let right = " ]";
    let left_size = ctx.font.measure_in(face, left);
    let text_size = ctx.font.measure_in(face, display);
    let right_size = ctx.font.measure_in(face, right);
    let size = [
        left_size[0] + text_size[0] + right_size[0] + 2.0 * style.frame_padding[0],
        left_size[1].max(text_size[1]).max(right_size[1]) + 2.0 * style.frame_padding[1],
    ];
    let rect = super::item_rect(ctx, size);
    let (font, draw_list) = super::draw_parts(ctx);
    chrome_fill(
        font,
        draw_list,
        style.corner_rounding,
        rect,
        style.widget_bg,
    );
    if style.border_size > 0.0 {
        draw_border(
            font,
            draw_list,
            style.corner_rounding,
            rect,
            style.border_size,
            style.scrollbar_thumb,
        );
    }
    let x = rect[0] + style.frame_padding[0];
    let y = rect[1] + style.frame_padding[1];
    draw_list.add_text(font, face, [x, y], left, style.scrollbar_thumb_hovered);
    draw_list.add_text(font, face, [x + left_size[0], y], display, style.text);
    draw_list.add_text(
        font,
        face,
        [x + left_size[0] + text_size[0], y],
        right,
        style.scrollbar_thumb_hovered,
    );
}
