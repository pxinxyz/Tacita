mod renderer;

use std::sync::Arc;
use std::time::Instant;

use renderer::Renderer;

use restricted_gui::context::Context;
use restricted_gui::draw::TextureId;
use restricted_gui::font::IconFont;
use restricted_gui::icon;
use restricted_gui::input::{InputState, Key, KeyBinding, KeyEvent};
use restricted_gui::style::Style;
use restricted_gui::ui::Ui;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const DEJAVU_SANS: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
const FA_SOLID: &[u8] = include_bytes!("../../core/tests/assets/fa-solid-900.ttf");

const ICON_RANGE: (u32, u32) = (icon::STAR as u32, icon::TRIANGLE_EXCLAMATION as u32);

struct DemoState {
    enabled: bool,
    speed: f32,
    count: i32,
    text: String,
    fps_text: String,
    toggle: bool,
    anti_aliasing: bool,
    theme: usize,
    circle_color: [u8; 4],
    binding: KeyBinding,
    long_list: usize,
    rounded_corners: bool,
    sandbox_clicks: u32,
    sandbox_check: bool,
    sandbox_slider: f32,
    reopen_main: bool,
    reopen_sandbox: bool,
}

impl DemoState {
    fn new() -> Self {
        DemoState {
            enabled: true,
            speed: 25.0,
            count: 10,
            text: String::from("héllo wörld — edit me!"),
            fps_text: String::new(),
            toggle: false,
            anti_aliasing: true,
            theme: 0,
            circle_color: [160, 120, 240, 255],
            binding: KeyBinding::None,
            long_list: 0,
            rounded_corners: true,
            sandbox_clicks: 0,
            sandbox_check: false,
            sandbox_slider: 0.5,
            reopen_main: false,
            reopen_sandbox: false,
        }
    }

    fn reset(&mut self) {
        self.enabled = true;
        self.speed = 25.0;
        self.count = 10;
    }
}

const THEME_NAMES: [&str; 2] = ["Catppuccin Mocha", "Default dark"];

const LONG_LIST: [&str; 30] = [
    "Item 01", "Item 02", "Item 03", "Item 04", "Item 05", "Item 06", "Item 07", "Item 08",
    "Item 09", "Item 10", "Item 11", "Item 12", "Item 13", "Item 14", "Item 15", "Item 16",
    "Item 17", "Item 18", "Item 19", "Item 20", "Item 21", "Item 22", "Item 23", "Item 24",
    "Item 25", "Item 26", "Item 27", "Item 28", "Item 29", "Item 30",
];

const LOG_LINES: usize = 30;

const MAIN_WINDOW_TITLE: &str = "restricted-gui demo";
const SANDBOX_WINDOW_TITLE: &str = "H1 sandbox##demo";

fn window_manager(ui: &mut Ui<'_>, state: &mut DemoState) {
    ui.label("Windows (reopen after a close):");
    if ui.button("Main window") {
        state.reopen_main = true;
    }
    ui.same_line();
    if ui.button("H1 sandbox") {
        state.reopen_sandbox = true;
    }
    ui.tooltip_text(
        "Reopens a window closed via its title-bar ×.\nIf both are closed, restart the demo.",
    );
}

