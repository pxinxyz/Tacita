use crate::draw::{sin_cos_approx, sqrt_approx};

pub(crate) const MAX_POINTS: usize = 260;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundedRect {
    pub rect: [f32; 4],
    pub radii: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeError {
    InvalidGeometry,
    ScopeLimit,
}

impl RoundedRect {
    pub fn new(rect: [f32; 4], radii: [f32; 4]) -> Result<Self, ShapeError> {
        if !rect.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
            || rect[0] >= rect[2]
            || rect[1] >= rect[3]
            || !radii
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0 && *v <= 1_000_000.0)
        {
            return Err(ShapeError::InvalidGeometry);
        }
        let cap = ((rect[2] - rect[0]).min(rect[3] - rect[1])) * 0.5;
        Ok(Self {
            rect,
            radii: radii.map(|r| r.min(cap)),
        })
    }

    pub(crate) fn canonical(self) -> Result<Self, ShapeError> {
        Self::new(self.rect, self.radii)
    }

    pub fn distance(self, point: [f32; 2]) -> f32 {
        let x = point[0] - (self.rect[0] + self.rect[2]) * 0.5;
        let y = point[1] - (self.rect[1] + self.rect[3]) * 0.5;
        let r = self.radii[match (x >= 0.0, y >= 0.0) {
            (false, false) => 0,
            (true, false) => 1,
            (true, true) => 2,
            (false, true) => 3,
        }];
        let q = [
            x.abs() - (self.rect[2] - self.rect[0]) * 0.5 + r,
            y.abs() - (self.rect[3] - self.rect[1]) * 0.5 + r,
        ];
        sqrt_approx(q[0].max(0.0) * q[0].max(0.0) + q[1].max(0.0) * q[1].max(0.0))
            + q[0].max(q[1]).min(0.0)
            - r
    }

    pub fn contains(self, point: [f32; 2]) -> bool {
        self.distance(point) <= 0.0
    }

    pub(crate) fn intersects(self, rect: [f32; 4]) -> bool {
        let r = crate::draw::intersect_rects(self.rect, rect);
        if r[0] >= r[2] || r[1] >= r[3] {
            return false;
        }
        self.contains([
            ((self.rect[0] + self.rect[2]) * 0.5).clamp(r[0], r[2]),
            ((self.rect[1] + self.rect[3]) * 0.5).clamp(r[1], r[3]),
        ])
    }

    pub(crate) fn shifted(self, offset: [f32; 2]) -> Self {
        Self {
            rect: [
                self.rect[0] + offset[0],
                self.rect[1] + offset[1],
                self.rect[2] + offset[0],
                self.rect[3] + offset[1],
            ],
            ..self
        }
    }

    pub(crate) fn contour(
        self,
        distance: f32,
        steps: usize,
        out: &mut [[f32; 2]; MAX_POINTS],
    ) -> usize {
        let r = [
            self.rect[0] - distance,
            self.rect[1] - distance,
            self.rect[2] + distance,
            self.rect[3] + distance,
        ];
        if r[0] >= r[2] || r[1] >= r[3] {
            let center = [
                (self.rect[0] + self.rect[2]) * 0.5,
                (self.rect[1] + self.rect[3]) * 0.5,
            ];
            out[..4 * (steps + 1)].fill(center);
            return 4 * (steps + 1);
        }
        let cap = (r[2] - r[0]).min(r[3] - r[1]) * 0.5;
        let radii = self.radii.map(|v| (v + distance).max(0.0).min(cap));
        let mut count = 0;
        for (i, radius) in radii.into_iter().enumerate() {
            let center = match i {
                0 => [r[0] + radius, r[1] + radius],
                1 => [r[2] - radius, r[1] + radius],
                2 => [r[2] - radius, r[3] - radius],
                _ => [r[0] + radius, r[3] - radius],
            };
            for step in 0..=steps {
                let angle = core::f32::consts::PI
                    + (i as f32 + step as f32 / steps as f32) * core::f32::consts::FRAC_PI_2;
                let (s, c) = sin_cos_approx(angle);
                out[count] = [center[0] + c * radius, center[1] + s * radius];
                count += 1;
            }
        }
        count
    }
    pub(crate) fn steps(self, scale: f32, spread: f32) -> usize {
        (crate::draw::circle_segment_count(
            (self.radii.into_iter().fold(0.0_f32, f32::max) + spread) * scale,
        ) / 4)
            .clamp(2, 64)
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BorderPlacement {
    Inner = 0,
    #[default]
    Center = 1,
    Outer = 2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub offset: [f32; 2],
    pub spread: f32,
    pub blur: f32,
    pub color: [u8; 4],
    pub inset: bool,
}
impl Shadow {
    pub fn is_valid(self) -> bool {
        self.offset
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 4096.0)
            && self.spread.is_finite()
            && (0.0..=4096.0).contains(&self.spread)
            && self.blur.is_finite()
            && (0.0..=4096.0).contains(&self.blur)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corner_distances_and_clamping_are_independent() {
        let r = RoundedRect::new([0.0, 0.0, 100.0, 60.0], [80.0, 0.0, 8.0, 12.0]).unwrap();
        assert_eq!(r.radii, [30.0, 0.0, 8.0, 12.0]);
        assert!(!r.contains([1.0, 1.0]));
        assert!(r.contains([99.0, 1.0]));
        assert!(r.contains([50.0, 30.0]));
        assert!((r.distance([50.0, -1.0]) - 1.0).abs() < 0.0001);
        assert!(RoundedRect::new([0.0; 4], [0.0; 4]).is_err());
        assert!(RoundedRect::new([0.0, 0.0, 10.0, 10.0], [f32::NAN; 4]).is_err());
    }
    #[test]
    fn contours_are_bounded_and_clockwise() {
        let r = RoundedRect::new([10.0, 20.0, 110.0, 80.0], [30.0, 0.0, 8.0, 12.0]).unwrap();
        let mut points = [[0.0; 2]; MAX_POINTS];
        let n = r.contour(0.0, 64, &mut points);
        assert_eq!(n, MAX_POINTS);
        for p in &points[..n] {
            assert!(r.distance(*p).abs() < 0.0001);
        }
        let area = (0..n)
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % n];
                a[0] * b[1] - a[1] * b[0]
            })
            .sum::<f32>();
        assert!(area > 0.0);
    }
}
