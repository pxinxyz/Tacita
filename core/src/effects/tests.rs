use super::*;

#[test]
fn renderer_alpha_header_preserves_every_effect_offset() {
    let record = DrawEffects::default();
    let original = record.uniform_bytes();
    for flags in 0..8 {
        let packed = record.uniform_bytes_with_alpha(flags);
        assert_eq!(
            f32::from_le_bytes(packed[52..56].try_into().unwrap()),
            flags as f32
        );
        assert_eq!(&packed[..52], &original[..52]);
        assert_eq!(&packed[56..], &original[56..]);
    }
    assert_eq!(record.uniform_bytes_with_alpha(0), original);
    use crate::rendering::{alpha_flags, AlphaMode};
    assert_eq!(
        alpha_flags(
            AlphaMode::Premultiplied,
            AlphaMode::Premultiplied,
            AlphaMode::Premultiplied
        ),
        7
    );
}
use crate::{
    context::Context,
    draw::{DrawList, PaintRange},
    input::InputState,
    ScrollFlags,
};
use alloc::vec::Vec;
const FONT: &[u8] = include_bytes!("../../tests/assets/DejaVuSans.ttf");
const STOPS: [GradientStop; 2] = [
    GradientStop {
        position: 0.0,
        color: [255, 0, 0, 255],
    },
    GradientStop {
        position: 1.0,
        color: [0, 0, 255, 128],
    },
];

#[test]
fn surface_filter_layout_and_uniform_tail_are_explicit_and_identity_is_unchanged() {
    use core::mem::{offset_of, size_of};
    assert_eq!(size_of::<SurfaceStyle>(), 128);
    assert_eq!(size_of::<ImageFilter>(), 64);
    assert_eq!(
        [
            offset_of!(DrawEffects, surface),
            offset_of!(DrawEffects, filter),
            offset_of!(DrawEffects, stencil)
        ],
        [440, 568, 632]
    );
    let identity = DrawEffects::default();
    assert!(identity.identity());
    assert!(identity.uniform_bytes().iter().all(|b| *b == 0));
    let effect = DrawEffects {
        surface: SurfaceStyle {
            flags: SURFACE_GRID | SURFACE_GRAIN,
            phase: 1.25,
            rect: [1., 2., 3., 4.],
            ..Default::default()
        },
        filter: ImageFilter {
            kind: FilterKind::GaussianY,
            overwrite: 1,
            radius: 8.,
            texel: [0.01, 0.02],
            ..Default::default()
        },
        stencil: [3, 1],
        ..Default::default()
    };
    let bytes = effect.uniform_bytes();
    let at = |i: usize| f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
    assert_eq!(
        [
            at(14),
            at(15),
            at(208),
            at(212),
            at(213),
            at(240),
            at(241),
            at(242)
        ],
        [1., 3., 1., 258., 1.25, 2., 1., 8.]
    );
    assert_eq!([at(245), at(246)], [0.01, 0.02]);
    assert!(bytes[992..].iter().all(|b| *b == 0));
}

#[test]
fn filter_plans_bound_downsampling_preserve_odd_sizes_and_resolve_bloom_source() {
    let blur = Blur {
        method: BlurMethod::DualKawase,
        radius: 3.,
        levels: 6,
    };
    let plan = FilterPlan::bloom([65, 33], blur, 0.7, 1.5).unwrap();
    assert_eq!(plan.input_size(), [65, 33]);
    assert!(plan.steps().len() <= MAX_FILTER_PASSES);
    assert_eq!(plan.steps()[0].filter.kind, FilterKind::Threshold);
    assert_eq!(plan.steps()[1].size, [33, 17]);
    assert_eq!(plan.steps().last().unwrap().size, [65, 33]);
    for (i, step) in plan.steps().iter().enumerate() {
        assert_eq!(step.source, i);
        assert!(step.filter.is_valid());
        assert_eq!(step.filter.overwrite, 1);
        assert!(step.size.iter().all(|v| *v > 0));
    }
    let ids: Vec<_> = (100..115).map(crate::draw::TextureId).collect();
    let last = plan
        .packet(plan.steps().len() - 1, crate::draw::TextureId(42), &ids)
        .unwrap();
    assert_eq!(last.draw_data().effects[0].filter.secondary, 42);
    assert_eq!(
        last.draw_data().effects[0].mask_texture(3),
        Some(crate::draw::TextureId(42))
    );
    assert!(plan.packet(99, crate::draw::TextureId(42), &ids).is_err());
    assert_eq!(FilterPlan::blur([1, 1], blur).unwrap().steps().len(), 1);
    for size in [[0, 1], [1, 0], [16385, 1]] {
        assert!(FilterPlan::blur(size, blur).is_err());
    }
    for radius in [f32::NAN, -1., 65.] {
        assert!(FilterPlan::blur([64, 32], Blur { radius, ..blur }).is_err());
    }
    assert!(FilterPlan::bloom([64, 32], blur, 1.1, 1.).is_err());
    assert!(
        FilterPacket::new([0, 64], crate::draw::TextureId(42), ImageFilter::default()).is_err()
    );
    let gaussian = FilterPlan::blur(
        [65, 33],
        Blur {
            method: BlurMethod::Separable,
            ..blur
        },
    )
    .unwrap();
    assert_eq!(
        gaussian
            .steps()
            .iter()
            .map(|s| s.filter.kind)
            .collect::<Vec<_>>(),
        [FilterKind::GaussianX, FilterKind::GaussianY]
    );
}