const TEST_PATTERN_ID: TextureId = TextureId(2);
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
    ui.label("Custom widget (docs/custom-widgets.md):");
    toggle_square(ui, "canvas toggle##demo", &mut state.toggle);
    ui.label("Canvas primitives — one allocated region:");

    let canvas = ui.allocate([280.0, 232.0]);
    let (ox, oy) = (canvas[0], canvas[1]);
    let caption = [160, 160, 170, 255];

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
    ui.draw_text_at(
        [ox + 2.0, oy + 58.0],
        "t>2r ring: disk via mask, inversion via fringe",
        caption,
    );

    let (row_b, r) = (oy + 108.0, 24.0);
    ui.draw_arc(
        [ox + 28.0, row_b],
        r,
        std::f32::consts::FRAC_PI_2..=std::f32::consts::PI,
        [240, 150, 60, 255],
        4.0,
    );
    ui.draw_arc([ox + 86.0, row_b], r, 0.3..=0.45, [60, 220, 220, 255], 4.0);
    ui.draw_arc(
        [ox + 144.0, row_b],
        r,
        0.7..=(0.7 + std::f32::consts::TAU),
        [200, 200, 240, 255],
        4.0,
    );
    let anim_r = 8.0 + 14.0 * (0.5 + 0.5 * (time * 1.5).sin());
    ui.draw_circle_filled([ox + 202.0, row_b], anim_r, state.circle_color);
    ui.draw_polygon_filled(
        &[
            [ox + 234.0, row_b - 18.0],
            [ox + 252.0, row_b - 18.0],
            [ox + 268.0, row_b],
            [ox + 252.0, row_b + 18.0],
            [ox + 234.0, row_b + 18.0],
        ],
        [230, 100, 100, 255],
    );
    ui.draw_text_at(
        [ox + 2.0, oy + 136.0],
        "arcs: quarter / 1-seg / full seam",
        caption,
    );

    ui.draw_image(
        TEST_PATTERN_ID,
        [ox + 2.0, oy + 160.0],
        [ox + 66.0, oy + 224.0],
        [0.0, 0.0],
        [1.0, 1.0],
        [255, 255, 255, 255],
    );
    let window_left = ox - window_pad_x;
    ui.draw_circle_filled([window_left, oy + 192.0], 24.0, [240, 220, 80, 255]);
    ui.draw_text_at(
        [ox + 74.0, oy + 166.0],
        "draw_image: 64x64 host texture",
        caption,
    );
    ui.draw_text_at(
        [ox + 74.0, oy + 192.0],
        "half outside window = clip test",
        caption,
    );
}

struct App {
    ctx: Context,
    state: DemoState,
    themes: [Style; 2],
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    mouse_pos: [f32; 2],
    mouse_down: [bool; 3],
    wheel: f32,
    keys: Vec<KeyEvent>,
    viewport_size: [f32; 2],
    last_frame: Instant,
    frame_count: u64,
    fps_ema: f32,
    time: f32,
    last_atlas_generation: u64,
}

impl App {
    fn new(ctx: Context) -> Self {
        App {
            themes: [ctx.style, Style::default()],
            ctx,
            state: DemoState::new(),
            window: None,
            renderer: None,
            mouse_pos: [0.0; 2],
            mouse_down: [false; 3],
            wheel: 0.0,
            keys: Vec::new(),
            viewport_size: [0.0; 2],
            last_frame: Instant::now(),
            frame_count: 0,
            fps_ema: 0.0,
            time: 0.0,
            last_atlas_generation: 0,
        }
    }

