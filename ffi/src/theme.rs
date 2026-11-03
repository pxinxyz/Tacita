use crate::{gui_export, GuiColor, GuiContext, GuiErrorCode, GuiVec2};
use tacita::theme::{ThemeTokens, THEME_ROLES};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GuiThemeTokens {
    pub colors: [GuiColor; 16],
    pub frame_padding: GuiVec2,
    pub item_spacing: GuiVec2,
    pub window_padding: GuiVec2,
    pub slider_grab_size: f32,
    pub scrollbar_size: f32,
    pub corner_rounding: f32,
    pub border_size: f32,
    pub anti_aliasing: u8,
    pub reserved: [u8; 3],
}
impl From<ThemeTokens> for GuiThemeTokens {
    fn from(t: ThemeTokens) -> Self {
        Self {
            colors: t.colors.map(Into::into),
            frame_padding: t.frame_padding.into(),
            item_spacing: t.item_spacing.into(),
            window_padding: t.window_padding.into(),
            slider_grab_size: t.slider_grab_size,
            scrollbar_size: t.scrollbar_size,
            corner_rounding: t.corner_rounding,
            border_size: t.border_size,
            anti_aliasing: t.anti_aliasing as u8,
            reserved: [0; 3],
        }
    }
}
impl GuiThemeTokens {
    fn decode(self) -> Option<ThemeTokens> {
        let t = ThemeTokens {
            colors: self.colors.map(Into::into),
            frame_padding: self.frame_padding.into(),
            item_spacing: self.item_spacing.into(),
            window_padding: self.window_padding.into(),
            slider_grab_size: self.slider_grab_size,
            scrollbar_size: self.scrollbar_size,
            corner_rounding: self.corner_rounding,
            border_size: self.border_size,
            anti_aliasing: self.anti_aliasing != 0,
        };
        t.is_valid().then_some(t)
    }
}
#[no_mangle]
pub unsafe extern "C" fn gui_theme_tokens(ctx: *mut GuiContext) -> GuiThemeTokens {
    gui_export!(
        || {
            if ctx.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return ThemeTokens::default().into();
            }
            unsafe { (&*ctx).ctx.theme_tokens().into() }
        },
        ThemeTokens::default().into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_set_theme_tokens(ctx: *mut GuiContext, tokens: GuiThemeTokens) {
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
            let Some(tokens) = tokens.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            gui.ctx.set_theme_tokens(tokens);
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_theme_color(ctx: *mut GuiContext, role: u32) -> GuiColor {
    gui_export!(
        || {
            if ctx.is_null() || role as usize >= THEME_ROLES.len() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return [0; 4].into();
            }
            unsafe { (&*ctx).ctx.theme_color(THEME_ROLES[role as usize]).into() }
        },
        [0; 4].into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_interpolate_theme(
    from: GuiThemeTokens,
    to: GuiThemeTokens,
    amount: f32,
) -> GuiThemeTokens {
    gui_export!(
        || {
            let (Some(from), Some(to)) = (from.decode(), to.decode()) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return ThemeTokens::default().into();
            };
            if !amount.is_finite() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return ThemeTokens::default().into();
            }
            from.interpolate(to, amount).into()
        },
        ThemeTokens::default().into()
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_metrics_flags_and_setter_timing() {
        unsafe {
            let ctx = crate::test_support::new_context();
            let original = gui_theme_tokens(ctx);
            let mut t = original;
            t.border_size = -1.0;
            gui_set_theme_tokens(ctx, t);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert_eq!(gui_theme_tokens(ctx).border_size, original.border_size);
            t = original;
            t.colors[15] = [1, 2, 3, 4].into();
            t.anti_aliasing = 255;
            gui_set_theme_tokens(ctx, t);
            assert_eq!(gui_theme_color(ctx, 15), t.colors[15]);
            assert_eq!(gui_theme_tokens(ctx).anti_aliasing, 1);
            crate::gui_new_frame(ctx, core::ptr::null(), 0.01);
            gui_set_theme_tokens(ctx, original);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            crate::gui_render(ctx);
            crate::gui_context_free(ctx);
        }
    }
}
