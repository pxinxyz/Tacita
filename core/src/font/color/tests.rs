use super::*;

const NOTO: &[u8] = include_bytes!("../../../tests/assets/Noto-COLRv1.ttf");
#[test]
fn emoji_presentation_obeys_unicode_defaults_and_variation_selectors() {
    for text in ["☕", "⌚", "😀", "🇬🇧", "👩‍💻", "1️⃣", "©️"] {
        assert!(prefers_color(text), "{text}");
    }
    for text in ["1", "©", "☺", "☕︎", "ordinary text"] {
        assert!(!prefers_color(text), "{text}");
    }
}

#[test]
fn noto_clusters_have_color_coverage_and_bounded_storage() {
    let face = rustybuzz::Face::from_slice(NOTO, 0).unwrap();
    for text in ["😀", "👩‍💻", "🇬🇧", "👍🏽", "🚀"] {
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        let shaped = rustybuzz::shape(&face, &[], buffer);
        assert_eq!(shaped.glyph_infos().len(), 1, "{text}");
        let bitmap = rasterize(NOTO, shaped.glyph_infos()[0].glyph_id as u16, 32.).unwrap();
        assert!(bitmap.width <= 128 && bitmap.height <= 128);
        assert!(bitmap
            .pixels
            .chunks_exact(4)
            .any(|p| p[3] > 128 && p[0] != p[2]));
        assert_eq!(
            bitmap.pixels.len(),
            (bitmap.width * bitmap.height * 4) as usize
        );
    }
}

#[test]
fn gradients_interpolate_premultiplied_colors_and_extend() {
    let stops = [
        Stop {
            offset: 0.,
            color: [1., 0., 0., 1.],
        },
        Stop {
            offset: 1.,
            color: [0., 0., 0., 0.],
        },
    ];
    assert_eq!(
        gradient(&stops, GradientExtend::Pad, 0.5),
        [0.5, 0., 0., 0.5]
    );
    assert_eq!(
        gradient(&stops, GradientExtend::Repeat, 1.5),
        [0.5, 0., 0., 0.5]
    );
    assert_eq!(
        gradient(&stops, GradientExtend::Reflect, 1.5),
        [0.5, 0., 0., 0.5]
    );
}
#[test]
fn color_transfer_uses_linear_light_and_round_trips_palette_bytes() {
    for byte in 0..=255u8 {
        assert_eq!(srgb::encode(srgb::DECODE[byte as usize]), byte);
    }
    assert!((rgba(ttf::RgbaColor::new(128, 128, 128, 255))[0] - 0.21586).abs() < 0.00001);
    assert_eq!(srgb::encode(0.5), 188);
}

#[test]
fn gradient_geometry_respects_third_point_and_two_circle_roots() {
    let colors = || {
        vec![
            Stop {
                offset: 0.,
                color: [0., 0., 0., 1.],
            },
            Stop {
                offset: 1.,
                color: [1.; 4],
            },
        ]
    };
    let linear = Brush::Linear([0., 0., 10., 0., 5., 5.], colors(), GradientExtend::Pad);
    assert_eq!(linear.sample([5., 0.]), [0.5, 0.5, 0.5, 1.]);
    assert_eq!(linear.sample([10., 5.]), [0.5, 0.5, 0.5, 1.]);
    let radial = Brush::Radial([0., 0., 1., 0., 0., 3.], colors(), GradientExtend::Pad);
    assert_eq!(radial.sample([2., 0.]), [0.5, 0.5, 0.5, 1.]);
    let cone = Brush::Radial([0., 0., 1., 2., 0., 1.], colors(), GradientExtend::Pad);
    assert_eq!(cone.sample([1., 0.]), [1.; 4]);
    assert_eq!(cone.sample([1., 2.]), [0.; 4]);
}

#[test]
fn fixed_atlas_guards_and_exhaustion_preserve_prior_pixels_and_generation() {
    let mut atlas = atlas::Atlas::default();
    let bitmap = Bitmap {
        width: 254,
        height: 254,
        pixels: vec![255; 254 * 254 * 4],
        origin: [-1, -2],
    };
    let first = atlas.insert(&bitmap).unwrap();
    assert_eq!(first.rect, [1, 1, 254, 254]);
    assert_eq!(&atlas.pixels[..4], &[0; 4]);
    for _ in 1..16 {
        atlas.insert(&bitmap).unwrap();
    }
    let before = atlas.pixels.clone();
    let generation = atlas.generation;
    assert_eq!(atlas.insert(&bitmap).err(), Some(ColorFontError::AtlasFull));
    assert_eq!(atlas.pixels, before);
    assert_eq!(atlas.generation, generation);
    assert_eq!((atlas.width, atlas.height), (1024, 1024));
}

#[test]
fn composition_handles_alpha_and_soft_light() {
    let source = [0.5, 0., 0., 0.5];
    let backdrop = [0., 0., 0.5, 0.5];
    assert_eq!(
        composite(source, backdrop, CompositeMode::SourceOver),
        [0.5, 0., 0.25, 0.75]
    );
    assert_eq!(
        composite(source, backdrop, CompositeMode::SourceIn),
        [0.25, 0., 0., 0.25]
    );
    let result = composite(
        [0.5, 0.5, 0.5, 1.],
        [0.25, 0.25, 0.25, 1.],
        CompositeMode::SoftLight,
    );
    assert_eq!(result, [0.25, 0.25, 0.25, 1.]);
}

