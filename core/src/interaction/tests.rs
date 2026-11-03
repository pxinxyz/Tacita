use super::*;
use crate::input::InputState;
use crate::widgets::test_support::{frame, frame_with_input, test_context};
#[test]
fn nested_stack_restores_parent_and_escapes_clip() {
    let mut c = test_context();
    let a = Id(11);
    let b = Id(12);
    frame(&mut c, [0.; 2], false, |ui| {
        ui.open_popup(a);
        ui.popup(a, [80., 80., 180., 180.], |ui| {
            ui.open_modal(b);
            ui.modal(b, [300., 200., 500., 400.], |ui| ui.label("child"));
            ui.label("parent after child");
        });
    });
    assert_eq!(c.popup_depth(), 2);
    assert_eq!(c.modal_depth(), 1);
    assert!(c.routing.popups[1]
        .draw
        .as_draw_data()
        .commands
        .iter()
        .any(|cmd| cmd.clip_rect[0] >= 300. && cmd.clip_rect[2] > 180.));
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Press(Key::Escape)],
            ..Default::default()
        },
        |ui| {
            assert!(ui.is_popup_open(a));
            assert!(!ui.is_popup_open(b));
            ui.popup(a, [80., 80., 180., 180.], |ui| ui.label("parent"));
        },
    );
    assert_eq!(c.popup_depth(), 1);
}
#[test]
fn gamepad_focus_activation_and_shortcut_consumption() {
    let mut c = test_context();
    let mut first = Id(0);
    let mut second = Id(0);
    frame(&mut c, [0.; 2], false, |ui| {
        first = ui.widget_id("one");
        ui.button("one");
        second = ui.widget_id("two");
        ui.button("two");
        ui.shortcut(
            Id(20),
            Shortcut {
                key: Key::K,
                modifiers: Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                scope: ShortcutScope::Global,
            },
        );
    });
    c.navigate(Navigation::Next);
    c.navigate(Navigation::Down);
    c.navigate(Navigation::Activate);
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Press(Key::Ctrl), KeyEvent::Press(Key::K)],
            ..Default::default()
        },
        |ui| {
            assert!(!ui.button("one"));
            assert!(ui.button("two"));
            assert!(ui.shortcut(
                Id(20),
                Shortcut {
                    key: Key::K,
                    modifiers: Modifiers {
                        ctrl: true,
                        ..Default::default()
                    },
                    scope: ShortcutScope::Global
                }
            ));
        },
    );
    assert_eq!(c.focused_widget_id(), Some(second));
    assert_ne!(first, second);
}
#[test]
fn double_click_requires_owner_distance_interval_and_button() {
    let mut c = test_context();
    let query = |ui: &mut crate::ui::Ui| ui.double_clicked(Id(1), [50., 50., 150., 150.], 0);
    frame(&mut c, [100., 100.], true, |ui| assert!(!query(ui)));
    frame(&mut c, [100., 100.], false, |_| {});
    frame(&mut c, [101., 100.], true, |ui| assert!(query(ui)));
    frame(&mut c, [100., 100.], false, |_| {});
    frame_with_input(
        &mut c,
        0.4,
        InputState {
            mouse_pos: [100., 100.],
            mouse_down: [true, false, false, false, false],
            ..Default::default()
        },
        |ui| assert!(!query(ui)),
    );
}

