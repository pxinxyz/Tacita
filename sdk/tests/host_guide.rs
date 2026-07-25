use restricted_gui_sdk::{Gui, InputState, Key, KeyBinding, KeyEvent};

const DEJAVU_SANS: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

#[derive(Default)]
struct HostInput {
    mouse_pos: [f32; 2],
    mouse_down: [bool; 3],
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

// Every snippet in docs/host-guide.md is compiled and run HERE. Documentation cannot rot when the test is the gardener.
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

        gui.begin_window("Status");
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
        gui.begin_window("Panel");
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
                mouse_down: [down, false, false],
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
    gui.begin_window("Oops");
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
    gui.begin_window("Outer");
    gui.begin_window("Inner");
}

#[test]
#[should_panic(expected = "innermost open scope is a scroll region")]
fn mismatched_container_end_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Oops");
    gui.begin_scroll_region("log", [100.0, 50.0]);
    gui.end_collapsing_header();
}

#[test]
#[should_panic(expected = "while a container scope is open")]
fn end_window_with_an_open_container_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("bundled font must load");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Oops");
    gui.begin_scroll_region("log", [100.0, 50.0]);
    gui.end_window();
}
