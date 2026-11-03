use crate::{
    active_frame_ctx, gui_export, slice_arg, str_arg, GuiColor, GuiContext, GuiErrorCode, GuiRect,
    GuiStr, GuiVec2, WindowState,
};
use restricted_gui::font::FontId;
use restricted_gui::text::{TextAlign, TextMetrics, TextOptions, TextOverflow, TextSpan};

pub const GUI_TEXT_ALIGN_START: u32 = 0;
pub const GUI_TEXT_ALIGN_CENTER: u32 = 1;
pub const GUI_TEXT_ALIGN_END: u32 = 2;
pub const GUI_TEXT_CLIP: u32 = 0;
pub const GUI_TEXT_TRUNCATE: u32 = 1;
pub const GUI_TEXT_ELLIPSIS: u32 = 2;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GuiTextOptions {
    pub wrap: u8,
    pub reserved: [u8; 3],
    pub overflow: u32,
    pub horizontal: u32,
    pub vertical: u32,
    pub line_spacing: f32,
    pub max_lines: u32,
}
fn align(value: u32) -> Option<TextAlign> {
    Some(match value {
        0 => TextAlign::Start,
        1 => TextAlign::Center,
        2 => TextAlign::End,
        _ => return None,
    })
}
impl GuiTextOptions {
    fn decode(self) -> Option<TextOptions> {
        let o = TextOptions {
            wrap: self.wrap != 0,
            overflow: match self.overflow {
                0 => TextOverflow::Clip,
                1 => TextOverflow::Truncate,
                2 => TextOverflow::Ellipsis,
                _ => return None,
            },
            horizontal: align(self.horizontal)?,
            vertical: align(self.vertical)?,
            line_spacing: self.line_spacing,
            max_lines: self.max_lines as usize,
        };
        o.is_valid().then_some(o)
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuiTextMetrics {
    pub size: GuiVec2,
    pub lines: usize,
    pub truncated: u8,
    pub reserved: [u8; 7],
}
impl From<TextMetrics> for GuiTextMetrics {
    fn from(m: TextMetrics) -> Self {
        Self {
            size: m.size.into(),
            lines: m.lines,
            truncated: m.truncated as u8,
            reserved: [0; 7],
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GuiTextSpan {
    pub text: GuiStr,
    pub font: u32,
    pub color: GuiColor,
}

unsafe fn span<'a>(raw: GuiTextSpan) -> TextSpan<'a> {
    let text = if raw.text.len == 0 {
        ""
    } else {
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(raw.text.ptr, raw.text.len))
        }
    };
    TextSpan {
        text,
        font: FontId(raw.font),
        color: raw.color.into(),
    }
}
unsafe fn spans<'a>(
    gui: &GuiContext,
    ptr: *const GuiTextSpan,
    count: usize,
) -> Option<&'a [GuiTextSpan]> {
    let spans = unsafe { slice_arg(ptr, count) }?;
    for s in spans {
        if !gui.ctx.font().is_valid(FontId(s.font))
            || unsafe { str_arg(s.text.ptr, s.text.len) }.is_none()
        {
            return None;
        }
    }
    Some(spans)
}
fn width_valid(width: f32) -> bool {
    width.is_finite() && width >= 0.0
}
fn error() -> GuiTextMetrics {
    crate::set_last_error(GuiErrorCode::InvalidInput);
    TextMetrics::default().into()
}
fn drawing(gui: &GuiContext) -> bool {
    if matches!(gui.window, WindowState::Open) {
        true
    } else {
        crate::set_last_error(GuiErrorCode::ContractViolation);
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_measure_text_block(
    ctx: *mut GuiContext,
    text: *const u8,
    len: usize,
    width: f32,
    options: GuiTextOptions,
) -> GuiTextMetrics {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return TextMetrics::default().into();
            };
            let (Some(text), Some(options)) = (unsafe { str_arg(text, len) }, options.decode())
            else {
                return error();
            };
            if !width_valid(width) {
                return error();
            }
            gui.ctx.ui().measure_text_block(text, width, options).into()
        },
        TextMetrics::default().into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_measure_rich_text(
    ctx: *mut GuiContext,
    ptr: *const GuiTextSpan,
    count: usize,
    width: f32,
    options: GuiTextOptions,
) -> GuiTextMetrics {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return TextMetrics::default().into();
            };
            let (Some(spans), Some(options)) =
                (unsafe { spans(gui, ptr, count) }, options.decode())
            else {
                return error();
            };
            if !width_valid(width) {
                return error();
            }
            gui.ctx
                .ui()
                .measure_rich_text_with(spans.len(), |i| unsafe { span(spans[i]) }, width, options)
                .into()
        },
        TextMetrics::default().into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_text_block(
    ctx: *mut GuiContext,
    text: *const u8,
    len: usize,
    width: f32,
    options: GuiTextOptions,
) -> GuiTextMetrics {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return TextMetrics::default().into();
            };
            if !drawing(gui) {
                return TextMetrics::default().into();
            }
            let (Some(text), Some(options)) = (unsafe { str_arg(text, len) }, options.decode())
            else {
                return error();
            };
            if !width_valid(width) {
                return error();
            }
            gui.ctx.ui().text_block(text, width, options).into()
        },
        TextMetrics::default().into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_rich_text(
    ctx: *mut GuiContext,
    ptr: *const GuiTextSpan,
    count: usize,
    width: f32,
    options: GuiTextOptions,
) -> GuiTextMetrics {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return TextMetrics::default().into();
            };
            if !drawing(gui) {
                return TextMetrics::default().into();
            }
            let (Some(spans), Some(options)) =
                (unsafe { spans(gui, ptr, count) }, options.decode())
            else {
                return error();
            };
            if !width_valid(width) {
                return error();
            }
            gui.ctx
                .ui()
                .rich_text_with(spans.len(), |i| unsafe { span(spans[i]) }, width, options)
                .into()
        },
        TextMetrics::default().into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_text_box(
    ctx: *mut GuiContext,
    rect: GuiRect,
    text: *const u8,
    len: usize,
    color: GuiColor,
    options: GuiTextOptions,
) -> GuiTextMetrics {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return TextMetrics::default().into();
            };
            if !drawing(gui) {
                return TextMetrics::default().into();
            }
            let (Some(text), Some(options)) = (unsafe { str_arg(text, len) }, options.decode())
            else {
                return error();
            };
            if !restricted_gui::animation::layout::valid_rect(rect.into()) {
                return error();
            }
            gui.ctx
                .ui()
                .draw_text_box(rect.into(), text, color.into(), options)
                .into()
        },
        TextMetrics::default().into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_rich_text_box(
    ctx: *mut GuiContext,
    rect: GuiRect,
    ptr: *const GuiTextSpan,
    count: usize,
    options: GuiTextOptions,
) -> GuiTextMetrics {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return TextMetrics::default().into();
            };
            if !drawing(gui) {
                return TextMetrics::default().into();
            }
            let (Some(spans), Some(options)) =
                (unsafe { spans(gui, ptr, count) }, options.decode())
            else {
                return error();
            };
            if !restricted_gui::animation::layout::valid_rect(rect.into()) {
                return error();
            }
            gui.ctx
                .ui()
                .draw_rich_text_box_with(
                    rect.into(),
                    spans.len(),
                    |i| unsafe { span(spans[i]) },
                    options,
                )
                .into()
        },
        TextMetrics::default().into()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_rich_input_before_any_geometry_and_accepts_borrowed_spans() {
        unsafe {
            let ctx = crate::test_support::new_context();
            crate::gui_new_frame(ctx, core::ptr::null(), 0.0);
            crate::gui_begin_window(ctx, b"Text".as_ptr(), 4, crate::GUI_SCROLL_NONE);
            let options = GuiTextOptions {
                wrap: 1,
                reserved: [0; 3],
                overflow: GUI_TEXT_ELLIPSIS,
                horizontal: GUI_TEXT_ALIGN_START,
                vertical: GUI_TEXT_ALIGN_START,
                line_spacing: 0.0,
                max_lines: 2,
            };
            let valid = GuiTextSpan {
                text: GuiStr {
                    ptr: b"Valid".as_ptr(),
                    len: 5,
                },
                font: 0,
                color: [255; 4].into(),
            };
            let invalid_utf8 = [255u8];
            for bad in [
                GuiTextSpan {
                    font: u32::MAX,
                    ..valid
                },
                GuiTextSpan {
                    text: GuiStr {
                        ptr: invalid_utf8.as_ptr(),
                        len: 1,
                    },
                    ..valid
                },
                GuiTextSpan {
                    text: GuiStr {
                        ptr: core::ptr::null(),
                        len: 5,
                    },
                    ..valid
                },
            ] {
                let spans = [valid, bad];
                let before = (&*ctx).ctx.inspect_windows().next().unwrap().vertices;
                assert_eq!(
                    gui_rich_text(ctx, spans.as_ptr(), 2, 100.0, options).lines,
                    0
                );
                assert_eq!(
                    crate::gui_last_error_code(),
                    GuiErrorCode::InvalidInput as u8
                );
                assert_eq!(
                    (&*ctx).ctx.inspect_windows().next().unwrap().vertices,
                    before
                );
            }
            for invalid in [
                GuiTextOptions {
                    overflow: u32::MAX,
                    ..options
                },
                GuiTextOptions {
                    line_spacing: f32::NAN,
                    ..options
                },
                GuiTextOptions {
                    horizontal: 3,
                    ..options
                },
            ] {
                gui_rich_text(ctx, &valid, 1, 100.0, invalid);
                assert_eq!(
                    crate::gui_last_error_code(),
                    GuiErrorCode::InvalidInput as u8
                );
            }
            let m = gui_rich_text(ctx, &valid, 1, 100.0, options);
            assert_eq!(m.lines, 1);
            assert_eq!(m.reserved, [0; 7]);
            assert_eq!(crate::gui_last_error_code(), GuiErrorCode::None as u8);
            crate::gui_end_window(ctx);
            crate::gui_render(ctx);
            crate::gui_context_free(ctx);
        }
    }
}
