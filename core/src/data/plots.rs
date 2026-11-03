use super::{editor_canvas::Canvas, DataError, MAX_DATA_ROWS};
use crate::ui::Ui;
use core::fmt::Write;

pub const MAX_PLOT_COLUMNS: usize = 512;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotOptions {
    pub size: [f32; 2],
    pub range: Option<[f32; 2]>,
    pub axes: bool,
    pub color: [u8; 4],
}
impl Default for PlotOptions {
    fn default() -> Self {
        Self {
            size: [400., 160.],
            range: None,
            axes: true,
            color: [124, 117, 255, 255],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotHover {
    pub index: usize,
    pub value: f32,
}
#[derive(Clone, Copy, Default)]
struct Bin {
    min: f32,
    max: f32,
    first: f32,
    last: f32,
    any: bool,
    gap: bool,
}
impl Ui<'_> {
    pub fn plot(
        &mut self,
        label: &str,
        samples: &[f32],
        options: PlotOptions,
    ) -> Result<Option<PlotHover>, DataError> {
        self.plot_with(label, samples.len(), |i| samples[i], options)
    }
    pub fn sparkline(
        &mut self,
        label: &str,
        samples: &[f32],
        size: [f32; 2],
        color: [u8; 4],
    ) -> Result<Option<PlotHover>, DataError> {
        self.plot(
            label,
            samples,
            PlotOptions {
                size,
                color,
                axes: false,
                ..PlotOptions::default()
            },
        )
    }
    pub fn plot_with(
        &mut self,
        label: &str,
        count: usize,
        get: impl Fn(usize) -> f32,
        options: PlotOptions,
    ) -> Result<Option<PlotHover>, DataError> {
        if count > MAX_DATA_ROWS
            || options
                .range
                .is_some_and(|r| !r.iter().all(|v| v.is_finite()) || r[0] >= r[1])
        {
            return Err(DataError::InvalidInput);
        }
        let canvas = Canvas::begin(self, label, options.size, options.axes, false)?;
        if !canvas.visible {
            canvas.close(self);
            return Ok(None);
        }
        let columns = ((canvas.rect[2] - canvas.rect[0]) as usize)
            .clamp(1, MAX_PLOT_COLUMNS)
            .min(count.max(1));
        let mut bins = [Bin::default(); MAX_PLOT_COLUMNS];
        let (mut low, mut high) = (f32::INFINITY, f32::NEG_INFINITY);
        for i in 0..count {
            let value = get(i);
            let bin = &mut bins[i * columns / count];
            if !value.is_finite() {
                bin.gap = true;
                continue;
            }
            low = low.min(value);
            high = high.max(value);
            if !bin.any {
                *bin = Bin {
                    min: value,
                    max: value,
                    first: value,
                    last: value,
                    any: true,
                    gap: bin.gap,
                };
            } else {
                bin.min = bin.min.min(value);
                bin.max = bin.max.max(value);
                bin.last = value;
            }
        }
        let range = options.range.unwrap_or_else(|| {
            if !low.is_finite() {
                [0., 1.]
            } else if low == high {
                let margin = (low as f64).abs().max(1.) * 0.05;
                [
                    (low as f64 - margin).max(-(f32::MAX as f64)) as f32,
                    (high as f64 + margin).min(f32::MAX as f64) as f32,
                ]
            } else {
                [low, high]
            }
        });
        let y = |v: f32| {
            (((v as f64 - range[0] as f64) / (range[1] as f64 - range[0] as f64)).clamp(0., 1.))
                as f32
        };
        crate::widgets::draw_parts(self.ctx)
            .1
            .push_clip_rect(canvas.rect);
        let mut previous = None;
        for (i, bin) in bins[..columns].iter().enumerate() {
            if !bin.any {
                previous = None;
                continue;
            }
            let x = if columns == 1 {
                0.5
            } else {
                i as f32 / (columns - 1) as f32
            };
            let first = canvas.point(x, y(bin.first));
            if !bin.gap {
                if let Some(p) = previous {
                    self.draw_line(p, first, options.color, 1.5);
                }
            }
            self.draw_line(
                canvas.point(x, y(bin.min)),
                canvas.point(x, y(bin.max)),
                options.color,
                1.5,
            );
            if count == 1 {
                super::editor_canvas::marker(self, first, options.color, false);
            }
            previous = if bin.gap {
                None
            } else {
                Some(canvas.point(x, y(bin.last)))
            };
        }
        let mut hover = None;
        if count > 0 && self.is_hovered(canvas.rect) {
            let t = canvas.value(self.mouse_pos())[0];
            let index = ((t * (count - 1) as f32 + 0.5) as usize).min(count - 1);
            let value = get(index);
            if value.is_finite() {
                hover = Some(PlotHover { index, value });
                let p = canvas.point(
                    if count == 1 {
                        0.5
                    } else {
                        index as f32 / (count - 1) as f32
                    },
                    y(value),
                );
                self.draw_line(
                    [p[0], canvas.rect[1]],
                    [p[0], canvas.rect[3]],
                    self.ctx.style.text,
                    1.,
                );
                super::editor_canvas::marker(self, p, options.color, true);
            }
        }
        crate::widgets::draw_parts(self.ctx).1.pop_clip_rect();
        if options.axes {
            let col = self.ctx.style.text;
            for (value, py) in [(range[1], canvas.rect[1]), (range[0], canvas.rect[3] - 12.)] {
                let mut text = crate::widgets::FmtBuf::<64>::new();
                let _ = write!(text, "{value:.2}");
                self.draw_text_at([canvas.rect[0] - 40., py], text.as_str(), col);
            }
            let mut text = crate::widgets::FmtBuf::<64>::new();
            if let Some(h) = hover {
                let _ = write!(text, "{}: {:.3}", h.index, h.value);
            } else {
                let _ = write!(text, "0 .. {}", count.saturating_sub(1));
            }
            self.draw_text_at([canvas.rect[0], canvas.rect[3] + 4.], text.as_str(), col);
        }
        canvas.close(self);
        Ok(hover)
    }
}
