use crate::{
    active_frame_ctx, gui_export, scroll_flags_from_bits, str_arg, ContainerKind, ContainerScope,
    GuiContext, GuiErrorCode, GuiRect, GuiScrollbarSkin, GuiVec2, WindowState,
};
use restricted_gui::id::Id;

#[no_mangle]
pub unsafe extern "C" fn gui_open_popup(ctx: *mut GuiContext, id: u64) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if !gui.ctx.can_open_popup(Id(id)) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            gui.ctx.ui().open_popup(Id(id));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_open_modal(ctx: *mut GuiContext, id: u64) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if !gui.ctx.can_open_popup(Id(id)) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            gui.ctx.ui().open_modal(Id(id));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_close_popup(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx.ui().close_popup();
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_is_popup_open(ctx: *mut GuiContext, id: u64) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            gui.ctx.ui().is_popup_open(Id(id))
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_popup(ctx: *mut GuiContext, id: u64, rect: GuiRect) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if !matches!(gui.window, WindowState::Open) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            let coordinates: [f32; 4] = rect.into();
            if !coordinates
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                || coordinates[2] <= coordinates[0]
                || coordinates[3] <= coordinates[1]
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let open = gui.ctx.ui().begin_popup(Id(id), rect.into());
            gui.containers.push(ContainerScope::Popup(open));
            open
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_modal(ctx: *mut GuiContext, id: u64, rect: GuiRect) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if !matches!(gui.window, WindowState::Open) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            let coordinates: [f32; 4] = rect.into();
            if !coordinates
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                || coordinates[2] <= coordinates[0]
                || coordinates[3] <= coordinates[1]
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let open = gui.ctx.ui().begin_modal(Id(id), rect.into());
            gui.containers.push(ContainerScope::Popup(open));
            open
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_context_menu(
    ctx: *mut GuiContext,
    id: u64,
    target: GuiRect,
    size: GuiVec2,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if !matches!(gui.window, WindowState::Open) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            let open = gui
                .ctx
                .ui()
                .begin_context_menu(Id(id), target.into(), size.into());
            gui.containers.push(ContainerScope::Popup(open));
            open
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_popup(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            match take_scope(gui, ContainerKind::Popup) {
                Some(ContainerScope::Popup(true)) => gui.ctx.ui().end_popup(),
                Some(ContainerScope::Popup(false)) | None => {}
                Some(_) => unreachable!("take_scope checked the container kind"),
            }
        },
        ()
    )
}

fn take_scope(gui: &mut GuiContext, expected: ContainerKind) -> Option<ContainerScope> {
    if gui.containers.last().map(ContainerScope::kind) != Some(expected) {
        crate::set_last_error(GuiErrorCode::ContractViolation);
        return None;
    }
    gui.containers.pop()
}

