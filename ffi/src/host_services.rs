use crate::{gui_export, GuiContext, GuiErrorCode, GuiStr, GuiVec2};
use tacita::{
    accessibility::{AccessibilityAction, AccessibilityActionKind, AccessibilityRole},
    host_requests::{CursorShape, HitTargetPolicy},
    id::Id,
    theme::ThemeTokens,
};
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuiAccessibilityNode {
    pub id: u64,
    pub parent: u32,
    pub role: u32,
    pub rect: [f32; 4],
    pub states: u32,
    pub actions: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuiDuplicateId {
    pub id: u64,
    pub first: [f32; 4],
    pub second: [f32; 4],
}
fn invalid() {
    crate::set_last_error(GuiErrorCode::InvalidInput);
}
unsafe fn context<'a>(p: *mut GuiContext, idle: bool) -> Option<&'a mut GuiContext> {
    if p.is_null() || !p.is_aligned() {
        invalid();
        return None;
    }
    let g = &mut *p;
    if idle && g.frame_active {
        crate::set_last_error(GuiErrorCode::ContractViolation);
        return None;
    }
    Some(g)
}
unsafe fn declaring<'a>(p: *mut GuiContext) -> Option<&'a mut GuiContext> {
    if p.is_null() || !p.is_aligned() {
        invalid();
        return None;
    }
    crate::data::canvas(p)
}
unsafe fn output<T>(p: *mut T) -> bool {
    if p.is_null() || !p.is_aligned() {
        invalid();
        false
    } else {
        true
    }
}
unsafe fn copy_text(g: &GuiContext, id: u64, value: bool, out: *mut u8, cap: usize) -> usize {
    let mut text = String::new();
    if value {
        g.ctx.accessibility_value(Id(id), &mut text);
    } else {
        g.ctx.accessibility_name(Id(id), &mut text);
    }
    let required = text.len();
    if out.is_null() && cap == 0 {
        return required;
    }
    if out.is_null()
        || cap < required
        || cap > isize::MAX as usize
        || (out as usize).checked_add(cap).is_none()
    {
        invalid();
        return required;
    }
    std::ptr::copy_nonoverlapping(text.as_ptr(), out, required);
    required
}
#[no_mangle]
pub unsafe extern "C" fn gui_set_accessibility(ctx: *mut GuiContext, on: bool) -> bool {
    gui_export!(
        || {
            let Some(g) = context(ctx, true) else {
                return false;
            };
            g.ctx.set_accessibility_enabled(on);
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_accessibility_count(ctx: *mut GuiContext) -> usize {
    gui_export!(
        || context(ctx, false).map_or(0, |g| g.ctx.accessibility_count()),
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_accessibility_node(
    ctx: *mut GuiContext,
    index: usize,
    out: *mut GuiAccessibilityNode,
) -> bool {
    gui_export!(
        || {
            if !output(out) {
                return false;
            }
            let Some(g) = context(ctx, false) else {
                return false;
            };
            let Some(n) = g.ctx.accessibility_node(index) else {
                invalid();
                return false;
            };
            *out = GuiAccessibilityNode {
                id: n.id.0,
                parent: n.parent,
                role: n.role as u32,
                rect: n.rect,
                states: n.states,
                actions: n.actions,
            };
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_accessibility_text(
    ctx: *mut GuiContext,
    id: u64,
    value: bool,
    out: *mut u8,
    cap: usize,
) -> usize {
    gui_export!(
        || {
            let Some(g) = context(ctx, false) else {
                return 0;
            };
            copy_text(g, id, value, out, cap)
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_accessibility_action(
    ctx: *mut GuiContext,
    id: u64,
    kind: u32,
    number: f64,
) -> bool {
    gui_export!(
        || {
            let kind = match kind {
                1 => AccessibilityActionKind::Focus,
                2 => AccessibilityActionKind::Activate,
                4 => AccessibilityActionKind::SetNumber,
                _ => {
                    invalid();
                    return false;
                }
            };
            if !number.is_finite() {
                invalid();
                return false;
            }
            let Some(g) = context(ctx, true) else {
                return false;
            };
            g.ctx.submit_accessibility_action(AccessibilityAction {
                id: Id(id),
                kind,
                number,
            })
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_accessibility_info(
    ctx: *mut GuiContext,
    id: u64,
    role: u32,
    value: GuiStr,
    states: u32,
    actions: u32,
) -> bool {
    gui_export!(
        || {
            let Some(role) = AccessibilityRole::from_code(role) else {
                invalid();
                return false;
            };
            if value.len > 256 || states & !255 != 0 || actions & !7 != 0 {
                invalid();
                return false;
            }
            let Some(value) = crate::str_arg(value.ptr, value.len) else {
                invalid();
                return false;
            };
            let Some(g) = declaring(ctx) else {
                return false;
            };
            g.ctx
                .ui()
                .accessibility_info(Id(id), role, value, states, actions);
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_accessibility_name(
    ctx: *mut GuiContext,
    id: u64,
    name: GuiStr,
) -> bool {
    gui_export!(
        || {
            if name.len > 256 {
                invalid();
                return false;
            }
            let Some(name) = crate::str_arg(name.ptr, name.len) else {
                invalid();
                return false;
            };
            let Some(g) = declaring(ctx) else {
                return false;
            };
            g.ctx.ui().accessibility_name(Id(id), name);
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_set_hit_target(ctx: *mut GuiContext, minimum: GuiVec2) -> bool {
    gui_export!(
        || {
            let p = HitTargetPolicy {
                minimum: minimum.into(),
            };
            if !p.is_valid() {
                invalid();
                return false;
            }
            let Some(g) = context(ctx, true) else {
                return false;
            };
            g.ctx.set_hit_target_policy(p);
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_cursor_shape(ctx: *mut GuiContext) -> u32 {
    gui_export!(
        || context(ctx, false).map_or(0, |g| g.ctx.cursor_shape() as u32),
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_request_cursor(
    ctx: *mut GuiContext,
    id: u64,
    rect: crate::GuiRect,
    shape: u32,
) -> bool {
    gui_export!(
        || {
            let Some(shape) = CursorShape::from_code(shape) else {
                invalid();
                return false;
            };
            let r: [f32; 4] = rect.into();
            if r.iter().any(|n| !n.is_finite()) || r[2] < r[0] || r[3] < r[1] {
                invalid();
                return false;
            }
            let Some(g) = declaring(ctx) else {
                return false;
            };
            g.ctx.ui().request_cursor(Id(id), r, shape);
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_invalidate(ctx: *mut GuiContext) {
    gui_export!(
        || {
            if let Some(g) = context(ctx, false) {
                g.ctx.invalidate();
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_next_redraw_in(ctx: *mut GuiContext) -> f32 {
    gui_export!(
        || context(ctx, false)
            .and_then(|g| g.ctx.next_redraw_in())
            .unwrap_or(-1.),
        -1.
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_request_redraw_in(ctx: *mut GuiContext, seconds: f32) -> bool {
    gui_export!(
        || {
            if !seconds.is_finite() || seconds < 0. {
                invalid();
                return false;
            }
            let Some(g) = context(ctx, false) else {
                return false;
            };
            g.ctx.request_redraw_in(seconds);
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_set_duplicate_id_diagnostics(ctx: *mut GuiContext, on: bool) -> bool {
    gui_export!(
        || {
            let Some(g) = context(ctx, true) else {
                return false;
            };
            g.ctx.set_duplicate_id_diagnostics(on);
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_duplicate_id_count(ctx: *mut GuiContext) -> usize {
    gui_export!(
        || context(ctx, false).map_or(0, |g| g.ctx.duplicate_ids().len()),
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_duplicate_id(
    ctx: *mut GuiContext,
    index: usize,
    out: *mut GuiDuplicateId,
) -> bool {
    gui_export!(
        || {
            if !output(out) {
                return false;
            }
            let Some(g) = context(ctx, false) else {
                return false;
            };
            let Some(d) = g.ctx.duplicate_ids().get(index) else {
                invalid();
                return false;
            };
            *out = GuiDuplicateId {
                id: d.id.0,
                first: d.first,
                second: d.second,
            };
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_apply_high_contrast(ctx: *mut GuiContext) -> bool {
    gui_export!(
        || {
            let Some(g) = context(ctx, true) else {
                return false;
            };
            g.ctx.set_theme_tokens(ThemeTokens::high_contrast());
            g.ctx.invalidate();
            true
        },
        false
    )
}

pub struct GuiScreenReaderBridge {
    #[cfg(windows)]
    bridge: crate::accessibility_host::ScreenReaderBridge,
}
#[no_mangle]
pub unsafe extern "C" fn gui_screen_reader_attach(
    ctx: *mut GuiContext,
    hwnd: usize,
) -> *mut GuiScreenReaderBridge {
    gui_export!(
        || {
            let Some(g) = context(ctx, true) else {
                return std::ptr::null_mut();
            };
            #[cfg(windows)]
            {
                match crate::accessibility_host::ScreenReaderBridge::attach(hwnd as isize) {
                    Ok(bridge) => {
                        g.ctx.set_accessibility_enabled(true);
                        Box::into_raw(Box::new(GuiScreenReaderBridge { bridge }))
                    }
                    Err(_) => {
                        invalid();
                        std::ptr::null_mut()
                    }
                }
            }
            #[cfg(not(windows))]
            {
                let _ = (g, hwnd);
                invalid();
                std::ptr::null_mut()
            }
        },
        std::ptr::null_mut()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_screen_reader_free(handle: *mut GuiScreenReaderBridge) {
    gui_export!(
        || {
            if !handle.is_null() {
                if !handle.is_aligned() {
                    invalid();
                    return;
                }
                drop(Box::from_raw(handle));
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_screen_reader_publish(
    handle: *mut GuiScreenReaderBridge,
    ctx: *mut GuiContext,
    origin: GuiVec2,
    scale: f32,
) -> bool {
    gui_export!(
        || {
            if handle.is_null()
                || !handle.is_aligned()
                || !scale.is_finite()
                || scale <= 0.
                || !origin.x.is_finite()
                || !origin.y.is_finite()
            {
                invalid();
                return false;
            }
            let Some(g) = context(ctx, true) else {
                return false;
            };
            #[cfg(windows)]
            {
                (*handle).bridge.publish(&g.ctx, origin.into(), scale);
                true
            }
            #[cfg(not(windows))]
            {
                let _ = g;
                invalid();
                false
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_screen_reader_drain(
    handle: *mut GuiScreenReaderBridge,
    ctx: *mut GuiContext,
) -> usize {
    gui_export!(
        || {
            if handle.is_null() || !handle.is_aligned() {
                invalid();
                return 0;
            }
            let Some(g) = context(ctx, true) else {
                return 0;
            };
            #[cfg(windows)]
            {
                (*handle).bridge.drain_actions(&mut g.ctx)
            }
            #[cfg(not(windows))]
            {
                let _ = g;
                invalid();
                0
            }
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_host_services_overflow(ctx: *mut GuiContext) -> u32 {
    gui_export!(
        || context(ctx, false).map_or(0, |g| u32::from(g.ctx.accessibility_overflowed())
            | (u32::from(g.ctx.duplicate_ids_overflowed()) << 1)),
        0
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_layouts() {
        assert_eq!(std::mem::size_of::<GuiAccessibilityNode>(), 40);
        assert_eq!(std::mem::align_of::<GuiAccessibilityNode>(), 8);
        assert_eq!(std::mem::offset_of!(GuiAccessibilityNode, id), 0);
        assert_eq!(std::mem::offset_of!(GuiAccessibilityNode, parent), 8);
        assert_eq!(std::mem::offset_of!(GuiAccessibilityNode, role), 12);
        assert_eq!(std::mem::offset_of!(GuiAccessibilityNode, rect), 16);
        assert_eq!(std::mem::offset_of!(GuiAccessibilityNode, actions), 36);
        assert_eq!(std::mem::offset_of!(GuiAccessibilityNode, states), 32);
        assert_eq!(std::mem::size_of::<GuiDuplicateId>(), 40);
        assert_eq!(std::mem::align_of::<GuiDuplicateId>(), 8);
        assert_eq!(std::mem::offset_of!(GuiDuplicateId, id), 0);
        assert_eq!(std::mem::offset_of!(GuiDuplicateId, first), 8);
        assert_eq!(std::mem::offset_of!(GuiDuplicateId, second), 24);
    }
    #[test]
    fn invalid_queries_are_atomic() {
        unsafe {
            let mut out = GuiAccessibilityNode {
                id: 123,
                parent: 0,
                role: 0,
                rect: [1.; 4],
                states: 0,
                actions: 0,
            };
            assert!(!gui_accessibility_node(std::ptr::null_mut(), 0, &mut out));
            assert_eq!(out.id, 123);
            assert!(!gui_set_hit_target(
                std::ptr::null_mut(),
                GuiVec2 {
                    x: f32::NAN,
                    y: 24.
                }
            ));
        }
    }
    #[test]
    fn utf8_output_action_validation_and_lifetime_gates() {
        use crate::test_support::{new_context, window_frame, AWAY};
        unsafe {
            let p = new_context();
            assert!(gui_set_accessibility(p, true));
            let mut id = 0;
            window_frame(p, AWAY, false, |p| {
                id = (*p).ctx.ui().widget_id("é##id").0;
                (*p).ctx.ui().button("é##id");
                assert!(!gui_set_hit_target(p, GuiVec2 { x: 32., y: 32. }));
                assert_eq!(
                    crate::gui_last_error_code(),
                    GuiErrorCode::ContractViolation as u8
                );
                assert!(!gui_accessibility_info(
                    p,
                    id,
                    99,
                    GuiStr {
                        ptr: std::ptr::null(),
                        len: 0
                    },
                    0,
                    0
                ));
            });
            assert_eq!(
                gui_accessibility_text(p, id, false, std::ptr::null_mut(), 0),
                2
            );
            let mut out = [9u8];
            assert_eq!(gui_accessibility_text(p, id, false, out.as_mut_ptr(), 1), 2);
            assert_eq!(out, [9]);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert!(!gui_accessibility_action(p, id, 3, 0.));
            assert!(gui_accessibility_action(p, id, 2, 0.));
            let misaligned = (p as *mut u8).add(1).cast::<GuiContext>();
            assert_eq!(gui_accessibility_count(misaligned), 0);
            crate::gui_context_free(p);
        }
    }
}
