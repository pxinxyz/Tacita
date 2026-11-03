#![cfg(windows)]
#![forbid(unsafe_code)]

use restricted_gui::input::{InputState, Key, KeyEvent};

const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_CHAR: u32 = 0x0102;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSKEYUP: u32 = 0x0105;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_LBUTTONDBLCLK: u32 = 0x0203;
const WM_RBUTTONDOWN: u32 = 0x0204;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_RBUTTONDBLCLK: u32 = 0x0206;
const WM_MBUTTONDOWN: u32 = 0x0207;
const WM_MBUTTONUP: u32 = 0x0208;
const WM_MBUTTONDBLCLK: u32 = 0x0209;
const WM_MOUSEWHEEL: u32 = 0x020A;
const WM_XBUTTONDOWN: u32 = 0x020B;
const WM_XBUTTONUP: u32 = 0x020C;
const WM_XBUTTONDBLCLK: u32 = 0x020D;
const WM_SIZE: u32 = 0x0005;

const XBUTTON1: u16 = 0x0001;
const XBUTTON2: u16 = 0x0002;

const WHEEL_DELTA: i16 = 120;

const VK_BACK: u16 = 0x08;
const VK_TAB: u16 = 0x09;
const VK_RETURN: u16 = 0x0D;
const VK_SHIFT: u16 = 0x10;
const VK_CONTROL: u16 = 0x11;
const VK_MENU: u16 = 0x12;
const VK_ESCAPE: u16 = 0x1B;
const VK_SPACE: u16 = 0x20;
const VK_END: u16 = 0x23;
const VK_HOME: u16 = 0x24;
const VK_LEFT: u16 = 0x25;
const VK_UP: u16 = 0x26;
const VK_RIGHT: u16 = 0x27;
const VK_DOWN: u16 = 0x28;
const VK_INSERT: u16 = 0x2D;
const VK_DELETE: u16 = 0x2E;
const VK_0: u16 = 0x30;
const VK_9: u16 = 0x39;
const VK_A: u16 = 0x41;
const VK_Z: u16 = 0x5A;
const VK_F1: u16 = 0x70;
const VK_F12: u16 = 0x7B;
const VK_LSHIFT: u16 = 0xA0;
const VK_RSHIFT: u16 = 0xA1;
const VK_LCONTROL: u16 = 0xA2;
const VK_RCONTROL: u16 = 0xA3;
const VK_LMENU: u16 = 0xA4;
const VK_RMENU: u16 = 0xA5;

fn vk_to_key(vk: u16) -> Option<Key> {
    const LETTERS: [Key; 26] = [
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
    ];
    const DIGITS: [Key; 10] = [
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
    ];
    const F_KEYS: [Key; 12] = [
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
    ];
    match vk {
        VK_BACK => Some(Key::Backspace),
        VK_DELETE => Some(Key::Delete),
        VK_LEFT => Some(Key::Left),
        VK_UP => Some(Key::Up),
        VK_RIGHT => Some(Key::Right),
        VK_DOWN => Some(Key::Down),
        VK_HOME => Some(Key::Home),
        VK_END => Some(Key::End),
        VK_RETURN => Some(Key::Enter),
        VK_ESCAPE => Some(Key::Escape),
        VK_INSERT => Some(Key::Insert),
        VK_SPACE => Some(Key::Space),
        VK_TAB => Some(Key::Tab),
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => Some(Key::Shift),
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Some(Key::Ctrl),
        VK_MENU | VK_LMENU | VK_RMENU => Some(Key::Alt),
        VK_A..=VK_Z => Some(LETTERS[(vk - VK_A) as usize]),
        VK_0..=VK_9 => Some(DIGITS[(vk - VK_0) as usize]),
        VK_F1..=VK_F12 => Some(F_KEYS[(vk - VK_F1) as usize]),
        _ => None,
    }
}

fn lparam_xy(lparam: isize) -> (i16, i16) {
    (
        (lparam & 0xFFFF) as u16 as i16,
        ((lparam >> 16) & 0xFFFF) as u16 as i16,
    )
}

