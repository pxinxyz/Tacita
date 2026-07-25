use restricted_gui::context::Context;
use restricted_gui::draw::{DrawCmd, DrawVert};
use restricted_gui::input::InputState;
use restricted_gui::style::Style;
use restricted_gui::ui::Ui;
use restricted_gui_sdk::{icon, Gui, IconFont};

const DEJAVU_SANS: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

const AWAY: [f32; 2] = [-1000.0, -1000.0];

const FIRST_WINDOW_POS: [f32; 2] = [60.0, 60.0];

type Frame = (Vec<DrawVert>, Vec<u32>, Vec<DrawCmd>);

fn input(mouse: [f32; 2], down: bool, wheel: f32) -> InputState<'static> {
    InputState {
        mouse_pos: mouse,
        mouse_down: [down, false, false],
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
    )
}

fn keystone_flat(gui: &mut Gui, state: &mut (bool, f32, i32, String)) {
    let (check, gain, count, name) = state;
    gui.begin_window("Main");
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
    ui.window("Main", |ui| {
        ui.label("status: ok");
        let _ = ui.button("Apply");
        ui.checkbox("Enabled", check);
        ui.slider_float("Gain", gain, 0.0..=1.0);
        ui.slider_int("Count", count, 0..=10);
        ui.text_input("Name", name);
    });
}

// The facade's contract is "forwards only". These tests exist so it never quietly becomes "forwards mostly".
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
    gui.begin_window("Main");
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
    ui.window("Main", |ui| {
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
    assert!(
        flat[4].2.iter().any(|cmd| cmd.clip_rect == pts.region_rect),
        "no draw command carries the region's clip rect"
    );
}

#[test]
fn closed_header_content_is_suppressed_like_the_closure_not_running() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    let flat = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main");
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
        ui.window("Main", |ui| {
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
        g.begin_window("Main");
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
        ui.window("Main", |ui| {
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
        g.begin_window("Main");
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
    gui.begin_window("Main");
    let _ = gui.render();
}

#[test]
#[should_panic(expected = "Gui::render called with open scopes")]
fn render_with_an_open_container_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main");
    gui.begin_scroll_region("Log", [200.0, 96.0]);
    let _ = gui.render();
}

#[test]
#[should_panic(expected = "Gui::begin_window called while a window scope is open")]
fn nested_begin_window_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("A");
    gui.begin_window("B");
}

#[test]
#[should_panic(expected = "innermost open scope is a scroll region")]
fn mismatched_container_end_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main");
    gui.begin_scroll_region("Log", [200.0, 96.0]);
    gui.end_collapsing_header();
}

#[test]
#[should_panic(expected = "Gui::end_tab_bar called with no tab bar open")]
fn end_tab_bar_without_an_open_bar_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("Main");
    gui.end_tab_bar();
}

#[test]
#[should_panic(expected = "Gui::end_window called while a container scope is open")]
fn end_window_with_an_open_container_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("A");
    gui.begin_scroll_region("Log", [200.0, 96.0]);
    gui.end_window();
}

#[test]
#[should_panic(expected = "Gui::begin_window called while a container scope is open")]
fn begin_window_with_an_open_container_panics() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui.new_frame(InputState::default(), 0.016);
    gui.begin_window("A");
    gui.begin_collapsing_header("Settings");
    gui.begin_window("B");
}

#[test]
fn closed_window_begin_end_is_a_graceful_noop_emitting_nothing() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main");
        g.end_window();
    });
    gui.set_window_open("Main", false);
    let frame = gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main");
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

#[test]
fn atlas_passthroughs_match_the_core() {
    let mut gui = Gui::new(DEJAVU_SANS).expect("font loads");
    let mut ctx = Context::new(DEJAVU_SANS).expect("font loads");
    assert_eq!(gui.atlas_generation(), ctx.font().atlas_generation());
    assert_eq!(gui.atlas_pixels(), ctx.font().atlas_pixels());
    gui_frame(&mut gui, AWAY, false, 0.0, |g| {
        g.begin_window("Main");
        g.label("atlas warm-up text");
        g.end_window();
    });
    ctx_frame(&mut ctx, AWAY, false, 0.0, |ui| {
        ui.window("Main", |ui| {
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
    gui.begin_window("Main");
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
        ui.window("Main", |ui| {
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
    gui.begin_window("Icons");
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
