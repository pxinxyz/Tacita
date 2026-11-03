use restricted_gui::context::Context;
use restricted_gui::input::InputState;
use restricted_gui::ScrollFlags;

const FONT: &[u8] = include_bytes!("assets/DejaVuSans.ttf");

fn frame(ctx: &mut Context, input: InputState, items: &[&str], selected: &mut usize) -> bool {
    frame_with_vertices(ctx, input, items, selected).0
}

fn frame_with_vertices(
    ctx: &mut Context,
    input: InputState,
    items: &[&str],
    selected: &mut usize,
) -> (bool, usize) {
    ctx.set_input(input);
    ctx.new_frame(1.0 / 60.0);
    let mut changed = false;
    ctx.ui().window("list test", ScrollFlags::NONE, |ui| {
        changed = ui.list_box("##Items", items, selected, [180.0, 100.0]);
    });
    let vertices = ctx.render().vertices.len();
    (changed, vertices)
}

fn row_point(ctx: &Context, row: usize) -> [f32; 2] {
    let style = ctx.style;
    let line_h = ctx.font().line_height();
    let panel_y = 60.0 + line_h + 2.0 * style.frame_padding[1] + style.window_padding[1];
    let inset =
        style.border_size + style.corner_rounding * (1.0 - core::f32::consts::FRAC_1_SQRT_2);
    let row_h = line_h + 2.0 * style.frame_padding[1];
    [
        60.0 + style.window_padding[0] + inset + 12.0,
        panel_y + inset + row as f32 * (row_h + style.item_spacing[1]) + row_h * 0.5,
    ]
}

fn input_at(mouse: [f32; 2], down: bool, wheel: f32) -> InputState<'static> {
    InputState {
        mouse_pos: mouse,
        mouse_down: [down, false, false, false, false],
        mouse_wheel: wheel,
        viewport_size: [400.0, 500.0],
        ..Default::default()
    }
}

#[test]
fn list_box_keeps_selection_until_a_row_is_clicked() {
    let mut ctx = Context::new(FONT).unwrap();
    let items = ["same", "same", "third"];
    let mut selected = 2;
    assert!(!frame(
        &mut ctx,
        InputState::default(),
        &items,
        &mut selected
    ));
    assert_eq!(selected, 2);

    let second = row_point(&ctx, 1);
    assert!(!frame(
        &mut ctx,
        input_at(second, true, 0.0),
        &items,
        &mut selected
    ));
    assert!(frame(
        &mut ctx,
        input_at(second, false, 0.0),
        &items,
        &mut selected
    ));
    assert_eq!(selected, 1, "duplicate labels must still select by index");
    assert!(!frame(
        &mut ctx,
        input_at(second, true, 0.0),
        &items,
        &mut selected
    ));
    assert!(!frame(
        &mut ctx,
        input_at(second, false, 0.0),
        &items,
        &mut selected
    ));
}

#[test]
fn list_box_wheel_reveals_and_selects_later_rows() {
    let mut ctx = Context::new(FONT).unwrap();
    let items = ["00", "01", "02", "03", "04", "05", "06", "07", "08", "09"];
    let mut selected = 0;
    frame(&mut ctx, InputState::default(), &items, &mut selected);
    let top = row_point(&ctx, 0);
    frame(&mut ctx, input_at(top, false, -1.0), &items, &mut selected);
    frame(&mut ctx, input_at(top, false, 0.0), &items, &mut selected);
    assert!(!frame(
        &mut ctx,
        input_at(top, true, 0.0),
        &items,
        &mut selected
    ));
    assert!(frame(
        &mut ctx,
        input_at(top, false, 0.0),
        &items,
        &mut selected
    ));
    assert_eq!(selected, 2);
}

