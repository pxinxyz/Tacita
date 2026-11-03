use crate::{
    active_frame_ctx, gui_export, str_arg, ContainerKind, ContainerScope, GuiColor, GuiContext,
    GuiErrorCode, GuiRect, GuiVec2, WindowState,
};
use tacita::shapes::{BorderPlacement, RoundedRect, Shadow};

pub const GUI_BORDER_INNER: u32 = 0;
pub const GUI_BORDER_CENTER: u32 = 1;
pub const GUI_BORDER_OUTER: u32 = 2;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiRoundedRect {
    pub rect: GuiRect,
    pub radii: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiShadow {
    pub offset: GuiVec2,
    pub spread: f32,
    pub blur: f32,
    pub color: GuiColor,
    pub inset: u8,
    pub reserved: [u8; 3],
}
impl GuiRoundedRect {
    fn decode(self) -> Option<RoundedRect> {
        RoundedRect::new(self.rect.into(), self.radii).ok()
    }
}
fn placement(code: u32) -> Option<BorderPlacement> {
    match code {
        GUI_BORDER_INNER => Some(BorderPlacement::Inner),
        GUI_BORDER_CENTER => Some(BorderPlacement::Center),
        GUI_BORDER_OUTER => Some(BorderPlacement::Outer),
        _ => None,
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

#[no_mangle]
pub unsafe extern "C" fn gui_rounded_distance(shape: GuiRoundedRect, point: GuiVec2) -> f32 {
    gui_export!(
        || {
            let Some(shape) = shape.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            };
            if !point.x.is_finite()
                || !point.y.is_finite()
                || point.x.abs() > 1_000_000.0
                || point.y.abs() > 1_000_000.0
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            }
            shape.distance(point.into())
        },
        0.0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_rounded_rect(
    ctx: *mut GuiContext,
    shape: GuiRoundedRect,
    color: GuiColor,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            let Some(shape) = shape.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            if gui.ctx.ui().draw_rounded_rect(shape, color.into()).is_err() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_rounded_border(
    ctx: *mut GuiContext,
    shape: GuiRoundedRect,
    color: GuiColor,
    width: f32,
    mode: u32,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            let (Some(shape), Some(mode)) = (shape.decode(), placement(mode)) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            if gui
                .ctx
                .ui()
                .draw_rounded_border(shape, color.into(), width, mode)
                .is_err()
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_shadow(
    ctx: *mut GuiContext,
    shape: GuiRoundedRect,
    shadow: GuiShadow,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            let Some(shape) = shape.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            if shadow.inset > 1 {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let spec = Shadow {
                offset: shadow.offset.into(),
                spread: shadow.spread,
                blur: shadow.blur,
                color: shadow.color.into(),
                inset: shadow.inset != 0,
            };
            if gui.ctx.ui().draw_shadow(shape, spec).is_err() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_glow(
    ctx: *mut GuiContext,
    shape: GuiRoundedRect,
    color: GuiColor,
    reach: f32,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            let Some(shape) = shape.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            if gui.ctx.ui().draw_glow(shape, color.into(), reach).is_err() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    );
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_rounded_clip(
    ctx: *mut GuiContext,
    label: *const u8,
    len: usize,
    shape: GuiRoundedRect,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let (Some(label), Some(shape)) = ((unsafe { str_arg(label, len) }), shape.decode())
            else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            match gui.ctx.ui().begin_rounded_clip(label, shape) {
                Ok(scope) => {
                    gui.containers.push(ContainerScope::RoundedClip(scope));
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
pub unsafe extern "C" fn gui_end_rounded_clip(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return;
            };
            if gui.containers.last().map(ContainerScope::kind) != Some(ContainerKind::RoundedClip) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            if let Some(ContainerScope::RoundedClip(scope)) = gui.containers.pop() {
                gui.ctx.ui().end_rounded_clip(scope);
            }
        },
        ()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_bad_shapes_and_crossed_ends_before_mutation_and_unwinds() {
        unsafe {
            let bytes = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
            let ctx = crate::gui_context_new(bytes.as_ptr(), bytes.len());
            assert!(!ctx.is_null());
            crate::gui_new_frame(ctx, core::ptr::null(), 0.016);
            crate::gui_begin_window(ctx, b"W".as_ptr(), 1, 0);
            let shape = GuiRoundedRect {
                rect: [100.0, 100.0, 220.0, 200.0].into(),
                radii: [24.0; 4],
            };
            let generation = crate::gui_atlas_generation(ctx);
            let before = (&*ctx).ctx.inspect_windows().next().unwrap().indices;
            gui_draw_rounded_border(ctx, shape, [255; 4].into(), 1.0, 99);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert_eq!(
                (&*ctx).ctx.inspect_windows().next().unwrap().indices,
                before
            );
            assert_eq!(crate::gui_atlas_generation(ctx), generation);
            let bad = GuiRoundedRect {
                radii: [f32::NAN; 4],
                ..shape
            };
            assert!(!gui_begin_rounded_clip(ctx, b"clip".as_ptr(), 4, bad));
            assert!((&*ctx).containers.is_empty());
            assert!(gui_begin_rounded_clip(ctx, b"clip".as_ptr(), 4, shape));
            crate::gui_end_child(ctx);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert_eq!((&*ctx).containers.len(), 1);
            gui_draw_rounded_rect(ctx, shape, [255; 4].into());
            let data = crate::gui_render(ctx);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert!(data.index_count > 0);
            assert!((&*ctx).containers.is_empty());
            crate::gui_new_frame(ctx, core::ptr::null(), 0.016);
            crate::gui_render(ctx);
            assert_eq!(crate::gui_last_error_code(), GuiErrorCode::None as u8);
            crate::gui_context_free(ctx);
        }
    }
}