#[test]
fn invalid_surfaces_reject_before_emission_and_stencil_restores_nested_effects() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.new_frame(0.016);
    ctx.ui().window("Surface", ScrollFlags::NONE, |ui| {
        let before = crate::widgets::draw_parts(ui.ctx).1.paint_mark();
        assert_eq!(
            ui.draw_surface(
                shape(),
                None,
                [0., 0., 1., 1.],
                [255; 4],
                SurfaceStyle {
                    flags: 1 << 30,
                    ..Default::default()
                }
            ),
            Err(EffectError::InvalidInput)
        );
        let after = crate::widgets::draw_parts(ui.ctx).1.paint_mark();
        assert_eq!(before, after);
        let fade = ui.begin_gradient_mask(fade()).unwrap();
        let outer = ui.begin_stencil_clip("Outer", shape()).unwrap();
        let inner = ui.begin_stencil_clip("Inner", shape()).unwrap();
        ui.draw_surface(
            shape(),
            None,
            [0., 0., 1., 1.],
            [255; 4],
            SurfaceStyle {
                flags: SURFACE_NOISE,
                ..Default::default()
            },
        )
        .unwrap();
        let effect = crate::widgets::draw_parts(ui.ctx).1.effects_state();
        assert_eq!(effect.stencil, [2, 0]);
        assert_eq!(effect.masks.count, 1);
        ui.end_stencil_clip(inner);
        ui.end_stencil_clip(outer);
        ui.end_mask(fade);
        assert!(crate::widgets::draw_parts(ui.ctx)
            .1
            .effects_state()
            .identity());
    });
    let draw = ctx.render();
    assert!(draw.effects.iter().any(|e| e.stencil == [0, 1]));
    assert!(draw.effects.iter().any(|e| e.stencil == [2, 2]));
    assert!(draw
        .effects
        .iter()
        .filter(|e| e.stencil[1] != 0)
        .all(|e| e.masks.count == 0 && e.sdf.light[3] == 0.));
}

#[test]
fn stencil_crossed_mask_and_clip_end_fails_before_mutation() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.new_frame(0.016);
    ctx.ui().window("Crossed", ScrollFlags::NONE, |ui| {
        let stencil = ui.begin_stencil_clip("S", shape()).unwrap();
        let mask = ui.begin_gradient_mask(fade()).unwrap();
        let before = crate::widgets::draw_parts(ui.ctx).1.paint_mark();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ui.end_stencil_clip(stencil)
        }));
        assert!(result.is_err());
        assert_eq!(before, crate::widgets::draw_parts(ui.ctx).1.paint_mark());
        ui.end_mask(mask);
    });
}
fn shape() -> RoundedRect {
    RoundedRect::new([100.0, 100.0, 200.0, 180.0], [16.0; 4]).unwrap()
}
fn fade() -> GradientMask {
    GradientMask {
        start: [100.0, 100.0],
        end: [200.0, 100.0],
        opacity: [1.0, 0.0],
    }
}

