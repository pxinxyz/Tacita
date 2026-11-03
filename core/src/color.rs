pub(crate) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = wrap_hue(h);
    let s = clamp01(s);
    let v = clamp01(v);

    let c = v * s;
    let hp = h / 60.0;
    let t = hp % 2.0 - 1.0;
    let t = if t < 0.0 { -t } else { t };
    let x = c * (1.0 - t);
    let (r, g, b) = if hp < 1.0 {
        (c, x, 0.0)
    } else if hp < 2.0 {
        (x, c, 0.0)
    } else if hp < 3.0 {
        (0.0, c, x)
    } else if hp < 4.0 {
        (0.0, x, c)
    } else if hp < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = v - c;
    [r + m, g + m, b + m]
}

pub(crate) fn rgb_to_hsv(rgb: [f32; 3]) -> (f32, f32, f32) {
    let r = clamp01(rgb[0]);
    let g = clamp01(rgb[1]);
    let b = clamp01(rgb[2]);

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;

    let v = max;
    let s = if max > 0.0 { d / max } else { 0.0 };
    let h = if d == 0.0 {
        0.0
    } else {
        let h = if max == r {
            ((g - b) / d) % 6.0
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        let h = h * 60.0;
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    };
    (h, s, v)
}

pub(crate) fn rgba_u8_to_hsv(rgba: [u8; 4]) -> (f32, f32, f32) {
    rgb_to_hsv([
        rgba[0] as f32 / 255.0,
        rgba[1] as f32 / 255.0,
        rgba[2] as f32 / 255.0,
    ])
}

pub(crate) fn hsv_to_rgba_u8(h: f32, s: f32, v: f32, a: u8) -> [u8; 4] {
    let [r, g, b] = hsv_to_rgb(h, s, v);
    [channel_to_u8(r), channel_to_u8(g), channel_to_u8(b), a]
}

pub(crate) fn parse_rgba_hex(s: &str) -> Option<[u8; 4]> {
    let digits = s.strip_prefix('#').unwrap_or(s);
    if digits.len() != 6 && digits.len() != 8 {
        return None;
    }
    let mut rgba = [0u8; 4];
    for (i, pair) in digits.as_bytes().chunks_exact(2).enumerate() {
        rgba[i] = hex_digit(pair[0])? << 4 | hex_digit(pair[1])?;
    }
    if digits.len() == 6 {
        rgba[3] = 255;
    }
    Some(rgba)
}

pub(crate) fn format_rgba_hex(rgba: [u8; 4]) -> alloc::string::String {
    let mut buf = alloc::string::String::new();
    format_rgba_hex_into(&mut buf, rgba);
    buf
}

pub(crate) fn format_rgba_hex_into(buf: &mut alloc::string::String, rgba: [u8; 4]) {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    buf.clear();
    buf.push('#');
    for channel in rgba {
        buf.push(DIGITS[(channel >> 4) as usize] as char);
        buf.push(DIGITS[(channel & 0xF) as usize] as char);
    }
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn wrap_hue(h: f32) -> f32 {
    if !h.is_finite() {
        return 0.0;
    }
    let mut h = h % 360.0;
    if h < 0.0 {
        h += 360.0;
    }
    h
}

fn clamp01(x: f32) -> f32 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(0.0, 1.0)
    }
}

