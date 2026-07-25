use restricted_gui::id::Id;

use crate::{active_frame_ctx, gui_export, str_arg, GuiContext, GuiErrorCode, GuiRect, GuiVec2};

#[no_mangle]
// The escape hatch: hosts build first-class widgets through the FFI with these primitives. With great pointer arithmetic comes great responsibility.
pub unsafe extern "C" fn gui_widget_id(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
) -> u64 {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return 0;
            };
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0;
            };
            gui.ctx.ui().widget_id(label).0
        },
        0
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_allocate(ctx: *mut GuiContext, size: GuiVec2) -> GuiRect {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return GuiRect::ZERO;
            };
            gui.ctx.ui().allocate(size.into()).into()
        },
        GuiRect::ZERO
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_is_hovered(ctx: *mut GuiContext, rect: GuiRect) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            gui.ctx.ui().is_hovered(rect.into())
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_capture(ctx: *mut GuiContext, id: u64) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx.ui().capture(Id(id));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_release_capture(ctx: *mut GuiContext, id: u64) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx.ui().release_capture(Id(id));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_is_captured(ctx: *mut GuiContext, id: u64) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            gui.ctx.ui().is_captured(Id(id))
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_mouse_pos(ctx: *mut GuiContext) -> GuiVec2 {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return GuiVec2 { x: 0.0, y: 0.0 };
            };
            gui.ctx.ui().mouse_pos().into()
        },
        GuiVec2 { x: 0.0, y: 0.0 }
    )
}

unsafe fn mouse_button_ctx<'a>(ctx: *mut GuiContext, button: usize) -> Option<&'a mut GuiContext> {
    let gui = unsafe { active_frame_ctx(ctx) }?;
    if button >= 3 {
        crate::set_last_error(GuiErrorCode::InvalidInput);
        return None;
    }
    Some(gui)
}

#[no_mangle]
pub unsafe extern "C" fn gui_mouse_down(ctx: *mut GuiContext, button: usize) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { mouse_button_ctx(ctx, button) }) else {
                return false;
            };
            gui.ctx.ui().mouse_down(button)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_mouse_pressed(ctx: *mut GuiContext, button: usize) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { mouse_button_ctx(ctx, button) }) else {
                return false;
            };
            gui.ctx.ui().mouse_pressed(button)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_mouse_released(ctx: *mut GuiContext, button: usize) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { mouse_button_ctx(ctx, button) }) else {
                return false;
            };
            gui.ctx.ui().mouse_released(button)
        },
        false
    )
}

#[cfg(test)]
mod tests {
    use crate::test_support::*;
    use crate::{gui_last_error_code, GuiColor, GuiErrorCode, GuiRect, GuiVec2};

    const IDLE: GuiColor = GuiColor {
        r: 60,
        g: 60,
        b: 80,
        a: 255,
    };

    unsafe fn toggle_square(ctx: *mut crate::GuiContext, label: &[u8], value: &mut bool) -> bool {
        unsafe {
            let id = crate::gui_widget_id(ctx, label.as_ptr(), label.len());
            let rect = crate::gui_allocate(ctx, GuiVec2 { x: 24.0, y: 24.0 });
            let rect_arr: [f32; 4] = rect.into();

            let mouse = crate::gui_mouse_pos(ctx);
            let inside = mouse.x >= rect_arr[0]
                && mouse.x < rect_arr[2]
                && mouse.y >= rect_arr[1]
                && mouse.y < rect_arr[3];
            if inside && crate::gui_mouse_pressed(ctx, 0) {
                crate::gui_capture(ctx, id);
            }
            let mut toggled = false;
            if crate::gui_is_captured(ctx, id) && crate::gui_mouse_released(ctx, 0) {
                crate::gui_release_capture(ctx, id);
                if inside {
                    *value = !*value;
                    toggled = true;
                }
            }
            let _hovered = crate::gui_is_hovered(ctx, rect);
            crate::gui_draw_rect_filled(
                ctx,
                GuiVec2 {
                    x: rect_arr[0],
                    y: rect_arr[1],
                },
                GuiVec2 {
                    x: rect_arr[2],
                    y: rect_arr[3],
                },
                IDLE,
            );
            toggled
        }
    }

