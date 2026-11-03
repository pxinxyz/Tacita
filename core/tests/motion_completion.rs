use tacita::animation::{
    Easing, GradientOptions, GradientStop, Keyframe, MotionSettings, PlaybackMode, TimelineOptions,
};
use tacita::context::Context;
use tacita::id::Id;
use tacita::input::InputState;
use tacita::ScrollFlags;

fn context() -> Context {
    let mut ctx = Context::new(include_bytes!("assets/DejaVuSans.ttf")).unwrap();
    ctx.set_motion_settings(MotionSettings {
        enabled: true,
        ..MotionSettings::default()
    });
    ctx
}
#[test]
fn animated_region_routes_actual_button_input_and_restores_id_scope() {
    let mut ctx = context();
    let mut clicks = 0;
    let mut outer = Id(0);
    for frame in 0..3 {
        ctx.set_input(InputState {
            mouse_pos: if frame == 0 {
                [-1000.0; 2]
            } else {
                [130.0, 110.0]
            },
            mouse_down: [frame == 1, false, false, false, false],
            viewport_size: [800.0, 600.0],
            ..InputState::default()
        });
        ctx.new_frame(if frame == 1 { 0.5 } else { 0.0 });
        ctx.ui().window("Layout", ScrollFlags::NONE, |ui| {
            outer = ui.widget_id("Outer");
            let target = if frame == 0 {
                [80.0, 100.0, 220.0, 200.0]
            } else {
                [160.0, 100.0, 300.0, 200.0]
            };
            ui.animated_layout("Moving", target, 1.0, Easing::Linear, |ui, rect| {
                assert_eq!(rect[0], if frame == 0 { 80.0 } else { 120.0 });
                if ui.button("Hit") {
                    clicks += 1;
                }
            });
            assert_eq!(ui.widget_id("Outer"), outer);
        });
        ctx.render();
    }
    assert_eq!(clicks, 1);
}
#[test]
fn cached_timeline_advances_once_pauses_seeks_and_restarts() {
    let mut ctx = context();
    let keys = [
        Keyframe {
            time: 0.0,
            value: 0.0,
            easing: Easing::Linear,
        },
        Keyframe {
            time: 1.0,
            value: 1.0,
            easing: Easing::Linear,
        },
    ];
    let options = TimelineOptions {
        mode: PlaybackMode::Loop,
        ..TimelineOptions::default()
    };
    ctx.new_frame(0.25);
    assert_eq!(ctx.animate_timeline(Id(1), &keys, options), 0.25);
    assert_eq!(ctx.animate_timeline(Id(1), &keys, options), 0.25);
    ctx.seek_timeline(Id(1), 0.75);
    assert_eq!(ctx.animate_timeline(Id(1), &keys, options), 0.75);
    ctx.render();
    ctx.new_frame(0.5);
    assert_eq!(
        ctx.animate_timeline(
            Id(1),
            &keys,
            TimelineOptions {
                playing: false,
                ..options
            }
        ),
        0.75
    );
    ctx.render();
    ctx.new_frame(0.1);
    assert_eq!(
        ctx.animate_timeline(
            Id(1),
            &keys,
            TimelineOptions {
                revision: 1,
                ..options
            }
        ),
        0.1
    );
    ctx.render();
}
#[test]
fn gradient_clock_freezes_under_reduced_motion() {
    let mut ctx = context();
    let mut expected = Vec::new();
    let stops = [
        GradientStop {
            position: 0.0,
            color: [0; 4],
        },
        GradientStop {
            position: 1.0,
            color: [255; 4],
        },
    ];
    for frame in 0..3 {
        ctx.set_motion_settings(MotionSettings {
            enabled: true,
            reduced_motion: frame == 2,
            ..MotionSettings::default()
        });
        ctx.new_frame(0.02);
        ctx.ui().window("Gradient", ScrollFlags::NONE, |ui| {
            ui.draw_animated_gradient(
                [100.0, 100.0, 200.0, 120.0],
                &stops,
                GradientOptions::default(),
                if frame == 0 { 0.0 } else { 0.5 },
                2.0,
            );
        });
        let data = ctx.render();
        let colors: Vec<_> = data
            .vertices
            .iter()
            .filter(|v| v.pos[1] == 100.0 || v.pos[1] == 120.0)
            .map(|v| v.col)
            .collect();
        if frame == 0 {
            expected = colors;
        } else if frame == 1 {
            assert_ne!(colors, expected);
        } else {
            assert_eq!(colors, expected);
        }
    }
}
