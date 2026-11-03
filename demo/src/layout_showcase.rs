use super::DemoState;
use tacita::font::{FontId, FontInfo};
use tacita::layout::{
    resolve_layout, LayoutAlign, LayoutItem, LayoutJustify, LayoutKind, LayoutSpec,
};
use tacita::svg::{SvgError, SvgIconId};
use tacita::text::TextOptions;
use tacita::ui::Ui;

const SVG_ICONS: [&str; 3] = [
    r#"<svg viewBox="0 0 24 24"><path d="M12 1 Q12 12 23 12 Q12 12 12 23 Q12 12 1 12 Q12 12 12 1Z"/></svg>"#,
    r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="10" cy="10" r="6.5"/><path d="M15 15L21 21"/></svg>"#,
    r#"<svg viewBox="0 0 24 24"><path fill-rule="evenodd" d="M12 2 A10 10 0 1 1 12 22 A10 10 0 1 1 12 2Z M12 6 A6 6 0 1 0 12 18 A6 6 0 1 0 12 6Z"/><path d="M12 9 Q12 12 15 12 Q12 12 12 15 Q12 12 9 12 Q12 12 12 9Z"/></svg>"#,
];
pub(super) fn register_icons(
    mut add: impl FnMut(&str, [f32; 2]) -> Result<SvgIconId, SvgError>,
) -> Result<[SvgIconId; 9], SvgError> {
    let mut ids = [SvgIconId::INVALID; 9];
    for (i, source) in SVG_ICONS.iter().enumerate() {
        for (j, size) in [18.0, 24.0, 32.0].iter().enumerate() {
            ids[i * 3 + j] = add(source, [*size; 2])?;
        }
    }
    Ok(ids)
}
pub(super) fn icons(ui: &mut Ui<'_>, ids: &[SvgIconId; 9], color: [u8; 4]) {
    ui.label("Registered SVG icons: 18 / 24 / 32 px");
    for i in 0..3 {
        for (j, size) in [18.0, 24.0, 32.0].iter().enumerate() {
            if j > 0 {
                ui.same_line();
            }
            if let Err(error) = ui.svg_icon(ids[i * 3 + j], [*size; 2], color) {
                ui.label(&format!("SVG: {error:?}"));
            }
        }
    }
    ui.label("Curved fill / round stroke / compound path with a hole.");
}
pub(super) struct State {
    width: f32,
    kind: usize,
    align: usize,
    justify: usize,
    columns: i32,
    gap: f32,
    padding: f32,
    auto: bool,
    grow: bool,
    pub(super) clicks: [u32; 3],
}
impl Default for State {
    fn default() -> Self {
        Self {
            width: 420.0,
            kind: 0,
            align: 0,
            justify: 0,
            columns: 2,
            gap: 8.0,
            padding: 8.0,
            auto: false,
            grow: true,
            clicks: [0; 3],
        }
    }
}
pub(super) fn show(ui: &mut Ui<'_>, state: &mut DemoState, faces: &[(FontId, FontInfo)]) {
    ui.filled_slider_float(
        "Container width",
        &mut state.general_layout.width,
        120.0..=620.0,
    );
    ui.combo(
        "Container",
        &["Row / flex", "Column", "Grid"],
        &mut state.general_layout.kind,
    );
    ui.combo(
        "Alignment",
        &["Start", "Center", "End", "Stretch", "Baseline"],
        &mut state.general_layout.align,
    );
    ui.combo(
        "Main distribution",
        &["Start", "Center", "End", "Space between"],
        &mut state.general_layout.justify,
    );
    ui.filled_slider_int("Grid columns", &mut state.general_layout.columns, 1..=3);
    ui.filled_slider_float("Gap", &mut state.general_layout.gap, 0.0..=24.0);
    ui.filled_slider_float("Padding", &mut state.general_layout.padding, 0.0..=24.0);
    ui.checkbox("Automatic width", &mut state.general_layout.auto);
    ui.same_line();
    ui.checkbox("Grow 1 : 2 : 1", &mut state.general_layout.grow);
    let active = ui.active_font();
    let info = faces
        .iter()
        .find(|(id, _)| *id == active)
        .map(|(_, f)| *f)
        .unwrap_or(faces[0].1);
    let row = info.line_height + state.style.frame_padding[1] * 2.0;
    let kind = [LayoutKind::Row, LayoutKind::Column, LayoutKind::Grid][state.general_layout.kind];
    let mut align = [
        LayoutAlign::Start,
        LayoutAlign::Center,
        LayoutAlign::End,
        LayoutAlign::Stretch,
        LayoutAlign::Baseline,
    ][state.general_layout.align];
    if kind == LayoutKind::Column && align == LayoutAlign::Baseline {
        align = LayoutAlign::Start;
    }
    let width = state.general_layout.width.min(ui.available_size()[0]);
    let spec = LayoutSpec {
        kind,
        size: [
            if state.general_layout.auto {
                0.0
            } else {
                width
            },
            0.0,
        ],
        padding: [state.general_layout.padding; 4],
        gap: [state.general_layout.gap; 2],
        align,
        justify: [
            LayoutJustify::Start,
            LayoutJustify::Center,
            LayoutJustify::End,
            LayoutJustify::SpaceBetween,
        ][state.general_layout.justify],
        columns: state.general_layout.columns as usize,
        ..LayoutSpec::default()
    };
    let mut items = core::array::from_fn::<_, 3, _>(|i| LayoutItem {
        preferred: [100.0, row + 12.0],
        min: [48.0, row + 12.0],
        max: [if i == 0 { 160.0 } else { 400.0 }, (row + 12.0).max(120.0)],
        grow: if state.general_layout.grow {
            if i == 1 {
                2.0
            } else {
                1.0
            }
        } else {
            0.0
        },
        margin: [2.0; 4],
        baseline: info.ascent + 6.0,
        ..LayoutItem::default()
    });
    items[1].preferred[0] = 130.0;
    let resolved = resolve_layout(spec, &items).expect("valid demo layout");
    ui.label(&format!(
        "Resolved {:.0} x {:.0} (minimums remain firm; overflow clips)",
        resolved.size[0], resolved.size[1]
    ));
    ui.layout("Flex/grid showcase", spec, &items, |ui, layout| {
        for i in 0..3 {
            ui.layout_item(layout, i, |ui, rect| {
                ui.draw_rect_filled(
                    [rect[0], rect[1]],
                    [rect[2], rect[3]],
                    state.style.widget_bg,
                );
                if ui.button(["First", "Second", "Third"][i]) {
                    state.general_layout.clicks[i] += 1;
                    eprintln!("[demo] layout item {i} clicked");
                }
            })
            .expect("ordered demo items");
        }
    })
    .expect("valid demo layout");
    ui.label(&format!(
        "Clicks: {} / {} / {}",
        state.general_layout.clicks[0],
        state.general_layout.clicks[1],
        state.general_layout.clicks[2]
    ));

    ui.collapsing_header("Nested auto-sizing and clips", |ui| {
        let children = [
            LayoutItem::fixed([130.0, row]),
            LayoutItem::fixed([130.0, row]),
        ];
        let inner = LayoutSpec {
            kind: LayoutKind::Row,
            gap: [8.0; 2],
            ..LayoutSpec::default()
        };
        let size = resolve_layout(inner, &children)
            .expect("intrinsic row")
            .size;
        let outer = [
            LayoutItem::fixed(size),
            LayoutItem::fixed([size[0], info.line_height]),
        ];
        ui.layout(
            "Auto column",
            LayoutSpec {
                padding: [10.0; 4],
                ..LayoutSpec::default()
            },
            &outer,
            |ui, parent| {
                ui.layout_item(parent, 0, |ui, _| {
                    ui.layout("Nested row", inner, &children, |ui, child| {
                        ui.layout_item(child, 0, |ui, _| {
                            ui.checkbox("Nested check", &mut state.nested_enabled);
                        })
                        .expect("child");
                        ui.layout_item(child, 1, |ui, rect| {
                            ui.draw_text_at(
                                [rect[0], rect[1]],
                                "This long label is clipped inside its item",
                                state.style.text,
                            );
                        })
                        .expect("child");
                    })
                    .expect("nested row");
                })
                .expect("parent");
                ui.layout_item(parent, 1, |ui, _| {
                    ui.label("Column follows the nested row.");
                })
                .expect("parent");
            },
        )
        .expect("auto column");
        ui.button("Ordinary control after auto-sized column");
    });
    ui.collapsing_header("Mixed-font baseline alignment", |ui| {
        let large = faces
            .iter()
            .find(|(_, f)| f.px_size == 32.0)
            .copied()
            .unwrap_or(faces[0]);
        let items = [
            LayoutItem {
                baseline: info.ascent,
                ..LayoutItem::fixed([115.0, info.line_height])
            },
            LayoutItem {
                baseline: large.1.ascent,
                ..LayoutItem::fixed([160.0, large.1.line_height])
            },
        ];
        ui.layout(
            "Baseline row",
            LayoutSpec {
                kind: LayoutKind::Row,
                align: LayoutAlign::Baseline,
                ..LayoutSpec::default()
            },
            &items,
            |ui, layout| {
                ui.layout_item(layout, 0, |ui, rect| {
                    ui.draw_text_at([rect[0], rect[1]], "Regular", state.style.text);
                    ui.draw_line(
                        [rect[0], rect[1] + info.ascent],
                        [rect[2], rect[1] + info.ascent],
                        [100, 180, 220, 160],
                        1.0,
                    );
                })
                .expect("baseline");
                ui.layout_item(layout, 1, |ui, rect| {
                    ui.with_font(large.0, |ui| {
                        ui.label("Large");
                    });
                    ui.draw_line(
                        [rect[0], rect[1] + large.1.ascent],
                        [rect[2], rect[1] + large.1.ascent],
                        [100, 180, 220, 160],
                        1.0,
                    );
                })
                .expect("baseline");
            },
        )
        .expect("baseline row");
    });
    ui.collapsing_header("Anchors, absolute placement and layers", |ui| {
        let width = state
            .general_layout
            .width
            .min(ui.available_size()[0])
            .max(140.0);
        let items = [
            LayoutItem {
                absolute: true,
                offset: [8.0, 8.0],
                layer: 20,
                ..LayoutItem::fixed([180.0, row + 20.0])
            },
            LayoutItem {
                absolute: true,
                offset: [38.0, 20.0],
                layer: 0,
                ..LayoutItem::fixed([180.0, row + 20.0])
            },
            LayoutItem {
                absolute: true,
                anchor: [1.0, 1.0],
                offset: [-6.0, -6.0],
                layer: 30,
                hit_test: false,
                ..LayoutItem::fixed([110.0, info.line_height])
            },
        ];
        ui.layout(
            "Layered canvas",
            LayoutSpec {
                size: [width, row + 70.0],
                ..LayoutSpec::default()
            },
            &items,
            |ui, layout| {
                for i in 0..3 {
                    ui.layout_item(layout, i, |ui, rect| {
                        if i < 2 {
                            ui.draw_rect_filled(
                                [rect[0], rect[1]],
                                [rect[2], rect[3]],
                                if i == 0 {
                                    state.style.widget_bg_hovered
                                } else {
                                    state.style.widget_bg
                                },
                            );
                            if ui.button(if i == 0 { "Upper layer" } else { "Lower layer" }) {
                                state.general_layout.clicks[i] += 1;
                                eprintln!("[demo] layer {i} clicked");
                            }
                        } else {
                            ui.draw_text_box(
                                rect,
                                "bottom-right",
                                state.style.text,
                                TextOptions::default(),
                            );
                        }
                    })
                    .expect("layer");
                }
            },
        )
        .expect("layer canvas");
        ui.label("Upper layer paints last and owns overlap clicks; bottom-right stays anchored.");
    });
}
