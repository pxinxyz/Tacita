use crate::{
    active_frame_ctx, gui_export, ContainerKind, ContainerScope, GuiColor, GuiContext,
    GuiErrorCode, GuiGradientStop, GuiRect, GuiRoundedRect, GuiVec2, WindowState,
};
use tacita::{
    effects::{
        AlphaMask, GradientKind, GradientMask, GradientPaint, GradientSpread, GradientStop,
        RevealDirection, RevealMask, SdfKind, SdfStyle,
    },
    shapes::{BorderPlacement, RoundedRect},
};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiDrawEffects {
    pub gradient: GuiGradientFill,
    pub masks: GuiMaskStack,
    pub sdf: GuiSdfShape,
    pub surface: GuiSurfaceStyle,
    pub filter: GuiImageFilter,
    pub stencil: [u32; 2],
}
impl GuiDrawEffects {
    fn uniform_bytes(self) -> Option<[u8; tacita::effects::EFFECT_UNIFORM_SIZE]> {
        if self.gradient.kind > 3
            || self.gradient.spread > 2
            || self.gradient.stop_count > 16
            || self.masks.count > 4
            || self.stencil[0] > 8
            || self.stencil[1] > 2
            || self.masks.ops[..self.masks.count as usize]
                .iter()
                .any(|m| m.kind > 7)
            || (self.filter.kind == GUI_FILTER_BLOOM_COMPOSE && self.masks.count == 4)
        {
            return None;
        }
        let surface = if self.surface.flags == 0 {
            tacita::effects::SurfaceStyle::default()
        } else {
            self.surface.decode()?
        };
        Some(
            tacita::effects::DrawEffects {
                gradient: self.gradient,
                masks: self.masks,
                sdf: self.sdf,
                surface,
                filter: self.filter.decode()?,
                stencil: self.stencil,
            }
            .uniform_bytes(),
        )
    }
}
pub type GuiGradientFill = tacita::effects::GradientFill;
pub type GuiMaskStack = tacita::effects::MaskStack;
pub type GuiMaskOp = tacita::effects::MaskOp;
pub const GUI_PAINT_LINEAR: u32 = 1;
pub const GUI_PAINT_RADIAL: u32 = 2;
pub const GUI_PAINT_CONIC: u32 = 3;
pub const GUI_MAX_PAINT_STOPS: u32 = 16;
pub const GUI_MAX_MASKS: u32 = 4;
pub const GUI_EFFECT_UNIFORM_SIZE: u32 = tacita::effects::EFFECT_UNIFORM_SIZE as u32;
pub type GuiSdfShape = tacita::effects::SdfShape;
pub const GUI_SDF_FILL: u32 = 1;
pub const GUI_SDF_BORDER: u32 = 2;
pub const GUI_SDF_GLOW: u32 = 3;
pub const GUI_SDF_EDGE_GLOW: u32 = 4;
pub const GUI_SDF_HIGHLIGHT: u32 = 5;
pub const GUI_REVEAL_LEFT_TO_RIGHT: u32 = 3;
pub const GUI_REVEAL_RIGHT_TO_LEFT: u32 = 4;
pub const GUI_REVEAL_TOP_TO_BOTTOM: u32 = 5;
pub const GUI_REVEAL_BOTTOM_TO_TOP: u32 = 6;
pub const GUI_REVEAL_RADIAL: u32 = 7;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiSdfStyle {
    pub kind: u32,
    pub width: f32,
    pub placement: u32,
    pub reach: f32,
    pub light_direction: GuiVec2,
    pub light_power: f32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiRevealMask {
    pub rect: GuiRect,
    pub direction: u32,
    pub progress: f32,
    pub softness: f32,
    pub inverted: u8,
    pub reserved: [u8; 3],
}
impl GuiSdfStyle {
    fn decode(self) -> Option<SdfStyle> {
        let kind = match self.kind {
            1 => SdfKind::Fill,
            2 => SdfKind::Border,
            3 => SdfKind::Glow,
            4 => SdfKind::EdgeGlow,
            5 => SdfKind::Highlight,
            _ => return None,
        };
        let placement = match self.placement {
            0 => BorderPlacement::Inner,
            1 => BorderPlacement::Center,
            2 => BorderPlacement::Outer,
            _ => return None,
        };
        let style = SdfStyle {
            kind,
            width: self.width,
            placement,
            reach: self.reach,
            light_direction: self.light_direction.into(),
            light_power: self.light_power,
        };
        style.is_valid().then_some(style)
    }
}
impl GuiRevealMask {
    fn decode(self) -> Option<RevealMask> {
        let direction = match self.direction {
            3 => RevealDirection::LeftToRight,
            4 => RevealDirection::RightToLeft,
            5 => RevealDirection::TopToBottom,
            6 => RevealDirection::BottomToTop,
            7 => RevealDirection::Radial,
            _ => return None,
        };
        let mask = RevealMask {
            rect: self.rect.into(),
            direction,
            progress: self.progress,
            softness: self.softness,
            inverted: self.inverted != 0,
        };
        mask.is_valid().then_some(mask)
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiGradientPaint {
    pub kind: u32,
    pub origin: GuiVec2,
    pub vector: GuiVec2,
    pub spread: u32,
    pub phase: f32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiAlphaMask {
    pub texture: u64,
    pub rect: GuiRect,
    pub uv: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiGradientMask {
    pub start: GuiVec2,
    pub end: GuiVec2,
    pub opacity: [f32; 2],
}
impl GuiGradientPaint {
    fn decode(self) -> Option<GradientPaint> {
        let kind = match self.kind {
            1 => GradientKind::Linear,
            2 => GradientKind::Radial,
            3 => GradientKind::Conic,
            _ => return None,
        };
        let spread = match self.spread {
            0 => GradientSpread::Clamp,
            1 => GradientSpread::Repeat,
            2 => GradientSpread::Mirror,
            _ => return None,
        };
        let paint = GradientPaint {
            kind,
            origin: self.origin.into(),
            vector: self.vector.into(),
            spread,
            phase: self.phase,
        };
        paint.is_valid().then_some(paint)
    }
}
unsafe fn canvas<'a>(ctx: *mut GuiContext) -> Option<&'a mut GuiContext> {
    let gui = unsafe { active_frame_ctx(ctx) }?;
    if !matches!(gui.window, WindowState::Open) {
        crate::set_last_error(GuiErrorCode::ContractViolation);
        return None;
    }
    Some(gui)
}
unsafe fn stops<'a>(data: *const GuiGradientStop, count: usize) -> Option<&'a [GuiGradientStop]> {
    if data.is_null() || !(2..=16).contains(&count) {
        crate::set_last_error(GuiErrorCode::InvalidInput);
        return None;
    }
    match unsafe { crate::slice_arg(data, count) } {
        Some(stops) => Some(stops),
        None => {
            crate::set_last_error(GuiErrorCode::InvalidInput);
            None
        }
    }
}
fn stop(value: GuiGradientStop) -> GradientStop {
    GradientStop {
        position: value.position,
        color: value.color.into(),
    }
}
fn shape(value: GuiRoundedRect) -> Option<RoundedRect> {
    RoundedRect::new(value.rect.into(), value.radii).ok()
}

#[no_mangle]
pub unsafe extern "C" fn gui_draw_sdf_shape(
    ctx: *mut GuiContext,
    rect: GuiRoundedRect,
    color: GuiColor,
    style: GuiSdfStyle,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            let (Some(rect), Some(style)) = (shape(rect), style.decode()) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            if gui
                .ctx
                .ui()
                .draw_sdf_shape(rect, color.into(), style)
                .is_err()
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_sdf_shape_gradient(
    ctx: *mut GuiContext,
    rect: GuiRoundedRect,
    data: *const GuiGradientStop,
    count: usize,
    paint: GuiGradientPaint,
    style: GuiSdfStyle,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            let (Some(rect), Some(paint), Some(style), Some(stops)) =
                (shape(rect), paint.decode(), style.decode(), unsafe {
                    stops(data, count)
                })
            else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            if gui
                .ctx
                .ui()
                .draw_sdf_shape_gradient_with(rect, count, |i| stop(stops[i]), paint, style)
                .is_err()
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_reveal_mask(ctx: *mut GuiContext, mask: GuiRevealMask) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(mask) = mask.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            match gui.ctx.ui().begin_reveal_mask(mask) {
                Ok(scope) => {
                    gui.containers.push(ContainerScope::Mask(scope));
                    true
                }
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_shape_gradient(
    ctx: *mut GuiContext,
    rect: GuiRoundedRect,
    data: *const GuiGradientStop,
    count: usize,
    paint: GuiGradientPaint,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            let (Some(rect), Some(paint), Some(stops)) =
                (shape(rect), paint.decode(), unsafe { stops(data, count) })
            else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            if gui
                .ctx
                .ui()
                .draw_shape_gradient_with(rect, count, |i| stop(stops[i]), paint)
                .is_err()
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_gradient_border(
    ctx: *mut GuiContext,
    rect: GuiRoundedRect,
    data: *const GuiGradientStop,
    count: usize,
    paint: GuiGradientPaint,
    width: f32,
    placement: u32,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            let placement = match placement {
                0 => BorderPlacement::Inner,
                1 => BorderPlacement::Center,
                2 => BorderPlacement::Outer,
                _ => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    return;
                }
            };
            let (Some(rect), Some(paint), Some(stops)) =
                (shape(rect), paint.decode(), unsafe { stops(data, count) })
            else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            if gui
                .ctx
                .ui()
                .draw_gradient_border_with(rect, count, |i| stop(stops[i]), paint, width, placement)
                .is_err()
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_alpha_mask(ctx: *mut GuiContext, mask: GuiAlphaMask) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            match gui.ctx.ui().begin_alpha_mask(AlphaMask {
                texture: tacita::draw::TextureId(mask.texture),
                rect: mask.rect.into(),
                uv: mask.uv,
            }) {
                Ok(scope) => {
                    gui.containers.push(ContainerScope::Mask(scope));
                    true
                }
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_gradient_mask(
    ctx: *mut GuiContext,
    mask: GuiGradientMask,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            match gui.ctx.ui().begin_gradient_mask(GradientMask {
                start: mask.start.into(),
                end: mask.end.into(),
                opacity: mask.opacity,
            }) {
                Ok(scope) => {
                    gui.containers.push(ContainerScope::Mask(scope));
                    true
                }
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_mask(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            if gui.containers.last().map(ContainerScope::kind) != Some(ContainerKind::Mask) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            if let Some(ContainerScope::Mask(scope)) = gui.containers.pop() {
                gui.ctx.ui().end_mask(scope);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_frame_effects(
    ctx: *mut GuiContext,
    count: *mut usize,
) -> *const GuiDrawEffects {
    gui_export!(
        || {
            if !count.is_null() {
                unsafe { *count = 0 };
            }
            let Some(gui) = (unsafe { ctx.as_ref() }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return std::ptr::null();
            };
            if gui.frame_active {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return std::ptr::null();
            }
            if !count.is_null() {
                unsafe { *count = gui.last_effect_count };
            }
            gui.last_effects
        },
        std::ptr::null()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_effect_uniform(
    effect: *const GuiDrawEffects,
    output: *mut u8,
    capacity: usize,
) -> bool {
    gui_export!(
        || {
            if effect.is_null()
                || !effect.is_aligned()
                || output.is_null()
                || capacity < GUI_EFFECT_UNIFORM_SIZE as usize
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(bytes) = (unsafe { *effect }).uniform_bytes() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
            }
            true
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_effect_uniform_alpha(
    effect: *const GuiDrawEffects,
    flags: u32,
    output: *mut u8,
    capacity: usize,
) -> bool {
    gui_export!(
        || {
            if effect.is_null()
                || !effect.is_aligned()
                || output.is_null()
                || capacity < GUI_EFFECT_UNIFORM_SIZE as usize
                || flags & !7 != 0
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let Some(mut bytes) = (unsafe { *effect }).uniform_bytes() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            bytes[52..56].copy_from_slice(&(flags as f32).to_le_bytes());
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len()) };
            true
        },
        false
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_before_mutation_preserves_crossed_scopes_and_exposes_table() {
        unsafe {
            let ctx = crate::test_support::new_context();
            crate::gui_new_frame(ctx, std::ptr::null(), 0.016);
            crate::gui_begin_window(ctx, b"W".as_ptr(), 1, 0);
            let shape = GuiRoundedRect {
                rect: [100.0, 100.0, 220.0, 180.0].into(),
                radii: [12.0; 4],
            };
            let stops = [
                GuiGradientStop {
                    position: 0.0,
                    color: [255; 4].into(),
                },
                GuiGradientStop {
                    position: 1.0,
                    color: [99, 91, 255, 255].into(),
                },
            ];
            let paint = GuiGradientPaint {
                kind: GUI_PAINT_CONIC,
                origin: [0.5; 2].into(),
                vector: [1.0, 0.0].into(),
                spread: 1,
                phase: 0.25,
            };
            let before = (&*ctx).ctx.inspect_windows().next().unwrap().indices;
            gui_draw_shape_gradient(
                ctx,
                shape,
                stops.as_ptr(),
                2,
                GuiGradientPaint { kind: 99, ..paint },
            );
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            gui_draw_shape_gradient(ctx, shape, std::ptr::null(), 2, paint);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            gui_draw_shape_gradient(
                ctx,
                shape,
                stops.as_ptr().cast::<u8>().add(1).cast(),
                2,
                paint,
            );
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            gui_draw_shape_gradient(ctx, shape, stops.as_ptr(), usize::MAX, paint);
            assert_eq!(
                (&*ctx).ctx.inspect_windows().next().unwrap().indices,
                before
            );
            assert!((&*ctx).containers.is_empty());
            let alpha = GuiAlphaMask {
                texture: 42,
                rect: shape.rect,
                uv: [0.0, 0.0, 1.0, 1.0],
            };
            assert!(gui_begin_alpha_mask(ctx, alpha));
            assert!(crate::gui_begin_rounded_clip(
                ctx,
                b"clip".as_ptr(),
                4,
                shape
            ));
            gui_end_mask(ctx);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert_eq!((&*ctx).containers.len(), 2);
            gui_draw_shape_gradient(ctx, shape, stops.as_ptr(), 2, paint);
            let mut count = 99;
            assert!(gui_frame_effects(ctx, &mut count).is_null());
            assert_eq!(count, 0);
            let draw = crate::gui_render(ctx);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert!((&*ctx).containers.is_empty());
            let effects = gui_frame_effects(ctx, &mut count);
            assert!(!effects.is_null() && count > 0);
            let records = std::slice::from_raw_parts(effects, count);
            assert!(records
                .iter()
                .any(|e| e.gradient.kind == 3 && e.masks.count == 1));
            let commands = std::slice::from_raw_parts(draw.commands, draw.command_count as usize);
            assert!(commands.iter().all(|c| c.effects as usize <= count));
            let mut bytes = [0u8; 1024];
            assert!(!gui_effect_uniform(effects, bytes.as_mut_ptr(), 1023));
            assert_eq!(bytes, [0; 1024]);
            assert!(gui_effect_uniform(effects, bytes.as_mut_ptr(), 1024));
            assert_eq!(Some(bytes), (*effects).uniform_bytes());
            assert_eq!(gui_frame_effects(ctx, &mut count), effects);
            crate::gui_new_frame(ctx, std::ptr::null(), 0.016);
            crate::gui_render(ctx);
            gui_frame_effects(ctx, &mut count);
            assert_eq!(count, 0);
            crate::gui_context_free(ctx);
        }
    }

    #[test]
    fn sdf_and_reveal_validate_wire_values_before_mutating_and_unwind_masks() {
        unsafe {
            let ctx = crate::test_support::new_context();
            crate::gui_new_frame(ctx, std::ptr::null(), 0.016);
            crate::gui_begin_window(ctx, b"SDF".as_ptr(), 3, 0);
            let rect = GuiRoundedRect {
                rect: [100.0, 100.0, 240.0, 180.0].into(),
                radii: [12.0; 4],
            };
            let sdf = GuiSdfStyle {
                kind: GUI_SDF_FILL,
                width: 1.0,
                placement: 0,
                reach: 8.0,
                light_direction: [-1.0, -1.0].into(),
                light_power: 2.0,
            };
            let before = (&*ctx).ctx.inspect_windows().next().unwrap().indices;
            for bad in [
                GuiSdfStyle { kind: 99, ..sdf },
                GuiSdfStyle {
                    placement: 99,
                    ..sdf
                },
                GuiSdfStyle {
                    width: f32::NAN,
                    ..sdf
                },
                GuiSdfStyle {
                    light_direction: [0.0; 2].into(),
                    ..sdf
                },
            ] {
                gui_draw_sdf_shape(ctx, rect, [255; 4].into(), bad);
                assert_eq!(
                    crate::gui_last_error_code(),
                    GuiErrorCode::InvalidInput as u8
                );
                assert_eq!(
                    (&*ctx).ctx.inspect_windows().next().unwrap().indices,
                    before
                );
            }
            let mask = GuiRevealMask {
                rect: rect.rect,
                direction: GUI_REVEAL_RADIAL,
                progress: 0.5,
                softness: 0.1,
                inverted: 255,
                reserved: [0; 3],
            };
            assert!(!gui_begin_reveal_mask(
                ctx,
                GuiRevealMask {
                    direction: 99,
                    ..mask
                }
            ));
            assert!(!gui_begin_reveal_mask(
                ctx,
                GuiRevealMask {
                    progress: f32::NAN,
                    ..mask
                }
            ));
            assert!((&*ctx).containers.is_empty());
            assert!(gui_begin_reveal_mask(ctx, mask));
            gui_draw_sdf_shape_gradient(
                ctx,
                rect,
                std::ptr::null(),
                2,
                GuiGradientPaint {
                    kind: GUI_PAINT_LINEAR,
                    origin: [0.0; 2].into(),
                    vector: [1.0, 0.0].into(),
                    spread: 0,
                    phase: 0.0,
                },
                sdf,
            );
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert_eq!(
                (&*ctx).ctx.inspect_windows().next().unwrap().indices,
                before
            );
            gui_draw_sdf_shape(ctx, rect, [255; 4].into(), sdf);
            crate::gui_render(ctx);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert!((&*ctx).containers.is_empty());
            let mut count = 0;
            let ptr = gui_frame_effects(ctx, &mut count);
            let records = std::slice::from_raw_parts(ptr, count);
            let effect = records.iter().find(|e| e.sdf.parameters[0] == 1.0).unwrap();
            assert_eq!(effect.masks.ops[0].kind, 7);
            assert_eq!(effect.masks.ops[0].params[2], 1.0);
            let mut buffer = [0u8; 1024];
            assert!(gui_effect_uniform(
                effect,
                buffer.as_mut_ptr(),
                buffer.len()
            ));
            assert_eq!(
                f32::from_le_bytes(buffer[800..804].try_into().unwrap()),
                1.0
            );
            crate::gui_context_free(ctx);
        }
    }

    #[test]
    fn fifth_mask_is_rejected_and_balanced_ends_restore_identity() {
        unsafe {
            let ctx = crate::test_support::new_context();
            crate::gui_new_frame(ctx, std::ptr::null(), 0.016);
            crate::gui_begin_window(ctx, b"W".as_ptr(), 1, 0);
            let mask = GuiGradientMask {
                start: [100.0; 2].into(),
                end: [200.0, 100.0].into(),
                opacity: [1.0, 0.0],
            };
            for _ in 0..4 {
                assert!(gui_begin_gradient_mask(ctx, mask));
            }
            assert!(!gui_begin_gradient_mask(ctx, mask));
            assert_eq!((&*ctx).containers.len(), 4);
            for _ in 0..4 {
                gui_end_mask(ctx);
            }
            crate::gui_button(ctx, b"Restored".as_ptr(), 8);
            crate::gui_end_window(ctx);
            let draw = crate::gui_render(ctx);
            let mut count = 99;
            gui_frame_effects(ctx, &mut count);
            assert_eq!(count, 0);
            assert!(
                std::slice::from_raw_parts(draw.commands, draw.command_count as usize)
                    .iter()
                    .all(|c| c.effects == 0)
            );
            crate::gui_context_free(ctx);
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiSurfaceStyle {
    pub flags: u32,
    pub phase: f32,
    pub pixel_scale: [f32; 2],
    pub noise: f32,
    pub grain: f32,
    pub dither: f32,
    pub dither_kind: u32,
    pub exposure: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub desaturation: f32,
    pub tint: [f32; 4],
    pub scanlines: f32,
    pub spacing: f32,
    pub grid_width: f32,
    pub dot_radius: f32,
    pub vignette: f32,
    pub shimmer: f32,
    pub sweep_width: f32,
    pub sweep_speed: f32,
    pub distortion: f32,
    pub frequency: f32,
    pub chromatic: f32,
    pub refraction: f32,
    pub rect: [f32; 4],
}
impl From<tacita::effects::SurfaceStyle> for GuiSurfaceStyle {
    fn from(v: tacita::effects::SurfaceStyle) -> Self {
        Self {
            flags: v.flags,
            phase: v.phase,
            pixel_scale: v.pixel_scale,
            noise: v.noise,
            grain: v.grain,
            dither: v.dither,
            dither_kind: v.dither_kind as u32,
            exposure: v.exposure,
            contrast: v.contrast,
            saturation: v.saturation,
            desaturation: v.desaturation,
            tint: v.tint,
            scanlines: v.scanlines,
            spacing: v.spacing,
            grid_width: v.grid_width,
            dot_radius: v.dot_radius,
            vignette: v.vignette,
            shimmer: v.shimmer,
            sweep_width: v.sweep_width,
            sweep_speed: v.sweep_speed,
            distortion: v.distortion,
            frequency: v.frequency,
            chromatic: v.chromatic,
            refraction: v.refraction,
            rect: v.rect,
        }
    }
}
impl GuiSurfaceStyle {
    fn decode(self) -> Option<tacita::effects::SurfaceStyle> {
        let dither_kind = match self.dither_kind {
            0 => tacita::effects::DitherKind::Bayer,
            1 => tacita::effects::DitherKind::BlueNoise,
            _ => return None,
        };
        let v = tacita::effects::SurfaceStyle {
            flags: self.flags,
            phase: self.phase,
            pixel_scale: self.pixel_scale,
            noise: self.noise,
            grain: self.grain,
            dither: self.dither,
            dither_kind,
            exposure: self.exposure,
            contrast: self.contrast,
            saturation: self.saturation,
            desaturation: self.desaturation,
            tint: self.tint,
            scanlines: self.scanlines,
            spacing: self.spacing,
            grid_width: self.grid_width,
            dot_radius: self.dot_radius,
            vignette: self.vignette,
            shimmer: self.shimmer,
            sweep_width: self.sweep_width,
            sweep_speed: self.sweep_speed,
            distortion: self.distortion,
            frequency: self.frequency,
            chromatic: self.chromatic,
            refraction: self.refraction,
            rect: self.rect,
        };
        v.is_valid().then_some(v)
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GuiImageFilter {
    pub kind: u32,
    pub overwrite: u32,
    pub radius: f32,
    pub threshold: f32,
    pub intensity: f32,
    pub texel: [f32; 2],
    pub reserved: f32,
    pub secondary: u64,
    pub params: [f32; 6],
}
impl From<tacita::effects::ImageFilter> for GuiImageFilter {
    fn from(v: tacita::effects::ImageFilter) -> Self {
        Self {
            kind: v.kind as u32,
            overwrite: v.overwrite,
            radius: v.radius,
            threshold: v.threshold,
            intensity: v.intensity,
            texel: v.texel,
            reserved: v.reserved,
            secondary: v.secondary,
            params: v.params,
        }
    }
}
impl GuiImageFilter {
    fn decode(self) -> Option<tacita::effects::ImageFilter> {
        use tacita::effects::FilterKind;
        let kind = match self.kind {
            0 => FilterKind::Copy,
            1 => FilterKind::GaussianX,
            2 => FilterKind::GaussianY,
            3 => FilterKind::KawaseDown,
            4 => FilterKind::KawaseUp,
            5 => FilterKind::Threshold,
            6 => FilterKind::BloomCompose,
            _ => return None,
        };
        let v = tacita::effects::ImageFilter {
            kind,
            overwrite: self.overwrite,
            radius: self.radius,
            threshold: self.threshold,
            intensity: self.intensity,
            texel: self.texel,
            reserved: self.reserved,
            secondary: self.secondary,
            params: self.params,
        };
        v.is_valid().then_some(v)
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiBlur {
    pub method: u32,
    pub radius: f32,
    pub levels: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GuiFilterStep {
    pub source: u32,
    pub size: [u32; 2],
    pub reserved: u32,
    pub filter: GuiImageFilter,
}

#[no_mangle]
pub unsafe extern "C" fn gui_surface_default() -> GuiSurfaceStyle {
    gui_export!(
        || tacita::effects::SurfaceStyle::default().into(),
        tacita::effects::SurfaceStyle::default().into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_surface(
    ctx: *mut GuiContext,
    rect: GuiRoundedRect,
    texture: u64,
    textured: u8,
    uv: GuiRect,
    color: GuiColor,
    style: GuiSurfaceStyle,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            let (Some(shape), Some(style)) = (shape(rect), style.decode()) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            if textured > 1 {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let texture = (textured != 0).then_some(tacita::draw::TextureId(texture));
            if gui
                .ctx
                .ui()
                .draw_surface(shape, texture, uv.into(), color.into(), style)
                .is_err()
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_filtered_image(
    ctx: *mut GuiContext,
    texture: u64,
    rect: GuiRect,
    uv: GuiRect,
    color: GuiColor,
    filter: GuiImageFilter,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            let Some(filter) = filter.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            if gui
                .ctx
                .ui()
                .draw_filtered_image(
                    tacita::draw::TextureId(texture),
                    rect.into(),
                    uv.into(),
                    color.into(),
                    filter,
                )
                .is_err()
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_stencil_clip(
    ctx: *mut GuiContext,
    label: crate::GuiStr,
    rect: GuiRoundedRect,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let (Some(label), Some(shape)) =
                (unsafe { crate::str_arg(label.ptr, label.len) }, shape(rect))
            else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            match gui.ctx.ui().begin_stencil_clip(label, shape) {
                Ok(token) => {
                    gui.containers.push(ContainerScope::Stencil(token));
                    true
                }
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_stencil_clip(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            if gui.containers.last().map(ContainerScope::kind) != Some(ContainerKind::Stencil) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            if let Some(ContainerScope::Stencil(token)) = gui.containers.pop() {
                gui.ctx.ui().end_stencil_clip(token);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_filter_plan(
    width: u32,
    height: u32,
    blur: GuiBlur,
    bloom: u8,
    threshold: f32,
    intensity: f32,
    output: *mut GuiFilterStep,
    capacity: usize,
) -> usize {
    gui_export!(
        || {
            use tacita::effects::{Blur, BlurMethod, FilterPlan};
            let method = match blur.method {
                0 => BlurMethod::Separable,
                1 => BlurMethod::DualKawase,
                _ => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    return 0;
                }
            };
            if bloom > 1 {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0;
            }
            let blur = Blur {
                method,
                radius: blur.radius,
                levels: blur.levels,
            };
            let plan = if bloom != 0 {
                FilterPlan::bloom([width, height], blur, threshold, intensity)
            } else {
                FilterPlan::blur([width, height], blur)
            };
            let Ok(plan) = plan else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0;
            };
            let count = plan.steps().len();
            if output.is_null() {
                return count;
            }
            if capacity < count
                || !(output as usize).is_multiple_of(core::mem::align_of::<GuiFilterStep>())
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0;
            }
            for (i, v) in plan.steps().iter().enumerate() {
                unsafe {
                    output.add(i).write(GuiFilterStep {
                        source: v.source as u32,
                        size: v.size,
                        reserved: 0,
                        filter: v.filter.into(),
                    });
                }
            }
            count
        },
        0
    )
}
pub const GUI_SURFACE_NOISE: u32 = 1;
pub const GUI_SURFACE_GRAIN: u32 = 2;
pub const GUI_SURFACE_DITHER: u32 = 4;
pub const GUI_SURFACE_GRADE: u32 = 8;
pub const GUI_SURFACE_DESATURATE: u32 = 16;
pub const GUI_SURFACE_VIGNETTE: u32 = 32;
pub const GUI_SURFACE_CHROMATIC: u32 = 64;
pub const GUI_SURFACE_SCANLINES: u32 = 128;
pub const GUI_SURFACE_GRID: u32 = 256;
pub const GUI_SURFACE_DOTS: u32 = 512;
pub const GUI_SURFACE_SHIMMER: u32 = 1024;
pub const GUI_SURFACE_SWEEP: u32 = 2048;
pub const GUI_SURFACE_DISTORT: u32 = 4096;
pub const GUI_SURFACE_GLASS: u32 = 8192;
pub const GUI_SURFACE_ALL: u32 = 16383;
pub const GUI_DITHER_BAYER: u32 = 0;
pub const GUI_DITHER_BLUE_NOISE: u32 = 1;
pub const GUI_BLUR_SEPARABLE: u32 = 0;
pub const GUI_BLUR_DUAL_KAWASE: u32 = 1;
pub const GUI_MAX_FILTER_PASSES: u32 = 15;
pub const GUI_FILTER_COPY: u32 = 0;
pub const GUI_FILTER_GAUSSIAN_X: u32 = 1;
pub const GUI_FILTER_GAUSSIAN_Y: u32 = 2;
pub const GUI_FILTER_KAWASE_DOWN: u32 = 3;
pub const GUI_FILTER_KAWASE_UP: u32 = 4;
pub const GUI_FILTER_THRESHOLD: u32 = 5;
pub const GUI_FILTER_BLOOM_COMPOSE: u32 = 6;

#[cfg(test)]
mod surface_tests {
    use super::*;
    #[test]
    fn host_filter_records_pack_without_foreign_enum_reinterpretation() {
        unsafe {
            let mut record = GuiDrawEffects {
                gradient: Default::default(),
                masks: Default::default(),
                sdf: Default::default(),
                surface: gui_surface_default(),
                filter: GuiImageFilter {
                    kind: GUI_FILTER_THRESHOLD,
                    overwrite: 1,
                    threshold: 0.75,
                    ..Default::default()
                },
                stencil: [0; 2],
            };
            let mut bytes = [0; 1024];
            assert!(gui_effect_uniform(&record, bytes.as_mut_ptr(), bytes.len()));
            assert_eq!(f32::from_le_bytes(bytes[960..964].try_into().unwrap()), 5.);
            let before = bytes;
            record.filter.kind = 999;
            assert!(!gui_effect_uniform(
                &record,
                bytes.as_mut_ptr(),
                bytes.len()
            ));
            assert_eq!(bytes, before);
            record.filter.kind = GUI_FILTER_COPY;
            record.surface.flags = GUI_SURFACE_DITHER;
            record.surface.dither_kind = 999;
            assert!(!gui_effect_uniform(
                &record,
                bytes.as_mut_ptr(),
                bytes.len()
            ));
            assert_eq!(bytes, before);
        }
    }
    #[test]
    fn foreign_enums_and_short_plan_buffers_reject_without_writing() {
        unsafe {
            let style = gui_surface_default();
            assert!(style.decode().is_some());
            assert!(GuiSurfaceStyle {
                dither_kind: 2,
                ..style
            }
            .decode()
            .is_none());
            assert!(GuiSurfaceStyle {
                spacing: f32::NAN,
                ..style
            }
            .decode()
            .is_none());
            assert!(GuiImageFilter {
                kind: 99,
                ..Default::default()
            }
            .decode()
            .is_none());
            let blur = GuiBlur {
                method: GUI_BLUR_DUAL_KAWASE,
                radius: 4.,
                levels: 3,
            };
            let count = gui_filter_plan(65, 33, blur, 1, 0.7, 1., std::ptr::null_mut(), 0);
            assert_eq!(count, 8);
            let sentinel = GuiFilterStep {
                source: 999,
                ..Default::default()
            };
            let mut steps = [sentinel; 15];
            assert_eq!(
                gui_filter_plan(65, 33, blur, 1, 0.7, 1., steps.as_mut_ptr(), count - 1),
                0
            );
            assert!(steps.iter().all(|s| s.source == 999));
            assert_eq!(
                gui_filter_plan(65, 33, blur, 1, 0.7, 1., steps.as_mut_ptr(), 15),
                count
            );
            assert_eq!(steps[0].filter.kind, GUI_FILTER_THRESHOLD);
            assert_eq!(steps[count - 1].filter.kind, GUI_FILTER_BLOOM_COMPOSE);
            assert_eq!(steps[1].size, [33, 17]);
            assert_eq!(steps[count].source, 999);
        }
    }
    #[test]
    fn surface_validation_and_crossed_stencil_end_preserve_state() {
        unsafe {
            let ctx = crate::test_support::new_context();
            crate::gui_new_frame(ctx, std::ptr::null(), 0.016);
            crate::gui_begin_window(ctx, b"W".as_ptr(), 1, 0);
            let rect = GuiRoundedRect {
                rect: [100., 100., 220., 200.].into(),
                radii: [12.; 4],
            };
            let before = (&*ctx).ctx.inspect_windows().next().unwrap().indices;
            gui_draw_surface(
                ctx,
                rect,
                0,
                0,
                [0., 0., 1., 1.].into(),
                [255; 4].into(),
                GuiSurfaceStyle {
                    dither_kind: 99,
                    ..gui_surface_default()
                },
            );
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert_eq!(
                (&*ctx).ctx.inspect_windows().next().unwrap().indices,
                before
            );
            assert!(gui_begin_stencil_clip(
                ctx,
                crate::GuiStr {
                    ptr: b"S".as_ptr(),
                    len: 1
                },
                rect
            ));
            assert!(gui_begin_gradient_mask(
                ctx,
                GuiGradientMask {
                    start: [100., 100.].into(),
                    end: [220., 100.].into(),
                    opacity: [1., 0.]
                }
            ));
            gui_end_stencil_clip(ctx);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert_eq!((&*ctx).containers.len(), 2);
            gui_end_mask(ctx);
            gui_end_stencil_clip(ctx);
            crate::gui_end_window(ctx);
            crate::gui_render(ctx);
            assert_eq!(crate::gui_last_error_code(), GuiErrorCode::None as u8);
            crate::gui_context_free(ctx);
        }
    }
}
