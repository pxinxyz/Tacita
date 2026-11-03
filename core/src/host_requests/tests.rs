use super::*;
use crate::{input::InputState, ScrollFlags};
fn context() -> Context {
    Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap()
}
#[test]
fn idle_frame_can_sleep_and_external_deadlines_take_the_minimum() {
    let mut c = context();
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.button("button");
    });
    c.render();
    assert_eq!(c.next_redraw_in(), None);
    c.request_redraw_in(0.5);
    c.request_redraw_in(0.2);
    assert_eq!(c.next_redraw_in(), Some(0.2));
    c.invalidate();
    assert_eq!(c.next_redraw_in(), Some(0.));
}
#[test]
fn minimum_targets_reserve_space_without_enlarging_visual_controls() {
    let mut c = context();
    c.set_hit_target_policy(HitTargetPolicy { minimum: [40.; 2] });
    let mut r = [0.; 4];
    let mut hit = [0.; 4];
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.button("x");
        r = ui.ctx.last_item.unwrap().1;
        hit = ui.hit_target_rect(r);
    });
    c.render();
    assert!(r[2] - r[0] < 40.);
    assert_eq!(hit[2] - hit[0], 40.);
    assert_eq!(hit[3] - hit[1], 40.);
}
#[test]
fn padded_click_toggles_without_changing_pill_paint_size() {
    let mut c = context();
    c.set_hit_target_policy(HitTargetPolicy { minimum: [40.; 2] });
    let mut v = false;
    let mut p = [0.; 2];
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.pill_toggle("##pill", &mut v);
        let r = ui.ctx.last_item.unwrap().1;
        p = [r[0] - 2., (r[1] + r[3]) * 0.5];
    });
    c.render();
    for down in [true, false] {
        c.set_input(InputState {
            mouse_pos: p,
            mouse_down: [down, false, false, false, false],
            ..InputState::default()
        });
        c.new_frame(0.016);
        c.ui().window("root", ScrollFlags::NONE, |ui| {
            ui.pill_toggle("##pill", &mut v);
        });
        c.render();
    }
    assert!(v);
}
#[test]
fn cursor_follows_hover_and_capture_and_resets_next_frame() {
    let mut c = context();
    let mut r = [0.; 4];
    let mut v = 5.;
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.slider_float("value", &mut v, 0.0..=10.0);
        r = ui.ctx.last_item.unwrap().1;
    });
    c.render();
    c.set_input(InputState {
        mouse_pos: [r[0] + 10., r[1] + 4.],
        ..InputState::default()
    });
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.slider_float("value", &mut v, 0.0..=10.0);
    });
    c.render();
    assert_eq!(c.cursor_shape(), CursorShape::ResizeHorizontal);
    c.set_input(InputState::default());
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.slider_float("value", &mut v, 0.0..=10.0);
    });
    c.render();
    assert_eq!(c.cursor_shape(), CursorShape::Default);
}
#[test]
fn invalid_minimum_is_rejected() {
    assert!(!HitTargetPolicy {
        minimum: [f32::NAN, 32.]
    }
    .is_valid());
    assert!(!HitTargetPolicy {
        minimum: [-1., 32.]
    }
    .is_valid());
    assert!(!HitTargetPolicy {
        minimum: [257., 32.]
    }
    .is_valid());
}
#[test]
fn captured_slider_keeps_cursor_outside_track_then_releases_it() {
    let mut c = context();
    let mut value = 5.;
    let mut rect = [0.; 4];
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.slider_float("value", &mut value, 0.0..=10.0);
        rect = ui.ctx.last_item.unwrap().1;
    });
    c.render();
    for position in [[rect[0] + 4., rect[1] + 4.], [-40., -40.]] {
        c.set_input(InputState {
            mouse_pos: position,
            mouse_down: [true, false, false, false, false],
            ..Default::default()
        });
        c.new_frame(0.016);
        c.ui().window("root", ScrollFlags::NONE, |ui| {
            ui.slider_float("value", &mut value, 0.0..=10.0);
        });
        c.render();
        assert_eq!(c.cursor_shape(), CursorShape::ResizeHorizontal);
    }
    c.set_input(InputState {
        mouse_pos: [-40., -40.],
        ..Default::default()
    });
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.slider_float("value", &mut value, 0.0..=10.0);
    });
    c.render();
    assert_eq!(c.cursor_shape(), CursorShape::Default);
}
#[test]
fn finished_tween_stops_waking() {
    let mut c = context();
    let mut m = c.motion_settings();
    m.enabled = true;
    c.set_motion_settings(m);
    c.new_frame(0.);
    c.animate_float(Id(42), 0., 1., 0.05, crate::animation::Easing::Linear);
    c.render();
    assert!(c.next_redraw_in().is_some());
    c.new_frame(1.);
    c.animate_float(Id(42), 0., 1., 0.05, crate::animation::Easing::Linear);
    c.render();
    assert_eq!(c.next_redraw_in(), None);
}
