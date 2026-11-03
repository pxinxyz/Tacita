use crate::context::Context;
use crate::id::Id;
use alloc::collections::BTreeMap;

pub mod gradient;
pub mod layout;
pub mod timeline;
pub use gradient::{GradientOptions, GradientSpread, GradientStop};
pub use layout::AnimatedLayoutScope;
pub use timeline::{sample_keyframes, Keyframe, PlaybackMode, Timeline, TimelineOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Easing {
    Linear,
    SmoothStep,
    SmootherStep,
    QuadIn,
    QuadOut,
    CubicInOut,
    CubicOut,
}

impl Easing {
    pub fn sample(self, t: f32) -> f32 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let value = match self {
            Self::Linear => t,
            Self::SmoothStep => t * t * (3.0 - 2.0 * t),
            Self::SmootherStep => t * t * t * (t * (6.0 * t - 15.0) + 10.0),
            Self::QuadIn => t * t,
            Self::QuadOut => t * (2.0 - t),
            Self::CubicInOut if t < 0.5 => 4.0 * t * t * t,
            Self::CubicInOut => 1.0 - 4.0 * (1.0 - t) * (1.0 - t) * (1.0 - t),
            Self::CubicOut => 1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t),
        };
        value.clamp(0.0, 1.0)
    }
}

fn seconds(dt: f32) -> f32 {
    if dt.is_finite() {
        dt.max(0.0)
    } else {
        0.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tween {
    value: f32,
    from: f32,
    target: f32,
    elapsed: f32,
    duration: f32,
    easing: Easing,
}

impl Tween {
    pub fn new(value: f32) -> Self {
        assert!(value.is_finite(), "animation value must be finite");
        Self {
            value,
            from: value,
            target: value,
            elapsed: 0.0,
            duration: 0.0,
            easing: Easing::Linear,
        }
    }
    pub fn value(&self) -> f32 {
        self.value
    }
    pub fn target(&self) -> f32 {
        self.target
    }
    pub fn is_settled(&self) -> bool {
        self.value == self.target
    }
    pub fn retarget(&mut self, target: f32, duration: f32, easing: Easing) {
        assert!(
            target.is_finite() && duration.is_finite() && duration >= 0.0,
            "invalid tween target or duration"
        );
        if duration == 0.0 {
            self.snap(target);
            return;
        }
        if target != self.target || duration != self.duration || easing != self.easing {
            self.from = self.value;
            self.target = target;
            self.duration = duration;
            self.easing = easing;
            self.elapsed = 0.0;
        }
    }
    pub fn snap(&mut self, value: f32) {
        *self = Self::new(value);
    }
    pub fn advance(&mut self, dt: f32) -> f32 {
        self.elapsed = (self.elapsed + seconds(dt)).min(self.duration);
        if self.elapsed >= self.duration {
            self.value = self.target;
        } else {
            let t = f64::from(self.easing.sample(self.elapsed / self.duration));
            self.value = (f64::from(self.from) * (1.0 - t) + f64::from(self.target) * t) as f32;
        }
        self.value
    }
}

fn exp_negative(mut x: f32) -> f32 {
    if x >= 64.0 {
        return 0.0;
    }
    let mut squares = 0;
    while x > 0.25 {
        x *= 0.5;
        squares += 1;
    }
    let mut value = 1.0
        / (1.0
            + x * (1.0
                + x * (0.5 + x * (1.0 / 6.0 + x * (1.0 / 24.0 + x * (1.0 / 120.0 + x / 720.0))))));
    for _ in 0..squares {
        value *= value;
    }
    value
}

#[derive(Debug, Clone, Copy)]
pub struct Spring {
    value: f32,
    target: f32,
    velocity: f64,
}
impl Spring {
    pub fn new(value: f32) -> Self {
        assert!(value.is_finite(), "animation value must be finite");
        Self {
            value,
            target: value,
            velocity: 0.0,
        }
    }
    pub fn value(&self) -> f32 {
        self.value
    }
    pub fn velocity(&self) -> f32 {
        self.velocity
            .clamp(-f64::from(f32::MAX), f64::from(f32::MAX)) as f32
    }
    pub fn target(&self) -> f32 {
        self.target
    }
    pub fn retarget(&mut self, target: f32) {
        assert!(target.is_finite(), "animation target must be finite");
        self.target = target;
    }
    pub fn snap(&mut self, value: f32) {
        *self = Self::new(value);
    }
    pub fn advance(&mut self, dt: f32, response: f32) -> f32 {
        assert!(
            response.is_finite() && response >= 0.0,
            "invalid spring response"
        );
        if response == 0.0 {
            self.snap(self.target);
            return self.value;
        }
        let dt = seconds(dt);
        if dt == 0.0 {
            return self.value;
        }
        let omega = 3.89 / response.max(0.0001);
        let decay = exp_negative(omega * dt);
        if decay == 0.0 {
            self.snap(self.target);
            return self.value;
        }
        let (omega, dt, decay) = (f64::from(omega), f64::from(dt), f64::from(decay));
        let displacement = f64::from(self.value) - f64::from(self.target);
        let c = self.velocity + omega * displacement;
        let position = f64::from(self.target) + (displacement + c * dt) * decay;
        self.value = position.clamp(-f64::from(f32::MAX), f64::from(f32::MAX)) as f32;
        self.velocity = (self.velocity - omega * c * dt) * decay;
        if (f64::from(self.value) - f64::from(self.target)).abs() < 0.0001
            && self.velocity.abs() < 0.001
        {
            self.snap(self.target);
        }
        self.value
    }
}

pub fn lerp_color(from: [u8; 4], to: [u8; 4], t: f32) -> [u8; 4] {
    let t = Easing::Linear.sample(t);
    core::array::from_fn(|i| (from[i] as f32 + (to[i] as f32 - from[i] as f32) * t + 0.5) as u8)
}

pub fn pulse(time: f32, period: f32) -> f32 {
    if !time.is_finite() || !period.is_finite() || period <= 0.0 {
        return 1.0;
    }
    let phase = ((time % period) / period + 1.0) % 1.0;
    Easing::SmootherStep.sample(1.0 - (2.0 * phase - 1.0).abs())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSettings {
    pub enabled: bool,
    pub reduced_motion: bool,
    pub hover: f32,
    pub focus: f32,
    pub indicator: f32,
    pub popup: f32,
    pub collapse: f32,
    pub scroll: f32,
    pub easing: Easing,
}
impl Default for MotionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            reduced_motion: false,
            hover: 0.14,
            focus: 0.16,
            indicator: 0.20,
            popup: 0.18,
            collapse: 0.20,
            scroll: 0.12,
            easing: Easing::CubicOut,
        }
    }
}
impl MotionSettings {
    pub fn is_valid(&self) -> bool {
        [
            self.hover,
            self.focus,
            self.indicator,
            self.popup,
            self.collapse,
            self.scroll,
        ]
        .iter()
        .all(|t| t.is_finite() && *t >= 0.0)
    }
    pub fn active(&self) -> bool {
        self.enabled && !self.reduced_motion
    }
}

