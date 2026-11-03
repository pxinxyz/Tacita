use tacita::{
    data::{
        CurveEditor, CurveInterpolation, CurveKey, DataTableOptions, DataTableState,
        GradientEditor, NumericEdit, PlotOptions, Selection, SortSpec,
    },
    ui::Ui,
    ScrollFlags,
};

struct Row {
    id: usize,
    name: String,
    group: String,
    score: i32,
    enabled: bool,
}
pub(super) struct State {
    labels: Vec<String>,
    selection: Selection,
    rows: Vec<Row>,
    table: DataTableState,
    freeze_columns: bool,
    freeze_rows: bool,
    last_sort: Option<SortSpec>,
    float: f32,
    integer: i32,
    float_edit: NumericEdit,
    int_edit: NumericEdit,
    enabled: bool,
    mode: usize,
    clipped_rows: usize,
    menu_actions: u32,
    samples: Vec<f32>,
    curve: CurveEditor,
    gradient: GradientEditor,
    auto_range: bool,
    plot_axes: bool,
    editors_disabled: bool,
    curve_dirty: bool,
    gradient_dirty: bool,
}
impl Default for State {
    fn default() -> Self {
        let mut table = DataTableState::default();
        table.widths[..5].copy_from_slice(&[90., 210., 160., 140., 160.]);
        Self {
            labels: (0..10_000).map(|i| format!("resource {i:05}")).collect(),
            selection: Selection::new(10_000).expect("bounded list"),
            rows: (0..512)
                .map(|i| Row {
                    id: i,
                    name: format!("asset_{i:04}"),
                    group: format!("group {}", i % 7),
                    score: ((i * 73) % 1000) as i32,
                    enabled: i % 3 == 0,
                })
                .collect(),
            table,
            freeze_columns: true,
            freeze_rows: false,
            last_sort: None,
            float: 1.25,
            integer: 12,
            float_edit: NumericEdit::default(),
            int_edit: NumericEdit::default(),
            enabled: true,
            mode: 1,
            clipped_rows: 0,
            menu_actions: 0,
            samples: (0..100_000)
                .map(|i| {
                    if i == 12345 {
                        3.2
                    } else {
                        ((i as f32) * 0.003).sin() * 0.8
                    }
                })
                .collect(),
            curve: CurveEditor::new(&[
                CurveKey {
                    interpolation: CurveInterpolation::Cubic,
                    out_slope: 2.,
                    ..CurveKey::linear(0., 0.1)
                },
                CurveKey {
                    interpolation: CurveInterpolation::Cubic,
                    in_slope: 0.,
                    out_slope: -1.,
                    ..CurveKey::linear(0.5, 0.85)
                },
                CurveKey {
                    in_slope: -1.,
                    ..CurveKey::linear(1., 0.25)
                },
            ])
            .expect("valid curve"),
            gradient: GradientEditor::default(),
            auto_range: true,
            plot_axes: true,
            editors_disabled: false,
            curve_dirty: false,
            gradient_dirty: false,
        }
    }
}
impl State {
    fn draw_editors(&mut self, ui: &mut Ui<'_>) {
        let width = ui.available_width().clamp(100., 600.);
        let accent = ui.theme_color(tacita::theme::ThemeRole::Accent);
        ui.label("Plots / 100,000 samples with one narrow peak");
        ui.checkbox("Auto range", &mut self.auto_range);
        ui.same_line();
        ui.checkbox("Plot axes", &mut self.plot_axes);
        let _ = ui.plot(
            "##samples",
            &self.samples,
            PlotOptions {
                size: [width, 165.],
                range: if self.auto_range {
                    None
                } else {
                    Some([-1.2, 1.2])
                },
                axes: self.plot_axes,
                color: accent,
            },
        );
        ui.label("Hover for the original sample. Manual range clips the peak; auto reveals it.");
        ui.checkbox("Disable editors", &mut self.editors_disabled);
        ui.disabled("editor_disabled", self.editors_disabled, |ui| {
            ui.label("Curve / drag keys and white tangent handles");
            self.curve_dirty |= ui
                .curve_editor("##curve", &mut self.curve, [width, 180.])
                .unwrap_or(false);
            ui.label(
                "Double-click empty space to add; right-click/Delete removes; Escape cancels drag.",
            );
            ui.label("Select a key, then choose step / linear / cubic below the graph.");
            let values: [f32; 128] =
                std::array::from_fn(|i| self.curve.evaluate(i as f32 / 127.).unwrap_or(0.));
            let _ = ui.sparkline("Live curve evaluation", &values, [width, 60.], accent);
            ui.label("Gradient / double-click preview to add; drag interior markers");
            self.gradient_dirty |= ui
                .gradient_editor("##gradient", &mut self.gradient, [width, 80.])
                .unwrap_or(false);
            ui.label("Select a marker for RGBA color. Endpoints stay fixed; Escape cancels drag.");
        });
        if !ui.mouse_down(0) {
            if self.curve_dirty {
                println!(
                    "[data] curve edited: {} keys, selected {:?}",
                    self.curve.keys().len(),
                    self.curve.selected()
                );
                self.curve_dirty = false;
            }
            if self.gradient_dirty {
                println!(
                    "[data] gradient edited: {} stops, selected {:?}",
                    self.gradient.stops().len(),
                    self.gradient.selected()
                );
                self.gradient_dirty = false;
            }
        }
    }

