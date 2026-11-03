use crate::style_watch::{ReloadStatus, StyleWatcher};
use crate::{
    active_frame_ctx, gui_export, slice_arg, str_arg, GuiContext, GuiErrorCode, GuiRect, GuiStr,
};
use tacita::{
    id::Id,
    input::Modifiers,
    interaction::{Navigation, Shortcut, ShortcutScope},
    panels::{DockState, DockZone, SplitAxis},
    persistence::{decode_theme, encode_theme, Config, History, LayoutSnapshot, PanelState, Value},
};
pub const GUI_NAV_NEXT: u32 = 0;
pub const GUI_NAV_PREVIOUS: u32 = 1;
pub const GUI_NAV_LEFT: u32 = 2;
pub const GUI_NAV_RIGHT: u32 = 3;
pub const GUI_NAV_UP: u32 = 4;
pub const GUI_NAV_DOWN: u32 = 5;
pub const GUI_NAV_ACTIVATE: u32 = 6;
pub const GUI_NAV_CANCEL: u32 = 7;
pub const GUI_SHORTCUT_GLOBAL: u32 = 0;
pub const GUI_SHORTCUT_WINDOW: u32 = 1;
pub const GUI_SHORTCUT_POPUP: u32 = 2;
pub const GUI_MOD_CTRL: u32 = 1;
pub const GUI_MOD_SHIFT: u32 = 2;
pub const GUI_MOD_ALT: u32 = 4;
pub const GUI_VALUE_BOOL: u32 = 0;
pub const GUI_VALUE_INTEGER: u32 = 1;
pub const GUI_VALUE_NUMBER: u32 = 2;
pub const GUI_VALUE_COLOR: u32 = 3;
pub const GUI_VALUE_TEXT: u32 = 4;
pub const GUI_PANEL_RECT: u32 = 0;
pub const GUI_PANEL_SIZE: u32 = 1;
pub const GUI_PANEL_SPLIT: u32 = 2;
pub const GUI_PANEL_DOCK: u32 = 3;
pub const GUI_PANEL_SCROLL: u32 = 4;
pub const GUI_RELOAD_WAITING: u32 = 0;
pub const GUI_RELOAD_UNCHANGED: u32 = 1;
pub const GUI_RELOAD_APPLIED: u32 = 2;
pub const GUI_RELOAD_INVALID: u32 = 3;
pub const GUI_RELOAD_IO: u32 = 4;
pub const GUI_HISTORY_BEGIN: u32 = 0;
pub const GUI_HISTORY_END: u32 = 1;
pub const GUI_HISTORY_CANCEL: u32 = 2;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GuiPanelState {
    pub kind: u32,
    pub mode: u32,
    pub values: [f32; 4],
}
impl GuiPanelState {
    fn decode(self) -> Option<PanelState> {
        let v = self.values;
        let (used, state) = match (self.kind, self.mode) {
            (0, 0) => (4, PanelState::Rect(v)),
            (1, 0) => (2, PanelState::Size([v[0], v[1]])),
            (2, 0 | 1) => (
                1,
                PanelState::Split {
                    axis: if self.mode == 0 {
                        SplitAxis::Horizontal
                    } else {
                        SplitAxis::Vertical
                    },
                    ratio: v[0],
                },
            ),
            (3, 0..=2) => (
                2,
                PanelState::Dock(DockState {
                    zone: match self.mode {
                        0 => DockZone::Left,
                        1 => DockZone::Right,
                        _ => DockZone::Floating,
                    },
                    float_pos: [v[0], v[1]],
                }),
            ),
            (4, 0) => (1, PanelState::Scroll(v[0])),
            _ => return None,
        };
        (state.is_valid() && v[used..].iter().all(|v| *v == 0.)).then_some(state)
    }
}
impl From<PanelState> for GuiPanelState {
    fn from(state: PanelState) -> Self {
        let (kind, mode, values) = match state {
            PanelState::Rect(v) => (0, 0, v),
            PanelState::Size(v) => (1, 0, [v[0], v[1], 0., 0.]),
            PanelState::Split { axis, ratio } => (
                2,
                if axis == SplitAxis::Horizontal { 0 } else { 1 },
                [ratio, 0., 0., 0.],
            ),
            PanelState::Dock(v) => (3, v.zone as u32, [v.float_pos[0], v.float_pos[1], 0., 0.]),
            PanelState::Scroll(v) => (4, 0, [v, 0., 0., 0.]),
        };
        Self { kind, mode, values }
    }
}
pub(crate) struct StateBridge {
    config: Config,
    layout: LayoutSnapshot,
    history: History<Vec<u8>>,
    watch: Option<StyleWatcher>,
}
impl StateBridge {
    pub(crate) fn new() -> Self {
        let Ok(history) = History::new(Vec::new(), 64) else {
            panic!("fixed history capacity rejected");
        };
        Self {
            config: Config::new(),
            layout: LayoutSnapshot::default(),
            history,
            watch: None,
        }
    }
}
fn invalid() {
    crate::set_last_error(GuiErrorCode::InvalidInput);
}
unsafe fn live<'a>(ctx: *mut GuiContext) -> Option<&'a mut GuiContext> {
    if ctx.is_null() {
        invalid();
        None
    } else {
        Some(unsafe { &mut *ctx })
    }
}
unsafe fn idle<'a>(ctx: *mut GuiContext) -> Option<&'a mut GuiContext> {
    let gui = unsafe { live(ctx) }?;
    if gui.frame_active {
        crate::set_last_error(GuiErrorCode::ContractViolation);
        None
    } else {
        Some(gui)
    }
}
unsafe fn key<'a>(arg: GuiStr) -> Option<&'a str> {
    let text = unsafe { str_arg(arg.ptr, arg.len) };
    if text.is_none() {
        invalid();
    }
    text
}
unsafe fn bytes<'a>(data: *const u8, len: usize) -> Option<&'a [u8]> {
    if len > 2 * 1024 * 1024 {
        invalid();
        return None;
    }
    let slice = unsafe { slice_arg(data, len) };
    if slice.is_none() {
        invalid();
    }
    slice
}
unsafe fn output(data: &[u8], out: *mut u8, cap: usize) -> bool {
    if out.is_null() && cap == 0 {
        return false;
    }
    if out.is_null() || cap < data.len() || cap > isize::MAX as usize {
        invalid();
        return false;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(data.as_ptr(), out, data.len());
    }
    true
}
#[no_mangle]
pub unsafe extern "C" fn gui_navigate(ctx: *mut GuiContext, event: u32) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return false;
            };
            let event = match event {
                0 => Navigation::Next,
                1 => Navigation::Previous,
                2 => Navigation::Left,
                3 => Navigation::Right,
                4 => Navigation::Up,
                5 => Navigation::Down,
                6 => Navigation::Activate,
                7 => Navigation::Cancel,
                _ => {
                    invalid();
                    return false;
                }
            };
            if g.ctx.navigate(event) {
                true
            } else {
                invalid();
                false
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_stack_depth(ctx: *mut GuiContext, modal: u8) -> usize {
    gui_export!(
        || {
            let Some(g) = (unsafe { live(ctx) }) else {
                return 0;
            };
            if modal != 0 {
                g.ctx.modal_depth()
            } else {
                g.ctx.popup_depth()
            }
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_set_double_click_settings(
    ctx: *mut GuiContext,
    seconds: f32,
    distance: f32,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return false;
            };
            if g.ctx.set_double_click_settings(seconds, distance) {
                true
            } else {
                invalid();
                false
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_shortcut(
    ctx: *mut GuiContext,
    id: u64,
    key_code: u16,
    modifiers: u32,
    scope: u32,
    owner: u64,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            let Some(&key) = crate::ALL_KEYS.get(key_code as usize) else {
                invalid();
                return false;
            };
            if modifiers & !7 != 0 || (scope != 0 && owner == 0) {
                invalid();
                return false;
            }
            let scope = match scope {
                0 => ShortcutScope::Global,
                1 => ShortcutScope::Window(Id(owner)),
                2 => ShortcutScope::Popup(Id(owner)),
                _ => {
                    invalid();
                    return false;
                }
            };
            g.ctx.ui().shortcut(
                Id(id),
                Shortcut {
                    key,
                    modifiers: Modifiers {
                        ctrl: modifiers & 1 != 0,
                        shift: modifiers & 2 != 0,
                        alt: modifiers & 4 != 0,
                    },
                    scope,
                },
            )
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_double_clicked(
    ctx: *mut GuiContext,
    id: u64,
    rect: GuiRect,
    button: usize,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            let rect: [f32; 4] = rect.into();
            if button >= 5
                || !rect.iter().all(|v| v.is_finite())
                || rect[2] <= rect[0]
                || rect[3] <= rect[1]
            {
                invalid();
                return false;
            }
            g.ctx.ui().double_clicked(Id(id), rect, button)
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_config_register(
    ctx: *mut GuiContext,
    name: GuiStr,
    kind: u32,
    data: *const u8,
    len: usize,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return false;
            };
            let Some(name) = (unsafe { key(name) }) else {
                return false;
            };
            let Some(data) = (unsafe { bytes(data, len) }) else {
                return false;
            };
            let Ok(value) = Value::from_payload(kind, data) else {
                invalid();
                return false;
            };
            if g.state.config.register(name, value).is_err() {
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
pub unsafe extern "C" fn gui_config_set(
    ctx: *mut GuiContext,
    name: GuiStr,
    kind: u32,
    data: *const u8,
    len: usize,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { live(ctx) }) else {
                return false;
            };
            let Some(name) = (unsafe { key(name) }) else {
                return false;
            };
            let Some(data) = (unsafe { bytes(data, len) }) else {
                return false;
            };
            let Ok(value) = Value::from_payload(kind, data) else {
                invalid();
                return false;
            };
            if g.state.config.set(name, value).is_err() {
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
pub unsafe extern "C" fn gui_config_get(
    ctx: *mut GuiContext,
    name: GuiStr,
    out: *mut u8,
    cap: usize,
) -> usize {
    gui_export!(
        || {
            let Some(g) = (unsafe { live(ctx) }) else {
                return 0;
            };
            let Some(name) = (unsafe { key(name) }) else {
                return 0;
            };
            let Some(value) = g.state.config.get(name) else {
                invalid();
                return 0;
            };
            let data = value.payload();
            unsafe { output(&data, out, cap) };
            data.len()
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_config_kind(ctx: *mut GuiContext, name: GuiStr) -> u32 {
    gui_export!(
        || {
            let Some(g) = (unsafe { live(ctx) }) else {
                return u32::MAX;
            };
            let Some(name) = (unsafe { key(name) }) else {
                return u32::MAX;
            };
            g.state.config.get(name).map_or_else(
                || {
                    invalid();
                    u32::MAX
                },
                Value::kind,
            )
        },
        u32::MAX
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_save_config(ctx: *mut GuiContext, out: *mut u8, cap: usize) -> usize {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return 0;
            };
            let data = g.state.config.encode();
            unsafe { output(&data, out, cap) };
            data.len()
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_load_config(
    ctx: *mut GuiContext,
    data: *const u8,
    len: usize,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return false;
            };
            let Some(data) = (unsafe { bytes(data, len) }) else {
                return false;
            };
            if g.state.config.load(data).is_err() {
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
pub unsafe extern "C" fn gui_save_theme(ctx: *mut GuiContext, out: *mut u8, cap: usize) -> usize {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return 0;
            };
            let Ok(data) = encode_theme(g.ctx.theme_tokens()) else {
                invalid();
                return 0;
            };
            unsafe { output(data.as_bytes(), out, cap) };
            data.len()
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_load_theme(ctx: *mut GuiContext, data: *const u8, len: usize) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return false;
            };
            let Some(data) = (unsafe { bytes(data, len) }) else {
                return false;
            };
            let Ok(theme) = decode_theme(data) else {
                invalid();
                return false;
            };
            g.ctx.set_theme_tokens(theme);
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_set_layout_panel(
    ctx: *mut GuiContext,
    name: GuiStr,
    state: GuiPanelState,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { live(ctx) }) else {
                return false;
            };
            let Some(name) = (unsafe { key(name) }) else {
                return false;
            };
            let Some(state) = state.decode() else {
                invalid();
                return false;
            };
            if g.state.layout.set_panel(name, state).is_err() {
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
pub unsafe extern "C" fn gui_get_layout_panel(
    ctx: *mut GuiContext,
    name: GuiStr,
    out: *mut GuiPanelState,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { live(ctx) }) else {
                return false;
            };
            let Some(name) = (unsafe { key(name) }) else {
                return false;
            };
            if out.is_null()
                || !(out as usize).is_multiple_of(core::mem::align_of::<GuiPanelState>())
            {
                invalid();
                return false;
            }
            let Some(value) = g.state.layout.panel(name) else {
                invalid();
                return false;
            };
            unsafe {
                out.write(value.into());
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_save_layout(ctx: *mut GuiContext, out: *mut u8, cap: usize) -> usize {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return 0;
            };
            let mut layout = g.ctx.save_layout();
            layout.panels.clone_from(&g.state.layout.panels);
            let Ok(data) = layout.encode() else {
                invalid();
                return 0;
            };
            unsafe { output(&data, out, cap) };
            data.len()
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_load_layout(
    ctx: *mut GuiContext,
    data: *const u8,
    len: usize,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return false;
            };
            let Some(data) = (unsafe { bytes(data, len) }) else {
                return false;
            };
            let Ok(layout) = LayoutSnapshot::decode(data) else {
                invalid();
                return false;
            };
            if g.ctx.load_layout(&layout).is_err() {
                invalid();
                return false;
            }
            g.state.layout = layout;
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_history_reset(
    ctx: *mut GuiContext,
    data: *const u8,
    len: usize,
    capacity: usize,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return false;
            };
            let Some(data) = (unsafe { bytes(data, len) }) else {
                return false;
            };
            let Ok(history) = History::new(data.to_vec(), capacity) else {
                invalid();
                return false;
            };
            g.state.history = history;
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_history_record(
    ctx: *mut GuiContext,
    data: *const u8,
    len: usize,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { live(ctx) }) else {
                return false;
            };
            let Some(data) = (unsafe { bytes(data, len) }) else {
                return false;
            };
            g.state.history.record(data.to_vec())
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_history_can_step(ctx: *mut GuiContext, direction: i32) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { live(ctx) }) else {
                return false;
            };
            match direction {
                -1 => g.state.history.can_undo(),
                1 => g.state.history.can_redo(),
                _ => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_history_step(
    ctx: *mut GuiContext,
    direction: i32,
    out: *mut u8,
    cap: usize,
) -> usize {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return 0;
            };
            let target = match direction {
                -1 => g.state.history.undo_target(),
                1 => g.state.history.redo_target(),
                _ => {
                    invalid();
                    return 0;
                }
            };
            let Some(target) = target else {
                return 0;
            };
            let len = target.len();
            if unsafe { output(target, out, cap) } {
                if direction < 0 {
                    g.state.history.undo();
                } else {
                    g.state.history.redo();
                }
            }
            len
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_history_group(ctx: *mut GuiContext, action: u32) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { live(ctx) }) else {
                return false;
            };
            let grouped = g.state.history.is_grouping();
            match action {
                0 if !grouped => g.state.history.begin_group(),
                1 if grouped => {
                    g.state.history.end_group();
                }
                2 if grouped => g.state.history.cancel_group(),
                _ => {
                    crate::set_last_error(GuiErrorCode::ContractViolation);
                    return false;
                }
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_watch_style(
    ctx: *mut GuiContext,
    path: GuiStr,
    interval_ms: u32,
) -> bool {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return false;
            };
            let Some(path) = (unsafe { key(path) }) else {
                return false;
            };
            if path.is_empty() || path.contains('\0') || !(50..=60000).contains(&interval_ms) {
                invalid();
                return false;
            }
            g.state.watch = Some(StyleWatcher::new(
                path,
                std::time::Duration::from_millis(interval_ms as u64),
            ));
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_unwatch_style(ctx: *mut GuiContext) {
    gui_export!(
        || {
            if let Some(g) = unsafe { idle(ctx) } {
                g.state.watch = None;
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_poll_style(ctx: *mut GuiContext) -> u32 {
    gui_export!(
        || {
            let Some(g) = (unsafe { idle(ctx) }) else {
                return 3;
            };
            let Some(watch) = &mut g.state.watch else {
                return 0;
            };
            if watch.poll_apply(&mut g.ctx) {
                return 2;
            }
            match watch.status() {
                ReloadStatus::Waiting => 0,
                ReloadStatus::Unchanged | ReloadStatus::Reloaded => 1,
                ReloadStatus::Invalid(_) => 3,
                ReloadStatus::Io(_) => 4,
            }
        },
        3
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn name(s: &str) -> GuiStr {
        GuiStr {
            ptr: s.as_ptr(),
            len: s.len(),
        }
    }
    #[test]
    fn snapshots_and_history_are_transactional_at_the_c_boundary() {
        unsafe {
            let c = crate::test_support::new_context();
            let one = [1u8];
            let zero = [0u8];
            assert!(gui_config_register(c, name("enabled"), 0, one.as_ptr(), 1));
            let n = gui_save_config(c, core::ptr::null_mut(), 0);
            let mut blob = vec![0; n];
            assert_eq!(gui_save_config(c, blob.as_mut_ptr(), n), n);
            assert!(gui_config_set(c, name("enabled"), 0, zero.as_ptr(), 1));
            assert!(!gui_load_config(c, blob.as_ptr(), n - 1));
            let mut value = [7u8];
            gui_config_get(c, name("enabled"), value.as_mut_ptr(), 1);
            assert_eq!(value, [0]);
            assert!(gui_load_config(c, blob.as_ptr(), n));
            gui_config_get(c, name("enabled"), value.as_mut_ptr(), 1);
            assert_eq!(value, [1]);
            assert!(gui_history_reset(c, one.as_ptr(), 1, 4));
            assert!(gui_history_record(c, zero.as_ptr(), 1));
            assert_eq!(gui_history_step(c, -1, core::ptr::null_mut(), 0), 1);
            assert!(gui_history_can_step(c, -1));
            assert_eq!(gui_history_step(c, -1, value.as_mut_ptr(), 1), 1);
            assert_eq!(value, [1]);
            assert!(gui_history_can_step(c, 1));
            let bad = [1u8, 2];
            assert!(!gui_config_set(c, name("enabled"), 0, bad.as_ptr(), 2));
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert!(!gui_navigate(c, 999));
            assert!(!gui_set_double_click_settings(c, f32::NAN, 4.));
            let n = gui_save_theme(c, core::ptr::null_mut(), 0);
            let mut text = vec![0; n];
            gui_save_theme(c, text.as_mut_ptr(), n);
            assert!(gui_load_theme(c, text.as_ptr(), n));
            let p = GuiPanelState {
                kind: 2,
                mode: 0,
                values: [0.4, 0., 0., 0.],
            };
            assert!(gui_set_layout_panel(c, name("split"), p));
            let mut output = p;
            assert!(gui_get_layout_panel(c, name("split"), &mut output));
            assert_eq!(output.values, p.values);
            crate::gui_context_free(c);
        }
    }
    #[test]
    fn panel_abi() {
        assert_eq!(core::mem::size_of::<GuiPanelState>(), 24);
        assert_eq!(core::mem::align_of::<GuiPanelState>(), 4);
        assert_eq!(core::mem::offset_of!(GuiPanelState, values), 8);
    }
    #[test]
    fn nested_c_scopes_and_invalid_outputs_preserve_state() {
        unsafe {
            let c = crate::test_support::new_context();
            let input = crate::test_support::input_at([-1000.; 2], false);
            crate::gui_new_frame(c, &input, 0.016);
            crate::gui_begin_window(c, b"W".as_ptr(), 1, 0);
            crate::gui_open_modal(c, 100);
            assert!(crate::gui_begin_modal(
                c,
                100,
                GuiRect {
                    min: [100., 100.].into(),
                    max: [300., 300.].into()
                }
            ));
            crate::gui_open_popup(c, 101);
            assert!(crate::gui_begin_popup(
                c,
                101,
                GuiRect {
                    min: [350., 200.].into(),
                    max: [550., 400.].into()
                }
            ));
            assert_eq!(gui_stack_depth(c, 0), 2);
            assert_eq!(gui_stack_depth(c, 1), 1);
            crate::gui_end_popup(c);
            crate::gui_end_popup(c);
            crate::gui_end_window(c);
            crate::gui_render(c);
            let state = GuiPanelState {
                kind: 1,
                mode: 0,
                values: [200., 120., 0., 0.],
            };
            assert!(gui_set_layout_panel(c, name("size"), state));
            let mut untouched = [0xABu8; 32];
            assert!(!gui_get_layout_panel(
                c,
                name("size"),
                untouched.as_mut_ptr().add(1).cast()
            ));
            assert_eq!(untouched, [0xAB; 32]);
            let count = gui_save_layout(c, core::ptr::null_mut(), 0);
            assert!(count > 32);
            assert_eq!(gui_save_layout(c, untouched.as_mut_ptr(), 32), count);
            assert_eq!(untouched, [0xAB; 32]);
            let mut bytes = vec![0; count];
            gui_save_layout(c, bytes.as_mut_ptr(), count);
            assert!(gui_load_layout(c, bytes.as_ptr(), count));
            assert_eq!(gui_stack_depth(c, 0), 0);
            assert!(gui_history_group(c, 0));
            assert!(!gui_history_group(c, 0));
            assert!(gui_history_group(c, 2));
            crate::gui_context_free(c);
        }
    }
}