pub(crate) const HOVER: u32 = 0;
pub(crate) const INDICATOR: u32 = 1;
pub(crate) const INDICATOR_WIDTH: u32 = 2;
pub(crate) const FOCUS: u32 = 3;
pub(crate) const COLLAPSE: u32 = 4;
pub(crate) const SCROLL: u32 = 5;
const CUSTOM: u32 = 32;

#[derive(Clone, Copy)]
enum Value {
    Tween(Tween),
    Spring(Spring),
    Rect {
        from: [f32; 4],
        value: [f32; 4],
        target: [f32; 4],
        phase: Tween,
    },
    Timeline {
        clock: Timeline,
        duration: f32,
    },
    Color {
        from: [f32; 4],
        value: [f32; 4],
        target: [u8; 4],
        phase: Tween,
    },
}
pub(crate) struct Channel {
    value: Value,
    frame: u64,
}
pub(crate) type AnimationCache = BTreeMap<(Id, u32), Channel>;

pub(crate) fn arrow_triangle(ax: f32, ay: f32, size: f32, amount: f32) -> [[f32; 2]; 3] {
    if amount <= 0.0 {
        return [[ax, ay], [ax, ay + size], [ax + size, ay + size * 0.5]];
    }
    if amount >= 1.0 {
        return [[ax, ay], [ax + size, ay], [ax + size * 0.5, ay + size]];
    }
    let (sin, cos) = crate::draw::sin_cos_approx(amount * core::f32::consts::FRAC_PI_2);
    let center = [ax + size * 0.5, ay + size * 0.5];
    [[-0.5, -0.5], [-0.5, 0.5], [0.5, 0.0]].map(|p| {
        [
            center[0] + size * (p[0] * cos - p[1] * sin),
            center[1] + size * (p[0] * sin + p[1] * cos),
        ]
    })
}

pub(crate) struct PopupAnimation {
    id: Option<Id>,
    phase: Tween,
    frame: u64,
    start: Option<(usize, usize, u64)>,
    emitted: bool,
    snapshot: crate::draw::DrawList,
    generation: u64,
}
impl Context {
    pub(crate) fn animation_needs_redraw(&self) -> bool {
        self.motion.active()
            && self.animations.values().any(|s| {
                s.frame == self.motion_frame
                    && match s.value {
                        Value::Tween(t) => !t.is_settled(),
                        Value::Spring(s) => {
                            (s.value() - s.target()).abs() > 0.0001 || s.velocity().abs() > 0.0001
                        }
                        Value::Rect { phase, .. } | Value::Color { phase, .. } => {
                            !phase.is_settled()
                        }
                        Value::Timeline { .. } => false,
                    }
            })
    }
}
impl PopupAnimation {
    pub(crate) fn new() -> Self {
        Self {
            id: None,
            phase: Tween::new(0.0),
            frame: 0,
            start: None,
            emitted: false,
            snapshot: crate::draw::DrawList::new(),
            generation: 0,
        }
    }
    pub(crate) fn needs_redraw(&self) -> bool {
        !self.phase.is_settled() || (!self.emitted && self.phase.value() > 0.)
    }
    pub(crate) fn invalidate(&mut self) {
        self.snapshot.clear();
    }
}

