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
            effects: 0,
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
        },
        DrawCmd {
            effects: 0,
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
        },
        DrawCmd {
            effects: 0,
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::FONT,
        },
    ];
    let draw = DrawData {
        effects: &[],
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
        effects: 0,
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        effects: &[],
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
        effects: 0,
        elem_count: 6,
        clip_rect: [10.0, 10.0, 20.0, 20.0],
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        effects: &[],
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
            effects: 0,
            elem_count: 3,
            clip_rect: [30.0, 10.0, 5.0, 50.0],
            texture: TextureId::WHITE,
        },
        DrawCmd {
            effects: 0,
            elem_count: 3,
            clip_rect: [10.0, 40.0, 50.0, 10.0],
            texture: TextureId::WHITE,
        },
    ];
    let draw = DrawData {
        effects: &[],
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
            effects: 0,
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId(42),
        },
        DrawCmd {
            effects: 0,
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
        },
    ];
    let draw = DrawData {
        effects: &[],
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
        effects: 0,
        elem_count: indices.len() as u32,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        effects: &[],
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
        effects: 0,
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        effects: &[],
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
        effects: 0,
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        effects: &[],
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
        effects: 0,
        elem_count: indices.len() as u32,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        effects: &[],
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

#[test]
fn per_pixel_radial_conic_and_nested_alpha_masks() {
    let (mut backend, target, queue) = setup();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [0.0; 2],
        [64.0; 2],
        [0.0; 2],
        [255; 4],
    );
    let commands = [DrawCmd {
        effects: 1,
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let mut effect = restricted_gui::effects::DrawEffects::default();
    effect.gradient.rect = full_clip();
    effect.gradient.origin = [0.5; 2];
    effect.gradient.vector = [0.5; 2];
    effect.gradient.kind = 2;
    effect.gradient.stop_count = 2;
    effect.gradient.stops[0] = restricted_gui::effects::GradientStop {
        position: 0.0,
        color: [255, 0, 0, 255],
    };
    effect.gradient.stops[1] = restricted_gui::effects::GradientStop {
        position: 1.0,
        color: [0, 0, 255, 255],
    };
    let effects = [effect];
    let draw = DrawData {
        effects: &effects,
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };
    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert!(
        px(&pixels, 32, 32)[0] > 240 && px(&pixels, 32, 32)[2] < 15,
        "radial origin"
    );
    assert!(
        px(&pixels, 60, 32)[2] > 220 && px(&pixels, 60, 32)[0] < 35,
        "radial rim"
    );
    effect.gradient.kind = 3;
    effect.gradient.vector = [1.0, 0.0];
    let effects = [effect];
    let draw = DrawData {
        effects: &effects,
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };
    let pixels = render_on(&mut backend, &target, &queue, &draw);
    let samples = [
        px(&pixels, 60, 32)[2],
        px(&pixels, 32, 60)[2],
        px(&pixels, 4, 32)[2],
        px(&pixels, 32, 4)[2],
    ];
    assert!(
        samples[0] < 5
            && (58..=68).contains(&samples[1])
            && (122..=132).contains(&samples[2])
            && (187..=197).contains(&samples[3]),
        "clockwise conic quarters: {samples:?}"
    );
    backend
        .add_host_texture(TextureId(42), &[10, 40, 80, 128], 1, 1)
        .unwrap();
    effect.gradient = Default::default();
    effect.masks.count = 2;
    effect.masks.ops[0] = restricted_gui::effects::MaskOp {
        texture: 42,
        kind: 1,
        rect: [8.0, 8.0, 56.0, 56.0],
        params: [0.0, 0.0, 1.0, 1.0],
        ..Default::default()
    };
    effect.masks.ops[1] = restricted_gui::effects::MaskOp {
        kind: 2,
        rect: [0.0, 0.0, 64.0, 0.0],
        params: [1.0, 0.0, 0.0, 0.0],
        ..Default::default()
    };
    let effects = [effect];
    let draw = DrawData {
        effects: &effects,
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };
    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert_eq!(
        px(&pixels, 4, 32),
        BG,
        "texture mask excludes outside rectangle"
    );
    assert!(
        (60..=67).contains(&px(&pixels, 32, 32)[0]),
        "texture and linear masks multiply: {:?}",
        px(&pixels, 32, 32)
    );
    assert!(
        px(&pixels, 12, 32)[0] > px(&pixels, 48, 32)[0],
        "nested fade direction"
    );
}

#[test]
fn invalid_effect_and_missing_mask_skip_without_losing_index_offsets() {
    let (mut backend, target, queue) = setup();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [2.0; 2],
        [12.0; 2],
        [0.0; 2],
        [255, 0, 0, 255],
    );
    push_quad(
        &mut vertices,
        &mut indices,
        [20.0; 2],
        [30.0; 2],
        [0.0; 2],
        [0, 255, 0, 255],
    );
    push_quad(
        &mut vertices,
        &mut indices,
        [40.0; 2],
        [50.0; 2],
        [0.0; 2],
        [255; 4],
    );
    let mut effect = restricted_gui::effects::DrawEffects::default();
    effect.masks.count = 1;
    effect.masks.ops[0] = restricted_gui::effects::MaskOp {
        kind: 1,
        texture: 999,
        rect: full_clip(),
        params: [0.0, 0.0, 1.0, 1.0],
        ..Default::default()
    };
    let effects = [effect];
    let commands = [
        DrawCmd {
            effects: 99,
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
        },
        DrawCmd {
            effects: 1,
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
        },
        DrawCmd {
            effects: 0,
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
        },
    ];
    let draw = DrawData {
        effects: &effects,
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };
    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert_eq!(px(&pixels, 5, 5), BG);
    assert_eq!(px(&pixels, 25, 25), BG);
    assert_eq!(
        px(&pixels, 45, 45),
        [255; 4],
        "identity command after skipped effects"
    );
}

#[test]
fn sdf_edges_border_placement_glow_highlight_and_reveal_pixels() {
    let (mut backend, target, queue) = setup();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [0.0; 2],
        [64.0; 2],
        [0.0; 2],
        [255; 4],
    );
    let commands = [DrawCmd {
        effects: 1,
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
    }];
    let mut render = |effect: restricted_gui::effects::DrawEffects| {
        let effects = [effect];
        let draw = DrawData {
            effects: &effects,
            vertices: &vertices,
            indices: &indices,
            commands: &commands,
        };
        render_on(&mut backend, &target, &queue, &draw)
    };
    let mut effect = restricted_gui::effects::DrawEffects::default();
    effect.sdf.rect = [8.0, 8.0, 56.0, 56.0];
    effect.sdf.radii = [12.0, 0.0, 8.0, 4.0];
    effect.sdf.parameters = [1.0, 1.0, 0.0, 4.0];
    effect.sdf.light = [-1.0, 0.0, 2.0, 1.0];
    let pixels = render(effect);
    assert_eq!(px(&pixels, 9, 9), BG, "rounded TL cutout");
    assert_eq!(px(&pixels, 54, 9), [255; 4], "square TR preserved");
    assert_eq!(px(&pixels, 32, 32), [255; 4], "fill interior");
    effect.sdf.parameters[0] = 2.0;
    let pixels = render(effect);
    assert_eq!(px(&pixels, 32, 8), [255; 4], "one-pixel inner border");
    assert_eq!(px(&pixels, 32, 9), BG, "no second inner row");
    assert_eq!(px(&pixels, 32, 7), BG, "inner border does not grow out");
    effect.sdf.parameters[2] = 2.0;
    let pixels = render(effect);
    assert_eq!(px(&pixels, 32, 7), [255; 4], "one-pixel outer border");
    assert_eq!(px(&pixels, 32, 8), BG, "outer border stays outside");
    effect.sdf.parameters[2] = 1.0;
    let pixels = render(effect);
    assert!((120..=135).contains(&px(&pixels, 32, 7)[0]));
    assert!((120..=135).contains(&px(&pixels, 32, 8)[0]));
    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        for phase in [0.0_f32, 0.25, 0.5, 0.75] {
            effect.sdf.rect = [
                8.0 * scale,
                8.0 * scale + phase,
                24.0 * scale,
                24.0 * scale + phase,
            ];
            effect.sdf.radii = [4.0 * scale; 4];
            let pixels = render(effect);
            let x = (16.0 * scale) as usize;
            let y = (8.0 * scale + phase) as usize;
            let energy: u32 = (y - 2..y + 4)
                .map(|row| px(&pixels, x, row)[0] as u32)
                .sum();
            assert!(
                (250..=260).contains(&energy),
                "1px coverage at scale {scale}, phase {phase}: {energy}"
            );
        }
    }
    effect.sdf.rect = [8.0, 8.0, 56.0, 56.0];
    effect.sdf.radii = [12.0, 0.0, 8.0, 4.0];
    effect.sdf.parameters = [3.0, 1.0, 0.0, 4.0];
    let pixels = render(effect);
    assert!(
        px(&pixels, 32, 7)[0] > px(&pixels, 32, 2)[0],
        "halo decays outwards"
    );
    effect.sdf.parameters[0] = 4.0;
    let pixels = render(effect);
    assert!(
        px(&pixels, 32, 9)[0] > px(&pixels, 32, 25)[0],
        "edge glow decays inwards too"
    );
    effect.sdf.parameters = [5.0, 3.0, 0.0, 4.0];
    let pixels = render(effect);
    assert_eq!(
        px(&pixels, 8, 32),
        [255; 4],
        "left light reaches left normal"
    );
    assert_eq!(px(&pixels, 55, 32), BG, "opposite edge unlit");
    effect.sdf = Default::default();
    effect.masks.count = 1;
    effect.masks.ops[0] = restricted_gui::effects::MaskOp {
        kind: 3,
        rect: full_clip(),
        params: [0.5, 0.0, 0.0, 0.0],
        ..Default::default()
    };
    for kind in 3..=7 {
        effect.masks.ops[0].kind = kind;
        for softness in [0.0, 0.2] {
            effect.masks.ops[0].params[1] = softness;
            for inverted in [0.0, 1.0] {
                effect.masks.ops[0].params[2] = inverted;
                for progress in [0.0, 1.0] {
                    effect.masks.ops[0].params[0] = progress;
                    let pixels = render(effect);
                    let expected = if progress != inverted { [255; 4] } else { BG };
                    for point in [(0, 0), (32, 32), (63, 63)] {
                        assert_eq!(
                            px(&pixels, point.0, point.1),
                            expected,
                            "exact reveal endpoint"
                        );
                    }
                }
            }
        }
    }
    effect.masks.ops[0].kind = 3;
    effect.masks.ops[0].params = [0.5, 0.2, 0.0, 0.0];
    let pixels = render(effect);
    assert_eq!(px(&pixels, 8, 32), [255; 4]);
    assert_eq!(px(&pixels, 56, 32), BG);
    assert!(
        (90..=165).contains(&px(&pixels, 32, 32)[0]),
        "soft midpoint"
    );
    effect.masks.ops[0].kind = 7;
    effect.masks.ops[0].params = [0.5, 0.0, 0.0, 0.0];
    let pixels = render(effect);
    assert_eq!(px(&pixels, 32, 32), [255; 4]);
    assert_eq!(px(&pixels, 0, 0), BG, "radial corners still hidden");
}

