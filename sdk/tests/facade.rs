use tacita::context::Context;
use tacita::draw::{DrawCmd, DrawVert};
use tacita::input::InputState;
use tacita::style::Style;
use tacita::ui::Ui;
use tacita::ScrollFlags;
use tacita_sdk::{icon, FontId, Gui, IconFont};

const DEJAVU_SANS: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

#[test]
fn connected_paths_and_extended_layout_forward_exactly_and_suppress() {
    use tacita_sdk::{
        ContentAlign, DashPattern, GridCell, LayoutAlign, LayoutItem, LayoutKind, LayoutOptions,
        LayoutSpec, LineCap, LineJoin, StrokeStyle,
    };
    let line = [[100., 120.], [170., 120.], [190., 170.]];
    let quadratic = [[100., 200.], [170., 150.], [240., 200.]];
    let cubic = [[100., 250.], [120., 190.], [210., 310.], [240., 250.]];
    let polygon = [
        [100., 320.],
        [160., 320.],
        [160., 340.],
        [120., 340.],
        [120., 380.],
        [100., 380.],
    ];
    let style = StrokeStyle {
        width: 6.,
        join: LineJoin::Round,
        cap: LineCap::Square,
        ..Default::default()
    };
    let dash = DashPattern {
        lengths: &[15., 5., 3.],
        phase: -4.,
    };
    let items = [LayoutItem {
        preferred: [80., 32.],
        min: [32., 24.],
        grow: 1.,
        ..Default::default()
    }; 3];
    let wrap = LayoutSpec {
        size: [180., 100.],
        gap: [8.; 2],
        align: LayoutAlign::Stretch,
        ..Default::default()
    };
    let options = LayoutOptions {
        wrap: true,
        align_content: ContentAlign::Stretch,
    };
    let grid = LayoutSpec {
        kind: LayoutKind::Grid,
        columns: 3,
        ..wrap
    };
    let cells = [
        GridCell::at(0, 0, 1, 2),
        GridCell::at(0, 2, 2, 1),
        GridCell::default(),
    ];
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let expected = ctx_frame(&mut ctx, AWAY, false, 0., |ui| {
        ui.window("Paths", ScrollFlags::NONE, |ui| {
            ui.draw_polyline(&line, false, style, [255; 4]).unwrap();
            ui.draw_dashed_polyline(&line, true, style, dash, [255; 4])
                .unwrap();
            ui.draw_quadratic_bezier(quadratic, style, [255; 4])
                .unwrap();
            ui.draw_cubic_bezier(cubic, style, [255; 4]).unwrap();
            ui.draw_concave_polygon(&polygon, [255; 4]).unwrap();
            for (spec, options, cells) in [
                (wrap, options, &[][..]),
                (grid, LayoutOptions::default(), &cells[..]),
            ] {
                ui.layout_extended(
                    if spec.kind == LayoutKind::Grid {
                        "Grid"
                    } else {
                        "Wrap"
                    },
                    spec,
                    &items,
                    options,
                    cells,
                    |ui, layout| {
                        for i in 0..3 {
                            ui.layout_item(layout, i, |ui, _| {
                                ui.button("Cell");
                            })
                            .unwrap();
                        }
                    },
                )
                .unwrap();
            }
        })
    });
    let actual = gui_frame(&mut gui, AWAY, false, 0., |g| {
        g.begin_window("Paths", ScrollFlags::NONE);
        g.draw_polyline(&line, false, style, [255; 4]).unwrap();
        g.draw_dashed_polyline(&line, true, style, dash, [255; 4])
            .unwrap();
        g.draw_quadratic_bezier(quadratic, style, [255; 4]).unwrap();
        g.draw_cubic_bezier(cubic, style, [255; 4]).unwrap();
        g.draw_concave_polygon(&polygon, [255; 4]).unwrap();
        for (spec, options, cells) in [
            (wrap, options, &[][..]),
            (grid, LayoutOptions::default(), &cells[..]),
        ] {
            g.begin_layout_extended(
                if spec.kind == LayoutKind::Grid {
                    "Grid"
                } else {
                    "Wrap"
                },
                spec,
                &items,
                options,
                cells,
            )
            .unwrap();
            for i in 0..3 {
                g.begin_layout_item(i).unwrap();
                g.button("Cell");
                g.end_layout_item();
            }
            g.end_layout();
        }
        g.end_window();
    });
    assert_eq!(actual, expected);
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let expected = gui_frame(&mut gui, AWAY, false, 0., |g| {
        g.begin_window("Closed", ScrollFlags::NONE);
        g.begin_collapsing_header("Closed paths");
        g.end_collapsing_header();
        g.end_window();
    });
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let actual = gui_frame(&mut gui, AWAY, false, 0., |g| {
        g.begin_window("Closed", ScrollFlags::NONE);
        g.begin_collapsing_header("Closed paths");
        g.draw_polyline(&line, false, style, [255; 4]).unwrap();
        g.draw_dashed_polyline(&line, false, style, dash, [255; 4])
            .unwrap();
        g.draw_quadratic_bezier(quadratic, style, [255; 4]).unwrap();
        g.draw_cubic_bezier(cubic, style, [255; 4]).unwrap();
        g.draw_concave_polygon(&polygon, [255; 4]).unwrap();
        g.begin_layout_extended("L", wrap, &items, options, &[])
            .unwrap();
        g.begin_layout_item(0).unwrap();
        g.button("Hidden");
        g.end_layout_item();
        g.end_layout();
        g.end_collapsing_header();
        g.end_window();
    });
    assert_eq!(actual, expected);
}

#[test]
fn paints_and_mask_scopes_match_core_including_effect_tables() {
    use tacita_sdk::{
        AlphaMask, BorderPlacement, GradientKind, GradientMask, GradientPaint, GradientStop,
        RoundedRect,
    };
    let shape = RoundedRect::new([100.0, 100.0, 240.0, 180.0], [12.0, 0.0, 24.0, 4.0]).unwrap();
    let stops = [
        GradientStop {
            position: 0.0,
            color: [255; 4],
        },
        GradientStop {
            position: 1.0,
            color: [99, 91, 255, 255],
        },
    ];
    let paint = GradientPaint {
        kind: GradientKind::Radial,
        origin: [0.5; 2],
        vector: [0.5; 2],
        ..Default::default()
    };
    let alpha = AlphaMask {
        texture: tacita::draw::TextureId(42),
        rect: shape.rect,
        uv: [0.0, 0.0, 1.0, 1.0],
    };
    let fade = GradientMask {
        start: [100.0, 100.0],
        end: [240.0, 100.0],
        opacity: [1.0, 0.0],
    };
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let expected = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("Paint", ScrollFlags::NONE, |ui| {
            ui.alpha_mask(alpha, |ui| {
                ui.gradient_mask(fade, |ui| {
                    ui.draw_shape_gradient(shape, &stops, paint).unwrap();
                    ui.draw_gradient_border(shape, &stops, paint, 1.0, BorderPlacement::Inner)
                        .unwrap();
                    ui.button("Masked");
                })
                .unwrap();
            })
            .unwrap();
            ui.button("Unmasked");
        });
    });
    let actual = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Paint", ScrollFlags::NONE);
        g.begin_alpha_mask(alpha).unwrap();
        g.begin_gradient_mask(fade).unwrap();
        g.draw_shape_gradient(shape, &stops, paint).unwrap();
        g.draw_gradient_border(shape, &stops, paint, 1.0, BorderPlacement::Inner)
            .unwrap();
        g.button("Masked");
        g.end_mask();
        g.end_mask();
        g.button("Unmasked");
        g.end_window();
    });
    assert_eq!(actual, expected);
    assert!(actual.3.iter().any(|e| e.masks.count == 2));
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let suppressed = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Closed", ScrollFlags::NONE);
        g.begin_collapsing_header("Initially closed");
        g.begin_alpha_mask(alpha).unwrap();
        g.begin_gradient_mask(fade).unwrap();
        g.draw_shape_gradient(shape, &stops, paint).unwrap();
        g.end_mask();
        g.end_mask();
        g.end_collapsing_header();
        g.end_window();
    });
    assert!(suppressed.3.is_empty());
}

