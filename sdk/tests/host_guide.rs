use tacita_sdk::{FontId, Gui, InputState, Key, KeyBinding, KeyEvent, ScrollFlags};

const DEJAVU_SANS: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

#[derive(Default)]
struct HostInput {
    mouse_pos: [f32; 2],
    mouse_down: [bool; 5],
    wheel: f32,
    keys: Vec<KeyEvent>,
}

impl HostInput {
    fn key(&mut self, key: Key, pressed: bool) {
        self.keys.push(if pressed {
            KeyEvent::Press(key)
        } else {
            KeyEvent::Release(key)
        });
    }

    fn text(&mut self, text: &str) {
        self.keys.extend(text.chars().map(KeyEvent::Char));
    }

    fn take(&mut self) -> InputState<'_> {
        InputState {
            mouse_pos: self.mouse_pos,
            mouse_down: self.mouse_down,
            mouse_wheel: self.wheel,
            keys: &self.keys,
            viewport_size: [1280.0, 720.0],
        }
    }

    fn clear(&mut self) {
        self.keys.clear();
        self.wheel = 0.0;
    }
}

#[test]
fn frame_lifecycle() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    let mut host = HostInput::default();
    let mut last_atlas_generation = 0u64;
    let mut uploads = 0u32;

    for frame in 0..3 {
        if frame == 0 {
            host.key(Key::Tab, true);
            host.key(Key::Tab, false);
            host.text("a");
            host.mouse_pos = [100.0, 100.0];
            host.wheel += 1.0;
        }
        gui.new_frame(host.take(), 0.016);
        host.clear();

        gui.begin_window("Status", ScrollFlags::NONE);
        gui.label("frame ok");
        gui.end_window();

        let (pixels, w, h) = gui.atlas_pixels();
        let generation = gui.atlas_generation();
        if generation != last_atlas_generation {
            let _ = (pixels, w, h);
            uploads += 1;
            last_atlas_generation = generation;
        }

        let draw_data = gui.render();
        assert!(!draw_data.vertices.is_empty());
        assert!(!draw_data.commands.is_empty());
    }

    assert!(uploads >= 1, "the initial atlas must upload at least once");
}

struct PanelState {
    enabled: bool,
    fov: f32,
    count: i32,
    name: String,
    theme: usize,
    accent: [u8; 4],
    toggle_key: KeyBinding,
}

impl Default for PanelState {
    fn default() -> Self {
        PanelState {
            enabled: true,
            fov: 90.0,
            count: 10,
            name: String::from("player"),
            theme: 0,
            accent: [137, 180, 250, 255],
            toggle_key: KeyBinding::None,
        }
    }
}

impl PanelState {
    fn declare(&mut self, gui: &mut Gui) {
        gui.begin_window("Panel", ScrollFlags::NONE);
        gui.checkbox("Enabled", &mut self.enabled);
        gui.tooltip_text("Master switch for the whole panel.");
        gui.slider_float("FOV", &mut self.fov, 60.0..=120.0);
        gui.slider_int("Count", &mut self.count, 0..=50);
        gui.text_input("Name", &mut self.name);
        gui.combo("Theme", &["Dark", "Light", "Solarized"], &mut self.theme);
        gui.color_picker_rgba("Accent", &mut self.accent);
        gui.keybind("Toggle key", &mut self.toggle_key);
        gui.tooltip_text("Click, then press the new binding. Escape cancels.");
        if gui.button("Reset") {
            *self = PanelState::default();
        }
        gui.begin_scroll_region("log", [280.0, 120.0]);
        for i in 0..30 {
            gui.label(&format!("log line {i}"));
        }
        gui.end_scroll_region();
        gui.end_window();
    }
}