fn mouse_button(msg: u32, wparam: usize) -> Option<(usize, bool)> {
    let xbutton = || match ((wparam >> 16) & 0xFFFF) as u16 {
        XBUTTON1 => Some(3usize),
        XBUTTON2 => Some(4usize),
        _ => None,
    };
    match msg {
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => Some((0, true)),
        WM_LBUTTONUP => Some((0, false)),
        WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => Some((1, true)),
        WM_RBUTTONUP => Some((1, false)),
        WM_MBUTTONDOWN | WM_MBUTTONDBLCLK => Some((2, true)),
        WM_MBUTTONUP => Some((2, false)),
        WM_XBUTTONDOWN | WM_XBUTTONDBLCLK => xbutton().map(|i| (i, true)),
        WM_XBUTTONUP => xbutton().map(|i| (i, false)),
        _ => None,
    }
}

pub struct Win32InputAdapter {
    mouse_pos: [f32; 2],
    mouse_down: [bool; 5],
    wheel: f32,
    events: Vec<KeyEvent>,
    spare: Vec<KeyEvent>,
    viewport: Option<[f32; 2]>,
}

impl Win32InputAdapter {
    pub fn new() -> Self {
        Self {
            mouse_pos: [0.0; 2],
            mouse_down: [false; 5],
            wheel: 0.0,
            events: Vec::new(),
            spare: Vec::new(),
            viewport: None,
        }
    }

