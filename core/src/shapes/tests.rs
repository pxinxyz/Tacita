use super::*;
use crate::{
    context::Context,
    draw::{DrawList, TextureId},
    input::InputState,
    ScrollFlags,
};
const FONT: &[u8] = include_bytes!("../../tests/assets/DejaVuSans.ttf");
fn shape() -> RoundedRect {
    RoundedRect::new([100.0, 100.0, 200.0, 160.0], [24.0, 0.0, 12.0, 8.0]).unwrap()
}

#[test]
fn thin_border_retains_coverage_and_placement_bounds() {
    let mut draw = DrawList::new();
    mesh::border(
        &mut draw,
        shape(),
        2.0,
        true,
        [255; 4],
        0.125,
        BorderPlacement::Outer,
    );
    let data = draw.as_draw_data();
    assert!(!data.indices.is_empty());
    assert!(data.vertices.iter().all(|v| v.col[3] <= 64));
    assert!(data.vertices.iter().any(|v| v.col[3] == 64));
    assert!(data
        .vertices
        .iter()
        .all(|v| v.pos[0] >= 99.625 - 0.001 && v.pos[0] <= 200.375 + 0.001));
    assert!(data.vertices.iter().any(|v| v.pos[0] > 200.0));
    let mut inner = DrawList::new();
    mesh::border(
        &mut inner,
        shape(),
        1.0,
        false,
        [255; 4],
        2.0,
        BorderPlacement::Inner,
    );
    assert!(inner
        .as_draw_data()
        .vertices
        .iter()
        .all(|v| shape().contains(v.pos) || shape().distance(v.pos).abs() < 0.001));
}
#[test]
fn rounded_tail_preserves_texture_order_and_interpolates_images() {
    let mut draw = DrawList::new();
    draw.add_filled_rect([0.0; 2], [5.0; 2], [255; 4]);
    let mark = draw.paint_mark();
    draw.add_textured_quad(
        TextureId(42),
        [80.0, 80.0],
        [220.0, 180.0],
        [0.0; 2],
        [1.0; 2],
        [200; 4],
    );
    draw.clip_rounded_tail(mark, shape(), 1.5);
    let data = draw.as_draw_data();
    assert_eq!(data.commands[0].texture, TextureId::WHITE);
    assert_eq!(data.commands[1].texture, TextureId(42));
    for &index in &data.indices[mark[0]..] {
        let v = data.vertices[index as usize];
        assert!(shape().distance(v.pos) <= 0.001);
        assert!((v.uv[0] - (v.pos[0] - 80.0) / 140.0).abs() < 0.0001);
        assert!((v.uv[1] - (v.pos[1] - 80.0) / 100.0).abs() < 0.0001);
    }
    assert_eq!(
        data.commands
            .iter()
            .map(|c| c.elem_count as usize)
            .sum::<usize>(),
        data.indices.len()
    );
}
#[test]
fn nested_scopes_restore_geometry_layout_and_input() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.set_window_rect("W", [20.0, 20.0], [400.0, 300.0]);
    ctx.new_frame(0.016);
    ctx.ui().window("W", ScrollFlags::NONE, |ui| {
        let before = ui.allocate([10.0, 10.0]);
        let outer = ui.begin_rounded_clip("outer", shape()).unwrap();
        let inner_shape = RoundedRect::new([120.0, 112.0, 180.0, 148.0], [16.0; 4]).unwrap();
        ui.rounded_clip("inner", inner_shape, |ui| {
            ui.draw_rect_filled([0.0; 2], [300.0; 2], [255; 4])
        })
        .unwrap();
        ui.end_rounded_clip(outer);
        let after = ui.allocate([10.0, 10.0]);
        assert_eq!(before[0], after[0]);
        assert!(after[1] > before[1]);
    });
    assert!(ctx.rounded_clips.is_empty());
    ctx.render();
}
#[test]
fn clipped_corner_cannot_press_or_complete_a_button_click() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.set_window_rect("W", [20.0, 20.0], [400.0, 300.0]);
    let s = RoundedRect::new([100.0, 100.0, 200.0, 160.0], [30.0; 4]).unwrap();
    let run = |ctx: &mut Context, p, down| {
        ctx.set_input(InputState {
            mouse_pos: p,
            mouse_down: [down, false, false, false, false],
            ..Default::default()
        });
        ctx.new_frame(0.016);
        let mut clicked = false;
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            ui.rounded_clip("clip", s, |ui| {
                clicked = ui.button("A fairly wide button");
            })
            .unwrap();
        });
        ctx.render();
        clicked
    };
    run(&mut ctx, [105.0, 105.0], false);
    run(&mut ctx, [105.0, 105.0], true);
    assert!(ctx.captured_widget_id().is_none());
    assert!(!run(&mut ctx, [105.0, 105.0], false));
    run(&mut ctx, [130.0, 110.0], true);
    assert!(ctx.captured_widget_id().is_some());
    assert!(!run(&mut ctx, [105.0, 105.0], false));
    assert!(ctx.captured_widget_id().is_none());
}
#[test]
fn shadow_falloff_is_translucent_and_inset_stays_inside() {
    let mut draw = DrawList::new();
    mesh::shadow(
        &mut draw,
        shape(),
        1.0,
        Shadow {
            offset: [2.0, 3.0],
            spread: 1.0,
            blur: 8.0,
            color: [10, 20, 30, 160],
            inset: true,
        },
    );
    let d = draw.as_draw_data();
    assert!(d.vertices.iter().any(|v| v.col[3] > 0 && v.col[3] < 160));
    for &i in d.indices {
        assert!(shape().distance(d.vertices[i as usize].pos) <= 0.001);
    }
    assert!(
        d.indices.iter().any(|&i| {
            let v = d.vertices[i as usize];
            (v.pos[0] - 100.0).abs() < 0.001
                && (124.0..=152.0).contains(&v.pos[1])
                && v.col[3] == 160
        }),
        "the exposed inset edge must be shaded after translation"
    );
}

