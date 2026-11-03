use tacita::draw::TextureId;

use crate::{active_frame_ctx, gui_export, str_arg, GuiColor, GuiContext, GuiErrorCode, GuiVec2};

#[no_mangle]
pub unsafe extern "C" fn gui_draw_line(
    ctx: *mut GuiContext,
    p0: GuiVec2,
    p1: GuiVec2,
    col: GuiColor,
    thickness: f32,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx
                .ui()
                .draw_line(p0.into(), p1.into(), col.into(), thickness);
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_draw_rect_filled(
    ctx: *mut GuiContext,
    min: GuiVec2,
    max: GuiVec2,
    col: GuiColor,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx
                .ui()
                .draw_rect_filled(min.into(), max.into(), col.into());
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_draw_rect_outline(
    ctx: *mut GuiContext,
    min: GuiVec2,
    max: GuiVec2,
    col: GuiColor,
    thickness: f32,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx
                .ui()
                .draw_rect_outline(min.into(), max.into(), col.into(), thickness);
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_draw_circle_filled(
    ctx: *mut GuiContext,
    center: GuiVec2,
    radius: f32,
    col: GuiColor,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx
                .ui()
                .draw_circle_filled(center.into(), radius, col.into());
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_draw_circle_outline(
    ctx: *mut GuiContext,
    center: GuiVec2,
    radius: f32,
    col: GuiColor,
    thickness: f32,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx
                .ui()
                .draw_circle_outline(center.into(), radius, col.into(), thickness);
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_draw_arc(
    ctx: *mut GuiContext,
    center: GuiVec2,
    radius: f32,
    angle_min: f32,
    angle_max: f32,
    col: GuiColor,
    thickness: f32,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx.ui().draw_arc(
                center.into(),
                radius,
                angle_min..=angle_max,
                col.into(),
                thickness,
            );
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_draw_polygon_filled(
    ctx: *mut GuiContext,
    points: *const GuiVec2,
    point_count: usize,
    col: GuiColor,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if points.is_null() && point_count > 0 {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            let Some(raw) = (unsafe { crate::slice_arg(points, point_count) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            let mut pts = Vec::new();
            if pts.try_reserve_exact(raw.len()).is_err() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            pts.extend(raw.iter().map(|&p| -> [f32; 2] { p.into() }));
            gui.ctx.ui().draw_polygon_filled(&pts, col.into());
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_draw_image(
    ctx: *mut GuiContext,
    texture: u64,
    min: GuiVec2,
    max: GuiVec2,
    uv_min: GuiVec2,
    uv_max: GuiVec2,
    tint: GuiColor,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            gui.ctx.ui().draw_image(
                TextureId(texture),
                min.into(),
                max.into(),
                uv_min.into(),
                uv_max.into(),
                tint.into(),
            );
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_measure_text(
    ctx: *mut GuiContext,
    text: *const u8,
    text_len: usize,
) -> GuiVec2 {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return GuiVec2 { x: 0.0, y: 0.0 };
            };
            let Some(text) = (unsafe { str_arg(text, text_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return GuiVec2 { x: 0.0, y: 0.0 };
            };
            gui.ctx.ui().measure_text(text).into()
        },
        GuiVec2 { x: 0.0, y: 0.0 }
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_draw_text_at(
    ctx: *mut GuiContext,
    pos: GuiVec2,
    text: *const u8,
    text_len: usize,
    col: GuiColor,
) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            let Some(text) = (unsafe { str_arg(text, text_len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            gui.ctx.ui().draw_text_at(pos.into(), text, col.into());
        },
        ()
    )
}

#[cfg(test)]
mod tests {
    use std::ptr;

    use crate::test_support::*;
    use crate::{gui_last_error_code, GuiColor, GuiErrorCode, GuiVec2};

    const RED: GuiColor = GuiColor {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };

    fn v(x: f32, y: f32) -> GuiVec2 {
        GuiVec2 { x, y }
    }

    fn vertex_count_with(
        ctx: *mut crate::GuiContext,
        content: impl FnOnce(*mut crate::GuiContext),
    ) -> u32 {
        let input = input_at(AWAY, false);
        let mut count = 0;
        crate::test_support::frame(ctx, &input, |ctx| {
            assert!(unsafe {
                crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE)
            });
            content(ctx);
            unsafe { crate::gui_end_window(ctx) };
            count = unsafe { crate::gui_render(ctx) }.vertex_count;
        });
        count
    }

    #[test]
    fn canvas_primitives_emit_geometry() {
        let ctx = new_context();
        let baseline = vertex_count_with(ctx, |_| {});
        let with_canvas = vertex_count_with(ctx, |ctx| unsafe {
            crate::gui_draw_line(ctx, v(70.0, 100.0), v(120.0, 100.0), RED, 1.0);
            crate::gui_draw_rect_filled(ctx, v(70.0, 110.0), v(120.0, 130.0), RED);
            crate::gui_draw_rect_outline(ctx, v(70.0, 140.0), v(120.0, 160.0), RED, 1.0);
            crate::gui_draw_circle_filled(ctx, v(90.0, 180.0), 10.0, RED);
            crate::gui_draw_circle_outline(ctx, v(90.0, 210.0), 10.0, RED, 1.0);
            crate::gui_draw_arc(ctx, v(90.0, 240.0), 10.0, 0.0, 3.0, RED, 1.0);
        });
        assert!(
            with_canvas > baseline,
            "canvas primitives must emit vertices"
        );
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn polygon_and_image_emit_geometry() {
        let ctx = new_context();
        let baseline = vertex_count_with(ctx, |_| {});
        let points = [v(70.0, 100.0), v(120.0, 100.0), v(95.0, 140.0)];
        let with_draw = vertex_count_with(ctx, |ctx| unsafe {
            crate::gui_draw_polygon_filled(ctx, points.as_ptr(), points.len(), RED);
            crate::gui_draw_image(
                ctx,
                42,
                v(70.0, 150.0),
                v(120.0, 200.0),
                v(0.0, 0.0),
                v(1.0, 1.0),
                RED,
            );
        });
        assert!(with_draw > baseline);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn draw_image_uses_the_host_texture_id() {
        let ctx = new_context();
        let input = input_at(AWAY, false);
        crate::test_support::frame(ctx, &input, |ctx| {
            assert!(unsafe {
                crate::gui_begin_window(ctx, b"W".as_ptr(), 1, crate::GUI_SCROLL_NONE)
            });
            unsafe {
                crate::gui_draw_image(
                    ctx,
                    42,
                    v(70.0, 150.0),
                    v(120.0, 200.0),
                    v(0.0, 0.0),
                    v(1.0, 1.0),
                    RED,
                );
            }
            unsafe { crate::gui_end_window(ctx) };
            let data = unsafe { crate::gui_render(ctx) };
            let commands =
                unsafe { std::slice::from_raw_parts(data.commands, data.command_count as usize) };
            assert!(
                commands.iter().any(|cmd| cmd.texture == 42),
                "the host texture handle must reach the command stream"
            );
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn polygon_null_points_are_invalid_input() {
        let ctx = new_context();
        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_draw_polygon_filled(ctx, ptr::null(), 3, RED) };
            assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        });
        window_frame(ctx, AWAY, false, |ctx| {
            unsafe { crate::gui_draw_polygon_filled(ctx, ptr::null(), 0, RED) };
            assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        });
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn measure_text_matches_the_core_font() {
        let ctx = new_context();
        let mut m = Metrics::new();
        let mut size = GuiVec2 { x: 0.0, y: 0.0 };
        window_frame(ctx, AWAY, false, |ctx| {
            size = unsafe { crate::gui_measure_text(ctx, b"hello".as_ptr(), 5) };
        });
        let expected = m.measure("hello");
        assert_eq!([size.x, size.y], expected);
        unsafe { crate::gui_context_free(ctx) };
    }

    #[test]
    fn draw_text_at_emits_glyphs() {
        let ctx = new_context();
        let baseline = vertex_count_with(ctx, |_| {});
        let with_text = vertex_count_with(ctx, |ctx| {
            unsafe { crate::gui_draw_text_at(ctx, v(70.0, 100.0), b"free text".as_ptr(), 9, RED) };
        });
        assert!(with_text > baseline, "free text must emit glyph vertices");
        unsafe { crate::gui_context_free(ctx) };
    }
}
