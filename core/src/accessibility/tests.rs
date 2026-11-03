use super::*;
use crate::ScrollFlags;
fn context() -> Context {
    let mut c = Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap();
    c.set_accessibility_enabled(true);
    c
}
fn frame(c: &mut Context, enabled: bool, value: &mut bool) -> Id {
    let mut id = Id(0);
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        let disabled = ui.begin_disabled("form", !enabled);
        id = ui.widget_id("Toggle##id");
        ui.checkbox("Toggle##id", value);
        ui.end_disabled(disabled);
    });
    c.render();
    id
}
#[test]
fn stock_toggle_exposes_role_state_name_and_ancestry() {
    let mut c = context();
    let mut v = true;
    let id = frame(&mut c, true, &mut v);
    let n = (0..c.accessibility_count())
        .filter_map(|i| c.accessibility_node(i))
        .find(|n| n.id == id)
        .unwrap();
    assert_eq!(n.role, AccessibilityRole::CheckBox);
    assert_ne!(n.states & AccessibilityState::CHECKED, 0);
    assert_ne!(n.parent, u32::MAX);
    let mut name = String::new();
    c.accessibility_name(id, &mut name);
    assert_eq!(name, "Toggle");
}
#[test]
fn child_scope_has_one_identity_and_retains_its_visible_bounds() {
    let mut c = context();
    let mut child_id = Id(0);
    let mut leaf_id = Id(0);
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        child_id = ui.widget_id("child");
        ui.child("child", [180., 100.], ScrollFlags::NONE, |ui| {
            leaf_id = ui.widget_id("inside");
            ui.button("inside");
        });
    });
    c.render();
    let nodes: Vec<_> = (0..c.accessibility_count())
        .filter_map(|i| c.accessibility_node(i))
        .collect();
    assert_eq!(nodes.iter().filter(|n| n.id == child_id).count(), 1);
    let (index, group) = nodes
        .iter()
        .enumerate()
        .find(|(_, n)| n.id == child_id)
        .unwrap();
    assert_eq!(group.role, AccessibilityRole::Group);
    assert!(group.rect[2] > group.rect[0] && group.rect[3] > group.rect[1]);
    assert_eq!(
        nodes.iter().find(|n| n.id == leaf_id).unwrap().parent,
        index as u32
    );
}
#[test]
fn activation_is_deferred_once_and_rejects_disabled_and_stale_owners() {
    let mut c = context();
    let mut v = false;
    let id = frame(&mut c, true, &mut v);
    assert!(c.submit_accessibility_action(AccessibilityAction {
        id,
        kind: AccessibilityActionKind::Activate,
        number: 0.
    }));
    assert!(c.submit_accessibility_action(AccessibilityAction {
        id,
        kind: AccessibilityActionKind::Activate,
        number: 0.
    }));
    assert!(!v);
    frame(&mut c, true, &mut v);
    assert!(v);
    frame(&mut c, true, &mut v);
    assert!(v);
    frame(&mut c, false, &mut v);
    assert!(!c.submit_accessibility_action(AccessibilityAction {
        id,
        kind: AccessibilityActionKind::Activate,
        number: 0.
    }));
    assert!(!c.submit_accessibility_action(AccessibilityAction {
        id: Id(7),
        kind: AccessibilityActionKind::Activate,
        number: 0.
    }));
}
#[test]
fn queued_activation_rechecks_new_disabled_state() {
    let mut c = context();
    let mut v = false;
    let id = frame(&mut c, true, &mut v);
    assert!(c.submit_accessibility_action(AccessibilityAction {
        id,
        kind: AccessibilityActionKind::Activate,
        number: 0.
    }));
    frame(&mut c, false, &mut v);
    assert!(!v);
}
#[test]
fn numeric_action_uses_current_range_and_changed_flag() {
    let mut c = context();
    let mut value = 5.;
    let mut id = Id(0);
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        id = ui.widget_id("value");
        ui.slider_float("value", &mut value, 0.0..=10.0);
    });
    c.render();
    assert!(c.submit_accessibility_action(AccessibilityAction {
        id,
        kind: AccessibilityActionKind::SetNumber,
        number: 99.
    }));
    c.new_frame(0.016);
    let mut changed = false;
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        changed = ui.slider_float("value", &mut value, 0.0..=7.0);
    });
    c.render();
    assert!(changed);
    assert_eq!(value, 7.);
}
#[test]
fn completely_clipped_action_is_rejected() {
    let mut c = context();
    let mut id = Id(0);
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.allocate([1., 500.]);
        id = ui.widget_id("hidden");
        ui.button("hidden");
    });
    c.render();
    assert!(!c.submit_accessibility_action(AccessibilityAction {
        id,
        kind: AccessibilityActionKind::Activate,
        number: 0.
    }));
}
#[test]
fn queued_activation_rechecks_new_clip_and_window_bounds_follow_drag() {
    let mut c = context();
    c.new_frame(0.016);
    let mut id = Id(0);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        id = ui.widget_id("action");
        ui.button("action");
    });
    c.render();
    assert!(c.submit_accessibility_action(AccessibilityAction {
        id,
        kind: AccessibilityActionKind::Activate,
        number: 0.,
    }));
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.allocate([1., 500.]);
        assert!(!ui.button("action"));
    });
    c.render();
    for mouse in [[70., 65.], [170., 165.]] {
        c.set_input(crate::input::InputState {
            mouse_pos: mouse,
            mouse_down: [true, false, false, false, false],
            viewport_size: [600., 400.],
            ..Default::default()
        });
        c.new_frame(0.016);
        c.ui().window("root", ScrollFlags::NONE, |_| {});
        c.render();
    }
    let bounds = c.accessibility_node(0).unwrap().rect;
    assert_eq!(bounds, c.windows[0].rect());
    assert_eq!(&bounds[..2], &[160., 160.]);
}
#[test]
fn collection_is_opt_in_and_string_limits_preserve_utf8() {
    let mut c = Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap();
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.button("button");
    });
    c.render();
    assert_eq!(c.accessibility_count(), 0);
    c.set_accessibility_enabled(true);
    let label = "é".repeat(200);
    let mut id = Id(0);
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        id = ui.widget_id(&label);
        ui.button(&label);
    });
    c.render();
    let mut name = String::new();
    c.accessibility_name(id, &mut name);
    assert_eq!(name.len(), 256);
}
#[test]
fn pending_action_wakes_an_idle_host() {
    let mut c = context();
    let mut v = false;
    let id = frame(&mut c, true, &mut v);
    assert_eq!(c.next_redraw_in(), None);
    assert!(c.submit_accessibility_action(AccessibilityAction {
        id,
        kind: AccessibilityActionKind::Activate,
        number: 0.
    }));
    assert_eq!(c.next_redraw_in(), Some(0.));
}
#[test]
fn blocked_background_cannot_accept_host_actions() {
    let mut c = context();
    let mut id = Id(0);
    let mut popup = Id(0);
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        id = ui.widget_id("behind");
        ui.button("behind");
        popup = ui.widget_id("modal");
        ui.open_popup(popup);
    });
    c.render();
    c.new_frame(0.016);
    c.ui().window("root", ScrollFlags::NONE, |ui| {
        ui.button("behind");
        ui.modal(popup, [80., 80., 260., 200.], |ui| {
            ui.button("top");
        });
    });
    c.render();
    assert!(!c.submit_accessibility_action(AccessibilityAction {
        id,
        kind: AccessibilityActionKind::Activate,
        number: 0.
    }));
}