#[test]
fn rejects_invalid_and_excessive_rasters() {
    assert!(rasterize(&[], 0, 16.).is_err());
    assert!(rasterize(NOTO, 0, f32::NAN).is_err());
    let face = rustybuzz::Face::from_slice(NOTO, 0).unwrap();
    let index = face.glyph_index('😀').unwrap().0;
    assert_eq!(
        rasterize(NOTO, index, 4096.).err(),
        Some(ColorFontError::TooComplex)
    );
}

#[test]
fn color_fallback_emits_untinted_rgba_and_survives_density_rebuild() {
    use crate::{
        draw::{DrawList, TextureId},
        font::{Font, FontId, ShapedOptions, ShapedText},
    };
    let mut font = Font::new(include_bytes!("../../../tests/assets/DejaVuSans.ttf"), 16.).unwrap();
    let color = font.add_face(NOTO, 16.).unwrap();
    let mono = font
        .add_face(include_bytes!("../../../tests/assets/NotoEmoji.ttf"), 16.)
        .unwrap();
    font.set_fallback_chain(FontId::DEFAULT, &[color, mono])
        .unwrap();
    assert_eq!(
        font.set_color_atlas_texture(Some(TextureId::FONT)),
        Err(ColorFontError::InvalidTexture)
    );
    font.set_color_atlas_texture(Some(TextureId(90))).unwrap();
    for scale in [1., 1.5, 2.] {
        let mut font = font.rescaled(scale).unwrap();
        let mut layout = ShapedText::default();
        layout
            .prepare(
                &mut font,
                FontId::DEFAULT,
                "office ☕ 😀 👩‍💻 🇬🇧 👍🏽",
                ShapedOptions::default(),
            )
            .unwrap();
        assert!(layout.glyphs().iter().any(|g| g.font == color));
        let mut draw = DrawList::new();
        layout.draw(&mut font, &mut draw, [0., 0.], [10, 20, 30, 128]);
        let generation = font.color_atlas_generation();
        assert!(generation > 0);
        assert!(draw
            .as_draw_data()
            .commands
            .iter()
            .any(|c| c.texture == TextureId(90)));
        assert!(draw
            .as_draw_data()
            .vertices
            .iter()
            .any(|v| v.col == [255, 255, 255, 128]));
        draw.clear();
        layout.draw(&mut font, &mut draw, [0., 0.], [255; 4]);
        assert_eq!(font.color_atlas_generation(), generation);
        font.set_color_atlas_texture(None).unwrap();
        layout
            .prepare(&mut font, FontId::DEFAULT, "👩‍💻", ShapedOptions::default())
            .unwrap();
        assert!(layout.glyphs().iter().all(|g| g.font == mono));
    }
}

#[test]
fn unsupported_color_paint_selects_monochrome_and_caches_failure() {
    use crate::font::{Font, FontId, ShapedOptions, ShapedText};
    let mut bytes = NOTO.to_vec();
    let face = rustybuzz::Face::from_slice(&bytes, 0).unwrap();
    let index = face.glyph_index('😀').unwrap().0;
    let offset = paint_offset(&bytes, index);
    bytes[offset] = 8;
    let mut font = Font::new(include_bytes!("../../../tests/assets/DejaVuSans.ttf"), 16.).unwrap();
    let color = font.add_face(&bytes, 16.).unwrap();
    let mono = font
        .add_face(include_bytes!("../../../tests/assets/NotoEmoji.ttf"), 16.)
        .unwrap();
    font.set_fallback_chain(FontId::DEFAULT, &[color, mono])
        .unwrap();
    font.set_color_atlas_texture(Some(crate::draw::TextureId(90)))
        .unwrap();
    let mut layout = ShapedText::default();
    layout
        .prepare(&mut font, FontId::DEFAULT, "😀", ShapedOptions::default())
        .unwrap();
    assert!(layout.glyphs().iter().all(|g| g.font == mono));
    assert_eq!(
        font.color_glyph_error(color, index),
        Some(ColorFontError::Unsupported)
    );
    assert_eq!(font.color_atlas_generation(), 0);
}

fn paint_offset(bytes: &[u8], index: u16) -> usize {
    let face = rustybuzz::Face::from_slice(bytes, 0).unwrap();
    let table = face
        .raw_face()
        .table(ttf::Tag::from_bytes(b"COLR"))
        .unwrap();
    let start = table.as_ptr() as usize - bytes.as_ptr() as usize;
    let base = u32::from_be_bytes(table[14..18].try_into().unwrap()) as usize;
    let count = u32::from_be_bytes(table[base..base + 4].try_into().unwrap()) as usize;
    let record = (0..count)
        .map(|i| base + 4 + i * 6)
        .find(|p| u16::from_be_bytes(table[*p..*p + 2].try_into().unwrap()) == index)
        .unwrap();
    let offset = u32::from_be_bytes(table[record + 2..record + 6].try_into().unwrap()) as usize;
    start + base + offset
}

#[test]
fn malformed_descendants_and_recursive_colr_glyphs_are_rejected_before_painting() {
    let face = rustybuzz::Face::from_slice(NOTO, 0).unwrap();
    let index = face.glyph_index('😀').unwrap().0;
    let offset = paint_offset(NOTO, index);
    let mut bytes = NOTO.to_vec();
    bytes[offset..offset + 6].copy_from_slice(&[10, 255, 255, 254, 0, 1]);
    assert_eq!(
        rasterize(&bytes, index, 16.).err(),
        Some(ColorFontError::InvalidFont)
    );
    bytes[offset..offset + 3].copy_from_slice(&[11, (index >> 8) as u8, index as u8]);
    assert_eq!(
        rasterize(&bytes, index, 16.).err(),
        Some(ColorFontError::InvalidFont)
    );
}
