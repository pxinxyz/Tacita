use crate::{gui_export, set_last_error, slice_arg, GuiCmd, GuiContext, GuiErrorCode};
use tacita::{
    draw::{DrawCmd, DrawData, DrawVert, TextureId},
    effects::DrawEffects,
    rendering::{CommandPlan, DrawOptimization, GeometryRange, QuadInstance},
};

pub const GUI_RENDER_SORT_DISJOINT: u32 = 1;
pub const GUI_RENDER_INSTANCE_QUADS: u32 = 2;
pub const GUI_GEOMETRY_TRIANGLES: u32 = 0;
pub const GUI_GEOMETRY_QUADS: u32 = 1;
pub const GUI_ALPHA_INPUT_PREMULTIPLIED: u32 = 1;
pub const GUI_ALPHA_OUTPUT_PREMULTIPLIED: u32 = 2;
pub const GUI_ALPHA_SECONDARY_PREMULTIPLIED: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiPlannedDraw {
    pub command: GuiCmd,
    pub kind: u32,
    pub first: u32,
    pub count: u32,
}
pub type GuiQuadInstance = QuadInstance;
pub struct GuiRenderPlan {
    plan: CommandPlan,
    commands: Vec<DrawCmd>,
    draws: Vec<GuiPlannedDraw>,
}