#[test]
fn rounded_focus_outlines_and_traversal_follow_the_visible_silhouette() {
    for enabled in [false, true] {
        let mut ctx = Context::new(FONT).unwrap();
        ctx.set_motion_settings(crate::animation::MotionSettings {
            enabled,
            ..Default::default()
        });
        ctx.set_window_rect("W", [20.0, 20.0], [400.0, 300.0]);
        let s = RoundedRect::new([100.0, 100.0, 200.0, 160.0], [30.0; 4]).unwrap();
        ctx.new_frame(0.016);
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            ui.rounded_clip("clip", s, |ui| {
                let hidden = ui.widget_id("Hidden");
                ui.focusable(hidden, [100.0, 100.0, 104.0, 104.0]);
                let visible = ui.widget_id("Visible");
                ui.focus(visible);
                ui.focusable(visible, s.rect);
            })
            .unwrap();
        });
        assert_eq!(ctx.focus_order.len(), 1);
        ctx.render();
        let overlay = ctx.overlay.as_draw_data();
        assert!(!overlay.indices.is_empty());
        for &index in overlay.indices {
            assert!(s.distance(overlay.vertices[index as usize].pos) <= 0.001);
        }
    }
}

#[test]
fn labeled_rounded_scopes_disambiguate_child_widget_ids() {
    let mut ctx = Context::new(FONT).unwrap();
    let mut ids = [crate::id::Id(0); 2];
    ctx.new_frame(0.016);
    ctx.ui().window("W", ScrollFlags::NONE, |ui| {
        for (index, label) in ["Left", "Right"].into_iter().enumerate() {
            ui.rounded_clip(label, shape(), |ui| ids[index] = ui.widget_id("Shared"))
                .unwrap();
        }
    });
    ctx.render();
    assert_ne!(ids[0], ids[1]);
}
