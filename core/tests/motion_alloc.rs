use restricted_gui::animation::{
    Easing, GradientOptions, GradientStop, Keyframe, MotionSettings, TimelineOptions,
};
use restricted_gui::context::Context;
use restricted_gui::id::Id;
use restricted_gui::input::InputState;
use restricted_gui::ScrollFlags;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counter;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;

fn trajectory(ctx: &mut Context) -> (usize, usize) {
    let row_y = 60.0
        + ctx.font().line_height()
        + 2.0 * ctx.style.frame_padding[1]
        + ctx.style.window_padding[1]
        + 5.0;
    let mut content_frames = 0;
    let mut exit_frames = 0;
    for frame in 0..96 {
        let row_click = matches!(frame, 0 | 1 | 40 | 41);
        ctx.set_input(InputState {
            mouse_pos: if row_click {
                [85.0, row_y]
            } else {
                [-1000.0; 2]
            },
            mouse_down: [matches!(frame, 0 | 40), false, false, false, false],
            viewport_size: [800.0, 600.0],
            ..InputState::default()
        });
        ctx.new_frame(1.0 / 144.0);
        ctx.ui().window("motion", ScrollFlags::NONE, |ui| {
            let spans = [
                restricted_gui::text::TextSpan {
                    text: "Wrapping across ",
                    font: restricted_gui::font::FontId::DEFAULT,
                    color: [255; 4],
                },
                restricted_gui::text::TextSpan {
                    text: "colored spans, café λ and an oversizedword.\nNext line.",
                    font: restricted_gui::font::FontId::DEFAULT,
                    color: [100, 150, 200, 255],
                },
            ];
            let options = restricted_gui::text::TextOptions {
                wrap: true,
                overflow: restricted_gui::text::TextOverflow::Ellipsis,
                max_lines: 3,
                ..restricted_gui::text::TextOptions::default()
            };
            ui.draw_rich_text_box([320.0, 90.0, 460.0, 180.0], &spans, options);
            ui.measure_rich_text(&spans, 140.0, options);
            let paint_stops = [
                GradientStop {
                    position: 0.0,
                    color: [99, 91, 255, 255],
                },
                GradientStop {
                    position: 1.0,
                    color: [0, 229, 255, 255],
                },
            ];
            let paint_shape = restricted_gui::shapes::RoundedRect::new(
                [320.0, 300.0, 460.0, 370.0],
                [12.0, 0.0, 24.0, 8.0],
            )
            .unwrap();
            ui.gradient_mask(
                restricted_gui::effects::GradientMask {
                    start: [320.0, 300.0],
                    end: [460.0, 300.0],
                    opacity: [1.0, frame as f32 / 96.0],
                },
                |ui| {
                    ui.alpha_mask(
                        restricted_gui::effects::AlphaMask {
                            texture: restricted_gui::draw::TextureId(42),
                            rect: paint_shape.rect,
                            uv: [0.0, 0.0, 1.0, 1.0],
                        },
                        |ui| {
                            ui.draw_shape_gradient(
                                paint_shape,
                                &paint_stops,
                                restricted_gui::effects::GradientPaint {
                                    kind: restricted_gui::effects::GradientKind::Conic,
                                    origin: [0.5; 2],
                                    phase: frame as f32 / 96.0,
                                    ..Default::default()
                                },
                            )
                            .unwrap();
                        },
                    )
                    .unwrap();
                },
            )
            .unwrap();
            ui.reveal_mask(
                restricted_gui::effects::RevealMask {
                    rect: paint_shape.rect,
                    direction: restricted_gui::effects::RevealDirection::Radial,
                    progress: frame as f32 / 96.0,
                    softness: 0.15,
                    inverted: frame % 2 == 0,
                },
                |ui| {
                    ui.draw_sdf_shape_gradient(
                        paint_shape,
                        &paint_stops,
                        restricted_gui::effects::GradientPaint {
                            phase: frame as f32 / 96.0,
                            ..Default::default()
                        },
                        restricted_gui::effects::SdfStyle {
                            kind: restricted_gui::effects::SdfKind::Border,
                            width: 1.0 + frame as f32 / 96.0,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    ui.draw_sdf_shape(
                        paint_shape,
                        [255; 4],
                        restricted_gui::effects::SdfStyle {
                            kind: restricted_gui::effects::SdfKind::EdgeGlow,
                            reach: 8.0 + frame as f32 / 96.0,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                },
            )
            .unwrap();
            ui.draw_svg_icon(
                restricted_gui::svg::SvgIconId(0),
                [330.0, 60.0, 346.0, 76.0],
                [255; 4],
            )
            .unwrap();
            let keys = [
                Keyframe {
                    time: 0.0,
                    value: 0.0,
                    easing: Easing::Linear,
                },
                Keyframe {
                    time: 0.2,
                    value: 1.0,
                    easing: Easing::CubicOut,
                },
            ];
            ui.animate_timeline(
                Id(100),
                &keys,
                TimelineOptions {
                    revision: (frame / 32) as u64,
                    ..TimelineOptions::default()
                },
            );
            let x = if (frame / 12) % 2 == 0 { 80.0 } else { 150.0 };
            ui.animated_layout(
                "Live layout",
                [x, 200.0, x + 130.0, 280.0],
                0.1,
                Easing::Linear,
                |ui, rect| {
                    let stops = [
                        GradientStop {
                            position: 0.0,
                            color: [20; 4],
                        },
                        GradientStop {
                            position: 1.0,
                            color: [220; 4],
                        },
                    ];
                    ui.draw_animated_gradient(
                        rect,
                        &stops,
                        GradientOptions::default(),
                        frame as f32 * 0.01,
                        1.0,
                    );
                    ui.button("Live");
                },
            );
            ui.collapsing_header("Reveal", |ui| {
                content_frames += 1;
                ui.label("Revealed body");
                ui.button("Child");
            });
            let mut pill = (frame / 12) % 2 == 0;
            let mut selected = (frame / 12) % 2;
            ui.pill_toggle("Pill", &mut pill);
            ui.segmented_control("Choice", &["One", "Two"], &mut selected);
            ui.nav_row("One", &mut selected, 0);
            ui.nav_row("Two", &mut selected, 1);
            let button = ui.widget_id("Focus");
            ui.focus(button);
            ui.button("Focus");
            let target = if pill { 1.0 } else { 0.0 };
            let value = ui.animate_float(Id(1), 0.0, target, 0.2, Easing::CubicOut);
            ui.animate_spring(Id(1), 0.0, target, 0.15);
            ui.animate_color(
                Id(1),
                [0; 4],
                if pill { [255; 4] } else { [0; 4] },
                0.2,
                Easing::Linear,
            );
            ui.progress_bar("Motion", value, [180.0, 16.0]);
            if frame == 20 {
                ui.open_popup(Id(2));
            }
            ui.popup(Id(2), [400.0, 80.0, 680.0, 180.0], |ui| {
                ui.label("Popup snapshot");
                ui.button("Popup button");
                if frame == 30 {
                    ui.close_popup();
                }
            });
        });
        let data = ctx.render();
        if frame > 30
            && frame < 54
            && data
                .vertices
                .iter()
                .any(|v| v.pos[0] >= 400.0 && v.col[3] > 0)
        {
            exit_frames += 1;
        }
    }
    (content_frames, exit_frames)
}

#[test]
fn repeated_motion_trajectories_allocate_zero_after_warmup() {
    let mut ctx = Context::new(include_bytes!("assets/DejaVuSans.ttf")).unwrap();
    ctx.add_svg_icon(
        "<svg viewBox='0 0 16 16'><circle cx='8' cy='8' r='6'/></svg>",
        [16.0; 2],
    )
    .unwrap();
    ctx.set_motion_settings(MotionSettings {
        enabled: true,
        ..MotionSettings::default()
    });
    for _ in 0..3 {
        trajectory(&mut ctx);
    }
    let before = ALLOCS.load(Ordering::Relaxed);
    let (content, exits) = trajectory(&mut ctx);
    let allocated = ALLOCS.load(Ordering::Relaxed) - before;
    assert!(
        content > 0 && exits > 0,
        "must measure collapse and popup exit geometry"
    );
    assert_eq!(
        allocated, 0,
        "repeated animated trajectories must allocate zero"
    );
    check_nested_layout_and_layer_allocations();
    check_subpixel_text_allocations();
    check_shape_allocations();
}

fn check_shape_allocations() {
    use restricted_gui::shapes::{BorderPlacement, RoundedRect, Shadow};
    let mut ctx = Context::new(include_bytes!("assets/DejaVuSans.ttf")).unwrap();
    let run = |ctx: &mut Context| {
        for frame in 0..8 {
            ctx.new_frame(0.016);
            ctx.ui().window("shapes", ScrollFlags::NONE, |ui| {
                let shape = RoundedRect::new(
                    [100.0 + frame as f32 * 0.25, 100.0, 260.0, 220.0],
                    [24.0, 0.0, 12.0, 8.0],
                )
                .unwrap();
                ui.draw_shadow(
                    shape,
                    Shadow {
                        offset: [2.0, 3.0],
                        spread: 1.0,
                        blur: 8.0,
                        color: [0, 0, 0, 120],
                        inset: false,
                    },
                )
                .unwrap();
                ui.draw_glow(shape, [99, 91, 255, 60], 8.0).unwrap();
                ui.draw_rounded_border(shape, [255; 4], 0.25, BorderPlacement::Outer)
                    .unwrap();
                ui.rounded_clip("clip", shape, |ui| {
                    ui.draw_text_at([101.0, 101.0], "Clipped text", [255; 4]);
                    ui.button("Capture");
                })
                .unwrap();
            });
            ctx.render();
        }
    };
    for _ in 0..2 {
        run(&mut ctx);
    }
    let before = ALLOCS.load(Ordering::Relaxed);
    run(&mut ctx);
    assert_eq!(
        ALLOCS.load(Ordering::Relaxed) - before,
        0,
        "warmed shape and clip trajectories must allocate zero"
    );
}

fn check_subpixel_text_allocations() {
    use restricted_gui::font::{Font, FontId, IconFont, TextRasterMode};
    let font = Font::new_with_icons_at_scale(
        include_bytes!("assets/DejaVuSans.ttf"),
        16.0,
        &[IconFont {
            bytes: include_bytes!("assets/fa-solid-900.ttf"),
            range: (0xf005, 0xf005),
        }],
        1.5,
    )
    .unwrap();
    let mut ctx = Context::with_font(font);
    ctx.set_text_raster_mode(TextRasterMode::Subpixel);
    let big = ctx
        .add_font(include_bytes!("assets/DejaVuSans.ttf"), 24.0)
        .unwrap();
    let run = |ctx: &mut Context| {
        for frame in 0..32 {
            ctx.new_frame(0.016);
            ctx.ui().window("phases", ScrollFlags::NONE, |ui| {
                ui.push_font(if frame < 16 { FontId::DEFAULT } else { big });
                let x = 100.0 + (frame % 4) as f32 / 6.0;
                let y = 90.0 + (frame % 16 / 4) as f32 / 6.0;
                ui.draw_text_at([x, y], "Hgj café \u{f005}", [255; 4]);
                ui.pop_font();
            });
            ctx.render();
        }
    };
    for _ in 0..2 {
        run(&mut ctx);
    }
    let before = ALLOCS.load(Ordering::Relaxed);
    run(&mut ctx);
    assert_eq!(
        ALLOCS.load(Ordering::Relaxed) - before,
        0,
        "warmed phase trajectories must allocate zero"
    );
}

fn check_nested_layout_and_layer_allocations() {
    use restricted_gui::layout::{LayoutItem, LayoutSpec};
    let mut ctx = Context::new(include_bytes!("assets/DejaVuSans.ttf")).unwrap();
    let run = |ctx: &mut Context| {
        for frame in 0..12 {
            ctx.set_input(InputState {
                viewport_size: [800.0, 600.0],
                ..InputState::default()
            });
            ctx.new_frame(0.016);
            ctx.ui().window("layouts", ScrollFlags::NONE, |ui| {
                let items = [
                    LayoutItem {
                        absolute: true,
                        layer: 20,
                        ..LayoutItem::fixed([120.0, 80.0])
                    },
                    LayoutItem {
                        absolute: true,
                        layer: 0,
                        offset: [10.0, 10.0],
                        ..LayoutItem::fixed([120.0, 80.0])
                    },
                ];
                ui.layout(
                    "layers",
                    LayoutSpec {
                        size: [150.0 + frame as f32, 110.0],
                        ..LayoutSpec::default()
                    },
                    &items,
                    |ui, parent| {
                        for i in 0..2 {
                            ui.layout_item(parent, i, |ui, _| {
                                ui.layout(
                                    "auto",
                                    LayoutSpec::default(),
                                    &[LayoutItem::fixed([100.0, 30.0])],
                                    |ui, inner| {
                                        ui.layout_item(inner, 0, |ui, _| {
                                            ui.button("Repeated");
                                        })
                                        .unwrap();
                                    },
                                )
                                .unwrap();
                            })
                            .unwrap();
                        }
                    },
                )
                .unwrap();
            });
            ctx.render();
        }
    };
    for _ in 0..3 {
        run(&mut ctx);
    }
    let before = ALLOCS.load(Ordering::Relaxed);
    run(&mut ctx);
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - before, 0);
}
