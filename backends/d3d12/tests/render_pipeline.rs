use restricted_gui::draw::{DrawCmd, DrawData, DrawVert, TextureId};
use restricted_gui_d3d12::{D3d12Backend, OffscreenTarget, Queue};

const W: u32 = 64;
const H: u32 = 64;
const BG: [u8; 4] = [0, 0, 64, 255];

fn setup() -> (D3d12Backend, OffscreenTarget, Queue) {
    let (backend, _device, queue) = D3d12Backend::create_device().expect("device creation failed");
    let target =
        OffscreenTarget::new(backend.device(), W, H).expect("offscreen target creation failed");
    (backend, target, queue)
}

fn px(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * W as usize + x) * 4;
    pixels[i..i + 4].try_into().expect("pixel slice")
}

fn render_on(
    backend: &mut D3d12Backend,
    target: &OffscreenTarget,
    queue: &Queue,
    draw: &DrawData,
) -> Vec<u8> {
    target
        .clear(queue, backend.sync_mut(), [0.0, 0.0, 64.0 / 255.0, 1.0])
        .expect("clear failed");
    backend
        .render(draw, target.resource(), target.rtv(), [W as f32, H as f32])
        .expect("render failed");
    target
        .readback(queue, backend.sync_mut())
        .expect("readback failed")
}

fn push_quad(
    vertices: &mut Vec<DrawVert>,
    indices: &mut Vec<u32>,
    min: [f32; 2],
    max: [f32; 2],
    uv: [f32; 2],
    col: [u8; 4],
) {
    let base = vertices.len() as u32;
    let v = |pos: [f32; 2]| DrawVert { pos, uv, col };
    vertices.extend_from_slice(&[v(min), v([max[0], min[1]]), v(max), v([min[0], max[1]])]);
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn full_clip() -> [f32; 4] {
    [0.0, 0.0, W as f32, H as f32]
}

#[test]
fn colored_rects_and_textured_quad_draw_exact_pixels() {
    let (mut backend, target, queue) = setup();

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [2.0, 2.0],
        [10.0, 10.0],
        [0.0, 0.0],
        [255, 0, 0, 255],
    );
    push_quad(
        &mut vertices,
        &mut indices,
        [20.0, 20.0],
        [30.0, 30.0],
        [0.0, 0.0],
        [0, 255, 0, 255],
    );
    let base = vertices.len() as u32;
    let v = |pos: [f32; 2], uv: [f32; 2]| DrawVert {
        pos,
        uv,
        col: [0, 0, 255, 255],
    };
    vertices.extend_from_slice(&[
        v([40.0, 40.0], [0.0, 0.0]),
        v([48.0, 40.0], [1.0, 0.0]),
        v([48.0, 48.0], [1.0, 1.0]),
        v([40.0, 48.0], [0.0, 1.0]),
    ]);
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

    let commands = [
        DrawCmd {
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
        },
        DrawCmd {
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
        },
        DrawCmd {
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::FONT,
        },
    ];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };

    let pixels = render_on(&mut backend, &target, &queue, &draw);

    assert_eq!(px(&pixels, 5, 5), [255, 0, 0, 255], "rect A interior");
    assert_eq!(px(&pixels, 25, 25), [0, 255, 0, 255], "rect B interior");
    assert_eq!(
        px(&pixels, 44, 44),
        [0, 0, 255, 255],
        "textured quad interior"
    );
    assert_eq!(px(&pixels, 10, 10), BG, "just outside rect A");
    assert_eq!(px(&pixels, 0, 0), BG, "top-left background");
    assert_eq!(px(&pixels, 63, 63), BG, "bottom-right background");
    assert_eq!(px(&pixels, 35, 35), BG, "gap between quads");
}

#[test]
fn blends_src_alpha_over_background() {
    let (mut backend, target, queue) = setup();

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [4.0, 4.0],
        [12.0, 12.0],
        [0.0, 0.0],
        [128, 0, 0, 128],
    );
    let commands = [DrawCmd {
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };

    let pixels = render_on(&mut backend, &target, &queue, &draw);

    assert_eq!(px(&pixels, 8, 8), [64, 0, 32, 255], "blended pixel");
    assert_eq!(px(&pixels, 20, 20), BG, "outside the quad");
}

