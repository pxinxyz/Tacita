use crate::{
    active_frame_ctx, gui_export, str_arg, GuiContext, GuiErrorCode, GuiVec2, WindowState,
};

#[no_mangle]
// Begin/end pairs tracked on a stack; a mismatched pair is caught BEFORE the core panics, because panicking across the boundary at a C++ host is rude.
pub unsafe extern "C" fn gui_begin_window(
    ctx: *mut GuiContext,
    title: *const u8,
    title_len: usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            let Some(title) = (unsafe { str_arg(title, title_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            if !matches!(gui.window, WindowState::None) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            let opened = gui.ctx.ui().begin_window(title);
            gui.window = if opened {
                WindowState::Open
            } else {
                WindowState::Closed
            };
            opened
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_window(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            match gui.window {
                WindowState::Open => {
                    gui.ctx.ui().end_window();
                    gui.window = WindowState::None;
                }
                WindowState::Closed => gui.window = WindowState::None,
                WindowState::None => crate::set_last_error(GuiErrorCode::ContractViolation),
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_collapsing_header(
    ctx: *mut GuiContext,
    title: *const u8,
    title_len: usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            let Some(title) = (unsafe { str_arg(title, title_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let scope = gui.ctx.ui().begin_collapsing_header(title);
            let open = scope.is_some();
            gui.header_scopes.push(scope);
            open
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_collapsing_header(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            match gui.header_scopes.pop() {
                Some(Some(scope)) => gui.ctx.ui().end_collapsing_header(scope),
                Some(None) => {}
                None => crate::set_last_error(GuiErrorCode::ContractViolation),
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_scroll_region(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    size: GuiVec2,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            let scope = gui.ctx.ui().begin_scroll_region(label, size.into());
            gui.scroll_scopes.push(scope);
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_scroll_region(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            match gui.scroll_scopes.pop() {
                Some(scope) => gui.ctx.ui().end_scroll_region(scope),
                None => crate::set_last_error(GuiErrorCode::ContractViolation),
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_tab_bar(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            gui.ctx.ui().begin_tab_bar(label);
            gui.tab_bar_depth += 1;
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_tab(ctx: *mut GuiContext, title: *const u8, title_len: usize) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            let Some(title) = (unsafe { str_arg(title, title_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            if gui.tab_bar_depth == 0 {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            gui.ctx.ui().tab(title)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_tab_bar(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if gui.tab_bar_depth == 0 {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            gui.tab_bar_depth -= 1;
            gui.ctx.ui().end_tab_bar();
        },
        ()
    )
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use crate::test_support::*;
    use crate::{gui_last_error_code, GuiErrorCode};

    #[test]
    fn begin_end_window_round_trips_and_renders() {
        let ctx = new_context();
        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_label(ctx, b"hello".as_ptr(), 5) };
        });
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn end_window_with_none_open_is_contract_violation_not_panic() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        unsafe { crate::gui_end_window(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn nested_begin_window_is_contract_violation_not_panic() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) });
        let nested = unsafe { crate::gui_begin_window(ctx, b"V".as_ptr(), 1) };
        assert!(!nested, "a nested begin must not open a window");
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_end_window(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn closed_window_begin_returns_false_and_end_is_graceful_noop() {
        let ctx = new_context();
        window_frame(ctx, AWAY, false, |_| {});
        unsafe { (*ctx).ctx.set_window_open("W", false) };

        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        let opened = unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) };
        assert!(!opened, "a closed window's begin must return false");
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_end_window(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_render(ctx) };

        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        unsafe { crate::gui_end_window(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn widget_calls_without_a_frame_are_contract_violations() {
        let ctx = new_context();
        let opened = unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) };
        assert!(!opened);
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn window_null_title_is_invalid_input() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        let opened = unsafe { crate::gui_begin_window(ctx, ptr::null(), 3) };
        assert!(!opened);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn collapsing_header_opens_on_click_and_mismatches_are_violations() {
        let ctx = new_context();
        let m = Metrics::new();
        let mut open = true;
        window_frame(ctx, AWAY, false, |ctx| {
            open = unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
            if open {
                unsafe { crate::gui_end_collapsing_header(ctx) };
            }
        });
        assert!(!open, "a fresh header starts closed");

        let at = center(m.full_width_row_rect());
        window_frame(ctx, at, true, |ctx| {
            unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
        });
        window_frame(ctx, at, false, |ctx| {
            unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
        });
        window_frame(ctx, AWAY, false, |ctx| {
            open = unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
            if open {
                unsafe { crate::gui_label(ctx, b"inside".as_ptr(), 6) };
                unsafe { crate::gui_end_collapsing_header(ctx) };
            }
        });
        assert!(open, "the clicked header must open on the next frame");
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) });
        unsafe { crate::gui_end_collapsing_header(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn scroll_region_round_trips_and_mismatches_are_violations() {
        let ctx = new_context();
        window_frame(ctx, AWAY, false, |ctx| {
            unsafe {
                crate::gui_begin_scroll_region(
                    ctx,
                    b"R".as_ptr(),
                    1,
                    crate::GuiVec2 { x: 200.0, y: 60.0 },
                )
            };
            for label in [b"one".as_slice(), b"two".as_slice(), b"three".as_slice()] {
                unsafe { crate::gui_label(ctx, label.as_ptr(), label.len()) };
            }
            unsafe { crate::gui_end_scroll_region(ctx) };
        });
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) });
        unsafe { crate::gui_end_scroll_region(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn tab_bar_round_trips_and_mismatches_are_violations() {
        let ctx = new_context();
        let mut first_active = false;
        let mut second_active = true;
        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_begin_tab_bar(ctx, b"T".as_ptr(), 1) };
            first_active = unsafe { crate::gui_tab(ctx, b"One".as_ptr(), 3) };
            if first_active {
                unsafe { crate::gui_label(ctx, b"first".as_ptr(), 5) };
            }
            second_active = unsafe { crate::gui_tab(ctx, b"Two".as_ptr(), 3) };
            unsafe { crate::gui_end_tab_bar(ctx) };
        });
        assert!(first_active, "the first tab starts active");
        assert!(!second_active);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) });
        unsafe { crate::gui_end_tab_bar(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        let active = unsafe { crate::gui_tab(ctx, b"X".as_ptr(), 1) };
        assert!(!active);
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn null_context_container_calls_are_invalid_input() {
        let open = unsafe { crate::gui_begin_collapsing_header(ptr::null_mut(), b"H".as_ptr(), 1) };
        assert!(!open);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        unsafe { crate::gui_end_scroll_region(ptr::null_mut()) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
    }

    #[test]
    fn unbalanced_window_end_missing_is_violation_at_render() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) });
        let data = unsafe { crate::gui_render(ctx) };
        assert!(data.vertex_count > 0, "the frame still renders");
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);

        window_frame(ctx, AWAY, false, |_| {});
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn unbalanced_header_end_missing_is_violation_at_render() {
        let ctx = new_context();
        let m = Metrics::new();
        let at = center(m.full_width_row_rect());
        window_frame(ctx, at, true, |ctx| {
            unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
        });
        window_frame(ctx, at, false, |ctx| {
            unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
        });

        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) });
        let open = unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
        assert!(open, "settled header must be open — the leaky case");
        unsafe { crate::gui_end_window(ctx) };
        unsafe { crate::gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);

        window_frame(ctx, AWAY, false, |_| {});
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn unbalanced_scroll_region_end_missing_is_violation_at_render() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) });
        unsafe {
            crate::gui_begin_scroll_region(
                ctx,
                b"R".as_ptr(),
                1,
                crate::GuiVec2 { x: 100.0, y: 50.0 },
            )
        };
        unsafe { crate::gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);

        window_frame(ctx, AWAY, false, |_| {});
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn unbalanced_tab_bar_end_missing_is_violation_at_render() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) });
        unsafe { crate::gui_begin_tab_bar(ctx, b"T".as_ptr(), 1) };
        unsafe { crate::gui_end_window(ctx) };
        unsafe { crate::gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);

        window_frame(ctx, AWAY, false, |_| {});
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }
}
