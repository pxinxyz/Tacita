use crate::{active_frame_ctx, gui_export, GuiColor, GuiContext, GuiErrorCode};
use crate::{slice_arg, str_arg, ContainerKind, ContainerScope, GuiRect, GuiVec2, WindowState};
use tacita::animation::{Easing, MotionSettings};
use tacita::animation::{
    GradientOptions, GradientSpread, GradientStop, Keyframe, PlaybackMode, TimelineOptions,
};
use tacita::id::Id;

const CURVES: [Easing; 7] = [
    Easing::Linear,
    Easing::SmoothStep,
    Easing::SmootherStep,
    Easing::QuadIn,
    Easing::QuadOut,
    Easing::CubicInOut,
    Easing::CubicOut,
];
pub const GUI_PLAYBACK_ONCE: u32 = 0;
pub const GUI_PLAYBACK_LOOP: u32 = 1;
pub const GUI_PLAYBACK_PING_PONG: u32 = 2;
pub const GUI_GRADIENT_CLAMP: u32 = 0;
pub const GUI_GRADIENT_REPEAT: u32 = 1;
pub const GUI_GRADIENT_MIRROR: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GuiKeyframe {
    pub time: f32,
    pub value: f32,
    pub easing: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GuiTimelineOptions {
    pub mode: u32,
    pub speed: f32,
    pub playing: u8,
    pub reserved: [u8; 7],
    pub revision: u64,
}
impl GuiTimelineOptions {
    fn decode(self) -> Option<TimelineOptions> {
        let mode = match self.mode {
            0 => PlaybackMode::Once,
            1 => PlaybackMode::Loop,
            2 => PlaybackMode::PingPong,
            _ => return None,
        };
        self.speed.is_finite().then_some(TimelineOptions {
            mode,
            speed: self.speed,
            playing: self.playing != 0,
            revision: self.revision,
        })
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GuiGradientStop {
    pub position: f32,
    pub color: GuiColor,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GuiGradientOptions {
    pub axis: GuiVec2,
    pub spread: u32,
    pub resolution: u32,
}
impl GuiGradientOptions {
    fn decode(self) -> Option<GradientOptions> {
        let spread = match self.spread {
            0 => GradientSpread::Clamp,
            1 => GradientSpread::Repeat,
            2 => GradientSpread::Mirror,
            _ => return None,
        };
        let options = GradientOptions {
            axis: self.axis.into(),
            spread,
            resolution: self.resolution,
        };
        options.is_valid().then_some(options)
    }
}
fn key(raw: GuiKeyframe) -> Keyframe {
    Keyframe {
        time: raw.time,
        value: raw.value,
        easing: CURVES[raw.easing as usize],
    }
}
fn valid_keys(keys: &[GuiKeyframe]) -> bool {
    keys.iter().all(|k| k.easing < CURVES.len() as u32)
        && tacita::animation::timeline::valid_keyframes_with(keys.len(), |i| key(keys[i]))
}
fn stop(raw: GuiGradientStop) -> GradientStop {
    GradientStop {
        position: raw.position,
        color: raw.color.into(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_sample_keyframes(
    keys: *const GuiKeyframe,
    count: usize,
    time: f32,
) -> f32 {
    gui_export!(
        || {
            let Some(keys) = (unsafe { slice_arg(keys, count) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            };
            if !time.is_finite() || !valid_keys(keys) {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            }
            tacita::animation::timeline::sample_keyframes_with(
                keys.len(),
                |i| key(keys[i]),
                time,
            )
        },
        0.0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_animate_timeline(
    ctx: *mut GuiContext,
    id: u64,
    keys: *const GuiKeyframe,
    count: usize,
    options: GuiTimelineOptions,
) -> f32 {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return 0.0;
            };
            let Some(keys) = (unsafe { slice_arg(keys, count) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            };
            let Some(options) = options.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            };
            if !valid_keys(keys) {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            }
            gui.ctx
                .animate_timeline_with(Id(id), keys.len(), |i| key(keys[i]), options)
        },
        0.0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_seek_timeline(ctx: *mut GuiContext, id: u64, time: f32) {
    gui_export!(
        || {
            if ctx.is_null() || !time.is_finite() || time < 0.0 {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            }
            unsafe {
                (&mut *ctx).ctx.seek_timeline(Id(id), time);
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_animate_rect(
    ctx: *mut GuiContext,
    id: u64,
    initial: GuiRect,
    target: GuiRect,
    duration: f32,
    curve: u32,
) -> GuiRect {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return [0.0; 4].into();
            };
            let Some(curve) = easing(curve) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return [0.0; 4].into();
            };
            if !tacita::animation::layout::valid_rect(initial.into())
                || !tacita::animation::layout::valid_rect(target.into())
                || !duration.is_finite()
                || duration < 0.0
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return [0.0; 4].into();
            }
            gui.ctx
                .animate_rect(Id(id), initial.into(), target.into(), duration, curve)
                .into()
        },
        [0.0; 4].into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_begin_animated_layout(
    ctx: *mut GuiContext,
    label: *const u8,
    len: usize,
    target: GuiRect,
    duration: f32,
    curve: u32,
    out_rect: *mut GuiRect,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return false;
            };
            if !matches!(gui.window, WindowState::Open) {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return false;
            }
            let Some(label) = (unsafe { str_arg(label, len) }) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            let Some(curve) = easing(curve) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            };
            if out_rect.is_null()
                || !tacita::animation::layout::valid_rect(target.into())
                || !duration.is_finite()
                || duration < 0.0
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return false;
            }
            let (scope, rect) =
                gui.ctx
                    .ui()
                    .begin_animated_layout(label, target.into(), duration, curve);
            gui.containers.push(ContainerScope::AnimatedLayout(scope));
            unsafe {
                *out_rect = rect.into();
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_end_animated_layout(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if gui.containers.last().map(ContainerScope::kind)
                != Some(ContainerKind::AnimatedLayout)
            {
                crate::set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            if let Some(ContainerScope::AnimatedLayout(scope)) = gui.containers.pop() {
                gui.ctx.ui().end_animated_layout(scope);
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_gradient(
    ctx: *mut GuiContext,
    rect: GuiRect,
    stops: *const GuiGradientStop,
    count: usize,
    options: GuiGradientOptions,
) {
    gui_export!(
        || {
            unsafe {
                draw_gradient(ctx, rect, stops, count, options, None);
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_draw_animated_gradient(
    ctx: *mut GuiContext,
    rect: GuiRect,
    stops: *const GuiGradientStop,
    count: usize,
    options: GuiGradientOptions,
    time: f32,
    period: f32,
) {
    gui_export!(
        || {
            unsafe {
                draw_gradient(ctx, rect, stops, count, options, Some((time, period)));
            }
        },
        ()
    )
}
unsafe fn draw_gradient(
    ctx: *mut GuiContext,
    rect: GuiRect,
    stops: *const GuiGradientStop,
    count: usize,
    options: GuiGradientOptions,
    clock: Option<(f32, f32)>,
) {
    let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
        return;
    };
    let Some(stops) = (unsafe { slice_arg(stops, count) }) else {
        crate::set_last_error(GuiErrorCode::InvalidInput);
        return;
    };
    let Some(options) = options.decode() else {
        crate::set_last_error(GuiErrorCode::InvalidInput);
        return;
    };
    if !tacita::animation::layout::valid_rect(rect.into())
        || !tacita::animation::gradient::valid_stops_with(stops.len(), |i| stop(stops[i]))
        || clock
            .is_some_and(|(time, period)| !time.is_finite() || !period.is_finite() || period <= 0.0)
    {
        crate::set_last_error(GuiErrorCode::InvalidInput);
        return;
    }
    if let Some((time, period)) = clock {
        gui.ctx.ui().draw_animated_gradient_with(
            rect.into(),
            stops.len(),
            |i| stop(stops[i]),
            options,
            time,
            period,
        );
    } else {
        gui.ctx
            .ui()
            .draw_gradient_with(rect.into(), stops.len(), |i| stop(stops[i]), options, 0.0);
    }
}

pub const GUI_EASING_LINEAR: u32 = 0;
pub const GUI_EASING_SMOOTH_STEP: u32 = 1;
pub const GUI_EASING_SMOOTHER_STEP: u32 = 2;
pub const GUI_EASING_QUAD_IN: u32 = 3;
pub const GUI_EASING_QUAD_OUT: u32 = 4;
pub const GUI_EASING_CUBIC_IN_OUT: u32 = 5;
pub const GUI_EASING_CUBIC_OUT: u32 = 6;

fn easing(value: u32) -> Option<Easing> {
    Some(match value {
        GUI_EASING_LINEAR => Easing::Linear,
        GUI_EASING_SMOOTH_STEP => Easing::SmoothStep,
        GUI_EASING_SMOOTHER_STEP => Easing::SmootherStep,
        GUI_EASING_QUAD_IN => Easing::QuadIn,
        GUI_EASING_QUAD_OUT => Easing::QuadOut,
        GUI_EASING_CUBIC_IN_OUT => Easing::CubicInOut,
        GUI_EASING_CUBIC_OUT => Easing::CubicOut,
        _ => return None,
    })
}
fn easing_code(value: Easing) -> u32 {
    match value {
        Easing::Linear => GUI_EASING_LINEAR,
        Easing::SmoothStep => GUI_EASING_SMOOTH_STEP,
        Easing::SmootherStep => GUI_EASING_SMOOTHER_STEP,
        Easing::QuadIn => GUI_EASING_QUAD_IN,
        Easing::QuadOut => GUI_EASING_QUAD_OUT,
        Easing::CubicInOut => GUI_EASING_CUBIC_IN_OUT,
        Easing::CubicOut => GUI_EASING_CUBIC_OUT,
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuiMotionSettings {
    pub enabled: u8,
    pub reduced_motion: u8,
    pub reserved: [u8; 2],
    pub easing: u32,
    pub hover: f32,
    pub focus: f32,
    pub indicator: f32,
    pub popup: f32,
    pub collapse: f32,
    pub scroll: f32,
}
impl From<MotionSettings> for GuiMotionSettings {
    fn from(s: MotionSettings) -> Self {
        Self {
            enabled: s.enabled as u8,
            reduced_motion: s.reduced_motion as u8,
            reserved: [0; 2],
            easing: easing_code(s.easing),
            hover: s.hover,
            focus: s.focus,
            indicator: s.indicator,
            popup: s.popup,
            collapse: s.collapse,
            scroll: s.scroll,
        }
    }
}
impl GuiMotionSettings {
    fn decode(self) -> Option<MotionSettings> {
        let s = MotionSettings {
            enabled: self.enabled != 0,
            reduced_motion: self.reduced_motion != 0,
            easing: easing(self.easing)?,
            hover: self.hover,
            focus: self.focus,
            indicator: self.indicator,
            popup: self.popup,
            collapse: self.collapse,
            scroll: self.scroll,
        };
        s.is_valid().then_some(s)
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_ease(curve: u32, progress: f32) -> f32 {
    gui_export!(
        || {
            let Some(curve) = easing(curve) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            };
            if !progress.is_finite() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            }
            curve.sample(progress)
        },
        0.0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_pulse(time: f32, period: f32) -> f32 {
    gui_export!(
        || {
            if !time.is_finite() || !period.is_finite() || period <= 0.0 {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 1.0;
            }
            tacita::animation::pulse(time, period)
        },
        1.0
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_motion_settings(ctx: *mut GuiContext) -> GuiMotionSettings {
    gui_export!(
        || {
            if ctx.is_null() {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return MotionSettings::default().into();
            }
            unsafe { (&*ctx).ctx.motion_settings().into() }
        },
        MotionSettings::default().into()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_set_motion_settings(
    ctx: *mut GuiContext,
    settings: GuiMotionSettings,
) {
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
            let Some(settings) = settings.decode() else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return;
            };
            gui.ctx.set_motion_settings(settings);
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_animate_float(
    ctx: *mut GuiContext,
    id: u64,
    initial: f32,
    target: f32,
    duration: f32,
    curve: u32,
) -> f32 {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return 0.0;
            };
            let Some(curve) = easing(curve) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            };
            if !initial.is_finite()
                || !target.is_finite()
                || !duration.is_finite()
                || duration < 0.0
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            }
            gui.ctx
                .animate_float(Id(id), initial, target, duration, curve)
        },
        0.0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_animate_spring(
    ctx: *mut GuiContext,
    id: u64,
    initial: f32,
    target: f32,
    response: f32,
) -> f32 {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return 0.0;
            };
            if !initial.is_finite()
                || !target.is_finite()
                || !response.is_finite()
                || response < 0.0
            {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return 0.0;
            }
            gui.ctx.animate_spring(Id(id), initial, target, response)
        },
        0.0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_animate_color(
    ctx: *mut GuiContext,
    id: u64,
    initial: GuiColor,
    target: GuiColor,
    duration: f32,
    curve: u32,
) -> GuiColor {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return [0; 4].into();
            };
            let Some(curve) = easing(curve) else {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return [0; 4].into();
            };
            if !duration.is_finite() || duration < 0.0 {
                crate::set_last_error(GuiErrorCode::InvalidInput);
                return [0; 4].into();
            }
            gui.ctx
                .animate_color(Id(id), initial.into(), target.into(), duration, curve)
                .into()
        },
        [0; 4].into()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::new_context;
    #[test]
    fn validates_borrowed_tracks_and_recovers_from_crossed_layout_end() {
        unsafe {
            let ctx = new_context();
            gui_set_motion_settings(
                ctx,
                GuiMotionSettings {
                    enabled: 1,
                    ..gui_motion_settings(ctx)
                },
            );
            let keys = [
                GuiKeyframe {
                    time: 0.0,
                    value: 0.0,
                    easing: 0,
                },
                GuiKeyframe {
                    time: 1.0,
                    value: 1.0,
                    easing: 0,
                },
            ];
            assert_eq!(gui_sample_keyframes(keys.as_ptr(), keys.len(), 0.5), 0.5);
            let options = GuiTimelineOptions {
                mode: GUI_PLAYBACK_LOOP,
                speed: 1.0,
                playing: 255,
                reserved: [0; 7],
                revision: 0,
            };
            crate::gui_new_frame(ctx, core::ptr::null(), 0.1);
            assert_eq!(gui_animate_timeline(ctx, 1, keys.as_ptr(), 2, options), 0.1);
            let bad = [
                GuiKeyframe {
                    easing: u32::MAX,
                    ..keys[0]
                },
                keys[1],
            ];
            gui_animate_timeline(ctx, 1, bad.as_ptr(), 2, options);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            assert_eq!(gui_animate_timeline(ctx, 1, keys.as_ptr(), 2, options), 0.1);
            crate::gui_begin_window(ctx, b"Layout".as_ptr(), 6, crate::GUI_SCROLL_NONE);
            let mut rect = GuiRect::ZERO;
            assert!(gui_begin_animated_layout(
                ctx,
                b"Region".as_ptr(),
                6,
                [80.0, 100.0, 220.0, 200.0].into(),
                0.3,
                0,
                &mut rect
            ));
            crate::gui_end_panel_cell(ctx);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            gui_end_animated_layout(ctx);
            assert_eq!(crate::gui_last_error_code(), GuiErrorCode::None as u8);
            crate::gui_end_window(ctx);
            crate::gui_render(ctx);
            crate::gui_context_free(ctx);
        }
    }
    #[test]
    fn rejects_invalid_settings_without_mutation_and_requires_between_frames() {
        unsafe {
            assert_eq!(gui_ease(GUI_EASING_SMOOTH_STEP, 0.5), 0.5);
            assert_eq!(gui_pulse(0.5, 1.0), 1.0);
            assert_eq!(gui_pulse(0.5, 0.0), 1.0);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            let ctx = new_context();
            let original = gui_motion_settings(ctx);
            for bad in [f32::NAN, f32::INFINITY, -0.1] {
                gui_set_motion_settings(
                    ctx,
                    GuiMotionSettings {
                        hover: bad,
                        ..original
                    },
                );
                assert_eq!(
                    crate::gui_last_error_code(),
                    GuiErrorCode::InvalidInput as u8
                );
                assert_eq!(gui_motion_settings(ctx), original);
            }
            gui_set_motion_settings(
                ctx,
                GuiMotionSettings {
                    easing: u32::MAX,
                    ..original
                },
            );
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            gui_set_motion_settings(
                ctx,
                GuiMotionSettings {
                    enabled: 255,
                    ..original
                },
            );
            assert_eq!(gui_motion_settings(ctx).enabled, 1);
            crate::gui_new_frame(ctx, core::ptr::null(), 0.1);
            gui_set_motion_settings(ctx, original);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::ContractViolation as u8
            );
            let value = gui_animate_float(ctx, 42, 0.0, 1.0, 1.0, GUI_EASING_LINEAR);
            assert!((value - 0.1).abs() < 0.0001);
            assert_eq!(
                gui_animate_float(ctx, 42, 0.0, 1.0, 1.0, GUI_EASING_LINEAR),
                value
            );
            gui_animate_spring(ctx, 4, 0.0, 1.0, f32::NAN);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            gui_animate_color(ctx, 4, [0; 4].into(), [255; 4].into(), 1.0, u32::MAX);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
            crate::gui_render(ctx);
            crate::gui_context_free(ctx);
            gui_set_motion_settings(core::ptr::null_mut(), original);
            assert_eq!(
                crate::gui_last_error_code(),
                GuiErrorCode::InvalidInput as u8
            );
        }
    }
}
