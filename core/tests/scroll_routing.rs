use restricted_gui::context::Context;
use restricted_gui::draw::TextureId;
use restricted_gui::input::InputState;
use restricted_gui::ui::Ui;
use restricted_gui::ScrollFlags;

const DEJAVU_SANS: &[u8] = include_bytes!("assets/DejaVuSans.ttf");

const VIEWPORT: [f32; 2] = [1000.0, 1000.0];
const AWAY: [f32; 2] = [-500.0, -500.0];

const WINDOW_POS: [f32; 2] = [60.0, 60.0];
const CASCADE_OFFSET: f32 = 24.0;

const MARK_SIZE: f32 = 20.0;
const FILLER_ROWS: usize = 30;

fn test_context() -> Context {
    Context::new(DEJAVU_SANS).expect("DejaVu Sans must load")
}

fn title_bar_height(ctx: &Context) -> f32 {
    ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1]
}

type Vert = ([f32; 2], [u8; 4]);

fn frame(
    ctx: &mut Context,
    mouse_pos: [f32; 2],
    left_down: bool,
    wheel: f32,
    f: impl FnOnce(&mut Ui),
) -> Vec<Vert> {
    ctx.set_input(InputState {
        mouse_pos,
        mouse_down: [left_down, false, false, false, false],
        mouse_wheel: wheel,
        keys: &[],
        viewport_size: VIEWPORT,
    });
    ctx.new_frame(0.016);
    {
        let mut ui = ctx.ui();
        f(&mut ui);
    }
    let data = ctx.render();
    data.vertices.iter().map(|v| (v.pos, v.col)).collect()
}

fn marker(ui: &mut Ui<'_>, color: [u8; 4]) {
    let rect = ui.allocate([MARK_SIZE, MARK_SIZE]);
    ui.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], color);
}

fn filler(ui: &mut Ui<'_>, n: usize) {
    for _ in 0..n {
        let _ = ui.allocate([MARK_SIZE, MARK_SIZE]);
    }
}