pub(crate) struct CollapseMotion {
    id: Id,
    cursor: crate::layout::Cursor,
    origin: f32,
    amount: f32,
    vertex_start: usize,
    saved_clip: Option<[f32; 4]>,
    saved_prev_left: bool,
    input: Option<InputMask>,
}
struct InputMask {
    mouse_pos: [f32; 2],
    mouse_down: [bool; 5],
    prev_mouse_down: [bool; 5],
    wheel: f32,
    keys: alloc::vec::Vec<crate::input::KeyEvent>,
    activate: Option<Id>,
}

impl crate::ui::Ui<'_> {
    pub fn animate_timeline(&mut self, id: Id, keys: &[Keyframe], options: TimelineOptions) -> f32 {
        self.ctx.animate_timeline(id, keys, options)
    }
    pub fn seek_timeline(&mut self, id: Id, time: f32) {
        self.ctx.seek_timeline(id, time);
    }
    pub fn animate_float(
        &mut self,
        id: Id,
        initial: f32,
        target: f32,
        duration: f32,
        easing: Easing,
    ) -> f32 {
        self.ctx
            .animate_float(id, initial, target, duration, easing)
    }
    pub fn animate_spring(&mut self, id: Id, initial: f32, target: f32, response: f32) -> f32 {
        self.ctx.animate_spring(id, initial, target, response)
    }
    pub fn animate_color(
        &mut self,
        id: Id,
        initial: [u8; 4],
        target: [u8; 4],
        duration: f32,
        easing: Easing,
    ) -> [u8; 4] {
        self.ctx
            .animate_color(id, initial, target, duration, easing)
    }
    pub(crate) fn begin_collapse_motion(
        &mut self,
        id: Id,
        amount: f32,
        closing: bool,
    ) -> Option<CollapseMotion> {
        let ctx = &mut *self.ctx;
        if !ctx.motion.active() {
            return None;
        }
        let cursor = if ctx.popup_scope_active {
            ctx.overlay_cursor
        } else {
            ctx.current_window_mut().cursor
        };
        let origin = cursor.pos_y();
        let height = ctx.collapse_heights.get(&id).copied().unwrap_or(0.0);
        let (_, draw) = crate::widgets::draw_parts(ctx);
        let outer = draw.current_clip();
        let bottom = if amount >= 1.0 {
            outer[3]
        } else {
            origin + height * amount
        };
        let reveal = [outer[0], origin, outer[2], bottom];
        draw.push_clip_rect(reveal);
        let vertex_start = draw.vertex_count();
        let saved_clip = ctx.interaction_clip;
        let clip = saved_clip.map_or(reveal, |clip| crate::draw::intersect_rects(clip, reveal));
        ctx.interaction_clip = Some(clip);
        let saved_prev_left = ctx.prev_mouse_down[0];
        if !closing && !crate::ui::point_in_rect(ctx.input.mouse_pos, clip) {
            ctx.prev_mouse_down[0] = true;
        }
        let input = if closing {
            ctx.disabled_depth += 1;
            let mask = InputMask {
                mouse_pos: ctx.input.mouse_pos,
                mouse_down: ctx.input.mouse_down,
                prev_mouse_down: ctx.prev_mouse_down,
                wheel: ctx.input.mouse_wheel,
                keys: core::mem::take(&mut ctx.input.keys),
                activate: ctx.keyboard_activate,
            };
            ctx.input.mouse_pos = [f32::NAN; 2];
            ctx.input.mouse_down = [false; 5];
            ctx.prev_mouse_down = [false; 5];
            ctx.input.mouse_wheel = 0.0;
            ctx.keyboard_activate = None;
            Some(mask)
        } else {
            None
        };
        Some(CollapseMotion {
            id,
            cursor,
            origin,
            amount,
            vertex_start,
            saved_clip,
            saved_prev_left,
            input,
        })
    }
    pub(crate) fn end_collapse_motion(&mut self, scope: Option<CollapseMotion>) {
        let Some(scope) = scope else {
            return;
        };
        let ctx = &mut *self.ctx;
        let cursor = if ctx.popup_scope_active {
            &mut ctx.overlay_cursor
        } else {
            &mut ctx.current_window_mut().cursor
        };
        let height = (cursor.pos_y() - scope.origin).max(0.0);
        if scope.amount < 1.0 {
            cursor.collapse_from(scope.cursor, height * scope.amount);
        }
        ctx.collapse_heights.insert(scope.id, height);
        let (_, draw) = crate::widgets::draw_parts(ctx);
        draw.fade_since(scope.vertex_start, scope.amount);
        draw.pop_clip_rect();
        ctx.interaction_clip = scope.saved_clip;
        ctx.prev_mouse_down[0] = scope.saved_prev_left;
        if let Some(input) = scope.input {
            ctx.disabled_depth -= 1;
            ctx.input.mouse_pos = input.mouse_pos;
            ctx.input.mouse_down = input.mouse_down;
            ctx.prev_mouse_down = input.prev_mouse_down;
            ctx.input.mouse_wheel = input.wheel;
            ctx.input.keys = input.keys;
            ctx.keyboard_activate = input.activate.filter(|id| ctx.focused == Some(*id));
        }
    }
}

