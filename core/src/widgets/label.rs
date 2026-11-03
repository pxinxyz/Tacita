use crate::id::split_label;
use crate::ui::Ui;

pub(crate) fn label(ui: &mut Ui, text: &str) {
    let ctx = &mut *ui.ctx;
    let _ = ctx.id_stack.id(text);
    let display = split_label(text).0;
    let face = ctx.active_font();
    let size = ctx.font.measure_in(face, display);
    let rect = super::item_rect(ctx, size);
    let color = ctx.style.text;
    let (font, draw_list) = super::draw_parts(ctx);
    draw_list.add_text(font, face, [rect[0], rect[1]], display, color);
}

#[cfg(test)]
mod tests {
    use crate::widgets::test_support::*;

    #[test]
    fn label_advances_cursor_by_measured_size_and_emits_glyphs() {
        let mut ctx = test_context();
        let mut before = [0.0; 2];
        let mut after = [0.0; 2];
        let mut measured = [0.0; 2];
        frame(&mut ctx, [0.0, 0.0], false, |ui| {
            before = ui.ctx.current_window_mut().cursor.next_pos();
            measured = ui.ctx.font.measure("Hello");
            ui.label("Hello##id");
            after = ui.ctx.current_window_mut().cursor.next_pos();
        });
        let style = ctx.style;
        assert_eq!(after[0], before[0]);
        assert_eq!(after[1], before[1] + measured[1] + style.item_spacing[1]);

        let rect = [
            before[0],
            before[1],
            before[0] + measured[0],
            before[1] + measured[1],
        ];
        let glyphs = glyph_colors_in(&ctx, rect);
        assert!(!glyphs.is_empty(), "label must emit font glyph quads");
        assert!(glyphs.iter().all(|&c| c == ctx.style.text));
    }
}