#[test]
fn worked_panel_declares_and_interacts() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    let mut state = PanelState {
        enabled: false,
        ..PanelState::default()
    };

    for _ in 0..3 {
        gui.new_frame(InputState::default(), 0.016);
        state.declare(&mut gui);
        let draw_data = gui.render();
        assert!(!draw_data.vertices.is_empty());
        assert!(!draw_data.indices.is_empty());
        assert!(!draw_data.commands.is_empty());
    }

    gui.new_frame(InputState::default(), 0.016);
    let line_height = gui.measure_text("X")[1];
    let (pad_w, pad_f) = (gui.style().window_padding, gui.style().frame_padding);
    state.declare(&mut gui);
    let _ = gui.render();
    let row_h = line_height + 2.0 * pad_f[1];
    let checkbox_center = [60.0 + pad_w[0] + 8.0, 60.0 + row_h + pad_w[1] + row_h * 0.5];

    let click = |gui: &mut Gui, state: &mut PanelState, down: bool| {
        gui.new_frame(
            InputState {
                mouse_pos: checkbox_center,
                mouse_down: [down, false, false, false, false],
                ..InputState::default()
            },
            0.016,
        );
        state.declare(gui);
        let _ = gui.render();
    };
    click(&mut gui, &mut state, false);
    click(&mut gui, &mut state, true);
    click(&mut gui, &mut state, false);
    assert!(state.enabled, "the scripted click must toggle the checkbox");
}

#[test]
fn multiple_font_faces() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");

    let big = gui.add_font(DEJAVU_SANS, 24.0).expect("face registers");
    assert_eq!(gui.font_count(), 2);
    let info = gui.font_info(big).expect("registered face has metrics");
    assert_eq!(info.px_size, 24.0);

    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Status", ScrollFlags::NONE);
    gui.label("default face");
    gui.push_font(big);
    assert_eq!(gui.active_font(), big);
    gui.label("big face");
    gui.pop_font();
    gui.end_window();
    let draw_data = gui.render();
    assert!(!draw_data.vertices.is_empty());
}

struct ScrollPanel {
    lines: usize,
    detail_scrollable: bool,
    window_flags: ScrollFlags,
}

impl ScrollPanel {
    fn declare(&mut self, gui: &mut Gui) {
        gui.begin_window("Console", self.window_flags);
        gui.checkbox("Scrollable detail pane", &mut self.detail_scrollable);

        gui.begin_child("log", [220.0, 120.0], ScrollFlags::BAR);
        for i in 0..self.lines {
            gui.label(&format!("log line {i}"));
        }
        gui.end_child();

        let detail = ScrollFlags {
            scrollable: self.detail_scrollable,
            scrollbar: false,
        };
        gui.begin_child("detail", [220.0, 120.0], detail);
        for i in 0..self.lines {
            gui.label(&format!("detail line {i}"));
        }
        gui.end_child();

        for i in 0..self.lines {
            gui.label(&format!("summary line {i}"));
        }
        gui.end_window();
    }
}

fn scroll_run(detail_scrollable: bool, pos: [f32; 2]) -> Vec<tacita_sdk::DrawVert> {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    let mut panel = ScrollPanel {
        lines: 30,
        detail_scrollable,
        window_flags: ScrollFlags::BAR,
    };
    for _ in 0..2 {
        gui.new_frame(InputState::default(), 0.016);
        panel.declare(&mut gui);
        let _ = gui.render();
    }
    gui.new_frame(
        InputState {
            mouse_pos: pos,
            mouse_wheel: -3.0,
            ..InputState::default()
        },
        0.016,
    );
    panel.declare(&mut gui);
    let _ = gui.render();
    gui.new_frame(
        InputState {
            mouse_pos: pos,
            ..InputState::default()
        },
        0.016,
    );
    panel.declare(&mut gui);
    gui.render().vertices.to_vec()
}

#[test]
fn scrolling_windows_children_and_wheel_routing() {
    let detail_point = [140.0, 310.0];
    let body_point = [140.0, 430.0];
    let log_point = [140.0, 180.0];

    assert_ne!(
        scroll_run(false, body_point),
        scroll_run(false, [900.0, 600.0]),
        "the wheel over a scrollable window's body scrolls it"
    );

    assert_eq!(
        scroll_run(false, detail_point),
        scroll_run(false, body_point),
        "a non-scrollable child is transparent to routing: the wheel \
         reaches its nearest scrollable ancestor"
    );

    assert_ne!(
        scroll_run(true, detail_point),
        scroll_run(true, body_point),
        "a scrollable child takes the wheel instead of its ancestor"
    );

    assert_ne!(
        scroll_run(false, log_point),
        scroll_run(false, body_point),
        "the `ScrollFlags::BAR` child scrolls itself, not the window"
    );
}

