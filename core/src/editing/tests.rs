use super::*;
use crate::{
    font::TextDirection,
    input::{InputState, Key, KeyEvent},
    widgets::test_support::{frame, frame_with_input, test_context, widget_id_in_w},
};
fn declare(ui: &mut crate::ui::Ui, e: &mut TextEditor) {
    ui.text_edit(
        "editor",
        e,
        TextEditOptions {
            size: [240., 80.],
            multiline: true,
            wrap: true,
            ..Default::default()
        },
    )
    .unwrap();
}

#[test]
fn text_edit_preserves_each_padding_axis_and_fits_the_default_line_height() {
    for (padding, border) in [
        ([9., 6.], 1.),
        ([6., 4.], 1.),
        ([14., 2.], 1.),
        ([2., 12.], 1.),
        ([1., 1.], 8.),
    ] {
        let mut ctx = test_context();
        ctx.style.frame_padding = padding;
        ctx.style.border_size = border;
        ctx.style.text = [12, 34, 56, 255];
        let mut editor = TextEditor::new("Ágjpqy").unwrap();
        let mut rect = [0.; 4];
        frame(&mut ctx, [-1.; 2], false, |ui| {
            rect = ui
                .text_edit("editor", &mut editor, TextEditOptions::default())
                .unwrap()
                .rect;
        });
        let data = ctx.windows[0].draw_list.as_draw_data();
        let mut offset = 0;
        let mut found = false;
        for command in data.commands {
            let end = offset + command.elem_count as usize;
            let glyphs: alloc::vec::Vec<_> = data.indices[offset..end]
                .iter()
                .map(|index| &data.vertices[*index as usize])
                .filter(|v| v.col == ctx.style.text && v.pos[0] >= rect[0] && v.pos[1] >= rect[1])
                .collect();
            if !glyphs.is_empty() {
                let clip = command.clip_rect;
                assert!(
                    clip[3] - clip[1] + 0.001 >= editor.layout.line_height(),
                    "padding {padding:?}: line {}, clip {clip:?}",
                    editor.layout.line_height()
                );
                assert!((clip[0] - rect[0] - padding[0].max(border)).abs() < 0.001);
                assert!((clip[1] - rect[1] - padding[1].max(border)).abs() < 0.001);
                for vertex in glyphs {
                    assert!(
                        vertex.pos[1] >= clip[1].floor() && vertex.pos[1] <= clip[3].ceil(),
                        "glyph outside {clip:?}: {:?}",
                        vertex.pos
                    );
                }
                found = true;
            }
            offset = end;
        }
        assert!(found);
    }
}

