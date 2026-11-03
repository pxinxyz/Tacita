use super::*;
use crate::{
    animation::GradientStop,
    context::Context,
    input::{InputState, Key, KeyEvent},
    ScrollFlags,
};
const FONT: &[u8] = include_bytes!("../../tests/assets/DejaVuSans.ttf");

#[test]
fn curve_evaluation_ordering_and_failed_mutations_are_atomic() {
    let mut curve = CurveEditor::default();
    assert_eq!(curve.evaluate(0.5).unwrap(), 0.5);
    assert_eq!(curve.evaluate(-1.).unwrap(), 0.);
    assert_eq!(curve.evaluate(2.).unwrap(), 1.);
    let mut key = curve.keys()[0];
    key.interpolation = CurveInterpolation::Step;
    curve.set_key(0, key).unwrap();
    assert_eq!(curve.evaluate(0.5).unwrap(), 0.);
    key.interpolation = CurveInterpolation::Cubic;
    key.out_slope = 2.;
    curve.set_key(0, key).unwrap();
    assert_eq!(curve.evaluate(0.5).unwrap(), 0.75);
    let saved = curve.keys().to_vec();
    assert!(curve.insert(CurveKey::linear(0., 0.5)).is_err());
    assert!(curve
        .replace(&[CurveKey::linear(0.8, 0.), CurveKey::linear(0.2, 1.)])
        .is_err());
    assert!(curve.set_key(1, CurveKey::linear(f32::NAN, 0.)).is_err());
    assert!(curve.remove(0).is_err());
    assert_eq!(curve.keys(), saved);
    let index = curve.insert(CurveKey::linear(0.25, 0.4)).unwrap();
    assert_eq!(index, 1);
    assert_eq!(curve.selected(), Some(1));
    curve.remove(1).unwrap();
    assert_eq!(curve.keys(), saved);
    assert!(curve.evaluate(f32::INFINITY).is_err());
}
#[test]
fn gradient_insertion_samples_color_and_preserves_endpoints() {
    let mut gradient = GradientEditor::new(&[
        GradientStop {
            position: 0.,
            color: [0, 0, 0, 0],
        },
        GradientStop {
            position: 1.,
            color: [200, 100, 50, 200],
        },
    ])
    .unwrap();
    let expected = gradient.sample(0.5).unwrap();
    let i = gradient.insert(0.5).unwrap();
    assert_eq!(gradient.stops()[i].color, expected);
    let saved = gradient.stops().to_vec();
    assert!(gradient.insert(0.5).is_err());
    assert!(gradient.remove(usize::MAX).is_err());
    assert!(gradient.remove(0).is_err());
    assert!(gradient.remove(2).is_err());
    assert!(gradient
        .set_stop(
            1,
            GradientStop {
                position: 1.,
                color: [0; 4]
            }
        )
        .is_err());
    assert!(gradient
        .replace(&[
            GradientStop {
                position: 0.1,
                color: [0; 4]
            },
            GradientStop {
                position: 1.,
                color: [0; 4]
            }
        ])
        .is_err());
    assert_eq!(gradient.stops(), saved);
    gradient.remove(1).unwrap();
    assert_eq!(gradient.stops().len(), 2);
}
#[test]
fn editor_capacity_and_neighbor_separation_are_bounded() {
    let keys: alloc::vec::Vec<_> = (0..MAX_CURVE_KEYS)
        .map(|i| CurveKey::linear(i as f32 / (MAX_CURVE_KEYS - 1) as f32, 0.5))
        .collect();
    let mut curve = CurveEditor::new(&keys).unwrap();
    assert!(curve.insert(CurveKey::linear(0.01, 0.5)).is_err());
    let stops: alloc::vec::Vec<_> = (0..MAX_EDITOR_STOPS)
        .map(|i| GradientStop {
            position: i as f32 / (MAX_EDITOR_STOPS - 1) as f32,
            color: [0; 4],
        })
        .collect();
    let mut gradient = GradientEditor::new(&stops).unwrap();
    assert!(gradient.insert(0.01).is_err());
    assert!(curve.select(Some(usize::MAX)).is_err());
    assert!(gradient.select(Some(usize::MAX)).is_err());
}
#[test]
fn hidden_plot_fetches_no_samples_and_large_peak_keeps_bounded_geometry() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.set_window_rect("W", [0., 0.], [700., 500.]);
    ctx.set_input(InputState::default());
    ctx.new_frame(0.016);
    let mut rect = [0.; 4];
    let color = [12, 34, 56, 255];
    ctx.ui().window("W", ScrollFlags::BAR, |ui| {
        ui.plot_with(
            "##peak",
            100_000,
            |i| if i == 50_000 { 1000. } else { 0. },
            PlotOptions {
                size: [400., 160.],
                color,
                axes: false,
                ..PlotOptions::default()
            },
        )
        .unwrap();
        rect = ui.ctx.last_item.unwrap().1;
        ui.allocate([100., 700.]);
        ui.plot_with(
            "##hidden",
            1_000_000,
            |_| panic!("hidden plot fetched sample"),
            PlotOptions::default(),
        )
        .unwrap();
    });
    let draw = ctx.render();
    assert!(draw.vertices.len() < 20_000);
    assert!(draw
        .vertices
        .iter()
        .any(|v| v.col == color && v.pos[1] <= rect[1] + 9.));
}
#[test]
fn plot_rejects_range_before_layout_and_accepts_empty_gap_or_extreme_series() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.set_input(InputState::default());
    ctx.new_frame(0.016);
    ctx.ui().window("W", ScrollFlags::NONE, |ui| {
        let before = ui.ctx.current_window_mut().cursor.position();
        assert!(ui
            .plot(
                "P",
                &[0.],
                PlotOptions {
                    range: Some([1., 0.]),
                    ..PlotOptions::default()
                }
            )
            .is_err());
        assert_eq!(ui.ctx.current_window_mut().cursor.position(), before);
        for samples in [&[][..], &[f32::NAN][..], &[f32::MAX][..]] {
            ui.plot(
                "##edge",
                samples,
                PlotOptions {
                    size: [100., 50.],
                    axes: false,
                    ..PlotOptions::default()
                },
            )
            .unwrap();
        }
    });
    assert!(ctx
        .render()
        .vertices
        .iter()
        .all(|v| v.pos.iter().all(|p| p.is_finite())));
}
#[test]
fn curve_drag_clamps_outside_and_escape_restores_original() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.set_window_rect("W", [0., 0.], [600., 450.]);
    let mut curve = CurveEditor::new(&[
        CurveKey::linear(0., 0.),
        CurveKey::linear(0.5, 0.5),
        CurveKey::linear(1., 1.),
    ])
    .unwrap();
    let frame = |ctx: &mut Context,
                 curve: &mut CurveEditor,
                 mouse: [f32; 2],
                 down: bool,
                 escape: bool,
                 disabled: bool| {
        let escape_keys = [KeyEvent::Press(Key::Escape)];
        let input = InputState {
            keys: if escape { &escape_keys } else { &[] },
            mouse_pos: mouse,
            mouse_down: [down, false, false, false, false],
            ..InputState::default()
        };
        ctx.set_input(input);
        ctx.new_frame(0.016);
        let mut origin = [0.; 2];
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            origin = ui.ctx.current_window_mut().cursor.position();
            ui.disabled("disabled", disabled, |ui| {
                ui.curve_editor("##curve", curve, [400., 180.]).unwrap();
            });
        });
        ctx.render();
        origin
    };
    let origin = frame(&mut ctx, &mut curve, [0., 0.], false, false, false);
    let point = [origin[0] + 42. + 175., origin[1] + 8. + 74.];
    frame(&mut ctx, &mut curve, point, true, false, false);
    assert!(ctx.active.is_some());
    frame(
        &mut ctx,
        &mut curve,
        [origin[0] + 700., origin[1] - 100.],
        true,
        false,
        false,
    );
    assert!(curve.keys()[1].time > 0.5);
    assert_eq!(curve.keys()[1].value, 1.);
    frame(
        &mut ctx,
        &mut curve,
        [origin[0] + 700., origin[1] - 100.],
        true,
        true,
        false,
    );
    frame(&mut ctx, &mut curve, point, false, false, false);
    assert!(ctx.active.is_none());
    assert_eq!(curve.keys()[1], CurveKey::linear(0.5, 0.5));
    frame(&mut ctx, &mut curve, point, true, false, true);
    assert!(ctx.active.is_none());
}

