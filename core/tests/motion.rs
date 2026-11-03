use tacita::animation::{Easing, MotionSettings};
use tacita::context::Context;
use tacita::id::Id;
use tacita::input::{InputState, Key, KeyEvent};
use tacita::ui::Ui;
use tacita::ScrollFlags;

const FONT: &[u8] = include_bytes!("assets/DejaVuSans.ttf");
fn context() -> Context {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.set_motion_settings(MotionSettings {
        enabled: true,
        easing: Easing::Linear,
        ..MotionSettings::default()
    });
    ctx
}
fn frame(ctx: &mut Context, dt: f32, input: InputState<'_>, body: impl FnOnce(&mut Ui)) {
    ctx.set_input(input);
    ctx.new_frame(dt);
    ctx.ui().window("motion", ScrollFlags::NONE, body);
    ctx.render();
}
fn neutral() -> InputState<'static> {
    InputState {
        mouse_pos: [-1000.0; 2],
        viewport_size: [800.0, 600.0],
        ..InputState::default()
    }
}

#[test]
fn custom_channels_reverse_from_current_value_and_advance_once() {
    let mut ctx = context();
    let id = Id(7);
    frame(&mut ctx, 0.25, neutral(), |ui| {
        let first = ui.animate_float(id, 0.0, 1.0, 1.0, Easing::Linear);
        assert_eq!(first, 0.25);
        assert_eq!(ui.animate_float(id, 0.0, 1.0, 1.0, Easing::Linear), first);
        let color = ui.animate_color(id, [0; 4], [255; 4], 1.0, Easing::Linear);
        assert_eq!(color, [64; 4]);
        assert_eq!(
            ui.animate_color(id, [0; 4], [255; 4], 1.0, Easing::Linear),
            color
        );
    });
    frame(&mut ctx, 0.25, neutral(), |ui| {
        assert_eq!(ui.animate_float(id, 0.0, 0.0, 1.0, Easing::Linear), 0.1875);
        assert_eq!(
            ui.animate_color(id, [0; 4], [0; 4], 1.0, Easing::Linear),
            [48; 4]
        );
    });
    ctx.set_motion_settings(MotionSettings {
        enabled: true,
        reduced_motion: true,
        ..MotionSettings::default()
    });
    frame(&mut ctx, 0.01, neutral(), |ui| {
        assert_eq!(ui.animate_float(id, 0.0, 0.8, 1.0, Easing::Linear), 0.8);
        assert_eq!(ui.animate_spring(id, 0.0, 0.8, 1.0), 0.8);
        assert_eq!(
            ui.animate_color(id, [0; 4], [10; 4], 1.0, Easing::Linear),
            [10; 4]
        );
    });
}

#[test]
fn popup_exit_does_not_call_content_or_retain_input_capture() {
    let mut ctx = context();
    let popup = Id(55);
    let rect = [400.0, 80.0, 680.0, 180.0];
    frame(&mut ctx, 0.18, neutral(), |ui| {
        ui.open_modal(popup);
        ui.modal(popup, rect, |ui| {
            ui.label("Exit snapshot");
            let id = ui.widget_id("captured");
            ui.capture(id);
        });
    });
    assert!(ctx.wants_keyboard_capture());
    frame(&mut ctx, 0.02, neutral(), |ui| {
        ui.modal(popup, rect, |ui| {
            ui.close_popup();
        });
    });
    assert!(!ctx.wants_keyboard_capture());
    ctx.set_input(neutral());
    ctx.new_frame(0.02);
    let mut called = false;
    ctx.ui().window("motion", ScrollFlags::NONE, |ui| {
        ui.modal(popup, rect, |_| {
            called = true;
        });
    });
    let data = ctx.render();
    assert!(!called);
    assert!(data
        .vertices
        .iter()
        .any(|v| v.pos[0] >= rect[0] && v.col[3] > 0 && v.col[3] < 255));
    assert!(!ctx.wants_keyboard_capture());
    frame(&mut ctx, 0.02, neutral(), |ui| {
        ui.open_modal(popup);
        ui.modal(popup, rect, |ui| {
            ui.label("Reopened");
        });
    });
    frame(&mut ctx, 0.02, neutral(), |ui| {
        assert!(ui.is_popup_open(popup));
    });
    frame(
        &mut ctx,
        0.18,
        InputState {
            keys: &[KeyEvent::Press(Key::Escape)],
            ..neutral()
        },
        |ui| {
            assert!(!ui.is_popup_open(popup));
        },
    );
}

#[test]
fn collapsing_content_lays_out_during_exit_then_stops() {
    let mut ctx = context();
    let y = 60.0
        + ctx.font().line_height()
        + 2.0 * ctx.style.frame_padding[1]
        + ctx.style.window_padding[1]
        + 5.0;
    let mouse = [85.0, y];
    let mut calls = 0;
    {
        let mut declare = |ui: &mut Ui| {
            ui.collapsing_header("Reveal", |ui| {
                calls += 1;
                ui.button("Child");
            });
        };
        frame(&mut ctx, 0.02, neutral(), &mut declare);
        frame(
            &mut ctx,
            0.02,
            InputState {
                mouse_pos: mouse,
                mouse_down: [true, false, false, false, false],
                ..neutral()
            },
            &mut declare,
        );
        frame(
            &mut ctx,
            0.02,
            InputState {
                mouse_pos: mouse,
                ..neutral()
            },
            &mut declare,
        );
        frame(&mut ctx, 0.20, neutral(), &mut declare);
        frame(
            &mut ctx,
            0.02,
            InputState {
                mouse_pos: mouse,
                mouse_down: [true, false, false, false, false],
                ..neutral()
            },
            &mut declare,
        );
        frame(
            &mut ctx,
            0.02,
            InputState {
                mouse_pos: mouse,
                ..neutral()
            },
            &mut declare,
        );
    }
    let before = calls;
    frame(&mut ctx, 0.02, neutral(), |ui| {
        ui.collapsing_header("Reveal", |ui| {
            calls += 1;
            assert!(!ui.button("Child"));
        });
    });
    assert_eq!(calls, before + 1);
    frame(&mut ctx, 1.0, neutral(), |ui| {
        ui.collapsing_header("Reveal", |_| {
            panic!("settled closed content must not run");
        });
    });
}
