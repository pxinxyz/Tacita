use super::*;
use tacita::layout::{LayoutItem, LayoutSpec};

pub const BANNER: TextureId = TextureId(5000);
const BACKGROUND: [u8; 4] = [9, 12, 17, 255];
const PANEL: [u8; 4] = [15, 19, 26, 255];
const LINE: [u8; 4] = [35, 43, 54, 255];
const TEXT: [u8; 4] = [226, 230, 235, 255];
const MUTED: [u8; 4] = [137, 151, 165, 255];
const ACCENT: [u8; 4] = [151, 196, 210, 255];

const PAGES: [(&str, &str, &str); 14] = [
    (
        "Overview",
        "START",
        "Explore the framework through working examples.",
    ),
    (
        "Controls",
        "INTERFACE",
        "Change values and observe each control's state.",
    ),
    (
        "Typography",
        "INTERFACE",
        "Edit Unicode text and compare font rendering.",
    ),
    (
        "Data",
        "INTERFACE",
        "Inspect tables, lists, plots, and editors.",
    ),
    (
        "Layout",
        "INTERFACE",
        "Change constraints and observe the resolved layout.",
    ),
    (
        "Motion",
        "INTERFACE",
        "Compare easing, springs, and keyframes.",
    ),
    (
        "Shapes",
        "DRAWING",
        "Inspect gradients, clipping, and surface effects.",
    ),
    (
        "Paths",
        "DRAWING",
        "Change stroke geometry and curve parameters.",
    ),
    (
        "Primitives",
        "DRAWING",
        "Inspect geometry, images, and antialiasing.",
    ),
    (
        "Input",
        "SYSTEM",
        "Inspect navigation, shortcuts, and saved state.",
    ),
    (
        "Scrolling",
        "SYSTEM",
        "Compare nested scrolling and wheel routing.",
    ),
    (
        "Host services",
        "SYSTEM",
        "Inspect accessibility, cursor, and redraw requests.",
    ),
    (
        "Rendering",
        "SYSTEM",
        "Change renderer settings and inspect GPU output.",
    ),
    (
        "Appearance",
        "SYSTEM",
        "Adjust the theme, fonts, and frame pacing.",
    ),
];

pub struct State {
    pub page: usize,
    query: String,
    animate: bool,
    radius: f32,
    opacity: f32,
    preview_mode: usize,
    clicks: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            page: 0,
            query: String::new(),
            animate: true,
            radius: 52.,
            opacity: 0.8,
            preview_mode: 0,
            clicks: 0,
        }
    }
}

pub struct Frame<'a> {
    pub size: [f32; 2],
    pub faces: &'a [(FontId, FontInfo)],
    pub icons: [tacita::svg::SvgIconId; 9],
    pub atlas: [u32; 2],
    pub atlas_generation: u64,
    pub density: f32,
    pub time: f32,
    pub stats: tacita::rendering::RenderStats,
    pub timing: bool,
    pub interval: f32,
    pub preview_font: Option<FontId>,
}

pub fn style() -> Style {
    Style {
        window_bg: BACKGROUND,
        title_bg: BACKGROUND,
        title_text: TEXT,
        widget_bg: [25, 33, 43, 255],
        widget_bg_hovered: [45, 61, 74, 255],
        widget_bg_active: [94, 139, 155, 255],
        text: TEXT,
        scrollbar_bg: [11, 15, 21, 255],
        scrollbar_thumb: [46, 58, 71, 255],
        scrollbar_thumb_hovered: [72, 92, 108, 255],
        scrollbar_thumb_active: [111, 152, 169, 255],
        frame_padding: [9., 6.],
        item_spacing: [10., 6.],
        window_padding: [18., 18.],
        slider_grab_size: 10.,
        scrollbar_size: 8.,
        corner_rounding: 4.,
        border_size: 1.,
        anti_aliasing: true,
    }
}

pub fn page_index(name: &str) -> Option<usize> {
    PAGES
        .iter()
        .position(|(title, _, _)| title.eq_ignore_ascii_case(name))
}