fn scroll_frame(
    gui: &mut Gui,
    panel: &mut ScrollPanel,
    pos: [f32; 2],
    wheel: f32,
) -> Vec<tacita_sdk::DrawVert> {
    gui.new_frame(
        InputState {
            mouse_pos: pos,
            mouse_wheel: wheel,
            ..InputState::default()
        },
        0.016,
    );
    panel.declare(gui);
    gui.render().vertices.to_vec()
}

fn assert_none_frame_parks(
    wheel_at: [f32; 2],
    set_scrollable: fn(&mut ScrollPanel, bool),
    what: &str,
) {
    let away = [900.0, 600.0];
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    let mut panel = ScrollPanel {
        lines: 30,
        detail_scrollable: true,
        window_flags: ScrollFlags::BAR,
    };
    for _ in 0..2 {
        let _ = scroll_frame(&mut gui, &mut panel, away, 0.0);
    }
    let _ = scroll_frame(&mut gui, &mut panel, wheel_at, -3.0);
    let scrolled = scroll_frame(&mut gui, &mut panel, away, 0.0);

    set_scrollable(&mut panel, false);
    let parked = scroll_frame(&mut gui, &mut panel, away, 0.0);
    assert!(
        parked != scrolled,
        "premise: a NONE {what} lays its content out at the top"
    );

    set_scrollable(&mut panel, true);
    let resumed = scroll_frame(&mut gui, &mut panel, away, 0.0);
    assert!(
        resumed == scrolled,
        "a NONE frame must park a {what}'s scroll offset, not destroy it"
    );
}

#[test]
fn a_none_frame_parks_a_live_windows_and_childs_scroll_offset() {
    assert_none_frame_parks(
        [140.0, 310.0],
        |panel, on| panel.detail_scrollable = on,
        "child",
    );
    assert_none_frame_parks(
        [140.0, 430.0],
        |panel, on| {
            panel.window_flags = if on {
                ScrollFlags::BAR
            } else {
                ScrollFlags::NONE
            }
        },
        "window",
    );
}

#[test]
fn closed_window_scope_is_a_graceful_no_op() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    let mut state = PanelState::default();

    gui.new_frame(InputState::default(), 0.016);
    state.declare(&mut gui);
    let _ = gui.render();
    gui.set_window_open("Panel", false);

    gui.new_frame(InputState::default(), 0.016);
    state.declare(&mut gui);
    let draw_data = gui.render();

    let mut empty = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    empty.new_frame(InputState::default(), 0.016);
    let empty_data = empty.render();
    assert_eq!(draw_data.vertices.len(), empty_data.vertices.len());
    assert_eq!(draw_data.commands.len(), empty_data.commands.len());

    assert!(state.enabled);
    assert_eq!(state.count, 10);
    assert_eq!(state.name, "player");

    gui.set_window_open("Panel", true);
    gui.new_frame(InputState::default(), 0.016);
    state.declare(&mut gui);
    assert!(!gui.render().vertices.is_empty());
}

#[test]
#[should_panic(expected = "open scopes")]
fn render_with_an_open_window_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Oops", ScrollFlags::NONE);
    let _ = gui.render();
}

#[test]
#[should_panic(expected = "no window open")]
fn end_window_without_begin_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    gui.new_frame(InputState::default(), 0.016);
    gui.end_window();
}

#[test]
#[should_panic(expected = "nested windows are not supported")]
fn nested_begin_window_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Outer", ScrollFlags::NONE);
    gui.begin_window("Inner", ScrollFlags::NONE);
}

#[test]
#[should_panic(expected = "innermost open scope is a scroll region")]
fn mismatched_container_end_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Oops", ScrollFlags::NONE);
    gui.begin_scroll_region("log", [100.0, 50.0]);
    gui.end_collapsing_header();
}

#[test]
#[should_panic(expected = "while a container scope is open")]
fn end_window_with_an_open_container_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Oops", ScrollFlags::NONE);
    gui.begin_scroll_region("log", [100.0, 50.0]);
    gui.end_window();
}

#[test]
#[should_panic(expected = "unbalanced font stack")]
fn render_with_an_open_font_scope_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    gui.new_frame(InputState::default(), 0.016);
    gui.push_font(FontId::DEFAULT);
    let _ = gui.render();
}
