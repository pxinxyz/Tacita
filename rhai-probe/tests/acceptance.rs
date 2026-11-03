use restricted_gui_rhai_probe::Probe;
use restricted_gui_sdk::{DrawCmd, DrawEffects, DrawVert, FontId, Gui, InputState, ScrollFlags};
use rhai::Map;

const FONT: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
const SCRIPT: &str = include_str!("../menu.rhai");

#[derive(Debug, PartialEq)]
struct Snapshot {
    vertices: Vec<DrawVert>,
    indices: Vec<u32>,
    commands: Vec<DrawCmd>,
    effects: Vec<DrawEffects>,
}

fn snapshot(gui: &mut Gui) -> Snapshot {
    let data = gui.render();
    Snapshot {
        vertices: data.vertices.to_vec(),
        indices: data.indices.to_vec(),
        commands: data.commands.to_vec(),
        effects: data.effects.to_vec(),
    }
}

fn gui() -> Gui {
    let mut gui = Gui::new(FONT).unwrap();
    gui.set_window_rect("Rhai probe", [40.0, 40.0], [360.0, 240.0]);
    gui
}

fn input(pos: [f32; 2], down: bool) -> InputState<'static> {
    InputState {
        mouse_pos: pos,
        mouse_down: [down, false, false, false, false],
        viewport_size: [640.0, 480.0],
        ..InputState::default()
    }
}

fn frame(probe: &mut Probe, input: InputState) -> Snapshot {
    probe.gui().new_frame(input, 1.0 / 144.0);
    probe.draw().unwrap();
    snapshot(&mut probe.gui())
}

fn state(probe: &Probe) -> Map {
    probe.state().clone().cast::<Map>()
}

#[test]
fn script_geometry_matches_native_controls_and_state_survives_frames() {
    let mut probe = Probe::new(gui(), SCRIPT).unwrap();
    let mut native = gui();
    for expected_frame in 1..=3 {
        let neutral = input([-100.0, -100.0], false);
        let scripted = frame(&mut probe, neutral);
        native.new_frame(neutral, 1.0 / 144.0);
        native.begin_window("Rhai probe", ScrollFlags::NONE);
        native.label("Scripted controls");
        let mut enabled = false;
        let mut gain = 0.5;
        native.checkbox("Enabled", &mut enabled);
        native.slider_float("Gain", &mut gain, 0.0..=1.0);
        native.button("Reset");
        native.end_window();
        assert_eq!(scripted, snapshot(&mut native));
        assert!(!scripted.vertices.is_empty());
        let state = state(&probe);
        assert_eq!(state["frames"].as_int().unwrap(), expected_frame);
        assert_eq!(state["enabled"].as_bool().unwrap(), false);
        assert_eq!(state["gain"].as_float().unwrap(), 0.5);
    }
}

#[test]
fn operation_budget_stops_an_infinite_script_without_panicking() {
    let mut probe = Probe::new(gui(), "fn draw(ui) { loop {} }").unwrap();
    probe.gui().new_frame(input([-100.0; 2], false), 0.01);
    let error = probe.draw().unwrap_err();
    assert!(error.to_string().contains("operations"), "{error}");
    snapshot(&mut probe.gui());
}

#[test]
fn malformed_script_reports_parse_error() {
    assert!(Probe::new(gui(), "fn draw(ui) {").is_err());
}

#[test]
fn mouse_release_updates_script_bound_checkbox_and_retains_it() {
    let mut probe = Probe::new(gui(), SCRIPT).unwrap();
    probe
        .gui()
        .new_frame(input([-100.0; 2], false), 1.0 / 144.0);
    let checkbox = {
        let mut gui = probe.gui();
        let style = *gui.style();
        let line = gui.font_info(FontId::DEFAULT).unwrap().line_height;
        let label_height = gui.measure_text("Scripted controls")[1];
        [
            40.0 + style.window_padding[0] + line * 0.5,
            40.0 + line
                + 2.0 * style.frame_padding[1]
                + style.window_padding[1]
                + label_height
                + style.item_spacing[1]
                + line * 0.5,
        ]
    };
    probe.draw().unwrap();
    snapshot(&mut probe.gui());
    frame(&mut probe, input(checkbox, true));
    assert!(!state(&probe)["enabled"].as_bool().unwrap());
    frame(&mut probe, input(checkbox, false));
    assert!(state(&probe)["enabled"].as_bool().unwrap());
    frame(&mut probe, input([-100.0; 2], false));
    assert!(state(&probe)["enabled"].as_bool().unwrap());
}