impl Context {
    pub fn set_motion_settings(&mut self, settings: MotionSettings) {
        assert!(
            !self.frame_active,
            "motion settings must change between frames"
        );
        assert!(settings.is_valid(), "invalid motion settings");
        if self.motion.active() && !settings.active() {
            for state in self.animations.values_mut() {
                match &mut state.value {
                    Value::Tween(t) => t.snap(t.target()),
                    Value::Spring(s) => s.snap(s.target()),
                    Value::Rect {
                        from,
                        value,
                        target,
                        phase,
                    } => {
                        *from = *target;
                        *value = *target;
                        phase.snap(1.0);
                    }
                    Value::Timeline { clock, duration } => clock.seek(*duration),
                    Value::Color {
                        from,
                        value,
                        target,
                        phase,
                    } => {
                        *from = target.map(f32::from);
                        *value = *from;
                        phase.snap(1.0);
                    }
                }
            }
            for (&(id, channel), state) in &self.animations {
                if channel != SCROLL {
                    continue;
                }
                let Value::Spring(spring) = state.value else {
                    continue;
                };
                if let Some(window) = self.windows.iter_mut().find(|window| window.id == id) {
                    window.scroll_offset = spring.value();
                }
                if let Some(crate::context::WidgetState::Scroll { offset, .. }) =
                    self.widget_states.get_mut(&id)
                {
                    *offset = spring.value();
                }
            }
            self.popup_animation
                .phase
                .snap(if self.open_popup.is_some() { 1.0 } else { 0.0 });
            self.popup_animation.invalidate();
            for (i, slot) in self.routing.popups.iter_mut().enumerate() {
                slot.motion
                    .phase
                    .snap(if i < self.routing.count { 1.0 } else { 0.0 });
                slot.motion.invalidate();
            }
        }
        self.motion = settings;
    }
    pub fn motion_settings(&self) -> MotionSettings {
        self.motion
    }

    pub fn animate_timeline(&mut self, id: Id, keys: &[Keyframe], options: TimelineOptions) -> f32 {
        self.animate_timeline_with(id, keys.len(), |i| keys[i], options)
    }
    pub fn animate_timeline_with(
        &mut self,
        id: Id,
        count: usize,
        get: impl Fn(usize) -> Keyframe,
        options: TimelineOptions,
    ) -> f32 {
        assert!(
            self.frame_active
                && options.speed.is_finite()
                && timeline::valid_keyframes_with(count, &get),
            "invalid timeline"
        );
        let duration = get(count - 1).time;
        if !self.motion.active() {
            return get(count - 1).value;
        }
        let state = self.animations.entry((id, CUSTOM + 8)).or_insert(Channel {
            value: Value::Timeline {
                clock: Timeline::new(),
                duration,
            },
            frame: self.motion_frame.wrapping_sub(1),
        });
        let Value::Timeline {
            clock,
            duration: stored,
        } = &mut state.value
        else {
            panic!("animation channel type changed");
        };
        *stored = duration;
        let time = if state.frame != self.motion_frame {
            state.frame = self.motion_frame;
            clock.advance(self.dt, duration, options)
        } else {
            clock.advance(0.0, duration, options)
        };
        if self.motion.active()
            && options.playing
            && options.speed != 0.
            && (options.mode != PlaybackMode::Once
                || (options.speed > 0. && time < duration)
                || (options.speed < 0. && time > 0.))
        {
            self.request_redraw_in(1. / 144.);
        }
        timeline::sample_valid(count, get, time)
    }

    pub fn seek_timeline(&mut self, id: Id, time: f32) {
        assert!(time.is_finite() && time >= 0.0, "invalid timeline seek");
        if let Some(Channel {
            value: Value::Timeline { clock, .. },
            frame,
        }) = self.animations.get_mut(&(id, CUSTOM + 8))
        {
            clock.seek(time);
            *frame = self.motion_frame;
        }
    }

