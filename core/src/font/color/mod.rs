use crate::draw::sqrt_approx;
use crate::svg::{inverse, multiply, path::Path, point, Edge, Matrix};
use alloc::{vec, vec::Vec};
use rustybuzz::ttf_parser::{
    self as ttf,
    colr::{CompositeMode, GradientExtend, Paint, Painter},
    GlyphId, OutlineBuilder,
};
pub(super) mod atlas;
mod emoji_presentation;
mod srgb;
mod validate;
pub(super) use emoji_presentation::prefers_color;

const MAX_DEPTH: usize = 48;
const MAX_NODES: usize = 4096;
const MAX_PIXELS: u32 = 256;
const MAX_WORK: usize = 200_000_000;
fn floor_f32(v: f32) -> f32 {
    let i = v as i32 as f32;
    if v < i {
        i - 1.
    } else {
        i
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorFontError {
    InvalidFont,
    Unsupported,
    TooComplex,
    AtlasFull,
    InvalidTexture,
}

impl super::Font {
    pub fn set_color_atlas_texture(
        &mut self,
        texture: Option<crate::draw::TextureId>,
    ) -> Result<(), ColorFontError> {
        if texture.is_some_and(|id| id.0 < 2) {
            return Err(ColorFontError::InvalidTexture);
        }
        if self.color_texture != texture {
            self.color_texture = texture;
            self.layout_key = self.layout_key.wrapping_mul(1099511628211).wrapping_add(1);
        }
        Ok(())
    }
    pub fn color_atlas_texture(&self) -> Option<crate::draw::TextureId> {
        self.color_texture
    }
    pub fn color_atlas_pixels(&self) -> (&[u8], u32, u32) {
        (
            &self.color_atlas.pixels,
            self.color_atlas.width,
            self.color_atlas.height,
        )
    }
    pub fn color_atlas_generation(&self) -> u64 {
        self.color_atlas.generation
    }
    pub fn color_glyph_error(&self, face: super::FontId, index: u16) -> Option<ColorFontError> {
        self.faces
            .get(face.0 as usize)?
            .color_glyphs
            .get(&index)?
            .as_ref()
            .err()
            .copied()
    }
    pub(super) fn ensure_color(&mut self, id: super::FontId, index: u16) -> bool {
        if self.color_texture.is_none() {
            return false;
        }
        let f = &self.faces[id.0 as usize];
        if !f.has_color {
            return false;
        }
        if let Some(entry) = f.color_glyphs.get(&index) {
            return entry.is_ok();
        }
        let result = rasterize(&f.source_bytes, index, f.px_size * self.raster_scale)
            .and_then(|bitmap| self.color_atlas.insert(&bitmap));
        let ok = result.is_ok();
        self.faces[id.0 as usize].color_glyphs.insert(index, result);
        ok
    }
    pub(super) fn color_quad(
        &self,
        id: super::FontId,
        index: u16,
        pen: [f32; 2],
    ) -> Option<super::GlyphQuad> {
        self.color_texture?;
        let face = &self.faces[id.0 as usize];
        let entry = face.color_glyphs.get(&index)?.as_ref().ok()?;
        let [x, y, w, h] = entry.rect;
        let min = [
            pen[0] + entry.origin[0] as f32 / self.raster_scale,
            pen[1] + face.ascent + entry.origin[1] as f32 / self.raster_scale,
        ];
        Some(super::GlyphQuad {
            min,
            max: [
                min[0] + w as f32 / self.raster_scale,
                min[1] + h as f32 / self.raster_scale,
            ],
            uv_min: [
                x as f32 / self.color_atlas.width as f32,
                y as f32 / self.color_atlas.height as f32,
            ],
            uv_max: [
                (x + w) as f32 / self.color_atlas.width as f32,
                (y + h) as f32 / self.color_atlas.height as f32,
            ],
        })
    }
}

pub(super) struct Bitmap {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub origin: [i32; 2],
}
#[derive(Clone, Copy)]
struct Stop {
    offset: f32,
    color: [f32; 4],
}
enum Brush {
    Solid([f32; 4]),
    Linear([f32; 6], Vec<Stop>, GradientExtend),
    Radial([f32; 6], Vec<Stop>, GradientExtend),
}
enum Op {
    Clip(usize),
    Unclip,
    Fill(Brush, Matrix),
    Layer(CompositeMode),
    EndLayer,
}
struct Recorder<'a> {
    face: &'a ttf::Face<'a>,
    transforms: Vec<Matrix>,
    paths: Vec<Vec<Edge>>,
    outline: Option<usize>,
    ops: Vec<Op>,
    error: Option<ColorFontError>,
    edges: usize,
    clips: usize,
    layers: usize,
}
struct Outline {
    path: Path,
    transform: Matrix,
    failed: bool,
}
impl OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.failed |= self.path.moveto(point(self.transform, [x, y])).is_err();
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.failed |= self.path.line(point(self.transform, [x, y])).is_err();
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.failed |= self
            .path
            .quadratic(
                point(self.transform, [x1, y1]),
                point(self.transform, [x, y]),
            )
            .is_err();
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.failed |= self
            .path
            .curve(
                point(self.transform, [x1, y1]),
                point(self.transform, [x2, y2]),
                point(self.transform, [x, y]),
            )
            .is_err();
    }
    fn close(&mut self) {
        self.failed |= self.path.finish_contour(true).is_err();
    }
}
fn rgba(color: ttf::RgbaColor) -> [f32; 4] {
    let alpha = color.alpha as f32 / 255.;
    [
        srgb::DECODE[color.red as usize] * alpha,
        srgb::DECODE[color.green as usize] * alpha,
        srgb::DECODE[color.blue as usize] * alpha,
        alpha,
    ]
}
fn stops(iter: impl Iterator<Item = ttf::colr::ColorStop>) -> Result<Vec<Stop>, ColorFontError> {
    let mut result = Vec::new();
    for stop in iter {
        if result.len() == 16 || !stop.stop_offset.is_finite() {
            return Err(ColorFontError::TooComplex);
        }
        result.push(Stop {
            offset: stop.stop_offset,
            color: rgba(stop.color),
        });
    }
    for i in 1..result.len() {
        let mut n = i;
        while n > 0 && result[n].offset < result[n - 1].offset {
            result.swap(n, n - 1);
            n -= 1;
        }
    }
    Ok(result)
}
impl Recorder<'_> {
    fn op(&mut self, op: Op) {
        if self.ops.len() >= MAX_NODES * 4 {
            self.error = Some(ColorFontError::TooComplex);
        }
        if self.error.is_none() {
            self.ops.push(op);
        }
    }
    fn path(&mut self, edges: Vec<Edge>) {
        self.edges += edges.len();
        if self.edges > 32768 || self.paths.len() >= MAX_NODES {
            self.error = Some(ColorFontError::TooComplex);
            return;
        }
        self.outline = Some(self.paths.len());
        self.paths.push(edges);
    }
}
impl<'a> Painter<'a> for Recorder<'a> {
    fn outline_glyph(&mut self, glyph: GlyphId) {
        if self.error.is_some() {
            return;
        }
        let mut builder = Outline {
            path: Path::new(0.08),
            transform: *self.transforms.last().unwrap(),
            failed: false,
        };
        if self.face.outline_glyph(glyph, &mut builder).is_none() || builder.failed {
            self.error = Some(ColorFontError::InvalidFont);
            return;
        }
        if builder.path.finish_contour(true).is_err() {
            self.error = Some(ColorFontError::TooComplex);
            return;
        }
        self.path(builder.path.edges);
    }
    fn paint(&mut self, paint: Paint<'a>) {
        if self.error.is_some() {
            return;
        }
        let brush = match paint {
            Paint::Solid(c) => Ok(Brush::Solid(rgba(c))),
            Paint::LinearGradient(g) => stops(g.stops(0, &[]))
                .map(|s| Brush::Linear([g.x0, g.y0, g.x1, g.y1, g.x2, g.y2], s, g.extend)),
            Paint::RadialGradient(g) => stops(g.stops(0, &[]))
                .map(|s| Brush::Radial([g.x0, g.y0, g.r0, g.x1, g.y1, g.r1], s, g.extend)),
            _ => Err(ColorFontError::Unsupported),
        };
        match (brush, inverse(*self.transforms.last().unwrap())) {
            (Ok(b), Ok(m)) => self.op(Op::Fill(b, m)),
            (Err(e), _) => self.error = Some(e),
            _ => self.error = Some(ColorFontError::InvalidFont),
        }
    }
    fn push_clip(&mut self) {
        if let Some(index) = self.outline {
            self.clips += 1;
            if self.clips > MAX_DEPTH {
                self.error = Some(ColorFontError::TooComplex);
            }
            self.op(Op::Clip(index));
        } else {
            self.error = Some(ColorFontError::InvalidFont);
        }
    }
    fn push_clip_box(&mut self, b: ttf::colr::ClipBox) {
        if self.error.is_some() {
            return;
        }
        let mut path = Path::new(0.08);
        let m = *self.transforms.last().unwrap();
        let result = path
            .moveto(point(m, [b.x_min, b.y_min]))
            .and_then(|_| path.line(point(m, [b.x_max, b.y_min])))
            .and_then(|_| path.line(point(m, [b.x_max, b.y_max])))
            .and_then(|_| path.line(point(m, [b.x_min, b.y_max])))
            .and_then(|_| path.finish_contour(true));
        if result.is_err() {
            self.error = Some(ColorFontError::InvalidFont);
            return;
        }
        self.path(path.edges);
        self.push_clip();
    }
    fn pop_clip(&mut self) {
        if self.clips == 0 {
            self.error = Some(ColorFontError::InvalidFont);
            return;
        }
        self.clips -= 1;
        self.op(Op::Unclip);
    }
    fn push_layer(&mut self, mode: CompositeMode) {
        if ![
            CompositeMode::SourceOver,
            CompositeMode::SourceIn,
            CompositeMode::SoftLight,
        ]
        .contains(&mode)
        {
            self.error = Some(ColorFontError::Unsupported);
        }
        self.layers += 1;
        if self.layers >= MAX_DEPTH {
            self.error = Some(ColorFontError::TooComplex);
        }
        self.op(Op::Layer(mode));
    }
    fn pop_layer(&mut self) {
        if self.layers == 0 {
            self.error = Some(ColorFontError::InvalidFont);
            return;
        }
        self.layers -= 1;
        self.op(Op::EndLayer);
    }
    fn push_transform(&mut self, t: ttf::Transform) {
        if self.error.is_some() {
            return;
        }
        if self.transforms.len() >= MAX_DEPTH {
            self.error = Some(ColorFontError::TooComplex);
            return;
        }
        let m = multiply(
            *self.transforms.last().unwrap(),
            [t.a, t.b, t.c, t.d, t.e, t.f],
        );
        if m.iter().any(|v| !v.is_finite() || v.abs() > 1_000_000.) {
            self.error = Some(ColorFontError::InvalidFont);
            return;
        }
        self.transforms.push(m);
    }
    fn pop_transform(&mut self) {
        if self.error.is_some() {
            return;
        }
        if self.transforms.len() <= 1 {
            self.error = Some(ColorFontError::InvalidFont);
            return;
        }
        self.transforms.pop();
    }
}
fn gradient(stops: &[Stop], extend: GradientExtend, t: f32) -> [f32; 4] {
    if !t.is_finite() {
        return [0.; 4];
    }
    if stops.is_empty() {
        return [0.; 4];
    }
    if stops.len() == 1 {
        return stops[0].color;
    }
    let start = stops[0].offset;
    let end = stops[stops.len() - 1].offset;
    let range = end - start;
    if range == 0. {
        return if matches!(extend, GradientExtend::Pad) {
            if t < start {
                stops[0].color
            } else {
                stops[stops.len() - 1].color
            }
        } else {
            [0.; 4]
        };
    }
    let t = match extend {
        GradientExtend::Pad => t,
        GradientExtend::Repeat => {
            let r = (t - start) % range;
            start + if r < 0. { r + range } else { r }
        }
        GradientExtend::Reflect => {
            let v = (t - start) / range;
            let q = v % 2.;
            let q = if q < 0. { q + 2. } else { q };
            start + if q <= 1. { q } else { 2. - q } * range
        }
    };
    if t < start {
        return stops[0].color;
    }
    let next = stops.iter().position(|s| s.offset > t);
    let Some(n) = next else {
        return stops[stops.len() - 1].color;
    };
    if n == 0 {
        return stops[0].color;
    }
    let a = stops[n - 1];
    let b = stops[n];
    let f = (t - a.offset) / (b.offset - a.offset);
    core::array::from_fn(|i| a.color[i] + (b.color[i] - a.color[i]) * f)
}
impl Brush {
    fn sample(&self, p: [f32; 2]) -> [f32; 4] {
        match self {
            Self::Solid(color) => *color,
            Self::Linear(g, stops, extend) => {
                let n = [g[5] - g[1], g[0] - g[4]];
                let d = (g[2] - g[0]) * n[0] + (g[3] - g[1]) * n[1];
                if d == 0. {
                    return [0.; 4];
                }
                gradient(
                    stops,
                    *extend,
                    ((p[0] - g[0]) * n[0] + (p[1] - g[1]) * n[1]) / d,
                )
            }
            Self::Radial(g, stops, extend) => {
                let q = [p[0] - g[0], p[1] - g[1]];
                let dc = [g[3] - g[0], g[4] - g[1]];
                let dr = g[5] - g[2];
                let a = dc[0] * dc[0] + dc[1] * dc[1] - dr * dr;
                let b = -2. * (q[0] * dc[0] + q[1] * dc[1] + g[2] * dr);
                let c = q[0] * q[0] + q[1] * q[1] - g[2] * g[2];
                let roots = if a.abs() < 1e-6 {
                    if b.abs() < 1e-6 {
                        return [0.; 4];
                    }
                    [-c / b; 2]
                } else {
                    let d = b * b - 4. * a * c;
                    if d < 0. {
                        return [0.; 4];
                    }
                    let d = sqrt_approx(d);
                    [(-b + d) / (2. * a), (-b - d) / (2. * a)]
                };
                let mut t = f32::NEG_INFINITY;
                for r in roots {
                    if g[2] + r * dr > 0. {
                        t = t.max(r);
                    }
                }
                gradient(stops, *extend, t)
            }
        }
    }
}
fn composite(s: [f32; 4], b: [f32; 4], mode: CompositeMode) -> [f32; 4] {
    match mode {
        CompositeMode::SourceIn => s.map(|c| c * b[3]),
        CompositeMode::SoftLight => {
            let mut out = [0.; 4];
            for i in 0..3 {
                let cb = if b[3] > 0. { b[i] / b[3] } else { 0. };
                let cs = if s[3] > 0. { s[i] / s[3] } else { 0. };
                let blend = if cs <= 0.5 {
                    cb - (1. - 2. * cs) * cb * (1. - cb)
                } else {
                    let d = if cb <= 0.25 {
                        ((16. * cb - 12.) * cb + 4.) * cb
                    } else {
                        sqrt_approx(cb)
                    };
                    cb + (2. * cs - 1.) * (d - cb)
                };
                out[i] = s[i] * (1. - b[3]) + b[i] * (1. - s[3]) + s[3] * b[3] * blend;
            }
            out[3] = s[3] + b[3] * (1. - s[3]);
            out
        }
        _ => core::array::from_fn(|i| s[i] + b[i] * (1. - s[3])),
    }
}

