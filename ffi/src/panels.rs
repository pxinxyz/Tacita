use crate::{
    active_frame_ctx, gui_export, scroll_flags_from_bits, str_arg, ContainerKind, ContainerScope,
    GuiContext, GuiErrorCode, GuiVec2,
};
use restricted_gui::panels::{DockState, DockZone, SplitAxis};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiDockState {
    pub zone: u32,
    pub float_pos: GuiVec2,
}

fn pop(gui: &mut GuiContext, kind: ContainerKind) -> Option<ContainerScope> {
    if gui.containers.last().map(ContainerScope::kind) != Some(kind) {
        crate::set_last_error(GuiErrorCode::ContractViolation);
        return None;
    }
    gui.containers.pop()
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_table(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    size: GuiVec2,
    columns: usize,
    rows: usize,
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
            if columns == 0 || rows == 0 || !size.x.is_finite() || !size.y.is_finite() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            gui.containers
                .push(ContainerScope::Table(gui.ctx.ui().begin_table(
                    label,
                    size.into(),
                    columns,
                    rows,
                )));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_table_cell(ctx: *mut GuiContext, row: usize, column: usize) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            let Some(ContainerScope::Table(table)) = gui.containers.last() else {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            };
            if row >= table.rows() || column >= table.columns() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let scope = gui.ctx.ui().begin_table_cell(table, row, column);
            gui.containers.push(ContainerScope::PanelCell(scope));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_panel_cell(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::PanelCell(scope)) = pop(gui, ContainerKind::PanelCell) {
                gui.ctx.ui().end_panel_cell(scope);
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_table(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::Table(scope)) = pop(gui, ContainerKind::Table) {
                gui.ctx.ui().end_table(scope);
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_split(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    size: GuiVec2,
    axis: u32,
    ratio: *mut f32,
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
            if ratio.is_null() || !ratio.is_aligned() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            if !size.x.is_finite() || !size.y.is_finite() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let axis = match axis {
                0 => SplitAxis::Horizontal,
                1 => SplitAxis::Vertical,
                _ => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    return;
                }
            };
            let ratio = unsafe { &mut *ratio };
            gui.containers
                .push(ContainerScope::Split(gui.ctx.ui().begin_split(
                    label,
                    size.into(),
                    axis,
                    ratio,
                )));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_split_pane(ctx: *mut GuiContext, index: usize) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            let Some(ContainerScope::Split(split)) = gui.containers.last() else {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            };
            if index > 1 {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let scope = gui.ctx.ui().begin_split_pane(split, index);
            gui.containers.push(ContainerScope::PanelCell(scope));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_split(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::Split(scope)) = pop(gui, ContainerKind::Split) {
                gui.ctx.ui().end_split(scope);
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_resizable_child(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    size: *mut GuiVec2,
    min_size: GuiVec2,
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
            if size.is_null() || !size.is_aligned() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let size = unsafe { &mut *size };
            let mut logical = [size.x, size.y];
            if !logical
                .iter()
                .chain([min_size.x, min_size.y].iter())
                .all(|v| v.is_finite())
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let scope =
                gui.ctx
                    .ui()
                    .begin_resizable_child(label, &mut logical, min_size.into(), flags);
            *size = logical.into();
            gui.containers.push(ContainerScope::ResizableChild(scope));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_resizable_child(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::ResizableChild(scope)) =
                pop(gui, ContainerKind::ResizableChild)
            {
                gui.ctx.ui().end_resizable_child(scope);
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_dockspace(
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
            if !size.x.is_finite() || !size.y.is_finite() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            gui.containers.push(ContainerScope::Dockspace(
                gui.ctx.ui().begin_dockspace(label, size.into()),
            ));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_begin_dock_panel(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    floating_size: GuiVec2,
    placement: *mut GuiDockState,
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
            if placement.is_null() || !placement.is_aligned() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let raw = unsafe { &mut *placement };
            if !floating_size.x.is_finite()
                || !floating_size.y.is_finite()
                || !raw.float_pos.x.is_finite()
                || !raw.float_pos.y.is_finite()
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let zone = match raw.zone {
                0 => DockZone::Left,
                1 => DockZone::Right,
                2 => DockZone::Floating,
                _ => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    return;
                }
            };
            let mut state = DockState {
                zone,
                float_pos: raw.float_pos.into(),
            };
            let Some(ContainerScope::Dockspace(dock)) = gui.containers.last_mut() else {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            };
            let scope =
                gui.ctx
                    .ui()
                    .begin_dock_panel(dock, label, floating_size.into(), &mut state);
            raw.zone = state.zone as u32;
            raw.float_pos = state.float_pos.into();
            gui.containers.push(ContainerScope::DockPanel(scope));
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_dock_panel(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::DockPanel(scope)) = pop(gui, ContainerKind::DockPanel) {
                gui.ctx.ui().end_dock_panel(scope);
            }
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_end_dockspace(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::Dockspace(scope)) = pop(gui, ContainerKind::Dockspace) {
                gui.ctx.ui().end_dockspace(scope);
            }
        },
        ()
    )
}