#[test]
fn gradient_marker_lower_edge_captures_and_drags() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.set_window_rect("W", [0., 0.], [600., 450.]);
    let mut gradient = GradientEditor::default();
    gradient.insert(0.5).unwrap();
    let frame = |ctx: &mut Context, model: &mut GradientEditor, mouse, down| {
        ctx.set_input(InputState {
            mouse_pos: mouse,
            mouse_down: [down, false, false, false, false],
            ..InputState::default()
        });
        ctx.new_frame(0.016);
        let mut origin = [0.; 2];
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            origin = ui.ctx.current_window_mut().cursor.position();
            ui.gradient_editor("##gradient", model, [400., 80.])
                .unwrap();
        });
        ctx.render();
        origin
    };
    let origin = frame(&mut ctx, &mut gradient, [0., 0.], false);
    let lower_edge = [origin[0] + 200., origin[1] + 72.];
    frame(&mut ctx, &mut gradient, lower_edge, true);
    assert!(ctx.active.is_some(), "the painted marker edge must capture");
    let moved = [lower_edge[0] + 80., lower_edge[1]];
    frame(&mut ctx, &mut gradient, moved, true);
    assert!(gradient.stops()[1].position > 0.7);
    frame(&mut ctx, &mut gradient, moved, false);
    assert!(ctx.active.is_none());
    assert_eq!(gradient.stops()[0].position, 0.);
    assert_eq!(gradient.stops()[2].position, 1.);
}