fn channel_to_u8(x: f32) -> u8 {
    ((x * 255.0 + 0.5) as i32).clamp(0, 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    fn hue_eps(h: f32) -> f32 {
        1e-5 + 1e-4 * h / 360.0
    }

    fn assert_rgb(actual: [f32; 3], expected: [f32; 3]) {
        for (a, e) in actual.iter().zip(expected.iter()) {
            assert!(
                (a - e).abs() <= EPS,
                "expected {expected:?}, got {actual:?}"
            );
        }
    }

    fn assert_hsv(actual: (f32, f32, f32), expected: (f32, f32, f32)) {
        assert!(
            (actual.0 - expected.0).abs() <= hue_eps(expected.0)
                && (actual.1 - expected.1).abs() <= EPS
                && (actual.2 - expected.2).abs() <= EPS,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn hsv_to_rgb_primaries_and_secondaries() {
        assert_rgb(hsv_to_rgb(0.0, 1.0, 1.0), [1.0, 0.0, 0.0]);
        assert_rgb(hsv_to_rgb(60.0, 1.0, 1.0), [1.0, 1.0, 0.0]);
        assert_rgb(hsv_to_rgb(120.0, 1.0, 1.0), [0.0, 1.0, 0.0]);
        assert_rgb(hsv_to_rgb(180.0, 1.0, 1.0), [0.0, 1.0, 1.0]);
        assert_rgb(hsv_to_rgb(240.0, 1.0, 1.0), [0.0, 0.0, 1.0]);
        assert_rgb(hsv_to_rgb(300.0, 1.0, 1.0), [1.0, 0.0, 1.0]);
    }

    #[test]
    fn rgb_to_hsv_primaries_and_secondaries() {
        assert_hsv(rgb_to_hsv([1.0, 0.0, 0.0]), (0.0, 1.0, 1.0));
        assert_hsv(rgb_to_hsv([1.0, 1.0, 0.0]), (60.0, 1.0, 1.0));
        assert_hsv(rgb_to_hsv([0.0, 1.0, 0.0]), (120.0, 1.0, 1.0));
        assert_hsv(rgb_to_hsv([0.0, 1.0, 1.0]), (180.0, 1.0, 1.0));
        assert_hsv(rgb_to_hsv([0.0, 0.0, 1.0]), (240.0, 1.0, 1.0));
        assert_hsv(rgb_to_hsv([1.0, 0.0, 1.0]), (300.0, 1.0, 1.0));
    }

    #[test]
    fn grayscale_has_zero_hue_and_saturation() {
        assert_hsv(rgb_to_hsv([0.4, 0.4, 0.4]), (0.0, 0.0, 0.4));
        assert_rgb(hsv_to_rgb(123.0, 0.0, 0.4), [0.4, 0.4, 0.4]);
    }

    #[test]
    fn black_and_white() {
        assert_rgb(hsv_to_rgb(210.0, 0.7, 0.0), [0.0, 0.0, 0.0]);
        assert_rgb(hsv_to_rgb(210.0, 0.0, 1.0), [1.0, 1.0, 1.0]);
        assert_hsv(rgb_to_hsv([0.0, 0.0, 0.0]), (0.0, 0.0, 0.0));
        assert_hsv(rgb_to_hsv([1.0, 1.0, 1.0]), (0.0, 0.0, 1.0));
    }

    #[test]
    fn arbitrary_values_both_directions() {
        let hsv = rgb_to_hsv([0.2, 0.6, 0.9]);
        assert_hsv(hsv, (1440.0 / 7.0, 7.0 / 9.0, 0.9));
        assert_rgb(hsv_to_rgb(hsv.0, hsv.1, hsv.2), [0.2, 0.6, 0.9]);

        assert_rgb(hsv_to_rgb(210.0, 0.75, 0.8), [0.2, 0.5, 0.8]);
        assert_hsv(rgb_to_hsv([0.2, 0.5, 0.8]), (210.0, 0.75, 0.8));
    }

    #[test]
    fn hue_wraps_into_0_360() {
        assert_eq!(hsv_to_rgb(370.0, 1.0, 1.0), hsv_to_rgb(10.0, 1.0, 1.0));
        assert_eq!(hsv_to_rgb(-10.0, 1.0, 1.0), hsv_to_rgb(350.0, 1.0, 1.0));
        assert_eq!(hsv_to_rgb(720.0, 1.0, 1.0), hsv_to_rgb(0.0, 1.0, 1.0));
        assert_rgb(hsv_to_rgb(360.0, 1.0, 1.0), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn saturation_and_value_clamp_to_0_1() {
        assert_rgb(hsv_to_rgb(120.0, 1.5, -0.2), [0.0, 0.0, 0.0]);
        assert_rgb(hsv_to_rgb(120.0, -0.3, 1.1), [1.0, 1.0, 1.0]);
        assert_hsv(rgb_to_hsv([2.0, -1.0, 0.5]), rgb_to_hsv([1.0, 0.0, 0.5]));
    }

    #[test]
    fn nan_inputs_use_documented_fallbacks() {
        assert_rgb(hsv_to_rgb(f32::NAN, f32::NAN, f32::NAN), [0.0, 0.0, 0.0]);
        assert_eq!(hsv_to_rgb(f32::NAN, 1.0, 1.0), hsv_to_rgb(0.0, 1.0, 1.0));
        assert_hsv(
            rgb_to_hsv([f32::NAN, 1.0, 0.0]),
            rgb_to_hsv([0.0, 1.0, 0.0]),
        );
    }

    #[test]
    fn infinite_hue_uses_documented_fallback() {
        assert_eq!(
            hsv_to_rgb(f32::INFINITY, 1.0, 1.0),
            hsv_to_rgb(0.0, 1.0, 1.0)
        );
        assert_eq!(
            hsv_to_rgb(f32::NEG_INFINITY, 1.0, 1.0),
            hsv_to_rgb(0.0, 1.0, 1.0)
        );
        assert_rgb(hsv_to_rgb(f32::INFINITY, 1.0, 1.0), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn rgba_u8_to_hsv_ignores_alpha() {
        assert_hsv(rgba_u8_to_hsv([255, 0, 0, 0]), (0.0, 1.0, 1.0));
        assert_hsv(rgba_u8_to_hsv([255, 0, 0, 255]), (0.0, 1.0, 1.0));
        assert_hsv(
            rgba_u8_to_hsv([128, 128, 128, 77]),
            (0.0, 0.0, 128.0 / 255.0),
        );
    }

    #[test]
    fn hsv_to_rgba_u8_passes_alpha_through() {
        assert_eq!(hsv_to_rgba_u8(0.0, 1.0, 1.0, 42), [255, 0, 0, 42]);
        assert_eq!(hsv_to_rgba_u8(240.0, 1.0, 1.0, 0), [0, 0, 255, 0]);
    }

    #[test]
    fn parse_accepts_rrggbb_and_rrggbbaa_with_optional_hash_any_case() {
        assert_eq!(parse_rgba_hex("#FF0000"), Some([255, 0, 0, 255]));
        assert_eq!(parse_rgba_hex("FF0000"), Some([255, 0, 0, 255]));
        assert_eq!(parse_rgba_hex("#a1B2c3"), Some([0xA1, 0xB2, 0xC3, 255]));
        assert_eq!(parse_rgba_hex("a1b2c3d4"), Some([0xA1, 0xB2, 0xC3, 0xD4]));
        assert_eq!(parse_rgba_hex("#1A2B3C4D"), Some([0x1A, 0x2B, 0x3C, 0x4D]));
        assert_eq!(parse_rgba_hex("#00FF80"), Some([0, 255, 128, 255]));
    }

    #[test]
    fn parse_rejects_malformed_input() {
        assert_eq!(parse_rgba_hex("#GGGGGG"), None);
        assert_eq!(parse_rgba_hex("12 4567"), None);
        assert_eq!(parse_rgba_hex(""), None);
        assert_eq!(parse_rgba_hex("#"), None);
        assert_eq!(parse_rgba_hex("#F"), None);
        assert_eq!(parse_rgba_hex("#FFF"), None);
        assert_eq!(parse_rgba_hex("#FFFFF"), None);
        assert_eq!(parse_rgba_hex("#FFFFFFF"), None);
        assert_eq!(parse_rgba_hex("#FFFFFFFFF"), None);
        assert_eq!(parse_rgba_hex(" FF0000"), None);
        assert_eq!(parse_rgba_hex("#FF0000 "), None);
        assert_eq!(parse_rgba_hex("##FF0000"), None);
    }

    #[test]
    fn format_produces_uppercase_rrggbbaa() {
        assert_eq!(format_rgba_hex([0x1A, 0x2B, 0x3C, 0x4D]), "#1A2B3C4D");
        assert_eq!(format_rgba_hex([255, 0, 128, 255]), "#FF0080FF");
        assert_eq!(format_rgba_hex([0, 0, 0, 0]), "#00000000");
    }

    #[test]
    fn format_into_reuses_the_buffer() {
        let mut buf = alloc::string::String::from("stale contents");
        format_rgba_hex_into(&mut buf, [0xAB, 0xCD, 0xEF, 0x01]);
        assert_eq!(buf, "#ABCDEF01");
        let capacity = buf.capacity();
        format_rgba_hex_into(&mut buf, [0, 0, 0, 0]);
        assert_eq!(buf, "#00000000");
        assert_eq!(
            buf.capacity(),
            capacity,
            "repeated formatting must not reallocate"
        );
    }

    #[test]
    fn format_then_parse_round_trips() {
        for value in [[0, 0, 0, 0], [255, 255, 255, 255], [0x1A, 0x2B, 0x3C, 0x4D]] {
            assert_eq!(parse_rgba_hex(&format_rgba_hex(value)), Some(value));
        }
    }

    #[test]
    fn u8_round_trip_is_exact() {
        let round_trip = |r: u8, g: u8, b: u8| {
            let (h, s, v) = rgba_u8_to_hsv([r, g, b, 255]);
            assert_eq!(
                hsv_to_rgba_u8(h, s, v, 255),
                [r, g, b, 255],
                "round trip of ({r}, {g}, {b})"
            );
        };

        for &r in &[0u8, 255] {
            for &g in &[0u8, 255] {
                for &b in &[0u8, 255] {
                    round_trip(r, g, b);
                }
            }
        }
        for level in (0..=255u16).step_by(17) {
            round_trip(level as u8, level as u8, level as u8);
        }
        for r in (0..=255u16).step_by(17) {
            for g in (0..=255u16).step_by(17) {
                for b in (0..=255u16).step_by(17) {
                    round_trip(r as u8, g as u8, b as u8);
                }
            }
        }
    }
}
