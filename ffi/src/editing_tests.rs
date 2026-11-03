use super::*;
use crate::{
    gui_context_free, gui_last_error_code,
    test_support::{new_context, window_frame, window_frame_keys, AWAY},
    GuiKeyEvent, GUI_KEY_EVENT_PRESS, GUI_KEY_EVENT_RELEASE,
};
fn value(s: &str) -> GuiStr {
    GuiStr {
        ptr: s.as_ptr(),
        len: s.len(),
    }
}
fn options() -> GuiTextEditOptions {
    GuiTextEditOptions {
        size: GuiVec2 { x: 220., y: 100. },
        flags: GUI_EDIT_MULTILINE | GUI_EDIT_WRAP,
        direction: GUI_DIRECTION_AUTO,
    }
}
fn response() -> GuiTextEditResponse {
    GuiTextEditResponse {
        id: 0,
        rect: [0.; 4],
        changed: 0,
        focused: 0,
        reserved: [0; 6],
    }
}
#[test]
fn color_atlas_configuration_queries_and_errors_are_atomic() {
    unsafe {
        let ctx = new_context();
        assert!(!gui_set_color_font_texture(ctx, 1));
        assert!(gui_set_color_font_texture(ctx, 90));
        let bytes = include_bytes!("../../core/tests/assets/Noto-COLRv1.ttf");
        let face = crate::gui_add_font(ctx, bytes.as_ptr(), bytes.len(), 16.);
        assert!(gui_set_font_fallback(ctx, 0, &face, 1));
        let layout = gui_shaped_text_new();
        let mut size = GuiVec2 { x: 0., y: 0. };
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!gui_set_color_font_texture(ctx, 0));
            assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
            assert!(gui_shaped_text(
                ctx,
                layout,
                value("👩‍💻 🇬🇧 👍🏽"),
                GuiShapedOptions {
                    width: 0.,
                    wrap: 0,
                    reserved: [0; 3],
                    direction: 0
                },
                &mut size
            ));
        });
        let (mut w, mut h, mut len, mut generation) = (99, 98, 97, 96);
        assert!(gui_color_atlas_pixels(
            ctx,
            &mut w,
            core::ptr::null_mut(),
            &mut len,
            &mut generation
        )
        .is_null());
        assert_eq!((w, h, len, generation), (99, 98, 97, 96));
        let pixels = gui_color_atlas_pixels(ctx, &mut w, &mut h, &mut len, &mut generation);
        assert!(!pixels.is_null());
        assert_eq!(len, (w * h * 4) as usize);
        assert!(generation > 0);
        assert!(core::slice::from_raw_parts(pixels, len)
            .chunks_exact(4)
            .any(|p| p[3] > 128 && p[0] != p[2]));
        assert!(gui_set_color_font_texture(ctx, 0));
        gui_shaped_text_free(layout);
        gui_context_free(ctx);
    }
}
#[test]
fn owned_text_bounds_selection_utf8_and_short_outputs_are_atomic() {
    unsafe {
        let e = gui_text_editor_new(value("e\u{301}👩‍💻"));
        assert!(!e.is_null());
        assert!(!gui_text_editor_select(e, 1, 1));
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        assert!(gui_text_editor_select(e, 3, 0));
        let mut selection = GuiTextSelection {
            anchor: 99,
            cursor: 99,
        };
        assert!(gui_text_editor_selection(e, &mut selection));
        assert_eq!((selection.anchor, selection.cursor), (3, 0));
        let mut out = [0xa5; 4];
        assert_eq!(
            gui_text_editor_copy(e, GUI_EDIT_DOCUMENT, out.as_mut_ptr(), 4),
            14
        );
        assert_eq!(out, [0xa5; 4]);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        assert_eq!(
            gui_text_editor_copy(e, GUI_EDIT_SELECTION, out.as_mut_ptr(), 4),
            3
        );
        assert_eq!(&out[..3], "e\u{301}".as_bytes());
        let invalid = [0xff];
        assert!(!gui_text_editor_set(
            e,
            GuiStr {
                ptr: invalid.as_ptr(),
                len: 1
            }
        ));
        assert_eq!(gui_text_editor_copy(e, 0, core::ptr::null_mut(), 0), 14);
        assert!(!gui_text_editor_set(e, value("bad\0text")));
        assert!(!gui_text_editor_set(
            e,
            value(&"x".repeat(MAX_EDIT_BYTES + 1))
        ));
        gui_text_editor_free(e);
        gui_text_editor_free(core::ptr::null_mut());
        assert!(gui_text_editor_new(GuiStr {
            ptr: core::ptr::null(),
            len: 1
        })
        .is_null());
    }
}
#[test]
fn edit_composition_clipboard_and_between_frame_guards() {
    unsafe {
        let c = new_context();
        let e = gui_text_editor_new(value("original"));
        let mut out = response();
        window_frame(c, AWAY, false, |c| {
            let id = crate::gui_widget_id(c, b"field".as_ptr(), 5);
            crate::gui_focus(c, id);
            assert!(gui_text_edit(c, value("field"), e, options(), &mut out));
            assert!(!gui_text_event(c, id, GUI_IME_COMMIT, value("bad"), 0, 0));
            assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        });
        assert!(gui_text_editor_select(e, 0, 8));
        assert!(!gui_text_event(
            c,
            out.id,
            GUI_IME_PREEDIT,
            value("日本"),
            1,
            3
        ));
        assert!(gui_text_event(
            c,
            out.id,
            GUI_IME_PREEDIT,
            value("日本"),
            6,
            6
        ));
        window_frame(c, AWAY, false, |c| {
            assert!(gui_text_edit(c, value("field"), e, options(), &mut out));
        });
        assert_eq!(gui_text_editor_copy(e, 0, core::ptr::null_mut(), 0), 8);
        assert_eq!(gui_text_editor_copy(e, 2, core::ptr::null_mut(), 0), 6);
        let mut caret = GuiTextCaret {
            owner: 0,
            rect: [0.; 4],
            composing: 0,
            reserved: [0; 7],
        };
        assert!(gui_text_caret(c, &mut caret));
        assert_eq!(caret.composing, 1);
        assert!(gui_text_event(
            c,
            out.id,
            GUI_IME_COMMIT,
            value("committed"),
            0,
            0
        ));
        window_frame(c, AWAY, false, |c| {
            assert!(gui_text_edit(c, value("field"), e, options(), &mut out));
        });
        assert_eq!(out.changed, 1);
        let keys = [
            GuiKeyEvent {
                kind: GUI_KEY_EVENT_PRESS,
                key: crate::key_to_code(restricted_gui::input::Key::Ctrl),
                ch: 0,
            },
            GuiKeyEvent {
                kind: GUI_KEY_EVENT_PRESS,
                key: crate::key_to_code(restricted_gui::input::Key::A),
                ch: 0,
            },
            GuiKeyEvent {
                kind: GUI_KEY_EVENT_PRESS,
                key: crate::key_to_code(restricted_gui::input::Key::X),
                ch: 0,
            },
            GuiKeyEvent {
                kind: GUI_KEY_EVENT_RELEASE,
                key: crate::key_to_code(restricted_gui::input::Key::Ctrl),
                ch: 0,
            },
        ];
        window_frame_keys(c, AWAY, &keys, |c| {
            assert!(gui_text_edit(c, value("field"), e, options(), &mut out));
        });
        let mut request = GuiClipboardRequest {
            request_id: 0,
            owner: 0,
            action: 0,
            reserved: 0,
            text: value(""),
        };
        assert!(gui_clipboard_request(c, &mut request));
        assert_eq!(request.action, GUI_CLIPBOARD_CUT);
        assert_eq!(request.text.len, 9);
        assert!(gui_clipboard_complete(c, request.request_id, 0, value("")));
        window_frame(c, AWAY, false, |c| {
            assert!(gui_text_edit(c, value("field"), e, options(), &mut out));
        });
        assert_eq!(gui_text_editor_copy(e, 0, core::ptr::null_mut(), 0), 9);
        gui_text_editor_free(e);
        gui_context_free(c);
    }
}
#[test]
fn shaping_hit_test_fallback_and_invalid_options_preserve_outputs() {
    unsafe {
        let c = new_context();
        assert!(!gui_set_font_fallback(c, 0, &0, 1));
        assert_eq!(gui_font_fallback(c, 0, core::ptr::null_mut(), 0), 0);
        let l = gui_shaped_text_new();
        let mut size = GuiVec2 { x: 0., y: 0. };
        let mut out = GuiShapedCaret {
            byte: 0,
            position: size,
            line: 0,
        };
        window_frame(c, AWAY, false, |c| {
            assert!(gui_shaped_text(
                c,
                l,
                value("office אבג"),
                GuiShapedOptions {
                    width: 220.,
                    wrap: 1,
                    reserved: [0; 3],
                    direction: 0
                },
                &mut size
            ));
        });
        assert!(size.x > 0.);
        assert!(gui_shaped_caret(l, 7, &mut out));
        let position = out.position;
        assert!(gui_shaped_hit_test(l, position, &mut out));
        assert_eq!(out.position, position);
        let size_before = size;
        window_frame(c, AWAY, false, |c| {
            assert!(!gui_shaped_text(
                c,
                l,
                value("rejected"),
                GuiShapedOptions {
                    width: 0.,
                    wrap: 1,
                    reserved: [0; 3],
                    direction: 0
                },
                &mut size
            ));
        });
        assert_eq!(size, size_before);
        assert!(!gui_shaped_hit_test(
            l,
            GuiVec2 { x: f32::NAN, y: 0. },
            &mut out
        ));
        gui_shaped_text_free(l);
        gui_context_free(c);
    }
}
