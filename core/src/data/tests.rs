use super::*;

#[test]
fn clipping_excludes_gaps_and_exact_boundaries() {
    assert_eq!(
        VisibleRows::new(100, 20., 4., 0., [24., 44.])
            .unwrap()
            .range(),
        1..2
    );
    assert_eq!(
        VisibleRows::new(100, 20., 4., 0., [20., 24.])
            .unwrap()
            .range(),
        1..1
    );
    assert_eq!(
        VisibleRows::new(100, 20., 0., -1800., [0., 40.])
            .unwrap()
            .range(),
        90..92
    );
    assert!(VisibleRows::new(10, f32::NAN, 0., 0., [0., 40.]).is_err());
    assert!(VisibleRows::new(10, 20., 0., 0., [20., 20.])
        .unwrap()
        .range()
        .is_empty());
}

#[test]
fn selection_replacement_toggle_and_additive_ranges() {
    let mut s = Selection::new(12).unwrap();
    assert!(s.select(3, false, false));
    s.select(6, false, true);
    assert_eq!(
        s.indices().collect::<alloc::vec::Vec<_>>(),
        alloc::vec![3, 4, 5, 6]
    );
    s.select(9, true, false);
    s.select(11, true, true);
    assert_eq!(
        s.indices().collect::<alloc::vec::Vec<_>>(),
        alloc::vec![3, 4, 5, 6, 9, 10, 11]
    );
    s.select(4, false, false);
    assert_eq!(s.indices().collect::<alloc::vec::Vec<_>>(), alloc::vec![4]);
    s.select(4, true, false);
    assert_eq!(s.selected_count(), 0);
    assert!(!s.select(12, false, false));
}

#[test]
fn selection_shrink_removes_tail_and_invalidates_anchor() {
    let mut s = Selection::new(130).unwrap();
    s.select(129, false, false);
    s.resize(64).unwrap();
    assert_eq!(s.selected_count(), 0);
    s.resize(130).unwrap();
    assert!(!s.is_selected(129));
    s.select(2, false, true);
    assert_eq!(s.indices().collect::<alloc::vec::Vec<_>>(), alloc::vec![2]);
}

#[test]
fn sort_requests_cycle_and_invalid_columns_do_not_mutate() {
    let mut state = DataTableState::default();
    state.cycle_sort(2);
    assert_eq!(
        state.sort,
        Some(SortSpec {
            column: 2,
            descending: false
        })
    );
    state.cycle_sort(2);
    assert!(state.sort.unwrap().descending);
    state.cycle_sort(2);
    assert_eq!(state.sort, None);
    state.cycle_sort(MAX_DATA_COLUMNS);
    assert_eq!(state.sort, None);
}

#[test]
fn numeric_commit_rejects_nonfinite_overflow_and_incomplete_text() {
    let mut edit = NumericEdit::default();
    let mut value = 3.;
    edit.text.push_str("NaN");
    assert!(!edit.commit_float(&mut value, 0., 10.));
    assert_eq!(value, 3.);
    assert!(edit.invalid());
    edit.text.clear();
    edit.text.push_str("15.5");
    assert!(edit.commit_float(&mut value, 0., 10.));
    assert_eq!(value, 10.);
    let mut integer = i32::MAX - 1;
    edit.text.clear();
    edit.text.push_str("2147483648");
    assert!(!edit.commit_int(&mut integer, i32::MIN, i32::MAX));
    assert_eq!(integer, i32::MAX - 1);
    edit.text.clear();
    edit.text.push('-');
    assert!(!edit.commit_int(&mut integer, i32::MIN, i32::MAX));
}

#[test]
fn row_clipper_requests_only_visible_custom_bodies() {
    use crate::{context::Context, input::InputState, ScrollFlags};
    let mut ctx = Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap();
    ctx.set_window_rect("W", [0., 0.], [400., 350.]);
    ctx.set_input(InputState::default());
    ctx.new_frame(0.016);
    let mut emitted = 0;
    ctx.ui().window("W", ScrollFlags::NONE, |ui| {
        ui.child("viewport", [250., 100.], ScrollFlags::BAR, |ui| {
            ui.rows("custom", 1_000_000, 24., |ui, rows| {
                assert!(rows.visible_rows().range().len() <= 6);
                for index in rows.visible_rows().range() {
                    ui.row(rows, index, |ui| {
                        emitted += 1;
                        ui.label("row");
                    });
                }
                assert!(!ui.row(rows, 999_999, |_| panic!("offscreen body executed")));
            })
            .unwrap();
        });
    });
    assert!(emitted > 0);
    assert!(ctx.render().vertices.len() < 500);
}

#[test]
fn offscreen_lists_decode_no_labels_and_preserve_full_layout_height() {
    use crate::{context::Context, input::InputState, ScrollFlags};
    for count in [10, 10_000, 100_000] {
        let mut ctx = Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap();
        ctx.set_window_rect("W", [0., 0.], [400., 350.]);
        ctx.set_input(InputState::default());
        ctx.new_frame(0.016);
        let mut selection = Selection::new(count).unwrap();
        selection.select(3, false, false);
        let mut single = 3;
        let mut custom_calls = 0;
        ctx.ui().window("W", ScrollFlags::BAR, |ui| {
            ui.allocate([200., 500.]);
            assert_eq!(
                ui.list_box_with(
                    "##single",
                    count,
                    |_| panic!("hidden single label decoded"),
                    &mut single,
                    [250., 100.]
                ),
                Some(false)
            );
            assert_eq!(
                ui.multi_select_list_with(
                    "##multi",
                    count,
                    |_| panic!("hidden multi label decoded"),
                    &mut selection,
                    [250., 100.]
                ),
                Some(false)
            );
            ui.child("custom", [250., 100.], ScrollFlags::BAR, |ui| {
                let origin = ui.ctx.current_window_mut().cursor.position()[1];
                ui.rows("rows", count, 24., |ui, rows| {
                    assert!(rows.visible_rows().range().is_empty());
                    for i in rows.visible_rows().range() {
                        ui.row(rows, i, |_| custom_calls += 1);
                    }
                })
                .unwrap();
                assert!(
                    ui.ctx.current_window_mut().cursor.position()[1] >= origin + count as f32 * 24.
                );
            });
        });
        assert_eq!(custom_calls, 0);
        assert_eq!(single, 3);
        assert!(selection.is_selected(3));
        assert_eq!(selection.selected_count(), 1);
        assert!(ctx.render().vertices.len() < 1000);
    }
}

#[test]
fn disjoint_or_zero_width_clip_rects_have_no_visible_rows() {
    for rect in [
        [0., 200., 100., 100.],
        [200., 0., 100., 100.],
        [0., 0., 0., 100.],
    ] {
        assert!(VisibleRows::in_rect(100_000, 24., 6., 0., rect)
            .unwrap()
            .range()
            .is_empty());
    }
    assert!(VisibleRows::new(10, 24., 6., 0., [200., 100.]).is_err());
}
