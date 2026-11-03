use tacita::context::Context as GuiContext;
use tacita::input::InputState;
use tacita::style::Style;
use tacita::ui::Ui;
use tacita::ScrollFlags;
use tacita_d3d11::{Context as D3dContext, D3d11Backend, Device, OffscreenTarget};

const DEJAVU_SANS: &[u8] = include_bytes!("../../../core/tests/assets/DejaVuSans.ttf");

const W: u32 = 400;
const H: u32 = 500;
const WINDOW_POS: [f32; 2] = [60.0, 60.0];
const AWAY: [f32; 2] = [-50.0, -50.0];
const MARGIN: i32 = 2;

fn px(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * W as usize + x) * 4;
    pixels[i..i + 4].try_into().expect("pixel slice")
}

fn input_at(mouse_pos: [f32; 2], left_down: bool) -> InputState<'static> {
    InputState {
        mouse_pos,
        mouse_down: [left_down, false, false, false, false],
        viewport_size: [W as f32, H as f32],
        ..Default::default()
    }
}

fn first_covered(c: f32) -> i32 {
    (c - 0.5).ceil() as i32
}

struct Harness {
    backend: D3d11Backend,
    _device: Device,
    d3dctx: D3dContext,
    target: OffscreenTarget,
}

impl Harness {
    fn new() -> Self {
        let (backend, _device, d3dctx) =
            D3d11Backend::create_device().expect("device creation failed");
        let target =
            OffscreenTarget::new(&_device, W, H).expect("offscreen target creation failed");
        Harness {
            backend,
            _device,
            d3dctx,
            target,
        }
    }

    fn frame(
        &mut self,
        ctx: &mut GuiContext,
        input: InputState,
        button_text_size: &mut [f32; 2],
    ) -> Vec<u8> {
        ctx.set_input(input);
        ctx.new_frame(1.0 / 60.0);
        ctx.ui().window("phase", ScrollFlags::NONE, |ui: &mut Ui| {
            *button_text_size = ui.measure_text("Click me");
            ui.button("Click me");
        });
        let (pixels, width, height) = ctx.font().atlas_pixels();
        self.backend
            .upload_atlas(pixels, width, height)
            .expect("atlas upload failed");
        let draw = ctx.render();
        self.target
            .clear(&self.d3dctx, [0.0, 0.0, 64.0 / 255.0, 1.0]);
        self.backend
            .render(&draw, self.target.rtv(), [W as f32, H as f32])
            .expect("render failed");
        self.target.readback(&self.d3dctx).expect("readback failed")
    }
}

fn button_rect(style: &Style, line_height: f32, pos: [f32; 2], text_size: [f32; 2]) -> [f32; 4] {
    let title_bar_height = line_height + 2.0 * style.frame_padding[1];
    let x = pos[0] + style.window_padding[0];
    let y = pos[1] + title_bar_height + style.window_padding[1];
    [
        x,
        y,
        x + text_size[0] + 2.0 * style.frame_padding[0],
        y + text_size[1] + 2.0 * style.frame_padding[1],
    ]
}

fn corner_block(pixels: &[u8], corner_min: [f32; 2], side: i32) -> Vec<[u8; 4]> {
    let ax = first_covered(corner_min[0]) - MARGIN;
    let ay = first_covered(corner_min[1]) - MARGIN;
    let n = side + 2 * MARGIN;
    let mut out = Vec::with_capacity((n * n) as usize);
    for dy in 0..n {
        for dx in 0..n {
            let (x, y) = (ax + dx, ay + dy);
            assert!(
                x >= 0 && y >= 0 && (x as u32) < W && (y as u32) < H,
                "corner block must stay inside the target"
            );
            out.push(px(pixels, x as usize, y as usize));
        }
    }
    out
}

fn render_block_map(block: &[[u8; 4]], side: i32) -> String {
    let n = (side + 2 * MARGIN) as usize;
    let mut s = String::new();
    for (i, p) in block.iter().enumerate() {
        if i % n == 0 {
            s.push('\n');
        }
        s.push_str(&alloc_fmt(*p));
        s.push(' ');
    }
    s
}

fn alloc_fmt(p: [u8; 4]) -> String {
    format!("{:02x}{:02x}{:02x}", p[0], p[1], p[2])
}

fn corners_at_offset(h: &mut Harness, delta: f32) -> ([Vec<[u8; 4]>; 4], [f32; 4], f32) {
    let mut ctx = GuiContext::new(DEJAVU_SANS).expect("test font loads");
    let style = ctx.style;
    let mut text_size = [0.0f32; 2];

    h.frame(&mut ctx, input_at(AWAY, false), &mut text_size);
    let title_bar_height = ctx.font().line_height() + 2.0 * style.frame_padding[1];
    let grab = [
        WINDOW_POS[0] + 150.0,
        WINDOW_POS[1] + title_bar_height / 2.0,
    ];
    let moved = [grab[0] + delta, grab[1] + delta];
    h.frame(&mut ctx, input_at(grab, true), &mut text_size);
    h.frame(&mut ctx, input_at(moved, true), &mut text_size);
    h.frame(&mut ctx, input_at(moved, false), &mut text_size);
    let pixels = h.frame(&mut ctx, input_at(AWAY, false), &mut text_size);

    let pos = [WINDOW_POS[0] + delta, WINDOW_POS[1] + delta];
    let rect = button_rect(&style, ctx.font().line_height(), pos, text_size);
    let [x0, y0, x1, y1] = rect;
    let w = x1 - x0;
    let hgt = y1 - y0;
    let r = style.corner_rounding.min(w.min(hgt) / 2.0);
    assert!(r > 0.0, "this test verifies the rounded-chrome path");
    let side = r as i32;
    assert_eq!(
        side as f32, r,
        "the corner square must be a whole number of pixels"
    );

    let blocks = [
        corner_block(&pixels, [x0, y0], side),
        corner_block(&pixels, [x1 - r, y0], side),
        corner_block(&pixels, [x0, y1 - r], side),
        corner_block(&pixels, [x1 - r, y1 - r], side),
    ];
    (blocks, rect, r)
}

#[test]
fn rounded_chrome_corners_are_identical_at_every_eighth_pixel_phase() {
    let mut h = Harness::new();
    const NAMES: [&str; 4] = ["top-left", "top-right", "bottom-left", "bottom-right"];

    let (reference, ref_rect, r) = corners_at_offset(&mut h, 0.0);
    let side = r as i32;

    let phase = |c: f32| c - c.floor();
    let mut seen_half_x = false;
    let mut seen_half_y = false;

    for k in 0..8u32 {
        let delta = k as f32 / 8.0;
        let (blocks, rect, r_k) = corners_at_offset(&mut h, delta);
        assert_eq!(r_k, r, "the radius must not change with the phase");
        seen_half_x |= phase(rect[2]) == 0.5;
        seen_half_y |= phase(rect[3]) == 0.5;
        seen_half_x |= phase(rect[0]) == 0.5;
        seen_half_y |= phase(rect[1]) == 0.5;

        for (i, name) in NAMES.iter().enumerate() {
            assert_eq!(
                blocks[i],
                reference[i],
                "the {name} corner differs at phase offset {delta} \
                 (rect {rect:?}, reference rect {ref_rect:?})\nphase {delta}:{}\nreference:{}",
                render_block_map(&blocks[i], side),
                render_block_map(&reference[i], side),
            );
        }
    }

    assert!(
        seen_half_x,
        "the sweep must include an x edge at phase exactly 0.5"
    );
    assert!(
        seen_half_y,
        "the sweep must include a y edge at phase exactly 0.5"
    );
}
