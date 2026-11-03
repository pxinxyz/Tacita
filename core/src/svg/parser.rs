use super::path::{ellipse, path, rectangle, Numbers, Path};
use super::{inverse, length, multiply, Matrix, Paint, Shape, SvgError, IDENTITY, MAX_EDGES};
use crate::draw::sin_cos_approx;
use alloc::vec::Vec;

struct Attributes<'a> {
    text: &'a str,
    pos: usize,
}
impl<'a> Attributes<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }
    fn next(&mut self) -> Result<Option<(&'a str, &'a str)>, SvgError> {
        let b = self.text.as_bytes();
        while b.get(self.pos).is_some_and(u8::is_ascii_whitespace) {
            self.pos += 1;
        }
        if self.pos == b.len() {
            return Ok(None);
        }
        let start = self.pos;
        while b
            .get(self.pos)
            .is_some_and(|c| c.is_ascii_alphanumeric() || b"_:-".contains(c))
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(SvgError::InvalidInput);
        }
        let name = &self.text[start..self.pos];
        while b.get(self.pos).is_some_and(u8::is_ascii_whitespace) {
            self.pos += 1;
        }
        if b.get(self.pos) != Some(&b'=') {
            return Err(SvgError::InvalidInput);
        }
        self.pos += 1;
        while b.get(self.pos).is_some_and(u8::is_ascii_whitespace) {
            self.pos += 1;
        }
        let quote = *b.get(self.pos).ok_or(SvgError::InvalidInput)?;
        if quote != b'\'' && quote != b'"' {
            return Err(SvgError::InvalidInput);
        }
        self.pos += 1;
        let start = self.pos;
        while b.get(self.pos).is_some_and(|c| *c != quote) {
            self.pos += 1;
        }
        if self.pos == b.len() {
            return Err(SvgError::InvalidInput);
        }
        let value = &self.text[start..self.pos];
        self.pos += 1;
        if value.contains(['<', '&']) {
            return Err(SvgError::Unsupported);
        }
        Ok(Some((name, value)))
    }
}
fn attr<'a>(attrs: &'a str, name: &str) -> Result<Option<&'a str>, SvgError> {
    let mut found = None;
    let mut iter = Attributes::new(attrs);
    while let Some((key, value)) = iter.next()? {
        if key == name {
            if found.is_some() {
                return Err(SvgError::InvalidInput);
            }
            found = Some(value);
        }
    }
    Ok(found)
}
fn scalar(value: &str) -> Result<f32, SvgError> {
    let mut n = Numbers::new(value.trim().strip_suffix("px").unwrap_or(value.trim()));
    let v = n.number()?;
    if !n.done() {
        return Err(SvgError::Unsupported);
    }
    Ok(v)
}
fn number(attrs: &str, name: &str, default: f32) -> Result<f32, SvgError> {
    attr(attrs, name)?.map_or(Ok(default), scalar)
}
fn opacity(value: &str) -> Result<f32, SvgError> {
    let n = scalar(value)?;
    if !(0.0..=1.0).contains(&n) {
        return Err(SvgError::InvalidInput);
    }
    Ok(n)
}
fn ink(value: &str) -> Result<bool, SvgError> {
    if value == "none" {
        return Ok(false);
    }
    if ["currentColor", "black", "white", "red", "blue", "green"].contains(&value) {
        return Ok(true);
    }
    if let Some(hex) = value.strip_prefix('#') {
        if [3, 6].contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(true);
        }
    }
    Err(SvgError::Unsupported)
}
fn transform(text: &str) -> Result<Matrix, SvgError> {
    let mut rest = text.trim();
    let mut matrix = IDENTITY;
    let mut count = 0;
    while !rest.is_empty() {
        count += 1;
        if count > 32 {
            return Err(SvgError::TooComplex);
        }
        let open = rest.find('(').ok_or(SvgError::InvalidInput)?;
        let close = rest[open + 1..].find(')').ok_or(SvgError::InvalidInput)? + open + 1;
        let name = rest[..open].trim();
        let mut n = Numbers::new(&rest[open + 1..close]);
        let mut v = [0.0; 6];
        let mut len = 0;
        while !n.done() {
            if len == 6 {
                return Err(SvgError::InvalidInput);
            }
            v[len] = n.number()?;
            len += 1;
        }
        let m = match (name, len) {
            ("matrix", 6) => v,
            ("translate", 1 | 2) => [1.0, 0.0, 0.0, 1.0, v[0], if len == 2 { v[1] } else { 0.0 }],
            ("scale", 1 | 2) => [v[0], 0.0, 0.0, if len == 2 { v[1] } else { v[0] }, 0.0, 0.0],
            ("rotate", 1 | 3) => {
                let (s, c) = sin_cos_approx((v[0] % 360.0) * core::f32::consts::PI / 180.0);
                let mut m = [c, s, -s, c, 0.0, 0.0];
                if len == 3 {
                    m[4] = v[1] - c * v[1] + s * v[2];
                    m[5] = v[2] - s * v[1] - c * v[2];
                }
                m
            }
            ("skewX", 1) | ("skewY", 1) => {
                let (s, c) = sin_cos_approx((v[0] % 360.0) * core::f32::consts::PI / 180.0);
                if c.abs() < 1e-6 {
                    return Err(SvgError::InvalidInput);
                }
                let mut m = IDENTITY;
                m[if name == "skewX" { 2 } else { 1 }] = s / c;
                m
            }
            _ => return Err(SvgError::Unsupported),
        };
        matrix = multiply(matrix, m);
        if matrix
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
        {
            return Err(SvgError::InvalidInput);
        }
        rest = rest[close + 1..].trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
    }
    Ok(matrix)
}
#[derive(Clone, Copy)]
struct State<'a> {
    tag: &'a str,
    paint: Paint,
    matrix: Matrix,
    round_join: bool,
}