#[test]
fn rejected_paints_and_masks_do_not_mutate_geometry_or_state() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.new_frame(0.016);
    ctx.ui().window("W", ScrollFlags::NONE, |ui| {
        let mark = crate::widgets::draw_parts(ui.ctx).1.paint_mark();
        let state = crate::widgets::draw_parts(ui.ctx).1.effects_state();
        let bad = GradientPaint {
            kind: GradientKind::Radial,
            vector: [0.0, 1.0],
            ..Default::default()
        };
        assert_eq!(
            ui.draw_shape_gradient(shape(), &STOPS, bad),
            Err(EffectError::InvalidInput)
        );
        let duplicate = [STOPS[0], STOPS[0], STOPS[1]];
        assert_eq!(
            ui.draw_shape_gradient(shape(), &duplicate, GradientPaint::default()),
            Err(EffectError::InvalidInput)
        );
        assert_eq!(
            ui.draw_shape_gradient_with(
                shape(),
                17,
                |_| panic!("must validate count first"),
                GradientPaint::default()
            ),
            Err(EffectError::InvalidInput)
        );
        assert_eq!(
            ui.draw_gradient_border(
                shape(),
                &STOPS,
                GradientPaint::default(),
                f32::NAN,
                BorderPlacement::Inner
            ),
            Err(EffectError::InvalidInput)
        );
        assert!(ui
            .begin_gradient_mask(GradientMask {
                end: fade().start,
                ..fade()
            })
            .is_err());
        assert!(ui
            .begin_alpha_mask(AlphaMask {
                texture: TextureId(2),
                rect: shape().rect,
                uv: [0.0, 0.0, 1.0, f32::NAN]
            })
            .is_err());
        let draw = crate::widgets::draw_parts(ui.ctx).1;
        assert_eq!(draw.paint_mark(), mark);
        assert_eq!(draw.effects_state(), state);
        assert!(draw.as_draw_data().effects.is_empty());
    });
    ctx.render();
}

#[test]
fn four_masks_restore_and_fifth_rejection_opens_no_scope() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.new_frame(0.016);
    ctx.ui().window("W", ScrollFlags::NONE, |ui| {
        let a = ui.begin_gradient_mask(fade()).unwrap();
        let b = ui
            .begin_alpha_mask(AlphaMask {
                texture: TextureId(42),
                rect: shape().rect,
                uv: [1.0, 0.0, 0.0, 1.0],
            })
            .unwrap();
        let c = ui.begin_gradient_mask(fade()).unwrap();
        let d = ui.begin_gradient_mask(fade()).unwrap();
        assert!(matches!(
            ui.begin_gradient_mask(fade()),
            Err(EffectError::ScopeLimit)
        ));
        ui.draw_shape_gradient(
            shape(),
            &STOPS,
            GradientPaint {
                phase: 0.25,
                ..Default::default()
            },
        )
        .unwrap();
        ui.end_mask(d);
        ui.end_mask(c);
        ui.end_mask(b);
        ui.draw_rect_filled([100.0, 100.0], [120.0, 120.0], [255; 4]);
        ui.end_mask(a);
        ui.draw_rect_filled([120.0, 100.0], [140.0, 120.0], [255; 4]);
    });
    let data = ctx.render();
    assert!(data
        .effects
        .iter()
        .any(|e| e.masks.count == 4 && e.gradient.phase == 0.25));
    assert!(data
        .effects
        .iter()
        .any(|e| e.masks.count == 1 && e.gradient.kind == 0));
    assert_eq!(data.commands.last().unwrap().effects, 0);
    let first = data.effects.iter().find(|e| e.masks.count == 4).unwrap();
    assert_eq!(first.mask_texture(1), Some(TextureId(42)));
    assert_eq!(first.mask_texture(0), None);
}

#[test]
fn frame_assembly_remaps_local_effect_references() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.new_frame(0.016);
    for (name, phase) in [("A", 0.125), ("B", 0.75)] {
        ctx.ui().window(name, ScrollFlags::NONE, |ui| {
            ui.draw_shape_gradient(
                shape(),
                &STOPS,
                GradientPaint {
                    phase,
                    ..Default::default()
                },
            )
            .unwrap();
        });
    }
    let data = ctx.render();
    assert_eq!(data.effects.len(), 2);
    let phases: alloc::vec::Vec<_> = data
        .commands
        .iter()
        .filter(|c| c.effects > 0)
        .map(|c| data.effects[c.effects as usize - 1].gradient.phase)
        .collect();
    assert_eq!(phases, alloc::vec![0.125, 0.75]);
    assert_eq!(
        data.commands
            .iter()
            .map(|c| c.elem_count as usize)
            .sum::<usize>(),
        data.indices.len()
    );
}

