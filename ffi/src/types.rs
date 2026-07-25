use restricted_gui::input::{KeyBinding, Modifiers};

use crate::{key_from_code, key_to_code};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuiVec2 {
    pub x: f32,
    pub y: f32,
}

impl From<GuiVec2> for [f32; 2] {
    fn from(v: GuiVec2) -> Self {
        [v.x, v.y]
    }
}

impl From<[f32; 2]> for GuiVec2 {
    fn from(a: [f32; 2]) -> Self {
        Self { x: a[0], y: a[1] }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuiRect {
    pub min: GuiVec2,
    pub max: GuiVec2,
}

impl GuiRect {
    pub const ZERO: Self = Self {
        min: GuiVec2 { x: 0.0, y: 0.0 },
        max: GuiVec2 { x: 0.0, y: 0.0 },
    };
}

impl From<GuiRect> for [f32; 4] {
    fn from(r: GuiRect) -> Self {
        [r.min.x, r.min.y, r.max.x, r.max.y]
    }
}

impl From<[f32; 4]> for GuiRect {
    fn from(a: [f32; 4]) -> Self {
        Self {
            min: GuiVec2 { x: a[0], y: a[1] },
            max: GuiVec2 { x: a[2], y: a[3] },
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<GuiColor> for [u8; 4] {
    fn from(c: GuiColor) -> Self {
        [c.r, c.g, c.b, c.a]
    }
}

impl From<[u8; 4]> for GuiColor {
    fn from(a: [u8; 4]) -> Self {
        Self {
            r: a[0],
            g: a[1],
            b: a[2],
            a: a[3],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiStr {
    pub ptr: *const u8,
    pub len: usize,
}

impl GuiStr {
    pub fn new(s: &str) -> Self {
        Self {
            ptr: s.as_ptr(),
            len: s.len(),
        }
    }
}

pub const GUI_KEY_BINDING_NONE: u8 = 0;
pub const GUI_KEY_BINDING_KEY: u8 = 1;
pub const GUI_KEY_BINDING_MOUSE: u8 = 2;
pub const GUI_KEY_BINDING_CHORD: u8 = 3;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
// ctrl/shift/alt are u8 on the wire. 0xFF means "held". 0x42 also means "held". A test feeds this garbage on purpose, for fun and profit.
pub struct GuiKeyBinding {
    pub tag: u8,
    pub key: u16,
    pub mouse_button: u8,
    pub ctrl: u8,
    pub shift: u8,
    pub alt: u8,
}

impl GuiKeyBinding {
    pub const NONE: Self = Self {
        tag: GUI_KEY_BINDING_NONE,
        key: 0,
        mouse_button: 0,
        ctrl: 0,
        shift: 0,
        alt: 0,
    };

    pub fn to_binding(self) -> Option<KeyBinding> {
        match self.tag {
            GUI_KEY_BINDING_NONE => Some(KeyBinding::None),
            GUI_KEY_BINDING_KEY => key_from_code(self.key).map(KeyBinding::Key),
            GUI_KEY_BINDING_MOUSE if self.mouse_button < 3 => {
                Some(KeyBinding::Mouse(self.mouse_button))
            }
            GUI_KEY_BINDING_CHORD => key_from_code(self.key).map(|key| KeyBinding::Chord {
                modifiers: Modifiers {
                    shift: self.shift != 0,
                    ctrl: self.ctrl != 0,
                    alt: self.alt != 0,
                },
                key,
            }),
            _ => None,
        }
    }

    pub fn from_binding(binding: KeyBinding) -> Self {
        match binding {
            KeyBinding::None => Self::NONE,
            KeyBinding::Key(key) => Self {
                tag: GUI_KEY_BINDING_KEY,
                key: key_to_code(key),
                ..Self::NONE
            },
            KeyBinding::Mouse(button) => Self {
                tag: GUI_KEY_BINDING_MOUSE,
                mouse_button: button,
                ..Self::NONE
            },
            KeyBinding::Chord { modifiers, key } => Self {
                tag: GUI_KEY_BINDING_CHORD,
                key: key_to_code(key),
                mouse_button: 0,
                ctrl: modifiers.ctrl as u8,
                shift: modifiers.shift as u8,
                alt: modifiers.alt as u8,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use restricted_gui::input::{Key, KeyBinding, Modifiers};

    use super::*;

    #[test]
    fn gui_vec2_and_gui_color_convert_both_ways() {
        let v = GuiVec2 { x: 1.5, y: -2.5 };
        let arr: [f32; 2] = v.into();
        assert_eq!(arr, [1.5, -2.5]);
        assert_eq!(GuiVec2::from(arr), v);

        let c = GuiColor {
            r: 10,
            g: 20,
            b: 30,
            a: 40,
        };
        let arr: [u8; 4] = c.into();
        assert_eq!(arr, [10, 20, 30, 40]);
        assert_eq!(GuiColor::from(arr), c);
    }

    #[test]
    fn gui_rect_converts_to_core_rect_array() {
        let r = GuiRect {
            min: GuiVec2 { x: 1.0, y: 2.0 },
            max: GuiVec2 { x: 3.0, y: 4.0 },
        };
        assert_eq!(<[f32; 4]>::from(r), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(GuiRect::from([1.0, 2.0, 3.0, 4.0]), r);
    }

    #[test]
    fn key_binding_round_trips_every_variant() {
        let cases = [
            KeyBinding::None,
            KeyBinding::Key(Key::F),
            KeyBinding::Mouse(2),
            KeyBinding::Chord {
                modifiers: Modifiers {
                    shift: true,
                    ctrl: true,
                    alt: false,
                },
                key: Key::K,
            },
        ];
        for binding in cases {
            let gui = GuiKeyBinding::from_binding(binding);
            assert_eq!(
                gui.to_binding(),
                Some(binding),
                "{binding:?} must round-trip through GuiKeyBinding"
            );
        }
    }

    #[test]
    fn key_binding_tags_match_the_documented_constants() {
        assert_eq!(
            GuiKeyBinding::from_binding(KeyBinding::None).tag,
            GUI_KEY_BINDING_NONE
        );
        assert_eq!(
            GuiKeyBinding::from_binding(KeyBinding::Key(Key::A)).tag,
            GUI_KEY_BINDING_KEY
        );
        assert_eq!(
            GuiKeyBinding::from_binding(KeyBinding::Mouse(0)).tag,
            GUI_KEY_BINDING_MOUSE
        );
        let chord = KeyBinding::Chord {
            modifiers: Modifiers {
                shift: false,
                ctrl: false,
                alt: true,
            },
            key: Key::F5,
        };
        let gui = GuiKeyBinding::from_binding(chord);
        assert_eq!(gui.tag, GUI_KEY_BINDING_CHORD);
        assert!(gui.alt != 0 && gui.ctrl == 0 && gui.shift == 0);
    }

    #[test]
    fn chord_modifiers_accept_any_nonzero_byte_pattern() {
        let garbage = GuiKeyBinding {
            tag: GUI_KEY_BINDING_CHORD,
            key: 0,
            ctrl: 0xFF,
            shift: 0x42,
            alt: 0,
            ..GuiKeyBinding::NONE
        };
        assert_eq!(
            garbage.to_binding(),
            Some(KeyBinding::Chord {
                modifiers: Modifiers {
                    shift: true,
                    ctrl: true,
                    alt: false
                },
                key: Key::Backspace,
            })
        );
    }

    #[test]
    fn invalid_tags_and_key_codes_are_rejected() {
        let bad_tag = GuiKeyBinding {
            tag: 9,
            ..GuiKeyBinding::NONE
        };
        assert_eq!(bad_tag.to_binding(), None);
        let bad_key = GuiKeyBinding {
            tag: GUI_KEY_BINDING_KEY,
            key: 62,
            ..GuiKeyBinding::NONE
        };
        assert_eq!(bad_key.to_binding(), None);
        let bad_chord = GuiKeyBinding {
            tag: GUI_KEY_BINDING_CHORD,
            key: u16::MAX,
            ctrl: 1,
            ..GuiKeyBinding::NONE
        };
        assert_eq!(bad_chord.to_binding(), None);
    }

    #[test]
    fn mouse_button_index_is_validated() {
        for button in 0..3u8 {
            let gui = GuiKeyBinding {
                tag: GUI_KEY_BINDING_MOUSE,
                mouse_button: button,
                ..GuiKeyBinding::NONE
            };
            assert_eq!(gui.to_binding(), Some(KeyBinding::Mouse(button)));
        }
        for button in [3u8, 200, u8::MAX] {
            let gui = GuiKeyBinding {
                tag: GUI_KEY_BINDING_MOUSE,
                mouse_button: button,
                ..GuiKeyBinding::NONE
            };
            assert_eq!(gui.to_binding(), None, "button {button} must be rejected");
        }
    }
}