#[test]
fn shape_and_rounded_scope_forwards_match_and_suppress_closed_content() {
    use tacita_sdk::{BorderPlacement, RoundedRect, Shadow};
    let shape = RoundedRect::new([90.0, 100.0, 250.0, 200.0], [24.0, 0.0, 12.0, 8.0]).unwrap();
    let shadow = Shadow {
        offset: [2.0, 3.0],
        spread: 1.0,
        blur: 8.0,
        color: [0, 0, 0, 90],
        inset: false,
    };
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let expected = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("shape", ScrollFlags::NONE, |ui| {
            ui.draw_shadow(shape, shadow).unwrap();
            ui.draw_glow(shape, [99, 91, 255, 40], 8.0).unwrap();
            ui.draw_rounded_rect(shape, [20, 24, 32, 255]).unwrap();
            ui.draw_rounded_border(shape, [255; 4], 1.0, BorderPlacement::Inner)
                .unwrap();
            ui.rounded_clip("clip", shape, |ui| {
                ui.label("Text and button");
                ui.button("Clipped");
            })
            .unwrap();
        });
    });
    let actual = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("shape", ScrollFlags::NONE);
        g.draw_shadow(shape, shadow).unwrap();
        g.draw_glow(shape, [99, 91, 255, 40], 8.0).unwrap();
        g.draw_rounded_rect(shape, [20, 24, 32, 255]).unwrap();
        g.draw_rounded_border(shape, [255; 4], 1.0, BorderPlacement::Inner)
            .unwrap();
        g.begin_rounded_clip("clip", shape).unwrap();
        g.label("Text and button");
        g.button("Clipped");
        g.end_rounded_clip();
        g.end_window();
    });
    assert_eq!(actual, expected);
    gui.set_window_open("shape", false);
    let closed = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("shape", ScrollFlags::NONE);
        g.begin_rounded_clip("clip", shape).unwrap();
        g.draw_shadow(shape, shadow).unwrap();
        g.button("Clipped");
        g.end_rounded_clip();
        g.end_window();
    });
    assert!(closed.0.is_empty());
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let expected = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("shape", ScrollFlags::NONE, |ui| {
            ui.collapsing_header("Closed shape section", |_| {
                panic!("new header unexpectedly open")
            });
        });
    });
    let suppressed = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("shape", ScrollFlags::NONE);
        g.begin_collapsing_header("Closed shape section");
        g.begin_rounded_clip("clip", shape).unwrap();
        assert!(!g.button("Suppressed"));
        g.draw_rounded_rect(shape, [255; 4]).unwrap();
        g.end_rounded_clip();
        g.end_collapsing_header();
        g.end_window();
    });
    assert_eq!(suppressed, expected);
}

#[test]
fn subpixel_mode_forwards_with_identical_text_draw_data() {
    use tacita_sdk::TextRasterMode;
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    assert_eq!(gui.text_raster_mode(), TextRasterMode::Legacy);
    ctx.set_text_raster_mode(TextRasterMode::Subpixel);
    gui.set_text_raster_mode(TextRasterMode::Subpixel);
    let expected = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("phase", ScrollFlags::NONE, |ui| {
            ui.draw_text_at([90.25, 100.5], "Quarter pixel coverage", [255; 4]);
        });
    });
    let actual = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("phase", ScrollFlags::NONE);
        g.draw_text_at([90.25, 100.5], "Quarter pixel coverage", [255; 4]);
        g.end_window();
    });
    assert_eq!(actual, expected);
    assert_eq!(gui.text_raster_mode(), TextRasterMode::Subpixel);
}

#[test]
fn svg_and_nested_layered_layout_match_core_and_suppress_closed_scopes() {
    use tacita_sdk::{LayoutItem, LayoutKind, LayoutSpec, SvgIconId};
    let source = "<svg viewBox='0 0 16 16'><circle cx='8' cy='8' r='6'/></svg>";
    let mut core = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let core_icon = core.add_svg_icon(source, [16.0; 2]).unwrap();
    let icon = gui.add_svg_icon(source, [16.0; 2]).unwrap();
    let items = [
        LayoutItem {
            layer: 5,
            ..LayoutItem::fixed([100.0, 50.0])
        },
        LayoutItem::fixed([100.0, 50.0]),
    ];
    let spec = LayoutSpec {
        kind: LayoutKind::Row,
        padding: [3.0; 4],
        ..LayoutSpec::default()
    };
    let expected = ctx_frame(&mut core, AWAY, false, 0.0, |ui| {
        ui.window("layout", ScrollFlags::NONE, |ui| {
            ui.layout("row", spec, &items, |ui, parent| {
                for i in 0..2 {
                    ui.layout_item(parent, i, |ui, rect| {
                        ui.draw_svg_icon(
                            core_icon,
                            [rect[0], rect[1], rect[0] + 16.0, rect[1] + 16.0],
                            [255; 4],
                        )
                        .unwrap();
                        ui.layout(
                            "nested",
                            LayoutSpec::default(),
                            &[LayoutItem::fixed([80.0, 30.0])],
                            |ui, child| {
                                ui.layout_item(child, 0, |ui, _| {
                                    ui.button("child");
                                })
                                .unwrap();
                            },
                        )
                        .unwrap();
                    })
                    .unwrap();
                }
            })
            .unwrap();
            ui.svg_icon(core_icon, [16.0; 2], [255; 4]).unwrap();
            ui.button("after");
        })
    });
    let actual = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("layout", ScrollFlags::NONE);
        g.begin_layout("row", spec, &items).unwrap();
        for i in 0..2 {
            let rect = g.begin_layout_item(i).unwrap();
            g.draw_svg_icon(
                icon,
                [rect[0], rect[1], rect[0] + 16.0, rect[1] + 16.0],
                [255; 4],
            )
            .unwrap();
            g.begin_layout(
                "nested",
                LayoutSpec::default(),
                &[LayoutItem::fixed([80.0, 30.0])],
            )
            .unwrap();
            g.begin_layout_item(0).unwrap();
            g.button("child");
            g.end_layout_item();
            g.end_layout();
            g.end_layout_item();
        }
        g.end_layout();
        g.svg_icon(icon, [16.0; 2], [255; 4]).unwrap();
        g.button("after");
        g.end_window();
    });
    assert_eq!(actual, expected);
    gui.set_window_open("layout", false);
    let suppressed = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("layout", ScrollFlags::NONE);
        g.begin_layout("hidden", spec, &items).unwrap();
        g.begin_layout_item(0).unwrap();
        g.draw_svg_icon(SvgIconId::INVALID, [0.0; 4], [255; 4])
            .unwrap();
        g.button("hidden");
        g.end_layout_item();
        g.end_layout();
        g.end_window();
    });
    assert!(suppressed.0.is_empty());
}

#[test]
fn rich_text_and_plain_blocks_match_core_and_suppress_closed_scopes() {
    use tacita_sdk::{FontId, TextAlign, TextOptions, TextOverflow, TextSpan};
    let mut core = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let opts = TextOptions {
        wrap: true,
        overflow: TextOverflow::Ellipsis,
        horizontal: TextAlign::Center,
        vertical: TextAlign::End,
        max_lines: 2,
        ..TextOptions::default()
    };
    let spans = [
        TextSpan {
            text: "A borrowed ",
            font: FontId::DEFAULT,
            color: [255; 4],
        },
        TextSpan {
            text: "rich paragraph across runs.\nSecond line.",
            font: FontId::DEFAULT,
            color: [40, 80, 150, 255],
        },
    ];
    let expected = ctx_frame(&mut core, AWAY, false, 0.0, |ui| {
        ui.window("Text", ScrollFlags::NONE, |ui| {
            let measured = ui.measure_rich_text(&spans, 150.0, opts);
            assert_eq!(ui.rich_text(&spans, 150.0, opts), measured);
            ui.text_block("Plain wrapping text after rich text.", 150.0, opts);
            ui.draw_rich_text_box([80.0, 250.0, 240.0, 340.0], &spans, opts);
            ui.draw_text_box([80.0, 350.0, 240.0, 400.0], "Aligned", [255; 4], opts);
        });
    });
    let actual = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Text", ScrollFlags::NONE);
        let measured = g.measure_rich_text(&spans, 150.0, opts);
        assert_eq!(g.rich_text(&spans, 150.0, opts), measured);
        g.text_block("Plain wrapping text after rich text.", 150.0, opts);
        g.draw_rich_text_box([80.0, 250.0, 240.0, 340.0], &spans, opts);
        g.draw_text_box([80.0, 350.0, 240.0, 400.0], "Aligned", [255; 4], opts);
        g.end_window();
    });
    assert_eq!(actual, expected);
    gui.set_window_open("Text", false);
    let data = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Text", ScrollFlags::NONE);
        assert_eq!(g.rich_text(&spans, 150.0, opts).lines, 0);
        g.text_block("Hidden", 150.0, opts);
        g.draw_rich_text_box([0.0; 4], &spans, opts);
        g.end_window();
    });
    assert!(data.0.is_empty());
}