#[no_mangle]
pub extern "C" fn gui_render_plan_new() -> *mut GuiRenderPlan {
    gui_export!(
        || Box::into_raw(Box::new(GuiRenderPlan {
            plan: CommandPlan::default(),
            commands: Vec::new(),
            draws: Vec::new()
        })),
        std::ptr::null_mut()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_render_plan_free(plan: *mut GuiRenderPlan) {
    gui_export!(
        || {
            if !plan.is_null() {
                if !plan.is_aligned() {
                    set_last_error(GuiErrorCode::InvalidInput);
                    return;
                }
                drop(unsafe { Box::from_raw(plan) });
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_render_plan_prepare(
    plan: *mut GuiRenderPlan,
    ctx: *const GuiContext,
    flags: u32,
    scale: f32,
) -> bool {
    gui_export!(
        || {
            if plan.is_null()
                || ctx.is_null()
                || !plan.is_aligned()
                || !ctx.is_aligned()
                || flags & !3 != 0
            {
                set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let plan = unsafe { &mut *plan };
            let ctx = unsafe { &*ctx };
            if ctx.frame_active {
                set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            let draw = ctx.last_draw;
            let Some(vertices) = (unsafe {
                slice_arg(draw.vertices.cast::<DrawVert>(), draw.vertex_count as usize)
            }) else {
                set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(indices) = (unsafe { slice_arg(draw.indices, draw.index_count as usize) })
            else {
                set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(effects) = (unsafe {
                slice_arg(
                    ctx.last_effects.cast::<DrawEffects>(),
                    ctx.last_effect_count,
                )
            }) else {
                set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            plan.commands.clear();
            plan.commands
                .extend(ctx.commands_staging.iter().map(|cmd| DrawCmd {
                    elem_count: cmd.elem_count,
                    clip_rect: cmd.clip_rect,
                    effects: cmd.effects,
                    texture: TextureId(cmd.texture),
                }));
            if plan
                .plan
                .prepare(
                    &DrawData {
                        vertices,
                        indices,
                        commands: &plan.commands,
                        effects,
                    },
                    DrawOptimization {
                        sort_disjoint: flags & 1 != 0,
                        instance_quads: flags & 2 != 0,
                    },
                    scale,
                )
                .is_err()
            {
                set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            plan.draws.clear();
            plan.draws.extend(plan.plan.draws().iter().map(|draw| {
                let (kind, first, count) = match draw.geometry {
                    GeometryRange::Triangles {
                        first_index,
                        index_count,
                    } => (GUI_GEOMETRY_TRIANGLES, first_index, index_count),
                    GeometryRange::Quads {
                        first_instance,
                        instance_count,
                    } => (GUI_GEOMETRY_QUADS, first_instance, instance_count),
                };
                let cmd = &draw.command;
                GuiPlannedDraw {
                    command: GuiCmd {
                        elem_count: cmd.elem_count,
                        clip_rect: cmd.clip_rect,
                        effects: cmd.effects,
                        texture: cmd.texture.0,
                    },
                    kind,
                    first,
                    count,
                }
            }));
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_render_plan_draws(
    plan: *const GuiRenderPlan,
    count: *mut usize,
) -> *const GuiPlannedDraw {
    gui_export!(
        || {
            if !count.is_null() {
                if !count.is_aligned() {
                    set_last_error(GuiErrorCode::InvalidInput);
                    return std::ptr::null();
                }
                unsafe { *count = 0 };
            }
            if plan.is_null() || !plan.is_aligned() {
                set_last_error(GuiErrorCode::InvalidInput);
                return std::ptr::null();
            }
            let plan = unsafe { &*plan };
            if !count.is_null() {
                unsafe { *count = plan.draws.len() };
            }
            if plan.draws.is_empty() {
                std::ptr::null()
            } else {
                plan.draws.as_ptr()
            }
        },
        std::ptr::null()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_render_plan_instances(
    plan: *const GuiRenderPlan,
    count: *mut usize,
) -> *const GuiQuadInstance {
    gui_export!(
        || {
            if !count.is_null() {
                if !count.is_aligned() {
                    set_last_error(GuiErrorCode::InvalidInput);
                    return std::ptr::null();
                }
                unsafe { *count = 0 };
            }
            if plan.is_null() || !plan.is_aligned() {
                set_last_error(GuiErrorCode::InvalidInput);
                return std::ptr::null();
            }
            let plan = unsafe { &*plan };
            if !count.is_null() {
                unsafe { *count = plan.plan.instances().len() };
            }
            if plan.plan.instances().is_empty() {
                std::ptr::null()
            } else {
                plan.plan.instances().as_ptr()
            }
        },
        std::ptr::null()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_render_plan_reordered(plan: *const GuiRenderPlan) -> u32 {
    gui_export!(
        || {
            if plan.is_null() || !plan.is_aligned() {
                set_last_error(GuiErrorCode::InvalidInput);
                0
            } else {
                unsafe { &*plan }.plan.reordered_commands()
            }
        },
        0
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_layout_and_invalid_handles() {
        assert_eq!(std::mem::size_of::<GuiPlannedDraw>(), 48);
        assert_eq!(std::mem::align_of::<GuiPlannedDraw>(), 8);
        assert_eq!(std::mem::offset_of!(GuiPlannedDraw, command), 0);
        assert_eq!(std::mem::offset_of!(GuiPlannedDraw, kind), 32);
        assert_eq!(std::mem::offset_of!(GuiPlannedDraw, first), 36);
        assert_eq!(std::mem::offset_of!(GuiPlannedDraw, count), 40);
        assert_eq!(std::mem::size_of::<GuiQuadInstance>(), 80);
        assert_eq!(std::mem::align_of::<GuiQuadInstance>(), 4);
        assert_eq!(std::mem::offset_of!(GuiQuadInstance, vertices), 0);
        let handle = gui_render_plan_new();
        assert!(!handle.is_null());
        assert!(!unsafe { gui_render_plan_prepare(handle, std::ptr::null(), 3, 1.) });
        let mut count = 12;
        assert!(unsafe { gui_render_plan_instances(std::ptr::null(), &mut count) }.is_null());
        assert_eq!(count, 0);
        unsafe { gui_render_plan_free(handle) };
    }
    #[test]
    fn completed_frame_plans_keep_output_after_rejection_and_active_frame_is_blocked() {
        unsafe {
            let ctx = crate::test_support::new_context();
            let plan = gui_render_plan_new();
            crate::gui_new_frame(ctx, std::ptr::null(), 0.016);
            assert!(!gui_render_plan_prepare(plan, ctx, 3, 1.));
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            crate::gui_begin_window(ctx, b"plan".as_ptr(), 4, 0);
            crate::gui_button(ctx, b"button".as_ptr(), 6);
            crate::gui_end_window(ctx);
            crate::gui_render(ctx);
            assert!(gui_render_plan_prepare(plan, ctx, 3, 1.5));
            let mut count = 0;
            let draws = gui_render_plan_draws(plan, &mut count);
            assert!(count > 0);
            let mut instances = 0;
            assert!(!gui_render_plan_instances(plan, &mut instances).is_null());
            assert!(instances > 0);
            assert!(!gui_render_plan_prepare(plan, ctx, 3, f32::NAN));
            let mut retained = 0;
            assert_eq!(gui_render_plan_draws(plan, &mut retained), draws);
            assert_eq!(retained, count);
            assert!(!gui_render_plan_prepare(plan, ctx, 4, 1.));
            gui_render_plan_free(plan);
            crate::gui_context_free(ctx);
        }
    }
    #[test]
    fn alpha_uniform_rejects_unknown_bits_before_writing() {
        let effect = DrawEffects::default();
        let pointer = (&effect as *const DrawEffects).cast::<crate::GuiDrawEffects>();
        let mut bytes = [0xa5; tacita::effects::EFFECT_UNIFORM_SIZE];
        assert!(!unsafe {
            crate::gui_effect_uniform_alpha(pointer, 8, bytes.as_mut_ptr(), bytes.len())
        });
        assert!(bytes.iter().all(|byte| *byte == 0xa5));
        assert!(unsafe {
            crate::gui_effect_uniform_alpha(pointer, 7, bytes.as_mut_ptr(), bytes.len())
        });
        assert_eq!(f32::from_le_bytes(bytes[52..56].try_into().unwrap()), 7.);
    }
}
