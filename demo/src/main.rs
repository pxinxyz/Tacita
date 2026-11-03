#[path = "../../host/src/accessibility/mod.rs"]
pub mod accessibility_host;
mod capture;
mod chrome;
#[path = "../../host/src/clipboard.rs"]
mod clipboard;
mod cursor;
mod data_showcase;
mod effects_renderer;
mod frame_watchdog;
mod gallery;
mod gpu_timing;
mod host_services_showcase;
mod input_state_showcase;
mod inspector;
mod layout_showcase;
mod path_layout_showcase;
mod postprocess;
mod render_showcase;
mod renderer;
mod renderer_smoke;
mod rendering_showcase;
#[path = "../../host/src/shader_watch.rs"]
pub mod shader_watch;
mod shapes_showcase;
#[path = "../../host/src/style_watch.rs"]
pub mod style_watch;
mod surface_showcase;
mod text_editing_showcase;

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrome::ChromeMesh;
use cursor::OrbitalCursor;
use inspector::InspectionSnapshot;
use renderer::Renderer;

use tacita::animation::{pulse, Easing, MotionSettings};
use tacita::animation::{GradientOptions, GradientStop, Keyframe, PlaybackMode, TimelineOptions};
use tacita::context::Context;
use tacita::draw::TextureId;
use tacita::font::{FontId, FontInfo, IconFont};
use tacita::icon;
use tacita::input::{InputState, Key, KeyBinding, KeyEvent};
use tacita::panels::{DockState, DockZone, SplitAxis};
use tacita::style::Style;
use tacita::text::{TextAlign, TextOptions, TextOverflow, TextSpan};
use tacita::theme::{ThemeRole, ThemeTokens};
use tacita::ui::{Ui, ValidationState};
use tacita::{ScrollFlags, ScrollbarSkin, ScrollbarTrackClick};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorIcon, Window, WindowId};

const DEJAVU_SANS: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
const FA_SOLID: &[u8] = include_bytes!("../../core/tests/assets/fa-solid-900.ttf");
const NOTO_EMOJI: &[u8] = include_bytes!("../../core/tests/assets/NotoEmoji.ttf");
const NOTO_COLOR_EMOJI: &[u8] = include_bytes!("../../core/tests/assets/Noto-COLRv1.ttf");
const COLOR_FONT_TEXTURE: TextureId = TextureId(90);

const ICON_RANGE: (u32, u32) = (icon::STAR as u32, icon::TRIANGLE_EXCLAMATION as u32);

struct DemoState {
    gallery: gallery::State,
    host_services: host_services_showcase::State,
    input_state: input_state_showcase::State,
    enabled: bool,
    speed: f32,
    count: i32,
    range_float_lower: f32,
    range_float_upper: f32,
    range_int_lower: i32,
    range_int_upper: i32,
    radio_choice: usize,
    pill_enabled: bool,
    segmented_choice: usize,
    nested_enabled: bool,
    text: String,
    fps_text: String,
    toggle: bool,
    anti_aliasing: bool,
    theme: usize,
    circle_color: [u8; 4],
    binding: KeyBinding,
    long_list: usize,
    list_box: usize,
    search_list: usize,
    search_query: String,
    standalone_query: String,
    nav_choice: usize,
    breadcrumb_choice: usize,
    split_ratio: f32,
    child_size: [f32; 2],
    dock_left: DockState,
    dock_right: DockState,
    ui_font: usize,
    font_path: String,
    font_px: i32,
    pending_font_load: bool,
    font_status: String,
    style: Style,
    limit_fps: bool,
    fps_limit: i32,
    routing_child_scrollable: bool,
    inspect: bool,
    drag_payload_value: u32,
    last_drop_value: Option<u32>,
    custom_scrollbar: bool,
    motion: MotionSettings,
    motion_target: bool,
    motion_curve: usize,
    motion_duration: f32,
    spring_response: f32,
    timeline_playing: bool,
    timeline_mode: usize,
    timeline_speed: f32,
    timeline_revision: u64,
    layout_expanded: bool,
    layout_clicks: u32,
    semantic_colors: [[u8; 4]; 5],
    text_width: f32,
    text_wrap: bool,
    text_overflow: usize,
    text_horizontal: usize,
    text_vertical: usize,
    text_lines: i32,
    text_spacing: f32,
    text_subpixel: bool,
    text_phase: f32,
    text_phase_animated: bool,
    general_layout: layout_showcase::State,
    path_layout: path_layout_showcase::State,
    data: data_showcase::State,
    text_editing: text_editing_showcase::State,
    shape_preview: shapes_showcase::State,
    render_preview: render_showcase::State,
}

impl DemoState {
    fn new() -> Self {
        let theme = ThemeTokens::from_style(tacita_style());
        let mut host_services = host_services_showcase::State::default();
        host_services.native_cursor = true;
        DemoState {
            gallery: gallery::State::default(),
            host_services,
            input_state: input_state_showcase::State::default(),
            enabled: true,
            speed: 25.0,
            count: 10,
            range_float_lower: 20.0,
            range_float_upper: 70.0,
            range_int_lower: 10,
            range_int_upper: 40,
            radio_choice: 0,
            pill_enabled: false,
            segmented_choice: 0,
            nested_enabled: false,
            text: String::from("Edit this text."),
            fps_text: String::new(),
            toggle: false,
            anti_aliasing: true,
            theme: 0,
            circle_color: [125, 207, 255, 255],
            binding: KeyBinding::None,
            long_list: 0,
            list_box: 0,
            search_list: 0,
            search_query: String::new(),
            standalone_query: String::new(),
            nav_choice: 0,
            breadcrumb_choice: 2,
            split_ratio: 0.5,
            child_size: [260.0, 110.0],
            dock_left: DockState::default(),
            dock_right: DockState {
                zone: DockZone::Right,
                float_pos: [40.0, 30.0],
            },
            ui_font: 1,
            font_path: String::new(),
            font_px: 16,
            pending_font_load: false,
            font_status: String::new(),
            style: Style::default(),
            limit_fps: true,
            fps_limit: 60,
            routing_child_scrollable: false,
            inspect: false,
            drag_payload_value: 1,
            last_drop_value: None,
            custom_scrollbar: false,
            motion: MotionSettings {
                enabled: true,
                ..MotionSettings::default()
            },
            motion_target: false,
            motion_curve: 6,
            motion_duration: 0.4,
            spring_response: 0.3,
            timeline_playing: true,
            timeline_mode: 2,
            timeline_speed: 1.0,
            timeline_revision: 0,
            layout_expanded: false,
            layout_clicks: 0,
            semantic_colors: core::array::from_fn(|i| theme.colors[11 + i]),
            text_width: 420.0,
            text_wrap: true,
            text_overflow: 2,
            text_horizontal: 0,
            text_vertical: 0,
            text_lines: 0,
            text_spacing: 2.0,
            text_subpixel: true,
            text_phase: 0.0,
            text_phase_animated: true,
            general_layout: layout_showcase::State::default(),
            path_layout: path_layout_showcase::State::default(),
            data: data_showcase::State::default(),
            text_editing: text_editing_showcase::State::default(),
            shape_preview: shapes_showcase::State::default(),
            render_preview: render_showcase::State::default(),
        }
    }

    fn reset(&mut self) {
        self.enabled = true;
        self.speed = 25.0;
        self.count = 10;
        self.range_float_lower = 20.0;
        self.range_float_upper = 70.0;
        self.range_int_lower = 10;
        self.range_int_upper = 40;
        self.radio_choice = 0;
        self.pill_enabled = false;
        self.segmented_choice = 0;
        self.nested_enabled = false;
        self.nav_choice = 0;
        self.breadcrumb_choice = 2;
        self.drag_payload_value = 1;
        self.last_drop_value = None;
        self.custom_scrollbar = false;
        self.motion = MotionSettings {
            enabled: true,
            ..MotionSettings::default()
        };
        self.motion_target = false;
        self.motion_curve = 6;
        self.motion_duration = 0.4;
        self.spring_response = 0.3;
        self.timeline_playing = true;
        self.timeline_mode = 2;
        self.timeline_speed = 1.0;
        self.timeline_revision = self.timeline_revision.wrapping_add(1);
        self.layout_expanded = false;
        self.layout_clicks = 0;
        self.semantic_colors =
            core::array::from_fn(|i| ThemeTokens::from_style(self.style).colors[11 + i]);
        self.text_width = 420.0;
        self.text_wrap = true;
        self.text_overflow = 2;
        self.text_horizontal = 0;
        self.text_vertical = 0;
        self.text_lines = 0;
        self.text_spacing = 2.0;
        self.text_subpixel = true;
        self.text_phase = 0.0;
        self.text_phase_animated = true;
        self.general_layout = layout_showcase::State::default();
        self.path_layout = path_layout_showcase::State::default();
        self.data = data_showcase::State::default();
        self.shape_preview = shapes_showcase::State::default();
        self.split_ratio = 0.5;
        self.child_size = [260.0, 110.0];
        self.dock_left = DockState::default();
        self.dock_right = DockState {
            zone: DockZone::Right,
            float_pos: [40.0, 30.0],
        };
    }
}

const THEME_NAMES: [&str; 4] = ["Tacita", "Graphite", "Nightfall", "High contrast"];

fn tacita_style() -> Style {
    gallery::style()
}
fn nightfall_style() -> Style {
    Style {
        window_bg: [0x0E, 0x10, 0x15, 255],
        title_bg: [0x08, 0x09, 0x0C, 255],
        title_text: [0xE2, 0xE5, 0xED, 255],
        widget_bg: [0x15, 0x18, 0x20, 255],
        widget_bg_hovered: [0x7C, 0x75, 0xFF, 255],
        widget_bg_active: [0x63, 0x5B, 0xFF, 255],
        text: [0xE2, 0xE5, 0xED, 255],
        scrollbar_bg: [0x08, 0x09, 0x0C, 255],
        scrollbar_thumb: [0x21, 0x26, 0x31, 255],
        scrollbar_thumb_hovered: [0x3D, 0x44, 0x57, 255],
        scrollbar_thumb_active: [0x63, 0x5B, 0xFF, 255],
        frame_padding: [6.0, 4.0],
        item_spacing: [8.0, 6.0],
        window_padding: [16.0, 16.0],
        slider_grab_size: 8.0,
        scrollbar_size: 10.0,
        corner_rounding: 2.5,
        border_size: 1.0,
        anti_aliasing: true,
    }
}

