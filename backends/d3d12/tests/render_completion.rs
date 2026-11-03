#![cfg(windows)]
use restricted_gui_d3d12::{D3d12Backend as Backend, OffscreenTarget, Queue, RenderTexture};
struct Harness {
    backend: Backend,
    queue: Queue,
    target: OffscreenTarget,
}
impl Harness {
    fn new() -> Self {
        let (backend, device, queue) = Backend::create_device().unwrap();
        let target = OffscreenTarget::new(&device, 64, 64).unwrap();
        Self {
            backend,
            queue,
            target,
        }
    }
    fn render(&mut self, draw: &DrawData) -> Vec<u8> {
        self.target
            .clear(&self.queue, self.backend.sync_mut(), [0., 0., 0.25, 1.])
            .unwrap();
        self.backend
            .render(draw, self.target.resource(), self.target.rtv(), [64.; 2])
            .unwrap();
        self.target
            .readback(&self.queue, self.backend.sync_mut())
            .unwrap()
    }
}

use restricted_gui::{
    draw::{DrawCmd, DrawData, DrawVert, TextureId},
    effects::{DrawEffects, MaskOp, MaskStack, SdfShape},
    rendering::{AlphaMode, ColorMode, DrawOptimization},
};
fn scene(overlap: bool) -> (Vec<DrawVert>, Vec<u32>, Vec<DrawCmd>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for i in 0..3 {
        let x = if overlap {
            4. + i as f32 * 3.
        } else {
            3. + i as f32 * 18.
        };
        let base = vertices.len() as u32;
        for (j, (pos, uv)) in [
            ([x, 3.], [0., 0.]),
            ([x + 14., 4.], [1., 0.]),
            ([x + 14., 30.], [1., 1.]),
            ([x, 30.], [0., 1.]),
        ]
        .into_iter()
        .enumerate()
        {
            vertices.push(DrawVert {
                pos,
                uv,
                col: [
                    (80 + i * 60) as u8,
                    (255 - j * 40) as u8,
                    200,
                    (90 + j * 35) as u8,
                ],
            });
        }
        indices.extend([0, 1, 2, 0, 2, 3].map(|index| base + index));
    }
    let commands = [2, 3, 2]
        .map(|id| DrawCmd {
            elem_count: 6,
            clip_rect: [0., 0., 64., 64.],
            texture: TextureId(id),
            effects: 0,
        })
        .to_vec();
    (vertices, indices, commands)
}
fn pixel(bytes: &[u8], x: usize, y: usize) -> [u8; 4] {
    bytes[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4]
        .try_into()
        .unwrap()
}
fn full_quad(id: TextureId) -> ([DrawVert; 4], [u32; 6], [DrawCmd; 1]) {
    let vertices = [[0., 0.], [64., 0.], [64., 64.], [0., 64.]].map(|pos| DrawVert {
        pos,
        uv: [0.; 2],
        col: [255, 220, 255, 160],
    });
    (
        vertices,
        [0, 1, 2, 0, 2, 3],
        [DrawCmd {
            elem_count: 6,
            clip_rect: [0., 0., 64., 64.],
            texture: id,
            effects: 0,
        }],
    )
}
#[test]
fn optimized_quads_match_original_interpolation_order_and_mixed_triangles() {
    let mut h = Harness::new();
    let rgba: Vec<u8> = (0..64 * 64)
        .flat_map(|p| {
            if (p % 64 / 8 + p / 64 / 8) % 2 == 0 {
                [255, 100, 60, 220]
            } else {
                [60, 190, 255, 170]
            }
        })
        .collect();
    for id in [2, 3] {
        h.backend
            .add_host_texture(TextureId(id), &rgba, 64, 64)
            .unwrap();
    }
    for overlap in [false, true] {
        let (mut vertices, mut indices, mut commands) = scene(overlap);
        let base = vertices.len() as u32;
        vertices.extend([[5., 40.], [50., 43.], [15., 59.]].map(|pos| DrawVert {
            pos,
            uv: [0.; 2],
            col: [150, 230, 170, 180],
        }));
        indices.extend([base, base + 1, base + 2]);
        commands.push(DrawCmd {
            elem_count: 3,
            clip_rect: [0., 0., 64., 64.],
            texture: TextureId::WHITE,
            effects: 0,
        });
        let draw = DrawData {
            vertices: &vertices,
            indices: &indices,
            commands: &commands,
            effects: &[],
        };
        h.backend.set_draw_optimization(DrawOptimization::default());
        let expected = h.render(&draw);
        h.backend.set_draw_optimization(DrawOptimization {
            sort_disjoint: true,
            instance_quads: true,
        });
        assert_eq!(
            h.render(&draw),
            expected,
            "quad interpolation/order changed; overlap={overlap}"
        );
    }
}
#[test]
fn premultiplied_inputs_outputs_and_alpha_masks_match_straight_reference() {
    let mut h = Harness::new();
    h.backend
        .add_host_texture(TextureId(2), &[255, 0, 0, 128].repeat(64 * 64), 64, 64)
        .unwrap();
    h.backend
        .add_host_texture(TextureId(3), &[128, 0, 0, 128].repeat(64 * 64), 64, 64)
        .unwrap();
    h.backend
        .set_texture_alpha_mode(TextureId(3), AlphaMode::Premultiplied)
        .unwrap();
    let (vertices, indices, mut commands) = full_quad(TextureId(2));
    commands[0].effects = 1;
    let mut masks = MaskStack {
        count: 1,
        ..Default::default()
    };
    masks.ops[0] = MaskOp {
        kind: 2,
        rect: [0., 0., 64., 64.],
        params: [0.2, 0.85, 0., 0.],
        ..Default::default()
    };
    let effects = [DrawEffects {
        masks,
        ..Default::default()
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &effects,
    };
    let expected = h.render(&draw);
    commands[0].texture = TextureId(3);
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &effects,
    };
    for mode in [AlphaMode::Straight, AlphaMode::Premultiplied] {
        h.backend.set_alpha_mode(mode);
        let actual = h.render(&draw);
        assert!(
            actual
                .iter()
                .zip(&expected)
                .all(|(a, b)| a.abs_diff(*b) <= 1),
            "alpha decoding or final coverage differs"
        );
    }
    h.backend
        .add_host_texture(TextureId(3), &[255, 0, 0, 128].repeat(64 * 64), 64, 64)
        .unwrap();
    let actual = h.render(&draw);
    assert!(
        actual
            .iter()
            .zip(&expected)
            .all(|(a, b)| a.abs_diff(*b) <= 1),
        "texture replacement retained stale alpha metadata"
    );
}
#[test]
fn shader_replacement_is_atomic_and_changes_pixels_without_replacing_textures() {
    let mut h = Harness::new();
    let (vertices, indices, commands) = full_quad(TextureId::WHITE);
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &[],
    };
    let expected = h.render(&draw);
    let (vertex, pixel_source) = Backend::shader_sources();
    assert!(h
        .backend
        .reload_shaders(vertex, "deliberately invalid HLSL")
        .is_err());
    assert!(h
        .backend
        .reload_shaders(
            vertex,
            &pixel_source.replace("register(b1)", "register(b7)")
        )
        .is_err());
    assert!(h
        .backend
        .reload_shaders(
            vertex,
            &pixel_source.replace(
                "float4 paintRect;",
                "float4 insertedField; float4 paintRect;"
            )
        )
        .is_err());
    assert_eq!(h.render(&draw), expected);
    let changed = pixel_source.replace(
        "return color;",
        "return float4(color.rgb * float3(0.5,1,1),color.a);",
    );
    assert_ne!(changed, pixel_source);
    h.backend.reload_shaders(vertex, &changed).unwrap();
    assert_ne!(h.render(&draw), expected);
    h.backend.reload_shaders(vertex, pixel_source).unwrap();
    assert_eq!(h.render(&draw), expected);
}
#[test]
fn multisample_resolve_preserves_stencil_and_creates_fractional_geometry_edges() {
    let mut h = Harness::new();
    assert!(RenderTexture::new_multisampled(
        &mut h.backend,
        TextureId(10),
        [64; 2],
        ColorMode::Legacy,
        3
    )
    .is_err());
    let vertices = [[5.2, 5.1], [59.4, 17.3], [12.6, 58.7]].map(|pos| DrawVert {
        pos,
        uv: [0.; 2],
        col: [255; 4],
    });
    let indices = [0, 1, 2];
    let commands = [DrawCmd {
        elem_count: 3,
        clip_rect: [0., 0., 64., 64.],
        texture: TextureId::WHITE,
        effects: 0,
    }];
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &[],
    };
    let (_, sample_indices, sample_commands) = full_quad(TextureId(10));
    let sample_vertices = [[0., 0.], [64., 0.], [64., 64.], [0., 64.]].map(|pos| DrawVert {
        pos,
        uv: [pos[0] / 64., pos[1] / 64.],
        col: [255; 4],
    });
    let sample = DrawData {
        vertices: &sample_vertices,
        indices: &sample_indices,
        commands: &sample_commands,
        effects: &[],
    };
    let mut tested = false;
    for count in [2, 4, 8] {
        let Ok(target) = RenderTexture::new_multisampled(
            &mut h.backend,
            TextureId(10),
            [64; 2],
            ColorMode::Legacy,
            count,
        ) else {
            continue;
        };
        tested = true;
        target
            .render(&mut h.backend, &draw, [0., 0., 0., 1.])
            .unwrap();
        let pixels = h.render(&sample);
        assert!(
            pixels.chunks_exact(4).any(|p| p[0] > 0 && p[0] < 255),
            "MSAA resolve has no fractional samples"
        );
        let quad = [[0., 0.], [64., 0.], [64., 64.], [0., 64.]].map(|pos| DrawVert {
            pos,
            uv: [0.; 2],
            col: [255; 4],
        });
        let stencil_indices = [0, 1, 2, 0, 2, 3, 0, 1, 2, 0, 2, 3, 0, 1, 2, 0, 2, 3];
        let stencil_commands = [1, 2, 3].map(|effects| DrawCmd {
            elem_count: 6,
            clip_rect: [0., 0., 64., 64.],
            texture: TextureId::WHITE,
            effects,
        });
        let shape = SdfShape {
            rect: [8., 8., 56., 56.],
            radii: [8.; 4],
            parameters: [1., 0., 0., 0.],
            ..Default::default()
        };
        let effects = [
            DrawEffects {
                sdf: shape,
                stencil: [0, 1],
                ..Default::default()
            },
            DrawEffects {
                stencil: [1, 0],
                ..Default::default()
            },
            DrawEffects {
                sdf: shape,
                stencil: [1, 2],
                ..Default::default()
            },
        ];
        target
            .render(
                &mut h.backend,
                &DrawData {
                    vertices: &quad,
                    indices: &stencil_indices,
                    commands: &stencil_commands,
                    effects: &effects,
                },
                [0., 0., 0., 1.],
            )
            .unwrap();
        let pixels = h.render(&sample);
        assert_eq!(pixel(&pixels, 32, 32), [255; 4]);
        assert_eq!(pixel(&pixels, 1, 1), [0, 0, 0, 255]);
    }
    assert!(
        tested,
        "device must support at least one multisample color/stencil pair"
    );
}

#[test]
fn shader_reload_preserves_fenced_pipeline_lifetimes_without_full_drain() {
    let mut h = Harness::new();
    let (vertices, indices, commands) = full_quad(TextureId::WHITE);
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &[],
    };
    h.backend.set_draw_optimization(DrawOptimization {
        instance_quads: true,
        ..Default::default()
    });
    h.backend
        .render(&draw, h.target.resource(), h.target.rtv(), [64.; 2])
        .unwrap();
    let idle = h.backend.full_idle_wait_count();
    let (vertex, pixel) = Backend::shader_sources();
    h.backend.reload_shaders(vertex, pixel).unwrap();
    assert_eq!(h.backend.full_idle_wait_count(), idle);
    assert!(h.backend.retirement_queue_len() > 0);
    h.render(&draw);
}
