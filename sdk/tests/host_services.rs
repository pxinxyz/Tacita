use restricted_gui_sdk::{
    AccessibilityAction, AccessibilityActionKind, AccessibilityRole, AccessibilityState,
    CursorShape, Gui, HitTargetPolicy, ScrollFlags,
};
const FONT: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

#[test]
fn semantics_actions_and_host_requests_forward() {
    let mut gui = Gui::new(FONT).unwrap();
    gui.set_accessibility_enabled(true);
    gui.set_duplicate_id_diagnostics(true);
    gui.set_hit_target_policy(HitTargetPolicy { minimum: [32.; 2] });
    let mut checked = false;
    gui.new_frame(Default::default(), 0.016);
    gui.begin_window("root", ScrollFlags::NONE);
    let id = gui.widget_id("toggle##id");
    gui.checkbox("toggle##id", &mut checked);
    gui.end_window();
    gui.render();
    let node = (0..gui.accessibility_count())
        .filter_map(|i| gui.accessibility_node(i))
        .find(|n| n.id == id)
        .unwrap();
    assert_eq!(node.role, AccessibilityRole::CheckBox);
    let mut name = String::new();
    gui.accessibility_name(id, &mut name);
    assert_eq!(name, "toggle");
    assert!(gui.submit_accessibility_action(AccessibilityAction {
        id,
        kind: AccessibilityActionKind::Activate,
        number: 0.
    }));
    gui.new_frame(Default::default(), 0.016);
    gui.begin_window("root", ScrollFlags::NONE);
    assert!(gui.checkbox("toggle##id", &mut checked));
    gui.end_window();
    gui.render();
    assert!(checked);
    let node = (0..gui.accessibility_count())
        .filter_map(|i| gui.accessibility_node(i))
        .find(|n| n.id == id)
        .unwrap();
    assert_ne!(node.states & AccessibilityState::CHECKED, 0);
    assert_eq!(gui.hit_target_policy().minimum, [32.; 2]);
    gui.request_redraw_in(0.3);
    gui.request_redraw_in(0.1);
    gui.invalidate();
    assert_eq!(gui.next_redraw_in(), Some(0.));
    assert_eq!(gui.cursor_shape(), CursorShape::Default);
}

#[test]
fn closed_scope_suppresses_semantic_metadata_and_cursor_requests() {
    let mut gui = Gui::new(FONT).unwrap();
    gui.set_accessibility_enabled(true);
    gui.new_frame(Default::default(), 0.016);
    gui.begin_window("root", ScrollFlags::NONE);
    let id = gui.widget_id("button");
    gui.button("button");
    gui.end_window();
    gui.render();
    gui.set_window_open("root", false);
    gui.new_frame(Default::default(), 0.016);
    gui.begin_window("root", ScrollFlags::NONE);
    gui.set_accessibility_name(id, "hidden name");
    gui.accessibility_info(id, AccessibilityRole::Button, "hidden value", 0, 3);
    gui.request_cursor(id, [0., 0., 100., 100.], CursorShape::Pointer);
    gui.end_window();
    gui.render();
    assert_eq!(gui.accessibility_count(), 0);
    let mut name = String::new();
    gui.accessibility_name(id, &mut name);
    assert_eq!(name, "button");
    assert_eq!(gui.cursor_shape(), CursorShape::Default);
}
