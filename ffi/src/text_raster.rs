use crate::{gui_export, GuiContext, GuiErrorCode};
use restricted_gui::font::TextRasterMode;

pub const GUI_TEXT_RASTER_LEGACY: u32 = 0;
pub const GUI_TEXT_RASTER_SUBPIXEL: u32 = 1;

#[no_mangle]
pub unsafe extern "C" fn gui_set_text_raster_mode(ctx: *mut GuiContext, mode: u32) {
    gui_export!(
        || {
            if ctx.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let gui = unsafe { &mut *ctx };
            if gui.frame_active {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            let mode = match mode {
                GUI_TEXT_RASTER_LEGACY => TextRasterMode::Legacy,
                GUI_TEXT_RASTER_SUBPIXEL => TextRasterMode::Subpixel,
                _ => {
                    crate::set_last_error(GuiErrorCode::InvalidInput);
                    return;
                }
            };
            gui.ctx.set_text_raster_mode(mode);
        },
        ()
    );
}

#[no_mangle]
pub unsafe extern "C" fn gui_text_raster_mode(ctx: *mut GuiContext) -> u32 {
    gui_export!(
        || {
            if ctx.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return GUI_TEXT_RASTER_LEGACY;
            }
            unsafe { &*ctx }.ctx.text_raster_mode() as u32
        },
        GUI_TEXT_RASTER_LEGACY
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_raster_mode_validates_codes_and_rejects_midframe_changes() {
        unsafe {
            let bytes = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
            let ctx = crate::gui_context_new(bytes.as_ptr(), bytes.len());
            assert!(!ctx.is_null());
            assert_eq!(gui_text_raster_mode(ctx), GUI_TEXT_RASTER_LEGACY);
            let generation = crate::gui_atlas_generation(ctx);
            gui_set_text_raster_mode(ctx, 99);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert_eq!(crate::gui_atlas_generation(ctx), generation);
            gui_set_text_raster_mode(ctx, GUI_TEXT_RASTER_SUBPIXEL);
            assert_eq!(crate::gui_last_error_code(), GuiErrorCode::None as u8);
            assert_eq!(gui_text_raster_mode(ctx), GUI_TEXT_RASTER_SUBPIXEL);
            crate::gui_new_frame(ctx, core::ptr::null(), 0.016);
            gui_set_text_raster_mode(ctx, GUI_TEXT_RASTER_LEGACY);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            assert_eq!(gui_text_raster_mode(ctx), GUI_TEXT_RASTER_SUBPIXEL);
            crate::gui_render(ctx);
            gui_set_text_raster_mode(ctx, GUI_TEXT_RASTER_LEGACY);
            assert_eq!(gui_text_raster_mode(ctx), GUI_TEXT_RASTER_LEGACY);
            crate::gui_context_free(ctx);
            gui_set_text_raster_mode(core::ptr::null_mut(), GUI_TEXT_RASTER_SUBPIXEL);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
        }
    }
}