    pub(super) fn draw(&mut self, ui: &mut Ui<'_>) {
        ui.label("Data widgets");
        ui.menu_bar("data_menu", |ui, bar| {
            let _ = ui.menu(bar, "Actions", [245., 215.], |ui, bar| {
                if ui.menu_item(bar, "Clear selection", "", true) {
                    self.selection.clear();
                    self.menu_actions += 1;
                    println!("[data] selection cleared from menu");
                }
                if ui.menu_item(bar, "Reset numeric values", "", true) {
                    self.float = 1.25;
                    self.integer = 12;
                    self.float_edit = NumericEdit::default();
                    self.int_edit = NumericEdit::default();
                    self.menu_actions += 1;
                    println!("[data] numeric defaults restored");
                }
                ui.separator();
                let _ = ui.submenu("More actions", [210., 125.], |ui| {
                    if ui.menu_item(bar, "Select first item", "", true) {
                        self.selection.select(0, false, false);
                        self.menu_actions += 1;
                        println!("[data] first item selected");
                    }
                    ui.menu_item(bar, "Unavailable action", "disabled", false);
                });
            });
            let _ = ui.menu(bar, "About", [230., 100.], |ui, bar| {
                if ui.menu_item(bar, "Log menu action", "", true) {
                    self.menu_actions += 1;
                    println!("[data] menu action {}", self.menu_actions);
                }
            });
        });
        ui.label("Click Actions; hover About to switch. More actions opens a submenu.");
        ui.separator();
        self.draw_editors(ui);
        ui.separator();
        ui.label("Multi-select: plain click / Ctrl toggle / Shift range / Ctrl+Shift add");
        let width = ui.available_width().clamp(100., 600.);
        if ui
            .multi_select_list_with(
                "##multi",
                self.labels.len(),
                |i| Some(self.labels[i].as_str()),
                &mut self.selection,
                [width, 145.],
            )
            .unwrap_or(false)
        {
            println!("[data] selection count {}", self.selection.selected_count());
        }
        ui.label(&format!(
            "{} selected across {} rows",
            self.selection.selected_count(),
            self.labels.len()
        ));
        if ui.button("Clear selected rows") {
            self.selection.clear();
            println!("[data] cleared selection");
        }
        ui.separator();
        ui.label("Resizable, sortable table / pinned header and optional frozen rows");
        ui.checkbox("Freeze first column", &mut self.freeze_columns);
        ui.same_line();
        ui.checkbox("Freeze first row", &mut self.freeze_rows);
        let options = DataTableOptions {
            size: [width, 240.],
            row_height: ui.measure_text("Data")[1] + 8.,
            frozen_columns: usize::from(self.freeze_columns),
            frozen_rows: usize::from(self.freeze_rows),
        };
        let _ = ui.data_table(
            "resources",
            &["ID", "Name", "Group", "Score", "State"],
            self.rows.len(),
            &mut self.table,
            options,
            |ui, table| {
                let indices = (0..table.frozen_rows()).chain(table.visible_rows().range());
                for index in indices {
                    let row = &self.rows[index];
                    for column in 0..5 {
                        ui.data_table_cell(table, index, column, |ui| match column {
                            0 => ui.label(&format!("{:04}", row.id)),
                            1 => ui.label(&row.name),
                            2 => ui.label(&row.group),
                            3 => ui.label(&row.score.to_string()),
                            _ => ui.label(if row.enabled { "enabled" } else { "idle" }),
                        });
                    }
                }
            },
        );
        if self.table.sort != self.last_sort {
            if let Some(sort) = self.table.sort {
                self.rows.sort_by(|a, b| {
                    let order = match sort.column {
                        0 => a.id.cmp(&b.id),
                        1 => a.name.cmp(&b.name),
                        2 => a.group.cmp(&b.group),
                        3 => a.score.cmp(&b.score),
                        _ => a.enabled.cmp(&b.enabled),
                    }
                    .then(a.id.cmp(&b.id));
                    if sort.descending {
                        order.reverse()
                    } else {
                        order
                    }
                });
            } else {
                self.rows.sort_by_key(|r| r.id);
            }
            self.last_sort = self.table.sort;
            println!("[data] host applied sort {:?}", self.table.sort);
        }
        ui.label("Drag header edges; click headers to cycle ascending / descending / none.");
        ui.label("Wheel vertically; bottom scrollbar moves columns without moving frozen ID.");
        ui.separator();
        ui.label("Property grid and direct numeric editing");
        let row_height = ui.measure_text("Property")[1] + 10.;
        let _ = ui.property_grid("properties", 110., row_height, |ui, grid| {
            ui.property_row(grid, "Enabled", |ui| {
                if ui.pill_toggle("##enabled", &mut self.enabled) {
                    println!("[data] enabled {}", self.enabled);
                }
            });
            ui.property_row(grid, "Gain", |ui| {
                if ui
                    .input_float(
                        "##gain",
                        &mut self.float,
                        &mut self.float_edit,
                        0.25,
                        0.,
                        10.,
                    )
                    .unwrap_or(false)
                {
                    println!("[data] gain {}", self.float);
                }
            });
            ui.property_row(grid, "Count", |ui| {
                if ui
                    .input_int("##count", &mut self.integer, &mut self.int_edit, 1, 0, 1000)
                    .unwrap_or(false)
                {
                    println!("[data] count {}", self.integer);
                }
            });
            ui.property_row(grid, "Mode", |ui| {
                if ui.combo("##mode", &["Quiet", "Balanced", "Fast"], &mut self.mode) {
                    println!("[data] mode {}", self.mode);
                }
            });
        });
        ui.label("Type then Enter/click away to commit; Escape cancels; +/- clamps.");
        ui.label("Try '-', NaN or overflowing integers: value stays, field marks invalid.");
        ui.separator();
        ui.label("Reusable clipper / 100,000 custom rows");
        ui.child("custom_rows", [width, 135.], ScrollFlags::BAR, |ui| {
            let height = ui.measure_text("Row")[1] + 8.;
            let _ = ui.rows("rows", 100_000, height, |ui, rows| {
                self.clipped_rows = rows.visible_rows().range().len();
                for index in rows.visible_rows().range() {
                    ui.row(rows, index, |ui| {
                        if ui.button(&format!("custom row {index:05}")) {
                            println!("[data] custom row {index}");
                        }
                    });
                }
            });
        });
        ui.label(&format!(
            "Only {} custom row bodies declared this frame",
            self.clipped_rows
        ));
    }
}
