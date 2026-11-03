use super::{
    editor_canvas::{distance_squared, marker, Canvas, SEPARATION},
    DataError,
};
use crate::{input::Key, ui::Ui};
use alloc::vec::Vec;

pub const MAX_CURVE_KEYS: usize = 64;
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CurveInterpolation {
    Step = 0,
    #[default]
    Linear = 1,
    Cubic = 2,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveKey {
    pub time: f32,
    pub value: f32,
    pub in_slope: f32,
    pub out_slope: f32,
    pub interpolation: CurveInterpolation,
}
impl CurveKey {
    pub fn linear(time: f32, value: f32) -> Self {
        Self {
            time,
            value,
            in_slope: 0.,
            out_slope: 0.,
            interpolation: CurveInterpolation::Linear,
        }
    }
    fn valid(self) -> bool {
        [self.time, self.value]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            && [self.in_slope, self.out_slope]
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 10000.)
    }
}
#[derive(Clone, Copy)]
struct Drag {
    index: usize,
    part: u8,
    original: CurveKey,
    grab: [f32; 2],
}
pub struct CurveEditor {
    keys: Vec<CurveKey>,
    selected: Option<usize>,
    drag: Option<Drag>,
}
impl Default for CurveEditor {
    fn default() -> Self {
        Self::new(&[CurveKey::linear(0., 0.), CurveKey::linear(1., 1.)]).expect("valid endpoints")
    }
}
impl CurveEditor {
    pub fn new(keys: &[CurveKey]) -> Result<Self, DataError> {
        let mut s = Self {
            keys: Vec::with_capacity(MAX_CURVE_KEYS),
            selected: None,
            drag: None,
        };
        s.replace(keys)?;
        Ok(s)
    }
    pub fn keys(&self) -> &[CurveKey] {
        &self.keys
    }
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }
    pub fn select(&mut self, index: Option<usize>) -> Result<(), DataError> {
        if index.is_some_and(|i| i >= self.keys.len()) {
            return Err(DataError::InvalidInput);
        }
        self.selected = index;
        self.drag = None;
        Ok(())
    }
    pub fn replace(&mut self, keys: &[CurveKey]) -> Result<(), DataError> {
        if keys.len() < 2
            || keys.len() > MAX_CURVE_KEYS
            || keys.iter().any(|k| !k.valid())
            || keys.windows(2).any(|p| p[1].time - p[0].time < SEPARATION)
        {
            return Err(DataError::InvalidInput);
        }
        self.keys.clear();
        self.keys.extend_from_slice(keys);
        self.selected = None;
        self.drag = None;
        Ok(())
    }
    pub fn set_key(&mut self, index: usize, key: CurveKey) -> Result<bool, DataError> {
        if index >= self.keys.len()
            || !key.valid()
            || index > 0 && key.time - self.keys[index - 1].time < SEPARATION
            || index + 1 < self.keys.len() && self.keys[index + 1].time - key.time < SEPARATION
        {
            return Err(DataError::InvalidInput);
        }
        let changed = self.keys[index] != key;
        self.keys[index] = key;
        self.drag = None;
        Ok(changed)
    }
    pub fn insert(&mut self, key: CurveKey) -> Result<usize, DataError> {
        if !key.valid()
            || self.keys.len() == MAX_CURVE_KEYS
            || self
                .keys
                .iter()
                .any(|k| (k.time - key.time).abs() < SEPARATION)
        {
            return Err(DataError::InvalidInput);
        }
        let index = self.keys.partition_point(|k| k.time < key.time);
        self.keys.insert(index, key);
        self.selected = Some(index);
        self.drag = None;
        Ok(index)
    }
    pub fn remove(&mut self, index: usize) -> Result<(), DataError> {
        if self.keys.len() <= 2 || index >= self.keys.len() {
            return Err(DataError::InvalidInput);
        }
        self.keys.remove(index);
        self.selected = Some(index.min(self.keys.len() - 1));
        self.drag = None;
        Ok(())
    }
    pub fn evaluate(&self, time: f32) -> Result<f32, DataError> {
        if !time.is_finite() {
            return Err(DataError::InvalidInput);
        }
        if time <= self.keys[0].time {
            return Ok(self.keys[0].value);
        }
        if time >= self.keys[self.keys.len() - 1].time {
            return Ok(self.keys[self.keys.len() - 1].value);
        }
        let right = self.keys.partition_point(|k| k.time <= time);
        let a = self.keys[right - 1];
        let b = self.keys[right];
        let dt = b.time - a.time;
        let t = (time - a.time) / dt;
        Ok(match a.interpolation {
            CurveInterpolation::Step => a.value,
            CurveInterpolation::Linear => a.value + (b.value - a.value) * t,
            CurveInterpolation::Cubic => {
                let t2 = t * t;
                let t3 = t2 * t;
                (2. * t3 - 3. * t2 + 1.) * a.value
                    + (t3 - 2. * t2 + t) * dt * a.out_slope
                    + (-2. * t3 + 3. * t2) * b.value
                    + (t3 - t2) * dt * b.in_slope
            }
        })
    }
    fn handle(&self, canvas: &Canvas, index: usize, part: u8) -> [f32; 2] {
        let key = self.keys[index];
        let dx = if part == 1 {
            -(key.time - self.keys[index - 1].time) / 3.
        } else {
            (self.keys[index + 1].time - key.time) / 3.
        };
        canvas.point(
            key.time + dx,
            (key.value
                + dx * if part == 1 {
                    key.in_slope
                } else {
                    key.out_slope
                })
            .clamp(0., 1.),
        )
    }
}
impl Ui<'_> {
    pub fn curve_editor(
        &mut self,
        label: &str,
        model: &mut CurveEditor,
        size: [f32; 2],
    ) -> Result<bool, DataError> {
        let canvas = Canvas::begin(self, label, size, true, true)?;
        self.ctx.id_stack.push_id(canvas.id);
        let mut changed = false;
        if !self.is_captured(canvas.id) {
            model.drag = None;
        }
        if canvas.key(self, Key::Escape) {
            if let Some(d) = model.drag.take() {
                changed |= model.keys[d.index] != d.original;
                model.keys[d.index] = d.original;
                self.release_capture(canvas.id);
            }
        } else if self.is_captured(canvas.id) {
            if !self.mouse_down(0) {
                model.drag = None;
                self.release_capture(canvas.id);
            } else if let Some(d) = model.drag {
                let mut key = model.keys[d.index];
                let mut value = canvas.value(self.mouse_pos());
                if d.part == 0 {
                    value[0] -= d.grab[0];
                    value[1] -= d.grab[1];
                    let low = if d.index == 0 {
                        0.
                    } else {
                        model.keys[d.index - 1].time + SEPARATION * 2.
                    };
                    let high = if d.index + 1 == model.keys.len() {
                        1.
                    } else {
                        model.keys[d.index + 1].time - SEPARATION * 2.
                    };
                    key.time = if low <= high {
                        value[0].clamp(low, high)
                    } else {
                        d.original.time
                    };
                    key.value = value[1].clamp(0., 1.);
                } else {
                    let dx = if d.part == 1 {
                        -(key.time - model.keys[d.index - 1].time) / 3.
                    } else {
                        (model.keys[d.index + 1].time - key.time) / 3.
                    };
                    let slope = ((value[1] - key.value) / dx).clamp(-10000., 10000.);
                    if d.part == 1 {
                        key.in_slope = slope;
                    } else {
                        key.out_slope = slope;
                    }
                }
                changed |= key != model.keys[d.index];
                model.keys[d.index] = key;
            }
        }
        let mouse = self.mouse_pos();
        let mut picked = None;
        let mut best = 64.;
        let mut consider = |index, part, point| {
            let distance = distance_squared(mouse, point);
            if distance <= best {
                best = distance;
                picked = Some((index, part));
            }
        };
        if let Some(i) = model.selected {
            if i > 0 && model.keys[i - 1].interpolation == CurveInterpolation::Cubic {
                consider(i, 1, model.handle(&canvas, i, 1));
            }
            if i + 1 < model.keys.len() && model.keys[i].interpolation == CurveInterpolation::Cubic
            {
                consider(i, 2, model.handle(&canvas, i, 2));
            }
        }
        for (i, key) in model.keys.iter().enumerate() {
            consider(i, 0, canvas.point(key.time, key.value));
        }
        if canvas.pressed(self, 0) {
            self.focus(canvas.id);
            if let Some((index, part)) = picked {
                model.selected = Some(index);
                let key = model.keys[index];
                let v = canvas.value(self.mouse_pos());
                model.drag = Some(Drag {
                    index,
                    part,
                    original: key,
                    grab: [v[0] - key.time, v[1] - key.value],
                });
                self.capture(canvas.id);
            } else if self.double_clicked(canvas.id, canvas.rect, 0) {
                let p = canvas.value(self.mouse_pos());
                changed |= model.insert(CurveKey::linear(p[0], p[1])).is_ok();
            }
        }
        if canvas.pressed(self, 1) {
            if let Some((i, 0)) = picked {
                changed |= model.remove(i).is_ok();
            }
        }
        if canvas.key(self, Key::Delete) {
            if let Some(i) = model.selected {
                changed |= model.remove(i).is_ok();
                self.release_capture(canvas.id);
            }
        }
        if canvas.visible {
            crate::widgets::draw_parts(self.ctx)
                .1
                .push_clip_rect(canvas.rect);
            let color = self.ctx.style.widget_bg_active;
            for pair in model.keys.windows(2) {
                let a = pair[0];
                let b = pair[1];
                if a.interpolation == CurveInterpolation::Step {
                    self.draw_line(
                        canvas.point(a.time, a.value),
                        canvas.point(b.time, a.value),
                        color,
                        1.5,
                    );
                    self.draw_line(
                        canvas.point(b.time, a.value),
                        canvas.point(b.time, b.value),
                        color,
                        1.5,
                    );
                } else {
                    let steps = if a.interpolation == CurveInterpolation::Linear {
                        1
                    } else {
                        24
                    };
                    let mut previous = canvas.point(a.time, a.value);
                    for j in 1..=steps {
                        let t = a.time + (b.time - a.time) * j as f32 / steps as f32;
                        let point = canvas.point(t, model.evaluate(t)?);
                        self.draw_line(previous, point, color, 1.5);
                        previous = point;
                    }
                }
            }
            crate::widgets::draw_parts(self.ctx).1.pop_clip_rect();
            for (i, key) in model.keys.iter().enumerate() {
                let p = canvas.point(key.time, key.value);
                if model.selected == Some(i) {
                    for part in 1..=2 {
                        if part == 1
                            && (i == 0
                                || model.keys[i - 1].interpolation != CurveInterpolation::Cubic)
                            || part == 2
                                && (i + 1 == model.keys.len()
                                    || key.interpolation != CurveInterpolation::Cubic)
                        {
                            continue;
                        }
                        let h = model.handle(&canvas, i, part);
                        self.draw_line(p, h, self.ctx.style.text, 1.);
                        marker(self, h, self.ctx.style.text, false);
                    }
                }
                marker(self, p, color, model.selected == Some(i));
            }
        }
        let id = canvas.id;
        canvas.close(self);
        let add_scope = self.begin_disabled("##add_limit", model.keys.len() == MAX_CURVE_KEYS);
        if self.button("Add key") {
            if let Some(pair) = model
                .keys
                .windows(2)
                .max_by(|a, b| (a[1].time - a[0].time).total_cmp(&(b[1].time - b[0].time)))
            {
                let t = (pair[0].time + pair[1].time) * 0.5;
                let value = model.evaluate(t)?.clamp(0., 1.);
                changed |= model.insert(CurveKey::linear(t, value)).is_ok();
            }
        }
        self.end_disabled(add_scope);
        self.same_line();
        let delete_scope = self.begin_disabled(
            "##delete_limit",
            model.keys.len() <= 2 || model.selected.is_none(),
        );
        if self.button("Delete key") {
            if let Some(i) = model.selected {
                changed |= model.remove(i).is_ok();
                self.release_capture(id);
            }
        }
        self.end_disabled(delete_scope);
        if let Some(i) = model.selected {
            let mut key = model.keys[i];
            let mut mode = key.interpolation as usize;
            if self.combo("Interpolation", &["step", "linear", "cubic"], &mut mode) {
                key.interpolation = match mode {
                    0 => CurveInterpolation::Step,
                    2 => CurveInterpolation::Cubic,
                    _ => CurveInterpolation::Linear,
                };
                changed |= model.set_key(i, key)?;
            }
        }
        self.ctx.id_stack.pop();
        Ok(changed)
    }
}