#[test]
fn surface_desaturation_and_grain_preserve_alpha_and_caller_phase() {
    use restricted_gui::effects::{DrawEffects, SurfaceStyle, SURFACE_DESATURATE, SURFACE_GRAIN};
    let (mut backend, target, queue) = setup();
    let texture = TextureId(77);
    backend
        .add_host_texture(texture, &[200, 40, 20, 255], 1, 1)
        .unwrap();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [0., 0.],
        [64., 64.],
        [0.5, 0.5],
        [255; 4],
    );
    let commands = [DrawCmd {
        elem_count: 6,
        clip_rect: full_clip(),
        texture,
        effects: 1,
    }];
    let mut effect = DrawEffects {
        surface: SurfaceStyle {
            flags: SURFACE_DESATURATE,
            rect: full_clip(),
            ..Default::default()
        },
        ..Default::default()
    };
    {
        let mut render = |effect| {
            let effects = [effect];
            let draw = DrawData {
                vertices: &vertices,
                indices: &indices,
                commands: &commands,
                effects: &effects,
            };
            render_on(&mut backend, &target, &queue, &draw)
        };
        let pixels = render(effect);
        let gray = px(&pixels, 32, 32);
        assert!(gray[..3].iter().all(|c| (71..=75).contains(c)));
        assert_eq!(gray[3], 255);
        effect.surface.flags = SURFACE_GRAIN;
        effect.surface.grain = 0.25;
        let a = render(effect);
        let b = render(effect);
        assert_eq!(a, b, "frozen phase is deterministic");
        effect.surface.phase = 1.;
        let c = render(effect);
        assert_ne!(a, c, "grain changes with explicit phase");
        assert!(c.chunks_exact(4).all(|p| p[3] == 255));
    }
    for vertex in &mut vertices {
        vertex.col = [200, 40, 20, 255];
    }
    effect.surface.flags = SURFACE_DESATURATE;
    let commands = [DrawCmd {
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
        effects: 1,
    }];
    let effects = [effect];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &effects,
    };
    let solid = render_on(&mut backend, &target, &queue, &draw);
    assert!(
        px(&solid, 32, 32)[..3]
            .iter()
            .all(|c| (71..=75).contains(c)),
        "solid color is graded too"
    );
}