pub(super) fn rasterize(bytes: &[u8], glyph: u16, px: f32) -> Result<Bitmap, ColorFontError> {
    if !px.is_finite() || px <= 0. {
        return Err(ColorFontError::InvalidFont);
    }
    if px > MAX_PIXELS as f32 {
        return Err(ColorFontError::TooComplex);
    }
    let face = ttf::Face::parse(bytes, 0).map_err(|_| ColorFontError::InvalidFont)?;
    validate::validate(&face, glyph)?;
    let scale = px / face.units_per_em() as f32;
    let mut painter = Recorder {
        face: &face,
        transforms: vec![[scale, 0., 0., -scale, 0., 0.]],
        paths: Vec::new(),
        outline: None,
        ops: Vec::new(),
        error: None,
        edges: 0,
        clips: 0,
        layers: 0,
    };
    face.paint_color_glyph(
        GlyphId(glyph),
        0,
        ttf::RgbaColor::new(255, 255, 255, 255),
        &mut painter,
    )
    .ok_or(ColorFontError::InvalidFont)?;
    if let Some(e) = painter.error {
        return Err(e);
    }
    if painter.clips != 0 || painter.layers != 0 || painter.transforms.len() != 1 {
        return Err(ColorFontError::InvalidFont);
    }
    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    let root = match painter.ops.first() {
        Some(Op::Clip(index))
            if face
                .tables()
                .colr
                .and_then(|t| t.clip_box(GlyphId(glyph), &[]))
                .is_some() =>
        {
            Some(*index)
        }
        _ => None,
    };
    for (i, path) in painter.paths.iter().enumerate() {
        if root.is_some_and(|root| root != i) {
            continue;
        }
        for e in path {
            for p in [e.a, e.b] {
                bounds[0] = bounds[0].min(p[0]);
                bounds[1] = bounds[1].min(p[1]);
                bounds[2] = bounds[2].max(p[0]);
                bounds[3] = bounds[3].max(p[1]);
            }
        }
    }
    if bounds.iter().any(|n| !n.is_finite()) {
        return Err(ColorFontError::InvalidFont);
    }
    let origin = [
        floor_f32(bounds[0]) as i32 - 1,
        floor_f32(bounds[1]) as i32 - 1,
    ];
    let width = (-floor_f32(-bounds[2]) as i32 - origin[0] + 1) as u32;
    let height = (-floor_f32(-bounds[3]) as i32 - origin[1] + 1) as u32;
    if width > MAX_PIXELS
        || height > MAX_PIXELS
        || (width as usize * height as usize * 16)
            .checked_mul(painter.edges + painter.ops.len())
            .is_none_or(|n| n > MAX_WORK)
    {
        return Err(ColorFontError::TooComplex);
    }
    let mut pixels = vec![0; (width * height * 4) as usize];
    let mut inside = vec![false; painter.paths.len()];
    for y in 0..height {
        for x in 0..width {
            let mut total = [0.; 4];
            for sy in 0..4 {
                for sx in 0..4 {
                    let p = [
                        origin[0] as f32 + x as f32 + (sx as f32 + 0.5) * 0.25,
                        origin[1] as f32 + y as f32 + (sy as f32 + 0.5) * 0.25,
                    ];
                    for (i, path) in painter.paths.iter().enumerate() {
                        inside[i] = path.iter().map(|e| e.winding_at(p)).sum::<i32>() != 0;
                    }
                    let mut clips = [true; MAX_DEPTH];
                    let mut clip_depth = 0;
                    let mut layers = [[0.; 4]; MAX_DEPTH];
                    let mut modes = [CompositeMode::SourceOver; MAX_DEPTH];
                    let mut layer = 0;
                    for op in &painter.ops {
                        match op {
                            Op::Clip(i) => {
                                clips[clip_depth] =
                                    inside[*i] && (clip_depth == 0 || clips[clip_depth - 1]);
                                clip_depth += 1;
                            }
                            Op::Unclip => clip_depth -= 1,
                            Op::Fill(brush, m) if clip_depth == 0 || clips[clip_depth - 1] => {
                                layers[layer] = composite(
                                    brush.sample(point(*m, p)),
                                    layers[layer],
                                    CompositeMode::SourceOver,
                                );
                            }
                            Op::Layer(mode) => {
                                layer += 1;
                                layers[layer] = [0.; 4];
                                modes[layer] = *mode;
                            }
                            Op::EndLayer => {
                                layers[layer - 1] =
                                    composite(layers[layer], layers[layer - 1], modes[layer]);
                                layer -= 1;
                            }
                            _ => {}
                        }
                    }
                    for i in 0..4 {
                        total[i] += layers[0][i] / 16.;
                    }
                }
            }
            let offset = ((y * width + x) * 4) as usize;
            if total[3] > 0. {
                for i in 0..3 {
                    pixels[offset + i] = srgb::encode(total[i] / total[3]);
                }
            }
            pixels[offset + 3] = (total[3] * 255. + 0.5).clamp(0., 255.) as u8;
        }
    }
    Ok(Bitmap {
        pixels,
        width,
        height,
        origin,
    })
}

#[cfg(test)]
mod tests;