#[test]
fn timeline_gradient_layout_and_theme_forwards_match_core() {
    use tacita_sdk::{
        Easing, GradientOptions, GradientStop, Keyframe, MotionSettings, ThemeTokens,
        TimelineOptions,
    };
    let mut core = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let motion = MotionSettings {
        enabled: true,
        ..MotionSettings::default()
    };
    core.set_motion_settings(motion);
    gui.set_motion_settings(motion);
    let tokens = ThemeTokens::default();
    core.set_theme_tokens(tokens);
    gui.set_theme_tokens(tokens);
    assert_eq!(gui.theme_tokens(), core.theme_tokens());
    let keys = [
        Keyframe {
            time: 0.0,
            value: 0.0,
            easing: Easing::Linear,
        },
        Keyframe {
            time: 1.0,
            value: 1.0,
            easing: Easing::Linear,
        },
    ];
    let stops = [
        GradientStop {
            position: 0.0,
            color: [0; 4],
        },
        GradientStop {
            position: 1.0,
            color: [255; 4],
        },
    ];
    for frame in 0..8 {
        let rect = [80.0 + frame as f32 * 5.0, 100.0, 260.0, 200.0];
        core.set_input(input(AWAY, false, 0.0));
        core.new_frame(0.02);
        let value =
            core.animate_timeline(tacita::id::Id(9), &keys, TimelineOptions::default());
        core.ui().window("Motion", ScrollFlags::NONE, |ui| {
            ui.animated_layout("Region", rect, 0.2, Easing::Linear, |ui, bounds| {
                ui.draw_animated_gradient(
                    bounds,
                    &stops,
                    GradientOptions::default(),
                    frame as f32 * 0.1,
                    1.0,
                );
                ui.button("Live");
            });
        });
        let data = core.render();
        let expected = (
            data.vertices.to_vec(),
            data.indices.to_vec(),
            data.commands.to_vec(),
        );
        gui.new_frame(input(AWAY, false, 0.0), 0.02);
        assert_eq!(
            gui.animate_timeline(tacita_sdk::Id(9), &keys, TimelineOptions::default()),
            value
        );
        gui.begin_window("Motion", ScrollFlags::NONE);
        let bounds = gui.begin_animated_layout("Region", rect, 0.2, Easing::Linear);
        gui.draw_animated_gradient(
            bounds,
            &stops,
            GradientOptions::default(),
            frame as f32 * 0.1,
            1.0,
        );
        gui.button("Live");
        gui.end_animated_layout();
        gui.end_window();
        let data = gui.render();
        assert_eq!(
            (
                data.vertices.to_vec(),
                data.indices.to_vec(),
                data.commands.to_vec()
            ),
            expected
        );
    }
    gui.set_window_open("Motion", false);
    gui.new_frame(input(AWAY, false, 0.0), 0.02);
    gui.begin_window("Motion", ScrollFlags::NONE);
    gui.begin_animated_layout("Suppressed", [0.0; 4], 0.1, Easing::Linear);
    gui.button("Hidden");
    gui.end_animated_layout();
    gui.end_window();
    assert!(gui.render().vertices.is_empty());
}

#[test]
fn motion_channels_and_animated_containers_match_core() {
    use tacita_sdk::{Easing, Id, MotionSettings};
    let settings = MotionSettings {
        enabled: true,
        ..MotionSettings::default()
    };
    let mut core = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    core.set_motion_settings(settings);
    gui.set_motion_settings(settings);
    assert_eq!(gui.motion_settings(), settings);
    let mut core_pill = false;
    let mut gui_pill = false;
    let mut core_selected = 0;
    let mut gui_selected = 0;
    let header_y = 60.0
        + core.font().line_height()
        + 2.0 * core.style.frame_padding[1]
        + core.style.window_padding[1]
        + 5.0;
    let mut content_frames = 0;
    for frame in 0..48 {
        let target = if frame < 12 { 1.0 } else { 0.0 };
        let header_click = matches!(frame, 3 | 4 | 28 | 29);
        let snapshot = input(
            if header_click { [85.0, header_y] } else { AWAY },
            matches!(frame, 3 | 28),
            0.0,
        );
        core.set_input(snapshot);
        core.new_frame(0.016);
        let mut core_values = (0.0, 0.0, [0; 4]);
        core.ui().window("Motion", ScrollFlags::BAR, |ui| {
            ui.collapsing_header("Reveal", |ui| {
                content_frames += 1;
                ui.label("Header body");
                ui.tree_node("Nested tree", |ui| {
                    ui.label("Tree body");
                });
            });
            core_values = (
                ui.animate_float(Id(1), 0.0, target, 0.4, Easing::Linear),
                ui.animate_spring(Id(1), 0.0, target, 0.2),
                ui.animate_color(
                    Id(1),
                    [0; 4],
                    if target == 1.0 { [255; 4] } else { [0; 4] },
                    0.4,
                    Easing::Linear,
                ),
            );
            ui.pill_toggle("Pill", &mut core_pill);
            ui.segmented_control("Choice", &["A", "B"], &mut core_selected);
            if frame == 12 {
                ui.open_modal(Id(22));
            }
            ui.modal(Id(22), [400.0, 100.0, 640.0, 240.0], |ui| {
                ui.button("Popup button");
                if frame == 16 {
                    ui.close_popup();
                }
            });
        });
        let data = core.render();
        let expected = (
            data.vertices.to_vec(),
            data.indices.to_vec(),
            data.commands.to_vec(),
        );
        gui.new_frame(snapshot, 0.016);
        gui.begin_window("Motion", ScrollFlags::BAR);
        gui.begin_collapsing_header("Reveal");
        gui.label("Header body");
        gui.begin_tree_node("Nested tree");
        gui.label("Tree body");
        gui.end_tree_node();
        gui.end_collapsing_header();
        let values = (
            gui.animate_float(Id(1), 0.0, target, 0.4, Easing::Linear),
            gui.animate_spring(Id(1), 0.0, target, 0.2),
            gui.animate_color(
                Id(1),
                [0; 4],
                if target == 1.0 { [255; 4] } else { [0; 4] },
                0.4,
                Easing::Linear,
            ),
        );
        assert_eq!(values, core_values);
        gui.pill_toggle("Pill", &mut gui_pill);
        gui.segmented_control("Choice", &["A", "B"], &mut gui_selected);
        if frame == 12 {
            gui.open_modal(Id(22));
        }
        if gui.begin_modal(Id(22), [400.0, 100.0, 640.0, 240.0]) {
            gui.button("Popup button");
            if frame == 16 {
                gui.close_popup();
            }
        }
        gui.end_popup();
        gui.end_window();
        let data = gui.render();
        assert_eq!(
            (
                data.vertices.to_vec(),
                data.indices.to_vec(),
                data.commands.to_vec()
            ),
            expected
        );
    }
    assert!(content_frames > 0, "must exercise live reveal scopes");
}

const AWAY: [f32; 2] = [-1000.0, -1000.0];

const FIRST_WINDOW_POS: [f32; 2] = [60.0, 60.0];

type Frame = (
    Vec<DrawVert>,
    Vec<u32>,
    Vec<DrawCmd>,
    Vec<tacita::effects::DrawEffects>,
);

fn input(mouse: [f32; 2], down: bool, wheel: f32) -> InputState<'static> {
    InputState {
        mouse_pos: mouse,
        mouse_down: [down, false, false, false, false],
        mouse_wheel: wheel,
        keys: &[],
        viewport_size: [0.0, 0.0],
    }
}

fn gui_frame(
    gui: &mut Gui,
    mouse: [f32; 2],
    down: bool,
    wheel: f32,
    declare: impl FnOnce(&mut Gui),
) -> Frame {
    gui.new_frame(input(mouse, down, wheel), 0.016);
    declare(gui);
    let data = gui.render();
    (
        data.vertices.to_vec(),
        data.indices.to_vec(),
        data.commands.to_vec(),
        data.effects.to_vec(),
    )
}

fn ctx_frame(
    ctx: &mut Context,
    mouse: [f32; 2],
    down: bool,
    wheel: f32,
    declare: impl FnOnce(&mut Ui),
) -> Frame {
    ctx.set_input(input(mouse, down, wheel));
    ctx.new_frame(0.016);
    {
        let mut ui = ctx.ui();
        declare(&mut ui);
    }
    let data = ctx.render();
    (
        data.vertices.to_vec(),
        data.indices.to_vec(),
        data.commands.to_vec(),
        data.effects.to_vec(),
    )
}