#[test]
fn editor_owns_shortcuts_and_gamepad_directions() {
    let mut c = test_context();
    let mut text = alloc::string::String::from("edit");
    let shortcut = Shortcut {
        key: Key::A,
        modifiers: Modifiers::default(),
        scope: ShortcutScope::Global,
    };
    frame(&mut c, [0.; 2], false, |ui| {
        ui.shortcut(Id(30), shortcut);
        let id = ui.widget_id("editor");
        ui.focus(id);
        ui.text_input("editor", &mut text);
    });
    let focused = c.focused_widget_id();
    c.navigate(Navigation::Next);
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Press(Key::A), KeyEvent::Char('a')],
            ..Default::default()
        },
        |ui| {
            assert!(!ui.shortcut(Id(30), shortcut));
            ui.text_input("editor", &mut text);
        },
    );
    assert_eq!(c.focused_widget_id(), focused);
    assert!(text.contains('a'));
}
#[test]
fn child_blocks_parent_mouse_and_survives_parent_rounding() {
    let mut c = test_context();
    let a = Id(40);
    let b = Id(41);
    let mut parent = false;
    frame(&mut c, [0.; 2], false, |ui| {
        ui.open_popup(a);
        ui.popup(a, [80., 80., 180., 180.], |ui| {
            let scope = ui
                .begin_rounded_clip(
                    "round",
                    crate::shapes::RoundedRect {
                        rect: [80., 80., 180., 180.],
                        radii: [5.; 4],
                    },
                )
                .unwrap();
            ui.open_popup(b);
            ui.popup(b, [300., 200., 500., 400.], |ui| {
                ui.button("child");
            });
            ui.end_rounded_clip(scope);
        });
    });
    frame(&mut c, [310., 210.], true, |ui| {
        ui.popup(a, [80., 80., 180., 180.], |ui| {
            parent = ui.button("parent");
            ui.popup(b, [300., 200., 500., 400.], |ui| {
                let id = ui.widget_id("child");
                ui.button("child");
                assert!(ui.is_captured(id));
            });
        });
    });
    assert!(!parent);
}
#[test]
fn repeated_double_click_queries_do_not_count_as_two_presses() {
    let mut c = test_context();
    frame(&mut c, [100., 100.], true, |ui| {
        for _ in 0..3 {
            assert!(!ui.double_clicked(Id(1), [50., 50., 150., 150.], 0));
        }
    });
}
#[test]
fn shortcuts_respect_modifier_order_and_popup_priority() {
    let mut c = test_context();
    let p = Id(60);
    let global = Shortcut {
        key: Key::K,
        modifiers: Modifiers {
            ctrl: true,
            ..Default::default()
        },
        scope: ShortcutScope::Global,
    };
    let local = Shortcut {
        scope: ShortcutScope::Popup(p),
        ..global
    };
    frame(&mut c, [0.; 2], false, |ui| {
        ui.shortcut(Id(61), global);
        ui.open_modal(p);
        ui.popup(p, [100., 100., 300., 300.], |ui| {
            ui.shortcut(Id(62), local);
        });
    });
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Press(Key::K), KeyEvent::Press(Key::Ctrl)],
            ..Default::default()
        },
        |ui| {
            ui.popup(p, [100., 100., 300., 300.], |ui| {
                assert!(!ui.shortcut(Id(62), local))
            });
        },
    );
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Press(Key::K)],
            ..Default::default()
        },
        |ui| {
            assert!(!ui.shortcut(Id(61), global));
            ui.popup(p, [100., 100., 300., 300.], |ui| {
                assert!(ui.shortcut(Id(62), local))
            });
        },
    );
}

#[test]
fn escape_restores_parent_editor_without_also_unfocusing_it() {
    let mut c = test_context();
    let mut text = alloc::string::String::from("parent");
    let mut parent = Id(0);
    frame(&mut c, [0.; 2], false, |ui| {
        ui.open_popup(Id(70));
        ui.popup(Id(70), [100., 100., 350., 350.], |ui| {
            parent = ui.widget_id("edit");
            ui.focus(parent);
            ui.text_input("edit", &mut text);
            ui.open_popup(Id(71));
            ui.popup(Id(71), [400., 200., 600., 400.], |ui| {
                let child = ui.widget_id("child");
                ui.focus(child);
                ui.button("child");
            });
        });
    });
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Press(Key::Escape)],
            ..Default::default()
        },
        |ui| {
            ui.popup(Id(70), [100., 100., 350., 350.], |ui| {
                ui.text_input("edit", &mut text);
                assert!(ui.is_focused(parent));
            });
        },
    );
    assert_eq!(c.focused_widget_id(), Some(parent));
    assert_eq!(c.captured_widget_id(), Some(parent));
}
#[test]
fn omitted_modals_expire_and_navigation_queue_is_bounded() {
    let mut c = test_context();
    frame(&mut c, [0.; 2], false, |ui| ui.open_modal(Id(80)));
    frame(&mut c, [0.; 2], false, |_| {});
    frame(&mut c, [0.; 2], false, |_| {});
    assert_eq!(c.modal_depth(), 0);
    for _ in 0..64 {
        assert!(c.navigate(Navigation::Next));
    }
    assert!(!c.navigate(Navigation::Next));
}
#[test]
fn shortcut_tab_is_consumed_before_stock_focus_navigation() {
    let mut c = test_context();
    let chord = Shortcut {
        key: Key::Tab,
        modifiers: Modifiers::default(),
        scope: ShortcutScope::Global,
    };
    let mut first = Id(0);
    frame(&mut c, [0.; 2], false, |ui| {
        first = ui.widget_id("one");
        ui.focus(first);
        ui.button("one");
        ui.button("two");
        ui.shortcut(Id(90), chord);
    });
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Press(Key::Tab)],
            ..Default::default()
        },
        |ui| {
            ui.button("one");
            ui.button("two");
            assert!(ui.shortcut(Id(90), chord));
        },
    );
    assert_eq!(c.focused_widget_id(), Some(first));
}
#[test]
fn closing_scope_can_schedule_a_replacement_without_reusing_live_buffers() {
    let mut c = test_context();
    frame(&mut c, [0.; 2], false, |ui| {
        ui.open_popup(Id(100));
        ui.popup(Id(100), [100., 100., 300., 300.], |ui| {
            ui.label("old");
            ui.close_popup();
            ui.open_modal(Id(101));
        });
        assert!(ui.is_popup_open(Id(101)));
        ui.modal(Id(101), [400., 200., 700., 500.], |ui| ui.label("new"));
    });
    assert_eq!(c.popup_depth(), 1);
    assert_eq!(c.modal_depth(), 1);
}