pub fn prepare(ctx: &mut Context, size: [f32; 2]) {
    let padding = ctx.style.window_padding;
    let title_height = ctx.font().line_height() + ctx.style.frame_padding[1] * 2.;
    ctx.set_window_rect(
        MAIN_WINDOW_TITLE,
        [-padding[0], -title_height - padding[1]],
        [
            size[0] + padding[0] * 2. + 80.,
            size[1] + title_height + padding[1] * 2. + 80.,
        ],
    );
    ctx.set_window_open(MAIN_WINDOW_TITLE, true);
}

fn text(ui: &mut Ui<'_>, value: &str, color: [u8; 4]) {
    let size = ui.measure_text(value);
    let rect = ui.allocate([size[0], size[1]]);
    ui.draw_text_at([rect[0], rect[1]], value, color);
}

fn caption(ui: &mut Ui<'_>, value: &str) {
    ui.with_font(FontId(1), |ui| text(ui, value, MUTED));
}

fn heading(ui: &mut Ui<'_>, value: &str) {
    ui.with_font(FontId(2), |ui| text(ui, value, TEXT));
}

fn space(ui: &mut Ui<'_>, height: f32) {
    ui.allocate([0., height]);
}

fn region(ui: &mut Ui<'_>, name: &str, size: [f32; 2], inset: f32, body: impl FnOnce(&mut Ui<'_>)) {
    let item = LayoutItem {
        absolute: true,
        offset: [inset; 2],
        ..LayoutItem::fixed([
            (size[0] - inset * 2.).max(1.),
            (size[1] - inset * 2.).max(1.),
        ])
    };
    ui.layout(
        name,
        LayoutSpec {
            size,
            ..Default::default()
        },
        &[item],
        |ui, layout| {
            ui.layout_item(layout, 0, |ui, _| body(ui))
                .expect("one region");
        },
    )
    .expect("finite gallery region");
}

fn card(ui: &mut Ui<'_>, name: &str, title: &str, size: [f32; 2], body: impl FnOnce(&mut Ui<'_>)) {
    let mut layout = ui
        .begin_layout(
            name,
            LayoutSpec {
                size,
                ..Default::default()
            },
            &[LayoutItem {
                absolute: true,
                offset: [20., 18.],
                ..LayoutItem::fixed([size[0] - 40., size[1] - 36.])
            }],
        )
        .expect("card geometry");
    let r = layout.rect();
    ui.draw_rect_filled([r[0], r[1]], [r[2], r[3]], PANEL);
    ui.draw_rect_outline([r[0], r[1]], [r[2], r[3]], LINE, 1.);
    ui.layout_item(&mut layout, 0, |ui, _| {
        text(ui, title, TEXT);
        space(ui, 3.);
        body(ui);
    })
    .expect("card contents");
    ui.end_layout(layout);
}