#[test]
fn set_window_rect_matches_core_placement() {
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    gui.set_window_rect("Placed", [245.0, 170.0], [320.0, 210.0]);
    ctx.set_window_rect("Placed", [245.0, 170.0], [320.0, 210.0]);

    let flat = gui_frame(&mut gui, AWAY, false, 0.0, |gui| {
        gui.begin_window("Placed", ScrollFlags::NONE);
        gui.label("inside");
        gui.end_window();
    });
    let core = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("Placed", ScrollFlags::NONE, |ui| ui.label("inside"));
    });
    assert_eq!(flat, core);
    assert!(flat.0.iter().any(|vertex| vertex.pos[0] > 245.0));
}

#[test]
fn new_widgets_forward_with_identical_draw_data() {
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let mut flat_selected = 1;
    let mut core_selected = 1;
    let mut flat_pill = false;
    let mut core_pill = false;
    let mut flat_segment = 1;
    let mut core_segment = 1;
    let mut flat_float = 0.4;
    let mut core_float = 0.4;
    let mut flat_int = 3;
    let mut core_int = 3;
    let mut flat_range_f = (0.2, 0.8);
    let mut core_range_f = (0.2, 0.8);
    let mut flat_range_i = (2, 8);
    let mut core_range_i = (2, 8);
    let mut flat_color = [120, 90, 200, 255];
    let mut core_color = flat_color;
    let flat = gui_frame(&mut gui, AWAY, false, 0.0, |gui| {
        gui.begin_window("New widgets", ScrollFlags::NONE);
        gui.radio_button("first", &mut flat_selected, 0);
        gui.radio_button("second", &mut flat_selected, 1);
        gui.pill_toggle("Pill", &mut flat_pill);
        gui.segmented_control("Mode", &["Quiet", "Bright"], &mut flat_segment);
        gui.filled_slider_float("Fill", &mut flat_float, 0.0..=1.0);
        gui.filled_slider_int("Count", &mut flat_int, 0..=10);
        gui.range_slider_float(
            "Range F",
            &mut flat_range_f.0,
            &mut flat_range_f.1,
            0.0..=1.0,
        );
        gui.range_slider_int("Range I", &mut flat_range_i.0, &mut flat_range_i.1, 0..=10);
        gui.drag_float("Drag F", &mut flat_float, 0.1, 0.0..=1.0);
        gui.drag_int("Drag I", &mut flat_int, 0.1, 0..=10);
        gui.progress_bar("work", 0.4, [180.0, 18.0]);
        gui.spinner("Loading", 22.0, 1.0);
        gui.color_picker_rgba("Tint", &mut flat_color);
        gui.begin_tree_node("details");
        gui.label("suppressed while closed");
        gui.end_tree_node();
        gui.end_window();
    });
    let core = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("New widgets", ScrollFlags::NONE, |ui| {
            ui.radio_button("first", &mut core_selected, 0);
            ui.radio_button("second", &mut core_selected, 1);
            ui.pill_toggle("Pill", &mut core_pill);
            ui.segmented_control("Mode", &["Quiet", "Bright"], &mut core_segment);
            ui.filled_slider_float("Fill", &mut core_float, 0.0..=1.0);
            ui.filled_slider_int("Count", &mut core_int, 0..=10);
            ui.range_slider_float(
                "Range F",
                &mut core_range_f.0,
                &mut core_range_f.1,
                0.0..=1.0,
            );
            ui.range_slider_int("Range I", &mut core_range_i.0, &mut core_range_i.1, 0..=10);
            ui.drag_float("Drag F", &mut core_float, 0.1, 0.0..=1.0);
            ui.drag_int("Drag I", &mut core_int, 0.1, 0..=10);
            ui.progress_bar("work", 0.4, [180.0, 18.0]);
            ui.spinner("Loading", 22.0, 1.0);
            ui.color_picker_rgba("Tint", &mut core_color);
            ui.tree_node("details", |ui| ui.label("suppressed while closed"));
        });
    });
    assert_eq!(flat_selected, core_selected);
    assert_eq!(flat_pill, core_pill);
    assert_eq!(flat_segment, core_segment);
    assert_eq!(flat_float, core_float);
    assert_eq!(flat_int, core_int);
    assert_eq!(flat_range_f, core_range_f);
    assert_eq!(flat_range_i, core_range_i);
    assert_eq!(flat_color, core_color);
    assert_eq!(flat, core);
    assert!(!flat.0.is_empty());
}

fn keystone_flat(gui: &mut Gui, state: &mut (bool, f32, i32, String)) {
    let (check, gain, count, name) = state;
    gui.begin_window("Main", ScrollFlags::NONE);
    gui.label("status: ok");
    let _ = gui.button("Apply");
    gui.checkbox("Enabled", check);
    gui.slider_float("Gain", gain, 0.0..=1.0);
    gui.slider_int("Count", count, 0..=10);
    gui.text_input("Name", name);
    gui.end_window();
}

fn keystone_closure(ui: &mut Ui, state: &mut (bool, f32, i32, String)) {
    let (check, gain, count, name) = state;
    ui.window("Main", ScrollFlags::NONE, |ui| {
        ui.label("status: ok");
        let _ = ui.button("Apply");
        ui.checkbox("Enabled", check);
        ui.slider_float("Gain", gain, 0.0..=1.0);
        ui.slider_int("Count", count, 0..=10);
        ui.text_input("Name", name);
    });
}

#[test]
fn flat_sequence_matches_closure_sequence_byte_identically() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let mut flat_state = (true, 0.5, 3, String::from("abc"));
    let mut closure_state = (true, 0.5, 3, String::from("abc"));
    for frame in 0..3 {
        let flat = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
            keystone_flat(g, &mut flat_state)
        });
        let closure = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
            keystone_closure(ui, &mut closure_state)
        });
        assert_eq!(flat, closure, "frame {frame} diverged");
        assert!(!flat.0.is_empty());
        assert!(!flat.1.is_empty());
        assert!(!flat.2.is_empty());
    }
    assert_eq!(flat_state, closure_state);
}

#[test]
fn list_box_forward_matches_core_draw_data() {
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let items = ["one", "two", "three", "four", "five", "six"];
    let mut flat_selected = 1;
    let mut core_selected = 1;
    for wheel in [0.0, -1.0, 0.0] {
        let flat = gui_frame(&mut gui, [100.0, 120.0], false, wheel, |gui| {
            gui.begin_window("Main", ScrollFlags::NONE);
            assert!(!gui.list_box("Items", &items, &mut flat_selected, [180.0, 90.0]));
            gui.end_window();
        });
        let core = ctx_frame(&mut ctx, [100.0, 120.0], false, wheel, |ui| {
            ui.window("Main", ScrollFlags::NONE, |ui| {
                assert!(!ui.list_box("Items", &items, &mut core_selected, [180.0, 90.0]));
            });
        });
        assert_eq!(flat, core);
    }
    assert_eq!(flat_selected, core_selected);
}

#[test]
fn searchable_combo_forward_matches_core_draw_data() {
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let items = ["Alpha", "Bravo", "Charlie"];
    let mut flat_selected = 1;
    let mut core_selected = 1;
    let mut flat_query = String::new();
    let mut core_query = String::new();
    let flat = gui_frame(&mut gui, AWAY, false, 0.0, |gui| {
        gui.begin_window("Main", ScrollFlags::NONE);
        assert!(!gui.searchable_combo("Find", &items, &mut flat_selected, &mut flat_query));
        gui.end_window();
    });
    let core = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("Main", ScrollFlags::NONE, |ui| {
            assert!(!ui.searchable_combo("Find", &items, &mut core_selected, &mut core_query));
        });
    });
    assert_eq!(flat, core);
    assert_eq!((flat_selected, flat_query), (core_selected, core_query));
}

struct Points {
    header: [f32; 2],
    region: [f32; 2],
    region_rect: [f32; 4],
    beta: [f32; 2],
}