    #[test]
    fn toggle_square_built_on_the_helper_exports_toggles() {
        let ctx = new_context();
        let m = Metrics::new();
        let origin = m.content_origin();
        let at = [origin[0] + 12.0, origin[1] + 12.0];
        let mut value = false;
        let mut toggled = false;

        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { toggle_square(ctx, b"sq", &mut value) };
        });
        window_frame(ctx, at, true, |ctx| {
            toggled = unsafe { toggle_square(ctx, b"sq", &mut value) };
        });
        assert!(!toggled);
        assert!(!value);
        window_frame(ctx, at, false, |ctx| {
            toggled = unsafe { toggle_square(ctx, b"sq", &mut value) };
        });
        assert!(toggled, "release inside the captured square toggles");
        assert!(value);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn allocate_advances_the_cursor() {
        let ctx = new_context();
        let m = Metrics::new();
        let mut first = GuiRect::ZERO;
        let mut second = GuiRect::ZERO;
        window_frame(ctx, AWAY, false, |ctx| unsafe {
            first = crate::gui_allocate(ctx, GuiVec2 { x: 24.0, y: 24.0 });
            second = crate::gui_allocate(ctx, GuiVec2 { x: 24.0, y: 24.0 });
        });
        let origin = m.content_origin();
        assert_eq!(
            <[f32; 4]>::from(first),
            [origin[0], origin[1], origin[0] + 24.0, origin[1] + 24.0]
        );
        assert!(
            second.min.y > first.min.y,
            "the second allocation must land on the next line: {first:?} vs {second:?}"
        );
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn mouse_queries_report_the_scripted_state() {
        let ctx = new_context();
        let at = [100.0, 100.0];
        let (mut pos, mut down, mut pressed, mut released) =
            (GuiVec2 { x: 0.0, y: 0.0 }, false, false, true);

        window_frame(ctx, at, true, |ctx| unsafe {
            pos = crate::gui_mouse_pos(ctx);
            down = crate::gui_mouse_down(ctx, 0);
            pressed = crate::gui_mouse_pressed(ctx, 0);
            released = crate::gui_mouse_released(ctx, 0);
        });
        assert_eq!([pos.x, pos.y], at);
        assert!(down && pressed && !released);

        window_frame(ctx, at, false, |ctx| unsafe {
            down = crate::gui_mouse_down(ctx, 0);
            pressed = crate::gui_mouse_pressed(ctx, 0);
            released = crate::gui_mouse_released(ctx, 0);
        });
        assert!(!down && !pressed && released);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn mouse_button_index_out_of_range_is_invalid_input() {
        let ctx = new_context();
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!unsafe { crate::gui_mouse_down(ctx, 3) });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!unsafe { crate::gui_mouse_pressed(ctx, usize::MAX) });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!unsafe { crate::gui_mouse_released(ctx, 99) });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn widget_id_matches_across_calls_and_scopes_differ() {
        let ctx = new_context();
        let (mut a1, mut a2, mut b) = (0u64, 0u64, 0u64);
        window_frame(ctx, AWAY, false, |ctx| unsafe {
            a1 = crate::gui_widget_id(ctx, b"same".as_ptr(), 4);
            a2 = crate::gui_widget_id(ctx, b"same".as_ptr(), 4);
            b = crate::gui_widget_id(ctx, b"other".as_ptr(), 5);
        });
        assert_eq!(a1, a2, "the same label derives the same id");
        assert_ne!(a1, b);
        assert_ne!(a1, 0);
        unsafe { crate::gui_context_free(ctx) };
    }
}