#[test]
fn clipping_layer_order_and_faded_snapshots_preserve_records() {
    let mut draw = DrawList::new();
    let mut a = DrawEffects::default();
    a.sdf.rect = shape().rect;
    a.sdf.radii = shape().radii;
    a.sdf.parameters = [1.0, 1.0, 0.0, 8.0];
    a.sdf.light = [-1.0, 0.0, 2.0, 1.0];
    a.masks.count = 1;
    a.masks.ops[0] = MaskOp {
        kind: 2,
        rect: [0.0, 0.0, 100.0, 0.0],
        params: [1.0, 0.0, 0.0, 0.0],
        ..Default::default()
    };
    let mut b = a;
    b.masks.ops[0].params[1] = 0.5;
    let start = draw.paint_mark();
    draw.set_effects(a);
    draw.add_filled_rect([90.0, 90.0], [210.0, 190.0], [255; 4]);
    let middle = draw.paint_mark();
    draw.set_effects(b);
    draw.add_filled_rect([90.0, 90.0], [210.0, 190.0], [255; 4]);
    let end = draw.paint_mark();
    draw.order_paint(
        &[
            PaintRange {
                start,
                end: middle,
                layer: 1,
            },
            PaintRange {
                start: middle,
                end,
                layer: 0,
            },
        ],
        start,
        end,
    );
    assert_eq!(draw.as_draw_data().commands[0].effects, 2);
    draw.clip_rounded_tail(start, shape(), 1.0);
    let mut snapshot = DrawList::new();
    snapshot.copy_tail(&draw, 0, 0);
    let mut replay = DrawList::new();
    replay.set_effects(a);
    replay.add_filled_rect([0.0; 2], [5.0; 2], [255; 4]);
    replay.append_faded(&snapshot, 0.5);
    let data = replay.as_draw_data();
    assert_eq!(data.effects.len(), 3);
    assert_eq!(data.commands[1].effects, 3);
    assert_eq!(data.effects[data.commands[1].effects as usize - 1], b);
    assert!(data.vertices[4..].iter().all(|v| v.col[3] <= 128));
    assert_eq!(
        data.commands
            .iter()
            .map(|c| c.elem_count as usize)
            .sum::<usize>(),
        data.indices.len()
    );
}

#[test]
fn identity_is_sparse_and_repeated_effects_batch() {
    let mut draw = DrawList::new();
    draw.add_filled_rect([0.0; 2], [4.0; 2], [255; 4]);
    assert!(draw.as_draw_data().effects.is_empty());
    let mut effect = DrawEffects::default();
    effect.masks.count = 1;
    effect.masks.ops[0] = MaskOp {
        kind: 2,
        ..Default::default()
    };
    draw.set_effects(effect);
    draw.add_filled_rect([10.0; 2], [14.0; 2], [255; 4]);
    draw.set_effects(effect);
    draw.add_filled_rect([20.0; 2], [24.0; 2], [255; 4]);
    assert_eq!(draw.as_draw_data().effects.len(), 1);
    assert_eq!(draw.as_draw_data().commands.len(), 2);
    assert_eq!(draw.as_draw_data().commands[1].elem_count, 12);
    draw.clear();
    draw.add_filled_rect([0.0; 2], [4.0; 2], [255; 4]);
    assert!(draw.as_draw_data().effects.is_empty());
    assert_eq!(draw.as_draw_data().commands[0].effects, 0);
}

#[test]
fn focus_outline_retains_visual_mask_without_changing_focus() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.new_frame(0.016);
    ctx.ui().window("W", ScrollFlags::NONE, |ui| {
        ui.button("Masked focus");
    });
    ctx.render();
    ctx.set_input(InputState {
        mouse_pos: [78.0, 99.0],
        mouse_down: [true, false, false, false, false],
        viewport_size: [800.0, 600.0],
        ..Default::default()
    });
    ctx.new_frame(0.016);
    ctx.ui().window("W", ScrollFlags::NONE, |ui| {
        ui.gradient_mask(
            GradientMask {
                opacity: [0.0, 0.0],
                ..fade()
            },
            |ui| {
                ui.button("Masked focus");
            },
        )
        .unwrap();
    });
    assert!(ctx.focused_widget_id().is_some());
    let data = ctx.render();
    let last = data.commands.last().unwrap();
    assert!(last.effects > 0);
    assert_eq!(data.effects[last.effects as usize - 1].masks.count, 1);
}