#[test]
fn stencil_push_pop_uses_rounded_contours_and_leaves_later_content_unclipped() {
    use restricted_gui::effects::{DrawEffects, SdfShape};
    let (mut backend, target, queue) = setup();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for color in [[255; 4], [255, 0, 0, 255], [255; 4], [0, 255, 0, 255]] {
        push_quad(
            &mut vertices,
            &mut indices,
            [0., 0.],
            [64., 64.],
            [0., 0.],
            color,
        );
    }
    let contour = SdfShape {
        rect: [8., 8., 56., 56.],
        radii: [16.; 4],
        parameters: [1., 0., 0., 0.],
        ..Default::default()
    };
    let effects = [
        DrawEffects {
            sdf: contour,
            stencil: [0, 1],
            ..Default::default()
        },
        DrawEffects {
            stencil: [1, 0],
            ..Default::default()
        },
        DrawEffects {
            sdf: contour,
            stencil: [1, 2],
            ..Default::default()
        },
    ];
    let commands = [
        DrawCmd {
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
            effects: 1,
        },
        DrawCmd {
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
            effects: 2,
        },
        DrawCmd {
            elem_count: 6,
            clip_rect: full_clip(),
            texture: TextureId::WHITE,
            effects: 3,
        },
        DrawCmd {
            elem_count: 6,
            clip_rect: [0., 0., 4., 4.],
            texture: TextureId::WHITE,
            effects: 0,
        },
    ];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &effects,
    };
    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert_eq!(px(&pixels, 32, 32), [255, 0, 0, 255]);
    assert_eq!(px(&pixels, 9, 9), BG);
    assert_eq!(
        px(&pixels, 2, 2),
        [0, 255, 0, 255],
        "pop restores unrestricted draws"
    );
}

