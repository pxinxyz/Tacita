use tacita::context::Context;
use tacita::input::InputState;
use tacita::ui::Ui;
use tacita::ScrollFlags;

const DEJAVU_SANS: &[u8] = include_bytes!("assets/DejaVuSans.ttf");

const IDLE: [u8; 4] = [60, 60, 80, 255];
const HOVER: [u8; 4] = [90, 90, 110, 255];
const ACTIVE: [u8; 4] = [200, 160, 60, 255];
const MARK: [u8; 4] = [120, 220, 120, 255];
const TEXT: [u8; 4] = [220, 220, 220, 255];

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
        ACTIVE
    } else if ui.is_hovered(rect) {
        HOVER
    } else {
        IDLE
    };
    ui.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], bg);
    if *value {
        ui.draw_rect_filled(
            [rect[0] + 6.0, rect[1] + 6.0],
            [rect[2] - 6.0, rect[3] - 6.0],
            MARK,
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
        TEXT,
    );
    toggled
}

type Vert = ([f32; 2], [u8; 4]);

fn test_context() -> Context {
    Context::new(DEJAVU_SANS).expect("DejaVu Sans must load")
}

fn frame(
    ctx: &mut Context,
    mouse_pos: [f32; 2],
    left_down: bool,
    f: impl FnOnce(&mut Ui<'_>),
) -> Vec<Vert> {
    ctx.set_input(InputState {
        mouse_pos,
        mouse_down: [left_down, false, false, false, false],
        ..Default::default()
    });
    ctx.new_frame(0.016);
    ctx.ui().window("W", ScrollFlags::NONE, f);
    let data = ctx.render();
    data.vertices.iter().map(|v| (v.pos, v.col)).collect()
}

fn fill_color_at(verts: &[Vert], rect: [f32; 4]) -> Option<[u8; 4]> {
    verts.windows(4).find_map(|quad| {
        if quad[0].0 == [rect[0], rect[1]] && quad[2].0 == [rect[2], rect[3]] {
            Some(quad[0].1)
        } else {
            None
        }
    })
}

fn find_square(verts: &[Vert], size: f32, col: [u8; 4]) -> Option<[f32; 4]> {
    find_all_squares(verts, size, col).into_iter().next()
}

fn find_all_squares(verts: &[Vert], size: f32, col: [u8; 4]) -> Vec<[f32; 4]> {
    verts
        .windows(4)
        .filter_map(|quad| {
            let (min, max) = (quad[0].0, quad[2].0);
            let matches = quad.iter().all(|v| v.1 == col)
                && (max[0] - min[0] - size).abs() < 1e-4
                && (max[1] - min[1] - size).abs() < 1e-4;
            if matches {
                Some([min[0], min[1], max[0], max[1]])
            } else {
                None
            }
        })
        .collect()
}

fn center(rect: [f32; 4]) -> [f32; 2] {
    [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
}

const AWAY: [f32; 2] = [-50.0, -50.0];

fn settle(ctx: &mut Context, value: &mut bool) -> [f32; 4] {
    let mut toggled = true;
    let verts = frame(ctx, AWAY, false, |ui| {
        toggled = toggle_square(ui, "sq##toggle", value);
    });
    assert!(!toggled, "an interaction-free frame must not toggle");
    find_square(&verts, 24.0, IDLE).expect("the toggle square must emit its idle geometry")
}

#[test]
fn idle_toggle_emits_geometry_in_idle_color() {
    let mut ctx = test_context();
    let mut value = false;
    let rect = settle(&mut ctx, &mut value);
    assert!(!value);
    assert!(rect[0] > 0.0 && rect[1] > 0.0);
    assert!(rect[2] - rect[0] == 24.0 && rect[3] - rect[1] == 24.0);
}

#[test]
fn hover_highlights_but_does_not_toggle() {
    let mut ctx = test_context();
    let mut value = false;
    let rect = settle(&mut ctx, &mut value);

    let mut toggled = true;
    let verts = frame(&mut ctx, center(rect), false, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
    });
    assert!(!toggled, "hover without a press is not a toggle");
    assert!(!value);
    assert_eq!(fill_color_at(&verts, rect), Some(HOVER));
}

#[test]
fn press_does_not_toggle_release_inside_toggles_exactly_once() {
    let mut ctx = test_context();
    let mut value = false;
    let rect = settle(&mut ctx, &mut value);
    let at = center(rect);
    let mut toggled = true;

    let verts = frame(&mut ctx, at, true, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
    });
    assert!(!toggled, "the press frame must not toggle");
    assert!(!value);
    assert_eq!(fill_color_at(&verts, rect), Some(ACTIVE));

    let outside = [rect[0] - 30.0, rect[1]];
    let verts = frame(&mut ctx, outside, true, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
    });
    assert!(!toggled);
    assert_eq!(fill_color_at(&verts, rect), Some(IDLE));

    let verts = frame(&mut ctx, at, false, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
    });
    assert!(toggled, "the release-inside frame must toggle");
    assert!(value);
    assert_eq!(fill_color_at(&verts, rect), Some(HOVER));
    let marker = [rect[0] + 6.0, rect[1] + 6.0, rect[2] - 6.0, rect[3] - 6.0];
    assert_eq!(fill_color_at(&verts, marker), Some(MARK));

    frame(&mut ctx, at, false, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
    });
    assert!(
        !toggled,
        "the toggle must not repeat after the release frame"
    );
    assert!(value);
}

