use crate::{
    active_frame_ctx, gui_export, slice_arg, str_arg, ContainerKind, ContainerScope, GuiContext,
    GuiErrorCode, GuiStr, GuiVec2, WindowState,
};
use tacita::data::{
    DataTableOptions, DataTableState, NumericEdit, Selection, SortSpec, VisibleRows,
    MAX_DATA_COLUMNS, MAX_DATA_ROWS,
};

pub const GUI_DATA_MAX_COLUMNS: u32 = MAX_DATA_COLUMNS as u32;
pub const GUI_DATA_MAX_ROWS: u32 = MAX_DATA_ROWS as u32;
pub const GUI_SORT_NONE: u32 = 0;
pub const GUI_SORT_ASCENDING: u32 = 1;
pub const GUI_SORT_DESCENDING: u32 = 2;
pub struct GuiSelection(Selection);
pub struct GuiNumericEdit(NumericEdit);
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GuiVisibleRows {
    pub start: usize,
    pub end: usize,
}
impl From<VisibleRows> for GuiVisibleRows {
    fn from(r: VisibleRows) -> Self {
        Self {
            start: r.start,
            end: r.end,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiDataTableState {
    pub widths: [f32; MAX_DATA_COLUMNS],
    pub scroll_x: f32,
    pub sort_column: i32,
    pub sort_direction: u32,
}
impl From<DataTableState> for GuiDataTableState {
    fn from(s: DataTableState) -> Self {
        Self {
            widths: s.widths,
            scroll_x: s.scroll_x,
            sort_column: s.sort.map_or(-1, |v| v.column as i32),
            sort_direction: s.sort.map_or(0, |v| if v.descending { 2 } else { 1 }),
        }
    }
}
impl GuiDataTableState {
    fn decode(self) -> Option<DataTableState> {
        let sort = match (self.sort_column, self.sort_direction) {
            (-1, 0) => None,
            (i, 1 | 2) if i >= 0 && (i as usize) < MAX_DATA_COLUMNS => Some(SortSpec {
                column: i as usize,
                descending: self.sort_direction == 2,
            }),
            _ => return None,
        };
        Some(DataTableState {
            widths: self.widths,
            scroll_x: self.scroll_x,
            sort,
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiDataTableOptions {
    pub size: GuiVec2,
    pub row_height: f32,
    pub frozen_columns: u32,
    pub frozen_rows: u32,
}
impl From<GuiDataTableOptions> for DataTableOptions {
    fn from(o: GuiDataTableOptions) -> Self {
        Self {
            size: o.size.into(),
            row_height: o.row_height,
            frozen_columns: o.frozen_columns as usize,
            frozen_rows: o.frozen_rows as usize,
        }
    }
}
pub(super) fn invalid() {
    crate::set_last_error(GuiErrorCode::InvalidInput);
}
fn contract() {
    crate::set_last_error(GuiErrorCode::ContractViolation);
}
pub(super) unsafe fn target<'a, T>(p: *mut T) -> Option<&'a mut T> {
    if p.is_null() || !(p as usize).is_multiple_of(core::mem::align_of::<T>()) {
        invalid();
        return None;
    }
    Some(unsafe { &mut *p })
}
pub(super) unsafe fn canvas<'a>(ctx: *mut GuiContext) -> Option<&'a mut GuiContext> {
    let gui = unsafe { active_frame_ctx(ctx) }?;
    if !matches!(gui.window, WindowState::Open)
        || gui.containers.iter().any(|s| {
            matches!(
                s,
                ContainerScope::Header(None)
                    | ContainerScope::Tree(None)
                    | ContainerScope::Popup(false)
            )
        })
    {
        contract();
        return None;
    }
    Some(gui)
}
fn pop(gui: &mut GuiContext, kind: ContainerKind) -> Option<ContainerScope> {
    if gui.containers.last().map(ContainerScope::kind) != Some(kind) {
        contract();
        return None;
    }
    gui.containers.pop()
}

#[no_mangle]
pub unsafe extern "C" fn gui_selection_new(count: usize) -> *mut GuiSelection {
    gui_export!(
        || {
            match Selection::new(count) {
                Ok(s) => Box::into_raw(Box::new(GuiSelection(s))),
                Err(_) => {
                    invalid();
                    core::ptr::null_mut()
                }
            }
        },
        core::ptr::null_mut()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_selection_free(selection: *mut GuiSelection) {
    gui_export!(
        || {
            if !selection.is_null() {
                unsafe {
                    drop(Box::from_raw(selection));
                }
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_selection_resize(selection: *mut GuiSelection, count: usize) -> bool {
    gui_export!(
        || {
            let Some(s) = (unsafe { target(selection) }) else {
                return false;
            };
            if s.0.resize(count).is_err() {
                invalid();
                return false;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_selection_clear(selection: *mut GuiSelection) {
    gui_export!(
        || {
            if let Some(s) = unsafe { target(selection) } {
                s.0.clear();
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_selection_count(selection: *mut GuiSelection) -> usize {
    gui_export!(
        || { unsafe { target(selection) }.map_or(0, |s| s.0.selected_count()) },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_selection_contains(
    selection: *mut GuiSelection,
    index: usize,
) -> bool {
    gui_export!(
        || { unsafe { target(selection) }.is_some_and(|s| s.0.is_selected(index)) },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_selection_select(
    selection: *mut GuiSelection,
    index: usize,
    ctrl: u8,
    shift: u8,
) -> bool {
    gui_export!(
        || {
            let Some(s) = (unsafe { target(selection) }) else {
                return false;
            };
            if index >= s.0.len() || ctrl > 1 || shift > 1 {
                invalid();
                return false;
            }
            s.0.select(index, ctrl != 0, shift != 0)
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_numeric_edit_new() -> *mut GuiNumericEdit {
    gui_export!(
        || Box::into_raw(Box::new(GuiNumericEdit(NumericEdit::default()))),
        core::ptr::null_mut()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_numeric_edit_free(edit: *mut GuiNumericEdit) {
    gui_export!(
        || {
            if !edit.is_null() {
                unsafe {
                    drop(Box::from_raw(edit));
                }
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_numeric_edit_invalid(edit: *mut GuiNumericEdit) -> bool {
    gui_export!(
        || { unsafe { target(edit) }.is_some_and(|s| s.0.invalid()) },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_clip_rows(
    count: usize,
    height: f32,
    gap: f32,
    origin: f32,
    clip: GuiVec2,
    out: *mut GuiVisibleRows,
) -> bool {
    gui_export!(
        || {
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            if count > MAX_DATA_ROWS {
                invalid();
                return false;
            }
            match VisibleRows::new(count, height, gap, origin, clip.into()) {
                Ok(r) => {
                    *out = r.into();
                    true
                }
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_rows(
    ctx: *mut GuiContext,
    label: GuiStr,
    count: usize,
    height: f32,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            match gui.ctx.ui().begin_rows(label, count, height) {
                Ok(s) => {
                    gui.containers.push(ContainerScope::Rows(s));
                    true
                }
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_visible_rows(ctx: *mut GuiContext, out: *mut GuiVisibleRows) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            let r = match gui.containers.last() {
                Some(ContainerScope::Rows(s)) => s.visible_rows(),
                Some(ContainerScope::DataTable(s)) => s.visible_rows(),
                _ => {
                    contract();
                    return false;
                }
            };
            *out = r.into();
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_row(ctx: *mut GuiContext, index: usize) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(ContainerScope::Rows(rows)) = gui.containers.last() else {
                contract();
                return false;
            };
            let Some(s) = gui.ctx.ui().begin_row(rows, index) else {
                return false;
            };
            gui.containers.push(ContainerScope::DataCell(s));
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_rows(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::Rows(s)) = pop(gui, ContainerKind::Rows) {
                gui.ctx.ui().end_rows(s);
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_multi_select_list(
    ctx: *mut GuiContext,
    label: GuiStr,
    items: *const GuiStr,
    count: usize,
    selection: *mut GuiSelection,
    size: GuiVec2,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(s) = (unsafe { target(selection) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            if count != s.0.len()
                || ![size.x, size.y]
                    .iter()
                    .all(|v| v.is_finite() && *v > 0. && *v <= 1_000_000.)
            {
                invalid();
                return false;
            }
            let Some(items) = (unsafe { slice_arg(items, count) }) else {
                invalid();
                return false;
            };
            match gui.ctx.ui().multi_select_list_with(
                label,
                count,
                |i| unsafe { str_arg(items[i].ptr, items[i].len) },
                &mut s.0,
                size.into(),
            ) {
                Some(changed) => changed,
                None => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_data_table_state_init(out: *mut GuiDataTableState) -> bool {
    gui_export!(
        || {
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            *out = DataTableState::default().into();
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_data_table(
    ctx: *mut GuiContext,
    label: GuiStr,
    headers: *const GuiStr,
    columns: usize,
    rows: usize,
    state: *mut GuiDataTableState,
    options: GuiDataTableOptions,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(state) = (unsafe { target(state) }) else {
                return false;
            };
            let Some(mut decoded) = state.decode() else {
                invalid();
                return false;
            };
            let options: DataTableOptions = options.into();
            if !options.is_valid(columns, rows) {
                invalid();
                return false;
            }
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            let Some(raw) = (unsafe { slice_arg(headers, columns) }) else {
                invalid();
                return false;
            };
            let mut labels = [""; MAX_DATA_COLUMNS];
            for (i, h) in raw.iter().enumerate() {
                let Some(text) = (unsafe { str_arg(h.ptr, h.len) }) else {
                    invalid();
                    return false;
                };
                labels[i] = text;
            }
            match gui.ctx.ui().begin_data_table(
                label,
                &labels[..columns],
                rows,
                &mut decoded,
                options,
            ) {
                Ok(s) => {
                    *state = decoded.into();
                    gui.containers.push(ContainerScope::DataTable(s));
                    true
                }
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_data_table_cell(
    ctx: *mut GuiContext,
    row: usize,
    column: usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(ContainerScope::DataTable(table)) = gui.containers.last() else {
                contract();
                return false;
            };
            if row >= table.rows() || column >= table.columns() {
                invalid();
                return false;
            }
            let Some(s) = gui.ctx.ui().begin_data_table_cell(table, row, column) else {
                return false;
            };
            gui.containers.push(ContainerScope::DataCell(s));
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_data_cell(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::DataCell(s)) = pop(gui, ContainerKind::DataCell) {
                gui.ctx.ui().end_data_cell(s);
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_data_table(ctx: *mut GuiContext) -> f32 {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return 0.;
            };
            if let Some(ContainerScope::DataTable(s)) = pop(gui, ContainerKind::DataTable) {
                gui.ctx.ui().end_data_table(s)
            } else {
                0.
            }
        },
        0.
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_input_float(
    ctx: *mut GuiContext,
    label: GuiStr,
    value: *mut f32,
    edit: *mut GuiNumericEdit,
    step: f32,
    min: f32,
    max: f32,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(value) = (unsafe { target(value) }) else {
                return false;
            };
            let Some(edit) = (unsafe { target(edit) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            match gui
                .ctx
                .ui()
                .input_float(label, value, &mut edit.0, step, min, max)
            {
                Ok(v) => v,
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_input_int(
    ctx: *mut GuiContext,
    label: GuiStr,
    value: *mut i32,
    edit: *mut GuiNumericEdit,
    step: i32,
    min: i32,
    max: i32,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(value) = (unsafe { target(value) }) else {
                return false;
            };
            let Some(edit) = (unsafe { target(edit) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            match gui
                .ctx
                .ui()
                .input_int(label, value, &mut edit.0, step, min, max)
            {
                Ok(v) => v,
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_menu_bar(ctx: *mut GuiContext, label: GuiStr) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            gui.containers
                .push(ContainerScope::MenuBar(gui.ctx.ui().begin_menu_bar(label)));
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_menu(
    ctx: *mut GuiContext,
    label: GuiStr,
    size: GuiVec2,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            let Some(ContainerScope::MenuBar(bar)) = gui.containers.last_mut() else {
                contract();
                return false;
            };
            match gui.ctx.ui().begin_menu(bar, label, size.into()) {
                Ok(true) => {
                    gui.containers.push(ContainerScope::DataMenu);
                    true
                }
                Ok(false) => false,
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_submenu(
    ctx: *mut GuiContext,
    label: GuiStr,
    size: GuiVec2,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            if gui.containers.last().map(ContainerScope::kind) != Some(ContainerKind::DataMenu) {
                contract();
                return false;
            }
            match gui.ctx.ui().begin_submenu(label, size.into()) {
                Ok(true) => {
                    gui.containers.push(ContainerScope::DataMenu);
                    true
                }
                Ok(false) => false,
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_menu_item(
    ctx: *mut GuiContext,
    label: GuiStr,
    hint: GuiStr,
    enabled: u8,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            if enabled > 1 {
                invalid();
                return false;
            }
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            let Some(hint) = (unsafe { str_arg(hint.ptr, hint.len) }) else {
                invalid();
                return false;
            };
            if gui.containers.last().map(ContainerScope::kind) != Some(ContainerKind::DataMenu) {
                contract();
                return false;
            }
            let Some(bar) = gui.containers.iter().rev().find_map(|s| {
                if let ContainerScope::MenuBar(b) = s {
                    Some(b)
                } else {
                    None
                }
            }) else {
                contract();
                return false;
            };
            gui.ctx.ui().menu_item(bar, label, hint, enabled != 0)
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_menu(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::DataMenu) = pop(gui, ContainerKind::DataMenu) {
                gui.ctx.ui().end_menu();
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_menu_bar(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::MenuBar(s)) = pop(gui, ContainerKind::MenuBar) {
                gui.ctx.ui().end_menu_bar(s);
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_property_grid(
    ctx: *mut GuiContext,
    label: GuiStr,
    label_width: f32,
    row_height: f32,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            match gui
                .ctx
                .ui()
                .begin_property_grid(label, label_width, row_height)
            {
                Ok(s) => {
                    gui.containers.push(ContainerScope::PropertyGrid(s));
                    true
                }
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_property_row(ctx: *mut GuiContext, label: GuiStr) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(label) = (unsafe { str_arg(label.ptr, label.len) }) else {
                invalid();
                return false;
            };
            let Some(ContainerScope::PropertyGrid(grid)) = gui.containers.last() else {
                contract();
                return false;
            };
            let s = gui.ctx.ui().begin_property_row(grid, label);
            gui.containers.push(ContainerScope::PropertyRow(s));
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_property_row(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::PropertyRow(s)) = pop(gui, ContainerKind::PropertyRow) {
                gui.ctx.ui().end_property_row(s);
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_property_grid(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            if let Some(ContainerScope::PropertyGrid(s)) = pop(gui, ContainerKind::PropertyGrid) {
                gui.ctx.ui().end_property_grid(s);
            }
        },
        ()
    )
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