#[test]
fn clip_rect_culls_geometry_outside_it() {
    let (mut backend, target, queue) = setup();

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [0.0, 0.0],
        [W as f32, H as f32],
        [0.0, 0.0],
        [255, 255, 255, 255],
    );
    let commands = [DrawCmd {
        elem_count: 6,
        clip_rect: [10.0, 10.0, 20.0, 20.0],
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };

    let pixels = render_on(&mut backend, &target, &queue, &draw);

    let white = [255, 255, 255, 255];
    assert_eq!(px(&pixels, 15, 15), white, "inside clip");
    assert_eq!(px(&pixels, 19, 19), white, "clip max is exclusive");
    assert_eq!(px(&pixels, 9, 15), BG, "left of clip");
    assert_eq!(px(&pixels, 20, 15), BG, "right of clip");
    assert_eq!(px(&pixels, 15, 9), BG, "above clip");
    assert_eq!(px(&pixels, 15, 20), BG, "below clip");
    assert_eq!(px(&pixels, 40, 40), BG, "far outside clip");
}

#[test]
fn empty_clip_rect_draws_nothing() {
    let (mut backend, target, queue) = setup();

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [0.0, 0.0],
        [W as f32, H as f32],
        [0.0, 0.0],
        [255, 255, 255, 255],
    );
    let commands = [
        DrawCmd {
            elem_count: 3,
            clip_rect: [30.0, 10.0, 5.0, 50.0],
            texture: TextureId::WHITE,
        },
        DrawCmd {
            elem_count: 3,
            clip_rect: [10.0, 40.0, 50.0, 10.0],
            texture: TextureId::WHITE,
        },
    ];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };

    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert!(
        pixels.chunks_exact(4).all(|p| p == BG),
        "empty clip rects must leave the whole target at the clear color"
    );
}

#[test]
fn unregistered_host_texture_skips_the_command() {
    let (mut backend, target, queue) = setup();

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [0.0, 0.0],
        [W as f32, H as f32],
        [0.0, 0.0],
        [255, 255, 255, 255],
    );
    push_quad(
        &mut vertices,
        &mut indices,
        [50.0, 50.0],
        [58.0, 58.0],
        [0.0, 0.0],
        [0, 255, 0, 255],
    );
    let commands = [
        DrawCmd {
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId(42),
        },
        DrawCmd {
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
        },
    ];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };

    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert_eq!(
        px(&pixels, 20, 20),
        BG,
        "unregistered texture draws nothing"
    );
    assert_eq!(
        px(&pixels, 54, 54),
        [0, 255, 0, 255],
        "later command still draws"
    );
}

#[test]
fn large_vertex_count_still_draws() {
    let (mut backend, target, queue) = setup();

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for _ in 0..19_999 {
        push_quad(
            &mut vertices,
            &mut indices,
            [0.0, 0.0],
            [8.0, 8.0],
            [0.0, 0.0],
            [255, 0, 0, 255],
        );
    }
    push_quad(
        &mut vertices,
        &mut indices,
        [56.0, 56.0],
        [64.0, 64.0],
        [0.0, 0.0],
        [0, 255, 0, 255],
    );
    assert!(vertices.len() > 65_536);

    let commands = [DrawCmd {
        elem_count: indices.len() as u32,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };

    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert_eq!(px(&pixels, 4, 4), [255, 0, 0, 255], "bulk quads drew");
    assert_eq!(
        px(&pixels, 60, 60),
        [0, 255, 0, 255],
        "vertices past 64k drew"
    );
    assert_eq!(px(&pixels, 30, 30), BG, "uncovered area stays bg");
}

#[test]
fn ring_reuse_and_growth_across_renders() {
    let (mut backend, target, queue) = setup();

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [2.0, 2.0],
        [10.0, 10.0],
        [0.0, 0.0],
        [255, 0, 0, 255],
    );
    let commands = [DrawCmd {
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };
    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert_eq!(px(&pixels, 5, 5), [255, 0, 0, 255], "frame 1 drew");

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [20.0, 20.0],
        [28.0, 28.0],
        [0.0, 0.0],
        [0, 255, 0, 255],
    );
    let commands = [DrawCmd {
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };
    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert_eq!(px(&pixels, 24, 24), [0, 255, 0, 255], "frame 2 drew");
    assert_eq!(px(&pixels, 5, 5), BG, "frame 1 left no residue");

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for _ in 0..60_000 {
        push_quad(
            &mut vertices,
            &mut indices,
            [0.0, 0.0],
            [8.0, 8.0],
            [0.0, 0.0],
            [255, 0, 0, 255],
        );
    }
    push_quad(
        &mut vertices,
        &mut indices,
        [56.0, 56.0],
        [64.0, 64.0],
        [0.0, 0.0],
        [0, 255, 0, 255],
    );
    assert!(vertices.len() * 20 + indices.len() * 4 > 4 * 1024 * 1024);
    let commands = [DrawCmd {
        elem_count: indices.len() as u32,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };
    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert_eq!(
        px(&pixels, 4, 4),
        [255, 0, 0, 255],
        "grown ring drew the bulk"
    );
    assert_eq!(
        px(&pixels, 60, 60),
        [0, 255, 0, 255],
        "grown ring drew the tail"
    );
}