const LONG_LIST: [&str; 30] = [
    "Item 01", "Item 02", "Item 03", "Item 04", "Item 05", "Item 06", "Item 07", "Item 08",
    "Item 09", "Item 10", "Item 11", "Item 12", "Item 13", "Item 14", "Item 15", "Item 16",
    "Item 17", "Item 18", "Item 19", "Item 20", "Item 21", "Item 22", "Item 23", "Item 24",
    "Item 25", "Item 26", "Item 27", "Item 28", "Item 29", "Item 30",
];

const UI_FONT_NAMES: [&str; 4] = ["12 px", "16 px (default)", "24 px", "32 px"];

const FONT_SAMPLE_TEXT: &str = "The quick brown fox jumps over the lazy dog 0123456789";

const ATLAS_PREVIEW_WIDTH: f32 = 280.0;

const MAIN_WINDOW_TITLE: &str = "Tacita##gallery";

const ROUTING_CHILD_SIZE: [f32; 2] = [180.0, 136.0];

const ROUTING_CHILD_ROWS: usize = 20;

const ROUTING_FILLER_ROWS: usize = 16;

fn window_manager(ui: &mut Ui<'_>, state: &mut DemoState) {
    if ui.button("Back to overview") {
        state.gallery.page = 0;
    }
}
fn routing_child_content(ui: &mut Ui<'_>, tag: &str) {
    for i in 1..=ROUTING_CHILD_ROWS {
        ui.label(&format!("{tag} {i:02}"));
    }
}

fn scroll_routing_window(ui: &mut Ui<'_>, state: &mut DemoState) {
    ui.label("Scroll laboratory");
    ui.label("Wheel over either list to compare routing.");
    if ui.checkbox(
        "Right child scrollable",
        &mut state.routing_child_scrollable,
    ) {
        eprintln!(
            "[demo] routing child scrollable -> {}",
            state.routing_child_scrollable
        );
    }
    ui.tooltip_text(
        "Flips the RIGHT child's ScrollFlags::scrollable at runtime.\n\
         Off: it registers nothing, so the wheel falls through to the\n\
         window. On: it takes the wheel itself and the window stays put.",
    );
    ui.label("Left: own scrollbar     Right: pass-through / wheel");
    let right_flags = ScrollFlags {
        scrollable: state.routing_child_scrollable,
        scrollbar: false,
    };
    ui.child(
        "bar child##routing",
        ROUTING_CHILD_SIZE,
        ScrollFlags::BAR,
        |ui| {
            routing_child_content(ui, "bar");
        },
    );
    ui.same_line();
    ui.child(
        "flag child##routing",
        ROUTING_CHILD_SIZE,
        right_flags,
        |ui| {
            routing_child_content(ui, "flag");
        },
    );
    ui.label("Scroll outside the lists to move this panel.");
    for i in 1..=ROUTING_FILLER_ROWS {
        ui.label(&format!("window row {i:02}: wheel here scrolls the window"));
    }
    window_manager(ui, state);
}

const TEST_PATTERN_ID: TextureId = TextureId(2);
const ALPHA_MASK_ID: TextureId = TextureId(3);
const TEST_PATTERN_SIZE: u32 = 64;

fn test_pattern_pixels() -> Vec<u8> {
    let mut px = Vec::with_capacity((TEST_PATTERN_SIZE * TEST_PATTERN_SIZE * 4) as usize);
    for y in 0..TEST_PATTERN_SIZE {
        for x in 0..TEST_PATTERN_SIZE {
            let checker = (x / 8 + y / 8) % 2 == 0;
            let (r, g, b): (u8, u8, u8) = if checker { (220, 60, 60) } else { (30, 30, 40) };
            px.extend_from_slice(&[
                r,
                g.saturating_add((x * 2) as u8),
                b.saturating_add((y * 2) as u8),
                255,
            ]);
        }
    }
    px
}

fn banner_pixels() -> (Vec<u8>, u32, u32) {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!("../../banner.png")));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().expect("bundled banner metadata");
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut pixels)
        .expect("bundled banner pixels");
    pixels.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels,
        png::ColorType::Rgb => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        _ => panic!("the bundled banner must use RGB or RGBA"),
    };
    (rgba, info.width, info.height)
}

fn alpha_mask_pixels() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((TEST_PATTERN_SIZE * TEST_PATTERN_SIZE * 4) as usize);
    for y in 0..TEST_PATTERN_SIZE {
        for x in 0..TEST_PATTERN_SIZE {
            let p = [
                ((x as f32 + 0.5) / TEST_PATTERN_SIZE as f32 * 2.0 - 1.0).abs(),
                ((y as f32 + 0.5) / TEST_PATTERN_SIZE as f32 * 2.0 - 1.0).abs(),
            ];
            let alpha =
                ((1.0 - p[0]) / 0.35).clamp(0.0, 1.0) * ((1.0 - p[1]) / 0.35).clamp(0.0, 1.0);
            pixels.extend_from_slice(&[255, 0, 0, (alpha * 255.0 + 0.5) as u8]);
        }
    }
    pixels
}

const TOGGLE_IDLE: [u8; 4] = [60, 60, 80, 255];
const TOGGLE_HOVER: [u8; 4] = [90, 90, 110, 255];
const TOGGLE_ACTIVE: [u8; 4] = [200, 160, 60, 255];
const TOGGLE_MARK: [u8; 4] = [120, 220, 120, 255];
const TOGGLE_TEXT: [u8; 4] = [220, 220, 220, 255];

fn toggle_square(ui: &mut Ui<'_>, label: &str, value: &mut bool) -> bool {
    let id = ui.widget_id(label);
    let rect = ui.allocate([24.0, 24.0]);

    let mouse = ui.mouse_pos();
    let inside =
        mouse[0] >= rect[0] && mouse[0] < rect[2] && mouse[1] >= rect[1] && mouse[1] < rect[3];
    if inside && ui.mouse_pressed(0) {
        ui.capture(id);
    }
    let mut toggled = false;
    if ui.is_captured(id) && ui.mouse_released(0) {
        ui.release_capture(id);
        if inside {
            *value = !*value;
            toggled = true;
        }
    }

    let bg = if ui.is_captured(id) && inside {
        TOGGLE_ACTIVE
    } else if ui.is_hovered(rect) {
        TOGGLE_HOVER
    } else {
        TOGGLE_IDLE
    };
    ui.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], bg);
    if *value {
        ui.draw_rect_filled(
            [rect[0] + 6.0, rect[1] + 6.0],
            [rect[2] - 6.0, rect[3] - 6.0],
            TOGGLE_MARK,
        );
    }
    let text = if *value { "on" } else { "off" };
    let text_size = ui.measure_text(text);
    ui.draw_text_at(
        [
            rect[2] + 6.0,
            rect[1] + (rect[3] - rect[1] - text_size[1]) / 2.0,
        ],
        text,
        TOGGLE_TEXT,
    );
    toggled
}

fn canvas_showcase(ui: &mut Ui<'_>, state: &mut DemoState, time: f32, window_pad_x: f32) {
    ui.checkbox("Anti-aliasing", &mut state.anti_aliasing);
    ui.label("on: circles = coverage masks, others = fringe · off: crisp");
    ui.label("Custom widget");
    toggle_square(ui, "canvas toggle##demo", &mut state.toggle);
    ui.label("Canvas primitives — one row per group:");

    let row_a = ui.allocate([280.0, 58.0]);
    let (ox, oy) = (row_a[0], row_a[1]);
    ui.draw_line(
        [ox + 2.0, oy + 8.0],
        [ox + 56.0, oy + 8.0],
        [240, 240, 240, 255],
        1.0,
    );
    ui.draw_line(
        [ox + 2.0, oy + 16.0],
        [ox + 56.0, oy + 16.0],
        [240, 240, 240, 255],
        4.0,
    );
    ui.draw_line(
        [ox + 2.0, oy + 24.0],
        [ox + 56.0, oy + 52.0],
        [240, 240, 240, 255],
        3.0,
    );
    ui.draw_rect_outline(
        [ox + 64.0, oy + 8.0],
        [ox + 104.0, oy + 52.0],
        [240, 200, 60, 255],
        2.0,
    );
    ui.draw_circle_filled([ox + 134.0, oy + 30.0], 20.0, [80, 160, 240, 255]);
    ui.draw_circle_outline([ox + 184.0, oy + 30.0], 20.0, [120, 220, 120, 255], 3.0);
    ui.draw_circle_outline([ox + 240.0, oy + 30.0], 13.0, [240, 120, 200, 255], 28.0);
    ui.label("t>2r ring: disk via mask, inversion via fringe");

    let row_b = ui.allocate([280.0, 50.0]);
    let (bx, by) = (row_b[0], row_b[1]);
    let (cy, r) = (by + 25.0, 24.0);
    ui.draw_arc(
        [bx + 28.0, cy],
        r,
        std::f32::consts::FRAC_PI_2..=std::f32::consts::PI,
        [240, 150, 60, 255],
        4.0,
    );
    ui.draw_arc([bx + 86.0, cy], r, 0.3..=0.45, [60, 220, 220, 255], 4.0);
    ui.draw_arc(
        [bx + 144.0, cy],
        r,
        0.7..=(0.7 + std::f32::consts::TAU),
        [200, 200, 240, 255],
        4.0,
    );
    let anim_r = 8.0 + 14.0 * (0.5 + 0.5 * (time * 1.5).sin());
    ui.draw_circle_filled([bx + 202.0, cy], anim_r, state.circle_color);
    ui.draw_polygon_filled(
        &[
            [bx + 234.0, cy - 18.0],
            [bx + 252.0, cy - 18.0],
            [bx + 268.0, cy],
            [bx + 252.0, cy + 18.0],
            [bx + 234.0, cy + 18.0],
        ],
        [230, 100, 100, 255],
    );
    ui.label("arcs: quarter / 1-seg / full seam");

    let row_c = ui.allocate([280.0, 64.0]);
    let (cx, cy) = (row_c[0], row_c[1]);
    ui.draw_image(
        TEST_PATTERN_ID,
        [cx + 2.0, cy],
        [cx + 66.0, cy + 64.0],
        [0.0, 0.0],
        [1.0, 1.0],
        [255, 255, 255, 255],
    );
    ui.draw_rect_filled(
        [cx + 86.0, cy + 12.0],
        [cx + 146.0, cy + 52.0],
        [255, 150, 70, 255],
    );
    let window_left = cx - window_pad_x;
    ui.draw_circle_filled([window_left, cy + 32.0], 24.0, [240, 220, 80, 255]);
    ui.label("image / filled rect: inspect the corners");
    ui.label("half outside window = clip test");
}

