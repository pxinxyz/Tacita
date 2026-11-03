use tacita::context::Context;
use tacita::input::InputState;
use tacita::ui::Ui;
use tacita::ScrollFlags;

const DEJAVU_SANS: &[u8] = include_bytes!("assets/DejaVuSans.ttf");

const VIEWPORT: [f32; 2] = [1000.0, 1000.0];

const WINDOW_POS: [f32; 2] = [60.0, 60.0];
const WINDOW_SIZE: [f32; 2] = [300.0, 400.0];

const MARK_SIZE: f32 = 20.0;
const FILLER_ROWS: usize = 40;

const WARMUP_FRAMES: usize = 60;
const STEADY_FRAMES: usize = 240;

const CHILD_MARK: [u8; 4] = [255, 0, 0, 255];
const WINDOW_MARK: [u8; 4] = [0, 255, 0, 255];

fn declare(ui: &mut Ui) {
    ui.window("Atlas stability", ScrollFlags::BAR, |ui| {
        ui.child("inner", [200.0, 120.0], ScrollFlags::BAR, |ui| {
            marker(ui, CHILD_MARK);
            for _ in 0..FILLER_ROWS {
                ui.label("inner row 0123456789");
            }
        });
        marker(ui, WINDOW_MARK);
        for _ in 0..FILLER_ROWS {
            ui.label("outer row 0123456789");
        }
        let _ = ui.button("Apply");
        let _ = ui.button("Cancel");
    });
}

fn marker(ui: &mut Ui<'_>, color: [u8; 4]) {
    let rect = ui.allocate([MARK_SIZE, MARK_SIZE]);
    ui.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], color);
}

type Vert = ([f32; 2], [u8; 4]);

fn frame(ctx: &mut Context, mouse_pos: [f32; 2], left_down: bool, wheel: f32) -> Vec<Vert> {
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
        declare(&mut ui);
    }
    let data = ctx.render();
    data.vertices.iter().map(|v| (v.pos, v.col)).collect()
}

