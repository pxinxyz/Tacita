use super::StateError;
use crate::theme::ThemeTokens;
use alloc::string::String;
use core::fmt::Write;
const COLORS: [&str; 16] = [
    "title",
    "surface",
    "input",
    "hover",
    "accent",
    "text",
    "title_text",
    "scroll_bg",
    "scroll_thumb",
    "scroll_hover",
    "scroll_active",
    "secondary",
    "muted",
    "success",
    "warning",
    "danger",
];
const METRICS: [&str; 10] = [
    "frame_padding_x",
    "frame_padding_y",
    "item_spacing_x",
    "item_spacing_y",
    "window_padding_x",
    "window_padding_y",
    "slider_grab_size",
    "scrollbar_size",
    "corner_rounding",
    "border_size",
];
fn metrics(t: &ThemeTokens) -> [f32; 10] {
    [
        t.frame_padding[0],
        t.frame_padding[1],
        t.item_spacing[0],
        t.item_spacing[1],
        t.window_padding[0],
        t.window_padding[1],
        t.slider_grab_size,
        t.scrollbar_size,
        t.corner_rounding,
        t.border_size,
    ]
}
fn valid(t: &ThemeTokens) -> bool {
    t.is_valid() && metrics(t).iter().all(|v| *v <= 4096.)
}
impl crate::context::Context {
    pub fn save_theme(&self) -> Result<String, StateError> {
        assert!(!self.frame_active, "theme save requires an idle context");
        encode_theme(self.theme_tokens())
    }
    pub fn load_theme(&mut self, data: &[u8]) -> Result<(), StateError> {
        assert!(!self.frame_active, "theme load requires an idle context");
        let tokens = decode_theme(data)?;
        self.set_theme_tokens(tokens);
        Ok(())
    }
}
pub fn encode_theme(t: ThemeTokens) -> Result<String, StateError> {
    if !valid(&t) {
        return Err(StateError::InvalidValue);
    }
    let mut out = String::from("version=1\n");
    for (name, c) in COLORS.iter().zip(t.colors) {
        writeln!(
            out,
            "{name}=#{:02X}{:02X}{:02X}{:02X}",
            c[0], c[1], c[2], c[3]
        )
        .unwrap();
    }
    for (name, v) in METRICS.iter().zip(metrics(&t)) {
        writeln!(out, "{name}={v}").unwrap();
    }
    writeln!(out, "anti_aliasing={}", t.anti_aliasing as u8).unwrap();
    Ok(out)
}
pub fn decode_theme(data: &[u8]) -> Result<ThemeTokens, StateError> {
    if data.len() > 16384 {
        return Err(StateError::Limit);
    }
    let text = core::str::from_utf8(data).map_err(|_| StateError::Malformed)?;
    let mut t = ThemeTokens::default();
    let mut m = [0.; 10];
    let mut seen = 0u32;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or(StateError::Malformed)?;
        let key = key.trim();
        let value = value.trim();
        let index = if key == "version" {
            if value != "1" {
                return Err(StateError::Version);
            }
            0
        } else if let Some(i) = COLORS.iter().position(|k| *k == key) {
            let hex = value.strip_prefix('#').ok_or(StateError::InvalidValue)?;
            if hex.len() != 8 || !hex.is_ascii() {
                return Err(StateError::InvalidValue);
            }
            let packed = u32::from_str_radix(hex, 16).map_err(|_| StateError::InvalidValue)?;
            t.colors[i] = packed.to_be_bytes();
            i + 1
        } else if let Some(i) = METRICS.iter().position(|k| *k == key) {
            m[i] = value.parse().map_err(|_| StateError::InvalidValue)?;
            i + 17
        } else if key == "anti_aliasing" {
            t.anti_aliasing = match value {
                "0" => false,
                "1" => true,
                _ => return Err(StateError::InvalidValue),
            };
            27
        } else {
            return Err(StateError::Schema);
        };
        if seen & (1 << index) != 0 {
            return Err(StateError::Schema);
        }
        seen |= 1 << index;
    }
    if seen != (1u32 << 28) - 1 {
        return Err(StateError::Schema);
    }
    t.frame_padding = [m[0], m[1]];
    t.item_spacing = [m[2], m[3]];
    t.window_padding = [m[4], m[5]];
    t.slider_grab_size = m[6];
    t.scrollbar_size = m[7];
    t.corner_rounding = m[8];
    t.border_size = m[9];
    if valid(&t) {
        Ok(t)
    } else {
        Err(StateError::InvalidValue)
    }
}