#[test]
fn press_inside_release_outside_does_not_toggle_and_drops_capture() {
    let mut ctx = test_context();
    let mut value = false;
    let rect = settle(&mut ctx, &mut value);
    let mut toggled = true;

    frame(&mut ctx, center(rect), true, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
    });
    assert!(!toggled);

    let outside = [rect[0] - 30.0, rect[1]];
    frame(&mut ctx, outside, false, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
    });
    assert!(!toggled, "release outside the rect is not a toggle");
    assert!(!value);

    let verts = frame(&mut ctx, center(rect), false, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
    });
    assert!(!toggled);
    assert_eq!(fill_color_at(&verts, rect), Some(HOVER));
}

fn greedy_pad(ui: &mut Ui<'_>, label: &str) -> bool {
    let id = ui.widget_id(label);
    let rect = ui.allocate([10.0, 10.0]);
    if ui.mouse_pressed(0) {
        ui.capture(id);
    }
    let mut clicked = false;
    if ui.is_captured(id) && ui.mouse_released(0) {
        ui.release_capture(id);
        clicked = true;
    }
    ui.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], IDLE);
    clicked
}

#[test]
fn later_declaration_steals_the_capture_and_the_first_widget_does_not_fire() {
    let mut ctx = test_context();
    let mut value = false;
    let verts = frame(&mut ctx, AWAY, false, |ui| {
        toggle_square(ui, "sq##toggle", &mut value);
        greedy_pad(ui, "pad");
    });
    let rect = find_square(&verts, 24.0, IDLE).expect("toggle geometry must be emitted");
    let at = center(rect);
    let mut toggled = true;
    let mut clicked = true;

    frame(&mut ctx, at, true, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
        clicked = greedy_pad(ui, "pad");
    });
    assert!(!toggled && !clicked);

    frame(&mut ctx, at, false, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
        clicked = greedy_pad(ui, "pad");
    });
    assert!(
        !toggled,
        "a widget whose capture was stolen must not fire on release"
    );
    assert!(!value);
    assert!(clicked, "the steal's winner interacts on the release frame");
}