    fn frame(&mut self) {
        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else {
            return;
        };

        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().clamp(0.0001, 0.25);
        self.last_frame = now;
        let fps = 1.0 / dt;
        self.fps_ema = if self.fps_ema == 0.0 {
            fps
        } else {
            self.fps_ema * 0.9 + fps * 0.1
        };
        self.state.fps_text = format!("FPS: {:.1}  ({:.2} ms)", self.fps_ema, dt * 1000.0);
        self.time += dt;

        self.ctx.set_input(InputState {
            mouse_pos: self.mouse_pos,
            mouse_down: self.mouse_down,
            mouse_wheel: self.wheel,
            keys: &self.keys,
            viewport_size: self.viewport_size,
        });
        self.keys.clear();
        self.wheel = 0.0;

        self.ctx.new_frame(dt);
        self.ctx.style = self.themes[self.state.theme];
        self.ctx.style.anti_aliasing = self.state.anti_aliasing;
        self.ctx.style.corner_rounding = if self.state.rounded_corners { 4.0 } else { 0.0 };
        let window_pad_x = self.ctx.style.window_padding[0];
        {
            let mut ui = self.ctx.ui();
            let state = &mut self.state;
            let time = self.time;
            ui.window(MAIN_WINDOW_TITLE, |ui| {
                window_manager(ui, state);
                ui.tab_bar("tabs##demo", |bar| {
                    bar.tab("Widgets", |ui| {
                        ui.label(&state.fps_text);
                        ui.checkbox("Enable feature", &mut state.enabled);
                        ui.tooltip_text("Master switch for the demo's pretend feature.\nPurely demonstrative — it gates nothing.");
                        ui.slider_float("Speed", &mut state.speed, 0.0..=100.0);
                        ui.tooltip_text("Drag the handle, or click to jump to a position.");
                        ui.slider_int("Count", &mut state.count, 0..=50);
                        if ui.button("Reset") {
                            state.reset();
                        }
                        ui.tooltip_text("Restores the widget defaults (text is kept).");
                        if ui.button(&format!("{} Apply", icon::CHECK)) {
                            eprintln!(
                                "[demo] Apply: enabled={} speed={:.2} count={} text={:?}",
                                state.enabled, state.speed, state.count, state.text
                            );
                        }
                        ui.same_line();
                        if ui.button("Clear text") {
                            state.text.clear();
                        }
                        ui.text_input("Text", &mut state.text);
                        ui.tooltip_text(
                            "UTF-8-safe editing. While this input is focused,\ntooltips UI-wide are muted (the gated-hover rule).",
                        );
                        ui.label(&format!(
                            "{} {} {} {}  Font Awesome icons, inline with text",
                            icon::STAR, icon::GEAR, icon::HOME, icon::EYE
                        ));
                        ui.tooltip_text(
                            "M4 icon fonts: the PUA range covering the curated\n\
                             icon::* constants was baked into the same atlas at\n\
                             init (Context::new_with_icons) — icons measure,\n\
                             lay out, and recolor exactly like text glyphs.",
                        );
                        ui.label("Log (H2 scroll region):");
                        ui.scroll_region("log##demo", [280.0, 150.0], |ui| {
                            for i in 1..=LOG_LINES {
                                ui.label(&format!("log line {i:02}: the quick brown fox"));
                            }
                        });
                        ui.tooltip_text(
                            "Wheel over the region scrolls it (3 lines per notch);\n\
                             content scrolled past the fold is clipped and can\n\
                             neither hover nor be pressed.",
                        );
                    });
                    bar.tab("Canvas", |ui| {
                        canvas_showcase(ui, state, time, window_pad_x);
                    });
                    bar.tab("Settings", |ui| {
                        ui.collapsing_header("Appearance", |ui| {
                            if ui.combo("Theme", &THEME_NAMES, &mut state.theme) {
                                eprintln!("[demo] theme -> {}", THEME_NAMES[state.theme]);
                            }
                            ui.tooltip_text("Swaps the whole Style palette\nat the top of the next frame.");
                            ui.combo("Long list", &LONG_LIST, &mut state.long_list);
                            ui.tooltip_text(
                                "H2 combo scrolling: 30 items overflow the\n\
                                 viewport-clamped popup — wheel over the open\n\
                                 list scrolls it (a fitting list wouldn't).",
                            );
                            ui.checkbox("Rounded corners", &mut state.rounded_corners);
                            ui.tooltip_text(
                                "H2 rounded chrome: 9-slice corner-mask fills\n\
                                 (Style::corner_rounding 4.0, on by default).\n\
                                 Off = the pre-H2 crisp rects. Applied at the\n\
                                 top of the next frame.",
                            );
                            ui.color_edit_rgba("Circle color", &mut state.circle_color);
                            ui.tooltip_text(
                                "RGBA sliders driving the Canvas tab's\nanimated growing circle.",
                            );
                            ui.color_picker_rgba("Circle color (rich)", &mut state.circle_color);
                            ui.tooltip_text(
                                "M3.5 rich picker (SV square, hue strip, HSB + RGBA\n\
                                 sliders, hex). Same target as the color edit above —\n\
                                 edits through either one show up in the other's swatch.",
                            );
                        });
                        ui.collapsing_header("Input", |ui| {
                            ui.keybind("Action key", &mut state.binding);
                            ui.tooltip_text(
                                "Click to arm, then press any key, mouse button,\nor modifier chord. Escape cancels.",
                            );
                            ui.label(&format!("captured: {}", state.binding));
                        });
                    });
                });
            });
            ui.window(SANDBOX_WINDOW_TITLE, |ui| {
                ui.label("Overlaps the main window on purpose.");
                ui.label("Click either window: the press raises it, and");
                ui.label("only the front window's widgets react.");
                if ui.button("Click counter") {
                    state.sandbox_clicks += 1;
                }
                ui.same_line();
                ui.label(&format!("= {}", state.sandbox_clicks));
                ui.checkbox("Sandbox checkbox", &mut state.sandbox_check);
                ui.slider_float("Sandbox slider", &mut state.sandbox_slider, 0.0..=1.0);
                ui.label("Grip (bottom-right) resizes; min 120x80, no max.");
                ui.label("The × closes this window — reopen below.");
                window_manager(ui, state);
            });
        }
        if self.state.reopen_main {
            self.state.reopen_main = false;
            self.ctx.set_window_open(MAIN_WINDOW_TITLE, true);
        }
        if self.state.reopen_sandbox {
            self.state.reopen_sandbox = false;
            self.ctx.set_window_open(SANDBOX_WINDOW_TITLE, true);
        }

        let (pixels, w, h) = self.ctx.font().atlas_pixels();
        let generation = self.ctx.font().atlas_generation();
        if generation != self.last_atlas_generation {
            eprintln!("[demo] atlas re-upload: generation {generation} ({w}x{h})");
            self.last_atlas_generation = generation;
        }
        renderer.sync_atlas(pixels, w, h, generation);

        let draw = self.ctx.render();
        let (verts, indices, cmds) = (draw.vertices.len(), draw.indices.len(), draw.commands.len());
        renderer.render(&draw);

        self.frame_count += 1;
        if self.frame_count <= 3 || self.frame_count.is_multiple_of(60) {
            eprintln!(
                "[demo] frame {} rendered: {verts} verts, {indices} indices, {cmds} cmds, dt {:.2} ms",
                self.frame_count,
                dt * 1000.0
            );
        }
        window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("restricted-gui demo")
                .with_inner_size(winit::dpi::PhysicalSize::new(960u32, 720u32)),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("[demo] failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };
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
                self.renderer = Some(renderer);
                self.window = Some(window.clone());
                let size = window.inner_size();
                self.viewport_size = [size.width as f32, size.height as f32];
                self.last_frame = Instant::now();
                window.request_redraw();
            }
            Err(e) => {
                eprintln!("[demo] renderer init failed: {e}");
                event_loop.exit();
            }
        }
    }

    // winit event -> InputState, one match arm at a time. The platform adapter guide calls this "the reference implementation". It calls a lot of things.
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.viewport_size = [size.width as f32, size.height as f32];
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = [position.x as f32, position.y as f32];
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let index = match button {
                    MouseButton::Left => Some(0),
                    MouseButton::Right => Some(1),
                    MouseButton::Middle => Some(2),
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
                        KeyCode::ArrowRight => Some(Key::Right),
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
                if event.state == ElementState::Pressed {
                    if let Some(text) = &event.text {
                        self.keys.extend(text.chars().map(KeyEvent::Char));
                    }
                }
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }
}

