use super::{length, Edge, SvgError, MAX_EDGES};
use crate::draw::{sin_cos_approx, sqrt_approx};
use alloc::vec::Vec;

pub(super) struct Numbers<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) pos: usize,
}
impl<'a> Numbers<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            pos: 0,
        }
    }
    pub(super) fn skip(&mut self) {
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_whitespace() || self.bytes[self.pos] == b',')
        {
            self.pos += 1;
        }
    }
    pub(super) fn done(&mut self) -> bool {
        self.skip();
        self.pos == self.bytes.len()
    }
    pub(super) fn number(&mut self) -> Result<f32, SvgError> {
        self.skip();
        let start = self.pos;
        if self
            .bytes
            .get(self.pos)
            .is_some_and(|c| *c == b'+' || *c == b'-')
        {
            self.pos += 1;
        }
        let mut digits = 0;
        while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
            digits += 1;
            self.pos += 1;
        }
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
                digits += 1;
                self.pos += 1;
            }
        }
        if digits == 0 {
            return Err(SvgError::InvalidInput);
        }
        if self
            .bytes
            .get(self.pos)
            .is_some_and(|c| *c == b'e' || *c == b'E')
        {
            self.pos += 1;
            if self
                .bytes
                .get(self.pos)
                .is_some_and(|c| *c == b'+' || *c == b'-')
            {
                self.pos += 1;
            }
            let exponent = self.pos;
            while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
                self.pos += 1;
            }
            if exponent == self.pos {
                return Err(SvgError::InvalidInput);
            }
        }
        let text = core::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| SvgError::InvalidInput)?;
        let value = text.parse::<f32>().map_err(|_| SvgError::InvalidInput)?;
        if !value.is_finite() || value.abs() > 1_000_000.0 {
            return Err(SvgError::InvalidInput);
        }
        Ok(value)
    }
    fn flag(&mut self) -> Result<bool, SvgError> {
        self.skip();
        let c = *self.bytes.get(self.pos).ok_or(SvgError::InvalidInput)?;
        if c != b'0' && c != b'1' {
            return Err(SvgError::InvalidInput);
        }
        self.pos += 1;
        Ok(c == b'1')
    }
    fn pair(&mut self, base: [f32; 2]) -> Result<[f32; 2], SvgError> {
        Ok([self.number()? + base[0], self.number()? + base[1]])
    }
}

