use crate::{
    active_frame_ctx, gui_export, str_arg, GuiColor, GuiContext, GuiErrorCode, GuiRect, GuiVec2,
    WindowState,
};
use restricted_gui::svg::SvgIconId;
pub const GUI_SVG_ICON_INVALID: u32 = u32::MAX;

#[no_mangle]
pub unsafe extern "C" fn gui_add_svg_icon(
    ctx: *mut GuiContext,
    source: *const u8,
    len: usize,
    size: GuiVec2,
) -> u32 {
    gui_export!(
        || {
            if ctx.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return GUI_SVG_ICON_INVALID;
            }
            let gui = unsafe { &mut *ctx };
            if gui.frame_active {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return GUI_SVG_ICON_INVALID;
            }
            let Some(source) = (unsafe { str_arg(source, len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return GUI_SVG_ICON_INVALID;
            };
            match gui.ctx.add_svg_icon(source, size.into()) {
                Ok(id) => id.0,
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    GUI_SVG_ICON_INVALID
                }
            }
        },
        GUI_SVG_ICON_INVALID
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_svg_icon(
    ctx: *mut GuiContext,
    id: u32,
    rect: GuiRect,
    tint: GuiColor,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if !matches!(gui.window, WindowState::Open) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            if gui
                .ctx
                .ui()
                .draw_svg_icon(SvgIconId(id), rect.into(), tint.into())
                .is_err()
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_svg_icon(
    ctx: *mut GuiContext,
    id: u32,
    size: GuiVec2,
    tint: GuiColor,
) -> GuiRect {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return GuiRect::ZERO;
            };
            if !matches!(gui.window, WindowState::Open) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return GuiRect::ZERO;
            }
            match gui
                .ctx
                .ui()
                .svg_icon(SvgIconId(id), size.into(), tint.into())
            {
                Ok(rect) => rect.into(),
                Err(_) => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    GuiRect::ZERO
                }
            }
        },
        GuiRect::ZERO
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registration_and_invalid_draws_do_not_partially_mutate() {
        unsafe {
            let font = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
            let ctx = crate::gui_context_new(font.as_ptr(), font.len());
            assert!(!ctx.is_null());
            let source = b"<svg viewBox='0 0 16 16'><circle cx='8' cy='8' r='6'/></svg>";
            let before = crate::gui_atlas_generation(ctx);
            let bad = b"<svg><image href='external'/></svg>";
            assert_eq!(
                gui_add_svg_icon(ctx, bad.as_ptr(), bad.len(), [16.0; 2].into()),
                GUI_SVG_ICON_INVALID
            );
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert_eq!(crate::gui_atlas_generation(ctx), before);
            let id = gui_add_svg_icon(ctx, source.as_ptr(), source.len(), [16.0; 2].into());
            assert_ne!(id, GUI_SVG_ICON_INVALID);
            crate::gui_new_frame(ctx, core::ptr::null(), 0.016);
            assert_eq!(
                gui_add_svg_icon(ctx, source.as_ptr(), source.len(), [16.0; 2].into()),
                GUI_SVG_ICON_INVALID
            );
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert!(crate::gui_begin_window(
                ctx,
                b"svg".as_ptr(),
                3,
                crate::GUI_SCROLL_NONE
            ));
            let before = (*ctx).ctx.inspect_windows().next().unwrap().vertices;
            let color = [255; 4].into();
            assert_eq!(
                gui_svg_icon(ctx, GUI_SVG_ICON_INVALID, [16.0; 2].into(), color),
                GuiRect::ZERO
            );
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert_eq!(
                (*ctx).ctx.inspect_windows().next().unwrap().vertices,
                before
            );
            let rect = gui_svg_icon(ctx, id, [16.0; 2].into(), color);
            assert!(rect.max.x > rect.min.x);
            gui_draw_svg_icon(ctx, id, rect, color);
            assert_eq!(crate::gui_last_error_code(), GuiErrorCode::None as u8);
            crate::gui_end_window(ctx);
            crate::gui_render(ctx);
            crate::gui_context_free(ctx);
        }
    }
}