fn columns(
    ui: &mut Ui<'_>,
    name: &str,
    count: usize,
    height: f32,
    mut body: impl FnMut(&mut Ui<'_>, usize, f32),
) {
    let width = ui.available_width();
    let columns = if width >= 700. { count } else { 1 };
    let gap = 18.;
    let cell = (width - gap * (columns - 1) as f32) / columns as f32;
    let rows = count.div_ceil(columns);
    let items: Vec<_> = (0..count)
        .map(|i| LayoutItem {
            absolute: true,
            offset: [
                (i % columns) as f32 * (cell + gap),
                (i / columns) as f32 * (height + gap),
            ],
            ..LayoutItem::fixed([cell, height])
        })
        .collect();
    ui.layout(
        name,
        LayoutSpec {
            size: [width, rows as f32 * (height + gap) - gap],
            ..Default::default()
        },
        &items,
        |ui, layout| {
            for i in 0..count {
                ui.layout_item(layout, i, |ui, _| body(ui, i, cell))
                    .expect("ordered cards");
            }
        },
    )
    .expect("responsive cards");
}

fn nav(ui: &mut Ui<'_>, label: &str, selected: bool) -> bool {
    let width = ui.available_width();
    let rect = ui.allocate([width, 31.]);
    let id = ui.widget_id(label);
    ui.focusable(id, rect);
    ui.accessibility_name(id, label);
    ui.accessibility_info(
        id,
        tacita::accessibility::AccessibilityRole::Tab,
        label,
        if selected {
            tacita::accessibility::AccessibilityState::SELECTED
        } else {
            0
        },
        tacita::accessibility::AccessibilityActions::FOCUS
            | tacita::accessibility::AccessibilityActions::ACTIVATE,
    );
    let hovered = ui.is_hovered(rect);
    if hovered && ui.mouse_pressed(0) {
        ui.capture(id);
    }
    let mut clicked = ui.keyboard_activated(id);
    if ui.is_captured(id) && ui.mouse_released(0) {
        ui.release_capture(id);
        clicked |= ui.is_hovered(rect);
    }
    if selected || hovered || ui.is_focused(id) {
        ui.draw_rect_filled(
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            if selected {
                [28, 39, 49, 255]
            } else {
                [20, 27, 36, 255]
            },
        );
    }
    if selected {
        ui.draw_rect_filled(
            [rect[0], rect[1] + 7.],
            [rect[0] + 2., rect[3] - 7.],
            ACCENT,
        );
    }
    ui.draw_text_at(
        [rect[0] + 12., rect[1] + 6.],
        label,
        if selected { TEXT } else { MUTED },
    );
    clicked
}

pub fn draw(ui: &mut Ui<'_>, state: &mut DemoState, frame: &Frame<'_>) {
    let [w, h] = frame.size;
    let sidebar = if w >= 1100. { 226. } else { 202. };
    ui.draw_rect_filled([0., 0.], [w, h], BACKGROUND);
    ui.draw_rect_filled([0., 0.], [sidebar, h], [12, 16, 22, 255]);
    ui.draw_line([sidebar, 0.], [sidebar, h], LINE, 1.);
    let items = [
        LayoutItem {
            absolute: true,
            ..LayoutItem::fixed([sidebar, h])
        },
        LayoutItem {
            absolute: true,
            offset: [sidebar, 0.],
            ..LayoutItem::fixed([w - sidebar, 105.])
        },
        LayoutItem {
            absolute: true,
            offset: [sidebar + 28., 112.],
            ..LayoutItem::fixed([w - sidebar - 56., h - 155.])
        },
        LayoutItem {
            absolute: true,
            offset: [sidebar, h - 30.],
            ..LayoutItem::fixed([w - sidebar, 30.])
        },
    ];
    ui.layout(
        "application",
        LayoutSpec {
            size: [w, h],
            ..Default::default()
        },
        &items,
        |ui, layout| {
            ui.layout_item(layout, 0, |ui, _| {
                region(ui, "sidebar", [sidebar, h], 18., |ui| {
                    heading(ui, "tacita");
                    caption(ui, "INTERACTIVE GALLERY");
                    space(ui, 4.);
                    ui.search_box("Find a page##navigation", &mut state.gallery.query);
                    let query = state.gallery.query.to_lowercase();
                    let mut group = "";
                    let mut matches = 0;
                    ui.child(
                        "navigation",
                        [sidebar - 36., h - 205.],
                        ScrollFlags::BAR,
                        |ui| {
                            for (index, &(name, section, _)) in PAGES.iter().enumerate() {
                                if !name.to_lowercase().contains(&query) {
                                    continue;
                                }
                                if section != group {
                                    if !group.is_empty() {
                                        space(ui, 2.);
                                    }
                                    caption(ui, section);
                                    group = section;
                                }
                                if nav(ui, name, state.gallery.page == index) {
                                    state.gallery.page = index;
                                }
                                matches += 1;
                            }
                            if matches == 0 {
                                caption(ui, "No matching pages");
                            }
                        },
                    );
                    caption(ui, "RUST 2021");
                    caption(ui, "Drawn with Tacita");
                });
            })
            .expect("sidebar item");
            ui.layout_item(layout, 1, |ui, _| {
                let width = w - sidebar;
                let items = [
                    LayoutItem {
                        absolute: true,
                        offset: [28., 22.],
                        ..LayoutItem::fixed([width - 220., 74.])
                    },
                    LayoutItem {
                        absolute: true,
                        offset: [width - 166., 31.],
                        ..LayoutItem::fixed([140., 52.])
                    },
                ];
                ui.layout(
                    "page heading",
                    LayoutSpec {
                        size: [width, 105.],
                        ..Default::default()
                    },
                    &items,
                    |ui, layout| {
                        ui.layout_item(layout, 0, |ui, _| {
                            heading(ui, PAGES[state.gallery.page].0);
                            caption(ui, PAGES[state.gallery.page].2);
                        })
                        .expect("page title");
                        ui.layout_item(layout, 1, |ui, _| {
                            if ui.button("Reset examples") {
                                state.reset();
                                state.gallery.animate = true;
                                state.gallery.radius = 52.;
                                state.gallery.opacity = 0.8;
                                state.gallery.preview_mode = 0;
                                state.gallery.clicks = 0;
                            }
                            ui.tooltip_text(
                                "Resets control values. Text and appearance stay unchanged.",
                            );
                        })
                        .expect("page reset");
                    },
                )
                .expect("page header");
                ui.draw_line([sidebar + 28., 104.], [w - 28., 104.], LINE, 1.);
            })
            .expect("header item");
            ui.layout_item(layout, 2, |ui, _| {
                state.host_services.idle &= state.gallery.page == 11;
                ui.child(
                    &format!("page {}", state.gallery.page),
                    [w - sidebar - 56., h - 155.],
                    ScrollFlags::BAR,
                    |ui| match state.gallery.page {
                        0 => overview(ui, state, frame),
                        1 => controls(ui, state, frame.time),
                        2 => {
                            heading(ui, "Unicode editor");
                            state.text_editing.draw(ui);
                            ui.collapsing_header("Text layout and coverage", |ui| {
                                text_showcase(ui, state, frame.faces, frame.time, frame.density)
                            });
                            ui.collapsing_header("Font atlas and registration", |ui| {
                                if let Some(face) = frame.preview_font {
                                    ui.with_font(face, |ui| ui.label(FONT_SAMPLE_TEXT));
                                }
                                fonts_panel(
                                    ui,
                                    state,
                                    frame.faces,
                                    frame.atlas,
                                    frame.atlas_generation,
                                );
                            });
                            layout_showcase::icons(ui, &frame.icons, ACCENT);
                        }
                        3 => state.data.draw(ui),
                        4 => {
                            path_layout_showcase::layouts(ui, &mut state.path_layout, state.style);
                            ui.collapsing_header("Layout constraints", |ui| {
                                layout_showcase::show(ui, state, frame.faces)
                            });
                        }
                        5 => motion_showcase(ui, state, frame.time),
                        6 => shapes_showcase::show(
                            ui,
                            &mut state.shape_preview,
                            TEST_PATTERN_ID,
                            frame.density,
                            state.style,
                            ALPHA_MASK_ID,
                            frame.time,
                            state.motion.enabled && !state.motion.reduced_motion,
                        ),
                        7 => path_layout_showcase::paths(ui, &mut state.path_layout, state.style),
                        8 => canvas_showcase(ui, state, frame.time, state.style.window_padding[0]),
                        9 => state.input_state.draw(ui),
                        10 => scroll_routing_window(ui, state),
                        11 => state.host_services.draw(ui),
                        12 => state.render_preview.draw(ui, frame.stats, frame.timing),
                        13 => appearance(ui, state),
                        _ => unreachable!("valid gallery page"),
                    },
                );
            })
            .expect("content item");
            ui.layout_item(layout, 3, |ui, _| {
                ui.draw_line([sidebar, h - 31.], [w, h - 31.], LINE, 1.);
                region(ui, "status", [w - sidebar, 30.], 7., |ui| {
                    caption(
                        ui,
                        &format!(
                            "wgpu     {}     {} draw calls",
                            state.fps_text, frame.stats.draw_calls
                        ),
                    );
                });
            })
            .expect("status item");
        },
    )
    .expect("application layout");
}

fn overview(ui: &mut Ui<'_>, state: &mut DemoState, frame: &Frame<'_>) {
    let width = ui.available_width();
    let height = (width / 3.).min(205.);
    let r = ui.allocate([width, height]);
    let visible = (height / (width / 3.)).min(1.);
    let margin = (1. - visible) * 0.5;
    ui.draw_image(
        BANNER,
        [r[0], r[1]],
        [r[2], r[3]],
        [0., margin],
        [1., 1. - margin],
        [255; 4],
    );
    space(ui, 5.);
    columns(ui, "frame measurements", 3, 108., |ui, i, width| {
        card(
            ui,
            &format!("measurement {i}"),
            ["FRAME INTERVAL", "DRAW CALLS", "GPU TIME"][i],
            [width, 108.],
            |ui| {
                let value = match i {
                    0 => format!("{:.2} ms", frame.interval),
                    1 => frame.stats.draw_calls.to_string(),
                    _ => frame
                        .stats
                        .gpu_ms
                        .map_or_else(|| "Unavailable".into(), |v| format!("{v:.3} ms")),
                };
                heading(ui, &value);
            },
        );
    });
    space(ui, 3.);
    columns(ui, "interactive preview", 2, 290., |ui, index, width| {
        if index == 0 {
            card(
                ui,
                "preview controls",
                "Live controls",
                [width, 290.],
                |ui| {
                    ui.pill_toggle("Animate", &mut state.gallery.animate);
                    ui.segmented_control(
                        "Geometry",
                        &["Ring", "Disc", "Both"],
                        &mut state.gallery.preview_mode,
                    );
                    ui.filled_slider_float("Radius", &mut state.gallery.radius, 12.0..=72.0);
                    ui.filled_slider_float("Opacity", &mut state.gallery.opacity, 0.1..=1.0);
                    if ui.button("Show notification") {
                        ui.toast("Preview updated.", 2.5);
                    }
                },
            );
        } else {
            card(ui, "live preview", "Canvas preview", [width, 290.], |ui| {
                let canvas_width = ui.available_width();
                ui.preview_canvas("geometry", [canvas_width, 180.], |ui, r| {
                    let center = [(r[0] + r[2]) * 0.5, (r[1] + r[3]) * 0.5];
                    let radius = state.gallery.radius;
                    let phase = if state.gallery.animate
                        && state.motion.enabled
                        && !state.motion.reduced_motion
                    {
                        frame.time * 0.8
                    } else {
                        0.
                    };
                    for x in (0..canvas_width as usize).step_by(24) {
                        ui.draw_line(
                            [r[0] + x as f32, r[1]],
                            [r[0] + x as f32, r[3]],
                            [26, 34, 44, 255],
                            1.,
                        );
                    }
                    for y in (0..180).step_by(24) {
                        ui.draw_line(
                            [r[0], r[1] + y as f32],
                            [r[2], r[1] + y as f32],
                            [26, 34, 44, 255],
                            1.,
                        );
                    }
                    let mut color = ACCENT;
                    color[3] = (state.gallery.opacity * 255.) as u8;
                    if state.gallery.preview_mode != 0 {
                        ui.draw_circle_filled(center, radius * 0.7, [45, 77, 92, color[3]]);
                    }
                    if state.gallery.preview_mode != 1 {
                        ui.draw_circle_outline(center, radius, [43, 59, 73, 255], 2.);
                        ui.draw_arc(center, radius, phase..=phase + 4.5, color, 3.);
                    }
                    ui.draw_circle_filled(
                        [
                            center[0] + phase.cos() * radius,
                            center[1] + phase.sin() * radius,
                        ],
                        4.,
                        color,
                    );
                });
                caption(ui, "Generated by the core drawing API");
            });
        }
    });
    space(ui, 3.);
    if ui.button("Explore controls") {
        state.gallery.page = 1;
    }
}

fn controls(ui: &mut Ui<'_>, state: &mut DemoState, time: f32) {
    columns(ui, "choices and values", 2, 355., |ui, index, width| {
        if index == 0 {
            card(ui, "choices", "Selection", [width, 355.], |ui| {
                ui.checkbox("Enable selection", &mut state.enabled);
                let disabled = ui.begin_disabled("selection controls", !state.enabled);
                ui.pill_toggle("Notifications", &mut state.pill_enabled);
                ui.segmented_control(
                    "Mode",
                    &["Quiet", "Balanced", "Bright"],
                    &mut state.segmented_choice,
                );
                ui.radio_button("Local", &mut state.radio_choice, 0);
                ui.same_line();
                ui.radio_button("Shared", &mut state.radio_choice, 1);
                ui.combo(
                    "Preset",
                    &["Default", "Compact", "Large"],
                    &mut state.long_list,
                );
                ui.keybind("Shortcut", &mut state.binding);
                ui.tooltip_text("Select a control, then press a key. Escape cancels.");
                toggle_square(ui, "Custom toggle", &mut state.toggle);
                ui.end_disabled(disabled);
            });
        } else {
            card(ui, "values", "Values", [width, 355.], |ui| {
                ui.slider_float("Speed", &mut state.speed, 0.0..=100.0);
                ui.filled_slider_int("Count", &mut state.count, 0..=50);
                ui.range_slider_float(
                    "Range",
                    &mut state.range_float_lower,
                    &mut state.range_float_upper,
                    0.0..=100.0,
                );
                ui.range_slider_int(
                    "Limits",
                    &mut state.range_int_lower,
                    &mut state.range_int_upper,
                    0..=50,
                );
                ui.drag_float("Drag speed", &mut state.speed, 0.2, 0.0..=100.0);
                ui.drag_int("Drag count", &mut state.count, 0.15, 0..=50);
                ui.progress_bar(
                    "Progress",
                    state.speed / 100.,
                    [(width - 42.).max(100.), 16.],
                );
            });
        }
    });
    space(ui, 3.);
    columns(ui, "text and actions", 2, 355., |ui, index, width| {
        if index == 0 {
            card(ui, "text inputs", "Text and color", [width, 355.], |ui| {
                ui.text_input("Name", &mut state.text);
                ui.search_box("Filter", &mut state.standalone_query);
                ui.color_picker_rgba("Accent", &mut state.circle_color);
                ui.searchable_combo(
                    "Search options",
                    &LONG_LIST,
                    &mut state.search_list,
                    &mut state.search_query,
                );
                ui.list_box(
                    "Items",
                    &LONG_LIST,
                    &mut state.list_box,
                    [width - 42., 105.],
                );
            });
        } else {
            card(ui, "actions", "Actions and feedback", [width, 355.], |ui| {
                if ui.button("Apply changes") {
                    state.gallery.clicks += 1;
                    ui.toast("Changes applied.", 2.5);
                }
                caption(ui, &format!("{} applied changes", state.gallery.clicks));
                ui.status_badge("Ready", [117, 182, 154, 255]);
                ui.same_line();
                ui.hotkey_hint("Enter");
                ui.spinner(
                    "Activity",
                    20.,
                    if state.motion.enabled && !state.motion.reduced_motion {
                        time * 2.
                    } else {
                        0.
                    },
                );
                ui.tree_node("Nested controls", |ui| {
                    ui.checkbox("Nested option", &mut state.nested_enabled);
                });
                ui.status_badge("Values are valid", [117, 182, 154, 255]);
                ui.collapsing_header("Pointer interactions", |ui| interaction_showcase(ui, state));
            });
        }
    });
}

fn appearance(ui: &mut Ui<'_>, state: &mut DemoState) {
    columns(ui, "application appearance", 2, 360., |ui, index, width| {
        if index == 0 {
            card(ui, "theme", "Theme", [width, 360.], |ui| {
                ui.combo("Palette", &THEME_NAMES, &mut state.theme);
                ui.color_edit_rgba("Text", &mut state.style.text);
                ui.color_edit_rgba("Control background", &mut state.style.widget_bg);
                ui.color_edit_rgba("Active control", &mut state.style.widget_bg_active);
                ui.checkbox("Shape antialiasing", &mut state.anti_aliasing);
                ui.checkbox("Text coverage", &mut state.text_subpixel);
                ui.checkbox("Inspect geometry", &mut state.inspect);
            });
        } else {
            card(ui, "preferences", "Interaction", [width, 360.], |ui| {
                ui.checkbox("Limit frame rate", &mut state.limit_fps);
                ui.slider_int("Frame rate", &mut state.fps_limit, 15..=240);
                ui.checkbox("Enable motion", &mut state.motion.enabled);
                ui.checkbox("Reduce motion", &mut state.motion.reduced_motion);
                ui.checkbox("System cursor", &mut state.host_services.native_cursor);
                ui.checkbox("Custom scrollbar", &mut state.custom_scrollbar);
                ui.checkbox("Large hit targets", &mut state.host_services.large_targets);
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tacita::accessibility::{
        AccessibilityAction, AccessibilityActionKind, AccessibilityNode, AccessibilityRole,
    };

    fn app() -> App {
        let mut ctx = Context::new_with_icons(
            DEJAVU_SANS,
            &[IconFont {
                bytes: FA_SOLID,
                range: ICON_RANGE,
            }],
        )
        .unwrap();
        let fonts = [12., 24., 32.].map(|size| ctx.add_font(DEJAVU_SANS, size).unwrap());
        App::new(ctx, fonts)
    }

    fn frame(app: &mut App, size: [f32; 2], mouse: [f32; 2], pressed: bool) {
        prepare(&mut app.ctx, size);
        app.ctx.set_input(InputState {
            viewport_size: size,
            mouse_pos: mouse,
            mouse_down: [pressed, false, false, false, false],
            ..Default::default()
        });
        app.ctx.new_frame(1. / 60.);
        let faces: Vec<_> = (0..app.ctx.font().face_count() as u32)
            .map(|i| {
                let id = FontId(i);
                (id, app.ctx.font().face_info(id).unwrap())
            })
            .collect();
        let (_, width, height) = app.ctx.font().atlas_pixels();
        let info = Frame {
            size,
            faces: &faces,
            icons: app.svg_icons,
            atlas: [width, height],
            atlas_generation: app.ctx.font().atlas_generation(),
            density: 1.,
            time: 1.,
            stats: Default::default(),
            timing: false,
            interval: 16.,
            preview_font: None,
        };
        app.ctx
            .ui()
            .window(MAIN_WINDOW_TITLE, ScrollFlags::NONE, |ui| {
                draw(ui, &mut app.state, &info)
            });
        let data = app.ctx.render();
        assert!(!data.vertices.is_empty());
        assert!(data
            .vertices
            .iter()
            .all(|v| v.pos.iter().all(|n| n.is_finite())));
    }

    fn node(app: &App, name: &str, role: AccessibilityRole) -> Option<AccessibilityNode> {
        (0..app.ctx.accessibility_count())
            .filter_map(|i| app.ctx.accessibility_node(i))
            .find(|n| {
                let mut value = String::new();
                app.ctx.accessibility_name(n.id, &mut value);
                n.role == role && value == name
            })
    }

    #[test]
    fn navigation_accepts_pointer_and_accessibility_activation() {
        let mut app = app();
        let size = [1360., 860.];
        frame(&mut app, size, [-1.; 2], false);
        let controls = node(&app, "Controls", AccessibilityRole::Tab).unwrap().rect;
        let point = [
            (controls[0] + controls[2]) * 0.5,
            (controls[1] + controls[3]) * 0.5,
        ];
        frame(&mut app, size, point, true);
        frame(&mut app, size, point, false);
        assert_eq!(app.state.gallery.page, 1);
        let typography = node(&app, "Typography", AccessibilityRole::Tab).unwrap();
        assert!(app.ctx.submit_accessibility_action(AccessibilityAction {
            id: typography.id,
            kind: AccessibilityActionKind::Activate,
            number: 0.
        }));
        frame(&mut app, size, [-1.; 2], false);
        assert_eq!(app.state.gallery.page, 2);
    }

    #[test]
    fn navigation_search_filters_pages_without_changing_the_selection() {
        let mut app = app();
        app.state.gallery.query = "sHaPeS".into();
        frame(&mut app, [1360., 860.], [-1.; 2], false);
        assert!(node(&app, "Shapes", AccessibilityRole::Tab).is_some());
        assert!(node(&app, "Controls", AccessibilityRole::Tab).is_none());
        assert_eq!(app.state.gallery.page, 0);
    }

    #[test]
    fn all_pages_render_after_window_resizing() {
        let mut app = app();
        for size in [[1360., 860.], [900., 620.], [1100., 720.]] {
            for page in 0..PAGES.len() {
                app.state.gallery.page = page;
                frame(&mut app, size, [-1.; 2], false);
                let reset = node(&app, "Reset examples", AccessibilityRole::Button)
                    .unwrap()
                    .rect;
                assert!(
                    reset[0] >= 0. && reset[2] <= size[0] && reset[1] >= 0. && reset[3] <= size[1]
                );
            }
        }
    }
}