#[test]
fn shader_uniform_pins_offsets_and_normalized_colors() {
    let mut effect = DrawEffects::default();
    effect.gradient.rect = [1.0, 2.0, 3.0, 4.0];
    effect.gradient.kind = 3;
    effect.gradient.phase = 0.25;
    effect.gradient.stop_count = 2;
    effect.gradient.stops[..2].copy_from_slice(&STOPS);
    effect.sdf.rect = [11.0, 12.0, 13.0, 14.0];
    effect.sdf.parameters = [2.0, 1.0, 0.0, 0.0];
    effect.masks.count = 1;
    effect.masks.ops[0] = MaskOp {
        kind: 2,
        rect: [10.0, 20.0, 30.0, 40.0],
        params: [1.0, 0.5, 0.0, 0.0],
        ..Default::default()
    };
    let bytes = effect.uniform_bytes();
    let at = |float: usize| f32::from_le_bytes(bytes[float * 4..float * 4 + 4].try_into().unwrap());
    assert_eq!(bytes.len(), 1024);
    assert_eq!(at(0), 1.0);
    assert_eq!(at(8), 3.0);
    assert_eq!(at(10), 0.25);
    assert_eq!(at(12), 1.0);
    assert_eq!(at(20), 1.0);
    assert_eq!(at(24), 1.0);
    assert_eq!(at(31), 128.0 / 255.0);
    assert_eq!(at(144), 10.0);
    assert_eq!(at(149), 0.5);
    assert_eq!(at(152), 2.0);
    assert_eq!(at(192), 11.0);
    assert_eq!(at(200), 2.0);
    assert!(bytes[832..].iter().all(|b| *b == 0));
    assert_eq!(core::mem::offset_of!(DrawEffects, sdf), 376);
    assert_eq!(core::mem::size_of::<DrawEffects>(), 640);
    assert_eq!(core::mem::offset_of!(DrawEffects, masks), 176);
}

#[test]
fn sdf_draws_one_quad_preserves_nested_masks_and_restores_plain_draws() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.new_frame(0.016);
    ctx.ui().window("SDF", ScrollFlags::NONE, |ui| {
        let mask = ui
            .begin_reveal_mask(RevealMask {
                rect: shape().rect,
                direction: RevealDirection::Radial,
                progress: 0.5,
                softness: 0.1,
                inverted: false,
            })
            .unwrap();
        let before = crate::widgets::draw_parts(ui.ctx)
            .1
            .as_draw_data()
            .vertices
            .len();
        ui.draw_sdf_shape_gradient(
            shape(),
            &STOPS,
            GradientPaint::default(),
            SdfStyle {
                kind: SdfKind::Border,
                width: 1.0,
                placement: BorderPlacement::Outer,
                ..Default::default()
            },
        )
        .unwrap();
        let draw = crate::widgets::draw_parts(ui.ctx).1;
        let data = draw.as_draw_data();
        assert_eq!(data.vertices.len() - before, 4);
        assert_eq!(data.commands.last().unwrap().elem_count, 6);
        let record = data.effects.last().unwrap();
        assert_eq!(record.sdf.rect, shape().rect);
        assert_eq!(record.sdf.radii, shape().radii);
        assert_eq!(record.sdf.parameters[0], 2.0);
        assert_eq!(record.masks.ops[0].kind, 7);
        assert_eq!(record.gradient.stop_count, 2);
        assert!(data.vertices[before..]
            .iter()
            .all(|v| v.pos[0] >= 97.0 && v.pos[0] <= 203.0));
        assert_eq!(draw.effects_state().sdf, SdfShape::default());
        ui.end_mask(mask);
        ui.draw_rect_filled([100.0; 2], [110.0; 2], [255; 4]);
        assert_eq!(
            crate::widgets::draw_parts(ui.ctx)
                .1
                .as_draw_data()
                .commands
                .last()
                .unwrap()
                .effects,
            0
        );
    });
    ctx.render();
}