pub(crate) struct Path {
    pub(crate) edges: Vec<Edge>,
    current: [f32; 2],
    start: [f32; 2],
    first: usize,
    live: bool,
    tolerance: f32,
}
impl Path {
    pub(crate) fn new(tolerance: f32) -> Self {
        Self {
            edges: Vec::new(),
            current: [0.0; 2],
            start: [0.0; 2],
            first: 0,
            live: false,
            tolerance,
        }
    }
    pub(crate) fn finish_contour(&mut self, closed: bool) -> Result<(), SvgError> {
        if !self.live {
            return Ok(());
        }
        if self.edges.len() > self.first {
            if closed {
                self.line(self.start)?;
            } else {
                self.edges[self.first].first = true;
                if let Some(last) = self.edges.last_mut() {
                    last.last = true;
                }
                self.edge(self.current, self.start, false)?;
            }
        }
        self.current = self.start;
        self.live = false;
        Ok(())
    }
    pub(crate) fn moveto(&mut self, p: [f32; 2]) -> Result<(), SvgError> {
        self.finish_contour(false)?;
        self.current = p;
        self.start = p;
        self.first = self.edges.len();
        self.live = true;
        Ok(())
    }
    fn edge(&mut self, a: [f32; 2], b: [f32; 2], stroke: bool) -> Result<(), SvgError> {
        if self.edges.len() >= MAX_EDGES {
            return Err(SvgError::TooComplex);
        }
        if a.iter()
            .chain(b.iter())
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
        {
            return Err(SvgError::InvalidInput);
        }
        if a != b || stroke {
            self.edges.push(Edge {
                a,
                b,
                stroke,
                first: false,
                last: false,
            });
        }
        Ok(())
    }
    pub(crate) fn line(&mut self, p: [f32; 2]) -> Result<(), SvgError> {
        if !self.live {
            self.start = self.current;
            self.first = self.edges.len();
            self.live = true;
        }
        self.edge(self.current, p, true)?;
        self.current = p;
        Ok(())
    }
    fn cubic(
        &mut self,
        a: [f32; 2],
        b: [f32; 2],
        c: [f32; 2],
        d: [f32; 2],
        depth: u8,
    ) -> Result<(), SvgError> {
        let chord = length([d[0] - a[0], d[1] - a[1]]);
        let polygon = length([b[0] - a[0], b[1] - a[1]])
            + length([c[0] - b[0], c[1] - b[1]])
            + length([d[0] - c[0], d[1] - c[1]]);
        let deviation = if chord > 0.0 {
            let perpendicular = |p: [f32; 2]| {
                ((d[0] - a[0]) * (p[1] - a[1]) - (d[1] - a[1]) * (p[0] - a[0])).abs() / chord
            };
            perpendicular(b).max(perpendicular(c))
        } else {
            length([b[0] - a[0], b[1] - a[1]]).max(length([c[0] - a[0], c[1] - a[1]]))
        };
        if deviation <= self.tolerance && polygon - chord <= self.tolerance {
            return self.line(d);
        }
        if depth >= 12 {
            return Err(SvgError::TooComplex);
        }
        let ab = mid(a, b);
        let bc = mid(b, c);
        let cd = mid(c, d);
        let abc = mid(ab, bc);
        let bcd = mid(bc, cd);
        let center = mid(abc, bcd);
        self.cubic(a, ab, abc, center, depth + 1)?;
        self.cubic(center, bcd, cd, d, depth + 1)
    }
    pub(crate) fn curve(&mut self, b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> Result<(), SvgError> {
        self.cubic(self.current, b, c, d, 0)
    }
    pub(crate) fn quadratic(&mut self, b: [f32; 2], d: [f32; 2]) -> Result<(), SvgError> {
        let a = self.current;
        self.curve(
            [
                a[0] + (b[0] - a[0]) * 2.0 / 3.0,
                a[1] + (b[1] - a[1]) * 2.0 / 3.0,
            ],
            [
                d[0] + (b[0] - d[0]) * 2.0 / 3.0,
                d[1] + (b[1] - d[1]) * 2.0 / 3.0,
            ],
            d,
        )
    }
    fn arc(
        &mut self,
        r: [f32; 2],
        rotation: f32,
        large: bool,
        sweep: bool,
        end: [f32; 2],
    ) -> Result<(), SvgError> {
        if end == self.current {
            return Ok(());
        }
        let mut rx = r[0].abs();
        let mut ry = r[1].abs();
        if rx == 0.0 || ry == 0.0 {
            return self.line(end);
        }
        if rx < 0.000001 || ry < 0.000001 {
            return Err(SvgError::InvalidInput);
        }
        let (s, c) = sin_cos_approx((rotation % 360.0) * core::f32::consts::PI / 180.0);
        let delta = [
            (self.current[0] - end[0]) * 0.5,
            (self.current[1] - end[1]) * 0.5,
        ];
        let p = [c * delta[0] + s * delta[1], -s * delta[0] + c * delta[1]];
        let lambda = p[0] * p[0] / (rx * rx) + p[1] * p[1] / (ry * ry);
        if lambda > 1.0 {
            let scale = sqrt_approx(lambda);
            rx *= scale;
            ry *= scale;
        }
        if !rx.is_finite() || !ry.is_finite() || rx > 1_000_000.0 || ry > 1_000_000.0 {
            return Err(SvgError::TooComplex);
        }
        let numerator =
            (rx * rx * ry * ry - rx * rx * p[1] * p[1] - ry * ry * p[0] * p[0]).max(0.0);
        let denominator = rx * rx * p[1] * p[1] + ry * ry * p[0] * p[0];
        let factor = if denominator > 0.0 {
            sqrt_approx(numerator / denominator) * if large == sweep { -1.0 } else { 1.0 }
        } else {
            0.0
        };
        let local = [factor * rx * p[1] / ry, -factor * ry * p[0] / rx];
        let center = [
            c * local[0] - s * local[1] + (self.current[0] + end[0]) * 0.5,
            s * local[0] + c * local[1] + (self.current[1] + end[1]) * 0.5,
        ];
        let from = angle([(p[0] - local[0]) / rx, (p[1] - local[1]) / ry]);
        let to = angle([(-p[0] - local[0]) / rx, (-p[1] - local[1]) / ry]);
        let mut span = to - from;
        if sweep && span < 0.0 {
            span += core::f32::consts::TAU;
        }
        if !sweep && span > 0.0 {
            span -= core::f32::consts::TAU;
        }
        let estimate = (span.abs() * sqrt_approx(rx.max(ry) / (8.0 * self.tolerance))).max(4.0);
        if !estimate.is_finite() || estimate > 2047.0 {
            return Err(SvgError::TooComplex);
        }
        let segments = estimate as usize + 1;
        for i in 1..segments {
            let (sa, ca) = sin_cos_approx(from + span * i as f32 / segments as f32);
            self.line([
                center[0] + c * rx * ca - s * ry * sa,
                center[1] + s * rx * ca + c * ry * sa,
            ])?;
        }
        self.line(end)
    }
}
fn mid(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]
}
fn angle(v: [f32; 2]) -> f32 {
    let len = length(v);
    if len == 0.0 {
        return 0.0;
    }
    let x = (v[0] / len).clamp(-1.0, 1.0);
    let mut low = 0.0;
    let mut high = core::f32::consts::PI;
    for _ in 0..24 {
        let mid = (low + high) * 0.5;
        if sin_cos_approx(mid).1 > x {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) * 0.5 * if v[1] < 0.0 { -1.0 } else { 1.0 }
}

pub(super) fn path(data: &str, tolerance: f32) -> Result<Vec<Edge>, SvgError> {
    let mut numbers = Numbers::new(data);
    let mut path = Path::new(tolerance);
    let mut cmd = b' ';
    let mut previous = b' ';
    let mut control = [0.0; 2];
    let mut began = false;
    while !numbers.done() {
        if numbers.bytes[numbers.pos].is_ascii_alphabetic() {
            cmd = numbers.bytes[numbers.pos];
            numbers.pos += 1;
        }
        let upper = cmd.to_ascii_uppercase();
        if !began && upper != b'M' {
            return Err(SvgError::InvalidInput);
        }
        let base = if cmd.is_ascii_lowercase() {
            path.current
        } else {
            [0.0; 2]
        };
        match upper {
            b'M' => {
                let p = numbers.pair(base)?;
                path.moveto(p)?;
                began = true;
                cmd = if cmd == b'm' { b'l' } else { b'L' };
            }
            b'L' => {
                let p = numbers.pair(base)?;
                path.line(p)?;
            }
            b'H' => {
                let x = numbers.number()? + base[0];
                path.line([x, path.current[1]])?;
            }
            b'V' => {
                let y = numbers.number()? + base[1];
                path.line([path.current[0], y])?;
            }
            b'C' => {
                let b = numbers.pair(base)?;
                let c = numbers.pair(base)?;
                let d = numbers.pair(base)?;
                path.curve(b, c, d)?;
                control = c;
            }
            b'S' => {
                let b = if previous == b'C' || previous == b'S' {
                    [
                        2.0 * path.current[0] - control[0],
                        2.0 * path.current[1] - control[1],
                    ]
                } else {
                    path.current
                };
                let c = numbers.pair(base)?;
                let d = numbers.pair(base)?;
                path.curve(b, c, d)?;
                control = c;
            }
            b'Q' => {
                let b = numbers.pair(base)?;
                let d = numbers.pair(base)?;
                path.quadratic(b, d)?;
                control = b;
            }
            b'T' => {
                let b = if previous == b'Q' || previous == b'T' {
                    [
                        2.0 * path.current[0] - control[0],
                        2.0 * path.current[1] - control[1],
                    ]
                } else {
                    path.current
                };
                let d = numbers.pair(base)?;
                path.quadratic(b, d)?;
                control = b;
            }
            b'A' => {
                let r = [numbers.number()?, numbers.number()?];
                let rotation = numbers.number()?;
                let large = numbers.flag()?;
                let sweep = numbers.flag()?;
                let end = numbers.pair(base)?;
                path.arc(r, rotation, large, sweep, end)?;
            }
            b'Z' => {
                path.finish_contour(true)?;
                cmd = b' ';
            }
            _ => return Err(SvgError::InvalidInput),
        }
        previous = upper;
    }
    path.finish_contour(false)?;
    Ok(path.edges)
}

pub(super) fn ellipse(
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    tolerance: f32,
) -> Result<Vec<Edge>, SvgError> {
    if rx < 0.0 || ry < 0.0 {
        return Err(SvgError::InvalidInput);
    }
    if rx == 0.0 || ry == 0.0 {
        return Ok(Vec::new());
    }
    let estimate = (core::f32::consts::TAU * sqrt_approx(rx.max(ry) / (8.0 * tolerance))).max(12.0);
    if !estimate.is_finite() || estimate > 2047.0 {
        return Err(SvgError::TooComplex);
    }
    let n = estimate as usize + 1;
    let mut p = Path::new(tolerance);
    p.moveto([cx + rx, cy])?;
    for i in 1..n {
        let (s, c) = sin_cos_approx(core::f32::consts::TAU * i as f32 / n as f32);
        p.line([cx + rx * c, cy + ry * s])?;
    }
    p.finish_contour(true)?;
    Ok(p.edges)
}

pub(super) fn rectangle(
    rect: [f32; 4],
    r: [f32; 2],
    tolerance: f32,
) -> Result<Vec<Edge>, SvgError> {
    let [x, y, w, h] = rect;
    if w < 0.0 || h < 0.0 || r[0] < 0.0 || r[1] < 0.0 {
        return Err(SvgError::InvalidInput);
    }
    if w == 0.0 || h == 0.0 {
        return Ok(Vec::new());
    }
    let rx = r[0].min(w * 0.5);
    let ry = r[1].min(h * 0.5);
    let mut p = Path::new(tolerance);
    p.moveto([x + rx, y])?;
    p.line([x + w - rx, y])?;
    p.arc([rx, ry], 0.0, false, true, [x + w, y + ry])?;
    p.line([x + w, y + h - ry])?;
    p.arc([rx, ry], 0.0, false, true, [x + w - rx, y + h])?;
    p.line([x + rx, y + h])?;
    p.arc([rx, ry], 0.0, false, true, [x, y + h - ry])?;
    p.line([x, y + ry])?;
    p.arc([rx, ry], 0.0, false, true, [x + rx, y])?;
    p.finish_contour(true)?;
    Ok(p.edges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shallow_curve_retains_its_perpendicular_deviation() {
        let edges = path("M0 0 C170 4 340 4 512 0", 0.125).unwrap();
        assert!(edges.len() > 2);
        assert!(edges.iter().any(|edge| edge.b[1] >= 2.9));
    }
}