fn main() {
    let mut ctx = Context::new_with_icons(
        DEJAVU_SANS,
        &[IconFont {
            bytes: FA_SOLID,
            range: ICON_RANGE,
        }],
    )
    .expect("bundled DejaVu Sans + Font Awesome must load");
    ctx.style.window_bg = [30, 30, 46, 255];
    ctx.style.title_bg = [24, 24, 37, 255];
    ctx.style.title_text = [205, 214, 244, 255];
    ctx.style.widget_bg = [69, 71, 90, 255];
    ctx.style.widget_bg_hovered = [137, 180, 250, 255];
    ctx.style.widget_bg_active = [203, 166, 247, 255];
    ctx.style.text = [205, 214, 244, 255];

    ctx.style.anti_aliasing = true;
    ctx.set_input(InputState {
        mouse_pos: [0.0; 2],
        mouse_down: [false; 3],
        mouse_wheel: 0.0,
        keys: &[],
        viewport_size: [960.0, 720.0],
    });
    ctx.new_frame(1.0 / 60.0);
    {
        let mut ui = ctx.ui();
        ui.window("mask warm-up##prewarm", |ui| {
            let c = [200.0, 200.0];
            let mut r = 8.0;
            while r <= 22.0 {
                ui.draw_circle_filled(c, r, [255, 255, 255, 255]);
                r += 0.25;
            }
            ui.draw_circle_filled(c, 24.0, [255, 255, 255, 255]);
            ui.draw_circle_outline(c, 20.0, [255, 255, 255, 255], 3.0);
            ui.draw_circle_outline(c, 13.0, [255, 255, 255, 255], 28.0);
        });
    }
    let _ = ctx.render();
    {
        let (_, w, h) = ctx.font().atlas_pixels();
        eprintln!(
            "[demo] shape-mask warm-up: atlas {w}x{h}, generation {}",
            ctx.font().atlas_generation()
        );
    }

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(ctx);
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("[demo] event loop error: {e}");
        std::process::exit(1);
    }
}