fn text_showcase(
    ui: &mut Ui<'_>,
    state: &mut DemoState,
    faces: &[(FontId, FontInfo)],
    time: f32,
    density: f32,
) {
    ui.checkbox("Phase-aware glyph coverage", &mut state.text_subpixel);
    ui.collapsing_header("Fractional text positions", |ui| {
        ui.checkbox(
            "Animate physical pixel phase",
            &mut state.text_phase_animated,
        );
        ui.filled_slider_float("Physical pixel offset", &mut state.text_phase, 0.0..=1.0);
        let info = faces
            .iter()
            .find(|(id, _)| *id == ui.active_font())
            .map_or(faces[0].1, |(_, info)| *info);
        let row = info.line_height + 8.0;
        let offset = if state.text_phase_animated {
            (time * 0.2) % 1.0
        } else {
            state.text_phase
        };
        ui.preview_canvas(
            "Text phases##demo",
            [state.text_width, row * 5.0 + 16.0],
            |ui, rect| {
                let x = ((rect[0] + 8.0) * density).floor() / density;
                for (i, phase) in [0.0, 0.25, 0.5, 0.75, offset].iter().enumerate() {
                    let baseline = ((rect[1] + 8.0 + row * i as f32 + info.ascent) * density)
                        .floor()
                        / density;
                    let y = baseline - info.ascent;
                    ui.draw_text_at([x, y], &format!("{phase:.2}"), state.style.text);
                    let sample_x = ((x + 60.0) * density).floor() / density + phase / density;
                    ui.draw_text_at(
                        [sample_x, y + phase / density],
                        "Hamburgefonts 012345 café",
                        state.style.text,
                    );
                }
            },
        );
        ui.label("Offsets affect both axes. Compare coverage on/off, larger fonts and DPI.");
    });
    ui.filled_slider_float("Text width", &mut state.text_width, 80.0..=580.0);
    ui.checkbox("Word wrapping", &mut state.text_wrap);
    ui.combo(
        "Overflow",
        &["Clip", "Truncate", "Ellipsis"],
        &mut state.text_overflow,
    );
    ui.combo(
        "Horizontal",
        &["Start", "Center", "End"],
        &mut state.text_horizontal,
    );
    ui.combo(
        "Vertical",
        &["Start", "Center", "End"],
        &mut state.text_vertical,
    );
    ui.filled_slider_int(
        "Maximum lines (0 = unlimited)",
        &mut state.text_lines,
        0..=6,
    );
    ui.filled_slider_float("Line spacing", &mut state.text_spacing, 0.0..=12.0);
    let aligns = [TextAlign::Start, TextAlign::Center, TextAlign::End];
    let options = TextOptions {
        wrap: state.text_wrap,
        overflow: [
            TextOverflow::Clip,
            TextOverflow::Truncate,
            TextOverflow::Ellipsis,
        ][state.text_overflow.min(2)],
        horizontal: aligns[state.text_horizontal.min(2)],
        vertical: aligns[state.text_vertical.min(2)],
        line_spacing: state.text_spacing,
        max_lines: state.text_lines.max(0) as usize,
    };
    const PARAGRAPH: &str = "Text now flows inside its box. Resize the width to watch words wrap; switch wrapping off to compare clipping, truncation and ellipsis.\nUnicode stays intact: café, λ, naïve.\nAnExtremelyLongUnbrokenWordStillMakesProgressAtNarrowWidths.";
    ui.label("Clipped box / alignment / line limits");
    let color = state.style.text;
    let mut shown = tacita::text::TextMetrics::default();
    ui.preview_canvas("Text box##demo", [state.text_width, 150.0], |ui, rect| {
        ui.draw_rect_filled(
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            state.style.widget_bg,
        );
        shown = ui.draw_text_box(rect, PARAGRAPH, color, options);
    });
    ui.label(&format!(
        "{} lines / {:.1} x {:.1} / hidden: {}",
        shown.lines, shown.size[0], shown.size[1], shown.truncated
    ));
    ui.separator();
    ui.label("Rich text / mixed-font baseline / inline icon");
    let big = faces
        .iter()
        .find(|(_, info)| info.px_size == 32.0)
        .map_or(FontId::DEFAULT, |(id, _)| *id);
    let face = ui.active_font();
    let spans = [
        TextSpan {
            text: "\u{f005} ",
            font: FontId::DEFAULT,
            color: state.style.widget_bg_active,
        },
        TextSpan {
            text: "Large ",
            font: big,
            color: [125, 207, 255, 255],
        },
        TextSpan {
            text: "and regular text share a baseline. Words can span ",
            font: face,
            color,
        },
        TextSpan {
            text: "multiple",
            font: face,
            color: [187, 154, 247, 255],
        },
        TextSpan {
            text: " colored runs without an artificial break.\nSecond line: café λ.",
            font: face,
            color,
        },
    ];
    ui.rich_text(&spans, state.text_width, options);
    ui.button("Ordinary control after text");
    ui.text_block(
        "This paragraph reserves its measured height, so the next widget follows it.",
        state.text_width,
        TextOptions {
            wrap: true,
            ..TextOptions::default()
        },
    );
}

fn motion_showcase(ui: &mut Ui<'_>, state: &mut DemoState, time: f32) {
    ui.collapsing_header("Timeline, layout and gradients", |ui| {
        ui.checkbox("Play timeline", &mut state.timeline_playing);
        ui.same_line();
        if ui.button("Restart timeline") {
            state.timeline_revision = state.timeline_revision.wrapping_add(1);
        }
        if ui.combo(
            "Playback",
            &["Once", "Loop", "Ping-pong"],
            &mut state.timeline_mode,
        ) {
            state.timeline_revision = state.timeline_revision.wrapping_add(1);
        }
        ui.filled_slider_float("Playback speed", &mut state.timeline_speed, -2.0..=2.0);
        let keys = [
            Keyframe {
                time: 0.0,
                value: 0.0,
                easing: Easing::Linear,
            },
            Keyframe {
                time: 0.6,
                value: 1.0,
                easing: Easing::CubicOut,
            },
            Keyframe {
                time: 1.0,
                value: 1.0,
                easing: Easing::Linear,
            },
            Keyframe {
                time: 2.0,
                value: 0.0,
                easing: Easing::SmootherStep,
            },
        ];
        let id = ui.widget_id("##keyframe-track");
        if ui.button("Seek halfway") {
            ui.seek_timeline(id, 1.0);
        }
        let value = ui.animate_timeline(
            id,
            &keys,
            TimelineOptions {
                mode: [
                    PlaybackMode::Once,
                    PlaybackMode::Loop,
                    PlaybackMode::PingPong,
                ][state.timeline_mode.min(2)],
                speed: state.timeline_speed,
                playing: state.timeline_playing,
                revision: state.timeline_revision,
            },
        );
        ui.progress_bar("Keyframe track", value, [320.0, 20.0]);
        ui.pill_toggle("Move and resize panel", &mut state.layout_expanded);
        let stops = [
            GradientStop {
                position: 0.0,
                color: state.style.widget_bg_active,
            },
            GradientStop {
                position: 0.45,
                color: [125, 207, 255, 255],
            },
            GradientStop {
                position: 1.0,
                color: [187, 154, 247, 255],
            },
        ];
        ui.preview_canvas("Animated layout##motion", [380.0, 156.0], |ui, rect| {
            ui.draw_animated_gradient(
                [rect[0], rect[1], rect[2], rect[1] + 22.0],
                &stops,
                GradientOptions::default(),
                time,
                4.0,
            );
            let x = rect[0] + if state.layout_expanded { 135.0 } else { 8.0 };
            let target = [
                x,
                rect[1] + 30.0,
                x + if state.layout_expanded { 235.0 } else { 175.0 },
                rect[3] - 6.0,
            ];
            ui.animated_layout(
                "Live panel",
                target,
                0.65,
                state.motion.easing,
                |ui, bounds| {
                    ui.draw_rect_filled(
                        [bounds[0], bounds[1]],
                        [bounds[2], bounds[3]],
                        state.style.widget_bg,
                    );
                    if ui.button("Click while moving") {
                        state.layout_clicks += 1;
                    }
                    ui.label(&format!("Clicks: {}", state.layout_clicks));
                    ui.label("Pointer follows the panel.");
                },
            );
        });
    });
    ui.collapsing_header("Semantic theme tokens", |ui| {
        for (i, name) in ["Secondary", "Muted", "Success", "Warning", "Danger"]
            .iter()
            .enumerate()
        {
            ui.color_edit_rgba(name, &mut state.semantic_colors[i]);
        }
        ui.validation_message(ValidationState::Success, "Success token");
        ui.validation_message(ValidationState::Warning, "Warning token");
        ui.validation_message(ValidationState::Error, "Danger token");
        let rect = ui.allocate([300.0, 22.0]);
        ui.draw_text_at(
            [rect[0], rect[1]],
            "Secondary text token",
            ui.theme_color(ThemeRole::Secondary),
        );
        ui.draw_text_at(
            [rect[0] + 170.0, rect[1]],
            "Muted token",
            ui.theme_color(ThemeRole::Muted),
        );
    });
    const CURVES: [Easing; 7] = [
        Easing::Linear,
        Easing::SmoothStep,
        Easing::SmootherStep,
        Easing::QuadIn,
        Easing::QuadOut,
        Easing::CubicInOut,
        Easing::CubicOut,
    ];
    const NAMES: [&str; 7] = [
        "Linear",
        "Smoothstep",
        "Smootherstep",
        "Quadratic in",
        "Quadratic out",
        "Cubic in/out",
        "Cubic out",
    ];
    ui.label("Shared motion");
    ui.checkbox("Enable motion", &mut state.motion.enabled);
    ui.same_line();
    ui.checkbox("Reduced motion", &mut state.motion.reduced_motion);
    ui.combo("Easing", &NAMES, &mut state.motion_curve);
    state.motion.easing = CURVES[state.motion_curve.min(CURVES.len() - 1)];
    ui.filled_slider_float("Tween seconds", &mut state.motion_duration, 0.0..=1.2);
    ui.filled_slider_float("Spring response", &mut state.spring_response, 0.0..=1.2);
    ui.pill_toggle("Reverse target", &mut state.motion_target);
    ui.tooltip_text("Reverse before the markers settle. The tween keeps its current position;\nthe spring keeps both its position and velocity.");
    let target = if state.motion_target { 1.0 } else { 0.0 };
    let tween_id = ui.widget_id("##motion-tween");
    let spring_id = ui.widget_id("##motion-spring");
    let color_id = ui.widget_id("##motion-color");
    let tween = ui.animate_float(
        tween_id,
        0.0,
        target,
        state.motion_duration,
        state.motion.easing,
    );
    let spring = ui.animate_spring(spring_id, 0.0, target, state.spring_response);
    let base = state.style.widget_bg_hovered;
    let accent = state.style.widget_bg_active;
    let color = ui.animate_color(
        color_id,
        base,
        if state.motion_target { accent } else { base },
        state.motion_duration,
        state.motion.easing,
    );
    let text = state.style.text;
    let track = state.style.widget_bg;
    let live_opacity = if state.motion.active() {
        0.35 + 0.65 * pulse(time, 1.6)
    } else {
        1.0
    };
    let mut live = [0, 229, 153, 255];
    live[3] = (255.0 * live_opacity) as u8;
    ui.preview_canvas("tween and spring##motion", [320.0, 112.0], |ui, rect| {
        let x = rect[0] + 14.0;
        let travel = (rect[2] - x - 18.0).max(0.0);
        for (row, value) in [tween, spring].iter().enumerate() {
            let y = rect[1] + 22.0 + row as f32 * 32.0;
            ui.draw_rect_filled([x, y - 2.0], [x + travel, y + 2.0], track);
            ui.draw_circle_filled([x + travel * value, y], 7.0, color);
        }
        ui.draw_circle_filled([x, rect[3] - 15.0], 4.0, live);
        ui.draw_text_at(
            [x + 12.0, rect[3] - 24.0],
            "tween / spring / opacity pulse",
            text,
        );
    });
    ui.label("Hover, Tab focus, and switch choices quickly to test reversal.");
    ui.segmented_control(
        "Indicator",
        &["Quiet", "Balanced", "Bright"],
        &mut state.segmented_choice,
    );
    ui.nav_row("Overview##motion-nav", &mut state.nav_choice, 0);
    ui.nav_row("Details##motion-nav", &mut state.nav_choice, 1);
    ui.collapsing_header("Reveal and collapse", |ui| {
        ui.label("Content clips to its changing height; the following rows move with it.");
        ui.button("Hover me##motion");
        ui.tree_node("Nested tree##motion", |ui| {
            ui.checkbox("Nested control##motion", &mut state.nested_enabled);
            ui.label("Reverse the parent or this row before the transition ends.");
        });
    });
    let popup = ui.widget_id("##motion-popup");
    if ui.button("Open popup##motion") {
        ui.open_popup(popup);
    }
    ui.same_line();
    let modal = ui.widget_id("##motion-modal");
    if ui.button("Open modal##motion") {
        ui.open_modal(modal);
    }
    let anchor = ui.allocate([0.0, 0.0]);
    let rect = [
        anchor[0] + 12.0,
        anchor[1] - 160.0,
        anchor[0] + 292.0,
        anchor[1] - 20.0,
    ];
    ui.popup(popup, rect, |ui| {
        ui.label("Fade in / out. Close then reopen to reverse.");
        if ui.button("Close popup##motion") {
            ui.close_popup();
        }
    });
    ui.modal(modal, rect, |ui| {
        ui.label("The backdrop fades with the modal.");
        if ui.button("Close modal##motion") {
            ui.close_popup();
        }
    });
    ui.child(
        "Smooth wheel##motion",
        [320.0, 100.0],
        ScrollFlags::BAR,
        |ui| {
            for row in 0..20 {
                ui.label(&format!("Row {:02}: wheel, then drag the thumb", row + 1));
            }
        },
    );
    ui.collapsing_header("Shared timing settings", |ui| {
        ui.filled_slider_float("Hover seconds", &mut state.motion.hover, 0.0..=1.0);
        ui.filled_slider_float("Focus seconds", &mut state.motion.focus, 0.0..=1.0);
        ui.filled_slider_float("Indicator seconds", &mut state.motion.indicator, 0.0..=1.0);
        ui.filled_slider_float("Popup seconds", &mut state.motion.popup, 0.0..=1.0);
        ui.filled_slider_float("Collapse seconds", &mut state.motion.collapse, 0.0..=1.0);
        ui.filled_slider_float("Scroll response", &mut state.motion.scroll, 0.0..=1.0);
    });
}

