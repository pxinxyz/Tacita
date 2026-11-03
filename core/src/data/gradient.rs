use super::{
    editor_canvas::{distance_squared, marker, Canvas, SEPARATION},
    DataError,
};
use crate::{
    animation::{lerp_color, GradientOptions, GradientSpread, GradientStop},
    input::Key,
    ui::Ui,
};
use alloc::vec::Vec;

pub const MAX_EDITOR_STOPS: usize = 16;
#[derive(Clone, Copy)]
struct Drag {
    index: usize,
    original: GradientStop,
    grab: f32,
}
pub struct GradientEditor {
    stops: Vec<GradientStop>,
    selected: Option<usize>,
    drag: Option<Drag>,
}
impl Default for GradientEditor {
    fn default() -> Self {
        Self::new(&[
            GradientStop {
                position: 0.,
                color: [99, 91, 255, 255],
            },
            GradientStop {
                position: 1.,
                color: [0, 229, 255, 255],
            },
        ])
        .expect("valid endpoints")
    }
}
impl GradientEditor {
    pub fn new(stops: &[GradientStop]) -> Result<Self, DataError> {
        let mut s = Self {
            stops: Vec::with_capacity(MAX_EDITOR_STOPS),
            selected: None,
            drag: None,
        };
        s.replace(stops)?;
        Ok(s)
    }
    pub fn stops(&self) -> &[GradientStop] {
        &self.stops
    }
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }
    pub fn select(&mut self, index: Option<usize>) -> Result<(), DataError> {
        if index.is_some_and(|i| i >= self.stops.len()) {
            return Err(DataError::InvalidInput);
        }
        self.selected = index;
        self.drag = None;
        Ok(())
    }
    pub fn replace(&mut self, stops: &[GradientStop]) -> Result<(), DataError> {
        if stops.len() > MAX_EDITOR_STOPS
            || !crate::animation::gradient::valid_stops_with(stops.len(), |i| stops[i])
            || stops
                .windows(2)
                .any(|p| p[1].position - p[0].position < SEPARATION)
        {
            return Err(DataError::InvalidInput);
        }
        self.stops.clear();
        self.stops.extend_from_slice(stops);
        self.selected = None;
        self.drag = None;
        Ok(())
    }
    pub fn sample(&self, position: f32) -> Result<[u8; 4], DataError> {
        if !position.is_finite() {
            return Err(DataError::InvalidInput);
        }
        let p = position.clamp(0., 1.);
        let i = self
            .stops
            .partition_point(|s| s.position <= p)
            .clamp(1, self.stops.len() - 1);
        let a = self.stops[i - 1];
        let b = self.stops[i];
        Ok(lerp_color(
            a.color,
            b.color,
            (p - a.position) / (b.position - a.position),
        ))
    }
    pub fn insert(&mut self, position: f32) -> Result<usize, DataError> {
        if !position.is_finite()
            || !(0.0..=1.0).contains(&position)
            || self.stops.len() == MAX_EDITOR_STOPS
            || self
                .stops
                .iter()
                .any(|s| (s.position - position).abs() < SEPARATION)
        {
            return Err(DataError::InvalidInput);
        }
        let color = self.sample(position)?;
        let index = self.stops.partition_point(|s| s.position < position);
        self.stops.insert(index, GradientStop { position, color });
        self.selected = Some(index);
        self.drag = None;
        Ok(index)
    }
    pub fn remove(&mut self, index: usize) -> Result<(), DataError> {
        if index == 0 || index >= self.stops.len() - 1 {
            return Err(DataError::InvalidInput);
        }
        self.stops.remove(index);
        self.selected = Some(index.min(self.stops.len() - 1));
        self.drag = None;
        Ok(())
    }
    pub fn set_stop(&mut self, index: usize, stop: GradientStop) -> Result<bool, DataError> {
        if index >= self.stops.len()
            || !stop.position.is_finite()
            || index == 0 && stop.position != 0.
            || index + 1 == self.stops.len() && stop.position != 1.
            || index > 0 && stop.position - self.stops[index - 1].position < SEPARATION
            || index + 1 < self.stops.len()
                && self.stops[index + 1].position - stop.position < SEPARATION
        {
            return Err(DataError::InvalidInput);
        }
        let changed = stop != self.stops[index];
        self.stops[index] = stop;
        self.drag = None;
        Ok(changed)
    }
}
impl Ui<'_> {
    pub fn gradient_editor(
        &mut self,
        label: &str,
        model: &mut GradientEditor,
        size: [f32; 2],
    ) -> Result<bool, DataError> {
        let canvas = Canvas::begin(self, label, size, false, true)?;
        self.ctx.id_stack.push_id(canvas.id);
        let mut changed = false;
        if !self.is_captured(canvas.id) {
            model.drag = None;
        }
        if canvas.key(self, Key::Escape) {
            if let Some(d) = model.drag.take() {
                changed |= model.stops[d.index] != d.original;
                model.stops[d.index] = d.original;
                self.release_capture(canvas.id);
            }
        } else if self.is_captured(canvas.id) {
            if !self.mouse_down(0) {
                model.drag = None;
                self.release_capture(canvas.id);
            } else if let Some(d) = model.drag {
                let low = model.stops[d.index - 1].position + SEPARATION * 2.;
                let high = model.stops[d.index + 1].position - SEPARATION * 2.;
                let p = if low <= high {
                    (canvas.value(self.mouse_pos())[0] - d.grab).clamp(low, high)
                } else {
                    d.original.position
                };
                changed |= p != model.stops[d.index].position;
                model.stops[d.index].position = p;
            }
        }
        let marker_y = canvas.rect[3] - 5.;
        let mouse = self.mouse_pos();
        let picked = model
            .stops
            .iter()
            .enumerate()
            .map(|(i, s)| {
                (
                    i,
                    distance_squared(mouse, [canvas.point(s.position, 0.)[0], marker_y]),
                )
            })
            .filter(|(_, d)| *d <= 81.)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        if canvas.pressed(self, 0) {
            self.focus(canvas.id);
            if let Some(i) = picked {
                model.selected = Some(i);
                if i > 0 && i + 1 < model.stops.len() {
                    model.drag = Some(Drag {
                        index: i,
                        original: model.stops[i],
                        grab: canvas.value(self.mouse_pos())[0] - model.stops[i].position,
                    });
                    self.capture(canvas.id);
                }
            } else if self.double_clicked(canvas.id, canvas.rect, 0) {
                changed |= model.insert(canvas.value(self.mouse_pos())[0]).is_ok();
            }
        }
        if canvas.pressed(self, 1) {
            if let Some(i) = picked {
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
            let preview = [
                canvas.rect[0],
                canvas.rect[1],
                canvas.rect[2],
                canvas.rect[3] - 16.,
            ];
            crate::widgets::draw_parts(self.ctx)
                .1
                .push_clip_rect(canvas.rect);
            for y in 0..4 {
                for x in 0..32 {
                    let a = [
                        preview[0] + (preview[2] - preview[0]) * x as f32 / 32.,
                        preview[1] + (preview[3] - preview[1]) * y as f32 / 4.,
                    ];
                    let b = [
                        preview[0] + (preview[2] - preview[0]) * (x + 1) as f32 / 32.,
                        preview[1] + (preview[3] - preview[1]) * (y + 1) as f32 / 4.,
                    ];
                    self.draw_rect_filled(
                        a,
                        b,
                        if (x + y) % 2 == 0 {
                            self.ctx.style.window_bg
                        } else {
                            self.ctx.style.scrollbar_bg
                        },
                    );
                }
            }
            self.draw_gradient(
                preview,
                &model.stops,
                GradientOptions {
                    spread: GradientSpread::Clamp,
                    resolution: 64,
                    ..GradientOptions::default()
                },
            );
            crate::widgets::draw_parts(self.ctx).1.pop_clip_rect();
            for (i, stop) in model.stops.iter().enumerate() {
                marker(
                    self,
                    [canvas.point(stop.position, 0.)[0], marker_y],
                    stop.color,
                    model.selected == Some(i),
                );
            }
        }
        let id = canvas.id;
        canvas.close(self);
        let add_scope = self.begin_disabled("##add_limit", model.stops.len() == MAX_EDITOR_STOPS);
        if self.button("Add stop") {
            if let Some(pair) = model.stops.windows(2).max_by(|a, b| {
                (a[1].position - a[0].position).total_cmp(&(b[1].position - b[0].position))
            }) {
                changed |= model
                    .insert((pair[0].position + pair[1].position) * 0.5)
                    .is_ok();
            }
        }
        self.end_disabled(add_scope);
        self.same_line();
        let delete_scope = self.begin_disabled(
            "##delete_limit",
            model
                .selected
                .is_none_or(|i| i == 0 || i + 1 == model.stops.len()),
        );
        if self.button("Delete stop") {
            if let Some(i) = model.selected {
                changed |= model.remove(i).is_ok();
                self.release_capture(id);
            }
        }
        self.end_disabled(delete_scope);
        if let Some(i) = model.selected {
            self.ctx.id_stack.push_id(crate::id::Id(i as u64));
            let mut color = model.stops[i].color;
            if self.color_picker_rgba("Stop color", &mut color) {
                model.stops[i].color = color;
                changed = true;
            }
            self.ctx.id_stack.pop();
        }
        self.ctx.id_stack.pop();
        Ok(changed)
    }
}