fn keys(ctx: &mut crate::context::Context, e: &mut TextEditor, events: &[KeyEvent]) {
    frame_with_input(
        ctx,
        0.016,
        InputState {
            keys: events,
            ..Default::default()
        },
        |ui| declare(ui, e),
    );
}
fn focus(ctx: &mut crate::context::Context, e: &mut TextEditor) {
    frame(ctx, [0.; 2], false, |ui| {
        ui.focus(widget_id_in_w("editor"));
        declare(ui, e);
    });
}
#[test]
fn shift_selection_grapheme_delete_and_multiline_navigation() {
    let mut c = test_context();
    let mut e = TextEditor::new("one e\u{301}\nsecond").unwrap();
    focus(&mut c, &mut e);
    keys(
        &mut c,
        &mut e,
        &[
            KeyEvent::Press(Key::Home),
            KeyEvent::Press(Key::Shift),
            KeyEvent::Press(Key::End),
            KeyEvent::Release(Key::Shift),
        ],
    );
    assert_eq!(e.selected_text(), "second");
    keys(&mut c, &mut e, &[KeyEvent::Char('X')]);
    assert_eq!(e.text(), "one e\u{301}\nX");
    keys(
        &mut c,
        &mut e,
        &[
            KeyEvent::Press(Key::Home),
            KeyEvent::Press(Key::Backspace),
            KeyEvent::Press(Key::Backspace),
        ],
    );
    assert_eq!(e.text(), "one X");
    assert!(is_grapheme_boundary(e.text(), e.cursor()));
}
#[test]
fn ime_transient_cancel_commit_and_focus_change() {
    let mut c = test_context();
    let mut e = TextEditor::new("old").unwrap();
    focus(&mut c, &mut e);
    e.select_all();
    let id = widget_id_in_w("editor");
    c.submit_text_event(id, TextEventKind::Preedit, "日本", 6..6)
        .unwrap();
    keys(&mut c, &mut e, &[]);
    assert_eq!(e.text(), "old");
    assert!(e.composing());
    assert!(c.text_caret().unwrap().composing);
    c.submit_text_event(id, TextEventKind::Cancel, "", 0..0)
        .unwrap();
    keys(&mut c, &mut e, &[]);
    assert_eq!(e.text(), "old");
    assert!(!e.composing());
    c.submit_text_event(id, TextEventKind::Preedit, "日本", 6..6)
        .unwrap();
    keys(&mut c, &mut e, &[]);
    c.submit_text_event(id, TextEventKind::Commit, "日本語", 0..0)
        .unwrap();
    keys(&mut c, &mut e, &[]);
    assert_eq!(e.text(), "日本語");
    c.submit_text_event(id, TextEventKind::Preedit, "字", 3..3)
        .unwrap();
    keys(&mut c, &mut e, &[]);
    frame(&mut c, [0.; 2], false, |ui| {
        ui.focus(crate::id::Id(999));
        declare(ui, &mut e);
    });
    assert!(!e.composing());
    assert!(c.text_caret().is_none());
    c.submit_text_event(id, TextEventKind::Commit, "late", 0..0)
        .unwrap();
    keys(&mut c, &mut e, &[]);
    assert_eq!(e.text(), "日本語");
}
#[test]
fn clipboard_failure_stale_reply_and_cut_acknowledgement() {
    let mut c = test_context();
    let mut e = TextEditor::new("copy me").unwrap();
    focus(&mut c, &mut e);
    e.select_all();
    keys(
        &mut c,
        &mut e,
        &[
            KeyEvent::Press(Key::Ctrl),
            KeyEvent::Press(Key::X),
            KeyEvent::Release(Key::Ctrl),
        ],
    );
    let request = c.clipboard_request().unwrap();
    assert_eq!(request.text, "copy me");
    let id = request.id;
    c.complete_clipboard(id, None).unwrap();
    keys(&mut c, &mut e, &[]);
    assert_eq!(e.text(), "copy me");
    keys(
        &mut c,
        &mut e,
        &[
            KeyEvent::Press(Key::Ctrl),
            KeyEvent::Press(Key::V),
            KeyEvent::Release(Key::Ctrl),
        ],
    );
    let id = c.clipboard_request().unwrap().id;
    e.select(0, 0).unwrap();
    c.complete_clipboard(id, Some("stale")).unwrap();
    keys(&mut c, &mut e, &[]);
    assert_eq!(e.text(), "copy me");
    e.select_all();
    keys(
        &mut c,
        &mut e,
        &[
            KeyEvent::Press(Key::Ctrl),
            KeyEvent::Press(Key::X),
            KeyEvent::Release(Key::Ctrl),
        ],
    );
    let id = c.clipboard_request().unwrap().id;
    c.complete_clipboard(id, Some("")).unwrap();
    keys(&mut c, &mut e, &[]);
    assert_eq!(e.text(), "");
    assert!(!c.complete_clipboard(id, Some("")).unwrap());
}
#[test]
fn disabled_and_read_only_fields_do_not_mutate_or_publish_caret() {
    let mut c = test_context();
    let mut e = TextEditor::new("fixed").unwrap();
    focus(&mut c, &mut e);
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Char('x')],
            ..Default::default()
        },
        |ui| {
            ui.text_edit(
                "editor",
                &mut e,
                TextEditOptions {
                    read_only: true,
                    ..Default::default()
                },
            )
            .unwrap();
        },
    );
    assert_eq!(e.text(), "fixed");
    assert!(c.text_caret().is_none());
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Char('x')],
            ..Default::default()
        },
        |ui| {
            let disabled = ui.begin_disabled("disabled", true);
            declare(ui, &mut e);
            ui.end_disabled(disabled);
        },
    );
    assert_eq!(e.text(), "fixed");
    assert!(c.text_caret().is_none());
}
#[test]
fn visual_navigation_through_bidi_boundary_does_not_cycle() {
    let mut c = test_context();
    let mut e = TextEditor::new("abc אבג xyz").unwrap();
    focus(&mut c, &mut e);
    e.select(0, 0).unwrap();
    e.affinity = None;
    let mut x = -1.;
    for _ in 0..24 {
        keys(&mut c, &mut e, &[KeyEvent::Press(Key::Right)]);
        let caret = c.text_caret().unwrap();
        assert!(caret.rect[0] >= x);
        x = caret.rect[0];
    }
    assert_eq!(e.cursor(), e.text().len());
    assert_eq!(e.layout.glyphs().iter().filter(|g| g.rtl).count(), 3);
    let mut boxes = 0;
    e.layout.selection_rects(4..10, |r| {
        assert!(r[2] >= r[0]);
        boxes += 1;
    });
    assert_eq!(boxes, 3);
    let _ = TextDirection::Auto;
}
#[test]
fn legacy_field_has_selection_clipboard_and_grapheme_deletion() {
    let mut c = test_context();
    let mut text = alloc::string::String::from("e\u{301}");
    let id = widget_id_in_w("legacy");
    frame(&mut c, [0.; 2], false, |ui| {
        ui.focus(id);
        ui.text_input("legacy", &mut text);
    });
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Press(Key::Backspace)],
            ..Default::default()
        },
        |ui| {
            ui.text_input("legacy", &mut text);
        },
    );
    assert_eq!(text, "");
    text.push_str("selected");
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[
                KeyEvent::Press(Key::Ctrl),
                KeyEvent::Press(Key::A),
                KeyEvent::Press(Key::C),
                KeyEvent::Release(Key::Ctrl),
            ],
            ..Default::default()
        },
        |ui| {
            ui.text_input("legacy", &mut text);
        },
    );
    assert_eq!(c.clipboard_request().unwrap().text, "selected");
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Char('\u{2028}')],
            ..Default::default()
        },
        |ui| {
            ui.text_input("legacy", &mut text);
        },
    );
    assert_eq!(text, "selected");
}