#[test]
fn gradient_drag_clamps_and_escape_restores_stop_without_moving_endpoints() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.set_window_rect("W", [0., 0.], [600., 450.]);
    let mut gradient = GradientEditor::default();
    gradient.insert(0.5).unwrap();
    let original = gradient.stops().to_vec();
    let frame = |ctx: &mut Context,
                 model: &mut GradientEditor,
                 mouse: [f32; 2],
                 down: bool,
                 escape: bool| {
        let escape_keys = [KeyEvent::Press(Key::Escape)];
        let input = InputState {
            keys: if escape { &escape_keys } else { &[] },
            mouse_pos: mouse,
            mouse_down: [down, false, false, false, false],
            ..InputState::default()
        };
        ctx.set_input(input);
        ctx.new_frame(0.016);
        let mut origin = [0.; 2];
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            origin = ui.ctx.current_window_mut().cursor.position();
            ui.gradient_editor("##gradient", model, [400., 80.])
                .unwrap();
        });
        ctx.render();
        origin
    };
    let origin = frame(&mut ctx, &mut gradient, [0., 0.], false, false);
    let point = [origin[0] + 200., origin[1] + 67.];
    frame(&mut ctx, &mut gradient, point, true, false);
    assert!(ctx.active.is_some());
    frame(
        &mut ctx,
        &mut gradient,
        [origin[0] + 700., origin[1] + 67.],
        true,
        false,
    );
    assert!(gradient.stops()[1].position < 1.);
    assert!(gradient.stops()[1].position > 0.5);
    frame(
        &mut ctx,
        &mut gradient,
        [origin[0] + 700., origin[1] + 67.],
        true,
        true,
    );
    frame(&mut ctx, &mut gradient, point, false, false);
    assert_eq!(gradient.stops(), original);
    assert!(ctx.active.is_none());
}