fn marker_top(verts: &[Vert], color: [u8; 4]) -> f32 {
    verts
        .windows(4)
        .find_map(|quad| {
            let (min, max) = (quad[0].0, quad[2].0);
            let matches = quad.iter().all(|v| v.1 == color)
                && (max[0] - min[0] - MARK_SIZE).abs() < 1e-3
                && (max[1] - min[1] - MARK_SIZE).abs() < 1e-3;
            if matches {
                Some(min[1])
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("marker color {color:?} not found in this frame's draw data"))
}

fn atlas_state(ctx: &Context) -> (u64, u32, u32) {
    let (_pixels, w, h) = ctx.font().atlas_pixels();
    (ctx.font().atlas_generation(), w, h)
}

fn over_child() -> [f32; 2] {
    [WINDOW_POS[0] + 60.0, WINDOW_POS[1] + 90.0]
}

fn over_window_body() -> [f32; 2] {
    [WINDOW_POS[0] + 60.0, WINDOW_POS[1] + 300.0]
}

fn window_scrollbar_x(ctx: &Context) -> f32 {
    WINDOW_POS[0] + WINDOW_SIZE[0] - ctx.style.scrollbar_size / 2.0
}

fn window_track_top(ctx: &Context) -> f32 {
    WINDOW_POS[1] + ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1]
}

fn test_context() -> Context {
    let mut ctx = Context::new(DEJAVU_SANS).expect("DejaVu Sans must load");
    ctx.style.corner_rounding = 4.0;
    ctx
}

fn warmup(ctx: &mut Context) {
    for i in 0..WARMUP_FRAMES {
        let (pos, wheel) = match i % 4 {
            0 => (over_child(), -1.0),
            1 => (over_child(), 1.0),
            2 => (over_window_body(), -1.0),
            _ => (over_window_body(), 1.0),
        };
        frame(ctx, pos, false, wheel);
    }
    let x = window_scrollbar_x(ctx);
    let top = window_track_top(ctx);
    frame(ctx, [x, top + 6.0], false, 0.0);
    frame(ctx, [x, top + 6.0], true, 0.0);
    for step in 1..=8 {
        frame(ctx, [x, top + 6.0 + step as f32 * 10.0], true, 0.0);
    }
    frame(ctx, [x, top + 86.0], false, 0.0);
    for _ in 0..20 {
        frame(ctx, over_child(), false, 1.0);
    }
    for _ in 0..20 {
        frame(ctx, over_window_body(), false, 1.0);
    }
}

#[test]
fn steady_wheel_scrolling_never_changes_the_atlas() {
    let mut ctx = test_context();
    warmup(&mut ctx);

    let baseline = atlas_state(&ctx);
    let mut moved_child = false;
    let mut moved_window = false;
    let mut last_child = marker_top(&frame(&mut ctx, over_child(), false, 0.0), CHILD_MARK);
    let mut last_window = marker_top(
        &frame(&mut ctx, over_window_body(), false, 0.0),
        WINDOW_MARK,
    );

    for i in 0..STEADY_FRAMES {
        let dir = if (i / 20) % 2 == 0 { -1.0 } else { 1.0 };
        let over_inner = (i / 10) % 2 == 0;
        let pos = if over_inner {
            over_child()
        } else {
            over_window_body()
        };
        let verts = frame(&mut ctx, pos, false, dir);

        let child = marker_top(&verts, CHILD_MARK);
        let window = marker_top(&verts, WINDOW_MARK);
        moved_child |= (child - last_child).abs() > 1e-3;
        moved_window |= (window - last_window).abs() > 1e-3;
        last_child = child;
        last_window = window;

        assert_eq!(
            atlas_state(&ctx),
            baseline,
            "atlas changed on steady scrolling frame {i} \
             (generation, width, height); a per-frame mask/glyph key would \
             do exactly this"
        );
    }

    assert!(
        moved_child,
        "the inner child never scrolled — test is vacuous"
    );
    assert!(
        moved_window,
        "the window body never scrolled — test is vacuous"
    );
}

#[test]
fn scrollbar_drag_never_changes_the_atlas() {
    let mut ctx = test_context();
    warmup(&mut ctx);

    let x = window_scrollbar_x(&ctx);
    let top = window_track_top(&ctx);
    let baseline = atlas_state(&ctx);

    let before = marker_top(&frame(&mut ctx, [x, top + 6.0], false, 0.0), WINDOW_MARK);
    frame(&mut ctx, [x, top + 6.0], true, 0.0);
    assert_eq!(
        atlas_state(&ctx),
        baseline,
        "press on the scrollbar changed the atlas"
    );

    let mut moved = false;
    let mut last = before;
    for i in 0..STEADY_FRAMES {
        let half = STEADY_FRAMES / 2;
        let dy = if i < half { i } else { STEADY_FRAMES - i } as f32;
        let verts = frame(&mut ctx, [x, top + 6.0 + dy], true, 0.0);
        let now = marker_top(&verts, WINDOW_MARK);
        moved |= (now - last).abs() > 1e-3;
        last = now;
        assert_eq!(
            atlas_state(&ctx),
            baseline,
            "atlas changed on scrollbar-drag frame {i}: the thumb's mask \
             must be keyed on the corner radius, never on its rect"
        );
    }
    frame(&mut ctx, [x, top + 6.0], false, 0.0);

    assert!(
        moved,
        "the scrollbar drag never scrolled the window — test is vacuous"
    );
    assert_eq!(atlas_state(&ctx), baseline, "release changed the atlas");
    assert_ne!(
        last, before,
        "the window ended the drag exactly where it started — the drag did nothing"
    );
}

#[test]
fn mixed_scroll_interaction_never_changes_the_atlas() {
    let mut ctx = test_context();
    warmup(&mut ctx);

    let baseline = atlas_state(&ctx);
    let x = window_scrollbar_x(&ctx);
    let top = window_track_top(&ctx);

    for i in 0..STEADY_FRAMES {
        match i % 3 {
            0 => {
                frame(
                    &mut ctx,
                    over_child(),
                    false,
                    if (i / 30) % 2 == 0 { -1.0 } else { 1.0 },
                );
            }
            1 => {
                let t = (i % 60) as f32 / 60.0;
                frame(
                    &mut ctx,
                    [
                        WINDOW_POS[0] + t * WINDOW_SIZE[0],
                        WINDOW_POS[1] + t * WINDOW_SIZE[1],
                    ],
                    false,
                    0.0,
                );
            }
            _ => {
                frame(&mut ctx, [x, top + 6.0 + (i % 120) as f32], true, 0.0);
            }
        }
        assert_eq!(
            atlas_state(&ctx),
            baseline,
            "atlas changed on mixed frame {i}"
        );
    }
    frame(&mut ctx, [x, top + 6.0], false, 0.0);
    assert_eq!(atlas_state(&ctx), baseline);
}