fn fonts_panel(
    ui: &mut Ui<'_>,
    state: &mut DemoState,
    faces: &[(FontId, FontInfo)],
    atlas_size: [u32; 2],
    atlas_generation: u64,
) {
    for &(face, info) in faces {
        ui.label(&format!(
            "face {}: {:.0} px · ascent {:.1} · line {:.1} · {} glyphs",
            face.0, info.px_size, info.ascent, info.line_height, info.cached_glyphs
        ));
        ui.with_font(face, |ui| {
            ui.label(FONT_SAMPLE_TEXT);
        });
    }
    if ui.combo("Preview font", &UI_FONT_NAMES, &mut state.ui_font) {
        eprintln!("[demo] ui font -> {}", UI_FONT_NAMES[state.ui_font]);
    }
    ui.tooltip_text("Selects the font for the sample above.");
    ui.text_input("Font file", &mut state.font_path);
    ui.tooltip_text("Path to a .ttf/.otf for the host to register.");
    ui.slider_int("Font px size", &mut state.font_px, 8..=64);
    if ui.button("Load font") {
        state.pending_font_load = true;
    }
    ui.tooltip_text(
        "Defers to the top of the next frame — add_font\nbakes synchronously and may grow the atlas.",
    );
    if !state.font_status.is_empty() {
        ui.label(&state.font_status);
    }

    let total_glyphs: usize = faces.iter().map(|(_, info)| info.cached_glyphs).sum();
    ui.label(&format!(
        "atlas: {}x{} · generation {} · {} faces · {} glyphs cached",
        atlas_size[0],
        atlas_size[1],
        atlas_generation,
        faces.len(),
        total_glyphs
    ));
    ui.tooltip_text(
        "Generation bumps on every bake/cache-on-miss;\nthe host re-uploads before render when it changes.",
    );
    let preview = [
        ATLAS_PREVIEW_WIDTH,
        ATLAS_PREVIEW_WIDTH * atlas_size[1] as f32 / atlas_size[0] as f32,
    ];
    let rect = ui.allocate(preview);
    ui.draw_image(
        TextureId::FONT,
        [rect[0], rect[1]],
        [rect[2], rect[3]],
        [0.0, 0.0],
        [1.0, 1.0],
        [255, 255, 255, 255],
    );
}

struct App {
    capture_output: Option<std::path::PathBuf>,
    watchdog: frame_watchdog::FrameWatchdog,
    renderer_smoke: bool,
    #[cfg(windows)]
    screen_reader: Option<accessibility_host::ScreenReaderBridge>,
    svg_icons: [tacita::svg::SvgIconId; 9],
    ctx: Context,
    state: DemoState,
    themes: [Style; 4],
    applied_theme: usize,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    mouse_pos: [f32; 2],
    mouse_pos_physical: [f64; 2],
    scale_factor: f32,
    cursor_inside: bool,
    native_cursor_visible: Option<bool>,
    native_cursor_icon: Option<CursorIcon>,
    ime_allowed: bool,
    ime_active: bool,
    ime_owner: Option<tacita::id::Id>,
    ime_composing: bool,
    chrome: ChromeMesh,
    cursor: OrbitalCursor,
    inspector_was_enabled: bool,
    inspector_selected: Option<u64>,
    mouse_down: [bool; 5],
    wheel: f32,
    keys: Vec<KeyEvent>,
    viewport_size: [f32; 2],
    runtime_fonts: Vec<(Vec<u8>, f32)>,
    last_frame: Instant,
    next_frame_deadline: Instant,
    paced_limit: Option<i32>,
    display_refresh_millihertz: Option<u32>,
    frame_count: u64,
    fps_measure_start: Instant,
    fps_measure_frames: u32,
    measured_fps: f32,
    measured_ms: f32,
    time: f32,
    last_atlas_generation: u64,
    ui_font_faces: [Option<FontId>; 4],
}

