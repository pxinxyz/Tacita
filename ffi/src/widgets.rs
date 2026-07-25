use std::ptr;

use crate::{
    active_frame_ctx, gui_export, str_arg, GuiContext, GuiErrorCode, GuiKeyBinding, GuiStr,
};

#[no_mangle]
pub unsafe extern "C" fn gui_label(ctx: *mut GuiContext, text: *const u8, text_len: usize) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            let Some(text) = (unsafe { str_arg(text, text_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            gui.ctx.ui().label(text);
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_button(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.ctx.ui().button(label)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_checkbox(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    // *mut bool, deliberately: this points at a LIVE C++ bool object, which is 0/1 by C++ law. The u8 paranoia is for bulk-filled structs. Soundness team, my lawyer is ready.
    value: *mut bool,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if value.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.ctx.ui().checkbox(label, unsafe { &mut *value })
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_slider_float(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    value: *mut f32,
    min: f32,
    max: f32,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if value.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.ctx
                .ui()
                .slider_float(label, unsafe { &mut *value }, min..=max)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_slider_int(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    value: *mut i32,
    min: i32,
    max: i32,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if value.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.ctx
                .ui()
                .slider_int(label, unsafe { &mut *value }, min..=max)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_same_line(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx.ui().same_line();
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_text_input(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    buf: *mut u8,
    buf_cap: usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if buf.is_null() || buf_cap == 0 {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let raw = unsafe { std::slice::from_raw_parts(buf, buf_cap) };
            let Some(content_len) = raw.iter().position(|&b| b == 0) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Ok(mut text) = std::str::from_utf8(&raw[..content_len]).map(str::to_owned) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let changed = gui.ctx.ui().text_input(label, &mut text);
            let mut written = text.len().min(buf_cap - 1);
            while !text.is_char_boundary(written) {
                written -= 1;
            }
            unsafe {
                ptr::copy_nonoverlapping(text.as_ptr(), buf, written);
                *buf.add(written) = 0;
            }
            changed
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_combo(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    items: *const GuiStr,
    item_count: usize,
    selected: *mut usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if selected.is_null() || (items.is_null() && item_count > 0) {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let raw = if item_count == 0 {
                &[][..]
            } else {
                unsafe { std::slice::from_raw_parts(items, item_count) }
            };
            let mut list: Vec<&str> = Vec::with_capacity(raw.len());
            for item in raw {
                let Some(s) = (unsafe { str_arg(item.ptr, item.len) }) else {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    return false;
                };
                list.push(s);
            }
            gui.ctx.ui().combo(label, &list, unsafe { &mut *selected })
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_color_edit_rgba(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    value: *mut u8,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if value.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let rgba = unsafe { &mut *(value as *mut [u8; 4]) };
            gui.ctx.ui().color_edit_rgba(label, rgba)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_keybind(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    binding: *mut GuiKeyBinding,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if binding.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(mut value) = (unsafe { *binding }).to_binding() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let changed = gui.ctx.ui().keybind(label, &mut value);
            unsafe { *binding = GuiKeyBinding::from_binding(value) };
            changed
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_tooltip_text(ctx: *mut GuiContext, text: *const u8, text_len: usize) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            let Some(text) = (unsafe { str_arg(text, text_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            gui.ctx.ui().tooltip_text(text);
        },
        ()
    )
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use crate::test_support::*;
    use crate::{
        gui_last_error_code, GuiErrorCode, GuiKeyBinding, GuiKeyEvent, GuiStr, GUI_KEY_BINDING_KEY,
        GUI_KEY_EVENT_PRESS,
    };

    const KEY_F: u16 = 14;

    #[test]
    fn button_clicks_on_the_release_frame_through_scripted_input() {
        let ctx = new_context();
        let mut m = Metrics::new();
        let at = center(m.text_widget_rect("OK"));
        let mut clicked = true;

        window_frame(ctx, AWAY, false, |ctx| {
            clicked = unsafe { crate::gui_button(ctx, b"OK".as_ptr(), 2) };
        });
        assert!(!clicked);

        window_frame(ctx, at, true, |ctx| {
            clicked = unsafe { crate::gui_button(ctx, b"OK".as_ptr(), 2) };
        });
        assert!(!clicked, "the press frame must not report a click");

        window_frame(ctx, at, false, |ctx| {
            clicked = unsafe { crate::gui_button(ctx, b"OK".as_ptr(), 2) };
        });
        assert!(clicked, "the release frame inside must report the click");

        window_frame(ctx, at, false, |ctx| {
            clicked = unsafe { crate::gui_button(ctx, b"OK".as_ptr(), 2) };
        });
        assert!(!clicked, "the click must not repeat");
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn checkbox_toggles_and_writes_the_host_value() {
        let ctx = new_context();
        let m = Metrics::new();
        let at = center(m.checkbox_box_rect());
        let mut value = false;
        let mut changed = false;

        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_checkbox(ctx, b"Flag".as_ptr(), 4, &mut value) };
        });
        window_frame(ctx, at, true, |ctx| {
            changed = unsafe { crate::gui_checkbox(ctx, b"Flag".as_ptr(), 4, &mut value) };
        });
        assert!(!changed);
        assert!(!value);
        window_frame(ctx, at, false, |ctx| {
            changed = unsafe { crate::gui_checkbox(ctx, b"Flag".as_ptr(), 4, &mut value) };
        });
        assert!(changed, "the release frame toggles");
        assert!(value, "the host's value must be written");
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn checkbox_null_value_is_invalid_input() {
        let ctx = new_context();
        window_frame(ctx, AWAY, false, |ctx| {
            let changed = unsafe { crate::gui_checkbox(ctx, b"Flag".as_ptr(), 4, ptr::null_mut()) };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn slider_drag_writes_the_host_value() {
        let ctx = new_context();
        let mut m = Metrics::new();
        let track = m.slider_track_rect("Vol");
        let mid = center(track);
        let mut value = 0.0f32;
        let mut changed = false;

        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_slider_float(ctx, b"Vol".as_ptr(), 3, &mut value, 0.0, 10.0) };
        });
        window_frame(ctx, mid, true, |ctx| {
            changed =
                unsafe { crate::gui_slider_float(ctx, b"Vol".as_ptr(), 3, &mut value, 0.0, 10.0) };
        });
        assert!(changed, "grabbing the track midpoint must change the value");
        assert!((value - 5.0).abs() < 0.2, "midpoint of 0..=10, got {value}");

        let mut ivalue = 0i32;
        window_frame(ctx, mid, false, |ctx| {
            unsafe { crate::gui_slider_int(ctx, b"Vol".as_ptr(), 3, &mut ivalue, 0, 10) };
        });
        window_frame(ctx, mid, true, |ctx| {
            changed = unsafe { crate::gui_slider_int(ctx, b"Vol".as_ptr(), 3, &mut ivalue, 0, 10) };
        });
        assert!(changed);
        assert!(
            (4..=6).contains(&ivalue),
            "midpoint of 0..=10, got {ivalue}"
        );
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn text_input_appends_into_the_host_buffer() {
        let ctx = new_context();
        let mut m = Metrics::new();
        let field = m.text_field_rect("Name");
        let at = center(field);
        let mut buf = [0u8; 32];

        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"Name".as_ptr(), 4, buf.as_mut_ptr(), buf.len()) };
        });
        window_frame(ctx, at, true, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"Name".as_ptr(), 4, buf.as_mut_ptr(), buf.len()) };
        });
        window_frame(ctx, at, false, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"Name".as_ptr(), 4, buf.as_mut_ptr(), buf.len()) };
        });
        let keys = [
            GuiKeyEvent {
                kind: 2,
                key: 0,
                ch: 'h' as u32,
            },
            GuiKeyEvent {
                kind: 2,
                key: 0,
                ch: 'i' as u32,
            },
        ];
        let mut changed = false;
        window_frame_keys(ctx, at, &keys, |ctx| {
            changed = unsafe {
                crate::gui_text_input(ctx, b"Name".as_ptr(), 4, buf.as_mut_ptr(), buf.len())
            };
        });
        assert!(changed, "typing must report the edit");
        assert_eq!(c_str(&buf), "hi");
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn text_input_truncates_at_cap_with_nul_and_never_overflows() {
        let ctx = new_context();
        let mut m = Metrics::new();
        let at = center(m.text_field_rect(""));
        let mut raw = [0xAAu8; 8];
        raw[..4].fill(0);
        let buf_cap = 4;

        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"".as_ptr(), 0, raw.as_mut_ptr(), buf_cap) };
        });
        window_frame(ctx, at, true, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"".as_ptr(), 0, raw.as_mut_ptr(), buf_cap) };
        });
        window_frame(ctx, at, false, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"".as_ptr(), 0, raw.as_mut_ptr(), buf_cap) };
        });
        let keys: Vec<GuiKeyEvent> = "abcdef"
            .chars()
            .map(|ch| GuiKeyEvent {
                kind: 2,
                key: 0,
                ch: ch as u32,
            })
            .collect();
        window_frame_keys(ctx, at, &keys, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"".as_ptr(), 0, raw.as_mut_ptr(), buf_cap) };
        });
        assert_eq!(&raw[..4], b"abc\0", "content clamps to cap-1 with a NUL");
        assert_eq!(&raw[4..], &[0xAA; 4], "nothing past buf_cap may be touched");
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn text_input_reads_the_existing_host_content() {
        let ctx = new_context();
        let mut m = Metrics::new();
        let at = center(m.text_field_rect(""));
        let mut buf = [0u8; 16];
        buf[..3].copy_from_slice(b"he\0");

        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"".as_ptr(), 0, buf.as_mut_ptr(), buf.len()) };
        });
        window_frame(ctx, at, true, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"".as_ptr(), 0, buf.as_mut_ptr(), buf.len()) };
        });
        window_frame(ctx, at, false, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"".as_ptr(), 0, buf.as_mut_ptr(), buf.len()) };
        });
        let keys = [GuiKeyEvent {
            kind: 2,
            key: 0,
            ch: 'y' as u32,
        }];
        window_frame_keys(ctx, at, &keys, |ctx| {
            unsafe { crate::gui_text_input(ctx, b"".as_ptr(), 0, buf.as_mut_ptr(), buf.len()) };
        });
        assert_eq!(c_str(&buf), "hey");
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn text_input_bad_buffers_are_invalid_input() {
        let ctx = new_context();
        window_frame(ctx, AWAY, false, |ctx| {
            let changed =
                unsafe { crate::gui_text_input(ctx, b"N".as_ptr(), 1, ptr::null_mut(), 8) };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });

        let mut no_nul = [b'x'; 4];
        window_frame(ctx, AWAY, false, |ctx| {
            let changed =
                unsafe { crate::gui_text_input(ctx, b"N".as_ptr(), 1, no_nul.as_mut_ptr(), 4) };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn combo_selection_writes_the_host_index() {
        let ctx = new_context();
        let mut m = Metrics::new();
        let items = [
            GuiStr::new("Red"),
            GuiStr::new("Green"),
            GuiStr::new("Blue"),
        ];
        let mut selected = 0usize;
        let button = m.combo_button_rect("Red");
        let row2 = m.combo_row_rect(button, &["Red", "Green", "Blue"], 2);

        let declare = |ctx: *mut crate::GuiContext, selected: &mut usize| -> bool {
            unsafe {
                crate::gui_combo(
                    ctx,
                    b"Color".as_ptr(),
                    5,
                    items.as_ptr(),
                    items.len(),
                    selected,
                )
            }
        };

        window_frame(ctx, AWAY, false, |ctx| {
            declare(ctx, &mut selected);
        });
        let at = center(button);
        window_frame(ctx, at, true, |ctx| {
            declare(ctx, &mut selected);
        });
        window_frame(ctx, at, false, |ctx| {
            declare(ctx, &mut selected);
        });
        let at_row = center(row2);
        window_frame(ctx, at_row, true, |ctx| {
            declare(ctx, &mut selected);
        });
        let mut changed = false;
        window_frame(ctx, at_row, false, |ctx| {
            changed = declare(ctx, &mut selected);
        });
        assert!(changed, "picking a row must report the change");
        assert_eq!(selected, 2);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn combo_bad_arguments_are_invalid_input() {
        let ctx = new_context();
        let mut selected = 0usize;
        let items = [GuiStr::new("A")];
        window_frame(ctx, AWAY, false, |ctx| {
            let changed =
                unsafe { crate::gui_combo(ctx, b"C".as_ptr(), 1, ptr::null(), 1, &mut selected) };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });

        window_frame(ctx, AWAY, false, |ctx| {
            let changed =
                unsafe { crate::gui_combo(ctx, b"C".as_ptr(), 1, ptr::null(), 0, &mut selected) };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        });

        window_frame(ctx, AWAY, false, |ctx| {
            let changed = unsafe {
                crate::gui_combo(ctx, b"C".as_ptr(), 1, items.as_ptr(), 1, ptr::null_mut())
            };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn combo_rejects_bad_item_strings() {
        let ctx = new_context();
        let mut selected = 0usize;

        let null_item = [GuiStr {
            ptr: ptr::null(),
            len: 3,
        }];
        window_frame(ctx, AWAY, false, |ctx| {
            let changed = unsafe {
                crate::gui_combo(ctx, b"C".as_ptr(), 1, null_item.as_ptr(), 1, &mut selected)
            };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });

        let bad_bytes = [0xFFu8, 0xFE];
        let items = [
            GuiStr::new("ok"),
            GuiStr {
                ptr: bad_bytes.as_ptr(),
                len: bad_bytes.len(),
            },
        ];
        window_frame(ctx, AWAY, false, |ctx| {
            let changed = unsafe {
                crate::gui_combo(ctx, b"C".as_ptr(), 1, items.as_ptr(), 2, &mut selected)
            };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn keybind_rejects_out_of_range_mouse_button() {
        let ctx = new_context();
        let mut binding = GuiKeyBinding {
            tag: crate::GUI_KEY_BINDING_MOUSE,
            mouse_button: 200,
            ..GuiKeyBinding::NONE
        };
        window_frame(ctx, AWAY, false, |ctx| {
            let changed = unsafe { crate::gui_keybind(ctx, b"J".as_ptr(), 1, &mut binding) };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn keybind_captures_a_key_through_scripted_events() {
        let ctx = new_context();
        let mut m = Metrics::new();
        let at = center(m.text_widget_rect("None"));
        let mut binding = GuiKeyBinding::NONE;
        let mut changed = false;

        let declare = |ctx: *mut crate::GuiContext, binding: &mut GuiKeyBinding| -> bool {
            unsafe { crate::gui_keybind(ctx, b"Jump".as_ptr(), 4, binding) }
        };

        window_frame(ctx, AWAY, false, |ctx| {
            declare(ctx, &mut binding);
        });
        window_frame(ctx, at, true, |ctx| {
            declare(ctx, &mut binding);
        });
        window_frame(ctx, at, false, |ctx| {
            declare(ctx, &mut binding);
        });
        assert_eq!(binding.tag, crate::GUI_KEY_BINDING_NONE);

        let keys = [GuiKeyEvent {
            kind: GUI_KEY_EVENT_PRESS,
            key: KEY_F,
            ch: 0,
        }];
        window_frame_keys(ctx, AWAY, &keys, |ctx| {
            changed = declare(ctx, &mut binding);
        });
        assert!(changed, "the capture frame must return true");
        assert_eq!(binding.tag, GUI_KEY_BINDING_KEY);
        assert_eq!(binding.key, KEY_F);

        window_frame_keys(ctx, AWAY, &keys, |ctx| {
            changed = declare(ctx, &mut binding);
        });
        assert!(!changed);
        assert_eq!(binding.tag, GUI_KEY_BINDING_KEY);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn keybind_null_binding_is_invalid_input() {
        let ctx = new_context();
        window_frame(ctx, AWAY, false, |ctx| {
            let changed = unsafe { crate::gui_keybind(ctx, b"J".as_ptr(), 1, ptr::null_mut()) };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn color_edit_declares_and_null_value_is_invalid_input() {
        let ctx = new_context();
        let mut value = [255u8, 0, 0, 255];
        window_frame(ctx, AWAY, false, |ctx| {
            let changed =
                unsafe { crate::gui_color_edit_rgba(ctx, b"Tint".as_ptr(), 4, value.as_mut_ptr()) };
            assert!(!changed, "no interaction, no change");
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        });
        assert_eq!(value, [255, 0, 0, 255]);

        window_frame(ctx, AWAY, false, |ctx| {
            let changed =
                unsafe { crate::gui_color_edit_rgba(ctx, b"Tint".as_ptr(), 4, ptr::null_mut()) };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn label_same_line_and_tooltip_declare_without_error() {
        let ctx = new_context();
        let mut m = Metrics::new();
        let at = center(m.text_widget_rect("A"));
        window_frame(ctx, at, false, |ctx| {
            unsafe { crate::gui_label(ctx, b"L".as_ptr(), 1) };
            unsafe { crate::gui_same_line(ctx) };
            unsafe { crate::gui_button(ctx, b"A".as_ptr(), 1) };
            unsafe { crate::gui_tooltip_text(ctx, b"hint".as_ptr(), 4) };
        });
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn label_with_invalid_utf8_is_invalid_input() {
        let ctx = new_context();
        let bad = [0xFFu8, 0xFE];
        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_label(ctx, bad.as_ptr(), bad.len()) };
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    fn c_str(buf: &[u8]) -> &str {
        let end = buf.iter().position(|&b| b == 0).expect("NUL present");
        std::str::from_utf8(&buf[..end]).expect("valid UTF-8")
    }

    #[test]
    fn gui_str_new_helper_works() {
        let s = GuiStr::new("hello");
        assert_eq!(s.len, 5);
        assert!(!s.ptr.is_null());
        let empty = GuiStr::new("");
        assert_eq!(empty.len, 0);
    }
}
