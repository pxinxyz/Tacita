use tacita_sdk::{
    DataTableOptions, DataTableState, Gui, InputState, NumericEdit, ScrollFlags,
};
const FONT: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

#[test]
fn flat_data_scopes_match_core_draw_data() {
    let mut core = tacita::context::Context::new(FONT).unwrap();
    let mut gui = Gui::new(FONT).unwrap();
    let input = InputState::default();
    core.set_input(input);
    core.new_frame(0.016);
    gui.new_frame(input, 0.016);
    let mut a = DataTableState::default();
    let mut b = a;
    let options = DataTableOptions {
        size: [200., 120.],
        row_height: 28.,
        frozen_columns: 1,
        frozen_rows: 0,
    };
    core.ui().window("W", ScrollFlags::NONE, |ui| {
        ui.data_table("T", &["ID", "Name"], 50, &mut a, options, |ui, t| {
            for row in t.visible_rows().range() {
                for column in 0..2 {
                    ui.data_table_cell(t, row, column, |ui| ui.label("cell"));
                }
            }
        })
        .unwrap();
        ui.property_grid("P", 80., 30., |ui, g| {
            ui.property_row(g, "Value", |ui| ui.label("data"));
        })
        .unwrap();
    });
    gui.begin_window("W", ScrollFlags::NONE);
    gui.begin_data_table("T", &["ID", "Name"], 50, &mut b, options)
        .unwrap();
    for row in gui.visible_rows().range() {
        for column in 0..2 {
            if gui.begin_data_table_cell(row, column) {
                gui.label("cell");
                gui.end_data_cell();
            }
        }
    }
    if let Some(offset) = gui.end_data_table() {
        b.scroll_x = offset;
    }
    gui.begin_property_grid("P", 80., 30.).unwrap();
    gui.begin_property_row("Value");
    gui.label("data");
    gui.end_property_row();
    gui.end_property_grid();
    gui.end_window();
    let expected = core.render();
    let actual = gui.render();
    assert_eq!(actual.vertices, expected.vertices);
    assert_eq!(actual.indices, expected.indices);
    assert_eq!(actual.commands, expected.commands);
    assert_eq!(b.scroll_x, a.scroll_x);
    assert_eq!(b.widths, a.widths);
    assert_eq!(b.sort, a.sort);
}

#[test]
fn suppressed_data_widgets_preserve_host_models() {
    let mut gui = Gui::new(FONT).unwrap();
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("W", ScrollFlags::NONE);
    gui.end_window();
    gui.render();
    gui.set_window_open("W", false);
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("W", ScrollFlags::NONE);
    let mut state = DataTableState {
        scroll_x: 73.,
        ..DataTableState::default()
    };
    let mut value = 7;
    let mut edit = NumericEdit::default();
    gui.begin_data_table("T", &["ID"], 40, &mut state, DataTableOptions::default())
        .unwrap();
    assert!(gui.visible_rows().range().is_empty());
    assert!(!gui.begin_data_table_cell(0, 0));
    assert!(gui.end_data_table().is_none());
    assert_eq!(state.scroll_x, 73.);
    assert!(!gui.input_int("N", &mut value, &mut edit, 1, 0, 10).unwrap());
    assert_eq!(value, 7);
    assert_eq!(edit.text(), "");
    gui.begin_menu_bar("M");
    assert!(!gui.begin_menu("File", [200., 100.]).unwrap());
    gui.end_menu_bar();
    gui.end_window();
    assert!(gui.render().vertices.is_empty());
}

#[test]
fn editor_forwards_match_core_and_suppression_preserves_models() {
    use tacita_sdk::{CurveEditor, GradientEditor, PlotOptions};
    let mut core = tacita::context::Context::new(FONT).unwrap();
    let mut gui = Gui::new(FONT).unwrap();
    let mut ca = CurveEditor::default();
    let mut cb = CurveEditor::default();
    let mut ga = GradientEditor::default();
    let mut gb = GradientEditor::default();
    core.set_input(InputState::default());
    core.new_frame(0.016);
    gui.new_frame(InputState::default(), 0.016);
    core.ui().window("W", ScrollFlags::NONE, |ui| {
        ui.plot(
            "##plot",
            &[0., 1., 0.5],
            PlotOptions {
                size: [200., 80.],
                ..PlotOptions::default()
            },
        )
        .unwrap();
        ui.curve_editor("##curve", &mut ca, [200., 100.]).unwrap();
        ui.gradient_editor("##gradient", &mut ga, [200., 60.])
            .unwrap();
    });
    gui.begin_window("W", ScrollFlags::NONE);
    gui.plot(
        "##plot",
        &[0., 1., 0.5],
        PlotOptions {
            size: [200., 80.],
            ..PlotOptions::default()
        },
    )
    .unwrap();
    gui.curve_editor("##curve", &mut cb, [200., 100.]).unwrap();
    gui.gradient_editor("##gradient", &mut gb, [200., 60.])
        .unwrap();
    gui.end_window();
    let expected = core.render();
    let actual = gui.render();
    assert_eq!(expected.vertices, actual.vertices);
    assert_eq!(expected.indices, actual.indices);
    assert_eq!(expected.commands, actual.commands);
    gui.set_window_open("W", false);
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("W", ScrollFlags::NONE);
    assert!(gui
        .plot_with(
            "P",
            1_000_000,
            |_| panic!("suppressed getter"),
            PlotOptions::default()
        )
        .unwrap()
        .is_none());
    assert!(!gui.curve_editor("C", &mut cb, [200., 100.]).unwrap());
    assert!(!gui.gradient_editor("G", &mut gb, [200., 60.]).unwrap());
    assert_eq!(ca.keys(), cb.keys());
    assert_eq!(ga.stops(), gb.stops());
    gui.end_window();
    assert!(gui.render().vertices.is_empty());
}