impl App {
    fn new(mut ctx: Context, preset_faces: [FontId; 3]) -> Self {
        let svg_icons =
            layout_showcase::register_icons(|source, size| ctx.add_svg_icon(source, size))
                .expect("valid built-in SVG icons fit the atlas");
        let themes = [
            tacita_style(),
            Style::default(),
            nightfall_style(),
            ThemeTokens::high_contrast().resolve(),
        ];
        ctx.set_accessibility_enabled(true);
        ctx.set_duplicate_id_diagnostics(true);
        let mut state = DemoState::new();
        let args: Vec<_> = std::env::args().collect();
        if let Some(page) = args.windows(2).find(|a| a[0] == "--page") {
            state.gallery.page = gallery::page_index(&page[1]).expect("unknown gallery page");
        }
        let capture_output = args
            .windows(2)
            .find(|a| a[0] == "--capture")
            .map(|a| std::path::PathBuf::from(&a[1]));
        state.style = themes[0];
        ctx.style = state.style;
        let fallback = ctx
            .add_font(FA_SOLID, 16.)
            .expect("fallback icon face loads");
        let emoji = ctx
            .add_font(NOTO_EMOJI, 16.)
            .expect("bundled Noto Emoji loads");
        let color_emoji = ctx
            .add_font(NOTO_COLOR_EMOJI, 16.)
            .expect("bundled Noto color emoji loads");
        ctx.set_color_atlas_texture(Some(COLOR_FONT_TEXTURE))
            .expect("host color atlas ID");
        ctx.set_font_fallback_chain(FontId::DEFAULT, &[color_emoji, emoji])
            .expect("registered emoji fallback");
        for face in preset_faces {
            if face != FontId::DEFAULT {
                ctx.set_font_fallback_chain(face, &[color_emoji, emoji])
                    .expect("registered preset emoji fallback");
            }
        }
        ctx.set_font_fallback_chain(fallback, &[FontId::DEFAULT, color_emoji, emoji])
            .expect("registered fallback chain");
        state.text_editing.fallback_face = fallback;
        ctx.set_text_raster_mode(tacita::font::TextRasterMode::Subpixel);
        warm_up_shapes(&mut ctx, [1350.0, 800.0], fallback);
        let renderer_smoke = std::env::args().any(|arg| arg == "--renderer-smoke");
        App {
            capture_output,
            watchdog: frame_watchdog::FrameWatchdog::new(
                renderer_smoke || std::env::var_os("TACITA_WATCHDOG").is_some(),
            ),
            renderer_smoke,
            #[cfg(windows)]
            screen_reader: None,
            svg_icons,
            themes,
            applied_theme: 0,
            ctx,
            state,
            window: None,
            renderer: None,
            mouse_pos: [0.0; 2],
            mouse_pos_physical: [0.0; 2],
            scale_factor: 1.0,
            cursor_inside: false,
            native_cursor_visible: None,
            native_cursor_icon: None,
            ime_allowed: false,
            ime_active: false,
            ime_owner: None,
            ime_composing: false,
            chrome: ChromeMesh::new(),
            cursor: OrbitalCursor::new(),
            inspector_was_enabled: false,
            inspector_selected: None,
            mouse_down: [false; 5],
            wheel: 0.0,
            keys: Vec::new(),
            viewport_size: [0.0; 2],
            runtime_fonts: Vec::new(),
            last_frame: Instant::now(),
            next_frame_deadline: Instant::now(),
            paced_limit: None,
            display_refresh_millihertz: None,
            frame_count: 0,
            fps_measure_start: Instant::now(),
            fps_measure_frames: 0,
            measured_fps: 0.0,
            measured_ms: 0.0,
            time: 0.0,
            last_atlas_generation: 0,
            ui_font_faces: [
                Some(preset_faces[0]),
                None,
                Some(preset_faces[1]),
                Some(preset_faces[2]),
            ],
        }
    }
    fn apply_theme(&mut self) {
        if self.state.theme != self.applied_theme {
            self.applied_theme = self.state.theme;
            self.state.style = self.themes[self.state.theme];
            self.state.semantic_colors = core::array::from_fn(|i| {
                let t = if self.state.theme == 3 {
                    ThemeTokens::high_contrast()
                } else {
                    ThemeTokens::from_style(self.state.style)
                };
                t.colors[11 + i]
            });
        }
        self.state.style.anti_aliasing = self.state.anti_aliasing;
        let mut tokens = ThemeTokens::from_style(self.state.style);
        tokens.colors[11..].copy_from_slice(&self.state.semantic_colors);
        self.ctx.set_theme_tokens(tokens);
    }

    fn software_cap_enabled(&self) -> bool {
        self.state.limit_fps
    }

    fn frame(&mut self) {
        self.watchdog.progress.frame(self.frame_count + 1);
        self.watchdog.progress.mark(frame_watchdog::Stage::Settings);
        if self.renderer_smoke {
            if let Some(renderer) = &self.renderer {
                if let Some(case) =
                    renderer_smoke::case(self.frame_count, renderer.supported_samples())
                {
                    if self
                        .frame_count
                        .is_multiple_of(renderer_smoke::FRAMES_PER_CASE)
                    {
                        eprintln!(
                            "[smoke] case {}: shape AA {}, {}x, {:?}, {:?}",
                            self.frame_count / renderer_smoke::FRAMES_PER_CASE,
                            case.aa,
                            [1, 2, 4, 8][case.samples],
                            case.alpha,
                            case.optimization
                        );
                    }
                    self.state.anti_aliasing = case.aa;
                    self.state.render_preview.smoke_settings(case);
                }
            }
        }
        if self.ctx.style.anti_aliasing != self.state.anti_aliasing {
            eprintln!("[demo] shape AA -> {}", self.state.anti_aliasing);
        }
        let software_cap = self.software_cap_enabled();
        self.apply_theme();
        self.ctx
            .set_hit_target_policy(tacita::host_requests::HitTargetPolicy {
                minimum: if self.state.host_services.large_targets {
                    [32.; 2]
                } else {
                    [0.; 2]
                },
            });
        #[cfg(windows)]
        if let Some(bridge) = &mut self.screen_reader {
            bridge.drain_actions(&mut self.ctx);
        }
        if let Some(action) = self.state.host_services.pending.take() {
            eprintln!(
                "[demo] accessibility action accepted: {}",
                self.ctx.submit_accessibility_action(action)
            );
        }
        self.state.host_services.collect(&self.ctx);
        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else {
            return;
        };

        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().clamp(
            0.0001,
            if self.state.host_services.idle {
                1.0
            } else {
                0.25
            },
        );
        self.last_frame = now;
        self.fps_measure_frames += 1;
        let measure_seconds = now.duration_since(self.fps_measure_start).as_secs_f32();
        if measure_seconds >= 1.0 {
            self.measured_fps = self.fps_measure_frames as f32 / measure_seconds;
            self.measured_ms = measure_seconds * 1000.0 / self.fps_measure_frames as f32;
            self.fps_measure_start = now;
            self.fps_measure_frames = 0;
        }
        let refresh = self
            .display_refresh_millihertz
            .map_or_else(String::new, |value| {
                format!("    display {:.0} Hz", value as f32 / 1000.)
            });
        self.state.fps_text = format!(
            "{:.0} FPS    {:.2} ms frame interval{}",
            self.measured_fps, self.measured_ms, refresh
        );
        self.time += dt;

        if self.state.pending_font_load {
            self.state.pending_font_load = false;
            let status = match std::fs::read(&self.state.font_path) {
                Ok(bytes) => match self.ctx.add_font(&bytes, self.state.font_px as f32) {
                    Ok(face) => {
                        self.runtime_fonts.push((bytes, self.state.font_px as f32));
                        eprintln!(
                            "[demo] add_font: {:?} at {} px -> face {}",
                            self.state.font_path, self.state.font_px, face.0
                        );
                        format!("loaded face {} ({} px)", face.0, self.state.font_px)
                    }
                    Err(e) => format!("font load failed: {e}"),
                },
                Err(e) => format!("font load failed: {:?}: {e}", self.state.font_path),
            };
            self.state.font_status = status;
        }

        let scrollbar_skin = if self.state.custom_scrollbar {
            ScrollbarSkin {
                track_inset: 2.0,
                thumb_inset: 3.0,
                track_rounding: 3.0,
                thumb_rounding: 3.0,
                min_thumb: 24.0,
                track_click: ScrollbarTrackClick::Page,
            }
        } else {
            ScrollbarSkin::default()
        };
        self.ctx.set_scrollbar_skin(scrollbar_skin);
        self.ctx.set_motion_settings(self.state.motion);
        self.ctx.set_text_raster_mode(if self.state.text_subpixel {
            tacita::font::TextRasterMode::Subpixel
        } else {
            tacita::font::TextRasterMode::Legacy
        });
        let panels = [
            (
                "split",
                tacita::persistence::PanelState::Split {
                    axis: SplitAxis::Horizontal,
                    ratio: self.state.split_ratio,
                },
            ),
            (
                "child",
                tacita::persistence::PanelState::Size(self.state.child_size),
            ),
            (
                "dock_left",
                tacita::persistence::PanelState::Dock(self.state.dock_left),
            ),
            (
                "dock_right",
                tacita::persistence::PanelState::Dock(self.state.dock_right),
            ),
        ];
        let outcome = self.state.input_state.process(&mut self.ctx, &panels);
        if let Some(tokens) = outcome.theme {
            self.state.style = tokens.resolve();
            self.state.anti_aliasing = tokens.anti_aliasing;
            self.state
                .semantic_colors
                .copy_from_slice(&tokens.colors[11..]);
            self.ctx.set_theme_tokens(tokens);
        }
        if let Some(layout) = outcome.layout {
            use tacita::persistence::PanelState;
            if let Some(PanelState::Split { ratio, .. }) = layout.panel("split") {
                self.state.split_ratio = ratio;
            }
            if let Some(PanelState::Size(size)) = layout.panel("child") {
                self.state.child_size = size;
            }
            if let Some(PanelState::Dock(dock)) = layout.panel("dock_left") {
                self.state.dock_left = dock;
            }
            if let Some(PanelState::Dock(dock)) = layout.panel("dock_right") {
                self.state.dock_right = dock;
            }
        }
        self.state.render_preview.prepare_rendering(renderer);
        if !self.state.host_services.idle {
            self.state.render_preview.prepare(
                renderer,
                InputState {
                    mouse_pos: self.mouse_pos,
                    mouse_down: self.mouse_down,
                    mouse_wheel: self.wheel,
                    keys: &self.keys,
                    viewport_size: self.viewport_size,
                },
                dt,
                self.state.style,
                self.scale_factor,
            );
        }
        let render_stats = renderer.stats();
        let timing_supported = renderer.timing_supported();
        self.ctx.set_inspection_enabled(self.state.inspect);
        self.ctx.set_input(InputState {
            mouse_pos: self.mouse_pos,
            mouse_down: self.mouse_down,
            mouse_wheel: self.wheel,
            keys: &self.keys,
            viewport_size: self.viewport_size,
        });
        self.keys.clear();
        self.wheel = 0.0;

        gallery::prepare(&mut self.ctx, self.viewport_size);
        self.ctx.new_frame(dt);
        let window_pad_x = self.ctx.style.window_padding[0];
        let text_density = self.ctx.font().raster_scale();
        let face_infos: Vec<(FontId, FontInfo)> = {
            let font = self.ctx.font();
            (0..font.face_count() as u32)
                .filter_map(|i| {
                    let id = FontId(i);
                    font.face_info(id).map(|info| (id, info))
                })
                .collect()
        };
        let (_, atlas_w, atlas_h) = self.ctx.font().atlas_pixels();
        let atlas_generation = self.ctx.font().atlas_generation();
        let ui_font_face = self.ui_font_faces[self.state.ui_font];
        {
            let mut ui = self.ctx.ui();
            let state = &mut self.state;
            self.watchdog.progress.mark(frame_watchdog::Stage::Widgets);
            ui.window(MAIN_WINDOW_TITLE, ScrollFlags::NONE, |ui| {
                if self.renderer_smoke {
                    canvas_showcase(ui, state, self.time, window_pad_x);
                    state
                        .render_preview
                        .draw(ui, render_stats, timing_supported);
                } else {
                    gallery::draw(
                        ui,
                        state,
                        &gallery::Frame {
                            size: self.viewport_size,
                            faces: &face_infos,
                            icons: self.svg_icons,
                            atlas: [atlas_w, atlas_h],
                            atlas_generation,
                            density: text_density,
                            time: self.time,
                            stats: render_stats,
                            timing: timing_supported,
                            interval: if self.measured_ms > 0. {
                                self.measured_ms
                            } else {
                                dt * 1000.
                            },
                            preview_font: ui_font_face,
                        },
                    );
                }
            });
        }
        let inspection = if self.state.inspect {
            if !self.inspector_was_enabled {
                let x = (self.viewport_size[0] - 452.0).max(8.0);
                self.ctx.set_window_rect(
                    inspector::WINDOW_TITLE,
                    [x, chrome::BAR_HEIGHT + 16.0],
                    [440.0, 480.0],
                );
                self.ctx.set_window_open(inspector::WINDOW_TITLE, true);
            }
            let snapshot = InspectionSnapshot::capture(&self.ctx, inspector::WINDOW_TITLE);
            inspector::show(&mut self.ctx.ui(), &snapshot, &mut self.inspector_selected);
            Some(snapshot)
        } else {
            if self.inspector_was_enabled {
                self.ctx.set_window_open(inspector::WINDOW_TITLE, false);
            }
            None
        };
        self.inspector_was_enabled = self.state.inspect;

        self.watchdog.progress.mark(frame_watchdog::Stage::Atlas);
        let (pixels, w, h) = self.ctx.font().atlas_pixels();
        let generation = self.ctx.font().atlas_generation();
        if generation != self.last_atlas_generation {
            eprintln!("[demo] atlas re-upload: generation {generation} ({w}x{h})");
            self.last_atlas_generation = generation;
        }
        renderer.sync_atlas(pixels, w, h, generation);
        renderer.sync_color_atlas(self.ctx.font());

        let draw = self.ctx.render();
        let (verts, indices, cmds) = (draw.vertices.len(), draw.indices.len(), draw.commands.len());
        self.chrome.clear();
        if let Some(snapshot) = &inspection {
            self.chrome
                .append_inspection_clips(&snapshot.clips, self.viewport_size);
            if let Some((node, _)) = snapshot
                .nodes
                .iter()
                .find(|(node, _)| Some(node.id) == self.inspector_selected)
            {
                self.chrome
                    .append_inspection_selection(node.rect, self.viewport_size);
            }
        }
        if self.cursor_inside
            && !self.state.host_services.native_cursor
            && !self.state.host_services.idle
        {
            self.cursor.rebuild(
                self.mouse_pos,
                dt,
                self.time,
                self.mouse_down[0],
                self.state.style.widget_bg_active,
            );
        } else {
            self.cursor.clear();
        }
        if draw
            .commands
            .iter()
            .any(|c| (20..=22).contains(&c.texture.0))
        {
            let effects = &self.state.shape_preview.surface;
            renderer.prepare_gallery(
                effects.phase,
                effects.blur(),
                effects.threshold,
                effects.intensity,
            );
        }
        if self.frame_count == 90 {
            if let Some(path) = &self.capture_output {
                renderer.capture_next_frame(path.clone());
            }
        }
        renderer.render(&draw, &self.chrome, &self.cursor, self.state.style.title_bg);
        self.watchdog.progress.mark(frame_watchdog::Stage::Idle);
        #[cfg(windows)]
        if let Some(bridge) = &mut self.screen_reader {
            if let Ok(origin) = window.inner_position() {
                bridge.publish(
                    &self.ctx,
                    [origin.x as f32, origin.y as f32],
                    self.scale_factor,
                );
            }
        }
        let caret = self.ctx.text_caret();
        let allowed = caret.is_some();
        if self.ime_allowed != allowed {
            window.set_ime_allowed(allowed);
            self.ime_allowed = allowed;
        }
        if self.ime_active {
            if let Some(c) = caret {
                window.set_ime_cursor_area(
                    winit::dpi::LogicalPosition::new(c.rect[0] as f64, c.rect[1] as f64),
                    winit::dpi::LogicalSize::new(
                        (c.rect[2] - c.rect[0]) as f64,
                        (c.rect[3] - c.rect[1]) as f64,
                    ),
                );
            }
        }
        if let Err(error) = clipboard::process(&mut self.ctx) {
            eprintln!("[demo] clipboard request failed: {error:?}");
        }

        self.frame_count += 1;
        if self.frame_count <= 3 || self.frame_count.is_multiple_of(60) {
            eprintln!(
                "[demo] frame {} rendered: {verts} verts, {indices} indices, {cmds} cmds, dt {:.2} ms",
                self.frame_count,
                dt * 1000.0
            );
        }
        if !software_cap && !self.state.host_services.idle {
            window.request_redraw();
        }
        self.sync_cursor_visibility();
    }