fn points_from(style: &Style, mut measure: impl FnMut(&str) -> [f32; 2]) -> Points {
    let line_height = measure("X")[1];
    let row_h = line_height + 2.0 * style.frame_padding[1];
    let spacing = style.item_spacing;
    let origin = [
        FIRST_WINDOW_POS[0] + style.window_padding[0],
        FIRST_WINDOW_POS[1] + row_h + style.window_padding[1],
    ];
    let indent = 2.0 * style.frame_padding[0] + line_height * 0.5;
    let region = [
        origin[0] + indent + 10.0,
        origin[1] + row_h + spacing[1] + 10.0,
    ];
    let region_rect = [
        origin[0] + indent,
        origin[1] + row_h + spacing[1],
        origin[0] + indent + 200.0,
        origin[1] + row_h + spacing[1] + 96.0,
    ];
    let alpha_w = measure("Alpha")[0] + 2.0 * style.frame_padding[0];
    let beta_w = measure("Beta")[0] + 2.0 * style.frame_padding[0];
    let tab_row_y = origin[1] + row_h + 2.0 * spacing[1] + 96.0;
    Points {
        header: [origin[0] + 5.0, origin[1] + row_h / 2.0],
        region,
        region_rect,
        beta: [
            origin[0] + alpha_w + spacing[0] + beta_w / 2.0,
            tab_row_y + row_h / 2.0,
        ],
    }
}

fn containers_flat(gui: &mut Gui) {
    gui.begin_window("Main", ScrollFlags::NONE);
    gui.begin_collapsing_header("Settings");
    gui.begin_scroll_region("Log", [200.0, 96.0]);
    for _ in 0..20 {
        gui.label("log line");
    }
    gui.end_scroll_region();
    gui.end_collapsing_header();
    gui.begin_tab_bar("Tabs");
    if gui.tab("Alpha") {
        gui.label("alpha body");
    }
    if gui.tab("Beta") {
        gui.label("beta body");
    }
    gui.end_tab_bar();
    gui.end_window();
}

fn containers_closure(ui: &mut Ui) {
    ui.window("Main", ScrollFlags::NONE, |ui| {
        ui.collapsing_header("Settings", |ui| {
            ui.scroll_region("Log", [200.0, 96.0], |ui| {
                for _ in 0..20 {
                    ui.label("log line");
                }
            });
        });
        ui.tab_bar("Tabs", |tabs| {
            tabs.tab("Alpha", |ui| {
                ui.label("alpha body");
            });
            tabs.tab("Beta", |ui| {
                ui.label("beta body");
            });
        });
    });
}

fn container_frame(
    gui: &mut Gui,
    ctx: &mut Context,
    flat: bool,
    mouse: [f32; 2],
    down: bool,
    wheel: f32,
) -> Frame {
    if flat {
        gui_frame(gui, mouse, down, wheel, containers_flat)
    } else {
        ctx_frame(ctx, mouse, down, wheel, containers_closure)
    }
}

fn container_script(flat: bool) -> (Vec<Frame>, Points) {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let mut frames = Vec::new();
    let mut pts = None;
    if flat {
        frames.push(gui_frame(&mut gui, AWAY, false, 0.0, |g| {
            let style = *g.style();
            pts = Some(points_from(&style, |t| g.measure_text(t)));
            containers_flat(g);
        }));
    } else {
        let style = ctx.style;
        frames.push(ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
            pts = Some(points_from(&style, |t| ui.measure_text(t)));
            containers_closure(ui);
        }));
    }
    let pts = pts.expect("the settle frame ran");
    frames.push(container_frame(
        &mut gui, &mut ctx, flat, pts.header, true, 0.0,
    ));
    frames.push(container_frame(
        &mut gui, &mut ctx, flat, pts.header, false, 0.0,
    ));
    frames.push(container_frame(&mut gui, &mut ctx, flat, AWAY, false, 0.0));
    frames.push(container_frame(
        &mut gui, &mut ctx, flat, pts.region, false, -1.0,
    ));
    frames.push(container_frame(
        &mut gui, &mut ctx, flat, pts.region, false, -1.0,
    ));
    frames.push(container_frame(
        &mut gui, &mut ctx, flat, pts.beta, true, 0.0,
    ));
    frames.push(container_frame(
        &mut gui, &mut ctx, flat, pts.beta, false, 0.0,
    ));
    frames.push(container_frame(&mut gui, &mut ctx, flat, AWAY, false, 0.0));
    (frames, pts)
}

#[test]
fn container_sequences_match_byte_identically() {
    let (closure, _) = container_script(false);
    assert_ne!(closure[0], closure[3], "header click had no effect");
    assert_ne!(closure[3], closure[5], "wheel had no effect");
    assert_ne!(closure[3], closure[8], "tab click had no effect");
    let (flat, pts) = container_script(true);
    assert_eq!(closure, flat);
    let inset = 1.0 + 4.0 * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
    let content_clip = [
        pts.region_rect[0] + inset,
        pts.region_rect[1] + inset,
        pts.region_rect[2] - inset,
        pts.region_rect[3] - inset,
    ];
    assert!(
        flat[4].2.iter().any(|cmd| cmd.clip_rect == content_clip),
        "no draw command carries the region's inset content clip"
    );
}

#[test]
fn closed_header_content_is_suppressed_like_the_closure_not_running() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let flat = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.begin_collapsing_header("Settings");
        g.label("hidden");
        let mut b = false;
        let _ = g.button("hidden button");
        g.checkbox("hidden check", &mut b);
        g.end_collapsing_header();
        g.label("visible");
        g.end_window();
    });
    let closure = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("Main", ScrollFlags::NONE, |ui| {
            ui.collapsing_header("Settings", |ui| {
                ui.label("hidden");
                let mut b = false;
                let _ = ui.button("hidden button");
                ui.checkbox("hidden check", &mut b);
            });
            ui.label("visible");
        });
    });
    assert_eq!(flat, closure);
}

#[test]
fn closed_header_suppresses_canvas_text_and_capture_too() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let mut captured = true;
    let flat = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.begin_collapsing_header("Settings");
        g.draw_line([100.0, 100.0], [150.0, 120.0], [255, 0, 0, 255], 1.0);
        g.draw_rect_filled([100.0, 100.0], [150.0, 120.0], [0, 255, 0, 255]);
        g.draw_rect_outline([100.0, 100.0], [150.0, 120.0], [0, 0, 255, 255], 1.0);
        g.draw_circle_filled([120.0, 110.0], 10.0, [255, 255, 0, 255]);
        g.draw_circle_outline([120.0, 110.0], 12.0, [255, 0, 255, 255], 1.0);
        g.draw_arc([120.0, 110.0], 14.0, 0.0..=3.0, [0, 255, 255, 255], 1.0);
        g.draw_polygon_filled(
            &[[100.0, 100.0], [150.0, 100.0], [125.0, 130.0]],
            [128, 128, 128, 255],
        );
        g.draw_text_at([100.0, 140.0], "hidden text", [255, 255, 255, 255]);
        let id = g.widget_id("custom");
        g.capture(id);
        captured = g.is_captured(id);
        g.release_capture(id);
        g.end_collapsing_header();
        g.label("visible");
        g.end_window();
    });
    assert!(!captured, "a suppressed capture must not take");
    let closure = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("Main", ScrollFlags::NONE, |ui| {
            ui.collapsing_header("Settings", |ui| {
                ui.draw_line([100.0, 100.0], [150.0, 120.0], [255, 0, 0, 255], 1.0);
                ui.draw_rect_filled([100.0, 100.0], [150.0, 120.0], [0, 255, 0, 255]);
                ui.draw_rect_outline([100.0, 100.0], [150.0, 120.0], [0, 0, 255, 255], 1.0);
                ui.draw_circle_filled([120.0, 110.0], 10.0, [255, 255, 0, 255]);
                ui.draw_circle_outline([120.0, 110.0], 12.0, [255, 0, 255, 255], 1.0);
                ui.draw_arc([120.0, 110.0], 14.0, 0.0..=3.0, [0, 255, 255, 255], 1.0);
                ui.draw_polygon_filled(
                    &[[100.0, 100.0], [150.0, 100.0], [125.0, 130.0]],
                    [128, 128, 128, 255],
                );
                ui.draw_text_at([100.0, 140.0], "hidden text", [255, 255, 255, 255]);
            });
            ui.label("visible");
        });
    });
    assert_eq!(flat, closure);
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let emitting = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.begin_collapsing_header("Settings");
        g.end_collapsing_header();
        g.draw_line([100.0, 100.0], [150.0, 120.0], [255, 0, 0, 255], 1.0);
        g.label("visible");
        g.end_window();
    });
    assert!(emitting.0.len() > flat.0.len());
}

#[test]
#[should_panic(expected = "Gui::end_window called with no window open")]
fn end_window_with_depth_zero_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.end_window();
}

#[test]
#[should_panic(expected = "Gui::render called with open scopes")]
fn render_with_an_open_window_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main", ScrollFlags::NONE);
    let _ = gui.render();
}

#[test]
#[should_panic(expected = "Gui::render called with open scopes")]
fn render_with_an_open_container_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main", ScrollFlags::NONE);
    gui.begin_scroll_region("Log", [200.0, 96.0]);
    let _ = gui.render();
}

