use tacita::context::Context;
use tacita_sdk::{
    Gui, Id, InputState, Key, KeyEvent, Modifiers, Navigation, PanelState, ScrollFlags, Shortcut,
    ShortcutScope,
};
const FONT: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
#[test]
fn nested_stacks_and_navigation_forward_identically() {
    let mut core = Context::new(FONT).unwrap();
    let mut sdk = Gui::new(FONT).unwrap();
    for step in 0..4 {
        if step == 1 {
            core.navigate(Navigation::Next);
            sdk.navigate(Navigation::Next);
        }
        let keys = if step == 2 {
            &[KeyEvent::Press(Key::Escape)][..]
        } else {
            &[]
        };
        let input = InputState {
            keys,
            ..Default::default()
        };
        core.set_input(input);
        core.new_frame(0.016);
        sdk.new_frame(input, 0.016);
        core.ui().window("stack", ScrollFlags::NONE, |ui| {
            if step == 0 {
                ui.open_popup(Id(1));
            }
            ui.popup(Id(1), [100., 100., 300., 300.], |ui| {
                ui.button("parent");
                if step == 0 {
                    ui.open_modal(Id(2));
                }
                ui.modal(Id(2), [350., 200., 550., 400.], |ui| {
                    ui.button("child");
                });
                ui.label("after");
            });
        });
        sdk.begin_window("stack", ScrollFlags::NONE);
        if step == 0 {
            sdk.open_popup(Id(1));
        }
        sdk.begin_popup(Id(1), [100., 100., 300., 300.]);
        sdk.button("parent");
        if step == 0 {
            sdk.open_modal(Id(2));
        }
        sdk.begin_modal(Id(2), [350., 200., 550., 400.]);
        sdk.button("child");
        sdk.end_popup();
        sdk.label("after");
        sdk.end_popup();
        sdk.end_window();
        assert_eq!(core.popup_depth(), sdk.popup_depth());
        assert_eq!(core.modal_depth(), sdk.modal_depth());
        let a = core.render();
        let b = sdk.render();
        assert_eq!(a.vertices, b.vertices);
        assert_eq!(a.indices, b.indices);
        assert_eq!(a.commands, b.commands);
        assert_eq!(a.effects, b.effects);
    }
}
#[test]
fn snapshots_and_closed_shortcuts_preserve_core_contract() {
    let mut sdk = Gui::new(FONT).unwrap();
    sdk.set_window_rect("W", [70., 90.], [450., 550.]);
    let mut layout = sdk.save_layout();
    layout
        .set_panel("size", PanelState::Size([200., 120.]))
        .unwrap();
    let bytes = layout.encode().unwrap();
    let layout = tacita_sdk::LayoutSnapshot::decode(&bytes).unwrap();
    sdk.load_layout(&layout).unwrap();
    assert_eq!(sdk.save_layout().windows, layout.windows);
    let text = sdk.save_theme().unwrap();
    sdk.load_theme(text.as_bytes()).unwrap();
    sdk.set_window_open("W", false);
    sdk.new_frame(InputState::default(), 0.016);
    sdk.begin_window("W", ScrollFlags::NONE);
    assert!(!sdk.shortcut(
        Id(3),
        Shortcut {
            key: Key::K,
            modifiers: Modifiers::default(),
            scope: ShortcutScope::Global
        }
    ));
    assert!(!sdk.double_clicked(Id(4), [0., 0., 100., 100.], 0));
    sdk.end_window();
    sdk.render();
}