fn find_marker(verts: &[Vert], color: [u8; 4]) -> [f32; 4] {
    verts
        .windows(4)
        .find_map(|quad| {
            let (min, max) = (quad[0].0, quad[2].0);
            let matches = quad.iter().all(|v| v.1 == color)
                && (max[0] - min[0] - MARK_SIZE).abs() < 1e-3
                && (max[1] - min[1] - MARK_SIZE).abs() < 1e-3;
            if matches {
                Some([min[0], min[1], max[0], max[1]])
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("marker color {color:?} not found in this frame's draw data"))
}

fn center(rect: [f32; 4]) -> [f32; 2] {
    [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
}

fn moved_up(before: [f32; 4], after: [f32; 4]) -> f32 {
    before[1] - after[1]
}

const CHILD_MARK: [u8; 4] = [255, 0, 0, 255];
const WINDOW_MARK: [u8; 4] = [0, 255, 0, 255];

fn declare_child_and_window(ui: &mut Ui, child_flags: ScrollFlags) {
    ui.window("W", ScrollFlags::WHEEL, |ui| {
        ui.child("c", [200.0, 100.0], child_flags, |ui| {
            marker(ui, CHILD_MARK);
            filler(ui, FILLER_ROWS);
        });
        marker(ui, WINDOW_MARK);
        filler(ui, FILLER_ROWS);
    });
}

#[test]
fn routing_a_scrollable_child_beats_the_scrollable_window_containing_it() {
    let mut ctx = test_context();
    let declare = |ui: &mut Ui| declare_child_and_window(ui, ScrollFlags::WHEEL);
    frame(&mut ctx, AWAY, false, 0.0, declare);
    let before = frame(&mut ctx, AWAY, false, 0.0, declare);
    let child_before = find_marker(&before, CHILD_MARK);
    let window_before = find_marker(&before, WINDOW_MARK);

    let probe = center(child_before);
    frame(&mut ctx, probe, false, -5.0, declare);
    let after = frame(&mut ctx, probe, false, 0.0, declare);
    let child_after = find_marker(&after, CHILD_MARK);
    let window_after = find_marker(&after, WINDOW_MARK);

    assert!(
        moved_up(child_before, child_after) > 1.0,
        "the innermost registered container under the mouse (the child) must take the notch"
    );
    assert!(
        (moved_up(window_before, window_after)).abs() < 1e-3,
        "the window must NOT co-scroll when its child claimed the wheel"
    );
}

#[test]
fn routing_the_bare_body_scrolls_the_window() {
    let mut ctx = test_context();
    let declare = |ui: &mut Ui| declare_child_and_window(ui, ScrollFlags::WHEEL);
    frame(&mut ctx, AWAY, false, 0.0, declare);
    let before = frame(&mut ctx, AWAY, false, 0.0, declare);
    let child_before = find_marker(&before, CHILD_MARK);
    let window_before = find_marker(&before, WINDOW_MARK);

    let probe = center(window_before);
    frame(&mut ctx, probe, false, -5.0, declare);
    let after = frame(&mut ctx, probe, false, 0.0, declare);
    let child_after = find_marker(&after, CHILD_MARK);
    let window_after = find_marker(&after, WINDOW_MARK);

    let window_delta = moved_up(window_before, window_after);
    let child_delta = moved_up(child_before, child_after);
    assert!(
        window_delta > 1.0,
        "with no child under the mouse, the window itself must take the notch"
    );
    assert!(
        (child_delta - window_delta).abs() < 1e-3,
        "the child must not independently scroll from outside its rect: \
         child shift {child_delta} vs window shift {window_delta}"
    );
}

#[test]
fn routing_a_non_scrollable_child_falls_through_to_the_window() {
    let mut ctx = test_context();
    let declare = |ui: &mut Ui| declare_child_and_window(ui, ScrollFlags::NONE);
    frame(&mut ctx, AWAY, false, 0.0, declare);
    let before = frame(&mut ctx, AWAY, false, 0.0, declare);
    let child_before = find_marker(&before, CHILD_MARK);
    let window_before = find_marker(&before, WINDOW_MARK);

    let probe = center(child_before);
    frame(&mut ctx, probe, false, -5.0, declare);
    let after = frame(&mut ctx, probe, false, 0.0, declare);
    let child_after = find_marker(&after, CHILD_MARK);
    let window_after = find_marker(&after, WINDOW_MARK);

    let window_delta = moved_up(window_before, window_after);
    let child_delta = moved_up(child_before, child_after);
    assert!(
        window_delta > 1.0,
        "a NONE child registers no wheel-routing entry, so the notch must \
         reach the window even though the mouse sits over the child"
    );
    assert!(
        (child_delta - window_delta).abs() < 1e-3,
        "the NONE child's own offset stays pinned at zero: it only moves \
         by riding the window's scroll, never on its own"
    );
}

const INNER_MARK: [u8; 4] = [255, 128, 0, 255];
const OUTER_MARK: [u8; 4] = [0, 128, 255, 255];
const NEST_WINDOW_MARK: [u8; 4] = [128, 0, 255, 255];

fn declare_nested_children(ui: &mut Ui, inner_flags: ScrollFlags) {
    ui.window("W", ScrollFlags::WHEEL, |ui| {
        ui.child("outer", [220.0, 160.0], ScrollFlags::WHEEL, |ui| {
            ui.child("inner", [180.0, 80.0], inner_flags, |ui| {
                marker(ui, INNER_MARK);
                filler(ui, FILLER_ROWS);
            });
            marker(ui, OUTER_MARK);
            filler(ui, FILLER_ROWS);
        });
        marker(ui, NEST_WINDOW_MARK);
        filler(ui, FILLER_ROWS);
    });
}

#[test]
fn routing_nested_scrollable_children_the_innermost_wins() {
    let mut ctx = test_context();
    let declare = |ui: &mut Ui| declare_nested_children(ui, ScrollFlags::WHEEL);
    frame(&mut ctx, AWAY, false, 0.0, declare);
    let before = frame(&mut ctx, AWAY, false, 0.0, declare);
    let inner_before = find_marker(&before, INNER_MARK);
    let outer_before = find_marker(&before, OUTER_MARK);
    let window_before = find_marker(&before, NEST_WINDOW_MARK);

    let probe = center(inner_before);
    frame(&mut ctx, probe, false, -5.0, declare);
    let after = frame(&mut ctx, probe, false, 0.0, declare);
    let inner_after = find_marker(&after, INNER_MARK);
    let outer_after = find_marker(&after, OUTER_MARK);
    let window_after = find_marker(&after, NEST_WINDOW_MARK);

    let window_shift = moved_up(window_before, window_after);
    let outer_shift = moved_up(outer_before, outer_after);
    let inner_shift = moved_up(inner_before, inner_after);
    let outer_own = outer_shift - window_shift;
    let inner_own = inner_shift - outer_shift;

    assert!(
        window_shift.abs() < 1e-3,
        "test premise: the window itself must not move (the probe never left the outer child)"
    );
    assert!(
        inner_own > 1.0,
        "the innermost registered container under the mouse (the inner \
         child) must take the notch"
    );
    assert!(
        outer_own.abs() < 1e-3,
        "the outer child must NOT co-consume the wheel when its inner \
         child is scrollable and under the mouse (innermost-wins)"
    );
}

#[test]
fn routing_a_non_scrollable_child_inside_a_scrollable_child_routes_to_that_child() {
    let mut ctx = test_context();
    let declare = |ui: &mut Ui| declare_nested_children(ui, ScrollFlags::NONE);
    frame(&mut ctx, AWAY, false, 0.0, declare);
    let before = frame(&mut ctx, AWAY, false, 0.0, declare);
    let inner_before = find_marker(&before, INNER_MARK);
    let outer_before = find_marker(&before, OUTER_MARK);
    let window_before = find_marker(&before, NEST_WINDOW_MARK);

    let probe = center(inner_before);
    frame(&mut ctx, probe, false, -5.0, declare);
    let after = frame(&mut ctx, probe, false, 0.0, declare);
    let inner_after = find_marker(&after, INNER_MARK);
    let outer_after = find_marker(&after, OUTER_MARK);
    let window_after = find_marker(&after, NEST_WINDOW_MARK);

    let window_shift = moved_up(window_before, window_after);
    let outer_shift = moved_up(outer_before, outer_after);
    let inner_shift = moved_up(inner_before, inner_after);
    let outer_own = outer_shift - window_shift;
    let inner_own = inner_shift - outer_shift;

    assert!(
        window_shift.abs() < 1e-3,
        "test premise: the window itself must not move"
    );
    assert!(
        outer_own > 1.0,
        "a NONE inner child registers nothing, so the notch must reach \
         the enclosing scrollable child, not fall all the way to the window"
    );
    assert!(
        inner_own.abs() < 1e-3,
        "the NONE inner child's own offset stays pinned at zero"
    );
}

#[test]
fn a_container_at_its_clamp_limit_still_consumes_the_wheel() {
    let mut ctx = test_context();
    let declare = |ui: &mut Ui| declare_child_and_window(ui, ScrollFlags::WHEEL);
    frame(&mut ctx, AWAY, false, 0.0, declare);
    let settled = frame(&mut ctx, AWAY, false, 0.0, declare);
    let child_settled = find_marker(&settled, CHILD_MARK);
    let probe = center(child_settled);

    frame(&mut ctx, probe, false, -10_000.0, declare);
    let at_limit = frame(&mut ctx, probe, false, 0.0, declare);
    let child_at_limit = find_marker(&at_limit, CHILD_MARK);
    let window_at_limit = find_marker(&at_limit, WINDOW_MARK);
    assert!(
        moved_up(child_settled, child_at_limit) > 50.0,
        "test premise: the huge notch must have driven the child to a \
         clearly non-zero, clamped offset"
    );

    frame(&mut ctx, probe, false, -5.0, declare);
    let after_extra = frame(&mut ctx, probe, false, 0.0, declare);
    let child_after_extra = find_marker(&after_extra, CHILD_MARK);
    let window_after_extra = find_marker(&after_extra, WINDOW_MARK);

    assert!(
        (moved_up(child_at_limit, child_after_extra)).abs() < 1e-3,
        "an exhausted child must stay pinned at its clamp limit, not move further"
    );
    assert!(
        (moved_up(window_at_limit, window_after_extra)).abs() < 1e-3,
        "the exhausted child must CONSUME the notch: its enclosing window \
         must not move, even though the child itself did not either"
    );
}

const A_MARK: [u8; 4] = [255, 0, 255, 255];
const B_MARK: [u8; 4] = [0, 255, 255, 255];

fn declare_two_windows(ui: &mut Ui) {
    ui.window("A", ScrollFlags::WHEEL, |ui| {
        marker(ui, A_MARK);
        filler(ui, FILLER_ROWS);
    });
    ui.window("B", ScrollFlags::WHEEL, |ui| {
        marker(ui, B_MARK);
        filler(ui, FILLER_ROWS);
    });
}

#[test]
fn routing_only_the_z_top_of_two_overlapping_scrollable_windows_scrolls() {
    let mut ctx = test_context();
    frame(&mut ctx, AWAY, false, 0.0, declare_two_windows);
    let before = frame(&mut ctx, AWAY, false, 0.0, declare_two_windows);
    let a_before = find_marker(&before, A_MARK);
    let b_before = find_marker(&before, B_MARK);

    let body_top_b = WINDOW_POS[1] + CASCADE_OFFSET + title_bar_height(&ctx);
    let probe = [WINDOW_POS[0] + 100.0, body_top_b + 40.0];

    frame(&mut ctx, probe, false, -5.0, declare_two_windows);
    let after = frame(&mut ctx, probe, false, 0.0, declare_two_windows);
    let a_after = find_marker(&after, A_MARK);
    let b_after = find_marker(&after, B_MARK);

    assert!(
        moved_up(b_before, b_after) > 1.0,
        "the z-top window (declared last, drawn on top) must take the notch"
    );
    assert!(
        (moved_up(a_before, a_after)).abs() < 1e-3,
        "the window BEHIND it must not co-scroll"
    );
}

const LONG_ITEMS: [&str; 6] = ["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot"];

fn declare_combo_window(ui: &mut Ui, selected: &mut usize) {
    ui.window("W", ScrollFlags::WHEEL, |ui| {
        ui.combo("Combo", &LONG_ITEMS, selected);
        marker(ui, WINDOW_MARK);
        filler(ui, FILLER_ROWS);
    });
}

fn combo_frame(
    ctx: &mut Context,
    mouse: [f32; 2],
    left_down: bool,
    wheel: f32,
    viewport: [f32; 2],
    selected: &mut usize,
    popup_top: f32,
) -> (Vec<Vert>, bool, Option<f32>) {
    ctx.set_input(InputState {
        mouse_pos: mouse,
        mouse_down: [left_down, false, false, false, false],
        mouse_wheel: wheel,
        keys: &[],
        viewport_size: viewport,
    });
    ctx.new_frame(0.016);
    {
        let mut ui = ctx.ui();
        declare_combo_window(&mut ui, selected);
    }
    let data = ctx.render();
    let verts: Vec<Vert> = data.vertices.iter().map(|v| (v.pos, v.col)).collect();
    let popup_visible = data
        .commands
        .iter()
        .any(|cmd| cmd.clip_rect == [0.0, 0.0, viewport[0], viewport[1]]);
    let mut popup_min_y: Option<f32> = None;
    let mut offset = 0usize;
    for cmd in data.commands {
        let next = offset + cmd.elem_count as usize;
        if cmd.texture == TextureId::FONT && cmd.clip_rect[1] > popup_top {
            for &idx in &data.indices[offset..next] {
                let y = data.vertices[idx as usize].pos[1];
                popup_min_y = Some(popup_min_y.map_or(y, |m: f32| m.min(y)));
            }
        }
        offset = next;
    }
    (verts, popup_visible, popup_min_y)
}

#[test]
fn routing_an_open_combo_popups_list_beats_the_scrollable_window_under_it() {
    let mut ctx = test_context();
    let mut selected = 0usize;

    let row_h = ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1];
    let pad_y = ctx.style.window_padding[1];
    let spacing_y = ctx.style.item_spacing[1];
    let content_origin = [
        WINDOW_POS[0] + ctx.style.window_padding[0],
        WINDOW_POS[1] + title_bar_height(&ctx) + pad_y,
    ];
    let button = [content_origin[0] + 4.0, content_origin[1] + row_h / 2.0];
    let viewport = [
        400.0,
        content_origin[1] + row_h + 2.0 * pad_y + 2.0 * row_h + spacing_y,
    ];
    let popup_top = content_origin[1] + row_h - 1.0;

    combo_frame(
        &mut ctx,
        AWAY,
        false,
        0.0,
        viewport,
        &mut selected,
        popup_top,
    );
    combo_frame(
        &mut ctx,
        button,
        true,
        0.0,
        viewport,
        &mut selected,
        popup_top,
    );
    combo_frame(
        &mut ctx,
        button,
        false,
        0.0,
        viewport,
        &mut selected,
        popup_top,
    );
    let (opened, popup_visible, popup_min_y) = combo_frame(
        &mut ctx,
        AWAY,
        false,
        0.0,
        viewport,
        &mut selected,
        popup_top,
    );
    assert!(
        popup_visible,
        "the click sequence must have opened the combo popup"
    );
    let min_before = popup_min_y.expect("the clamped popup must emit scrollable list rows");
    let window_mark_before = find_marker(&opened, WINDOW_MARK);

    let probe = [content_origin[0] + 10.0, content_origin[1] + row_h + 10.0];
    combo_frame(
        &mut ctx,
        probe,
        false,
        -3.0,
        viewport,
        &mut selected,
        popup_top,
    );
    let (settled, popup_visible2, popup_min_y2) = combo_frame(
        &mut ctx,
        probe,
        false,
        0.0,
        viewport,
        &mut selected,
        popup_top,
    );
    assert!(
        popup_visible2,
        "the popup must still be open after the wheel"
    );
    let min_after = popup_min_y2.expect("the popup must keep emitting scrollable list rows");
    let window_mark_after = find_marker(&settled, WINDOW_MARK);

    assert!(
        min_before - min_after > 1.0,
        "the popup layer's list must take the notch (its rows move up)"
    );
    assert!(
        (window_mark_before[1] - window_mark_after[1]).abs() < 1e-3,
        "the scrollable window under an open popup must not scroll"
    );
}