#[test]
fn gpu_capture_blur_and_threshold_bloom_reuse_targets_and_preserve_identity() {
    use restricted_gui::effects::{Blur, BlurMethod, FilterPacket, FilterPlan, ImageFilter};
    use restricted_gui_d3d12::{BackdropCapture, PostProcessor};
    let (mut backend, target, queue) = setup();
    let mut rgba = vec![0; 64 * 64 * 4];
    for p in rgba.chunks_exact_mut(4) {
        p[3] = 255;
    }
    for y in 28..36 {
        for x in 28..36 {
            rgba[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4].copy_from_slice(&[255; 4]);
        }
    }
    backend
        .add_host_texture(TextureId(77), &rgba, 64, 64)
        .unwrap();
    let packet = FilterPacket::new([64, 64], TextureId(77), ImageFilter::default()).unwrap();
    let draw = packet.draw_data();
    let original = render_on(&mut backend, &target, &queue, &draw);
    let mut capture = BackdropCapture::new(TextureId(78)).unwrap();
    let scene_resource = target.resource().clone();
    let source = capture.capture(&mut backend, &scene_resource).unwrap();
    let mut processor = PostProcessor::new(TextureId(100)).unwrap();
    for method in [BlurMethod::Separable, BlurMethod::DualKawase] {
        let identity = FilterPlan::blur(
            [64, 64],
            Blur {
                method,
                radius: 0.,
                levels: 2,
            },
        )
        .unwrap();
        let out = processor.process(&mut backend, source, &identity).unwrap();
        let packet = FilterPacket::new([64, 64], out, ImageFilter::default()).unwrap();
        let draw = packet.draw_data();
        assert_eq!(
            render_on(&mut backend, &target, &queue, &draw),
            original,
            "zero radius preserves scene"
        );
        let plan = FilterPlan::blur(
            [64, 64],
            Blur {
                method,
                radius: 4.,
                levels: 2,
            },
        )
        .unwrap();
        let out = processor.process(&mut backend, source, &plan).unwrap();
        let packet = FilterPacket::new([64, 64], out, ImageFilter::default()).unwrap();
        let draw = packet.draw_data();
        let blurred = render_on(&mut backend, &target, &queue, &draw);
        assert!(
            px(&blurred, 27, 32)[0] > 0,
            "blur spreads beyond original edge"
        );
        assert!(blurred.chunks_exact(4).all(|p| p[3] == 255));
        let gated = FilterPlan::bloom(
            [64, 64],
            Blur {
                method,
                radius: 4.,
                levels: 2,
            },
            1.,
            2.,
        )
        .unwrap();
        let out = processor.process(&mut backend, source, &gated).unwrap();
        let packet = FilterPacket::new([64, 64], out, ImageFilter::default()).unwrap();
        let draw = packet.draw_data();
        assert_eq!(
            render_on(&mut backend, &target, &queue, &draw),
            original,
            "threshold 1 suppresses LDR bloom"
        );
    }
    assert!(processor
        .process(
            &mut backend,
            TextureId(9999),
            &FilterPlan::blur([64, 64], Blur::default()).unwrap()
        )
        .is_err());
    target.readback(&queue, backend.sync_mut()).unwrap();
}