    fn sync_cursor_visibility(&mut self) {
        let Some(window) = &self.window else { return };
        let visible = !self.cursor_inside
            || self.state.host_services.native_cursor
            || self.state.host_services.idle;
        if self.native_cursor_visible != Some(visible) {
            window.set_cursor_visible(visible);
            self.native_cursor_visible = Some(visible);
        }
        let icon = host_services_showcase::native_cursor(self.ctx.cursor_shape());
        if self.native_cursor_icon != Some(icon) {
            window.set_cursor(icon);
            self.native_cursor_icon = Some(icon);
        }
    }

    fn update_display_metrics(&mut self, size: winit::dpi::PhysicalSize<u32>, scale: f32) {
        let requested = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            self.scale_factor
        };
        let rebuilt = self
            .ctx
            .update_display_metrics(tacita::display::DisplayMetrics {
                physical_size: [size.width, size.height],
                scale: requested,
            });
        let changed = match rebuilt {
            Ok(changed) => changed,
            Err(error) => {
                eprintln!("[demo] DPI rebuild failed; retaining previous density: {error}");
                false
            }
        };
        let scale = self.ctx.font().raster_scale();
        let scale_changed = self.scale_factor != scale;
        self.scale_factor = scale;
        self.viewport_size = [size.width as f32 / scale, size.height as f32 / scale];
        self.mouse_pos = [
            self.mouse_pos_physical[0] as f32 / scale,
            self.mouse_pos_physical[1] as f32 / scale,
        ];
        if changed {
            warm_up_shapes(
                &mut self.ctx,
                self.viewport_size,
                self.state.text_editing.fallback_face,
            );
            self.last_atlas_generation = u64::MAX;
            if let Some(renderer) = &mut self.renderer {
                renderer.invalidate_atlas();
            }
            eprintln!("[demo] DPI scale -> {scale:.2}, font atlas rebuilt");
        }
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(size.width, size.height, scale);
        }
        if scale_changed {
            self.cursor.clear();
        }
        self.sync_cursor_visibility();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("Tacita")
                .with_inner_size(winit::dpi::LogicalSize::new(1360.0, 860.0))
                .with_min_inner_size(winit::dpi::LogicalSize::new(900.0, 620.0))
                .with_decorations(true)
                .with_theme(Some(winit::window::Theme::Dark))
                .with_visible(false),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("[demo] failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };
        if let Some(monitor) = window
            .current_monitor()
            .or_else(|| window.primary_monitor())
        {
            let origin = monitor.position();
            let screen = monitor.size();
            let size = window.outer_size();
            window.set_outer_position(winit::dpi::PhysicalPosition::new(
                origin.x + (screen.width as i32 - size.width as i32) / 2,
                origin.y + (screen.height as i32 - size.height as i32) / 2,
            ));
        }
        window.set_visible(true);
        let initial_size = window.inner_size();
        let initial_scale = window.scale_factor() as f32;
        self.display_refresh_millihertz = window
            .current_monitor()
            .and_then(|monitor| monitor.refresh_rate_millihertz());
        eprintln!(
            "[demo] window: {}x{} physical, DPI scale {initial_scale:.2}",
            initial_size.width, initial_size.height
        );
        self.update_display_metrics(initial_size, initial_scale);
        let (pixels, w, h) = self.ctx.font().atlas_pixels();
        let generation = self.ctx.font().atlas_generation();
        match pollster::block_on(Renderer::new(window.clone(), pixels, w, h, generation)) {
            Ok(mut renderer) => {
                eprintln!("[demo] renderer up: {w}x{h} atlas (generation {generation})");
                self.last_atlas_generation = generation;
                renderer.add_host_texture(
                    TEST_PATTERN_ID,
                    &test_pattern_pixels(),
                    TEST_PATTERN_SIZE,
                    TEST_PATTERN_SIZE,
                );
                renderer.add_host_texture(
                    ALPHA_MASK_ID,
                    &alpha_mask_pixels(),
                    TEST_PATTERN_SIZE,
                    TEST_PATTERN_SIZE,
                );
                let (banner, banner_width, banner_height) = banner_pixels();
                renderer.add_host_texture(gallery::BANNER, &banner, banner_width, banner_height);
                renderer.set_frame_progress(self.watchdog.progress.clone());
                self.renderer = Some(renderer);
                self.window = Some(window.clone());
                self.state.host_services.bridge_status = "Off".into();
                #[cfg(windows)]
                if std::env::args().any(|arg| arg == "--native-accessibility") {
                    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
                    if let Ok(handle) = window.window_handle() {
                        if let RawWindowHandle::Win32(handle) = handle.as_raw() {
                            match unsafe {
                                accessibility_host::ScreenReaderBridge::attach(handle.hwnd.get())
                            } {
                                Ok(bridge) => {
                                    self.screen_reader = Some(bridge);
                                    self.state.host_services.bridge_status = "MSAA attached".into();
                                }
                                Err(e) => self.state.host_services.bridge_status = e.into(),
                            }
                        }
                    }
                }
                self.last_frame = Instant::now();
                self.fps_measure_start = self.last_frame;
                self.fps_measure_frames = 0;
                window.request_redraw();
            }
            Err(e) => {
                eprintln!("[demo] renderer init failed: {e}");
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = &self.window else {
            return;
        };
        if self.state.host_services.idle {
            let delay = self.ctx.next_redraw_in().unwrap_or(1.).clamp(0.001, 1.);
            let deadline = self.last_frame + Duration::from_secs_f32(delay);
            if Instant::now() >= deadline {
                window.request_redraw();
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            return;
        }
        if !self.software_cap_enabled() {
            self.paced_limit = None;
            event_loop.set_control_flow(ControlFlow::Poll);
            return;
        }
        let period = Duration::from_secs_f64(1.0 / f64::from(self.state.fps_limit));
        let now = Instant::now();
        if self.paced_limit != Some(self.state.fps_limit) {
            self.paced_limit = Some(self.state.fps_limit);
            self.next_frame_deadline = self.last_frame + period;
        }
        if now >= self.next_frame_deadline {
            window.request_redraw();
            self.next_frame_deadline = if now.duration_since(self.next_frame_deadline) < period {
                self.next_frame_deadline + period
            } else {
                now + period
            };
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame_deadline));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let wake = !matches!(&event, WindowEvent::RedrawRequested);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Moved(_) => {
                self.display_refresh_millihertz = self.window.as_ref().and_then(|window| {
                    window
                        .current_monitor()
                        .and_then(|monitor| monitor.refresh_rate_millihertz())
                });
            }
            WindowEvent::Resized(size) => {
                let scale = self
                    .window
                    .as_ref()
                    .map_or(self.scale_factor, |w| w.scale_factor() as f32);
                self.update_display_metrics(size, scale);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(size) = self.window.as_ref().map(|w| w.inner_size()) {
                    self.update_display_metrics(size, scale_factor as f32);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos_physical = [position.x, position.y];
                self.mouse_pos = [
                    position.x as f32 / self.scale_factor,
                    position.y as f32 / self.scale_factor,
                ];
                self.cursor_inside = true;
                self.sync_cursor_visibility();
            }
            WindowEvent::CursorEntered { .. } => {
                self.cursor_inside = true;
                self.sync_cursor_visibility();
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor_inside = false;
                self.sync_cursor_visibility();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let index = match button {
                    MouseButton::Left => Some(0),
                    MouseButton::Right => Some(1),
                    MouseButton::Middle => Some(2),
                    MouseButton::Back => Some(3),
                    MouseButton::Forward => Some(4),
                    _ => None,
                };
                if let Some(i) = index {
                    self.mouse_down[i] = state == ElementState::Pressed;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.wheel += match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 38.0,
                };
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let key = match code {
                        KeyCode::Backspace => Some(Key::Backspace),
                        KeyCode::Delete => Some(Key::Delete),
                        KeyCode::ArrowLeft => Some(Key::Left),
                        KeyCode::ArrowUp => Some(Key::Up),
                        KeyCode::ArrowRight => Some(Key::Right),
                        KeyCode::ArrowDown => Some(Key::Down),
                        KeyCode::Home => Some(Key::Home),
                        KeyCode::End => Some(Key::End),
                        KeyCode::Enter | KeyCode::NumpadEnter => Some(Key::Enter),
                        KeyCode::Escape => Some(Key::Escape),
                        KeyCode::Insert => Some(Key::Insert),
                        KeyCode::KeyA => Some(Key::A),
                        KeyCode::KeyB => Some(Key::B),
                        KeyCode::KeyC => Some(Key::C),
                        KeyCode::KeyD => Some(Key::D),
                        KeyCode::KeyE => Some(Key::E),
                        KeyCode::KeyF => Some(Key::F),
                        KeyCode::KeyG => Some(Key::G),
                        KeyCode::KeyH => Some(Key::H),
                        KeyCode::KeyI => Some(Key::I),
                        KeyCode::KeyJ => Some(Key::J),
                        KeyCode::KeyK => Some(Key::K),
                        KeyCode::KeyL => Some(Key::L),
                        KeyCode::KeyM => Some(Key::M),
                        KeyCode::KeyN => Some(Key::N),
                        KeyCode::KeyO => Some(Key::O),
                        KeyCode::KeyP => Some(Key::P),
                        KeyCode::KeyQ => Some(Key::Q),
                        KeyCode::KeyR => Some(Key::R),
                        KeyCode::KeyS => Some(Key::S),
                        KeyCode::KeyT => Some(Key::T),
                        KeyCode::KeyU => Some(Key::U),
                        KeyCode::KeyV => Some(Key::V),
                        KeyCode::KeyW => Some(Key::W),
                        KeyCode::KeyX => Some(Key::X),
                        KeyCode::KeyY => Some(Key::Y),
                        KeyCode::KeyZ => Some(Key::Z),
                        KeyCode::Digit0 => Some(Key::Num0),
                        KeyCode::Digit1 => Some(Key::Num1),
                        KeyCode::Digit2 => Some(Key::Num2),
                        KeyCode::Digit3 => Some(Key::Num3),
                        KeyCode::Digit4 => Some(Key::Num4),
                        KeyCode::Digit5 => Some(Key::Num5),
                        KeyCode::Digit6 => Some(Key::Num6),
                        KeyCode::Digit7 => Some(Key::Num7),
                        KeyCode::Digit8 => Some(Key::Num8),
                        KeyCode::Digit9 => Some(Key::Num9),
                        KeyCode::F1 => Some(Key::F1),
                        KeyCode::F2 => Some(Key::F2),
                        KeyCode::F3 => Some(Key::F3),
                        KeyCode::F4 => Some(Key::F4),
                        KeyCode::F5 => Some(Key::F5),
                        KeyCode::F6 => Some(Key::F6),
                        KeyCode::F7 => Some(Key::F7),
                        KeyCode::F8 => Some(Key::F8),
                        KeyCode::F9 => Some(Key::F9),
                        KeyCode::F10 => Some(Key::F10),
                        KeyCode::F11 => Some(Key::F11),
                        KeyCode::F12 => Some(Key::F12),
                        KeyCode::Space => Some(Key::Space),
                        KeyCode::Tab => Some(Key::Tab),
                        KeyCode::ShiftLeft | KeyCode::ShiftRight => Some(Key::Shift),
                        KeyCode::ControlLeft | KeyCode::ControlRight => Some(Key::Ctrl),
                        KeyCode::AltLeft | KeyCode::AltRight => Some(Key::Alt),
                        _ => None,
                    };
                    if let Some(key) = key {
                        self.keys.push(match event.state {
                            ElementState::Pressed => KeyEvent::Press(key),
                            ElementState::Released => KeyEvent::Release(key),
                        });
                    }
                }
                if event.state == ElementState::Pressed && !self.ime_composing {
                    if let Some(text) = &event.text {
                        self.keys.extend(text.chars().map(KeyEvent::Char));
                    }
                }
            }
            WindowEvent::Ime(event) => {
                use tacita::editing::TextEventKind;
                use winit::event::Ime;
                match event {
                    Ime::Enabled => self.ime_active = true,
                    Ime::Preedit(text, cursor) => {
                        if self.ime_owner.is_none() || !self.ime_composing && !text.is_empty() {
                            self.ime_owner = self.ctx.text_caret().map(|c| c.owner);
                        }
                        self.ime_composing = !text.is_empty();
                        if let Some(owner) = self.ime_owner {
                            let (a, b) = cursor.unwrap_or((text.len(), text.len()));
                            if let Err(error) = self.ctx.submit_text_event(
                                owner,
                                if text.is_empty() {
                                    TextEventKind::Cancel
                                } else if cursor.is_none() {
                                    TextEventKind::PreeditHidden
                                } else {
                                    TextEventKind::Preedit
                                },
                                &text,
                                a..b,
                            ) {
                                eprintln!("[demo] preedit rejected: {error:?}");
                            }
                        }
                    }
                    Ime::Commit(text) => {
                        let owner = self
                            .ime_owner
                            .take()
                            .or_else(|| self.ctx.text_caret().map(|c| c.owner));
                        self.ime_composing = false;
                        if let Some(owner) = owner {
                            if let Err(error) = self.ctx.submit_text_event(
                                owner,
                                TextEventKind::Commit,
                                &text,
                                0..0,
                            ) {
                                eprintln!("[demo] IME commit rejected: {error:?}");
                            }
                        }
                    }
                    Ime::Disabled => {
                        self.ime_active = false;
                        self.ime_composing = false;
                        if let Some(owner) = self.ime_owner.take() {
                            let _ =
                                self.ctx
                                    .submit_text_event(owner, TextEventKind::Cancel, "", 0..0);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.frame();
                if self.capture_output.is_some() && self.frame_count > 90 {
                    event_loop.exit();
                }
                if self.renderer_smoke && renderer_smoke::complete(self.frame_count) {
                    eprintln!("[smoke] shape-AA/renderer transitions completed");
                    event_loop.exit();
                }
            }
            _ => {}
        }
        if wake {
            self.ctx.invalidate();
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}

fn interaction_showcase(ui: &mut Ui, state: &mut DemoState) {
    use tacita::drag_drop::DragPayload;

    ui.label("Keyboard: Tab / Shift+Tab moves focus; Enter / Space activates controls.");
    ui.checkbox("Inset page-step scrollbars", &mut state.custom_scrollbar);
    let source = ui.allocate([230.0, 30.0]);
    let source_id = ui.widget_id("drag source##showcase");
    ui.focusable(source_id, source);
    ui.draw_rect_filled(
        [source[0], source[1]],
        [source[2], source[3]],
        state.style.widget_bg,
    );
    ui.draw_text_at(
        [source[0] + 7.0, source[1] + 5.0],
        "Drag this value",
        state.style.text,
    );
    if ui.keyboard_activated(source_id) {
        state.drag_payload_value += 1;
    }
    let payload = DragPayload::new(0x5255_4944, &state.drag_payload_value.to_le_bytes())
        .expect("four-byte demo payload fits");
    let dragging = ui.drag_source(source_id, source, payload);
    let target = ui.allocate([230.0, 30.0]);
    ui.draw_rect_filled(
        [target[0], target[1]],
        [target[2], target[3]],
        if dragging {
            state.style.widget_bg_hovered
        } else {
            state.style.widget_bg
        },
    );
    ui.draw_text_at(
        [target[0] + 7.0, target[1] + 5.0],
        "Drop value here",
        state.style.text,
    );
    if let Some(payload) = ui.drag_target(target, 0x5255_4944) {
        let mut data = [0u8; 4];
        data.copy_from_slice(payload.data());
        state.last_drop_value = Some(u32::from_le_bytes(data));
    }
    ui.label(&format!(
        "Source {} / last drop {:?}",
        state.drag_payload_value, state.last_drop_value
    ));
}

fn warm_up_shapes(ctx: &mut Context, viewport_size: [f32; 2], fallback_face: FontId) {
    let previous_style = ctx.style;
    ctx.style.anti_aliasing = true;
    ctx.set_input(InputState {
        mouse_pos: [0.0; 2],
        mouse_down: [false; 5],
        mouse_wheel: 0.0,
        keys: &[],
        viewport_size,
    });
    ctx.new_frame(1.0 / 60.0);
    {
        let mut ui = ctx.ui();
        ui.window("mask warm-up##prewarm", ScrollFlags::NONE, |ui| {
            let c = [200.0, 200.0];
            let mut r = 8.0;
            while r <= 22.0 {
                ui.draw_circle_filled(c, r, [255, 255, 255, 255]);
                r += 0.25;
            }
            ui.draw_circle_filled(c, 24.0, [255, 255, 255, 255]);
            ui.draw_circle_outline(c, 20.0, [255, 255, 255, 255], 3.0);
            ui.draw_circle_outline(c, 13.0, [255, 255, 255, 255], 28.0);
            let mut selected = 0;
            ui.radio_button("radio mask warm-up##prewarm", &mut selected, 0);
            let mut pill = false;
            ui.pill_toggle("pill mask warm-up##prewarm", &mut pill);
            let mut query = String::new();
            ui.search_box("search mask warm-up##prewarm", &mut query);
            let mut text = text_editing_showcase::State::default();
            text.fallback_face = fallback_face;
            text.draw(ui);
        });
    }
    let _ = ctx.render();
    ctx.style = previous_style;
    let (_, w, h) = ctx.font().atlas_pixels();
    eprintln!(
        "[demo] shape-mask warm-up: atlas {w}x{h}, generation {}",
        ctx.font().atlas_generation()
    );
}

fn main() {
    if std::env::args().any(|arg| arg == "--font-licenses") {
        print!(
            "DejaVu Sans\n{}\nFont Awesome\n{}\nNoto Emoji\n{}\nNoto Color Emoji\n{}\nUnicode emoji presentation data\n{}",
            include_str!("../../core/tests/assets/DejaVuFonts-LICENSE.txt"),
            include_str!("../../core/tests/assets/FontAwesome-LICENSE.txt"),
            include_str!("../../core/tests/assets/NotoEmoji-LICENSE.txt"),
            include_str!("../../core/tests/assets/NotoColorEmoji-LICENSE.txt"),
            include_str!("../../core/src/font/color/Unicode-LICENSE.txt"),
        );
        return;
    }
    let mut ctx = Context::new_with_icons(
        DEJAVU_SANS,
        &[IconFont {
            bytes: FA_SOLID,
            range: ICON_RANGE,
        }],
    )
    .expect("bundled DejaVu Sans + Font Awesome must load");
    let preset_faces = [
        ctx.add_font(DEJAVU_SANS, 12.0)
            .expect("DejaVu Sans must load at 12 px"),
        ctx.add_font(DEJAVU_SANS, 24.0)
            .expect("DejaVu Sans must load at 24 px"),
        ctx.add_font(DEJAVU_SANS, 32.0)
            .expect("DejaVu Sans must load at 32 px"),
    ];

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new(ctx, preset_faces);
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("[demo] event loop error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod dpi_tests {
    use super::*;

    #[test]
    fn startup_with_only_built_in_faces_loads_unicode_showcase() {
        let mut ctx = Context::new_with_icons(
            DEJAVU_SANS,
            &[IconFont {
                bytes: FA_SOLID,
                range: ICON_RANGE,
            }],
        )
        .unwrap();
        let presets = [12.0, 24.0, 32.0].map(|px| ctx.add_font(DEJAVU_SANS, px).unwrap());
        let mut app = App::new(ctx, presets);
        assert_eq!(app.ctx.font().face_count(), 7);
        let emoji = app.ctx.font_fallback_chain(FontId::DEFAULT).unwrap();
        assert_eq!(
            app.ctx
                .font_fallback_chain(app.state.text_editing.fallback_face),
            Some(&[FontId::DEFAULT, emoji[0], emoji[1]][..])
        );
        app.ctx
            .set_text_raster_mode(tacita::font::TextRasterMode::Legacy);
        app.ctx.new_frame(1.0 / 60.0);
        {
            let mut ui = app.ctx.ui();
            ui.window("startup glyph coverage", ScrollFlags::NONE, |ui| {
                for (i, text) in ["?", "é", "ö"].iter().enumerate() {
                    ui.draw_text_at([100.0, 200.0], text, [i as u8 + 1, 23, 45, 255]);
                }
            });
        }
        let draw = app.ctx.render();
        let uv = |red| {
            draw.vertices
                .iter()
                .find(|v| v.col == [red, 23, 45, 255])
                .unwrap()
                .uv
        };
        assert_ne!(
            uv(1),
            uv(2),
            "accented glyph must not reuse question-mark coverage"
        );
        assert_ne!(
            uv(1),
            uv(3),
            "accented glyph must not reuse question-mark coverage"
        );
    }

    #[test]
    fn startup_routes_emoji_clusters_to_bundled_fallback() {
        let ctx = Context::new(DEJAVU_SANS).unwrap();
        let app = App::new(ctx, [FontId::DEFAULT; 3]);
        let chain = app.ctx.font_fallback_chain(FontId::DEFAULT).unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(
            app.ctx
                .font_fallback_chain(app.state.text_editing.fallback_face),
            Some(&[FontId::DEFAULT, chain[0], chain[1]][..])
        );
        let mut font = app.ctx.font().clone();
        let mut layout = tacita::font::ShapedText::default();
        for cluster in ["👩‍💻", "🇬🇧", "👍🏽"] {
            layout
                .prepare(&mut font, FontId::DEFAULT, cluster, Default::default())
                .unwrap();
            assert_eq!(layout.glyphs().len(), 1);
            assert_eq!(layout.glyphs()[0].font, chain[0]);
            assert_ne!(layout.glyphs()[0].index, 0);
        }
    }

    #[test]
    fn display_change_preserves_face_ids_and_converts_input_coordinates() {
        let mut ctx = Context::new_with_icons(
            DEJAVU_SANS,
            &[IconFont {
                bytes: FA_SOLID,
                range: ICON_RANGE,
            }],
        )
        .unwrap();
        let presets = [12.0, 24.0, 32.0].map(|px| ctx.add_font(DEJAVU_SANS, px).unwrap());
        ctx.add_font(DEJAVU_SANS, 18.0).unwrap();
        let mut app = App::new(ctx, presets);
        app.runtime_fonts.push((DEJAVU_SANS.to_vec(), 18.0));
        app.mouse_pos_physical = [150.0, 90.0];
        let face_count = app.ctx.font().face_count();
        let fallback_face = app.state.text_editing.fallback_face;
        let fallback_chain = app.ctx.font_fallback_chain(fallback_face).unwrap().to_vec();
        let emoji_chain = app
            .ctx
            .font_fallback_chain(FontId::DEFAULT)
            .unwrap()
            .to_vec();

        app.update_display_metrics(winit::dpi::PhysicalSize::new(1500, 900), 1.5);
        assert_eq!(app.viewport_size, [1000.0, 600.0]);
        assert_eq!(app.mouse_pos, [100.0, 60.0]);
        assert_eq!(app.ctx.font().raster_scale(), 1.5);
        assert_eq!(app.ctx.font().face_count(), face_count);
        assert_eq!(
            app.ctx.font_fallback_chain(fallback_face),
            Some(fallback_chain.as_slice())
        );
        assert_eq!(
            app.ctx.font_fallback_chain(FontId::DEFAULT),
            Some(emoji_chain.as_slice())
        );
        assert_eq!(app.ctx.font().face_info(FontId(4)).unwrap().px_size, 18.0);
        assert_eq!(
            app.ui_font_faces,
            [Some(FontId(1)), None, Some(FontId(2)), Some(FontId(3))]
        );

        app.update_display_metrics(winit::dpi::PhysicalSize::new(2000, 1200), 2.0);
        assert_eq!(app.viewport_size, [1000.0, 600.0]);
        assert_eq!(app.ctx.font().raster_scale(), 2.0);
        assert_eq!(app.ctx.font().face_count(), face_count);
        assert_eq!(
            app.ctx.font_fallback_chain(fallback_face),
            Some(fallback_chain.as_slice())
        );
        assert_eq!(
            app.ctx.font_fallback_chain(FontId::DEFAULT),
            Some(emoji_chain.as_slice())
        );
    }
}

#[cfg(test)]
mod theme_tests {
    use super::*;

    #[test]
    fn nightfall_palette_and_compact_metrics_match_the_design() {
        let style = nightfall_style();
        assert_eq!(style.window_bg, [0x0E, 0x10, 0x15, 255]);
        assert_eq!(style.title_bg, [0x08, 0x09, 0x0C, 255]);
        assert_eq!(style.title_text, [0xE2, 0xE5, 0xED, 255]);
        assert_eq!(style.widget_bg, [0x15, 0x18, 0x20, 255]);
        assert_eq!(style.widget_bg_hovered, [0x7C, 0x75, 0xFF, 255]);
        assert_eq!(style.widget_bg_active, [0x63, 0x5B, 0xFF, 255]);
        assert_eq!(style.text, [0xE2, 0xE5, 0xED, 255]);
        assert_eq!(style.window_padding, [16.0, 16.0]);
        assert_eq!(style.item_spacing, [8.0, 6.0]);
        assert_eq!(style.border_size, 1.0);
        assert!((2.0..=3.0).contains(&style.corner_rounding));
    }

    #[test]
    fn selecting_nightfall_replaces_live_style_edits() {
        let ctx = Context::new(DEJAVU_SANS).unwrap();
        let mut app = App::new(ctx, [FontId(0); 3]);
        app.state.style.text = [1, 2, 3, 255];
        app.state.theme = 2;
        app.apply_theme();
        assert_eq!(THEME_NAMES[app.state.theme], "Nightfall");
        assert_eq!(app.state.style, nightfall_style());
        assert_eq!(app.ctx.style, nightfall_style());
    }
}
