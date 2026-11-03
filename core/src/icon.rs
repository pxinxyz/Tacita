pub const STAR: char = '\u{f005}';
pub const CHECK: char = '\u{f00c}';
pub const GEAR: char = '\u{f013}';
pub const HOME: char = '\u{f015}';
pub const EYE: char = '\u{f06e}';
pub const XMARK: char = '\u{f00d}';
pub const PLUS: char = '\u{f067}';
pub const TRIANGLE_EXCLAMATION: char = '\u{f071}';

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{Font, IconFont};

    const DEJAVU_SANS: &[u8] = include_bytes!("../tests/assets/DejaVuSans.ttf");
    const FA_SOLID: &[u8] = include_bytes!("../tests/assets/fa-solid-900.ttf");

    const ALL: &[(&str, char)] = &[
        ("STAR", STAR),
        ("CHECK", CHECK),
        ("GEAR", GEAR),
        ("HOME", HOME),
        ("EYE", EYE),
        ("XMARK", XMARK),
        ("PLUS", PLUS),
        ("TRIANGLE_EXCLAMATION", TRIANGLE_EXCLAMATION),
    ];

    fn icon_font() -> Font {
        Font::new_with_icons(
            DEJAVU_SANS,
            16.0,
            &[IconFont {
                bytes: FA_SOLID,
                range: (STAR as u32, TRIANGLE_EXCLAMATION as u32),
            }],
        )
        .expect("icon bake must succeed")
    }

    #[test]
    fn constants_are_distinct_and_in_the_private_use_area() {
        for (i, &(name_a, a)) in ALL.iter().enumerate() {
            assert!(
                (0xE000..=0xF8FF).contains(&(a as u32)),
                "{name_a} is outside the BMP Private Use Area"
            );
            for &(name_b, b) in &ALL[i + 1..] {
                assert_ne!(a, b, "{name_a} and {name_b} share a codepoint");
            }
        }
    }

    #[test]
    fn every_constant_maps_to_a_nonzero_coverage_glyph_baked_at_init() {
        let mut font = icon_font();
        for &(name, ch) in ALL {
            let gen = font.atlas_generation();
            let (_, quad) = font.glyph_quad(crate::font::FontId::DEFAULT, ch, [0.0, 0.0]);
            assert_eq!(
                font.atlas_generation(),
                gen,
                "{name} (U+{:04X}) was not baked at init",
                ch as u32
            );
            let quad = quad.unwrap_or_else(|| {
                panic!("{name} (U+{:04X}) has no ink — wrong codepoint?", ch as u32)
            });
            let (pixels, aw, ah) = font.atlas_pixels();
            let x0 = (quad.uv_min[0] * aw as f32 + 0.5) as u32;
            let y0 = (quad.uv_min[1] * ah as f32 + 0.5) as u32;
            let x1 = (quad.uv_max[0] * aw as f32 + 0.5) as u32;
            let y1 = (quad.uv_max[1] * ah as f32 + 0.5) as u32;
            let covered = (y0..y1).any(|y| (x0..x1).any(|x| pixels[(y * aw + x) as usize] > 0));
            assert!(
                covered,
                "{name} (U+{:04X}) has zero coverage in the atlas",
                ch as u32
            );
        }
    }
}
