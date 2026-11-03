use restricted_gui::ui::ValidationState;
use std::ptr;

use crate::{
    active_frame_ctx, gui_export, slice_arg, str_arg, GuiColor, GuiContext, GuiErrorCode,
    GuiKeyBinding, GuiStr, GuiVec2,
};

const INLINE_ITEM_CAPACITY: usize = 32;

unsafe fn with_items<R>(raw: &[GuiStr], use_items: impl FnOnce(&[&str]) -> R) -> Option<R> {
    if raw.len() <= INLINE_ITEM_CAPACITY {
        let mut items = [""; INLINE_ITEM_CAPACITY];
        for (slot, item) in items.iter_mut().zip(raw) {
            *slot = unsafe { str_arg(item.ptr, item.len) }?;
        }
        Some(use_items(&items[..raw.len()]))
    } else {
        let mut items = Vec::new();
        items.try_reserve_exact(raw.len()).ok()?;
        for item in raw {
            items.push(unsafe { str_arg(item.ptr, item.len) }?);
        }
        Some(use_items(&items))
    }
}

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
pub unsafe extern "C" fn gui_pill_toggle(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
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
            gui.ctx.ui().pill_toggle(label, unsafe { &mut *value })
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_segmented_control(
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
            if selected.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(raw) = (unsafe { slice_arg(items, item_count) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(changed) = (unsafe {
                with_items(raw, |list| {
                    gui.ctx.ui().segmented_control(label, list, &mut *selected)
                })
            }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            changed
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_icon_button(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    side: f32,
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
            gui.ctx.ui().icon_button(label, side)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_separator(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx.ui().separator();
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_color_swatch(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    color: GuiColor,
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
            gui.ctx.ui().color_swatch(label, color.into(), size.into());
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_status_badge(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    color: GuiColor,
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
            gui.ctx.ui().status_badge(label, color.into());
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_hotkey_hint(ctx: *mut GuiContext, label: *const u8, label_len: usize) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            gui.ctx.ui().hotkey_hint(label);
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_radio_button(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    selected: *mut usize,
    index: usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if selected.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.ctx
                .ui()
                .radio_button(label, unsafe { &mut *selected }, index)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_progress_bar(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    fraction: f32,
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
            gui.ctx.ui().progress_bar(label, fraction, size.into());
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_spinner(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    diameter: f32,
    phase: f32,
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
            gui.ctx.ui().spinner(label, diameter, phase);
        },
        ()
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
pub unsafe extern "C" fn gui_filled_slider_float(
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
                .filled_slider_float(label, unsafe { &mut *value }, min..=max)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_filled_slider_int(
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
                .filled_slider_int(label, unsafe { &mut *value }, min..=max)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_range_slider_float(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    lower: *mut f32,
    upper: *mut f32,
    min: f32,
    max: f32,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if lower.is_null() || upper.is_null() || lower == upper {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.ctx.ui().range_slider_float(
                label,
                unsafe { &mut *lower },
                unsafe { &mut *upper },
                min..=max,
            )
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_range_slider_int(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    lower: *mut i32,
    upper: *mut i32,
    min: i32,
    max: i32,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if lower.is_null() || upper.is_null() || lower == upper {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.ctx.ui().range_slider_int(
                label,
                unsafe { &mut *lower },
                unsafe { &mut *upper },
                min..=max,
            )
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_drag_float(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    value: *mut f32,
    speed: f32,
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
                .drag_float(label, unsafe { &mut *value }, speed, min..=max)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_drag_int(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    value: *mut i32,
    speed: f32,
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
                .drag_int(label, unsafe { &mut *value }, speed, min..=max)
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
            let Some(raw) = (unsafe { slice_arg(buf, buf_cap) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(content_len) = raw.iter().position(|&b| b == 0) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Ok(text) = std::str::from_utf8(&raw[..content_len]) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.text_input_staging.clear();
            if gui.text_input_staging.try_reserve(text.len()).is_err() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            gui.text_input_staging.push_str(text);
            let changed = gui.ctx.ui().text_input(label, &mut gui.text_input_staging);
            let mut written = gui.text_input_staging.len().min(buf_cap - 1);
            while !gui.text_input_staging.is_char_boundary(written) {
                written -= 1;
            }
            unsafe {
                ptr::copy_nonoverlapping(gui.text_input_staging.as_ptr(), buf, written);
                *buf.add(written) = 0;
            }
            changed
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_search_box(
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
            let Some(raw) = (unsafe { slice_arg(buf, buf_cap) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(content_len) = raw.iter().position(|&b| b == 0) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Ok(text) = std::str::from_utf8(&raw[..content_len]) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.text_input_staging.clear();
            if gui.text_input_staging.try_reserve(text.len()).is_err() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            gui.text_input_staging.push_str(text);
            let changed = gui.ctx.ui().search_box(label, &mut gui.text_input_staging);
            let mut written = gui.text_input_staging.len().min(buf_cap - 1);
            while !gui.text_input_staging.is_char_boundary(written) {
                written -= 1;
            }
            unsafe {
                ptr::copy_nonoverlapping(gui.text_input_staging.as_ptr(), buf, written);
                *buf.add(written) = 0;
            }
            changed
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_validation_message(
    ctx: *mut GuiContext,
    state: u8,
    message: *const u8,
    message_len: usize,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            let state = match state {
                0 => ValidationState::Success,
                1 => ValidationState::Warning,
                2 => ValidationState::Error,
                _ => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    return;
                }
            };
            let Some(message) = (unsafe { str_arg(message, message_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            gui.ctx.ui().validation_message(state, message);
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_nav_row(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    selected: *mut usize,
    index: usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if selected.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.ctx
                .ui()
                .nav_row(label, unsafe { &mut *selected }, index)
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_breadcrumbs(
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
            if selected.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(raw) = (unsafe { slice_arg(items, item_count) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(changed) = (unsafe {
                with_items(raw, |list| {
                    gui.ctx.ui().breadcrumbs(label, list, &mut *selected)
                })
            }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
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
            let Some(raw) = (unsafe { slice_arg(items, item_count) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(changed) = (unsafe {
                with_items(raw, |list| gui.ctx.ui().combo(label, list, &mut *selected))
            }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            changed
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_searchable_combo(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    items: *const GuiStr,
    item_count: usize,
    selected: *mut usize,
    query_buf: *mut u8,
    query_cap: usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if selected.is_null()
                || query_buf.is_null()
                || query_cap == 0
                || (items.is_null() && item_count > 0)
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, label_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(raw) = (unsafe { slice_arg(items, item_count) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(query_bytes) = (unsafe { slice_arg(query_buf, query_cap) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(query_len) = query_bytes.iter().position(|&byte| byte == 0) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Ok(query) = std::str::from_utf8(&query_bytes[..query_len]) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            gui.text_input_staging.clear();
            if gui.text_input_staging.try_reserve(query.len()).is_err() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            gui.text_input_staging.push_str(query);
            let result = unsafe {
                gui.ctx.ui().searchable_combo_with(
                    label,
                    raw.len(),
                    |index| str_arg(raw[index].ptr, raw[index].len),
                    &mut *selected,
                    &mut gui.text_input_staging,
                )
            };
            let Some(changed) = result else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let mut written = gui.text_input_staging.len().min(query_cap - 1);
            while !gui.text_input_staging.is_char_boundary(written) {
                written -= 1;
            }
            unsafe {
                ptr::copy_nonoverlapping(gui.text_input_staging.as_ptr(), query_buf, written);
                *query_buf.add(written) = 0;
            }
            changed
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_list_box(
    ctx: *mut GuiContext,
    label: *const u8,
    label_len: usize,
    items: *const GuiStr,
    item_count: usize,
    selected: *mut usize,
    size: GuiVec2,
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
            let Some(raw) = (unsafe { slice_arg(items, item_count) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            match unsafe {
                gui.ctx.ui().list_box_with(
                    label,
                    raw.len(),
                    |index| str_arg(raw[index].ptr, raw[index].len),
                    &mut *selected,
                    size.into(),
                )
            } {
                Some(changed) => changed,
                None => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    false
                }
            }
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
pub unsafe extern "C" fn gui_color_picker_rgba(
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
            gui.ctx.ui().color_picker_rgba(label, rgba)
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
pub unsafe extern "C" fn gui_toast(
    ctx: *mut GuiContext,
    text: *const u8,
    text_len: usize,
    seconds: f32,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            let Some(text) = (unsafe { str_arg(text, text_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            gui.ctx.ui().toast(text, seconds);
        },
        ()
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
        gui_last_error_code, GuiContext, GuiErrorCode, GuiKeyBinding, GuiKeyEvent, GuiStr, GuiVec2,
        GUI_KEY_BINDING_KEY, GUI_KEY_EVENT_PRESS,
    };

    const KEY_F: u16 = 14;

    #[test]
    fn large_item_arrays_are_passed_through() {
        let labels = vec!["entry"; 40];
        let raw: Vec<GuiStr> = labels
            .iter()
            .map(|label| GuiStr {
                ptr: label.as_ptr(),
                len: label.len(),
            })
            .collect();
        let observed = unsafe { super::with_items(&raw, |items| items.len()) };
        assert_eq!(observed, Some(40));
    }

    #[test]
    fn new_choice_controls_validate_pointers_and_keep_idle_state() {
        let ctx = new_context();
        let items = [GuiStr::new("A"), GuiStr::new("B")];
        let bad = [GuiStr {
            ptr: ptr::null(),
            len: 1,
        }];
        let mut enabled = false;
        let mut selected = 1usize;
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!unsafe { crate::gui_pill_toggle(ctx, b"P".as_ptr(), 1, &mut enabled) });
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
            assert!(!unsafe {
                crate::gui_segmented_control(
                    ctx,
                    b"S".as_ptr(),
                    1,
                    items.as_ptr(),
                    items.len(),
                    &mut selected,
                )
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        });
        assert!(!enabled);
        assert_eq!(selected, 1);
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!unsafe { crate::gui_pill_toggle(ctx, b"P".as_ptr(), 1, ptr::null_mut()) });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
            assert!(!unsafe {
                crate::gui_segmented_control(ctx, b"S".as_ptr(), 1, ptr::null(), 0, &mut selected)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
            assert!(!unsafe {
                crate::gui_segmented_control(ctx, b"S".as_ptr(), 1, ptr::null(), 1, &mut selected)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
            assert!(!unsafe {
                crate::gui_segmented_control(ctx, b"S".as_ptr(), 1, bad.as_ptr(), 1, &mut selected)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        assert_eq!(selected, 1);
        unsafe { crate::gui_context_free(ctx) };
    }

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
    fn filled_slider_exports_keep_idle_values_and_reject_null_targets() {
        let ctx = new_context();
        let mut gain = 0.4f32;
        let mut count = 7i32;
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!unsafe {
                crate::gui_filled_slider_float(ctx, b"Gain".as_ptr(), 4, &mut gain, 0.0, 1.0)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
            assert!(!unsafe {
                crate::gui_filled_slider_int(ctx, b"Count".as_ptr(), 5, &mut count, 0, 10)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
            assert!(!unsafe {
                crate::gui_filled_slider_float(ctx, b"Gain".as_ptr(), 4, ptr::null_mut(), 0.0, 1.0)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
            assert!(!unsafe {
                crate::gui_filled_slider_int(ctx, b"Count".as_ptr(), 5, ptr::null_mut(), 0, 10)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        assert_eq!(gain, 0.4);
        assert_eq!(count, 7);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn range_and_drag_exports_preserve_idle_values_and_reject_bad_targets() {
        let ctx = new_context();
        let (mut low_f, mut high_f) = (0.2f32, 0.8f32);
        let (mut low_i, mut high_i) = (2_000_000_020i32, 2_000_000_080i32);
        let (mut drag_f, mut drag_i) = (0.4f32, 2_000_000_001i32);
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!unsafe {
                crate::gui_range_slider_float(
                    ctx,
                    b"Range F".as_ptr(),
                    7,
                    &mut low_f,
                    &mut high_f,
                    0.0,
                    1.0,
                )
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
            assert!(!unsafe {
                crate::gui_range_slider_int(
                    ctx,
                    b"Range I".as_ptr(),
                    7,
                    &mut low_i,
                    &mut high_i,
                    2_000_000_000,
                    2_000_000_100,
                )
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
            assert!(!unsafe {
                crate::gui_drag_float(ctx, b"Drag F".as_ptr(), 6, &mut drag_f, 0.1, 0.0, 1.0)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
            assert!(!unsafe {
                crate::gui_drag_int(
                    ctx,
                    b"Drag I".as_ptr(),
                    6,
                    &mut drag_i,
                    1.0,
                    2_000_000_000,
                    2_000_000_100,
                )
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

            assert!(!unsafe {
                crate::gui_range_slider_float(
                    ctx,
                    b"Range F".as_ptr(),
                    7,
                    ptr::null_mut(),
                    &mut high_f,
                    0.0,
                    1.0,
                )
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
            assert!(!unsafe {
                crate::gui_range_slider_int(
                    ctx,
                    b"Range I".as_ptr(),
                    7,
                    &mut low_i,
                    ptr::null_mut(),
                    0,
                    10,
                )
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
            let same = &mut low_f as *mut f32;
            assert!(!unsafe {
                crate::gui_range_slider_float(ctx, b"Range F".as_ptr(), 7, same, same, 0.0, 1.0)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
            assert!(!unsafe {
                crate::gui_drag_float(ctx, b"Drag F".as_ptr(), 6, ptr::null_mut(), 0.1, 0.0, 1.0)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
            assert!(!unsafe {
                crate::gui_drag_int(ctx, b"Drag I".as_ptr(), 6, ptr::null_mut(), 1.0, 0, 10)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        assert_eq!((low_f, high_f), (0.2, 0.8));
        assert_eq!((low_i, high_i), (2_000_000_020, 2_000_000_080));
        assert_eq!((drag_f, drag_i), (0.4, 2_000_000_001));
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
    fn list_box_selects_duplicate_row_by_index() {
        let ctx = new_context();
        let m = Metrics::new();
        let items = [
            GuiStr::new("same"),
            GuiStr::new("same"),
            GuiStr::new("third"),
        ];
        let size = GuiVec2 { x: 180.0, y: 100.0 };
        let mut selected = 2usize;
        let declare = |ctx: *mut GuiContext, selected: &mut usize| unsafe {
            crate::gui_list_box(
                ctx,
                b"##Items".as_ptr(),
                7,
                items.as_ptr(),
                items.len(),
                selected,
                size,
            )
        };
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!declare(ctx, &mut selected))
        });
        let style = restricted_gui::style::Style::default();
        let inset =
            style.border_size + style.corner_rounding * (1.0 - core::f32::consts::FRAC_1_SQRT_2);
        let origin = m.content_origin();
        let row_h = m.row_h();
        let at = [
            origin[0] + inset + 12.0,
            origin[1] + inset + row_h + m.item_spacing[1] + row_h * 0.5,
        ];
        window_frame(ctx, at, true, |ctx| assert!(!declare(ctx, &mut selected)));
        window_frame(ctx, at, false, |ctx| assert!(declare(ctx, &mut selected)));
        assert_eq!(selected, 1);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn searchable_combo_filters_and_selects_with_host_query_buffer() {
        let ctx = new_context();
        let m = Metrics::new();
        let origin = m.content_origin();
        let at = [origin[0] + 12.0, origin[1] + m.row_h() * 0.5];
        let items = [
            GuiStr::new("Alpha"),
            GuiStr::new("Bravo"),
            GuiStr::new("Charlie"),
        ];
        let mut selected = 0usize;
        let mut query = [0u8; 24];
        query[..6].copy_from_slice(b"stale\0");
        let declare = |ctx: *mut GuiContext, selected: &mut usize, query: &mut [u8; 24]| unsafe {
            crate::gui_searchable_combo(
                ctx,
                b"Find".as_ptr(),
                4,
                items.as_ptr(),
                items.len(),
                selected,
                query.as_mut_ptr(),
                query.len(),
            )
        };
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!declare(ctx, &mut selected, &mut query))
        });
        assert_eq!(c_str(&query), "stale");
        window_frame(ctx, at, true, |ctx| {
            assert!(!declare(ctx, &mut selected, &mut query))
        });
        window_frame(ctx, at, false, |ctx| {
            assert!(!declare(ctx, &mut selected, &mut query))
        });
        assert_eq!(c_str(&query), "");
        let typed = [
            GuiKeyEvent {
                kind: 2,
                key: 0,
                ch: 'B' as u32,
            },
            GuiKeyEvent {
                kind: 2,
                key: 0,
                ch: 'r' as u32,
            },
        ];
        window_frame_keys(ctx, AWAY, &typed, |ctx| {
            assert!(!declare(ctx, &mut selected, &mut query));
        });
        assert_eq!(c_str(&query), "Br");
        let enter = [GuiKeyEvent {
            kind: GUI_KEY_EVENT_PRESS,
            key: 6,
            ch: 0,
        }];
        window_frame_keys(ctx, AWAY, &enter, |ctx| {
            assert!(declare(ctx, &mut selected, &mut query));
        });
        assert_eq!(selected, 1);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn searchable_combo_validates_query_and_only_decodes_open_items() {
        let ctx = new_context();
        let m = Metrics::new();
        let origin = m.content_origin();
        let at = [origin[0] + 12.0, origin[1] + m.row_h() * 0.5];
        let mut items = vec![GuiStr::new("valid"); 10_000];
        items[9_999] = GuiStr {
            ptr: ptr::null(),
            len: 1,
        };
        let mut selected = 0usize;
        let mut query = [0u8; 16];
        let declare = |ctx: *mut GuiContext, selected: &mut usize, query: &mut [u8; 16]| unsafe {
            crate::gui_searchable_combo(
                ctx,
                b"Find".as_ptr(),
                4,
                items.as_ptr(),
                items.len(),
                selected,
                query.as_mut_ptr(),
                query.len(),
            )
        };
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!declare(ctx, &mut selected, &mut query));
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        });
        window_frame(ctx, at, true, |ctx| {
            assert!(!declare(ctx, &mut selected, &mut query))
        });
        window_frame(ctx, at, false, |ctx| {
            assert!(!declare(ctx, &mut selected, &mut query));
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        assert_eq!(selected, 0);
        window_frame(ctx, AWAY, false, |ctx| {
            let changed = unsafe {
                crate::gui_searchable_combo(
                    ctx,
                    b"Find".as_ptr(),
                    4,
                    ptr::null(),
                    0,
                    &mut selected,
                    query.as_mut_ptr(),
                    query.len(),
                )
            };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        });
        window_frame(ctx, AWAY, false, |ctx| {
            let changed = unsafe {
                crate::gui_searchable_combo(
                    ctx,
                    b"Find".as_ptr(),
                    4,
                    items.as_ptr(),
                    items.len(),
                    &mut selected,
                    ptr::null_mut(),
                    0,
                )
            };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        let mut no_nul = [b'x'; 4];
        window_frame(ctx, AWAY, false, |ctx| {
            let changed = unsafe {
                crate::gui_searchable_combo(
                    ctx,
                    b"Find".as_ptr(),
                    4,
                    items.as_ptr(),
                    items.len(),
                    &mut selected,
                    no_nul.as_mut_ptr(),
                    no_nul.len(),
                )
            };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        let mut bad_utf8 = [0xFF, 0];
        window_frame(ctx, AWAY, false, |ctx| {
            let changed = unsafe {
                crate::gui_searchable_combo(
                    ctx,
                    b"Find".as_ptr(),
                    4,
                    items.as_ptr(),
                    items.len(),
                    &mut selected,
                    bad_utf8.as_mut_ptr(),
                    bad_utf8.len(),
                )
            };
            assert!(!changed);
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn radio_and_progress_exports_preserve_host_state_and_validate_pointers() {
        let ctx = new_context();
        let m = Metrics::new();
        let origin = m.content_origin();
        let at = [origin[0] + 8.0, origin[1] + m.line_height * 0.5];
        let mut selected = 0usize;
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!unsafe {
                crate::gui_radio_button(ctx, b"Choice".as_ptr(), 6, &mut selected, 1)
            });
            unsafe {
                crate::gui_progress_bar(
                    ctx,
                    b"Work".as_ptr(),
                    4,
                    0.5,
                    GuiVec2 { x: 160.0, y: 20.0 },
                )
            };
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        });
        window_frame(ctx, at, true, |ctx| {
            assert!(!unsafe {
                crate::gui_radio_button(ctx, b"Choice".as_ptr(), 6, &mut selected, 1)
            });
        });
        window_frame(ctx, at, false, |ctx| {
            assert!(unsafe {
                crate::gui_radio_button(ctx, b"Choice".as_ptr(), 6, &mut selected, 1)
            });
            assert_eq!(selected, 1);
            assert!(!unsafe {
                crate::gui_radio_button(ctx, b"Choice##bad".as_ptr(), 10, ptr::null_mut(), 2)
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn list_box_checks_only_visible_strings_and_rejects_bad_visible_utf8() {
        let ctx = new_context();
        let bad_bytes = [0xFFu8];
        let mut items = vec![GuiStr::new("same"); 10_000];
        items[9_998] = GuiStr {
            ptr: bad_bytes.as_ptr(),
            len: bad_bytes.len(),
        };
        let mut selected = 0usize;
        let size = GuiVec2 { x: 180.0, y: 100.0 };
        let declare = |ctx: *mut GuiContext, selected: &mut usize| unsafe {
            crate::gui_list_box(
                ctx,
                b"##Items".as_ptr(),
                7,
                items.as_ptr(),
                items.len(),
                selected,
                size,
            )
        };

        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!declare(ctx, &mut selected));
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        });

        let mut input = input_at([90.0, 100.0], false);
        input.mouse_wheel = -100_000.0;
        frame(ctx, &input, |ctx| {
            assert!(unsafe {
                crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE)
            });
            assert!(!declare(ctx, &mut selected));
            unsafe { crate::gui_end_window(ctx) };
        });
        window_frame(ctx, [90.0, 100.0], false, |ctx| {
            assert!(!declare(ctx, &mut selected));
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        assert_eq!(selected, 0);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn bad_visible_label_prevents_selection_of_earlier_valid_row() {
        let ctx = new_context();
        let bad_bytes = [0xFFu8];
        let items = [
            GuiStr::new("valid"),
            GuiStr {
                ptr: bad_bytes.as_ptr(),
                len: bad_bytes.len(),
            },
        ];
        let mut selected = 1usize;
        let style = restricted_gui::style::Style::default();
        let m = Metrics::new();
        let inset =
            style.border_size + style.corner_rounding * (1.0 - core::f32::consts::FRAC_1_SQRT_2);
        let origin = m.content_origin();
        let at = [
            origin[0] + inset + 12.0,
            origin[1] + inset + m.row_h() * 0.5,
        ];
        let declare = |ctx: *mut GuiContext, selected: &mut usize| unsafe {
            crate::gui_list_box(
                ctx,
                b"##Items".as_ptr(),
                7,
                items.as_ptr(),
                items.len(),
                selected,
                GuiVec2 { x: 180.0, y: 100.0 },
            )
        };

        for down in [false, true, false] {
            window_frame(ctx, at, down, |ctx| {
                assert!(!declare(ctx, &mut selected));
                assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
            });
            assert_eq!(selected, 1);
        }
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
    fn rich_picker_and_spinner_exports_validate_inputs() {
        let ctx = new_context();
        let mut value = [99u8, 91, 255, 200];
        window_frame(ctx, AWAY, false, |ctx| {
            assert!(!unsafe {
                crate::gui_color_picker_rgba(ctx, b"Tint".as_ptr(), 4, value.as_mut_ptr())
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
            unsafe { crate::gui_spinner(ctx, b"Loading".as_ptr(), 7, 22.0, 0.5) };
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
            assert!(!unsafe {
                crate::gui_color_picker_rgba(ctx, b"Tint".as_ptr(), 4, ptr::null_mut())
            });
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
            unsafe { crate::gui_spinner(ctx, ptr::null(), 1, 22.0, 0.5) };
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        assert_eq!(value, [99, 91, 255, 200]);
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