#[test]
fn hidden_id_suffixes_distinguish_toggles_in_one_window() {
    let mut ctx = test_context();
    let (mut a, mut b) = (false, false);
    let verts = frame(&mut ctx, AWAY, false, |ui| {
        toggle_square(ui, "sq##a", &mut a);
        toggle_square(ui, "sq##b", &mut b);
    });
    let rects = find_all_squares(&verts, 24.0, IDLE);
    assert_eq!(rects.len(), 2, "both toggles must emit their squares");
    let (rect_a, rect_b) = (rects[0], rects[1]);
    assert_ne!(rect_a, rect_b);
    let at_a = center(rect_a);
    let mut toggled_a = true;
    let mut toggled_b = true;

    frame(&mut ctx, at_a, true, |ui| {
        toggled_a = toggle_square(ui, "sq##a", &mut a);
        toggled_b = toggle_square(ui, "sq##b", &mut b);
    });
    assert!(!toggled_a && !toggled_b);

    let verts = frame(&mut ctx, at_a, false, |ui| {
        toggled_a = toggle_square(ui, "sq##a", &mut a);
        toggled_b = toggle_square(ui, "sq##b", &mut b);
    });
    assert!(toggled_a, "the pressed toggle flips");
    assert!(!toggled_b, "the other toggle must not flip from a's click");
    assert!(a && !b);
    let marker_a = [
        rect_a[0] + 6.0,
        rect_a[1] + 6.0,
        rect_a[2] - 6.0,
        rect_a[3] - 6.0,
    ];
    assert_eq!(fill_color_at(&verts, marker_a), Some(MARK));
    assert_eq!(fill_color_at(&verts, rect_b), Some(IDLE));
}

#[test]
fn mouse_edge_queries_report_exactly_once() {
    let mut ctx = test_context();
    let mut state = (true, true, true);

    frame(&mut ctx, [100.0, 100.0], true, |ui| {
        state = (ui.mouse_pressed(0), ui.mouse_down(0), ui.mouse_released(0));
    });
    assert_eq!(state, (true, true, false), "the press frame");

    frame(&mut ctx, [100.0, 100.0], true, |ui| {
        state = (ui.mouse_pressed(0), ui.mouse_down(0), ui.mouse_released(0));
    });
    assert_eq!(state, (false, true, false), "a held button is not pressed");

    frame(&mut ctx, [100.0, 100.0], false, |ui| {
        state = (ui.mouse_pressed(0), ui.mouse_down(0), ui.mouse_released(0));
    });
    assert_eq!(state, (false, false, true), "the release frame");

    frame(&mut ctx, [100.0, 100.0], false, |ui| {
        state = (ui.mouse_pressed(0), ui.mouse_down(0), ui.mouse_released(0));
    });
    assert_eq!(state, (false, false, false), "an up button is not released");
}

#[test]
#[should_panic]
fn mouse_button_index_above_4_panics() {
    let mut ctx = test_context();
    frame(&mut ctx, AWAY, false, |ui| {
        let _ = ui.mouse_down(5);
    });
}

fn release_probe(ui: &mut Ui<'_>, label: &str, other: &str) {
    let own = ui.widget_id(label);
    let foreign = ui.widget_id(other);
    ui.release_capture(own);
    ui.release_capture(foreign);
    ui.allocate([1.0, 1.0]);
}

#[test]
fn release_capture_leaves_a_foreign_capture_untouched() {
    let mut ctx = test_context();
    let mut value = false;
    let verts = frame(&mut ctx, AWAY, false, |ui| {
        toggle_square(ui, "sq##toggle", &mut value);
        release_probe(ui, "probe", "pad");
    });
    let rect = find_square(&verts, 24.0, IDLE).expect("toggle geometry must be emitted");
    let at = center(rect);
    let mut toggled = true;

    frame(&mut ctx, at, true, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
        release_probe(ui, "probe", "pad");
    });
    assert!(!toggled);

    frame(&mut ctx, at, false, |ui| {
        toggled = toggle_square(ui, "sq##toggle", &mut value);
        release_probe(ui, "probe", "pad");
    });
    assert!(
        toggled,
        "release_capture with an ID that holds nothing must not clear the toggle's capture"
    );
    assert!(value);
}