#[test]
fn large_list_emits_only_visible_rows_and_selects_the_last_item() {
    let mut ctx = Context::new(FONT).unwrap();
    let items = vec!["same"; 10_000];
    let mut selected = 0;
    let (_, top_vertices) =
        frame_with_vertices(&mut ctx, InputState::default(), &items, &mut selected);
    assert!(top_vertices < 500, "offscreen rows must not emit geometry");

    let top = row_point(&ctx, 0);
    frame(
        &mut ctx,
        input_at(top, false, -100_000.0),
        &items,
        &mut selected,
    );
    let (_, bottom_vertices) =
        frame_with_vertices(&mut ctx, input_at(top, false, 0.0), &items, &mut selected);
    assert!(bottom_vertices < 500);

    let style = ctx.style;
    let inset =
        style.border_size + style.corner_rounding * (1.0 - core::f32::consts::FRAC_1_SQRT_2);
    let row_h = ctx.font().line_height() + 2.0 * style.frame_padding[1];
    let panel_top =
        60.0 + ctx.font().line_height() + 2.0 * style.frame_padding[1] + style.window_padding[1];
    let last = [top[0], panel_top + 100.0 - inset - row_h * 0.5];
    assert!(!frame(
        &mut ctx,
        input_at(last, true, 0.0),
        &items,
        &mut selected
    ));
    assert!(frame(
        &mut ctx,
        input_at(last, false, 0.0),
        &items,
        &mut selected
    ));
    assert_eq!(selected, 9_999);
}

#[test]
fn a_pressed_row_scrolled_out_of_view_releases_without_selecting_another_row() {
    let mut ctx = Context::new(FONT).unwrap();
    let items = vec!["same"; 10_000];
    let mut selected = 9_999;
    frame(&mut ctx, InputState::default(), &items, &mut selected);
    let top = row_point(&ctx, 0);
    assert!(!frame(
        &mut ctx,
        input_at(top, true, -100_000.0),
        &items,
        &mut selected,
    ));
    assert!(!frame(
        &mut ctx,
        input_at(top, false, 0.0),
        &items,
        &mut selected,
    ));
    assert_eq!(selected, 9_999);
    assert!(!frame(
        &mut ctx,
        input_at(top, true, 0.0),
        &items,
        &mut selected
    ));
    assert!(frame(
        &mut ctx,
        input_at(top, false, 0.0),
        &items,
        &mut selected
    ));
    assert_ne!(selected, 9_999);
}

#[test]
fn lazy_list_reads_only_visible_labels() {
    let mut ctx = Context::new(FONT).unwrap();
    let mut selected = 0;
    let mut queried = Vec::new();
    ctx.set_input(InputState::default());
    ctx.new_frame(1.0 / 60.0);
    ctx.ui().window("list test", ScrollFlags::NONE, |ui| {
        assert_eq!(
            ui.list_box_with(
                "##Items",
                10_000,
                |index| {
                    queried.push(index);
                    Some("same")
                },
                &mut selected,
                [180.0, 100.0],
            ),
            Some(false)
        );
    });
    ctx.render();
    assert!(!queried.is_empty());
    assert!(queried.len() < 32, "only visible rows may be queried");
    assert!(queried.iter().all(|&index| index < 32));
}

#[test]
fn invalid_visible_label_does_not_change_selection() {
    let mut ctx = Context::new(FONT).unwrap();
    let mut selected = 2;
    let top = row_point(&ctx, 0);
    for down in [true, false] {
        ctx.set_input(input_at(top, down, 0.0));
        ctx.new_frame(1.0 / 60.0);
        ctx.ui().window("list test", ScrollFlags::NONE, |ui| {
            assert_eq!(
                ui.list_box_with(
                    "##Items",
                    3,
                    |index| (index != 1).then_some("same"),
                    &mut selected,
                    [180.0, 100.0],
                ),
                None
            );
        });
        ctx.render();
        assert_eq!(selected, 2);
    }
}

#[test]
fn row_below_panel_edge_is_not_queried() {
    let mut ctx = Context::new(FONT).unwrap();
    let style = ctx.style;
    let inset =
        style.border_size + style.corner_rounding * (1.0 - core::f32::consts::FRAC_1_SQRT_2);
    let row_h = ctx.font().line_height() + 2.0 * style.frame_padding[1];
    let panel_h = 2.0 * inset + row_h + style.item_spacing[1];
    let mut selected = 0;
    let mut queried = Vec::new();
    ctx.set_input(InputState::default());
    ctx.new_frame(1.0 / 60.0);
    ctx.ui().window("list test", ScrollFlags::NONE, |ui| {
        assert_eq!(
            ui.list_box_with(
                "##Items",
                2,
                |index| {
                    queried.push(index);
                    (index == 0).then_some("visible")
                },
                &mut selected,
                [180.0, panel_h],
            ),
            Some(false)
        );
    });
    ctx.render();
    assert!(!queried.is_empty());
    assert!(queried.iter().all(|&index| index == 0));
}
