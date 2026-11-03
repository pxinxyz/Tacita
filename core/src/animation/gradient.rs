use super::lerp_color;
use crate::ui::Ui;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub position: f32,
    pub color: [u8; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientSpread {
    Clamp,
    Repeat,
    Mirror,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientOptions {
    pub axis: [f32; 2],
    pub spread: GradientSpread,
    pub resolution: u32,
}
impl Default for GradientOptions {
    fn default() -> Self {
        Self {
            axis: [1.0, 0.0],
            spread: GradientSpread::Mirror,
            resolution: 24,
        }
    }
}
impl GradientOptions {
    pub fn is_valid(&self) -> bool {
        self.axis.iter().all(|v| v.is_finite())
            && self.axis != [0.0, 0.0]
            && (1..=64).contains(&self.resolution)
    }
}
pub fn valid_stops_with(count: usize, get: impl Fn(usize) -> GradientStop) -> bool {
    if count < 2 {
        return false;
    }
    let mut previous = -1.0;
    for i in 0..count {
        let p = get(i).position;
        if !p.is_finite() || !(0.0..=1.0).contains(&p) || p <= previous {
            return false;
        }
        previous = p;
    }
    get(0).position == 0.0 && previous == 1.0
}
fn spread(position: f32, mode: GradientSpread) -> f32 {
    match mode {
        GradientSpread::Clamp => position.clamp(0.0, 1.0),
        GradientSpread::Repeat | GradientSpread::Mirror => {
            let period = if mode == GradientSpread::Repeat {
                1.0
            } else {
                2.0
            };
            let mut p = position % period;
            if p < 0.0 {
                p += period;
            }
            if p <= 1.0 {
                p
            } else {
                2.0 - p
            }
        }
    }
}
fn sample(count: usize, get: &impl Fn(usize) -> GradientStop, position: f32) -> [u8; 4] {
    let (mut lo, mut hi) = (0, count - 1);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if get(mid).position <= position {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let (a, b) = (get(lo), get(hi));
    lerp_color(
        a.color,
        b.color,
        (position - a.position) / (b.position - a.position),
    )
}
impl Ui<'_> {
    pub fn draw_gradient(
        &mut self,
        rect: [f32; 4],
        stops: &[GradientStop],
        options: GradientOptions,
    ) {
        self.draw_gradient_with(rect, stops.len(), |i| stops[i], options, 0.0);
    }
    pub fn draw_animated_gradient(
        &mut self,
        rect: [f32; 4],
        stops: &[GradientStop],
        options: GradientOptions,
        time: f32,
        period: f32,
    ) {
        self.draw_animated_gradient_with(rect, stops.len(), |i| stops[i], options, time, period);
    }
    pub fn draw_animated_gradient_with(
        &mut self,
        rect: [f32; 4],
        count: usize,
        get: impl Fn(usize) -> GradientStop,
        options: GradientOptions,
        time: f32,
        period: f32,
    ) {
        assert!(
            time.is_finite() && period.is_finite() && period > 0.0,
            "invalid gradient clock"
        );
        let cycle = match options.spread {
            GradientSpread::Mirror => 2.0,
            _ => 1.0,
        };
        let phase = if self.ctx.motion.active() {
            (time % period) / period * cycle
        } else {
            0.0
        };
        self.draw_gradient_with(rect, count, get, options, phase);
    }
    pub fn draw_gradient_with(
        &mut self,
        rect: [f32; 4],
        count: usize,
        get: impl Fn(usize) -> GradientStop,
        options: GradientOptions,
        phase: f32,
    ) {
        assert!(
            super::layout::valid_rect(rect)
                && options.is_valid()
                && phase.is_finite()
                && valid_stops_with(count, &get),
            "invalid gradient"
        );
        let scale = options.axis[0].abs().max(options.axis[1].abs());
        let axis = [options.axis[0] / scale, options.axis[1] / scale];
        let norm = axis[0].abs() + axis[1].abs();
        let nx = if axis[0] == 0.0 {
            1
        } else {
            options.resolution
        };
        let ny = if axis[1] == 0.0 {
            1
        } else {
            options.resolution
        };
        crate::widgets::draw_parts(self.ctx).1.add_gradient_grid(
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            nx,
            ny,
            |x, y| {
                let p = 0.5
                    + ((x as f32 / nx as f32 - 0.5) * axis[0]
                        + (y as f32 / ny as f32 - 0.5) * axis[1])
                        / norm;
                sample(count, &get, spread(p + phase, options.spread))
            },
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spreads_and_stop_interpolation_preserve_alpha() {
        let get = |i| {
            [
                GradientStop {
                    position: 0.0,
                    color: [0; 4],
                },
                GradientStop {
                    position: 1.0,
                    color: [200; 4],
                },
            ][i]
        };
        assert!(valid_stops_with(2, get));
        assert_eq!(sample(2, &get, 0.5), [100; 4]);
        assert_eq!(spread(-0.25, GradientSpread::Repeat), 0.75);
        assert_eq!(spread(1.25, GradientSpread::Mirror), 0.75);
        assert!(!GradientOptions {
            axis: [0.0; 2],
            ..GradientOptions::default()
        }
        .is_valid());
    }
}
