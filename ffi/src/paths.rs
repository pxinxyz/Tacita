use crate::{
    active_frame_ctx, gui_export, slice_arg, GuiColor, GuiContext, GuiErrorCode, GuiVec2,
    WindowState,
};
use tacita::paths::{
    DashPattern, LineCap, LineJoin, StrokeStyle, MAX_DASH_LENGTHS, MAX_PATH_POINTS,
    MAX_POLYGON_POINTS,
};

pub const GUI_JOIN_MITER: u32 = 0;
pub const GUI_JOIN_BEVEL: u32 = 1;
pub const GUI_JOIN_ROUND: u32 = 2;
pub const GUI_CAP_BUTT: u32 = 0;
pub const GUI_CAP_SQUARE: u32 = 1;
pub const GUI_CAP_ROUND: u32 = 2;
pub const GUI_PATH_MAX_POINTS: u32 = MAX_PATH_POINTS as u32;
pub const GUI_POLYGON_MAX_POINTS: u32 = MAX_POLYGON_POINTS as u32;
pub const GUI_DASH_MAX_LENGTHS: u32 = MAX_DASH_LENGTHS as u32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiStrokeStyle {
    pub width: f32,
    pub join: u32,
    pub cap: u32,
    pub miter_limit: f32,
    pub tolerance: f32,
}
impl GuiStrokeStyle {
    fn decode(self) -> Option<StrokeStyle> {
        let style = StrokeStyle {
            width: self.width,
            join: match self.join {
                0 => LineJoin::Miter,
                1 => LineJoin::Bevel,
                2 => LineJoin::Round,
                _ => return None,
            },
            cap: match self.cap {
                0 => LineCap::Butt,
                1 => LineCap::Square,
                2 => LineCap::Round,
                _ => return None,
            },
            miter_limit: self.miter_limit,
            tolerance: self.tolerance,
        };
        style.is_valid().then_some(style)
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
unsafe fn points<'a>(ptr: *const GuiVec2, count: usize, limit: usize) -> Option<&'a [[f32; 2]]> {
    if count > limit {
        crate::set_last_error(GuiErrorCode::InvalidInput);
        return None;
    }
    let result = unsafe { slice_arg(ptr.cast::<[f32; 2]>(), count) };
    if result.is_none() {
        crate::set_last_error(GuiErrorCode::InvalidInput);
    }
    result
}
fn result(value: Result<(), tacita::paths::PathError>) -> bool {
    if value.is_err() {
        crate::set_last_error(GuiErrorCode::InvalidInput);
        false
    } else {
        true
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_draw_polyline(
    ctx: *mut GuiContext,
    ptr: *const GuiVec2,
    count: usize,
    closed: u8,
    style: GuiStrokeStyle,
    color: GuiColor,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(points) = (unsafe { points(ptr, count, MAX_PATH_POINTS) }) else {
                return false;
            };
            let Some(style) = style.decode().filter(|_| closed <= 1) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            result(
                gui.ctx
                    .ui()
                    .draw_polyline(points, closed != 0, style, color.into()),
            )
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_dashed_polyline(
    ctx: *mut GuiContext,
    ptr: *const GuiVec2,
    count: usize,
    closed: u8,
    style: GuiStrokeStyle,
    lengths: *const f32,
    dash_count: usize,
    phase: f32,
    color: GuiColor,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(points) = (unsafe { points(ptr, count, MAX_PATH_POINTS) }) else {
                return false;
            };
            let Some(style) = style
                .decode()
                .filter(|_| closed <= 1 && (1..=MAX_DASH_LENGTHS).contains(&dash_count))
            else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(lengths) = (unsafe { slice_arg(lengths, dash_count) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            result(gui.ctx.ui().draw_dashed_polyline(
                points,
                closed != 0,
                style,
                DashPattern { lengths, phase },
                color.into(),
            ))
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_quadratic_bezier(
    ctx: *mut GuiContext,
    ptr: *const GuiVec2,
    style: GuiStrokeStyle,
    color: GuiColor,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(points) = (unsafe { points(ptr, 3, 3) }) else {
                return false;
            };
            let Some(style) = style.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            result(gui.ctx.ui().draw_quadratic_bezier(
                [points[0], points[1], points[2]],
                style,
                color.into(),
            ))
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_cubic_bezier(
    ctx: *mut GuiContext,
    ptr: *const GuiVec2,
    style: GuiStrokeStyle,
    color: GuiColor,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(points) = (unsafe { points(ptr, 4, 4) }) else {
                return false;
            };
            let Some(style) = style.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            result(gui.ctx.ui().draw_cubic_bezier(
                [points[0], points[1], points[2], points[3]],
                style,
                color.into(),
            ))
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_concave_polygon(
    ctx: *mut GuiContext,
    ptr: *const GuiVec2,
    count: usize,
    color: GuiColor,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { canvas(ctx) }) else {
                return false;
            };
            let Some(points) = (unsafe { points(ptr, count, MAX_POLYGON_POINTS) }) else {
                return false;
            };
            result(gui.ctx.ui().draw_concave_polygon(points, color.into()))
        },
        false
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foreign_paths_validate_before_emission_and_match_core() {
        unsafe {
            let font = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
            let ctx = crate::gui_context_new(font.as_ptr(), font.len());
            assert!(!ctx.is_null());
            crate::gui_new_frame(ctx, core::ptr::null(), 0.016);
            assert!(crate::gui_begin_window(
                ctx,
                b"paths".as_ptr(),
                5,
                crate::GUI_SCROLL_NONE
            ));
            let points = [
                GuiVec2 { x: 100., y: 100. },
                GuiVec2 { x: 180., y: 100. },
                GuiVec2 { x: 180., y: 180. },
            ];
            let style = GuiStrokeStyle {
                width: 6.,
                join: GUI_JOIN_ROUND,
                cap: GUI_CAP_ROUND,
                miter_limit: 4.,
                tolerance: 0.25,
            };
            let color = GuiColor {
                r: 99,
                g: 91,
                b: 255,
                a: 255,
            };
            let before = (*ctx).ctx.inspect_windows().next().unwrap().vertices;
            assert!(!gui_draw_polyline(
                ctx,
                points.as_ptr(),
                3,
                0,
                GuiStrokeStyle { join: 99, ..style },
                color
            ));
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert!(!gui_draw_polyline(ctx, points.as_ptr(), 3, 2, style, color));
            assert!(!gui_draw_polyline(
                ctx,
                core::ptr::null(),
                1,
                0,
                style,
                color
            ));
            assert!(!gui_draw_polyline(
                ctx,
                points.as_ptr(),
                MAX_PATH_POINTS + 1,
                0,
                style,
                color
            ));
            assert!(!gui_draw_polyline(
                ctx,
                core::ptr::dangling::<u8>().cast::<GuiVec2>(),
                1,
                0,
                style,
                color
            ));
            assert!(!gui_draw_dashed_polyline(
                ctx,
                points.as_ptr(),
                3,
                0,
                style,
                [10., 0.].as_ptr(),
                2,
                0.,
                color
            ));
            let bow = [
                points[0],
                points[2],
                GuiVec2 { x: 100., y: 180. },
                points[1],
            ];
            assert!(!gui_draw_concave_polygon(ctx, bow.as_ptr(), 4, color));
            assert_eq!(
                (*ctx).ctx.inspect_windows().next().unwrap().vertices,
                before
            );
            assert!(gui_draw_polyline(ctx, points.as_ptr(), 3, 0, style, color));
            crate::gui_end_window(ctx);
            let actual = crate::gui_render(ctx);
            let mut core = tacita::context::Context::new(font).unwrap();
            core.new_frame(0.016);
            core.ui()
                .window("paths", tacita::ScrollFlags::NONE, |ui| {
                    ui.draw_polyline(
                        &[[100., 100.], [180., 100.], [180., 180.]],
                        false,
                        style.decode().unwrap(),
                        color.into(),
                    )
                    .unwrap();
                });
            let expected = core.render();
            assert_eq!(actual.vertex_count as usize, expected.vertices.len());
            let actual_vertices =
                core::slice::from_raw_parts(actual.vertices, actual.vertex_count as usize);
            for (a, e) in actual_vertices.iter().zip(expected.vertices) {
                assert_eq!(a.pos, e.pos);
                assert_eq!(a.uv, e.uv);
                assert_eq!(a.col, e.col);
            }
            assert_eq!(
                core::slice::from_raw_parts(actual.indices, actual.index_count as usize),
                expected.indices
            );
            crate::gui_context_free(ctx);
        }
    }
}
