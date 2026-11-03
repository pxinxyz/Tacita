use crate::{
    active_frame_ctx, gui_export, slice_arg, str_arg, ContainerKind, ContainerScope, GuiContext,
    GuiErrorCode, GuiRect, GuiVec2, WindowState,
};
use tacita::layout::{resolve_layout_extended_with, ContentAlign, GridCell, LayoutOptions};
use tacita::layout::{
    resolve_layout_with, LayoutAlign, LayoutItem, LayoutJustify, LayoutKind, LayoutSpec,
    MAX_LAYOUT_ITEMS,
};

pub const GUI_GRID_AUTO: u32 = u32::MAX;
pub const GUI_CONTENT_START: u32 = 0;
pub const GUI_CONTENT_CENTER: u32 = 1;
pub const GUI_CONTENT_END: u32 = 2;
pub const GUI_CONTENT_STRETCH: u32 = 3;
pub const GUI_CONTENT_SPACE_BETWEEN: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiLayoutOptions {
    pub wrap: u32,
    pub align_content: u32,
}
impl GuiLayoutOptions {
    fn decode(self) -> Option<LayoutOptions> {
        if self.wrap > 1 {
            return None;
        }
        Some(LayoutOptions {
            wrap: self.wrap != 0,
            align_content: match self.align_content {
                0 => ContentAlign::Start,
                1 => ContentAlign::Center,
                2 => ContentAlign::End,
                3 => ContentAlign::Stretch,
                4 => ContentAlign::SpaceBetween,
                _ => return None,
            },
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiGridCell {
    pub row: u32,
    pub column: u32,
    pub row_span: u32,
    pub column_span: u32,
}
impl GuiGridCell {
    fn decode(self) -> GridCell {
        GridCell::at(self.row, self.column, self.row_span, self.column_span)
    }
}
unsafe fn extended_arguments<'a>(
    options: GuiLayoutOptions,
    cells: *const GuiGridCell,
    count: usize,
) -> Option<(LayoutOptions, &'a [GuiGridCell])> {
    let decoded = options.decode();
    let cells = if cells.is_null() {
        Some(&[][..])
    } else {
        unsafe { slice_arg(cells, count) }
    };
    match (decoded, cells) {
        (Some(options), Some(cells)) => Some((options, cells)),
        _ => {
            crate::set_last_error(GuiErrorCode::InvalidInput);
            None
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_measure_layout_extended(
    spec: GuiLayoutSpec,
    items: *const GuiLayoutItem,
    count: usize,
    options: GuiLayoutOptions,
    cells: *const GuiGridCell,
) -> GuiVec2 {
    gui_export!(
        || {
            let Some((spec, items)) = (unsafe { arguments(spec, items, count) }) else {
                return [0.; 2].into();
            };
            let Some((options, cells)) = (unsafe { extended_arguments(options, cells, count) })
            else {
                return [0.; 2].into();
            };
            match resolve_layout_extended_with(spec, options, count, |i| {
                (
                    items[i].decode(),
                    cells.get(i).map_or_else(GridCell::default, |c| c.decode()),
                )
            }) {
                Ok(g) => g.size.into(),
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    [0.; 2].into()
                }
            }
        },
        [0.; 2].into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_layout_extended(
    ctx: *mut GuiContext,
    label: *const u8,
    len: usize,
    spec: GuiLayoutSpec,
    items: *const GuiLayoutItem,
    count: usize,
    options: GuiLayoutOptions,
    cells: *const GuiGridCell,
    out_rect: *mut GuiRect,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if !window(gui) {
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some((spec, items)) = (unsafe { arguments(spec, items, count) }) else {
                return false;
            };
            let Some((options, cells)) = (unsafe { extended_arguments(options, cells, count) })
            else {
                return false;
            };
            if out_rect.is_null()
                || !(out_rect as usize).is_multiple_of(core::mem::align_of::<GuiRect>())
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            match gui
                .ctx
                .ui()
                .begin_layout_extended_with(label, spec, options, count, |i| {
                    (
                        items[i].decode(),
                        cells.get(i).map_or_else(GridCell::default, |c| c.decode()),
                    )
                }) {
                Ok(scope) => {
                    unsafe {
                        *out_rect = scope.rect().into();
                    }
                    gui.containers.push(ContainerScope::Layout(scope));
                    true
                }
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    false
                }
            }
        },
        false
    )
}

pub const GUI_LAYOUT_ROW: u32 = 0;
pub const GUI_LAYOUT_COLUMN: u32 = 1;
pub const GUI_LAYOUT_GRID: u32 = 2;
pub const GUI_LAYOUT_START: u32 = 0;
pub const GUI_LAYOUT_CENTER: u32 = 1;
pub const GUI_LAYOUT_END: u32 = 2;
pub const GUI_LAYOUT_STRETCH: u32 = 3;
pub const GUI_LAYOUT_BASELINE: u32 = 4;
pub const GUI_LAYOUT_JUSTIFY_START: u32 = 0;
pub const GUI_LAYOUT_JUSTIFY_CENTER: u32 = 1;
pub const GUI_LAYOUT_JUSTIFY_END: u32 = 2;
pub const GUI_LAYOUT_SPACE_BETWEEN: u32 = 3;
pub const GUI_LAYOUT_MAX_ITEMS: u32 = 64;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GuiLayoutSpec {
    pub kind: u32,
    pub size: GuiVec2,
    pub min: GuiVec2,
    pub max: GuiVec2,
    pub padding: [f32; 4],
    pub gap: GuiVec2,
    pub align: u32,
    pub justify: u32,
    pub columns: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GuiLayoutItem {
    pub preferred: GuiVec2,
    pub min: GuiVec2,
    pub max: GuiVec2,
    pub grow: f32,
    pub shrink: f32,
    pub margin: [f32; 4],
    pub baseline: f32,
    pub absolute: u8,
    pub hit_test: u8,
    pub reserved: [u8; 2],
    pub anchor: GuiVec2,
    pub offset: GuiVec2,
    pub layer: i32,
}
impl GuiLayoutSpec {
    fn decode(self) -> Option<LayoutSpec> {
        let s = LayoutSpec {
            kind: match self.kind {
                0 => LayoutKind::Row,
                1 => LayoutKind::Column,
                2 => LayoutKind::Grid,
                _ => return None,
            },
            size: self.size.into(),
            min: self.min.into(),
            max: self.max.into(),
            padding: self.padding,
            gap: self.gap.into(),
            align: match self.align {
                0 => LayoutAlign::Start,
                1 => LayoutAlign::Center,
                2 => LayoutAlign::End,
                3 => LayoutAlign::Stretch,
                4 => LayoutAlign::Baseline,
                _ => return None,
            },
            justify: match self.justify {
                0 => LayoutJustify::Start,
                1 => LayoutJustify::Center,
                2 => LayoutJustify::End,
                3 => LayoutJustify::SpaceBetween,
                _ => return None,
            },
            columns: self.columns as usize,
        };
        s.is_valid().then_some(s)
    }
}
impl GuiLayoutItem {
    fn decode(self) -> LayoutItem {
        LayoutItem {
            preferred: self.preferred.into(),
            min: self.min.into(),
            max: self.max.into(),
            grow: self.grow,
            shrink: self.shrink,
            margin: self.margin,
            baseline: self.baseline,
            absolute: self.absolute != 0,
            hit_test: self.hit_test != 0,
            anchor: self.anchor.into(),
            offset: self.offset.into(),
            layer: self.layer,
        }
    }
}
unsafe fn arguments<'a>(
    spec: GuiLayoutSpec,
    items: *const GuiLayoutItem,
    count: usize,
) -> Option<(LayoutSpec, &'a [GuiLayoutItem])> {
    if count > MAX_LAYOUT_ITEMS {
        crate::set_last_error(GuiErrorCode::InvalidInput);
        return None;
    }
    let decoded = spec.decode();
    let items = unsafe { slice_arg(items, count) };
    match (decoded, items) {
        (Some(s), Some(i)) if i.iter().all(|v| v.decode().is_valid()) => Some((s, i)),
        _ => {
            crate::set_last_error(GuiErrorCode::InvalidInput);
            None
        }
    }
}
fn window(gui: &GuiContext) -> bool {
    if !matches!(gui.window, WindowState::Open) {
        crate::set_last_error(GuiErrorCode::ContractViolation);
        false
    } else {
        true
    }
}
#[no_mangle]
pub unsafe extern "C" fn gui_measure_layout(
    spec: GuiLayoutSpec,
    items: *const GuiLayoutItem,
    count: usize,
) -> GuiVec2 {
    gui_export!(
        || {
            let Some((spec, items)) = (unsafe { arguments(spec, items, count) }) else {
                return [0.0; 2].into();
            };
            match resolve_layout_with(spec, count, |i| items[i].decode()) {
                Ok(g) => g.size.into(),
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    [0.0; 2].into()
                }
            }
        },
        [0.0; 2].into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_layout(
    ctx: *mut GuiContext,
    label: *const u8,
    len: usize,
    spec: GuiLayoutSpec,
    items: *const GuiLayoutItem,
    count: usize,
    out_rect: *mut GuiRect,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if !window(gui) {
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some((spec, items)) = (unsafe { arguments(spec, items, count) }) else {
                return false;
            };
            if out_rect.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            match gui
                .ctx
                .ui()
                .begin_layout_with(label, spec, count, |i| items[i].decode())
            {
                Ok(scope) => {
                    unsafe {
                        *out_rect = scope.rect().into();
                    }
                    gui.containers.push(ContainerScope::Layout(scope));
                    true
                }
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_layout_item_rect(ctx: *mut GuiContext, index: usize) -> GuiRect {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return GuiRect::ZERO;
            };
            let Some(ContainerScope::Layout(scope)) = gui.containers.last() else {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return GuiRect::ZERO;
            };
            match scope.item_rect(index) {
                Some(rect) => rect.into(),
                None => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    GuiRect::ZERO
                }
            }
        },
        GuiRect::ZERO
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_layout_item(
    ctx: *mut GuiContext,
    index: usize,
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
            let Some(ContainerScope::Layout(parent)) = gui.containers.last_mut() else {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            };
            match gui.ctx.ui().begin_layout_item(parent, index) {
                Ok(scope) => {
                    unsafe {
                        *out_rect = parent.item_rect(index).unwrap_or([0.0; 4]).into();
                    }
                    gui.containers.push(ContainerScope::LayoutItem(scope));
                    true
                }
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_layout_item(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if gui.containers.last().map(ContainerScope::kind) != Some(ContainerKind::LayoutItem) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            if let Some(ContainerScope::LayoutItem(scope)) = gui.containers.pop() {
                if let Some(ContainerScope::Layout(parent)) = gui.containers.last_mut() {
                    gui.ctx.ui().end_layout_item(parent, scope);
                }
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_layout(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if gui.containers.last().map(ContainerScope::kind) != Some(ContainerKind::Layout) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            if let Some(ContainerScope::Layout(scope)) = gui.containers.pop() {
                gui.ctx.ui().end_layout(scope);
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_available_size(ctx: *mut GuiContext) -> GuiVec2 {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return [0.0; 2].into();
            };
            if !window(gui) {
                return [0.0; 2].into();
            }
            gui.ctx.ui().available_size().into()
        },
        [0.0; 2].into()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extended_layout_rejects_bad_options_cells_and_outputs_before_mutation() {
        unsafe {
            let items = [item(); 3];
            let row = GuiLayoutSpec {
                size: [180., 0.].into(),
                ..spec()
            };
            let options = GuiLayoutOptions {
                wrap: 1,
                align_content: GUI_CONTENT_START,
            };
            assert_eq!(
                gui_measure_layout_extended(row, items.as_ptr(), 3, options, core::ptr::null()),
                [180., 68.].into()
            );
            assert_eq!(crate::gui_last_error_code(), GuiErrorCode::None as u8);
            let font = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
            let ctx = crate::gui_context_new(font.as_ptr(), font.len());
            assert!(!ctx.is_null());
            crate::gui_new_frame(ctx, core::ptr::null(), 0.016);
            assert!(crate::gui_begin_window(
                ctx,
                b"ext".as_ptr(),
                3,
                crate::GUI_SCROLL_NONE
            ));
            let mut out: GuiRect = [-9.; 4].into();
            let sentinel = out;
            let before = (*ctx).ctx.inspect_windows().next().unwrap().vertices;
            for bad in [
                GuiLayoutOptions { wrap: 2, ..options },
                GuiLayoutOptions {
                    align_content: 99,
                    ..options
                },
            ] {
                assert!(!gui_begin_layout_extended(
                    ctx,
                    b"bad".as_ptr(),
                    3,
                    row,
                    items.as_ptr(),
                    3,
                    bad,
                    core::ptr::null(),
                    &mut out
                ));
                assert_eq!(
                    crate::gui_last_error_code(),
                    GuiErrorCode::InvalidInput as u8
                );
                assert_eq!(out, sentinel);
                assert!((*ctx).containers.is_empty());
            }
            assert!(!gui_begin_layout_extended(
                ctx,
                b"bad".as_ptr(),
                3,
                row,
                items.as_ptr(),
                3,
                options,
                core::ptr::null(),
                core::ptr::dangling_mut::<u8>().cast::<GuiRect>()
            ));
            let grid = GuiLayoutSpec {
                kind: GUI_LAYOUT_GRID,
                columns: 2,
                ..row
            };
            let cells = [GuiGridCell {
                row: 0,
                column: 0,
                row_span: 1,
                column_span: 2,
            }; 3];
            assert!(!gui_begin_layout_extended(
                ctx,
                b"bad".as_ptr(),
                3,
                grid,
                items.as_ptr(),
                3,
                GuiLayoutOptions { wrap: 0, ..options },
                cells.as_ptr(),
                &mut out
            ));
            assert_eq!(out, sentinel);
            assert_eq!(
                (*ctx).ctx.inspect_windows().next().unwrap().vertices,
                before
            );
            assert!((*ctx).containers.is_empty());
            assert!(gui_begin_layout_extended(
                ctx,
                b"wrap".as_ptr(),
                4,
                row,
                items.as_ptr(),
                3,
                options,
                core::ptr::null(),
                &mut out
            ));
            for i in 0..3 {
                assert!(gui_begin_layout_item(ctx, i, &mut out));
                crate::gui_button(ctx, b"cell".as_ptr(), 4);
                gui_end_layout_item(ctx);
            }
            gui_end_layout(ctx);
            crate::gui_end_window(ctx);
            crate::gui_render(ctx);
            assert_eq!(crate::gui_last_error_code(), GuiErrorCode::None as u8);
            crate::gui_context_free(ctx);
        }
    }
    fn spec() -> GuiLayoutSpec {
        GuiLayoutSpec {
            kind: GUI_LAYOUT_ROW,
            size: [180.0, 40.0].into(),
            min: [0.0; 2].into(),
            max: [1_000_000.0; 2].into(),
            padding: [0.0; 4],
            gap: [8.0; 2].into(),
            align: GUI_LAYOUT_START,
            justify: GUI_LAYOUT_JUSTIFY_START,
            columns: 1,
        }
    }
    fn item() -> GuiLayoutItem {
        GuiLayoutItem {
            preferred: [80.0, 30.0].into(),
            min: [0.0; 2].into(),
            max: [1_000_000.0; 2].into(),
            grow: 1.0,
            shrink: 1.0,
            margin: [0.0; 4],
            baseline: 0.0,
            absolute: 0,
            hit_test: 255,
            reserved: [255; 2],
            anchor: [0.0; 2].into(),
            offset: [0.0; 2].into(),
            layer: 0,
        }
    }
    #[test]
    fn invalid_descriptors_and_crossed_ends_preserve_scopes() {
        unsafe {
            let font = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
            let ctx = crate::gui_context_new(font.as_ptr(), font.len());
            assert!(!ctx.is_null());
            crate::gui_new_frame(ctx, core::ptr::null(), 0.016);
            assert!(crate::gui_begin_window(
                ctx,
                b"layout".as_ptr(),
                6,
                crate::GUI_SCROLL_NONE
            ));
            let mut rect = GuiRect::ZERO;
            let before = (*ctx).ctx.inspect_windows().next().unwrap().vertices;
            let valid = item();
            let mut bad = valid;
            bad.margin[0] = f32::NAN;
            assert!(!gui_begin_layout(
                ctx,
                b"bad".as_ptr(),
                3,
                spec(),
                [valid, bad].as_ptr(),
                2,
                &mut rect
            ));
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert!((*ctx).containers.is_empty());
            assert_eq!(
                (*ctx).ctx.inspect_windows().next().unwrap().vertices,
                before
            );
            assert_eq!(
                gui_measure_layout(
                    GuiLayoutSpec {
                        align: 99,
                        ..spec()
                    },
                    &valid,
                    1
                ),
                [0.0; 2].into()
            );
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert!(!gui_begin_layout(
                ctx,
                b"bad".as_ptr(),
                3,
                spec(),
                core::ptr::null(),
                1,
                &mut rect
            ));
            assert!(gui_begin_layout(
                ctx,
                b"ok".as_ptr(),
                2,
                spec(),
                &valid,
                1,
                &mut rect
            ));
            assert!(!gui_begin_layout_item(ctx, 2, &mut rect));
            assert_eq!((*ctx).containers.len(), 1);
            assert!(gui_begin_layout_item(ctx, 0, &mut rect));
            gui_end_layout(ctx);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert_eq!((*ctx).containers.len(), 2);
            crate::gui_button(ctx, b"child".as_ptr(), 5);
            gui_end_layout_item(ctx);
            assert!(!gui_begin_layout_item(ctx, 0, &mut rect));
            gui_end_layout(ctx);
            crate::gui_end_window(ctx);
            crate::gui_render(ctx);
            assert_eq!(crate::gui_last_error_code(), GuiErrorCode::None as u8);
            crate::gui_new_frame(ctx, core::ptr::null(), 0.016);
            assert!(crate::gui_begin_window(
                ctx,
                b"layout".as_ptr(),
                6,
                crate::GUI_SCROLL_NONE
            ));
            assert!(gui_begin_layout(
                ctx,
                b"ok".as_ptr(),
                2,
                spec(),
                &valid,
                1,
                &mut rect
            ));
            assert!(gui_begin_layout_item(ctx, 0, &mut rect));
            crate::gui_button(ctx, b"child".as_ptr(), 5);
            let draw = crate::gui_render(ctx);
            assert!(draw.vertex_count > 0);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert!((*ctx).containers.is_empty());
            crate::gui_context_free(ctx);
        }
    }
}
