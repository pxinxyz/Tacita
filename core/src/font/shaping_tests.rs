use super::*;
const FONT: &[u8] = include_bytes!("../../tests/assets/DejaVuSans.ttf");

#[test]
fn monochrome_emoji_fallback_shapes_complete_clusters_and_keeps_coverage_after_dpi() {
    let mut font = Font::new(FONT, 16.).unwrap();
    let emoji = font
        .add_face(include_bytes!("../../tests/assets/NotoEmoji.ttf"), 16.)
        .unwrap();
    font.set_fallback_chain(FontId::DEFAULT, &[emoji]).unwrap();
    font.set_text_raster_mode(TextRasterMode::Subpixel);
    let mut layout = ShapedText::default();
    for scale in [1.0, 1.5, 2.0] {
        if scale != 1.0 {
            font = font.rescaled(scale).unwrap();
        }
        for cluster in ["👩‍💻", "🇬🇧", "👍🏽", "🚀"] {
            layout
                .prepare(
                    &mut font,
                    FontId::DEFAULT,
                    cluster,
                    ShapedOptions::default(),
                )
                .unwrap();
            assert_eq!(
                layout.glyphs().len(),
                1,
                "{cluster} should be a composed emoji"
            );
            let glyph = &layout.glyphs()[0];
            assert_eq!(glyph.font, emoji);
            assert_ne!(glyph.index, 0, "{cluster} must not resolve to .notdef");
            assert!(layout
                .carets()
                .iter()
                .all(|c| c.byte == 0 || c.byte == cluster.len()));
            let mut draw = crate::draw::DrawList::new();
            layout.draw(&mut font, &mut draw, [0.; 2], [255; 4]);
            assert_eq!(
                font.faces[emoji.0 as usize].indexed[&glyph.index].source,
                Some(0)
            );
            assert!(!draw.as_draw_data().vertices.is_empty());
        }
    }
}