#[test]
#[should_panic(expected = "Gui::begin_window called while a window scope is open")]
fn nested_begin_window_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("A", ScrollFlags::NONE);
    gui.begin_window("B", ScrollFlags::NONE);
}

#[test]
#[should_panic(expected = "innermost open scope is a scroll region")]
fn mismatched_container_end_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main", ScrollFlags::NONE);
    gui.begin_scroll_region("Log", [200.0, 96.0]);
    gui.end_collapsing_header();
}

#[test]
#[should_panic(expected = "Gui::end_tab_bar called with no tab bar open")]
fn end_tab_bar_without_an_open_bar_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main", ScrollFlags::NONE);
    gui.end_tab_bar();
}

#[test]
#[should_panic(expected = "Gui::end_window called while a container scope is open")]
fn end_window_with_an_open_container_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("A", ScrollFlags::NONE);
    gui.begin_scroll_region("Log", [200.0, 96.0]);
    gui.end_window();
}

#[test]
#[should_panic(expected = "Gui::begin_window called while a container scope is open")]
fn begin_window_with_an_open_container_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("A", ScrollFlags::NONE);
    gui.begin_collapsing_header("Settings");
    gui.begin_window("B", ScrollFlags::NONE);
}

#[test]
fn closed_window_begin_end_is_a_graceful_noop_emitting_nothing() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.end_window();
    });
    gui.set_window_open("Main", false);
    let frame = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.label("hidden");
        let _ = g.button("hidden");
        g.begin_collapsing_header("Hidden");
        g.label("also hidden");
        g.end_collapsing_header();
        g.end_window();
    });
    assert!(frame.0.is_empty());
    assert!(frame.1.is_empty());
    assert!(frame.2.is_empty());
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let core = ctx_frame(&mut ctx, AWAY, false, 0.0, |_| {});
    assert_eq!(frame, core);
}

fn child_flat(gui: &mut Gui, flags: ScrollFlags) {
    gui.begin_window("Main", ScrollFlags::NONE);
    gui.begin_child("Child", [200.0, 80.0], flags);
    for i in 0..10 {
        gui.label(&format!("line {i}"));
    }
    gui.end_child();
    gui.end_window();
}

fn child_closure(ui: &mut Ui, flags: ScrollFlags) {
    ui.window("Main", ScrollFlags::NONE, |ui| {
        ui.child("Child", [200.0, 80.0], flags, |ui| {
            for i in 0..10 {
                ui.label(&format!("line {i}"));
            }
        });
    });
}

#[test]
fn child_scope_matches_core_byte_identically_for_none_and_wheel() {
    for flags in [ScrollFlags::NONE, ScrollFlags::WHEEL] {
        let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
        let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
        let flat = gui_frame(&mut gui, AWAY, false, 0.0, |g| child_flat(g, flags));
        let closure = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| child_closure(ui, flags));
        assert_eq!(flat, closure, "flags {flags:?} diverged");
        assert!(!flat.0.is_empty());
        assert!(!flat.2.is_empty());
    }
}

#[test]
#[should_panic(expected = "Gui::end_child called with no child open")]
fn end_child_with_no_child_open_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main", ScrollFlags::NONE);
    gui.end_child();
}

#[test]
#[should_panic(expected = "innermost open scope is a scroll region")]
fn end_child_closing_a_scroll_region_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main", ScrollFlags::NONE);
    gui.begin_scroll_region("Log", [200.0, 96.0]);
    gui.end_child();
}

#[test]
#[should_panic(expected = "innermost open scope is a child")]
fn end_scroll_region_closing_a_child_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main", ScrollFlags::NONE);
    gui.begin_child("Child", [200.0, 96.0], ScrollFlags::WHEEL);
    gui.end_scroll_region();
}

#[test]
#[should_panic(expected = "Gui::render called with open scopes")]
fn unclosed_child_at_render_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main", ScrollFlags::NONE);
    gui.begin_child("Child", [200.0, 96.0], ScrollFlags::NONE);
    let _ = gui.render();
}

#[test]
#[should_panic(expected = "current_window_mut called outside a window declaration")]
fn begin_child_with_no_window_open_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_child("Child", [200.0, 96.0], ScrollFlags::NONE);
}

#[test]
fn child_inside_a_closed_window_is_suppressed_and_balances() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.end_window();
    });
    gui.set_window_open("Main", false);
    let frame = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.begin_child("Child", [200.0, 96.0], ScrollFlags::BAR);
        g.label("hidden");
        let _ = g.button("hidden button");
        g.end_child();
        g.end_window();
    });
    assert!(frame.0.is_empty());
    assert!(frame.1.is_empty());
    assert!(frame.2.is_empty());
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let core = ctx_frame(&mut ctx, AWAY, false, 0.0, |_| {});
    assert_eq!(frame, core);
    gui.set_window_open("Main", true);
    let reopened = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.begin_child("Child", [200.0, 96.0], ScrollFlags::BAR);
        g.label("visible");
        g.end_child();
        g.end_window();
    });
    assert!(!reopened.0.is_empty());
}

#[test]
fn begin_window_bar_flags_forward_to_the_core() {
    let declare = |flags: ScrollFlags| {
        move |ui: &mut Ui| {
            ui.window("Main", flags, |ui| {
                for i in 0..40 {
                    ui.label(&format!("overflowing line {i}"));
                }
            });
        }
    };
    let mut ctx_bar = Context::new(DEJAVU_SANS).expect("font loads");
    let core_bar = ctx_frame(&mut ctx_bar, AWAY, false, 0.0, declare(ScrollFlags::BAR));

    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let flat_bar = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::BAR);
        for i in 0..40 {
            g.label(&format!("overflowing line {i}"));
        }
        g.end_window();
    });
    assert_eq!(
        flat_bar, core_bar,
        "Gui::begin_window(.., ScrollFlags::BAR) must match Ui::window(.., ScrollFlags::BAR, ..)"
    );

    let mut ctx_none = Context::new(DEJAVU_SANS).expect("font loads");
    let core_none = ctx_frame(&mut ctx_none, AWAY, false, 0.0, declare(ScrollFlags::NONE));
    assert_ne!(flat_bar, core_none, "BAR must forward and differ from NONE");
}

#[test]
fn atlas_passthroughs_match_the_core() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    assert_eq!(gui.atlas_generation(), ctx.font().atlas_generation());
    assert_eq!(gui.atlas_pixels(), ctx.font().atlas_pixels());
    gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.label("atlas warm-up text");
        g.end_window();
    });
    ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("Main", ScrollFlags::NONE, |ui| {
            ui.label("atlas warm-up text");
        });
    });
    assert_eq!(gui.atlas_generation(), ctx.font().atlas_generation());
    assert_eq!(gui.atlas_pixels(), ctx.font().atlas_pixels());
}

#[test]
fn custom_widget_helpers_forward_identically() {
    let mouse = [100.0, 100.0];
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(input(mouse, true, 0.0), 0.016);
    gui.begin_window("Main", ScrollFlags::NONE);
    let id = gui.widget_id("custom");
    let rect = gui.allocate([50.0, 20.0]);
    let hovered = gui.is_hovered(rect);
    gui.capture(id);
    let captured = gui.is_captured(id);
    gui.release_capture(id);
    let released = gui.is_captured(id);
    let flat = (
        id,
        rect,
        hovered,
        captured,
        released,
        gui.mouse_pos(),
        gui.mouse_pressed(0),
        gui.mouse_down(0),
        gui.mouse_released(0),
    );
    gui.end_window();
    let _ = gui.render();

    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let mut core = None;
    ctx_frame(&mut ctx, mouse, true, 0.0, |ui| {
        ui.window("Main", ScrollFlags::NONE, |ui| {
            let id = ui.widget_id("custom");
            let rect = ui.allocate([50.0, 20.0]);
            let hovered = ui.is_hovered(rect);
            ui.capture(id);
            let captured = ui.is_captured(id);
            ui.release_capture(id);
            let released = ui.is_captured(id);
            core = Some((
                id,
                rect,
                hovered,
                captured,
                released,
                ui.mouse_pos(),
                ui.mouse_pressed(0),
                ui.mouse_down(0),
                ui.mouse_released(0),
            ));
        });
    });
    let core = core.expect("window open");
    assert_eq!(flat, core);
}

const FA_SOLID: &[u8] = include_bytes!("../../core/tests/assets/fa-solid-900.ttf");