    pub fn handle_message(&mut self, msg: u32, wparam: usize, lparam: isize) -> bool {
        if let Some((index, pressed)) = mouse_button(msg, wparam) {
            self.mouse_down[index] = pressed;
            return true;
        }
        match msg {
            WM_MOUSEMOVE => {
                let (x, y) = lparam_xy(lparam);
                self.mouse_pos = [f32::from(x), f32::from(y)];
                true
            }
            WM_MOUSEWHEEL => {
                let delta = ((wparam >> 16) & 0xFFFF) as u16 as i16;
                self.wheel += f32::from(delta) / f32::from(WHEEL_DELTA);
                true
            }
            WM_SIZE => {
                let (w, h) = lparam_xy(lparam);
                if w > 0 && h > 0 {
                    self.viewport = Some([f32::from(w), f32::from(h)]);
                }
                false
            }
            WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP => {
                let pressed = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
                let vk = (wparam & 0xFFFF) as u16;
                if let Some(key) = vk_to_key(vk) {
                    self.events.push(if pressed {
                        KeyEvent::Press(key)
                    } else {
                        KeyEvent::Release(key)
                    });
                }
                true
            }
            WM_CHAR => {
                if let Some(c) = char::from_u32(wparam as u32) {
                    if !c.is_control() {
                        self.events.push(KeyEvent::Char(c));
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn clear_held(&mut self) {
        self.mouse_down = [false; 5];
        self.events.clear();
    }

    pub fn viewport(&self) -> Option<[f32; 2]> {
        self.viewport
    }

    pub fn begin_frame(&mut self) -> FrameInput {
        core::mem::swap(&mut self.events, &mut self.spare);
        let wheel = core::mem::take(&mut self.wheel);
        FrameInput {
            mouse_pos: self.mouse_pos,
            mouse_down: self.mouse_down,
            wheel,
            viewport: self.viewport.unwrap_or([0.0, 0.0]),
            keys: core::mem::take(&mut self.spare),
        }
    }

    pub fn end_frame(&mut self, frame: FrameInput) {
        let mut keys = frame.keys;
        keys.clear();
        self.spare = keys;
    }
}

impl Default for Win32InputAdapter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FrameInput {
    mouse_pos: [f32; 2],
    mouse_down: [bool; 5],
    wheel: f32,
    viewport: [f32; 2],
    keys: Vec<KeyEvent>,
}

impl FrameInput {
    pub fn input_state(&self) -> InputState<'_> {
        InputState {
            mouse_pos: self.mouse_pos,
            mouse_down: self.mouse_down,
            mouse_wheel: self.wheel,
            keys: &self.keys,
            viewport_size: self.viewport,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_and_vk_constants_match_the_win32_abi() {
        assert_eq!(WM_KEYDOWN, 0x0100);
        assert_eq!(WM_KEYUP, 0x0101);
        assert_eq!(WM_CHAR, 0x0102);
        assert_eq!(WM_SYSKEYDOWN, 0x0104);
        assert_eq!(WM_SYSKEYUP, 0x0105);
        assert_eq!(WM_MOUSEMOVE, 0x0200);
        assert_eq!(WM_LBUTTONDOWN, 0x0201);
        assert_eq!(WM_LBUTTONUP, 0x0202);
        assert_eq!(WM_LBUTTONDBLCLK, 0x0203);
        assert_eq!(WM_RBUTTONDOWN, 0x0204);
        assert_eq!(WM_RBUTTONUP, 0x0205);
        assert_eq!(WM_RBUTTONDBLCLK, 0x0206);
        assert_eq!(WM_MBUTTONDOWN, 0x0207);
        assert_eq!(WM_MBUTTONUP, 0x0208);
        assert_eq!(WM_MBUTTONDBLCLK, 0x0209);
        assert_eq!(WM_MOUSEWHEEL, 0x020A);
        assert_eq!(WM_XBUTTONDOWN, 0x020B);
        assert_eq!(WM_XBUTTONUP, 0x020C);
        assert_eq!(WM_XBUTTONDBLCLK, 0x020D);
        assert_eq!(WM_SIZE, 0x0005);
        assert_eq!(XBUTTON1, 0x0001);
        assert_eq!(XBUTTON2, 0x0002);
        assert_eq!(WHEEL_DELTA, 120);
        assert_eq!(VK_BACK, 0x08);
        assert_eq!(VK_TAB, 0x09);
        assert_eq!(VK_RETURN, 0x0D);
        assert_eq!(VK_SHIFT, 0x10);
        assert_eq!(VK_CONTROL, 0x11);
        assert_eq!(VK_MENU, 0x12);
        assert_eq!(VK_ESCAPE, 0x1B);
        assert_eq!(VK_SPACE, 0x20);
        assert_eq!(VK_END, 0x23);
        assert_eq!(VK_HOME, 0x24);
        assert_eq!(VK_LEFT, 0x25);
        assert_eq!(VK_UP, 0x26);
        assert_eq!(VK_RIGHT, 0x27);
        assert_eq!(VK_DOWN, 0x28);
        assert_eq!(VK_INSERT, 0x2D);
        assert_eq!(VK_DELETE, 0x2E);
        assert_eq!(VK_LSHIFT, 0xA0);
        assert_eq!(VK_RSHIFT, 0xA1);
        assert_eq!(VK_LCONTROL, 0xA2);
        assert_eq!(VK_RCONTROL, 0xA3);
        assert_eq!(VK_LMENU, 0xA4);
        assert_eq!(VK_RMENU, 0xA5);
    }

    const ALL_KEYS: [(u16, Key); 64] = [
        (0x08, Key::Backspace),
        (0x2E, Key::Delete),
        (0x25, Key::Left),
        (0x27, Key::Right),
        (0x24, Key::Home),
        (0x23, Key::End),
        (0x0D, Key::Enter),
        (0x1B, Key::Escape),
        (0x2D, Key::Insert),
        (0x41, Key::A),
        (0x42, Key::B),
        (0x43, Key::C),
        (0x44, Key::D),
        (0x45, Key::E),
        (0x46, Key::F),
        (0x47, Key::G),
        (0x48, Key::H),
        (0x49, Key::I),
        (0x4A, Key::J),
        (0x4B, Key::K),
        (0x4C, Key::L),
        (0x4D, Key::M),
        (0x4E, Key::N),
        (0x4F, Key::O),
        (0x50, Key::P),
        (0x51, Key::Q),
        (0x52, Key::R),
        (0x53, Key::S),
        (0x54, Key::T),
        (0x55, Key::U),
        (0x56, Key::V),
        (0x57, Key::W),
        (0x58, Key::X),
        (0x59, Key::Y),
        (0x5A, Key::Z),
        (0x30, Key::Num0),
        (0x31, Key::Num1),
        (0x32, Key::Num2),
        (0x33, Key::Num3),
        (0x34, Key::Num4),
        (0x35, Key::Num5),
        (0x36, Key::Num6),
        (0x37, Key::Num7),
        (0x38, Key::Num8),
        (0x39, Key::Num9),
        (0x70, Key::F1),
        (0x71, Key::F2),
        (0x72, Key::F3),
        (0x73, Key::F4),
        (0x74, Key::F5),
        (0x75, Key::F6),
        (0x76, Key::F7),
        (0x77, Key::F8),
        (0x78, Key::F9),
        (0x79, Key::F10),
        (0x7A, Key::F11),
        (0x7B, Key::F12),
        (0x20, Key::Space),
        (0x09, Key::Tab),
        (0xA0, Key::Shift),
        (0xA2, Key::Ctrl),
        (0xA4, Key::Alt),
        (0x26, Key::Up),
        (0x28, Key::Down),
    ];

    #[test]
    fn every_key_variant_is_reachable_from_its_vk_code() {
        for (vk, expected) in ALL_KEYS {
            assert_eq!(vk_to_key(vk), Some(expected), "VK {vk:#04X} must map");
        }
    }

    #[test]
    fn side_specific_and_generic_modifiers_collapse() {
        for vk in [VK_SHIFT, VK_LSHIFT, VK_RSHIFT] {
            assert_eq!(vk_to_key(vk), Some(Key::Shift));
        }
        for vk in [VK_CONTROL, VK_LCONTROL, VK_RCONTROL] {
            assert_eq!(vk_to_key(vk), Some(Key::Ctrl));
        }
        for vk in [VK_MENU, VK_LMENU, VK_RMENU] {
            assert_eq!(vk_to_key(vk), Some(Key::Alt));
        }
    }

    #[test]
    fn unmapped_vk_codes_are_ignored() {
        for vk in [0xC0u16, 0x5D, 0x00, 0xFF] {
            assert_eq!(vk_to_key(vk), None, "VK {vk:#04X} must stay unmapped");
            let mut adapter = Win32InputAdapter::new();
            assert!(adapter.handle_message(WM_KEYDOWN, vk as usize, 0));
            let frame = adapter.begin_frame();
            assert!(
                frame.keys.is_empty(),
                "unmapped VK {vk:#04X} must queue nothing"
            );
        }
    }

    #[test]
    fn key_messages_map_to_press_release_events_in_order() {
        let mut adapter = Win32InputAdapter::new();
        assert!(adapter.handle_message(WM_KEYDOWN, VK_A as usize, 0));
        assert!(adapter.handle_message(WM_SYSKEYDOWN, VK_LEFT as usize, 0));
        assert!(adapter.handle_message(WM_KEYUP, VK_A as usize, 0));
        assert!(adapter.handle_message(WM_SYSKEYUP, VK_LEFT as usize, 0));
        let frame = adapter.begin_frame();
        assert_eq!(
            frame.keys,
            [
                KeyEvent::Press(Key::A),
                KeyEvent::Press(Key::Left),
                KeyEvent::Release(Key::A),
                KeyEvent::Release(Key::Left),
            ]
        );
    }

    #[test]
    fn wm_char_accepts_text_and_drops_control_chars_and_surrogates() {
        let mut adapter = Win32InputAdapter::new();
        assert!(adapter.handle_message(WM_CHAR, 'a' as usize, 0));
        assert!(adapter.handle_message(WM_CHAR, 'É' as usize, 0));
        assert!(adapter.handle_message(WM_CHAR, '\r' as usize, 0));
        assert!(adapter.handle_message(WM_CHAR, '\t' as usize, 0));
        assert!(adapter.handle_message(WM_CHAR, 0x07usize, 0));
        assert!(adapter.handle_message(WM_CHAR, 0xD800usize, 0));
        assert!(adapter.handle_message(WM_CHAR, 0xDC00usize, 0));
        let frame = adapter.begin_frame();
        assert_eq!(frame.keys, [KeyEvent::Char('a'), KeyEvent::Char('É')]);
    }

    #[test]
    fn button_messages_track_state_and_double_clicks_fold_to_presses() {
        let mut adapter = Win32InputAdapter::new();
        assert!(adapter.handle_message(WM_LBUTTONDOWN, 0, 0));
        assert!(adapter.handle_message(WM_RBUTTONDOWN, 0, 0));
        assert!(adapter.handle_message(WM_MBUTTONDOWN, 0, 0));
        assert_eq!(adapter.mouse_down, [true, true, true, false, false]);
        assert!(adapter.handle_message(WM_RBUTTONUP, 0, 0));
        assert_eq!(adapter.mouse_down, [true, false, true, false, false]);
        assert!(adapter.handle_message(WM_LBUTTONUP, 0, 0));
        assert!(adapter.handle_message(WM_LBUTTONDBLCLK, 0, 0));
        assert_eq!(adapter.mouse_down, [true, false, true, false, false]);
        assert!(adapter.handle_message(WM_MBUTTONUP, 0, 0));
        assert_eq!(adapter.mouse_down, [true, false, false, false, false]);
        assert!(adapter.handle_message(WM_LBUTTONUP, 0, 0));
        assert_eq!(adapter.mouse_down, [false, false, false, false, false]);
    }

    #[test]
    fn xbutton_messages_map_the_hiword_to_the_back_forward_indices() {
        let mut adapter = Win32InputAdapter::new();
        assert!(adapter.handle_message(WM_XBUTTONDOWN, (XBUTTON1 as usize) << 16, 0));
        assert_eq!(adapter.mouse_down, [false, false, false, true, false]);
        assert!(adapter.handle_message(WM_XBUTTONDOWN, (XBUTTON2 as usize) << 16, 0));
        assert_eq!(adapter.mouse_down, [false, false, false, true, true]);
        assert!(adapter.handle_message(WM_XBUTTONUP, (XBUTTON1 as usize) << 16, 0));
        assert_eq!(adapter.mouse_down, [false, false, false, false, true]);
        assert!(adapter.handle_message(WM_XBUTTONUP, (XBUTTON2 as usize) << 16, 0));
        assert!(adapter.handle_message(WM_XBUTTONDBLCLK, (XBUTTON2 as usize) << 16, 0));
        assert_eq!(adapter.mouse_down, [false, false, false, false, true]);
    }

    #[test]
    fn an_unknown_xbutton_hiword_is_ignored() {
        let mut adapter = Win32InputAdapter::new();
        assert!(!adapter.handle_message(WM_XBUTTONDOWN, 0x0003usize << 16, 0));
        assert!(!adapter.handle_message(WM_XBUTTONUP, 0x0003usize << 16, 0));
        assert_eq!(adapter.mouse_down, [false; 5]);
    }

    #[test]
    fn mouse_move_reads_signed_lparam_words() {
        let mut adapter = Win32InputAdapter::new();
        let lparam = (200isize << 16) | 100isize;
        assert!(adapter.handle_message(WM_MOUSEMOVE, 0, lparam));
        assert_eq!(adapter.mouse_pos, [100.0, 200.0]);
        let lparam = ((-20isize) << 16) | ((-10isize) & 0xFFFF);
        assert!(adapter.handle_message(WM_MOUSEMOVE, 0, lparam));
        assert_eq!(adapter.mouse_pos, [-10.0, -20.0]);
    }

    #[test]
    fn wheel_delta_converts_and_accumulates_then_reports_and_zeroes() {
        let mut adapter = Win32InputAdapter::new();
        assert!(adapter.handle_message(WM_MOUSEWHEEL, 120usize << 16, 0));
        let down = ((-120i32) as u16 as usize) << 16;
        assert!(adapter.handle_message(WM_MOUSEWHEEL, down, 0));
        assert!(adapter.handle_message(WM_MOUSEWHEEL, 240usize << 16, 0));
        let frame = adapter.begin_frame();
        assert_eq!(frame.input_state().mouse_wheel, 2.0);
        adapter.end_frame(frame);
        let frame = adapter.begin_frame();
        assert_eq!(frame.input_state().mouse_wheel, 0.0);
    }

    #[test]
    fn wm_size_updates_the_viewport_and_zero_is_ignored() {
        let mut adapter = Win32InputAdapter::new();
        assert_eq!(adapter.viewport(), None);
        assert!(!adapter.handle_message(WM_SIZE, 0, (600isize << 16) | 800isize));
        assert_eq!(adapter.viewport(), Some([800.0, 600.0]));
        assert!(!adapter.handle_message(WM_SIZE, 0, 0));
        assert_eq!(adapter.viewport(), Some([800.0, 600.0]));
        assert!(!adapter.handle_message(WM_SIZE, 0, (480isize << 16) | 640isize));
        assert_eq!(adapter.viewport(), Some([640.0, 480.0]));
    }

    #[test]
    fn clear_held_releases_buttons_and_drops_pending_key_events() {
        let mut adapter = Win32InputAdapter::new();
        adapter.handle_message(WM_LBUTTONDOWN, 0, 0);
        adapter.handle_message(WM_MBUTTONDOWN, 0, 0);
        adapter.handle_message(WM_KEYDOWN, VK_A as usize, 0);
        adapter.handle_message(WM_CHAR, 'x' as usize, 0);
        adapter.clear_held();
        assert_eq!(adapter.mouse_down, [false; 5]);
        let frame = adapter.begin_frame();
        assert!(frame.keys.is_empty());
        assert_eq!(frame.input_state().mouse_down, [false; 5]);
    }

    #[test]
    fn events_between_begin_frames_land_in_the_next_frame_input() {
        let mut adapter = Win32InputAdapter::new();
        adapter.handle_message(WM_KEYDOWN, VK_A as usize, 0);
        let first = adapter.begin_frame();
        assert_eq!(first.keys, [KeyEvent::Press(Key::A)]);
        adapter.handle_message(WM_KEYUP, VK_A as usize, 0);
        adapter.end_frame(first);
        let second = adapter.begin_frame();
        assert_eq!(second.keys, [KeyEvent::Release(Key::A)]);
        adapter.end_frame(second);
    }

    #[test]
    fn end_frame_round_trip_retains_buffer_capacity() {
        let mut adapter = Win32InputAdapter::new();
        for _ in 0..32 {
            adapter.handle_message(WM_KEYDOWN, VK_A as usize, 0);
            adapter.handle_message(WM_KEYUP, VK_A as usize, 0);
        }
        let frame = adapter.begin_frame();
        let capacity = frame.keys.capacity();
        assert!(capacity >= 64);
        adapter.end_frame(frame);
        assert_eq!(
            adapter.spare.capacity(),
            capacity,
            "the spent buffer is parked for reuse, not dropped"
        );
        let frame = adapter.begin_frame();
        adapter.end_frame(frame);
        let frame = adapter.begin_frame();
        assert_eq!(frame.keys.capacity(), capacity);
        assert!(frame.keys.is_empty());
        adapter.end_frame(frame);
        adapter.handle_message(WM_KEYDOWN, 0x42usize, 0);
        let dropped = adapter.begin_frame();
        assert_eq!(dropped.keys, [KeyEvent::Press(Key::B)]);
        drop(dropped);
        adapter.handle_message(WM_KEYUP, 0x42usize, 0);
        let frame = adapter.begin_frame();
        assert_eq!(frame.keys, [KeyEvent::Release(Key::B)]);
        adapter.end_frame(frame);
    }

    #[test]
    fn input_state_matches_inputstate_field_semantics() {
        let mut adapter = Win32InputAdapter::new();
        adapter.handle_message(WM_MOUSEMOVE, 0, (200isize << 16) | 100isize);
        adapter.handle_message(WM_LBUTTONDOWN, 0, 0);
        adapter.handle_message(WM_MOUSEWHEEL, 120usize << 16, 0);
        adapter.handle_message(WM_KEYDOWN, 0x47usize, 0);
        let frame = adapter.begin_frame();
        let input = frame.input_state();
        assert_eq!(input.mouse_pos, [100.0, 200.0]);
        assert_eq!(input.mouse_down, [true, false, false, false, false]);
        assert_eq!(input.mouse_wheel, 1.0);
        assert_eq!(input.keys, [KeyEvent::Press(Key::G)]);
        assert_eq!(input.viewport_size, [0.0, 0.0]);
        adapter.end_frame(frame);
        adapter.handle_message(WM_SIZE, 0, (768isize << 16) | 1024isize);
        let frame = adapter.begin_frame();
        assert_eq!(frame.input_state().viewport_size, [1024.0, 768.0]);
        adapter.end_frame(frame);
    }

    #[test]
    fn unrecognized_messages_return_false_and_change_nothing() {
        let mut adapter = Win32InputAdapter::new();
        for msg in [0x000Fu32, 0x0002, 0x8000] {
            assert!(!adapter.handle_message(msg, 123, 456));
        }
        let frame = adapter.begin_frame();
        let input = frame.input_state();
        assert_eq!(input.mouse_pos, [0.0, 0.0]);
        assert_eq!(input.mouse_down, [false; 5]);
        assert_eq!(input.mouse_wheel, 0.0);
        assert!(input.keys.is_empty());
        assert_eq!(input.viewport_size, [0.0, 0.0]);
    }

    #[test]
    fn default_matches_new() {
        let adapter = Win32InputAdapter::default();
        assert_eq!(adapter.mouse_down, [false; 5]);
        assert_eq!(adapter.viewport(), None);
    }
}
