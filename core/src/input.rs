#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Enter,
    Escape,
    Insert,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Space,
    Tab,
    Shift,
    Ctrl,
    Alt,
    Up,
    Down,
}

impl Key {
    pub fn name(&self) -> &'static str {
        match self {
            Key::Backspace => "Backspace",
            Key::Delete => "Delete",
            Key::Left => "Left",
            Key::Right => "Right",
            Key::Up => "Up",
            Key::Down => "Down",
            Key::Home => "Home",
            Key::End => "End",
            Key::Enter => "Enter",
            Key::Escape => "Escape",
            Key::Insert => "Insert",
            Key::A => "A",
            Key::B => "B",
            Key::C => "C",
            Key::D => "D",
            Key::E => "E",
            Key::F => "F",
            Key::G => "G",
            Key::H => "H",
            Key::I => "I",
            Key::J => "J",
            Key::K => "K",
            Key::L => "L",
            Key::M => "M",
            Key::N => "N",
            Key::O => "O",
            Key::P => "P",
            Key::Q => "Q",
            Key::R => "R",
            Key::S => "S",
            Key::T => "T",
            Key::U => "U",
            Key::V => "V",
            Key::W => "W",
            Key::X => "X",
            Key::Y => "Y",
            Key::Z => "Z",
            Key::Num0 => "0",
            Key::Num1 => "1",
            Key::Num2 => "2",
            Key::Num3 => "3",
            Key::Num4 => "4",
            Key::Num5 => "5",
            Key::Num6 => "6",
            Key::Num7 => "7",
            Key::Num8 => "8",
            Key::Num9 => "9",
            Key::F1 => "F1",
            Key::F2 => "F2",
            Key::F3 => "F3",
            Key::F4 => "F4",
            Key::F5 => "F5",
            Key::F6 => "F6",
            Key::F7 => "F7",
            Key::F8 => "F8",
            Key::F9 => "F9",
            Key::F10 => "F10",
            Key::F11 => "F11",
            Key::F12 => "F12",
            Key::Space => "Space",
            Key::Tab => "Tab",
            Key::Shift => "Shift",
            Key::Ctrl => "Ctrl",
            Key::Alt => "Alt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyBinding {
    None,
    Key(Key),
    Mouse(u8),
    Chord { modifiers: Modifiers, key: Key },
}

impl core::fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            KeyBinding::None => f.write_str("None"),
            KeyBinding::Key(key) => f.write_str(key.name()),
            KeyBinding::Mouse(button) => write!(f, "Mouse{}", *button as u32 + 1),
            KeyBinding::Chord { modifiers, key } => {
                if modifiers.ctrl {
                    f.write_str("Ctrl+")?;
                }
                if modifiers.shift {
                    f.write_str("Shift+")?;
                }
                if modifiers.alt {
                    f.write_str("Alt+")?;
                }
                f.write_str(key.name())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Press(Key),
    Release(Key),
    Char(char),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputState<'a> {
    pub mouse_pos: [f32; 2],
    pub mouse_down: [bool; 5],
    pub mouse_wheel: f32,
    pub keys: &'a [KeyEvent],
    pub viewport_size: [f32; 2],
}

impl Default for InputState<'_> {
    fn default() -> Self {
        Self {
            mouse_pos: [0.0, 0.0],
            mouse_down: [false, false, false, false, false],
            mouse_wheel: 0.0,
            keys: &[],
            viewport_size: [0.0, 0.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn default_input_state_is_neutral() {
        let input = InputState::default();
        assert_eq!(input.mouse_pos, [0.0, 0.0]);
        assert_eq!(input.mouse_down, [false, false, false, false, false]);
        assert_eq!(input.mouse_wheel, 0.0);
        assert!(input.keys.is_empty());
        assert_eq!(input.viewport_size, [0.0, 0.0]);
    }

    const ALL_KEYS: [(Key, &str); 64] = [
        (Key::Backspace, "Backspace"),
        (Key::Delete, "Delete"),
        (Key::Left, "Left"),
        (Key::Right, "Right"),
        (Key::Home, "Home"),
        (Key::End, "End"),
        (Key::Enter, "Enter"),
        (Key::Escape, "Escape"),
        (Key::Insert, "Insert"),
        (Key::A, "A"),
        (Key::B, "B"),
        (Key::C, "C"),
        (Key::D, "D"),
        (Key::E, "E"),
        (Key::F, "F"),
        (Key::G, "G"),
        (Key::H, "H"),
        (Key::I, "I"),
        (Key::J, "J"),
        (Key::K, "K"),
        (Key::L, "L"),
        (Key::M, "M"),
        (Key::N, "N"),
        (Key::O, "O"),
        (Key::P, "P"),
        (Key::Q, "Q"),
        (Key::R, "R"),
        (Key::S, "S"),
        (Key::T, "T"),
        (Key::U, "U"),
        (Key::V, "V"),
        (Key::W, "W"),
        (Key::X, "X"),
        (Key::Y, "Y"),
        (Key::Z, "Z"),
        (Key::Num0, "0"),
        (Key::Num1, "1"),
        (Key::Num2, "2"),
        (Key::Num3, "3"),
        (Key::Num4, "4"),
        (Key::Num5, "5"),
        (Key::Num6, "6"),
        (Key::Num7, "7"),
        (Key::Num8, "8"),
        (Key::Num9, "9"),
        (Key::F1, "F1"),
        (Key::F2, "F2"),
        (Key::F3, "F3"),
        (Key::F4, "F4"),
        (Key::F5, "F5"),
        (Key::F6, "F6"),
        (Key::F7, "F7"),
        (Key::F8, "F8"),
        (Key::F9, "F9"),
        (Key::F10, "F10"),
        (Key::F11, "F11"),
        (Key::F12, "F12"),
        (Key::Space, "Space"),
        (Key::Tab, "Tab"),
        (Key::Shift, "Shift"),
        (Key::Ctrl, "Ctrl"),
        (Key::Alt, "Alt"),
        (Key::Up, "Up"),
        (Key::Down, "Down"),
    ];

    #[test]
    fn every_key_has_a_name() {
        for (key, expected) in ALL_KEYS {
            assert_eq!(key.name(), expected, "wrong name for {key:?}");
        }
    }

    #[test]
    fn key_names_are_unique() {
        for (i, (key, _)) in ALL_KEYS.iter().enumerate() {
            for (other, _) in &ALL_KEYS[i + 1..] {
                assert_ne!(
                    key.name(),
                    other.name(),
                    "{key:?} and {other:?} share a name"
                );
            }
        }
    }

    #[test]
    fn key_binding_display_is_the_button_text() {
        use alloc::string::ToString;

        assert_eq!(KeyBinding::None.to_string(), "None");
        assert_eq!(KeyBinding::Key(Key::F).to_string(), "F");
        assert_eq!(KeyBinding::Key(Key::F12).to_string(), "F12");
        assert_eq!(KeyBinding::Mouse(0).to_string(), "Mouse1");
        assert_eq!(KeyBinding::Mouse(3).to_string(), "Mouse4");
        assert_eq!(
            KeyBinding::Chord {
                modifiers: Modifiers {
                    shift: true,
                    ctrl: false,
                    alt: false
                },
                key: Key::G,
            }
            .to_string(),
            "Shift+G"
        );
        assert_eq!(
            KeyBinding::Chord {
                modifiers: Modifiers {
                    shift: true,
                    ctrl: true,
                    alt: false
                },
                key: Key::K,
            }
            .to_string(),
            "Ctrl+Shift+K"
        );
        assert_eq!(
            KeyBinding::Chord {
                modifiers: Modifiers {
                    shift: true,
                    ctrl: true,
                    alt: true
                },
                key: Key::K,
            }
            .to_string(),
            "Ctrl+Shift+Alt+K"
        );
    }

    #[test]
    fn new_key_variants_debug_and_compare() {
        let key = Key::G;
        assert_eq!(format!("{key:?}"), "G");
        assert_eq!(key, Key::G);
        assert_ne!(key, Key::H);
        assert_eq!(Key::Num5, Key::Num5);
        assert_ne!(Key::F1, Key::F12);
        assert_ne!(Key::Shift, Key::Ctrl);
    }
}
