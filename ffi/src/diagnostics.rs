use crate::{gui_export, slice_arg, translate_key_event, GuiContext, GuiErrorCode, GuiInput};
use tacita::{
    diagnostics::{InputRecording, InspectionNode, ReplayCursor},
    display::DisplayMetrics,
    input::InputState,
    interaction::Navigation,
};
pub struct GuiRecording {
    recording: InputRecording,
    cursor: ReplayCursor,
    bytes: Vec<u8>,
}
fn invalid() {
    crate::set_last_error(GuiErrorCode::InvalidInput);
}
fn nav(value: u32) -> Option<Navigation> {
    match value {
        0 => Some(Navigation::Next),
        1 => Some(Navigation::Previous),
        2 => Some(Navigation::Left),
        3 => Some(Navigation::Right),
        4 => Some(Navigation::Up),
        5 => Some(Navigation::Down),
        6 => Some(Navigation::Activate),
        7 => Some(Navigation::Cancel),
        _ => None,
    }
}
#[no_mangle]
pub unsafe extern "C" fn gui_recording_new() -> *mut GuiRecording {
    gui_export!(
        || Box::into_raw(Box::new(GuiRecording {
            recording: InputRecording::new(),
            cursor: ReplayCursor::default(),
            bytes: Vec::new()
        })),
        core::ptr::null_mut()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_recording_free(handle: *mut GuiRecording) {
    gui_export!(
        || {
            if !handle.is_null() {
                drop(unsafe { Box::from_raw(handle) });
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_recording_push(
    handle: *mut GuiRecording,
    input: *const GuiInput,
    dt: f32,
    navigation: *const u32,
    count: usize,
) -> bool {
    gui_export!(
        || {
            if handle.is_null() || input.is_null() {
                invalid();
                return false;
            }
            let input = unsafe { &*input };
            if input.key_count > 1024 || count > 64 {
                invalid();
                return false;
            }
            let (Some(events), Some(navigation)) =
                (unsafe { slice_arg(input.keys, input.key_count) }, unsafe {
                    slice_arg(navigation, count)
                })
            else {
                invalid();
                return false;
            };
            let Some(keys) = events
                .iter()
                .map(translate_key_event)
                .collect::<Option<Vec<_>>>()
            else {
                invalid();
                return false;
            };
            let Some(navigation) = navigation
                .iter()
                .copied()
                .map(nav)
                .collect::<Option<Vec<_>>>()
            else {
                invalid();
                return false;
            };
            let input = InputState {
                mouse_pos: input.mouse_pos,
                mouse_down: input.mouse_down.map(|v| v != 0),
                mouse_wheel: input.mouse_wheel,
                viewport_size: input.viewport_size,
                keys: &keys,
            };
            if unsafe { &mut *handle }
                .recording
                .push(input, dt, &navigation)
                .is_err()
            {
                invalid();
                false
            } else {
                true
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_recording_save(
    handle: *mut GuiRecording,
    out: *mut u8,
    capacity: usize,
) -> usize {
    gui_export!(
        || {
            if handle.is_null() {
                invalid();
                return 0;
            }
            let h = unsafe { &mut *handle };
            h.recording.encode(&mut h.bytes);
            let len = h.bytes.len();
            if capacity >= len && !out.is_null() {
                unsafe {
                    core::ptr::copy_nonoverlapping(h.bytes.as_ptr(), out, len);
                }
            }
            len
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_recording_load(
    handle: *mut GuiRecording,
    data: *const u8,
    len: usize,
) -> bool {
    gui_export!(
        || {
            if handle.is_null() || len > tacita::diagnostics::MAX_RECORDING_BYTES {
                invalid();
                return false;
            }
            let Some(bytes) = (unsafe { slice_arg(data, len) }) else {
                invalid();
                return false;
            };
            let Ok(recording) = InputRecording::decode(bytes) else {
                invalid();
                return false;
            };
            let h = unsafe { &mut *handle };
            h.recording = recording;
            h.cursor.restart();
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_recording_len(handle: *const GuiRecording) -> usize {
    gui_export!(
        || {
            if handle.is_null() {
                invalid();
                0
            } else {
                unsafe { &*handle }.recording.len()
            }
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_recording_seek(handle: *mut GuiRecording, index: usize) -> bool {
    gui_export!(
        || {
            if handle.is_null() {
                invalid();
                return false;
            }
            let h = unsafe { &mut *handle };
            if h.cursor.seek(&h.recording, index).is_err() {
                invalid();
                false
            } else {
                true
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_replay_next(ctx: *mut GuiContext, handle: *mut GuiRecording) -> bool {
    gui_export!(
        || {
            if ctx.is_null() || handle.is_null() {
                invalid();
                return false;
            }
            let gui = unsafe { &mut *ctx };
            if gui.frame_active {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            let h = unsafe { &mut *handle };
            let Some(frame) = h.recording.frame(h.cursor.position()) else {
                return false;
            };
            gui.window = crate::WindowState::None;
            gui.containers.clear();
            gui.last_effects = core::ptr::null();
            gui.last_effect_count = 0;
            gui.font_scope_depth = 0;
            gui.ctx.replay_frame(frame);
            gui.frame_active = true;
            let _ = h.cursor.next(&h.recording);
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_update_display_metrics(
    ctx: *mut GuiContext,
    width: u32,
    height: u32,
    scale: f32,
) -> bool {
    gui_export!(
        || {
            if ctx.is_null() {
                invalid();
                return false;
            }
            let gui = unsafe { &mut *ctx };
            if gui.frame_active {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            match gui.ctx.update_display_metrics(DisplayMetrics {
                physical_size: [width, height],
                scale,
            }) {
                Ok(_) => true,
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
pub unsafe extern "C" fn gui_set_inspection_enabled(ctx: *mut GuiContext, enabled: bool) -> bool {
    gui_export!(
        || {
            if ctx.is_null() {
                invalid();
                return false;
            }
            let gui = unsafe { &mut *ctx };
            if gui.frame_active {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            gui.ctx.set_inspection_enabled(enabled);
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_inspection_count(ctx: *const GuiContext) -> usize {
    gui_export!(
        || {
            if ctx.is_null() {
                invalid();
                0
            } else {
                unsafe { &*ctx }.ctx.inspection_count()
            }
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_inspection_node(
    ctx: *const GuiContext,
    index: usize,
    out: *mut InspectionNode,
) -> bool {
    gui_export!(
        || {
            if ctx.is_null() || out.is_null() {
                invalid();
                return false;
            }
            let Some(node) = unsafe { &*ctx }.ctx.inspection_node(index) else {
                invalid();
                return false;
            };
            unsafe {
                *out = node;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_inspection_name(
    ctx: *const GuiContext,
    id: u64,
    out: *mut u8,
    capacity: usize,
) -> usize {
    gui_export!(
        || {
            if ctx.is_null() {
                invalid();
                return 0;
            }
            let mut name = String::new();
            unsafe { &*ctx }.ctx.inspection_name(id, &mut name);
            if !out.is_null() && capacity >= name.len() {
                unsafe {
                    core::ptr::copy_nonoverlapping(name.as_ptr(), out, name.len());
                }
            }
            name.len()
        },
        0
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_boundary_round_trip_and_replay() {
        unsafe {
            let h = gui_recording_new();
            let input = GuiInput {
                mouse_pos: [3., 4.],
                mouse_down: [0; 5],
                mouse_wheel: 0.,
                keys: core::ptr::null(),
                key_count: 0,
                viewport_size: [900., 700.],
            };
            assert!(gui_recording_push(h, &input, 0.02, core::ptr::null(), 0));
            let len = gui_recording_save(h, core::ptr::null_mut(), 0);
            let mut bytes = vec![0; len];
            assert_eq!(gui_recording_save(h, bytes.as_mut_ptr(), len), len);
            assert!(!gui_recording_load(h, bytes.as_ptr(), len - 1));
            assert_eq!(gui_recording_len(h), 1);
            assert!(gui_recording_load(h, bytes.as_ptr(), len));
            let font = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
            let ctx = crate::gui_context_new(font.as_ptr(), font.len());
            assert!(gui_replay_next(ctx, h));
            crate::gui_render(ctx);
            assert!(!gui_replay_next(ctx, h));
            assert!(gui_recording_seek(h, 0));
            assert!(gui_replay_next(ctx, h));
            crate::gui_render(ctx);
            crate::gui_context_free(ctx);
            gui_recording_free(h);
        }
    }
}