#[no_mangle]
pub unsafe extern "C" fn gui_set_window_rect(
    ctx: *mut GuiContext,
    title: *const u8,
    title_len: usize,
    pos: GuiVec2,
    size: GuiVec2,
) {
    gui_export!(
        || {
            if ctx.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let Some(title) = (unsafe { str_arg(title, title_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            let gui = unsafe { &mut *ctx };
            if gui.frame_active {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            gui.ctx.set_window_rect(title, pos.into(), size.into());
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_set_scrollbar_skin(ctx: *mut GuiContext, skin: GuiScrollbarSkin) {
    gui_export!(
        || {
            if ctx.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let gui = unsafe { &mut *ctx };
            if gui.frame_active {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            let Some(skin) = skin.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            gui.ctx.set_scrollbar_skin(skin);
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_window(
    ctx: *mut GuiContext,
    title: *const u8,
    title_len: usize,
    scroll_flags: u32,
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
            let Some(flags) = scroll_flags_from_bits(scroll_flags) else {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            };
            if !matches!(gui.window, WindowState::None) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            let opened = gui.ctx.ui().begin_window(title, flags);
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
            if !gui.containers.is_empty() {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
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
            gui.containers.push(ContainerScope::Header(scope));
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
            match take_scope(gui, ContainerKind::Header) {
                Some(ContainerScope::Header(Some(scope))) => {
                    gui.ctx.ui().end_collapsing_header(scope)
                }
                Some(ContainerScope::Header(None)) | None => {}
                Some(_) => unreachable!("take_scope checked the container kind"),
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_tree_node(
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
            let scope = gui.ctx.ui().begin_tree_node(label);
            let open = scope.is_some();
            gui.containers.push(ContainerScope::Tree(scope));
            open
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_tree_node(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            match take_scope(gui, ContainerKind::Tree) {
                Some(ContainerScope::Tree(Some(scope))) => gui.ctx.ui().end_tree_node(scope),
                Some(ContainerScope::Tree(None)) | None => {}
                Some(_) => unreachable!("take_scope checked the container kind"),
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
            gui.containers.push(ContainerScope::ScrollRegion(scope));
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
            end_child_scope(gui, ContainerKind::ScrollRegion);
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_child(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    width: f32,
    height: f32,
    scroll_flags: u32,
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
            let Some(flags) = scroll_flags_from_bits(scroll_flags) else {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            };
            let scope = gui.ctx.ui().begin_child(label, [width, height], flags);
            gui.containers.push(ContainerScope::Child(scope));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_child(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            end_child_scope(gui, ContainerKind::Child);
        },
        ()
    )
}

fn end_child_scope(gui: &mut GuiContext, expected: ContainerKind) {
    match take_scope(gui, expected) {
        Some(ContainerScope::ScrollRegion(token)) | Some(ContainerScope::Child(token)) => {
            gui.ctx.ui().end_child(token)
        }
        Some(_) => unreachable!("take_scope checked the container kind"),
        None => {}
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_group_box(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    width: f32,
    height: f32,
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
            let token = gui.ctx.ui().begin_group_box(label, [width, height]);
            gui.containers.push(ContainerScope::GroupBox(token));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_group_box(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            match take_scope(gui, ContainerKind::GroupBox) {
                Some(ContainerScope::GroupBox(token)) => gui.ctx.ui().end_group_box(token),
                Some(_) => unreachable!(),
                None => {}
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_preview_canvas(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    width: f32,
    height: f32,
    out_rect: *mut GuiRect,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if out_rect.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let (token, rect) = gui.ctx.ui().begin_preview_canvas(label, [width, height]);
            unsafe {
                *out_rect = rect.into();
            }
            gui.containers.push(ContainerScope::PreviewCanvas(token));
            true
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_preview_canvas(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            match take_scope(gui, ContainerKind::PreviewCanvas) {
                Some(ContainerScope::PreviewCanvas(token)) => {
                    gui.ctx.ui().end_preview_canvas(token)
                }
                Some(_) => unreachable!(),
                None => {}
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_disabled(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    disabled: bool,
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
            let token = gui.ctx.ui().begin_disabled(label, disabled);
            gui.containers.push(ContainerScope::Disabled(token));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_disabled(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            match take_scope(gui, ContainerKind::Disabled) {
                Some(ContainerScope::Disabled(token)) => gui.ctx.ui().end_disabled(token),
                Some(_) => unreachable!(),
                None => {}
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
            gui.containers.push(ContainerScope::TabBar);
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
            if gui.containers.last().map(ContainerScope::kind) != Some(ContainerKind::TabBar) {
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
            if take_scope(gui, ContainerKind::TabBar).is_some() {
                gui.ctx.ui().end_tab_bar();
            }
        },
        ()
    )
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use crate::test_support::*;
    use crate::{
        gui_last_error_code, GuiErrorCode, GUI_SCROLL_NONE, GUI_SCROLL_SCROLLABLE,
        GUI_SCROLL_SCROLLBAR,
    };

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
    fn popup_scope_pairs_and_rejects_crossed_end() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        let id = unsafe { crate::gui_widget_id(ctx, b"menu".as_ptr(), 4) };
        unsafe { crate::gui_open_popup(ctx, id) };
        assert!(unsafe {
            crate::gui_begin_popup(
                ctx,
                id,
                crate::GuiRect {
                    min: crate::GuiVec2 { x: 200.0, y: 150.0 },
                    max: crate::GuiVec2 { x: 350.0, y: 270.0 },
                },
            )
        });
        unsafe { crate::gui_end_window(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_end_popup(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe {
            crate::gui_end_window(ctx);
            crate::gui_render(ctx);
            crate::gui_context_free(ctx);
        }
    }

    #[test]
    fn set_window_rect_places_window_between_frames() {
        let ctx = new_context();
        unsafe {
            crate::gui_set_window_rect(
                ctx,
                b"W".as_ptr(),
                1,
                crate::GuiVec2 { x: 260.0, y: 180.0 },
                crate::GuiVec2 { x: 320.0, y: 210.0 },
            );
        }
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) });
        unsafe { crate::gui_end_window(ctx) };
        let draw = unsafe { crate::gui_render(ctx) };
        let verts =
            unsafe { std::slice::from_raw_parts(draw.vertices, draw.vertex_count as usize) };
        assert!(verts.iter().any(|vertex| vertex.pos[0] >= 260.0));
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn set_window_rect_rejects_bad_arguments() {
        let ctx = new_context();
        unsafe {
            crate::gui_set_window_rect(
                ctx,
                ptr::null(),
                1,
                crate::GuiVec2 { x: 0.0, y: 0.0 },
                crate::GuiVec2 { x: 100.0, y: 100.0 },
            );
        }
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        unsafe {
            crate::gui_set_window_rect(
                ptr::null_mut(),
                b"W".as_ptr(),
                1,
                crate::GuiVec2 { x: 0.0, y: 0.0 },
                crate::GuiVec2 { x: 100.0, y: 100.0 },
            );
        }
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        unsafe {
            crate::gui_set_window_rect(
                ctx,
                b"W".as_ptr(),
                1,
                crate::GuiVec2 { x: 0.0, y: 0.0 },
                crate::GuiVec2 { x: 100.0, y: 100.0 },
            );
        }
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn tree_node_opens_after_click_and_pairs_with_end() {
        let ctx = new_context();
        let metrics = Metrics::new();
        let origin = metrics.content_origin();
        let at = [origin[0] + 10.0, origin[1] + metrics.row_h() * 0.5];
        let mut open = true;
        window_frame(ctx, AWAY, false, |ctx| {
            open = unsafe { crate::gui_begin_tree_node(ctx, b"Branch".as_ptr(), 6) };
            unsafe { crate::gui_end_tree_node(ctx) };
        });
        assert!(!open);
        for down in [true, false] {
            window_frame(ctx, at, down, |ctx| {
                unsafe { crate::gui_begin_tree_node(ctx, b"Branch".as_ptr(), 6) };
                unsafe { crate::gui_end_tree_node(ctx) };
            });
        }
        window_frame(ctx, AWAY, false, |ctx| {
            open = unsafe { crate::gui_begin_tree_node(ctx, b"Branch".as_ptr(), 6) };
            if open {
                unsafe { crate::gui_label(ctx, b"nested".as_ptr(), 6) };
            }
            unsafe { crate::gui_end_tree_node(ctx) };
        });
        assert!(open);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn tree_node_end_without_begin_is_contract_violation() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        unsafe { crate::gui_end_tree_node(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn crossed_container_ends_preserve_the_innermost_scope() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        unsafe { crate::gui_begin_child(ctx, b"C".as_ptr(), 1, 180.0, 90.0, GUI_SCROLL_NONE) };
        assert!(!unsafe { crate::gui_begin_tree_node(ctx, b"T".as_ptr(), 1) });

        unsafe { crate::gui_end_child(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_end_tree_node(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        unsafe { crate::gui_begin_tab_bar(ctx, b"Tabs".as_ptr(), 4) };
        unsafe { crate::gui_end_child(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe {
            crate::gui_begin_scroll_region(
                ctx,
                b"R".as_ptr(),
                1,
                crate::GuiVec2 { x: 80.0, y: 40.0 },
            )
        };
        assert!(!unsafe { crate::gui_tab(ctx, b"Wrong level".as_ptr(), 11) });
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_end_scroll_region(ctx) };
        assert!(unsafe { crate::gui_tab(ctx, b"One".as_ptr(), 3) });
        unsafe { crate::gui_end_tab_bar(ctx) };
        unsafe { crate::gui_end_child(ctx) };
        unsafe { crate::gui_end_window(ctx) };
        unsafe { crate::gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn closing_window_before_its_child_is_rejected_and_recoverable() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        unsafe { crate::gui_begin_child(ctx, b"C".as_ptr(), 1, 180.0, 90.0, GUI_SCROLL_NONE) };
        unsafe { crate::gui_end_window(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_end_child(ctx) };
        unsafe { crate::gui_end_window(ctx) };
        unsafe { crate::gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn unbalanced_mixed_containers_unwind_in_reverse_begin_order() {
        let ctx = new_context();
        let metrics = Metrics::new();
        let origin = metrics.content_origin();
        let at = [origin[0] + 10.0, origin[1] + metrics.row_h() * 0.5];
        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_begin_tree_node(ctx, b"T".as_ptr(), 1) };
            unsafe { crate::gui_end_tree_node(ctx) };
        });
        for down in [true, false] {
            window_frame(ctx, at, down, |ctx| {
                unsafe { crate::gui_begin_tree_node(ctx, b"T".as_ptr(), 1) };
                unsafe { crate::gui_end_tree_node(ctx) };
            });
        }
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        assert!(unsafe { crate::gui_begin_tree_node(ctx, b"T".as_ptr(), 1) });
        unsafe { crate::gui_begin_child(ctx, b"C".as_ptr(), 1, 180.0, 90.0, GUI_SCROLL_NONE) };
        unsafe { crate::gui_begin_tab_bar(ctx, b"Tabs".as_ptr(), 4) };
        assert!(unsafe { crate::gui_tab(ctx, b"One".as_ptr(), 3) });
        assert!(!unsafe { crate::gui_begin_collapsing_header(ctx, b"Closed".as_ptr(), 6) });
        unsafe { crate::gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        window_frame(ctx, AWAY, false, |ctx| unsafe {
            crate::gui_label(ctx, b"recovered".as_ptr(), 9);
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
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) });
        let nested =
            unsafe { crate::gui_begin_window(ctx, b"V".as_ptr(), 1, crate::GUI_SCROLL_NONE) };
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
        let opened =
            unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) };
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
        let opened =
            unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) };
        assert!(!opened);
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn window_null_title_is_invalid_input() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        let opened =
            unsafe { crate::gui_begin_window(ctx, ptr::null(), 3, crate::GUI_SCROLL_NONE) };
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
            unsafe { crate::gui_end_collapsing_header(ctx) };
        });
        assert!(!open, "a fresh header starts closed");

        let at = center(m.full_width_row_rect());
        window_frame(ctx, at, true, |ctx| {
            unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
            unsafe { crate::gui_end_collapsing_header(ctx) };
        });
        window_frame(ctx, at, false, |ctx| {
            unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
            unsafe { crate::gui_end_collapsing_header(ctx) };
        });
        window_frame(ctx, AWAY, false, |ctx| {
            open = unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
            if open {
                unsafe { crate::gui_label(ctx, b"inside".as_ptr(), 6) };
            }
            unsafe { crate::gui_end_collapsing_header(ctx) };
        });
        assert!(open, "the clicked header must open on the next frame");
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) });
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
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) });
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
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) });
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
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) });
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
            unsafe { crate::gui_end_collapsing_header(ctx) };
        });
        window_frame(ctx, at, false, |ctx| {
            unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
            unsafe { crate::gui_end_collapsing_header(ctx) };
        });

        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) });
        let open = unsafe { crate::gui_begin_collapsing_header(ctx, b"H".as_ptr(), 1) };
        assert!(open, "settled header must be open — the leaky case");
        unsafe { crate::gui_end_window(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
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
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) });
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
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE) });
        unsafe { crate::gui_begin_tab_bar(ctx, b"T".as_ptr(), 1) };
        unsafe { crate::gui_end_window(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);

        window_frame(ctx, AWAY, false, |_| {});
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn begin_end_child_round_trips_and_renders() {
        let ctx = new_context();
        for flags in [
            GUI_SCROLL_NONE,
            GUI_SCROLL_SCROLLABLE,
            GUI_SCROLL_SCROLLABLE | GUI_SCROLL_SCROLLBAR,
        ] {
            window_frame(ctx, AWAY, false, |ctx| {
                unsafe { crate::gui_begin_child(ctx, b"C".as_ptr(), 1, 200.0, 60.0, flags) };
                assert_eq!(
                    gui_last_error_code(),
                    GuiErrorCode::None as u8,
                    "flags {flags}"
                );
                for label in [b"one".as_slice(), b"two".as_slice(), b"three".as_slice()] {
                    unsafe { crate::gui_label(ctx, label.as_ptr(), label.len()) };
                }
                unsafe { crate::gui_end_child(ctx) };
                assert_eq!(
                    gui_last_error_code(),
                    GuiErrorCode::None as u8,
                    "flags {flags}"
                );
            });
            let data = unsafe { crate::gui_render(ctx) };
            assert!(data.vertex_count > 0, "flags {flags} rendered nothing");
            assert_eq!(
                gui_last_error_code(),
                GuiErrorCode::None as u8,
                "flags {flags}"
            );
        }
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn unknown_scroll_flag_bits_are_contract_violations() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };

        let opened =
            unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_SCROLLABLE | 0x4) };
        assert!(!opened, "an undefined flag bit must not open the window");
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);

        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        for bits in [0x4u32, 1 << 31, u32::MAX] {
            unsafe { crate::gui_begin_child(ctx, b"C".as_ptr(), 1, 100.0, 40.0, bits) };
            assert_eq!(
                gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8,
                "flag bits {bits:#x} must be rejected"
            );
        }
        unsafe { crate::gui_end_window(ctx) };
        unsafe { crate::gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn scrollbar_without_scrollable_is_accepted_by_window_and_child() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_SCROLLBAR) });
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_begin_child(ctx, b"C".as_ptr(), 1, 200.0, 60.0, GUI_SCROLL_SCROLLBAR) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_label(ctx, b"inside".as_ptr(), 6) };
        unsafe { crate::gui_end_child(ctx) };
        unsafe { crate::gui_end_window(ctx) };
        let data = unsafe { crate::gui_render(ctx) };
        assert!(data.vertex_count > 0);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn crossed_child_and_scroll_region_ends_are_contract_violations() {
        let ctx = new_context();

        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        unsafe {
            crate::gui_begin_scroll_region(
                ctx,
                b"R".as_ptr(),
                1,
                crate::GuiVec2 { x: 200.0, y: 60.0 },
            )
        };
        unsafe { crate::gui_end_child(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_end_scroll_region(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_end_window(ctx) };
        unsafe { crate::gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        unsafe {
            crate::gui_begin_child(ctx, b"C".as_ptr(), 1, 200.0, 60.0, GUI_SCROLL_SCROLLABLE)
        };
        unsafe { crate::gui_end_scroll_region(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_end_child(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_end_window(ctx) };
        unsafe { crate::gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn end_child_with_nothing_open_is_contract_violation_not_panic() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        unsafe { crate::gui_end_child(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn unbalanced_child_end_missing_is_violation_at_render() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        unsafe {
            crate::gui_begin_child(ctx, b"C".as_ptr(), 1, 100.0, 50.0, GUI_SCROLL_SCROLLABLE)
        };
        unsafe { crate::gui_label(ctx, b"leaky".as_ptr(), 5) };
        let data = unsafe { crate::gui_render(ctx) };
        assert!(data.vertex_count > 0, "the frame still renders");
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);

        window_frame(ctx, AWAY, false, |_| {});
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn begin_window_accepts_every_scroll_flag_combination() {
        let ctx = new_context();
        let mut vertex_counts = std::collections::HashMap::new();

        for flags in [
            GUI_SCROLL_NONE,
            GUI_SCROLL_SCROLLABLE,
            GUI_SCROLL_SCROLLBAR,
            GUI_SCROLL_SCROLLABLE | GUI_SCROLL_SCROLLBAR,
        ] {
            let input = input_at(AWAY, false);
            unsafe { crate::gui_new_frame(ctx, &input, 0.016) };
            let opened = unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1, flags) };
            assert!(opened, "flags {flags} must open the window");
            assert_eq!(
                gui_last_error_code(),
                GuiErrorCode::None as u8,
                "flags {flags}"
            );
            for _ in 0..40 {
                unsafe { crate::gui_label(ctx, b"row".as_ptr(), 3) };
            }
            unsafe { crate::gui_end_window(ctx) };
            let data = unsafe { crate::gui_render(ctx) };
            assert!(data.vertex_count > 0, "flags {flags} rendered nothing");
            assert_eq!(
                gui_last_error_code(),
                GuiErrorCode::None as u8,
                "flags {flags}"
            );
            vertex_counts.insert(flags, data.vertex_count);
        }

        let none_count = vertex_counts[&GUI_SCROLL_NONE];
        let bar_count = vertex_counts[&(GUI_SCROLL_SCROLLABLE | GUI_SCROLL_SCROLLBAR)];
        assert!(
            bar_count > none_count,
            "SCROLLABLE|SCROLLBAR should emit more geometry than NONE: {} > {}",
            bar_count,
            none_count
        );

        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn form_scopes_pair_strictly_and_preview_returns_content_rect() {
        let ctx = new_context();
        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_begin_group_box(ctx, b"Group".as_ptr(), 5, 180.0, 100.0) };
            unsafe { crate::gui_end_disabled(ctx) };
            assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
            unsafe { crate::gui_begin_disabled(ctx, b"Muted".as_ptr(), 5, true) };
            unsafe { crate::gui_label(ctx, b"inside".as_ptr(), 6) };
            unsafe { crate::gui_end_disabled(ctx) };
            unsafe { crate::gui_end_group_box(ctx) };
            let mut rect = crate::GuiRect::ZERO;
            assert!(unsafe {
                crate::gui_begin_preview_canvas(ctx, b"View".as_ptr(), 4, 120.0, 80.0, &mut rect)
            });
            assert!(rect.max.x > rect.min.x && rect.max.y > rect.min.y);
            unsafe { crate::gui_end_preview_canvas(ctx) };
        });
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }
}
