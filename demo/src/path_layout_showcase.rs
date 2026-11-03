use restricted_gui::{
    layout::{
        ContentAlign, GridCell, LayoutAlign, LayoutItem, LayoutKind, LayoutOptions, LayoutSpec,
    },
    paths::{DashPattern, LineCap, LineJoin, StrokeStyle},
    style::Style,
    ui::Ui,
};

pub(super) struct State {
    join: usize,
    cap: usize,
    width: f32,
    miter_limit: f32,
    tolerance: f32,
    dash_phase: f32,
    closed: bool,
    odd: bool,
    reverse: bool,
    bend: f32,
    layout_width: f32,
    layout_height: f32,
    column: bool,
    grow: bool,
    content: usize,
    auto_cell: bool,
    clicks: [u32; 12],
}
impl Default for State {
    fn default() -> Self {
        Self {
            join: 2,
            cap: 2,
            width: 8.0,
            miter_limit: 4.0,
            tolerance: 0.25,
            dash_phase: 0.0,
            closed: true,
            odd: true,
            reverse: false,
            bend: 0.5,
            layout_width: 410.0,
            layout_height: 160.0,
            column: false,
            grow: true,
            content: 3,
            auto_cell: true,
            clicks: [0; 12],
        }
    }
}
pub(super) fn paths(ui: &mut Ui<'_>, state: &mut State, theme: Style) {
    ui.label("Connected paths / joins / caps / adaptive curves / concave fills");
    ui.combo("Join", &["Miter", "Bevel", "Round"], &mut state.join);
    ui.combo("Cap", &["Butt", "Square", "Round"], &mut state.cap);
    ui.filled_slider_float("Stroke width", &mut state.width, 1.0..=28.0);
    ui.collapsing_header("Path controls", |ui| {
        ui.filled_slider_float("Miter limit", &mut state.miter_limit, 1.0..=8.0);
        ui.filled_slider_float("Physical tolerance", &mut state.tolerance, 0.05..=1.0);
        ui.filled_slider_float("Curve bend", &mut state.bend, 0.0..=1.0);
        ui.filled_slider_float("Dash phase", &mut state.dash_phase, -80.0..=80.0);
        ui.checkbox("Closed dashed contour", &mut state.closed);
        ui.checkbox("Odd dash pattern", &mut state.odd);
        ui.checkbox("Reverse polygon winding", &mut state.reverse);
    });
    let stroke = StrokeStyle {
        width: state.width,
        join: [LineJoin::Miter, LineJoin::Bevel, LineJoin::Round][state.join],
        cap: [LineCap::Butt, LineCap::Square, LineCap::Round][state.cap],
        miter_limit: state.miter_limit,
        tolerance: state.tolerance,
    };
    let width = ui.available_size()[0].clamp(160.0, 520.0);
    let accent = theme.widget_bg_active;
    let secondary = theme.text;
    ui.preview_canvas("Connected paths##gallery", [width, 440.0], |ui, rect| {
        let p = |x: f32, y: f32| [rect[0] + 16.0 + x * (width - 32.0), rect[1] + y];
        ui.draw_text_at(p(0.0, 8.0), "1 / shared joins and end caps", secondary);
        let line = [
            p(0.04, 78.0),
            p(0.25, 42.0),
            p(0.44, 82.0),
            p(0.58, 42.0),
            p(0.66, 80.0),
            p(0.94, 58.0),
        ];
        ui.draw_polyline(&line, false, stroke, accent)
            .expect("valid showcase line");
        ui.draw_text_at(p(0.0, 98.0), "2 / quadratic and cubic curves", secondary);
        let q = [
            p(0.04, 165.0),
            p(0.25, 122.0 - 40.0 * state.bend),
            p(0.45, 165.0),
        ];
        let c = [
            p(0.55, 165.0),
            p(0.58, 100.0 + state.bend * 30.0),
            p(0.88, 210.0 - state.bend * 30.0),
            p(0.96, 130.0),
        ];
        let guide = StrokeStyle {
            width: 1.0,
            ..Default::default()
        };
        ui.draw_polyline(
            &q,
            false,
            guide,
            [secondary[0], secondary[1], secondary[2], 80],
        )
        .unwrap();
        ui.draw_polyline(
            &c,
            false,
            guide,
            [secondary[0], secondary[1], secondary[2], 80],
        )
        .unwrap();
        ui.draw_quadratic_bezier(q, stroke, accent).unwrap();
        ui.draw_cubic_bezier(c, stroke, [125, 207, 255, 255])
            .unwrap();
        ui.draw_text_at(
            p(0.0, 202.0),
            "3 / continuous dashes through corners",
            secondary,
        );
        let dash = [
            p(0.04, 240.0),
            p(0.72, 240.0),
            p(0.96, 286.0),
            p(0.20, 286.0),
        ];
        ui.draw_dashed_polyline(
            &dash,
            state.closed,
            stroke,
            DashPattern {
                lengths: if state.odd {
                    &[18.0, 9.0, 5.0]
                } else {
                    &[18.0, 9.0]
                },
                phase: state.dash_phase,
            },
            accent,
        )
        .unwrap();
        ui.draw_text_at(p(0.0, 306.0), "4 / concave notches stay empty", secondary);
        let mut notch = [
            p(0.05, 338.0),
            p(0.43, 338.0),
            p(0.43, 360.0),
            p(0.20, 360.0),
            p(0.20, 416.0),
            p(0.05, 416.0),
        ];
        let mut star = [
            p(0.76, 333.0),
            p(0.80, 356.0),
            p(0.96, 359.0),
            p(0.83, 378.0),
            p(0.88, 414.0),
            p(0.76, 395.0),
            p(0.64, 414.0),
            p(0.69, 378.0),
            p(0.56, 359.0),
            p(0.72, 356.0),
        ];
        if state.reverse {
            notch.reverse();
            star.reverse();
        }
        ui.draw_concave_polygon(&notch, accent).unwrap();
        ui.draw_concave_polygon(&star, [125, 207, 255, 255])
            .unwrap();
    });
    ui.label("AA follows Settings > anti-aliasing. Compare at your monitor DPI.");
    if ui.button("Try invalid bow-tie (draws nothing)") {
        let rejected = ui.draw_concave_polygon(
            &[[0.0, 0.0], [40.0, 40.0], [0.0, 40.0], [40.0, 0.0]],
            accent,
        );
        eprintln!("[demo] invalid polygon: {rejected:?}");
    }
}
pub(super) fn layouts(ui: &mut Ui<'_>, state: &mut State, theme: Style) {
    ui.label("Wrapping flex and spanning grids");
    ui.filled_slider_float("Wrap width", &mut state.layout_width, 180.0..=520.0);
    ui.filled_slider_float("Wrap height", &mut state.layout_height, 100.0..=260.0);
    ui.checkbox("Wrap columns instead of rows", &mut state.column);
    ui.checkbox("Grow each line independently", &mut state.grow);
    ui.combo(
        "Line distribution",
        &["Start", "Center", "End", "Stretch", "Space between"],
        &mut state.content,
    );
    let width = state.layout_width.min(ui.available_size()[0]).max(100.0);
    let spec = LayoutSpec {
        kind: if state.column {
            LayoutKind::Column
        } else {
            LayoutKind::Row
        },
        size: [width, state.layout_height],
        padding: [8.0; 4],
        gap: [8.0; 2],
        align: LayoutAlign::Stretch,
        ..Default::default()
    };
    let items = core::array::from_fn::<_, 8, _>(|i| LayoutItem {
        preferred: [if i % 3 == 0 { 120.0 } else { 90.0 }, 40.0],
        min: [72.0, 32.0],
        grow: if state.grow { 1.0 } else { 0.0 },
        ..Default::default()
    });
    let options = LayoutOptions {
        wrap: true,
        align_content: [
            ContentAlign::Start,
            ContentAlign::Center,
            ContentAlign::End,
            ContentAlign::Stretch,
            ContentAlign::SpaceBetween,
        ][state.content],
    };
    ui.layout_extended(
        "Wrapped controls##gallery",
        spec,
        &items,
        options,
        &[],
        |ui, layout| {
            for i in 0..items.len() {
                ui.layout_item(layout, i, |ui, rect| {
                    ui.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], theme.widget_bg);
                    if ui.button(&format!("Cell {i}")) {
                        state.clicks[i] += 1;
                        eprintln!("[demo] wrap cell {i}: {}", state.clicks[i]);
                    }
                })
                .unwrap();
            }
        },
    )
    .unwrap();
    ui.label("Grid: 2-column header / 2-row rail / automatic cells");
    ui.checkbox("Automatic trailing cell", &mut state.auto_cell);
    let grid = LayoutSpec {
        kind: LayoutKind::Grid,
        columns: 3,
        size: [width, 180.0],
        padding: [8.0; 4],
        gap: [8.0; 2],
        align: LayoutAlign::Stretch,
        ..Default::default()
    };
    let items = [LayoutItem {
        preferred: [100.0, 44.0],
        min: [28.0, 32.0],
        ..Default::default()
    }; 4];
    let cells = [
        GridCell::at(0, 0, 1, 2),
        GridCell::at(0, 2, 2, 1),
        GridCell::at(1, 0, 1, 1),
        if state.auto_cell {
            GridCell::default()
        } else {
            GridCell::at(1, 1, 1, 1)
        },
    ];
    ui.layout_extended(
        "Spanning controls##gallery",
        grid,
        &items,
        LayoutOptions::default(),
        &cells,
        |ui, layout| {
            for i in 0..4 {
                ui.layout_item(layout, i, |ui, rect| {
                    let color = if i < 2 {
                        theme.widget_bg_active
                    } else {
                        theme.widget_bg
                    };
                    ui.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], color);
                    if ui.button(["Header 1x2", "Rail 2x1", "Body A", "Body B"][i]) {
                        state.clicks[8 + i] += 1;
                        eprintln!("[demo] spanning cell {i}: {}", state.clicks[8 + i]);
                    }
                })
                .unwrap();
            }
        },
    )
    .unwrap();
    ui.label(&format!("Live clicks: {:?}", state.clicks));
    ui.label("Resize the width: row/column breaks and hit regions resolve in this frame.");
}