pub(super) fn parse(source: &str, width: u32, height: u32) -> Result<Vec<Shape>, SvgError> {
    let mut stack = [State {
        tag: "",
        paint: Paint::default(),
        matrix: IDENTITY,
        round_join: false,
    }; 32];
    let mut depth = 0;
    let mut root = false;
    let mut root_closed = false;
    let mut shapes = Vec::new();
    let mut edges = 0;
    let mut pos = 0;
    while pos < source.len() {
        let remaining = &source[pos..];
        if remaining.starts_with("<!--") {
            let end = remaining.find("-->").ok_or(SvgError::InvalidInput)?;
            pos += end + 3;
            continue;
        }
        if remaining.starts_with("<?xml") {
            let end = remaining.find("?>").ok_or(SvgError::InvalidInput)?;
            pos += end + 2;
            continue;
        }
        if !remaining.starts_with('<') {
            let end = remaining.find('<').unwrap_or(remaining.len());
            if !remaining[..end].trim().is_empty() {
                return Err(SvgError::Unsupported);
            }
            pos += end;
            continue;
        }
        let mut quote = 0;
        let mut end = None;
        for (i, b) in remaining.bytes().enumerate().skip(1) {
            if quote != 0 {
                if b == quote {
                    quote = 0;
                }
            } else if b == b'\'' || b == b'"' {
                quote = b;
            } else if b == b'>' {
                end = Some(i);
                break;
            }
        }
        let end = end.ok_or(SvgError::InvalidInput)?;
        let tag = remaining[1..end].trim();
        pos += end + 1;
        if let Some(closing) = tag.strip_prefix('/') {
            let name = closing.trim();
            if depth == 0 || stack[depth - 1].tag != name {
                return Err(SvgError::InvalidInput);
            }
            depth -= 1;
            if depth == 0 {
                root_closed = true;
            }
            continue;
        }
        let (tag, closed) = tag
            .strip_suffix('/')
            .map_or((tag, false), |tag| (tag.trim_end(), true));
        let split = tag.find(char::is_whitespace).unwrap_or(tag.len());
        let name = &tag[..split];
        let attrs = &tag[split..];
        if root_closed {
            return Err(SvgError::InvalidInput);
        }
        let is_root = depth == 0;
        if is_root {
            if root || name != "svg" {
                return Err(SvgError::InvalidInput);
            }
            root = true;
        } else if ![
            "g", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon",
        ]
        .contains(&name)
        {
            return Err(SvgError::Unsupported);
        }
        if depth >= 32 || shapes.len() >= 256 {
            return Err(SvgError::TooComplex);
        }
        if depth > 0 && !["svg", "g"].contains(&stack[depth - 1].tag) {
            return Err(SvgError::Unsupported);
        }
        let mut state = if depth == 0 {
            stack[0]
        } else {
            stack[depth - 1]
        };
        state.tag = name;
        let mut iter = Attributes::new(attrs);
        let mut attribute_count = 0;
        while let Some((key, value)) = iter.next()? {
            attribute_count += 1;
            if attribute_count > 64 {
                return Err(SvgError::TooComplex);
            }
            attr(attrs, key)?;
            match key {
                "fill" => state.paint.fill = ink(value)?,
                "stroke" => state.paint.stroke = ink(value)?,
                "stroke-width" => {
                    let w = scalar(value)?;
                    if w < 0.0 {
                        return Err(SvgError::InvalidInput);
                    }
                    state.paint.width = w;
                }
                "fill-opacity" => state.paint.fill_alpha = opacity(value)?,
                "stroke-opacity" => state.paint.stroke_alpha = opacity(value)?,
                "opacity" => {
                    let a = opacity(value)?;
                    if ["svg", "g"].contains(&name) && a != 1.0 {
                        return Err(SvgError::Unsupported);
                    }
                    state.paint.opacity *= a;
                }
                "fill-rule" => {
                    state.paint.even_odd = match value {
                        "evenodd" => true,
                        "nonzero" => false,
                        _ => return Err(SvgError::Unsupported),
                    }
                }
                "stroke-linecap" => {
                    state.paint.round_caps = match value {
                        "round" => true,
                        "butt" => false,
                        _ => return Err(SvgError::Unsupported),
                    }
                }
                "stroke-linejoin" => {
                    if value != "round" {
                        return Err(SvgError::Unsupported);
                    }
                    state.round_join = true;
                }
                "transform" => state.matrix = multiply(state.matrix, transform(value)?),
                "viewBox" | "preserveAspectRatio" if is_root => {}
                "width" | "height" if is_root || name == "rect" => {}
                "x" | "y" | "rx" | "ry" if name == "rect" => {}
                "cx" | "cy" if name == "circle" || name == "ellipse" => {}
                "r" if name == "circle" => {}
                "rx" | "ry" if name == "ellipse" => {}
                "x1" | "x2" | "y1" | "y2" if name == "line" => {}
                "d" if name == "path" => {}
                "points" if name == "polygon" || name == "polyline" => {}
                "id" | "version" | "xmlns" | "role" | "focusable" => {}
                k if k.starts_with("aria-") || k.starts_with("data-") => {}
                _ => return Err(SvgError::Unsupported),
            }
        }
        if is_root {
            let view = if let Some(view) = attr(attrs, "viewBox")? {
                let mut n = Numbers::new(view);
                let v = [n.number()?, n.number()?, n.number()?, n.number()?];
                if !n.done() {
                    return Err(SvgError::InvalidInput);
                }
                v
            } else {
                [
                    0.0,
                    0.0,
                    number(attrs, "width", width as f32)?,
                    number(attrs, "height", height as f32)?,
                ]
            };
            if view[2] <= 0.0 || view[3] <= 0.0 {
                return Err(SvgError::InvalidInput);
            }
            let sx = width as f32 / view[2];
            let sy = height as f32 / view[3];
            let viewport = match attr(attrs, "preserveAspectRatio")?.unwrap_or("xMidYMid meet") {
                "none" => [sx, 0.0, 0.0, sy, -view[0] * sx, -view[1] * sy],
                "xMidYMid" | "xMidYMid meet" => {
                    let scale = sx.min(sy);
                    [
                        scale,
                        0.0,
                        0.0,
                        scale,
                        (width as f32 - view[2] * scale) * 0.5 - view[0] * scale,
                        (height as f32 - view[3] * scale) * 0.5 - view[1] * scale,
                    ]
                }
                _ => return Err(SvgError::Unsupported),
            };
            state.matrix = multiply(viewport, state.matrix);
        }
        if !["svg", "g"].contains(&name) {
            let matrix = state.matrix;
            let inv = inverse(matrix)?;
            let scale = length([matrix[0], matrix[1]])
                .max(length([matrix[2], matrix[3]]))
                .max(1e-6);
            let tolerance = 0.125 / scale;
            let geometry = match name {
                "path" => path(attr(attrs, "d")?.unwrap_or(""), tolerance)?,
                "rect" => {
                    let rx = number(attrs, "rx", number(attrs, "ry", 0.0)?)?;
                    let ry = number(attrs, "ry", rx)?;
                    rectangle(
                        [
                            number(attrs, "x", 0.0)?,
                            number(attrs, "y", 0.0)?,
                            number(attrs, "width", 0.0)?,
                            number(attrs, "height", 0.0)?,
                        ],
                        [rx, ry],
                        tolerance,
                    )?
                }
                "circle" => {
                    let r = number(attrs, "r", 0.0)?;
                    ellipse(
                        number(attrs, "cx", 0.0)?,
                        number(attrs, "cy", 0.0)?,
                        r,
                        r,
                        tolerance,
                    )?
                }
                "ellipse" => ellipse(
                    number(attrs, "cx", 0.0)?,
                    number(attrs, "cy", 0.0)?,
                    number(attrs, "rx", 0.0)?,
                    number(attrs, "ry", 0.0)?,
                    tolerance,
                )?,
                "line" => {
                    let mut p = Path::new(tolerance);
                    p.moveto([number(attrs, "x1", 0.0)?, number(attrs, "y1", 0.0)?])?;
                    p.line([number(attrs, "x2", 0.0)?, number(attrs, "y2", 0.0)?])?;
                    p.finish_contour(false)?;
                    p.edges
                }
                "polyline" | "polygon" => {
                    let mut n = Numbers::new(attr(attrs, "points")?.unwrap_or(""));
                    let mut p = Path::new(tolerance);
                    if !n.done() {
                        p.moveto([n.number()?, n.number()?])?;
                        while !n.done() {
                            p.line([n.number()?, n.number()?])?;
                        }
                    }
                    p.finish_contour(name == "polygon")?;
                    p.edges
                }
                _ => return Err(SvgError::Unsupported),
            };
            if state.paint.stroke
                && !state.round_join
                && !["line", "circle", "ellipse"].contains(&name)
            {
                return Err(SvgError::Unsupported);
            }
            edges += geometry.len();
            if edges > MAX_EDGES {
                return Err(SvgError::TooComplex);
            }
            shapes.push(Shape {
                edges: geometry,
                paint: state.paint,
                inverse: inv,
            });
        }
        if !closed {
            stack[depth] = state;
            depth += 1;
        } else if is_root {
            root_closed = true;
        }
    }
    if !root || depth != 0 {
        return Err(SvgError::InvalidInput);
    }
    Ok(shapes)
}
