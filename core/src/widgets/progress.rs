use crate::id::split_label;
use crate::ui::Ui;

pub(crate) fn progress_bar(ui: &mut Ui, label: &str, fraction: f32, size: [f32; 2]) {
    let ctx = &mut *ui.ctx;
    let style = ctx.style;
    let face = ctx.active_font();
    let width = if size[0].is_finite() {
        size[0].max(0.0)
    } else {
        0.0
    };
    let height = if size[1].is_finite() {
        size[1].max(0.0)
    } else {
        0.0
    };
    let id = super::widget_id(ctx, label);
    let rect = super::item_rect(ctx, [width, height]);
    let fraction = if fraction.is_finite() {
        fraction.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if ctx.accessibility_enabled() {
        use core::fmt::Write;
        let mut text = super::FmtBuf::<32>::new();
        let _ = write!(text, "{:.0}%", fraction * 100.);
        ctx.id_stack.semantics.describe(
            id,
            crate::accessibility::AccessibilityRole::ProgressBar,
            text.as_str(),
            0,
            0,
        );
    }
    let (font, draw_list) = super::draw_parts(ctx);
    super::chrome_fill(
        font,
        draw_list,
        style.corner_rounding,
        rect,
        style.widget_bg,
    );
    if fraction > 0.0 {
        let fill = [
            rect[0],
            rect[1],
            rect[0] + (rect[2] - rect[0]) * fraction,
            rect[3],
        ];
        super::chrome_fill(
            font,
            draw_list,
            style.corner_rounding,
            fill,
            style.widget_bg_active,
        );
    }
    if style.border_size > 0.0 {
        super::draw_border(
            font,
            draw_list,
            style.corner_rounding,
            rect,
            style.border_size,
            super::lighten(style.widget_bg, super::BORDER_LIGHTEN),
        );
    }
    let display = split_label(label).0;
    if !display.is_empty() {
        let text = font.measure_in(face, display);
        let pos = [
            rect[0] + ((rect[2] - rect[0]) - text[0]) / 2.0,
            rect[1] + ((rect[3] - rect[1]) - text[1]) / 2.0,
        ];
        draw_list.add_text(font, face, pos, display, style.text);
    }
}