#[test]
fn shaped_ligatures_arabic_and_mixed_direction_keep_logical_carets() {
    let mut font = Font::new(FONT, 16.).unwrap();
    let mut layout = ShapedText::default();
    layout
        .prepare(
            &mut font,
            FontId::DEFAULT,
            "office",
            ShapedOptions::default(),
        )
        .unwrap();
    assert!(
        layout.glyphs().len() < 6,
        "OpenType ligatures should be substituted"
    );
    assert!(layout.caret(2).is_some());
    assert!(layout.caret(3).is_some());
    layout
        .prepare(&mut font, FontId::DEFAULT, "سلام", ShapedOptions::default())
        .unwrap();
    assert!(
        layout.glyphs().len() < 4,
        "Arabic lam-alef must shape as a ligature"
    );
    assert!(layout.caret(0).unwrap().position[0] > layout.caret(8).unwrap().position[0]);
    layout
        .prepare(
            &mut font,
            FontId::DEFAULT,
            "abc אבג xyz",
            ShapedOptions::default(),
        )
        .unwrap();
    let hebrew: Vec<_> = layout.glyphs().iter().filter(|g| g.rtl).collect();
    assert_eq!(hebrew.len(), 3);
    assert!(hebrew[0].cluster.start > hebrew[1].cluster.start);
    for (i, _) in "abc אבג xyz".grapheme_indices(true) {
        assert!(layout.caret(i).is_some());
    }
}
#[test]
fn wrapping_newlines_and_hit_testing_share_positions() {
    let mut font = Font::new(FONT, 16.).unwrap();
    let mut layout = ShapedText::default();
    layout
        .prepare(
            &mut font,
            FontId::DEFAULT,
            "e\u{301} long words\r\nאבג\n",
            ShapedOptions {
                width: 55.,
                wrap: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(layout.lines() >= 4);
    assert!(layout.size()[0] <= 55.01);
    for c in layout.carets() {
        assert!(crate::editing::is_grapheme_boundary(layout.text(), c.byte));
        let nearest = layout
            .hit_test([c.position[0], c.position[1] + 1.])
            .unwrap();
        assert_eq!(nearest.line, c.line);
        assert!((nearest.position[0] - c.position[0]).abs() < 0.01);
    }
    let saved = layout.size();
    assert!(layout
        .prepare(
            &mut font,
            FontId::DEFAULT,
            "bad",
            ShapedOptions {
                width: f32::NAN,
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(layout.size(), saved);
}
#[test]
fn fallback_resolves_whole_clusters_and_density_keeps_layout_and_glyph_ids() {
    let icons = include_bytes!("../../tests/assets/fa-solid-900.ttf");
    let mut font = Font::new(icons, 16.).unwrap();
    let latin = font.add_face(FONT, 16.).unwrap();
    font.set_fallback_chain(FontId::DEFAULT, &[latin]).unwrap();
    assert!(font
        .set_fallback_chain(FontId::DEFAULT, &[latin, latin])
        .is_err());
    let mut layout = ShapedText::default();
    layout
        .prepare(
            &mut font,
            FontId::DEFAULT,
            "e\u{301} office",
            ShapedOptions::default(),
        )
        .unwrap();
    assert!(layout
        .glyphs()
        .iter()
        .filter(|g| g.cluster.start == 0)
        .all(|g| g.font == latin));
    assert!(layout.glyphs().iter().any(|g| g.font == FontId::DEFAULT));
    let glyphs = layout.glyphs().to_vec();
    let size = layout.size();
    let mut draw = crate::draw::DrawList::new();
    layout.draw(&mut font, &mut draw, [10., 10.], [255; 4]);
    font.set_text_raster_mode(TextRasterMode::Subpixel);
    let generation = font.atlas_generation();
    draw.clear();
    layout.draw(&mut font, &mut draw, [10., 10.], [255; 4]);
    assert_eq!(
        font.atlas_generation(),
        generation,
        "mode switch must prime cached indexed phases between frames"
    );
    let mut scaled = font.rescaled(1.5).unwrap();
    assert_eq!(scaled.fallback_chain(FontId::DEFAULT), Some(&[latin][..]));
    layout
        .prepare(
            &mut scaled,
            FontId::DEFAULT,
            "e\u{301} office",
            ShapedOptions::default(),
        )
        .unwrap();
    assert_eq!(layout.glyphs(), glyphs);
    assert_eq!(layout.size(), size);
    draw.clear();
    layout.draw(&mut scaled, &mut draw, [10., 10.], [255; 4]);
    assert!(!draw.as_draw_data().vertices.is_empty());
    assert!(draw.as_draw_data().vertices.iter().all(
        |v| v.pos.iter().all(|x| x.is_finite()) && v.uv.iter().all(|x| (0.0..=1.0).contains(x))
    ));
}

#[test]
fn unicode_paragraph_separators_and_mixed_size_fallbacks_share_line_baselines() {
    let mut font = Font::new(FONT, 16.).unwrap();
    let mut layout = ShapedText::default();
    layout
        .prepare(
            &mut font,
            FontId::DEFAULT,
            "abc\u{2029}אבג",
            ShapedOptions::default(),
        )
        .unwrap();
    assert_eq!(layout.lines(), 2);
    assert!(layout.caret(6).is_some());
    let icons = font
        .add_face(include_bytes!("../../tests/assets/fa-solid-900.ttf"), 12.)
        .unwrap();
    let large = font.add_face(FONT, 24.).unwrap();
    font.set_fallback_chain(icons, &[large]).unwrap();
    layout
        .prepare(
            &mut font,
            icons,
            "\u{f005} e\u{301}",
            ShapedOptions::default(),
        )
        .unwrap();
    let baseline = layout
        .glyphs()
        .iter()
        .map(|g| g.position[1] + font.faces[g.font.0 as usize].ascent)
        .collect::<Vec<_>>();
    assert!(baseline.iter().all(|b| (*b - baseline[0]).abs() < 0.01));
    layout.selection_rects(0..layout.text().len(), |r| {
        assert_eq!(r[1], 0.);
        assert_eq!(r[3], layout.line_height());
    });
}

#[test]
fn attached_marks_do_not_shift_selection_cells() {
    let mut font = Font::new(FONT, 16.).unwrap();
    let mut layout = ShapedText::default();
    layout
        .prepare(&mut font, FontId::DEFAULT, "سَ", ShapedOptions::default())
        .unwrap();
    let a = layout.caret(0).unwrap().position[0];
    let b = layout.caret(layout.text().len()).unwrap().position[0];
    let mut cells = 0;
    layout.selection_rects(0..layout.text().len(), |r| {
        assert!((r[0] - a.min(b)).abs() < 0.01);
        assert!((r[2] - a.max(b)).abs() < 0.01);
        cells += 1;
    });
    assert_eq!(cells, 1);
}