#[test]
fn sampleable_render_target_applies_managed_color_and_rejects_feedback() {
    let (mut backend, target, queue) = setup();
    let render_texture = restricted_gui_d3d12::RenderTexture::new_encoded(
        &mut backend,
        TextureId(50),
        [W, H],
        restricted_gui_d3d12::ColorMode::Srgb,
    )
    .unwrap();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [0.; 2],
        [W as f32, H as f32],
        [0.; 2],
        [128, 128, 128, 128],
    );
    let commands = [DrawCmd {
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
        effects: 0,
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &[],
    };
    render_texture
        .render(&mut backend, &draw, [0., 0., 0., 1.])
        .unwrap();
    let sample_vertices: Vec<_> = vertices
        .iter()
        .map(|v| DrawVert {
            col: [255; 4],
            ..*v
        })
        .collect();
    let sample_commands = [DrawCmd {
        texture: render_texture.id(),
        ..commands[0]
    }];
    let sampled = DrawData {
        vertices: &sample_vertices,
        indices: &indices,
        commands: &sample_commands,
        effects: &[],
    };
    assert!(render_texture
        .render(&mut backend, &sampled, [0.; 4])
        .is_err());
    backend
        .add_gpu_texture(TextureId(51), render_texture.resource())
        .unwrap();
    let alias_commands = [DrawCmd {
        texture: TextureId(51),
        ..commands[0]
    }];
    let alias_draw = DrawData {
        commands: &alias_commands,
        ..sampled
    };
    assert!(render_texture
        .render(&mut backend, &alias_draw, [0.; 4])
        .is_err());
    backend
        .add_host_texture(TextureId(50), &[255; 4], 1, 1)
        .unwrap();
    assert!(render_texture
        .render(&mut backend, &sampled, [0.; 4])
        .is_err());
    render_texture
        .render(&mut backend, &draw, [0., 0., 0., 1.])
        .unwrap();

    let pixels = render_on(&mut backend, &target, &queue, &sampled);
    let center = px(&pixels, 32, 32);
    assert!(
        (center[0] as i32 - 28).abs() <= 2,
        "managed gray over black must blend in linear space: {center:?}"
    );
}

#[test]
fn geometry_storage_reuses_capacity_and_gpu_timing_finishes() {
    let (mut backend, target, queue) = setup();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [2., 2.],
        [30., 30.],
        [0.; 2],
        [255; 4],
    );
    let commands = [DrawCmd {
        elem_count: 6,
        clip_rect: full_clip(),
        texture: TextureId::WHITE,
        effects: 0,
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &[],
    };
    backend.reserve_upload_capacity(8 * 1024 * 1024).unwrap();
    for _ in 0..2 {
        render_on(&mut backend, &target, &queue, &draw);
    }
    backend.set_upload_capacity(4 * 1024 * 1024).unwrap();
    for _ in 0..2 {
        render_on(&mut backend, &target, &queue, &draw);
    }
    backend.reserve_upload_capacity(8 * 1024 * 1024).unwrap();
    for _ in 0..2 {
        render_on(&mut backend, &target, &queue, &draw);
    }
    let stats = backend.stats();
    assert!(stats.reuses >= 4);
    assert!(stats.pool_entries > 0);
    assert!(stats.gpu_ms.is_some_and(|v| v.is_finite() && v >= 0.));
    backend.trim_upload_pool();
    assert_eq!(backend.stats().pool_entries, 0);
}

#[test]
fn pixel_density_scales_geometry_and_clip_together() {
    let (mut backend, target, queue) = setup();
    assert!(backend.set_scale_factor(f32::NAN).is_err());
    assert_eq!(backend.scale_factor(), 1.0);
    backend.set_scale_factor(2.0).unwrap();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_quad(
        &mut vertices,
        &mut indices,
        [4., 4.],
        [12., 12.],
        [0.; 2],
        [255, 0, 0, 255],
    );
    let commands = [DrawCmd {
        elem_count: 6,
        clip_rect: [0., 0., 10., 32.],
        texture: TextureId::WHITE,
        effects: 0,
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &[],
    };
    let pixels = render_on(&mut backend, &target, &queue, &draw);
    assert_eq!(px(&pixels, 9, 9), [255, 0, 0, 255]);
    assert_eq!(px(&pixels, 19, 19), [255, 0, 0, 255]);
    assert_eq!(px(&pixels, 21, 19), BG);
    assert_eq!(px(&pixels, 7, 9), BG);
}