#[test]
fn invalid_sdf_and_reveal_descriptors_and_zero_extent_effects_emit_nothing() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.new_frame(0.016);
    ctx.ui().window("SDF", ScrollFlags::NONE, |ui| {
        let mark = crate::widgets::draw_parts(ui.ctx).1.paint_mark();
        for style in [
            SdfStyle {
                width: f32::NAN,
                ..Default::default()
            },
            SdfStyle {
                reach: -1.0,
                ..Default::default()
            },
            SdfStyle {
                light_direction: [0.0; 2],
                ..Default::default()
            },
            SdfStyle {
                light_power: f32::INFINITY,
                ..Default::default()
            },
        ] {
            assert_eq!(
                ui.draw_sdf_shape(shape(), [255; 4], style),
                Err(EffectError::InvalidInput)
            );
        }
        assert!(ui
            .draw_sdf_shape_gradient_with(
                shape(),
                2,
                |_| panic!("invalid style checked first"),
                GradientPaint::default(),
                SdfStyle {
                    width: f32::NAN,
                    ..Default::default()
                }
            )
            .is_err());
        for kind in [
            SdfKind::Border,
            SdfKind::Highlight,
            SdfKind::Glow,
            SdfKind::EdgeGlow,
        ] {
            ui.draw_sdf_shape(
                shape(),
                [255; 4],
                SdfStyle {
                    kind,
                    width: 0.0,
                    reach: 0.0,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        for progress in [-1.0, 1.1, f32::NAN] {
            assert!(ui
                .begin_reveal_mask(RevealMask {
                    rect: shape().rect,
                    direction: RevealDirection::LeftToRight,
                    progress,
                    softness: 0.1,
                    inverted: false
                })
                .is_err());
        }
        assert!(ui
            .begin_reveal_mask(RevealMask {
                rect: [0.0; 4],
                direction: RevealDirection::Radial,
                progress: 0.5,
                softness: 0.1,
                inverted: false
            })
            .is_err());
        let draw = crate::widgets::draw_parts(ui.ctx).1;
        assert_eq!(draw.paint_mark(), mark);
        assert_eq!(draw.effects_state(), DrawEffects::default());
    });
    ctx.render();
}

#[test]
fn reveals_share_the_existing_four_mask_scope_limit() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.new_frame(0.016);
    ctx.ui().window("SDF", ScrollFlags::NONE, |ui| {
        let reveal = RevealMask {
            rect: shape().rect,
            direction: RevealDirection::TopToBottom,
            progress: 1.0,
            softness: 0.0,
            inverted: true,
        };
        let a = ui.begin_gradient_mask(fade()).unwrap();
        let b = ui.begin_reveal_mask(reveal).unwrap();
        let c = ui.begin_reveal_mask(reveal).unwrap();
        let d = ui.begin_reveal_mask(reveal).unwrap();
        assert!(matches!(
            ui.begin_reveal_mask(reveal),
            Err(EffectError::ScopeLimit)
        ));
        ui.draw_sdf_shape(shape(), [255; 4], SdfStyle::default())
            .unwrap();
        ui.end_mask(d);
        ui.end_mask(c);
        ui.end_mask(b);
        ui.end_mask(a);
    });
    let data = ctx.render();
    assert_eq!(data.effects.len(), 1);
    assert_eq!(data.effects[0].masks.count, 4);
    assert_eq!(data.effects[0].masks.ops[1].params, [1.0, 0.0, 1.0, 0.0]);
    assert_eq!(data.effects[0].mask_texture(1), None);
}

#[test]
fn popup_paint_escapes_parent_masks_and_can_have_its_own_mask() {
    let mut ctx = Context::new(FONT).unwrap();
    ctx.set_input(InputState {
        viewport_size: [800.0, 600.0],
        mouse_pos: [-1000.0; 2],
        ..Default::default()
    });
    ctx.new_frame(0.016);
    ctx.ui().window("W", ScrollFlags::NONE, |ui| {
        let scope = ui.begin_gradient_mask(fade()).unwrap();
        let id = ui.widget_id("Popup");
        ui.open_popup(id);
        ui.popup(id, [400.0, 100.0, 700.0, 300.0], |ui| {
            let sample = RoundedRect::new([420.0, 120.0, 660.0, 220.0], [12.0; 4]).unwrap();
            ui.draw_shape_gradient(
                sample,
                &STOPS,
                GradientPaint {
                    phase: 0.25,
                    ..Default::default()
                },
            )
            .unwrap();
            ui.gradient_mask(fade(), |ui| {
                ui.draw_shape_gradient(
                    sample,
                    &STOPS,
                    GradientPaint {
                        phase: 0.75,
                        ..Default::default()
                    },
                )
                .unwrap();
            })
            .unwrap();
        });
        ui.end_mask(scope);
    });
    let data = ctx.render();
    let unmasked = data
        .effects
        .iter()
        .find(|e| e.gradient.phase == 0.25)
        .unwrap();
    let masked = data
        .effects
        .iter()
        .find(|e| e.gradient.phase == 0.75)
        .unwrap();
    assert_eq!(unmasked.masks.count, 0);
    assert_eq!(masked.masks.count, 1);
}
