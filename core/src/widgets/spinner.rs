use crate::id::split_label;
use crate::ui::Ui;

const TAU: f32 = core::f32::consts::PI * 2.0;
const SWEEP: f32 = core::f32::consts::PI * 1.35;

pub(crate) fn spinner(ui: &mut Ui, label: &str, diameter: f32, phase: f32) {
    let ctx = &mut *ui.ctx;
    let style = ctx.style;
    let face = ctx.active_font();
    let display = split_label(label).0;
    let label_size = ctx.font.measure_in(face, display);
    let diameter = if diameter.is_finite() {
        diameter.clamp(0.0, 512.0)
    } else {
        0.0
    };
    let height = diameter.max(ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1]);
    let width = diameter
        + if display.is_empty() {
            0.0
        } else {
            style.item_spacing[0].max(0.0) + label_size[0]
        };
    let rect = super::item_rect(ctx, [width, height]);
    let (font, draw_list) = super::draw_parts(ctx);
    if diameter >= 6.0 {
        let thickness = (diameter * 0.11).clamp(1.5, 4.0);
        let radius = diameter * 0.5 - thickness * 0.5 - 1.5;
        let center = [rect[0] + diameter * 0.5, rect[1] + height * 0.5];
        let mut track = style.widget_bg_hovered;
        track[3] = track[3].min(80);
        draw_list.add_circle_outline(center, radius, track, thickness, style.anti_aliasing);
        let phase = if phase.is_finite() { phase % TAU } else { 0.0 };
        draw_list.add_arc(
            center,
            radius,
            phase,
            phase + SWEEP,
            style.widget_bg_active,
            thickness,
            style.anti_aliasing,
        );
    }
    if !display.is_empty() {
        draw_list.add_text(
            font,
            face,
            [
                rect[0] + diameter + style.item_spacing[0].max(0.0),
                rect[1] + (height - label_size[1]) * 0.5,
            ],
            display,
            style.text,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::widgets::test_support::{frame, test_context};

    #[test]
    fn spinner_is_inert_and_phase_changes_arc_geometry() {
        let mut ctx = test_context();
        frame(&mut ctx, [-50.0, -50.0], false, |ui| {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let row_height = 24.0_f32.max(
                ui.ctx.font.line_height_in(ui.ctx.active_font())
                    + 2.0 * ui.ctx.style.frame_padding[1],
            );
            let before = ui
                .ctx
                .current_window_mut()
                .draw_list
                .as_draw_data()
                .vertices
                .len();
            ui.spinner("##spin", 24.0, 0.0);
            let vertices = ui
                .ctx
                .current_window_mut()
                .draw_list
                .as_draw_data()
                .vertices;
            assert!(vertices[before..].iter().all(|v| {
                v.pos[0] >= pos[0]
                    && v.pos[0] <= pos[0] + 24.0
                    && v.pos[1] >= pos[1]
                    && v.pos[1] <= pos[1] + row_height
            }));
        });
        assert_eq!(ctx.hot, None);
        assert_eq!(ctx.active, None);
        let first = ctx.windows[0].draw_list.as_draw_data().vertices.to_vec();
        frame(&mut ctx, [-50.0, -50.0], false, |ui| {
            ui.spinner("##spin", 24.0, 1.0);
        });
        let second = ctx.windows[0].draw_list.as_draw_data().vertices;
        assert_eq!(first.len(), second.len());
        assert!(first.iter().zip(second).any(|(a, b)| a.pos != b.pos));
        assert!(second
            .iter()
            .all(|v| v.pos[0].is_finite() && v.pos[1].is_finite()));
    }
}