#[test]
fn new_with_icons_bakes_icons_and_runs_a_frame() {
    let star = icon::STAR;
    let mut gui = Gui::new_with_icons(
        DEJAVU_SANS,
        &[IconFont {
            bytes: FA_SOLID,
            range: (star as u32, star as u32),
        }],
    )
    .expect("icon bake must succeed");
    gui.new_frame(InputState::default(), 0.016);
    let gen = gui.atlas_generation();
    let icon_text = format!("{star}");
    let icon_w = gui.measure_text(&icon_text)[0];
    assert!(icon_w > 0.0);
    assert_eq!(
        gui.atlas_generation(),
        gen,
        "icon baked at init, no cache-on-miss"
    );
    gui.begin_window("Icons", ScrollFlags::NONE);
    gui.label(&format!("Aim {star}"));
    gui.end_window();
    let _ = gui.render();
}

#[test]
fn new_with_icons_rejects_invalid_icon_bytes() {
    let icons = [IconFont {
        bytes: b"this is not a font",
        range: (0xF005, 0xF005),
    }];
    assert!(Gui::new_with_icons(DEJAVU_SANS, &icons).is_err());
}

#[test]
fn font_scope_sequence_matches_closure_byte_identically() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let big = gui.add_font(DEJAVU_SANS, 24.0).expect("face registers");
    let big_core = ctx.add_font(DEJAVU_SANS, 24.0).expect("face registers");
    assert_eq!(big, big_core);
    let flat = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.label("default face");
        let default_w = g.measure_text("sized text")[0];
        g.push_font(big);
        assert_eq!(g.active_font(), big);
        let big_w = g.measure_text("sized text")[0];
        assert_ne!(default_w, big_w, "the second face must take effect");
        g.label("sized text");
        g.pop_font();
        assert_eq!(g.active_font(), FontId::DEFAULT);
        g.label("default again");
        g.end_window();
    });
    let closure = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("Main", ScrollFlags::NONE, |ui| {
            ui.label("default face");
            let _ = ui.measure_text("sized text");
            ui.with_font(big_core, |ui| {
                let _ = ui.measure_text("sized text");
                ui.label("sized text");
            });
            ui.label("default again");
        });
    });
    assert_eq!(flat, closure);
    assert!(!flat.0.is_empty());
}

#[test]
fn font_queries_track_registration() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let ctx = Context::new(DEJAVU_SANS).expect("font loads");
    assert_eq!(gui.font_count(), 1);
    assert_eq!(gui.active_font(), FontId::DEFAULT);
    assert_eq!(
        gui.font_info(FontId::DEFAULT),
        ctx.font().face_info(FontId::DEFAULT)
    );
    assert!(gui.font_info(FontId(1)).is_none());

    let big = gui.add_font(DEJAVU_SANS, 24.0).expect("face registers");
    assert_eq!(big, FontId(1));
    assert_eq!(gui.font_count(), 2);
    let info = gui.font_info(big).expect("a registered face has metrics");
    assert_eq!(info.px_size, 24.0);

    assert!(gui.add_font(b"this is not a font", 16.0).is_err());
    assert_eq!(gui.font_count(), 2);

    let star = icon::STAR;
    let iconic = gui
        .add_font_with_icons(
            DEJAVU_SANS,
            20.0,
            &[IconFont {
                bytes: FA_SOLID,
                range: (star as u32, star as u32),
            }],
        )
        .expect("icon face registers");
    assert_eq!(gui.font_count(), 3);
    assert_eq!(
        gui.font_info(iconic)
            .expect("a registered face has metrics")
            .px_size,
        20.0
    );

    gui.new_frame(InputState::default(), 0.016);
    gui.push_font(big);
    assert_eq!(gui.active_font(), big);
    gui.pop_font();
    assert_eq!(gui.active_font(), FontId::DEFAULT);
    let _ = gui.render();
}

#[test]
#[should_panic(expected = "Ui::push_font called with an unknown FontId")]
fn push_font_with_an_unknown_id_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.push_font(FontId(9));
}

#[test]
#[should_panic(expected = "Ui::pop_font called with no font scope open")]
fn pop_font_without_a_push_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.pop_font();
}

#[test]
#[should_panic(expected = "Context::render called with an unbalanced font stack")]
fn render_with_an_open_font_scope_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.push_font(FontId::DEFAULT);
    let _ = gui.render();
}

#[test]
fn suppressed_push_pop_font_are_graceful_noops() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let big = gui.add_font(DEJAVU_SANS, 24.0).expect("face registers");
    gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.end_window();
    });
    gui.set_window_open("Main", false);
    let frame = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.push_font(big);
        assert_eq!(g.active_font(), FontId::DEFAULT);
        g.label("hidden");
        g.pop_font();
        g.end_window();
    });
    assert_eq!(gui.active_font(), FontId::DEFAULT);
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let core = ctx_frame(&mut ctx, AWAY, false, 0.0, |_| {});
    assert_eq!(frame, core);
}

#[test]
fn font_scope_inside_a_closed_header_is_suppressed_like_the_closure() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let big = gui.add_font(DEJAVU_SANS, 24.0).expect("face registers");
    let big_core = ctx.add_font(DEJAVU_SANS, 24.0).expect("face registers");
    let flat = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main", ScrollFlags::NONE);
        g.begin_collapsing_header("Settings");
        g.push_font(big);
        assert_eq!(g.active_font(), FontId::DEFAULT);
        g.label("hidden");
        g.pop_font();
        g.end_collapsing_header();
        g.label("visible");
        g.end_window();
    });
    let closure = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("Main", ScrollFlags::NONE, |ui| {
            ui.collapsing_header("Settings", |ui| {
                ui.with_font(big_core, |ui| {
                    ui.label("hidden");
                });
            });
            ui.label("visible");
        });
    });
    assert_eq!(flat, closure);
}

#[test]
fn sdf_and_reveal_forwards_match_core_and_suppress_closed_content() {
    use tacita_sdk::{
        GradientPaint, GradientStop, RevealDirection, RevealMask, RoundedRect, SdfKind, SdfStyle,
    };
    let shape = RoundedRect::new([100.0, 100.0, 240.0, 180.0], [16.0, 0.0, 8.0, 4.0]).unwrap();
    let reveal = RevealMask {
        rect: shape.rect,
        direction: RevealDirection::RightToLeft,
        progress: 0.5,
        softness: 0.1,
        inverted: false,
    };
    let stops = [
        GradientStop {
            position: 0.0,
            color: [255; 4],
        },
        GradientStop {
            position: 1.0,
            color: [99, 91, 255, 255],
        },
    ];
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let expected = ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("SDF", ScrollFlags::NONE, |ui| {
            ui.reveal_mask(reveal, |ui| {
                for kind in [
                    SdfKind::Fill,
                    SdfKind::Border,
                    SdfKind::Glow,
                    SdfKind::EdgeGlow,
                    SdfKind::Highlight,
                ] {
                    ui.draw_sdf_shape(
                        shape,
                        [255; 4],
                        SdfStyle {
                            kind,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
                ui.draw_sdf_shape_gradient(
                    shape,
                    &stops,
                    GradientPaint::default(),
                    SdfStyle::default(),
                )
                .unwrap();
                ui.button("Masked");
            })
            .unwrap();
            ui.button("Restored");
        });
    });
    let actual = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("SDF", ScrollFlags::NONE);
        g.begin_reveal_mask(reveal).unwrap();
        for kind in [
            SdfKind::Fill,
            SdfKind::Border,
            SdfKind::Glow,
            SdfKind::EdgeGlow,
            SdfKind::Highlight,
        ] {
            g.draw_sdf_shape(
                shape,
                [255; 4],
                SdfStyle {
                    kind,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        g.draw_sdf_shape_gradient(shape, &stops, GradientPaint::default(), SdfStyle::default())
            .unwrap();
        g.button("Masked");
        g.end_mask();
        g.button("Restored");
        g.end_window();
    });
    assert_eq!(actual, expected);
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let hidden = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Hidden", ScrollFlags::NONE);
        g.begin_collapsing_header("Closed");
        g.begin_reveal_mask(reveal).unwrap();
        g.draw_sdf_shape(shape, [255; 4], SdfStyle::default())
            .unwrap();
        g.draw_sdf_shape_gradient(shape, &stops, GradientPaint::default(), SdfStyle::default())
            .unwrap();
        g.end_mask();
        g.end_collapsing_header();
        g.end_window();
    });
    assert!(hidden.3.is_empty());
}