    pub(crate) fn motion_rect(
        &mut self,
        id: Id,
        initial: [f32; 4],
        target: [f32; 4],
        duration: f32,
        easing: Easing,
    ) -> [f32; 4] {
        assert!(
            duration.is_finite() && duration >= 0.0,
            "invalid rectangle timing"
        );
        if !self.motion.active() {
            if let Some(state) = self.animations.get_mut(&(id, CUSTOM + 4)) {
                state.value = Value::Rect {
                    from: target,
                    value: target,
                    target,
                    phase: Tween::new(1.0),
                };
            }
            return target;
        }
        let state = self.animations.entry((id, CUSTOM + 4)).or_insert(Channel {
            value: Value::Rect {
                from: initial,
                value: initial,
                target: initial,
                phase: Tween::new(1.0),
            },
            frame: self.motion_frame.wrapping_sub(1),
        });
        let Value::Rect {
            from,
            value,
            target: previous,
            phase,
        } = &mut state.value
        else {
            panic!("animation channel type changed");
        };
        if *previous != target {
            *from = *value;
            *previous = target;
            phase.snap(0.0);
        }
        phase.retarget(1.0, duration, easing);
        if state.frame != self.motion_frame {
            phase.advance(self.dt);
            state.frame = self.motion_frame;
        }
        let t = f64::from(phase.value());
        *value = core::array::from_fn(|i| {
            (f64::from(from[i]) * (1.0 - t) + f64::from(target[i]) * t) as f32
        });
        *value
    }
    pub fn animate_float(
        &mut self,
        id: Id,
        initial: f32,
        target: f32,
        duration: f32,
        easing: Easing,
    ) -> f32 {
        assert!(self.frame_active, "animation requires an active frame");
        self.motion_tween(id, CUSTOM, initial, target, duration, easing)
    }
    pub fn animate_spring(&mut self, id: Id, initial: f32, target: f32, response: f32) -> f32 {
        assert!(self.frame_active, "animation requires an active frame");
        self.motion_spring(id, CUSTOM + 1, initial, target, response)
    }
    pub fn animate_color(
        &mut self,
        id: Id,
        initial: [u8; 4],
        target: [u8; 4],
        duration: f32,
        easing: Easing,
    ) -> [u8; 4] {
        assert!(self.frame_active, "animation requires an active frame");
        self.motion_color(id, CUSTOM + 2, initial, target, duration, easing)
    }
    pub(crate) fn motion_tween(
        &mut self,
        id: Id,
        channel: u32,
        initial: f32,
        target: f32,
        duration: f32,
        easing: Easing,
    ) -> f32 {
        assert!(
            initial.is_finite() && target.is_finite() && duration.is_finite() && duration >= 0.0,
            "invalid animation parameters"
        );
        if !self.motion.active() {
            if let Some(state) = self.animations.get_mut(&(id, channel)) {
                state.value = Value::Tween(Tween::new(target));
                state.frame = self.motion_frame;
            }
            return target;
        }
        let state = self.animations.entry((id, channel)).or_insert(Channel {
            value: Value::Tween(Tween::new(initial)),
            frame: self.motion_frame.wrapping_sub(1),
        });
        let Value::Tween(tween) = &mut state.value else {
            panic!("animation channel type changed");
        };
        tween.retarget(target, duration, easing);
        if state.frame != self.motion_frame {
            tween.advance(self.dt);
            state.frame = self.motion_frame;
        }
        tween.value()
    }
    pub(crate) fn motion_spring(
        &mut self,
        id: Id,
        channel: u32,
        initial: f32,
        target: f32,
        response: f32,
    ) -> f32 {
        assert!(
            initial.is_finite() && target.is_finite() && response.is_finite() && response >= 0.0,
            "invalid spring parameters"
        );
        if !self.motion.active() {
            if let Some(state) = self.animations.get_mut(&(id, channel)) {
                state.value = Value::Spring(Spring::new(target));
                state.frame = self.motion_frame;
            }
            return target;
        }
        let state = self.animations.entry((id, channel)).or_insert(Channel {
            value: Value::Spring(Spring::new(initial)),
            frame: self.motion_frame.wrapping_sub(1),
        });
        let Value::Spring(spring) = &mut state.value else {
            panic!("animation channel type changed");
        };
        spring.retarget(target);
        if state.frame != self.motion_frame {
            spring.advance(self.dt, response);
            state.frame = self.motion_frame;
        }
        spring.value()
    }
    pub(crate) fn motion_color(
        &mut self,
        id: Id,
        channel: u32,
        initial: [u8; 4],
        target: [u8; 4],
        duration: f32,
        easing: Easing,
    ) -> [u8; 4] {
        assert!(
            duration.is_finite() && duration >= 0.0,
            "invalid color duration"
        );
        if !self.motion.active() {
            if let Some(state) = self.animations.get_mut(&(id, channel)) {
                state.value = Value::Color {
                    from: target.map(f32::from),
                    value: target.map(f32::from),
                    target,
                    phase: Tween::new(1.0),
                };
                state.frame = self.motion_frame;
            }
            return target;
        }
        let state = self.animations.entry((id, channel)).or_insert(Channel {
            value: Value::Color {
                from: initial.map(f32::from),
                value: initial.map(f32::from),
                target: initial,
                phase: Tween::new(1.0),
            },
            frame: self.motion_frame.wrapping_sub(1),
        });
        let Value::Color {
            from,
            value,
            target: previous,
            phase,
        } = &mut state.value
        else {
            panic!("animation channel type changed");
        };
        if *previous != target {
            *from = *value;
            *previous = target;
            phase.snap(0.0);
        }
        phase.retarget(1.0, duration, easing);
        if state.frame != self.motion_frame {
            phase.advance(self.dt);
            state.frame = self.motion_frame;
        }
        let t = phase.value();
        *value = core::array::from_fn(|i| from[i] + (target[i] as f32 - from[i]) * t);
        value.map(|v| (v + 0.5) as u8)
    }
    pub(crate) fn widget_color(&mut self, id: Id, color: [u8; 4]) -> [u8; 4] {
        self.motion_color(
            id,
            HOVER,
            color,
            color,
            self.motion.hover,
            self.motion.easing,
        )
    }
    pub(crate) fn tick_popup_motion(&mut self) {
        for (i, slot) in self.routing.popups.iter_mut().enumerate() {
            slot.draw.clear();
            if self.input.viewport_size != [0.; 2] {
                slot.draw.push_clip_rect([
                    0.,
                    0.,
                    self.input.viewport_size[0],
                    self.input.viewport_size[1],
                ]);
            }
            let motion = &mut slot.motion;
            motion.start = None;
            motion.emitted = false;
            if motion.generation != self.font.atlas_generation() {
                motion.invalidate();
            }
            let live = i < self.routing.count;
            if live && slot.id != motion.id {
                motion.id = slot.id;
                motion.phase.snap(0.);
                motion.invalidate();
            }
            if self.motion.active() {
                motion.phase.retarget(
                    if live { 1. } else { 0. },
                    self.motion.popup,
                    self.motion.easing,
                );
                motion.phase.advance(self.dt);
            } else {
                motion.phase.snap(if live { 1. } else { 0. });
            }
            motion.frame = self.motion_frame;
        }
    }
    pub(crate) fn mark_popup_motion(&mut self, id: Id) {
        if !self.motion.active() {
            return;
        }
        let motion = &mut self.popup_animation;
        if motion.start.is_some() {
            return;
        }
        if motion.id != Some(id) {
            motion.id = Some(id);
            motion.phase.snap(0.0);
            motion.invalidate();
            motion.frame = self.motion_frame.wrapping_sub(1);
        }
        motion
            .phase
            .retarget(1.0, self.motion.popup, self.motion.easing);
        if motion.frame != self.motion_frame {
            motion.phase.advance(self.dt);
            motion.frame = self.motion_frame;
        }
        motion.start = Some((
            self.overlay.vertex_count(),
            self.overlay.index_count(),
            self.font.atlas_generation(),
        ));
    }
    pub(crate) fn finish_popup_motion(&mut self) {
        if !self.motion.active() {
            return;
        }
        let motion = &mut self.popup_animation;
        if let Some((vertices, indices, generation)) = motion.start.take() {
            motion.emitted = true;
            if generation == self.font.atlas_generation() {
                motion.snapshot.copy_tail(&self.overlay, vertices, indices);
                motion.generation = generation;
            } else {
                motion.invalidate();
            }
            self.overlay.fade_since(vertices, motion.phase.value());
        }
    }
    pub(crate) fn draw_popup_exit(&mut self) {
        for (i, slot) in self.routing.popups.iter_mut().enumerate() {
            if i < self.routing.count {
                continue;
            }
            if self.motion.active()
                && slot.motion.generation == self.font.atlas_generation()
                && slot.motion.phase.value() > 0.
            {
                self.overlay
                    .append_faded(&slot.motion.snapshot, slot.motion.phase.value());
            } else {
                slot.motion.invalidate();
            }
        }
    }
    pub(crate) fn scroll_position(&mut self, id: Id, offset: f32, max: f32) -> f32 {
        if !self.motion.active() {
            self.scroll_snap(id, offset);
            return offset;
        }
        let target = match self.animations.get(&(id, SCROLL)) {
            Some(Channel {
                value: Value::Spring(s),
                ..
            }) => s.target().clamp(0.0, max),
            _ => offset,
        };
        self.motion_spring(id, SCROLL, offset, target, self.motion.scroll)
            .clamp(0.0, max)
    }
    pub(crate) fn scroll_target(&mut self, id: Id, offset: f32, delta: f32, max: f32) -> f32 {
        if !self.motion.active() || self.motion.scroll == 0.0 {
            self.scroll_snap(id, (offset - delta).clamp(0.0, max));
            return (offset - delta).clamp(0.0, max);
        }
        let state = self.animations.entry((id, SCROLL)).or_insert(Channel {
            value: Value::Spring(Spring::new(offset)),
            frame: self.motion_frame,
        });
        if let Value::Spring(spring) = &mut state.value {
            spring.retarget((spring.target() - delta).clamp(0.0, max));
        }
        offset
    }
    pub(crate) fn scroll_snap(&mut self, id: Id, offset: f32) {
        if let Some(Channel {
            value: Value::Spring(s),
            frame,
        }) = self.animations.get_mut(&(id, SCROLL))
        {
            s.snap(offset);
            *frame = self.motion_frame;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn easing_endpoints_and_bounds() {
        for curve in [
            Easing::Linear,
            Easing::SmoothStep,
            Easing::SmootherStep,
            Easing::QuadIn,
            Easing::QuadOut,
            Easing::CubicInOut,
            Easing::CubicOut,
        ] {
            assert_eq!(curve.sample(-1.0), 0.0);
            assert_eq!(curve.sample(2.0), 1.0);
            assert_eq!(curve.sample(f32::NAN), 0.0);
            let mut previous = 0.0;
            for i in 0..=100 {
                let t = curve.sample(i as f32 / 100.0);
                assert!(t >= previous && t <= 1.0);
                previous = t;
            }
        }
    }
    #[test]
    fn tween_reversal_and_repeated_target_do_not_jump_or_restart() {
        let mut t = Tween::new(0.0);
        t.retarget(1.0, 1.0, Easing::Linear);
        assert_eq!(t.advance(0.4), 0.4);
        t.retarget(1.0, 1.0, Easing::Linear);
        assert_eq!(t.advance(0.1), 0.5);
        t.retarget(0.0, 1.0, Easing::Linear);
        assert_eq!(t.value(), 0.5);
        assert_eq!(t.advance(0.5), 0.25);
        assert_eq!(t.advance(-1.0), 0.25);
        assert_eq!(t.advance(f32::NAN), 0.25);
        assert_eq!(t.advance(5.0), 0.0);
        assert!(t.is_settled());
    }
    #[test]
    fn spring_preserves_velocity_and_matches_different_frame_rates() {
        let mut a = Spring::new(0.0);
        a.retarget(1.0);
        a.advance(0.04, 0.2);
        let velocity = a.velocity();
        let position = a.value();
        a.retarget(-1.0);
        assert_eq!(a.velocity(), velocity);
        assert_eq!(a.value(), position);
        let mut b = Spring::new(0.0);
        let mut c = b;
        b.retarget(1.0);
        c.retarget(1.0);
        for _ in 0..30 {
            b.advance(1.0 / 60.0, 0.2);
        }
        for _ in 0..72 {
            c.advance(1.0 / 144.0, 0.2);
        }
        assert!((b.value() - c.value()).abs() < 0.0001);
        assert_eq!(a.advance(10.0, 0.2), -1.0);
        assert_eq!(a.velocity(), 0.0);
    }
    #[test]
    fn colors_include_alpha_and_exact_endpoints() {
        assert_eq!(lerp_color([0; 4], [255; 4], 0.5), [128; 4]);
        assert_eq!(lerp_color([10; 4], [255; 4], 0.0), [10; 4]);
        assert_eq!(lerp_color([10; 4], [255; 4], 1.0), [255; 4]);
    }
    #[test]
    fn finite_scalar_endpoints_do_not_overflow_interpolation() {
        let mut tween = Tween::new(-f32::MAX);
        tween.retarget(f32::MAX, 1.0, Easing::Linear);
        assert_eq!(tween.advance(0.5), 0.0);
        let mut spring = Spring::new(-f32::MAX);
        spring.retarget(f32::MAX);
        for _ in 0..10 {
            assert!(spring.advance(0.001, 0.01).is_finite());
            assert!(spring.velocity().is_finite());
        }
    }
    #[test]
    fn pulse_wraps_and_has_smooth_extrema() {
        assert_eq!(pulse(0.0, 1.0), 0.0);
        assert_eq!(pulse(0.5, 1.0), 1.0);
        assert_eq!(pulse(1.0, 1.0), 0.0);
        assert_eq!(pulse(-0.5, 1.0), 1.0);
        assert_eq!(pulse(1.0, 0.0), 1.0);
    }
    #[test]
    fn scroll_targets_accumulate_and_direct_input_cancels_motion() {
        let mut ctx = crate::widgets::test_support::test_context();
        ctx.set_motion_settings(MotionSettings {
            enabled: true,
            ..MotionSettings::default()
        });
        let id = Id(88);
        ctx.new_frame(0.02);
        assert_eq!(ctx.scroll_position(id, 0.0, 100.0), 0.0);
        assert_eq!(ctx.scroll_target(id, 0.0, -30.0, 100.0), 0.0);
        assert_eq!(ctx.scroll_target(id, 0.0, -40.0, 100.0), 0.0);
        let Channel {
            value: Value::Spring(s),
            ..
        } = &ctx.animations[&(id, SCROLL)]
        else {
            panic!("scroll must use spring state");
        };
        assert_eq!(s.target(), 70.0);
        ctx.render();
        ctx.new_frame(0.02);
        let current = ctx.scroll_position(id, 0.0, 100.0);
        assert!(current > 0.0 && current < 70.0);
        ctx.scroll_snap(id, 11.0);
        assert_eq!(ctx.scroll_position(id, current, 100.0), 11.0);
        let Channel {
            value: Value::Spring(s),
            ..
        } = &ctx.animations[&(id, SCROLL)]
        else {
            panic!("scroll must use spring state");
        };
        assert_eq!(s.target(), 11.0);
        assert_eq!(s.velocity(), 0.0);
        ctx.render();
        ctx.new_frame(0.02);
        assert_eq!(ctx.scroll_position(id, 11.0, 5.0), 5.0);
        ctx.render();
    }
    #[test]
    fn reduced_motion_finishes_pending_scroll_destination() {
        let mut ctx = crate::widgets::test_support::test_context();
        ctx.set_motion_settings(MotionSettings {
            enabled: true,
            ..MotionSettings::default()
        });
        let mut id = Id(0);
        ctx.new_frame(0.02);
        ctx.ui().window("scroll", crate::ScrollFlags::BAR, |ui| {
            id = ui.ctx.current_window_mut().id;
            for _ in 0..30 {
                ui.label("Row");
            }
        });
        ctx.render();
        ctx.scroll_target(id, 0.0, -70.0, 100.0);
        ctx.set_motion_settings(MotionSettings {
            enabled: true,
            reduced_motion: true,
            ..MotionSettings::default()
        });
        assert_eq!(
            ctx.windows
                .iter()
                .find(|window| window.id == id)
                .unwrap()
                .scroll_offset,
            70.0
        );
    }
    #[test]
    fn atlas_replacement_invalidates_popup_exit_geometry() {
        let mut ctx = crate::widgets::test_support::test_context();
        ctx.set_motion_settings(MotionSettings {
            enabled: true,
            ..MotionSettings::default()
        });
        let id = Id(22);
        ctx.new_frame(0.2);
        ctx.ui().window("motion", crate::ScrollFlags::NONE, |ui| {
            ui.open_popup(id);
            ui.popup(id, [400.0, 80.0, 600.0, 160.0], |ui| {
                ui.label("Popup");
            });
        });
        ctx.render();
        ctx.new_frame(0.2);
        ctx.ui().window("motion", crate::ScrollFlags::NONE, |ui| {
            ui.popup(id, [400.0, 80.0, 600.0, 160.0], |ui| {
                ui.label("Popup");
                ui.close_popup();
            });
        });
        ctx.render();
        assert!(ctx.routing.popups[0].motion.snapshot.vertex_count() > 0);
        let font = crate::widgets::test_support::test_context().font;
        ctx.replace_font(font);
        assert_eq!(ctx.routing.popups[0].motion.snapshot.vertex_count(), 0);
    }
    #[test]
    fn closing_content_releases_focus_and_opening_fold_rejects_presses() {
        use crate::context::WidgetState;
        use crate::input::InputState;
        let mut ctx = crate::widgets::test_support::test_context();
        ctx.set_motion_settings(MotionSettings {
            enabled: true,
            easing: Easing::Linear,
            ..MotionSettings::default()
        });
        let mut text = alloc::string::String::from("Editor");
        let mut button_rect = [0.0; 4];
        for frame in 0..4 {
            let click = frame == 3;
            let mouse = if click {
                [
                    (button_rect[0] + button_rect[2]) * 0.5,
                    (button_rect[1] + button_rect[3]) * 0.5,
                ]
            } else {
                [-1000.0; 2]
            };
            ctx.set_input(InputState {
                mouse_pos: mouse,
                mouse_down: [click, false, false, false, false],
                ..InputState::default()
            });
            ctx.new_frame(if frame == 2 {
                0.15
            } else if click {
                0.01
            } else {
                0.2
            });
            ctx.ui().window("motion", crate::ScrollFlags::NONE, |ui| {
                let header = ui.widget_id("Reveal");
                ui.ctx
                    .widget_states
                    .insert(header, WidgetState::CollapsingHeader { open: frame != 2 });
                ui.collapsing_header("Reveal", |ui| {
                    if frame == 0 {
                        let editor = ui.widget_id("Editor");
                        ui.focus(editor);
                    }
                    ui.text_input("Editor", &mut text);
                    ui.button("Hidden button");
                    button_rect = ui.ctx.last_item.expect("button registered").1;
                });
            });
            ctx.render();
            if frame < 2 {
                assert!(ctx.wants_keyboard_capture());
            } else {
                assert!(!ctx.wants_keyboard_capture());
                assert_eq!(ctx.active, None);
                assert_eq!(ctx.focused, None);
            }
        }
    }
}
