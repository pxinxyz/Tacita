use super::*;

fn font() -> Font {
    Font::new(include_bytes!("../../tests/assets/DejaVuSans.ttf"), 16.0).unwrap()
}
fn span(text: &str) -> TextSpan<'_> {
    TextSpan {
        text,
        font: FontId::DEFAULT,
        color: [255; 4],
    }
}

#[test]
fn wraps_words_across_spans_and_handles_newline_variants() {
    let mut f = font();
    let spans = [span("one hel"), span("lo world")];
    let width = f.measure("one hello")[0];
    let l = line(
        &mut f,
        FontId::DEFAULT,
        2,
        &|i| spans[i],
        Position::default(),
        width,
        true,
    );
    assert_eq!(l.end, Position { span: 1, byte: 2 });
    let next = line(
        &mut f,
        FontId::DEFAULT,
        2,
        &|i| spans[i],
        l.next,
        width,
        true,
    );
    assert!(next.terminal);
    assert_eq!(next.width, f.measure("world")[0]);
    let spans = [span("a\r"), span("\nb\rc\n")];
    let m = measure(
        &mut f,
        FontId::DEFAULT,
        2,
        &|i| spans[i],
        100.0,
        f32::INFINITY,
        TextOptions::default(),
    );
    assert_eq!(m.lines, 4);
    assert_eq!(m.size[1], f.line_height() * 4.0);
}
#[test]
fn trailing_spaces_at_soft_break_do_not_create_extra_lines() {
    let mut f = font();
    let s = span("word     ");
    let width = f.measure("word")[0];
    let m = measure(
        &mut f,
        FontId::DEFAULT,
        1,
        &|_| s,
        width,
        f32::INFINITY,
        TextOptions {
            wrap: true,
            ..TextOptions::default()
        },
    );
    assert_eq!(m.lines, 1);
    assert!(!m.truncated);
    let s = span("word    \n");
    let m = measure(
        &mut f,
        FontId::DEFAULT,
        1,
        &|_| s,
        width,
        f32::INFINITY,
        TextOptions {
            wrap: true,
            ..TextOptions::default()
        },
    );
    assert_eq!(m.lines, 2);
}
#[test]
fn elision_is_utf8_safe_and_honors_marker_budget() {
    let mut f = font();
    let s = span("éλabcdef");
    let l = line(
        &mut f,
        FontId::DEFAULT,
        1,
        &|_| s,
        Position::default(),
        200.0,
        false,
    );
    let width = f.measure("éλ…")[0];
    let fit = fit_line(&mut f, 1, &|_| s, l, width, TextOverflow::Ellipsis, false);
    assert_eq!(fit.end.byte, "éλ".len());
    assert!(s.text.is_char_boundary(fit.end.byte));
    assert!(fit.marker.is_some());
    let marker_width = f.measure("…")[0];
    let fit = fit_line(
        &mut f,
        1,
        &|_| s,
        l,
        marker_width * 0.5,
        TextOverflow::Ellipsis,
        false,
    );
    assert!(fit.marker.is_none());
    assert_eq!(fit.width, 0.0);
    let zero = measure(
        &mut f,
        FontId::DEFAULT,
        1,
        &|_| s,
        0.0,
        f32::INFINITY,
        TextOptions {
            wrap: true,
            ..TextOptions::default()
        },
    );
    assert_eq!(zero.lines, s.text.chars().count());
}
#[test]
fn line_limit_and_box_height_elide_the_final_visible_line() {
    let mut f = font();
    let s = span("first\nsecond\nthird");
    let opts = TextOptions {
        overflow: TextOverflow::Ellipsis,
        max_lines: 2,
        line_spacing: 3.0,
        ..TextOptions::default()
    };
    let m = measure(
        &mut f,
        FontId::DEFAULT,
        1,
        &|_| s,
        200.0,
        f32::INFINITY,
        opts,
    );
    assert_eq!(m.lines, 2);
    assert!(m.truncated);
    assert_eq!(m.size[1], f.line_height() * 2.0 + 3.0);
    let height = f.line_height();
    let m = measure(&mut f, FontId::DEFAULT, 1, &|_| s, 200.0, height, opts);
    assert_eq!(m.lines, 1);
    assert!(m.truncated);
}
#[test]
fn mixed_faces_share_baseline_and_box_alignment_moves_geometry() {
    let mut f = font();
    let big = f
        .add_face(include_bytes!("../../tests/assets/DejaVuSans.ttf"), 32.0)
        .unwrap();
    let spans = [
        span("A"),
        TextSpan {
            text: "A",
            font: big,
            color: [20, 30, 40, 255],
        },
    ];
    let get = |i| spans[i];
    let l = line(
        &mut f,
        FontId::DEFAULT,
        2,
        &get,
        Position::default(),
        200.0,
        false,
    );
    let small = f.face_info(FontId::DEFAULT).unwrap();
    let large = f.face_info(big).unwrap();
    assert_eq!(l.ascent, small.ascent.max(large.ascent));
    let mut d = DrawList::new();
    let m = measure(
        &mut f,
        FontId::DEFAULT,
        2,
        &get,
        200.0,
        100.0,
        TextOptions::default(),
    );
    draw(
        (&mut f, &mut d),
        FontId::DEFAULT,
        2,
        &get,
        [10.0, 20.0, 210.0, 120.0],
        TextOptions::default(),
        m,
    );
    let baseline = 20.0 + l.ascent;
    let expected = f
        .glyph_quad(FontId::DEFAULT, 'A', [10.0, baseline - small.ascent])
        .1
        .unwrap();
    assert_eq!(d.as_draw_data().vertices[0].pos, expected.min);
    let before = d.as_draw_data().vertices[0].pos;
    d.clear();
    draw(
        (&mut f, &mut d),
        FontId::DEFAULT,
        2,
        &get,
        [10.0, 20.0, 210.0, 120.0],
        TextOptions {
            horizontal: TextAlign::End,
            vertical: TextAlign::Center,
            ..TextOptions::default()
        },
        m,
    );
    assert!((d.as_draw_data().vertices[0].pos[0] - before[0] - (200.0 - m.size[0])).abs() < 0.001);
    assert!(
        (d.as_draw_data().vertices[0].pos[1] - before[1] - (100.0 - m.size[1]) * 0.5).abs() < 0.001
    );
    assert!(d
        .as_draw_data()
        .commands
        .iter()
        .all(|cmd| cmd.clip_rect == [10.0, 20.0, 210.0, 120.0]));
}

#[test]
fn cross_span_graphemes_never_split_at_wrap_or_elision() {
    let mut f = font();
    for (a, b) in [("e", "\u{301}Z"), ("👩", "‍💻Z"), ("🇬", "🇧Z")] {
        let spans = [span(a), span(b)];
        let get = |i| spans[i];
        assert!(!boundary(
            2,
            &get,
            Position {
                span: 0,
                byte: a.len()
            }
        ));
        let width = f.measure(a)[0].max(1.);
        let l = line(
            &mut f,
            FontId::DEFAULT,
            2,
            &get,
            Position::default(),
            width,
            true,
        );
        assert_eq!(
            l.end,
            Position {
                span: 1,
                byte: b.len() - 1
            }
        );
        let fit = fit_line(
            &mut f,
            2,
            &get,
            l,
            (width - 0.5).max(0.),
            TextOverflow::Truncate,
            false,
        );
        assert_eq!(fit.end, Position::default());
        assert_eq!(fit.width, 0.);
    }
}
