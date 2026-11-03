use super::*;
use crate::test_support::{new_context, window_frame};
use core::mem::{align_of, offset_of, size_of};

#[test]
fn wire_layout_and_rejected_sort_codes_are_pinned() {
    assert_eq!(
        (
            size_of::<GuiDataTableState>(),
            align_of::<GuiDataTableState>()
        ),
        (140, 4)
    );
    assert_eq!(
        (
            offset_of!(GuiDataTableState, widths),
            offset_of!(GuiDataTableState, scroll_x),
            offset_of!(GuiDataTableState, sort_column),
            offset_of!(GuiDataTableState, sort_direction)
        ),
        (0, 128, 132, 136)
    );
    assert_eq!(
        (
            size_of::<GuiDataTableOptions>(),
            align_of::<GuiDataTableOptions>()
        ),
        (20, 4)
    );
    assert_eq!(
        (
            offset_of!(GuiDataTableOptions, size),
            offset_of!(GuiDataTableOptions, row_height),
            offset_of!(GuiDataTableOptions, frozen_columns),
            offset_of!(GuiDataTableOptions, frozen_rows)
        ),
        (0, 8, 12, 16)
    );
    assert_eq!(
        (size_of::<GuiVisibleRows>(), align_of::<GuiVisibleRows>()),
        (2 * size_of::<usize>(), align_of::<usize>())
    );
    assert_eq!(
        (
            offset_of!(GuiVisibleRows, start),
            offset_of!(GuiVisibleRows, end)
        ),
        (0, size_of::<usize>())
    );
    let mut state = GuiDataTableState::from(DataTableState::default());
    state.sort_column = 0;
    state.sort_direction = 99;
    assert!(state.decode().is_none());
    state.sort_direction = 1;
    assert_eq!(
        state.decode().unwrap().sort,
        Some(SortSpec {
            column: 0,
            descending: false
        })
    );
}

#[test]
fn borrowed_multi_list_ignores_invalid_offscreen_labels_and_preserves_selection_on_visible_failure()
{
    unsafe {
        let ctx = new_context();
        let selection = gui_selection_new(10_000);
        assert!(gui_selection_select(selection, 2, 0, 0));
        let mut items = vec![GuiStr::new("row"); 10_000];
        items[9_999] = GuiStr {
            ptr: core::ptr::null(),
            len: 1,
        };
        window_frame(ctx, [0., 0.], false, |ctx| {
            assert!(!gui_multi_select_list(
                ctx,
                GuiStr::new("list"),
                items.as_ptr(),
                items.len(),
                selection,
                GuiVec2 { x: 240., y: 100. }
            ));
            assert_eq!(crate::gui_last_error_code(), GuiErrorCode::None as u8);
        });
        items[0] = GuiStr {
            ptr: core::ptr::null(),
            len: 1,
        };
        window_frame(ctx, [0., 0.], false, |ctx| {
            assert!(!gui_multi_select_list(
                ctx,
                GuiStr::new("list"),
                items.as_ptr(),
                items.len(),
                selection,
                GuiVec2 { x: 240., y: 100. }
            ));
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
        });
        assert!(gui_selection_contains(selection, 2));
        assert_eq!(gui_selection_count(selection), 1);
        gui_selection_free(selection);
        crate::gui_context_free(ctx);
    }
}

#[test]
fn crossed_data_ends_leave_the_scope_open_and_render_unwinds() {
    unsafe {
        let ctx = new_context();
        let mut state = GuiDataTableState::from(DataTableState::default());
        window_frame(ctx, [0., 0.], false, |ctx| {
            let headers = [GuiStr::new("A"), GuiStr::new("B")];
            assert!(gui_begin_data_table(
                ctx,
                GuiStr::new("table"),
                headers.as_ptr(),
                2,
                100,
                &mut state,
                GuiDataTableOptions {
                    size: GuiVec2 { x: 240., y: 120. },
                    row_height: 28.,
                    frozen_columns: 1,
                    frozen_rows: 0
                }
            ));
            gui_end_rows(ctx);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert_eq!(
                (*ctx).containers.last().map(ContainerScope::kind),
                Some(ContainerKind::DataTable)
            );
            assert!(gui_begin_data_table_cell(ctx, 0, 0));
            gui_end_data_cell(ctx);
            gui_end_data_table(ctx);
        });
        let input = crate::test_support::input_at([0., 0.], false);
        crate::gui_new_frame(ctx, &input, 0.016);
        assert!(crate::gui_begin_window(
            ctx,
            b"W".as_ptr(),
            1,
            crate::GUI_SCROLL_NONE
        ));
        assert!(gui_begin_property_grid(ctx, GuiStr::new("grid"), 80., 28.));
        assert!(gui_begin_property_row(ctx, GuiStr::new("Value")));
        crate::gui_render(ctx);
        assert!((*ctx).containers.is_empty());
        crate::gui_context_free(ctx);
    }
}

#[test]
fn invalid_descriptors_preserve_outputs_and_handles() {
    unsafe {
        let mut out = GuiVisibleRows { start: 77, end: 88 };
        assert!(!gui_clip_rows(
            100,
            f32::NAN,
            0.,
            0.,
            GuiVec2 { x: 0., y: 40. },
            &mut out
        ));
        assert_eq!((out.start, out.end), (77, 88));
        let selection = gui_selection_new(3);
        gui_selection_select(selection, 1, 0, 0);
        assert!(!gui_selection_resize(selection, MAX_DATA_ROWS + 1));
        assert!(gui_selection_contains(selection, 1));
        assert!(!gui_selection_select(selection, 1, 3, 0));
        assert!(gui_selection_contains(selection, 1));
        gui_selection_free(selection);
        let edit = gui_numeric_edit_new();
        assert!(!gui_numeric_edit_invalid(edit));
        gui_numeric_edit_free(edit);
    }
}
