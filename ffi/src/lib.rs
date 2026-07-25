use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use restricted_gui::context::Context;
use restricted_gui::input::{InputState, Key, KeyEvent};
use restricted_gui::ui::{CollapsingHeaderScope, ScrollRegionScope};

mod canvas;
mod custom;
mod types;
mod widgets;
mod windows;

#[cfg(test)]
mod header_consistency;
#[cfg(test)]
mod test_support;

pub use canvas::*;
pub use custom::*;
pub use types::*;
pub use widgets::*;
pub use windows::*;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiErrorCode {
    None = 0,
    ContractViolation = 1,
    Panic = 2,
    InvalidInput = 3,
}

thread_local! {
    static LAST_ERROR: Cell<u8> = const { Cell::new(GuiErrorCode::None as u8) };
}

fn set_last_error(code: GuiErrorCode) {
    let _ = LAST_ERROR.try_with(|e| e.set(code as u8));
}

#[no_mangle]
pub extern "C" fn gui_last_error_code() -> u8 {
    catch_unwind(|| LAST_ERROR.with(Cell::get)).unwrap_or(GuiErrorCode::Panic as u8)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiVert {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub col: [u8; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiCmd {
    pub elem_count: u32,
    pub clip_rect: [f32; 4],
    pub texture: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiDrawData {
    pub vertices: *const GuiVert,
    pub vertex_count: u32,
    pub indices: *const u32,
    pub index_count: u32,
    pub commands: *const GuiCmd,
    pub command_count: u32,
}

impl GuiDrawData {
    const ZERO: Self = Self {
        vertices: ptr::null(),
        vertex_count: 0,
        indices: ptr::null(),
        index_count: 0,
        commands: ptr::null(),
        command_count: 0,
    };
}

pub const GUI_KEY_EVENT_PRESS: u8 = 0;
pub const GUI_KEY_EVENT_RELEASE: u8 = 1;
pub const GUI_KEY_EVENT_CHAR: u8 = 2;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiKeyEvent {
    pub kind: u8,
    pub key: u16,
    pub ch: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiInput {
    pub mouse_pos: [f32; 2],
    // u8, not bool: the host bulk-fills this struct and uninitialized bytes are a lifestyle. Any nonzero byte is "held". The bool-materialization UB goblin stays in its cage.
    pub mouse_down: [u8; 3],
    pub mouse_wheel: f32,
    pub viewport_size: [f32; 2],
    pub keys: *const GuiKeyEvent,
    pub key_count: usize,
}

const ALL_KEYS: [Key; 62] = [
    Key::Backspace,
    Key::Delete,
    Key::Left,
    Key::Right,
    Key::Home,
    Key::End,
    Key::Enter,
    Key::Escape,
    Key::Insert,
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
    Key::Num0,
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
    Key::Space,
    Key::Tab,
    Key::Shift,
    Key::Ctrl,
    Key::Alt,
];

pub(crate) fn key_to_code(key: Key) -> u16 {
    match key {
        Key::Backspace => 0,
        Key::Delete => 1,
        Key::Left => 2,
        Key::Right => 3,
        Key::Home => 4,
        Key::End => 5,
        Key::Enter => 6,
        Key::Escape => 7,
        Key::Insert => 8,
        Key::A => 9,
        Key::B => 10,
        Key::C => 11,
        Key::D => 12,
        Key::E => 13,
        Key::F => 14,
        Key::G => 15,
        Key::H => 16,
        Key::I => 17,
        Key::J => 18,
        Key::K => 19,
        Key::L => 20,
        Key::M => 21,
        Key::N => 22,
        Key::O => 23,
        Key::P => 24,
        Key::Q => 25,
        Key::R => 26,
        Key::S => 27,
        Key::T => 28,
        Key::U => 29,
        Key::V => 30,
        Key::W => 31,
        Key::X => 32,
        Key::Y => 33,
        Key::Z => 34,
        Key::Num0 => 35,
        Key::Num1 => 36,
        Key::Num2 => 37,
        Key::Num3 => 38,
        Key::Num4 => 39,
        Key::Num5 => 40,
        Key::Num6 => 41,
        Key::Num7 => 42,
        Key::Num8 => 43,
        Key::Num9 => 44,
        Key::F1 => 45,
        Key::F2 => 46,
        Key::F3 => 47,
        Key::F4 => 48,
        Key::F5 => 49,
        Key::F6 => 50,
        Key::F7 => 51,
        Key::F8 => 52,
        Key::F9 => 53,
        Key::F10 => 54,
        Key::F11 => 55,
        Key::F12 => 56,
        Key::Space => 57,
        Key::Tab => 58,
        Key::Shift => 59,
        Key::Ctrl => 60,
        Key::Alt => 61,
    }
}

fn key_from_code(code: u16) -> Option<Key> {
    ALL_KEYS.get(code as usize).copied()
}

fn translate_key_event(event: &GuiKeyEvent) -> Option<KeyEvent> {
    match event.kind {
        GUI_KEY_EVENT_PRESS => key_from_code(event.key).map(KeyEvent::Press),
        GUI_KEY_EVENT_RELEASE => key_from_code(event.key).map(KeyEvent::Release),
        GUI_KEY_EVENT_CHAR => char::from_u32(event.ch).map(KeyEvent::Char),
        _ => None,
    }
}

// catch_unwind around every export: panics never cross the FFI boundary. What happens in Rust stays in Rust.
macro_rules! gui_export {
    ($body:expr, $fallback:expr) => {{
        $crate::set_last_error($crate::GuiErrorCode::None);
        match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe($body)) {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(_) => {
                $crate::set_last_error($crate::GuiErrorCode::Panic);
                $fallback
            }
        }
    }};
}
pub(crate) use gui_export;

pub(crate) unsafe fn str_arg<'a>(ptr: *const u8, len: usize) -> Option<&'a str> {
    if len == 0 {
        return Some("");
    }
    if ptr.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    std::str::from_utf8(bytes).ok()
}

pub(crate) unsafe fn active_frame_ctx<'a>(ctx: *mut GuiContext) -> Option<&'a mut GuiContext> {
    if ctx.is_null() {
        set_last_error(GuiErrorCode::InvalidInput);
        return None;
    }
    let gui = unsafe { &mut *ctx };
    if !gui.frame_active {
        set_last_error(GuiErrorCode::ContractViolation);
        return None;
    }
    Some(gui)
}

pub struct GuiContext {
    pub(crate) ctx: Context,
    pub(crate) frame_active: bool,
    last_draw: GuiDrawData,
    commands_staging: Vec<GuiCmd>,
    pub(crate) window: WindowState,
    pub(crate) header_scopes: Vec<Option<CollapsingHeaderScope>>,
    pub(crate) scroll_scopes: Vec<ScrollRegionScope>,
    pub(crate) tab_bar_depth: usize,
}

pub(crate) enum WindowState {
    None,
    Open,
    Closed,
}

#[no_mangle]
pub unsafe extern "C" fn gui_context_new(font_bytes: *const u8, len: usize) -> *mut GuiContext {
    set_last_error(GuiErrorCode::None);
    catch_unwind(AssertUnwindSafe(|| {
        if font_bytes.is_null() || len == 0 {
            set_last_error(GuiErrorCode::InvalidInput);
            return ptr::null_mut();
        }
        let bytes = unsafe { std::slice::from_raw_parts(font_bytes, len) };
        match Context::new(bytes) {
            Ok(ctx) => Box::into_raw(Box::new(GuiContext {
                ctx,
                frame_active: false,
                last_draw: GuiDrawData::ZERO,
                commands_staging: Vec::new(),
                window: WindowState::None,
                header_scopes: Vec::new(),
                scroll_scopes: Vec::new(),
                tab_bar_depth: 0,
            })),
            Err(_) => {
                set_last_error(GuiErrorCode::InvalidInput);
                ptr::null_mut()
            }
        }
    }))
    .unwrap_or_else(|_| {
        set_last_error(GuiErrorCode::Panic);
        ptr::null_mut()
    })
}

#[no_mangle]
pub unsafe extern "C" fn gui_context_free(ctx: *mut GuiContext) {
    set_last_error(GuiErrorCode::None);
    let result = catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() {
            return;
        }
        drop(unsafe { Box::from_raw(ctx) });
    }));
    if result.is_err() {
        set_last_error(GuiErrorCode::Panic);
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_new_frame(ctx: *mut GuiContext, input: *const GuiInput, dt: f32) {
    set_last_error(GuiErrorCode::None);
    let result = catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() {
            set_last_error(GuiErrorCode::InvalidInput);
            return;
        }
        let gui = unsafe { &mut *ctx };
        gui.window = WindowState::None;
        gui.header_scopes.clear();
        gui.scroll_scopes.clear();
        gui.tab_bar_depth = 0;
        let mut key_events: Vec<KeyEvent> = Vec::new();
        let state = if input.is_null() {
            InputState::default()
        } else {
            let raw = unsafe { &*input };
            if !raw.keys.is_null() && raw.key_count > 0 {
                let events = unsafe { std::slice::from_raw_parts(raw.keys, raw.key_count) };
                key_events.extend(events.iter().filter_map(translate_key_event));
            }
            InputState {
                mouse_pos: raw.mouse_pos,
                mouse_down: raw.mouse_down.map(|b| b != 0),
                mouse_wheel: raw.mouse_wheel,
                keys: &key_events,
                viewport_size: raw.viewport_size,
            }
        };
        gui.ctx.set_input(state);
        gui.ctx.new_frame(dt);
        gui.frame_active = true;
    }));
    if result.is_err() {
        set_last_error(GuiErrorCode::Panic);
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_render(ctx: *mut GuiContext) -> GuiDrawData {
    set_last_error(GuiErrorCode::None);
    match catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() {
            set_last_error(GuiErrorCode::InvalidInput);
            return GuiDrawData::ZERO;
        }
        let gui = unsafe { &mut *ctx };
        if gui.frame_active {
            let unbalanced = !matches!(gui.window, WindowState::None)
                || !gui.header_scopes.is_empty()
                || !gui.scroll_scopes.is_empty()
                || gui.tab_bar_depth > 0;
            if unbalanced {
                while gui.tab_bar_depth > 0 {
                    gui.tab_bar_depth -= 1;
                    gui.ctx.ui().end_tab_bar();
                }
                if matches!(gui.window, WindowState::Open) {
                    while let Some(scope) = gui.header_scopes.pop() {
                        if let Some(scope) = scope {
                            gui.ctx.ui().end_collapsing_header(scope);
                        }
                    }
                    while let Some(scope) = gui.scroll_scopes.pop() {
                        gui.ctx.ui().end_scroll_region(scope);
                    }
                    gui.ctx.ui().end_window();
                } else {
                    gui.header_scopes.clear();
                    gui.scroll_scopes.clear();
                }
                gui.window = WindowState::None;
            }
            let data = gui.ctx.render();
            gui.frame_active = false;
            gui.commands_staging.clear();
            gui.commands_staging
                .extend(data.commands.iter().map(|cmd| GuiCmd {
                    elem_count: cmd.elem_count,
                    clip_rect: cmd.clip_rect,
                    texture: cmd.texture.0,
                }));
            gui.last_draw = GuiDrawData {
                vertices: data.vertices.as_ptr().cast::<GuiVert>(),
                vertex_count: data.vertices.len() as u32,
                indices: data.indices.as_ptr(),
                index_count: data.indices.len() as u32,
                commands: gui.commands_staging.as_ptr(),
                command_count: gui.commands_staging.len() as u32,
            };
            if unbalanced {
                set_last_error(GuiErrorCode::ContractViolation);
            }
        }
        gui.last_draw
    })) {
        Ok(draw) => draw,
        Err(_) => {
            set_last_error(GuiErrorCode::Panic);
            GuiDrawData::ZERO
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_atlas_pixels(
    ctx: *mut GuiContext,
    w: *mut u32,
    h: *mut u32,
) -> *const u8 {
    set_last_error(GuiErrorCode::None);
    match catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() || w.is_null() || h.is_null() {
            set_last_error(GuiErrorCode::InvalidInput);
            return ptr::null();
        }
        let gui = unsafe { &*ctx };
        let (pixels, atlas_w, atlas_h) = gui.ctx.font().atlas_pixels();
        unsafe {
            *w = atlas_w;
            *h = atlas_h;
        }
        pixels.as_ptr()
    })) {
        Ok(pixels) => pixels,
        Err(_) => {
            set_last_error(GuiErrorCode::Panic);
            ptr::null()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_atlas_generation(ctx: *mut GuiContext) -> u64 {
    set_last_error(GuiErrorCode::None);
    match catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() {
            set_last_error(GuiErrorCode::InvalidInput);
            return 0;
        }
        let gui = unsafe { &*ctx };
        gui.ctx.font().atlas_generation()
    })) {
        Ok(generation) => generation,
        Err(_) => {
            set_last_error(GuiErrorCode::Panic);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use restricted_gui::context::Context;
    use restricted_gui::draw::{DrawVert, TextureId};

    const DEJAVU_SANS: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

    fn new_context() -> *mut GuiContext {
        let ctx = unsafe { gui_context_new(DEJAVU_SANS.as_ptr(), DEJAVU_SANS.len()) };
        assert!(!ctx.is_null(), "valid font must produce a context");
        ctx
    }

    fn neutral_input() -> GuiInput {
        GuiInput {
            mouse_pos: [0.0; 2],
            mouse_down: [0; 3],
            mouse_wheel: 0.0,
            viewport_size: [800.0, 600.0],
            keys: std::ptr::null(),
            key_count: 0,
        }
    }

    #[test]
    fn null_font_ptr_returns_null_and_reports_invalid_input() {
        let ctx = unsafe { gui_context_new(std::ptr::null(), 0) };
        assert!(ctx.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
    }

    #[test]
    fn garbage_font_bytes_return_null_and_report_invalid_input() {
        let garbage = [0xABu8; 64];
        let ctx = unsafe { gui_context_new(garbage.as_ptr(), garbage.len()) };
        assert!(ctx.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
    }

    #[test]
    fn valid_font_produces_a_context_and_free_is_safe() {
        let ctx = new_context();
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
        unsafe { gui_context_free(std::ptr::null_mut()) };
    }

    #[test]
    fn atlas_accessors_match_core_values() {
        let reference = Context::new(DEJAVU_SANS).expect("reference context");
        let (ref_pixels, ref_w, ref_h) = reference.font().atlas_pixels();

        let ctx = new_context();
        let mut w = 0u32;
        let mut h = 0u32;
        let pixels = unsafe { gui_atlas_pixels(ctx, &mut w, &mut h) };
        assert!(!pixels.is_null());
        assert_eq!((w, h), (ref_w, ref_h));
        let ffi_pixels = unsafe { std::slice::from_raw_parts(pixels, (w * h) as usize) };
        assert_eq!(ffi_pixels, ref_pixels);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let generation = unsafe { gui_atlas_generation(ctx) };
        assert_eq!(generation, reference.font().atlas_generation());
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn atlas_null_arguments_report_invalid_input() {
        let ctx = new_context();
        let mut w = 0u32;
        let mut h = 0u32;
        assert!(unsafe { gui_atlas_pixels(ctx, &mut w, std::ptr::null_mut()) }.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        assert!(unsafe { gui_atlas_pixels(std::ptr::null_mut(), &mut w, &mut h) }.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        assert_eq!(unsafe { gui_atlas_generation(std::ptr::null_mut()) }, 0);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn render_after_new_frame_yields_geometry_and_stable_slices() {
        let ctx = new_context();
        let input = neutral_input();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe {
            (&mut *ctx)
                .ctx
                .ui()
                .window("ffi", |ui| ui.label("hello ffi"))
        };

        let first = unsafe { gui_render(ctx) };
        assert!(first.vertex_count > 0);
        assert!(first.index_count > 0);
        assert!(first.command_count > 0);
        assert!(!first.vertices.is_null());
        assert!(!first.indices.is_null());
        assert!(!first.commands.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let second = unsafe { gui_render(ctx) };
        assert_eq!(first.vertices, second.vertices);
        assert_eq!(first.indices, second.indices);
        assert_eq!(first.commands, second.commands);
        assert_eq!(first.vertex_count, second.vertex_count);
        assert_eq!(first.index_count, second.index_count);
        assert_eq!(first.command_count, second.command_count);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn null_input_means_neutral_input() {
        let ctx = new_context();
        unsafe { gui_new_frame(ctx, std::ptr::null(), 0.016) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        let data = unsafe { gui_render(ctx) };
        assert_eq!(data.vertex_count, 0);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn null_context_calls_report_invalid_input() {
        let input = neutral_input();
        unsafe { gui_new_frame(std::ptr::null_mut(), &input, 0.016) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        let data = unsafe { gui_render(std::ptr::null_mut()) };
        assert!(data.vertices.is_null());
        assert_eq!(data.vertex_count, 0);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
    }

    #[test]
    fn panic_is_contained_reported_and_the_host_continues() {
        let ctx = new_context();
        let input = neutral_input();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        let _ = std::panic::take_hook();
        std::panic::set_hook(hook);

        assert_eq!(gui_last_error_code(), GuiErrorCode::Panic as u8);

        let data = unsafe { gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        let _ = data;
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        let data = unsafe { gui_render(ctx) };
        let _ = data;
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn last_error_code_is_always_callable_without_unwinding() {
        let code = std::thread::spawn(|| gui_last_error_code())
            .join()
            .expect("reader thread must not panic");
        assert_eq!(code, GuiErrorCode::None as u8);
        let ctx = unsafe { gui_context_new(std::ptr::null(), 0) };
        assert!(ctx.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
    }

    #[test]
    fn every_key_round_trips_through_u16() {
        for (code, key) in ALL_KEYS.iter().enumerate() {
            let code = code as u16;
            assert_eq!(key_to_code(*key), code, "wrong code for {key:?}");
            assert_eq!(
                key_from_code(code),
                Some(*key),
                "code {code} does not map back"
            );
        }
        assert_eq!(key_to_code(Key::Alt) as usize + 1, ALL_KEYS.len());
        assert_eq!(key_from_code(ALL_KEYS.len() as u16), None);
        assert_eq!(key_from_code(u16::MAX), None);
    }

    #[test]
    fn key_event_translation_accepts_valid_and_ignores_invalid() {
        let press = GuiKeyEvent {
            kind: GUI_KEY_EVENT_PRESS,
            key: 0,
            ch: 0,
        };
        assert_eq!(
            translate_key_event(&press),
            Some(KeyEvent::Press(Key::Backspace))
        );
        let release = GuiKeyEvent {
            kind: GUI_KEY_EVENT_RELEASE,
            key: 61,
            ch: 0,
        };
        assert_eq!(
            translate_key_event(&release),
            Some(KeyEvent::Release(Key::Alt))
        );
        let character = GuiKeyEvent {
            kind: GUI_KEY_EVENT_CHAR,
            key: 0,
            ch: 'x' as u32,
        };
        assert_eq!(translate_key_event(&character), Some(KeyEvent::Char('x')));
        let bad_key = GuiKeyEvent {
            kind: GUI_KEY_EVENT_PRESS,
            key: 62,
            ch: 0,
        };
        assert_eq!(translate_key_event(&bad_key), None);
        let bad_kind = GuiKeyEvent {
            kind: 3,
            key: 0,
            ch: 0,
        };
        assert_eq!(translate_key_event(&bad_kind), None);
        let bad_char = GuiKeyEvent {
            kind: GUI_KEY_EVENT_CHAR,
            key: 0,
            ch: u32::MAX,
        };
        assert_eq!(translate_key_event(&bad_char), None);
    }

    #[test]
    fn new_frame_accepts_a_full_key_stream() {
        let ctx = new_context();
        let mut events = Vec::new();
        for code in 0..62u16 {
            events.push(GuiKeyEvent {
                kind: GUI_KEY_EVENT_PRESS,
                key: code,
                ch: 0,
            });
            events.push(GuiKeyEvent {
                kind: GUI_KEY_EVENT_RELEASE,
                key: code,
                ch: 0,
            });
        }
        events.push(GuiKeyEvent {
            kind: GUI_KEY_EVENT_CHAR,
            key: 0,
            ch: 'A' as u32,
        });
        events.push(GuiKeyEvent {
            kind: 7,
            key: 999,
            ch: u32::MAX,
        });
        let mut input = neutral_input();
        input.keys = events.as_ptr();
        input.key_count = events.len();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        let data = unsafe { gui_render(ctx) };
        let _ = data;
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn ffi_struct_layouts_match_core_layouts() {
        use std::mem::{align_of, offset_of, size_of};
        assert_eq!(size_of::<GuiVert>(), size_of::<DrawVert>());
        assert_eq!(align_of::<GuiVert>(), align_of::<DrawVert>());
        assert_eq!(offset_of!(GuiVert, pos), offset_of!(DrawVert, pos));
        assert_eq!(offset_of!(GuiVert, uv), offset_of!(DrawVert, uv));
        assert_eq!(offset_of!(GuiVert, col), offset_of!(DrawVert, col));
        assert_eq!(size_of::<TextureId>(), size_of::<u64>());
        assert_eq!(align_of::<TextureId>(), align_of::<u64>());
    }
}