#[test]
fn surface_filter_stencil_forwards_match_core_and_suppress_closed_scopes() {
    use tacita_sdk::{ImageFilter, RoundedRect, SurfaceStyle, TextureId, SURFACE_GRID};
    let shape = RoundedRect::new([100., 100., 240., 220.], [20.; 4]).unwrap();
    let style = SurfaceStyle {
        flags: SURFACE_GRID,
        ..Default::default()
    };
    let filter = ImageFilter::default();
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let expected = ctx_frame(&mut ctx, AWAY, false, 0., |ui| {
        ui.window("W", ScrollFlags::NONE, |ui| {
            ui.stencil_clip("S", shape, |ui| {
                ui.draw_surface(shape, None, [0., 0., 1., 1.], [255; 4], style)
                    .unwrap();
                ui.draw_filtered_image(
                    TextureId(42),
                    shape.rect,
                    [0., 0., 1., 1.],
                    [255; 4],
                    filter,
                )
                .unwrap();
            })
            .unwrap();
            ui.button("Outside");
        })
    });
    let actual = gui_frame(&mut gui, AWAY, false, 0., |g| {
        g.begin_window("W", ScrollFlags::NONE);
        g.begin_stencil_clip("S", shape).unwrap();
        g.draw_surface(shape, None, [0., 0., 1., 1.], [255; 4], style)
            .unwrap();
        g.draw_filtered_image(
            TextureId(42),
            shape.rect,
            [0., 0., 1., 1.],
            [255; 4],
            filter,
        )
        .unwrap();
        g.end_stencil_clip();
        g.button("Outside");
        g.end_window();
    });
    assert_eq!(actual, expected);
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    let closed = gui_frame(&mut gui, AWAY, false, 0., |g| {
        g.begin_window("Closed", ScrollFlags::NONE);
        g.begin_collapsing_header("Initially closed");
        g.begin_stencil_clip("S", shape).unwrap();
        g.draw_surface(shape, None, [0., 0., 1., 1.], [255; 4], style)
            .unwrap();
        g.draw_filtered_image(
            TextureId(42),
            shape.rect,
            [0., 0., 1., 1.],
            [255; 4],
            filter,
        )
        .unwrap();
        g.end_stencil_clip();
        g.end_collapsing_header();
        g.end_window();
    });
    assert!(closed.3.is_empty());
}

#[test]
fn images_and_inspection_forward_identical_geometry() {
    use tacita_sdk::{ImageFit, ImageRegion, NineSlice, TextureId};
    let image = ImageRegion {
        texture: TextureId(9),
        uv_min: [0.1; 2],
        uv_max: [0.9; 2],
        size: [32.; 2],
    };
    let slices = NineSlice {
        source: [6.; 4],
        destination: [4.; 4],
    };
    let mut ctx = Context::new(DEJAVU_SANS).unwrap();
    let mut gui = Gui::new(DEJAVU_SANS).unwrap();
    ctx.set_inspection_enabled(true);
    gui.set_inspection_enabled(true);
    ctx.new_frame(0.016);
    ctx.ui().window("image", ScrollFlags::NONE, |ui| {
        ui.image(image, [96., 32.], ImageFit::Contain, [255; 4])
            .unwrap();
        ui.image_button("sprite", image, [60., 40.], ImageFit::Cover, [255; 4])
            .unwrap();
        ui.draw_nine_slice(image, [120., 160., 220., 200.], slices, [255; 4])
            .unwrap();
    });
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("image", ScrollFlags::NONE);
    gui.image(image, [96., 32.], ImageFit::Contain, [255; 4])
        .unwrap();
    gui.image_button("sprite", image, [60., 40.], ImageFit::Cover, [255; 4])
        .unwrap();
    gui.draw_nine_slice(image, [120., 160., 220., 200.], slices, [255; 4])
        .unwrap();
    gui.end_window();
    let expected = ctx.render();
    let actual = gui.render();
    assert_eq!(actual.vertices, expected.vertices);
    assert_eq!(actual.indices, expected.indices);
    assert_eq!(actual.commands, expected.commands);
    assert_eq!(gui.inspection_count(), ctx.inspection_count());
    for i in 0..ctx.inspection_count() {
        assert_eq!(gui.inspection_node(i), ctx.inspection_node(i));
    }
}

#[test]
fn shaped_editors_and_host_text_services_forward_and_suppress() {
    use tacita_sdk::{
        ShapedOptions, ShapedText, TextEditOptions, TextEditor, TextEventKind,
    };
    let mut c = Context::new(DEJAVU_SANS).unwrap();
    let mut g = Gui::new(DEJAVU_SANS).unwrap();
    let mut a = TextEditor::new("office אבג\nسلام").unwrap();
    let mut b = TextEditor::new(a.text()).unwrap();
    let mut ca = ShapedText::default();
    let mut cb = ShapedText::default();
    let options = TextEditOptions {
        size: [220., 100.],
        multiline: true,
        wrap: true,
        ..Default::default()
    };
    let expected = ctx_frame(&mut c, AWAY, false, 0., |ui| {
        ui.window("edit", ScrollFlags::NONE, |ui| {
            ui.text_edit("field", &mut a, options).unwrap();
            ui.shaped_text(&mut ca, "office", ShapedOptions::default())
                .unwrap();
        });
    });
    let actual = gui_frame(&mut g, AWAY, false, 0., |g| {
        g.begin_window("edit", ScrollFlags::NONE);
        g.text_edit("field", &mut b, options).unwrap();
        g.shaped_text(&mut cb, "office", ShapedOptions::default())
            .unwrap();
        g.end_window();
    });
    assert_eq!(actual, expected);
    let mut id = tacita_sdk::Id(0);
    gui_frame(&mut g, AWAY, false, 0., |g| {
        g.begin_window("edit", ScrollFlags::NONE);
        id = g.widget_id("field");
        g.focus(id);
        g.text_edit("field", &mut b, options).unwrap();
        g.end_window();
    });
    assert_eq!(g.text_caret().unwrap().owner, id);
    g.submit_text_event(id, TextEventKind::Commit, "X", 0..0)
        .unwrap();
    gui_frame(&mut g, AWAY, false, 0., |g| {
        g.begin_window("edit", ScrollFlags::NONE);
        assert!(
            g.text_edit("field", &mut b, options)
                .unwrap()
                .unwrap()
                .changed
        );
        g.end_window();
    });
    assert!(b.text().ends_with('X'));
    g.set_window_open("edit", false);
    let before = b.text().to_owned();
    let data = gui_frame(&mut g, AWAY, false, 0., |g| {
        g.begin_window("edit", ScrollFlags::NONE);
        assert!(g.text_edit("field", &mut b, options).unwrap().is_none());
        assert!(g
            .shaped_text(&mut cb, "changed", ShapedOptions::default())
            .unwrap()
            .is_none());
        g.end_window();
    });
    assert!(data.0.is_empty());
    assert_eq!(b.text(), before);
    assert_eq!(cb.text(), "office");
}

#[test]
fn color_font_atlas_and_shaped_geometry_forward_without_tinting() {
    use tacita_sdk::{FontId, ShapedText, TextureId};
    let mut c = Context::new(DEJAVU_SANS).unwrap();
    let mut g = Gui::new(DEJAVU_SANS).unwrap();
    let bytes = include_bytes!("../../core/tests/assets/Noto-COLRv1.ttf");
    let a = c.add_font(bytes, 16.).unwrap();
    let b = g.add_font(bytes, 16.).unwrap();
    assert_eq!(a, b);
    c.set_font_fallback_chain(FontId::DEFAULT, &[a]).unwrap();
    g.set_font_fallback_chain(FontId::DEFAULT, &[b]).unwrap();
    c.set_color_atlas_texture(Some(TextureId(90))).unwrap();
    g.set_color_atlas_texture(Some(TextureId(90))).unwrap();
    let mut ca = ShapedText::default();
    let mut cb = ShapedText::default();
    let expected = ctx_frame(&mut c, AWAY, false, 0., |ui| {
        ui.window("color", ScrollFlags::NONE, |ui| {
            ui.shaped_text(&mut ca, "😀 👩‍💻 🇬🇧", Default::default())
                .unwrap();
        });
    });
    let actual = gui_frame(&mut g, AWAY, false, 0., |g| {
        g.begin_window("color", ScrollFlags::NONE);
        g.shaped_text(&mut cb, "😀 👩‍💻 🇬🇧", Default::default())
            .unwrap();
        g.end_window();
    });
    assert_eq!(actual, expected);
    assert_eq!(g.color_atlas_pixels(), c.font().color_atlas_pixels());
    assert_eq!(
        g.color_atlas_generation(),
        c.font().color_atlas_generation()
    );
    assert_eq!(g.color_atlas_texture(), Some(TextureId(90)));
}