#[test]
fn vertical_navigation_retains_column_and_horizontal_arrows_collapse_selection() {
    let mut c = test_context();
    let mut e = TextEditor::new("WWWWW\nx\nWWWWW").unwrap();
    focus(&mut c, &mut e);
    e.select(4, 4).unwrap();
    keys(&mut c, &mut e, &[KeyEvent::Press(Key::Down)]);
    assert_eq!(e.cursor(), 7);
    keys(&mut c, &mut e, &[KeyEvent::Press(Key::Down)]);
    assert_eq!(e.cursor(), 12);
    e.select(1, 4).unwrap();
    keys(&mut c, &mut e, &[KeyEvent::Press(Key::Left)]);
    assert_eq!(e.selection(), 1..1);
    e.select(1, 4).unwrap();
    keys(&mut c, &mut e, &[KeyEvent::Press(Key::Right)]);
    assert_eq!(e.selection(), 4..4);
}
#[test]
fn host_selection_resets_vertical_column_and_read_only_cancels_composition() {
    let mut c = test_context();
    let mut e = TextEditor::new("WWWWW\nx\nWWWWW").unwrap();
    focus(&mut c, &mut e);
    e.select(4, 4).unwrap();
    keys(&mut c, &mut e, &[KeyEvent::Press(Key::Down)]);
    e.select(0, 0).unwrap();
    keys(&mut c, &mut e, &[KeyEvent::Press(Key::Down)]);
    assert_eq!(e.cursor(), 6);
    e.set_preedit("candidate", 9..9).unwrap();
    frame(&mut c, [0.; 2], false, |ui| {
        ui.text_edit(
            "editor",
            &mut e,
            TextEditOptions {
                multiline: true,
                read_only: true,
                ..Default::default()
            },
        )
        .unwrap();
    });
    assert!(!e.composing());
    assert!(c.text_caret().is_none());
}

#[test]
fn host_selection_reveals_caret_and_single_line_ignores_unicode_line_breaks() {
    let mut c = test_context();
    let mut e = TextEditor::new("long words ".repeat(20).as_str()).unwrap();
    focus(&mut c, &mut e);
    e.select(0, 0).unwrap();
    keys(&mut c, &mut e, &[]);
    assert_eq!(e.scroll[1], 0.);
    e.select(e.text().len(), e.text().len()).unwrap();
    keys(&mut c, &mut e, &[]);
    assert!(e.scroll[1] > 0.);
    e.replace("one").unwrap();
    frame_with_input(
        &mut c,
        0.016,
        InputState {
            keys: &[KeyEvent::Char('\u{2028}'), KeyEvent::Char('\u{2029}')],
            ..Default::default()
        },
        |ui| {
            ui.text_edit("editor", &mut e, TextEditOptions::default())
                .unwrap();
        },
    );
    assert_eq!(e.text(), "one");
}
