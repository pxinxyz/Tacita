use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

use crate::raster;
mod color;
mod subpixel;
pub use color::ColorFontError;
mod shaping;
pub use shaping::{
    CaretPosition, RangePair, ShapedGlyph, ShapedOptions, ShapedText, TextDirection,
};

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextRasterMode {
    #[default]
    Legacy = 0,
    Subpixel = 1,
}

const ASCII_FIRST: char = '\u{20}';
const ASCII_LAST: char = '\u{7E}';

const FALLBACK_CHAR: char = '?';

const PAD: u32 = 1;

const INITIAL_ATLAS_SIZE: u32 = 128;

const MAX_ATLAS_SIZE: u32 = 2048;

pub(crate) const MAX_SHAPE_MASK_DIM: u32 = 512;

pub(crate) fn quantize_quarter_px(px: f32) -> u32 {
    (px * 4.0 + 0.5) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ShapeKey {
    Disk { radius_q: u32 },
    Ring { radius_q: u32, thickness_q: u32 },
    RoundedCorner { r_q: u32 },
    RoundedRing { r_q: u32, t_q: u32 },
}

impl ShapeKey {
    pub(crate) fn disk(radius: f32) -> Self {
        Self::Disk {
            radius_q: quantize_quarter_px(radius),
        }
    }

    pub(crate) fn ring(radius: f32, thickness: f32) -> Self {
        Self::Ring {
            radius_q: quantize_quarter_px(radius),
            thickness_q: quantize_quarter_px(thickness),
        }
    }

    pub(crate) fn rounded_corner(radius: f32) -> Self {
        Self::RoundedCorner {
            r_q: quantize_quarter_px(radius),
        }
    }

    pub(crate) fn rounded_ring(radius: f32, thickness: f32) -> Self {
        Self::RoundedRing {
            r_q: quantize_quarter_px(radius),
            t_q: quantize_quarter_px(thickness),
        }
    }

    fn bitmap_dims(&self) -> (u64, u64) {
        let diameter_q = match *self {
            ShapeKey::Disk { radius_q } => 2 * u64::from(radius_q),
            ShapeKey::Ring {
                radius_q,
                thickness_q,
            } => 2 * u64::from(radius_q) + u64::from(thickness_q),
            ShapeKey::RoundedCorner { r_q } | ShapeKey::RoundedRing { r_q, .. } => u64::from(r_q),
        };
        let d = diameter_q.div_ceil(4) + 2;
        (d, d)
    }

    fn bitmap_dims_at_scale(&self, raster_scale: f32) -> (u64, u64) {
        if raster_scale == 1.0 {
            return self.bitmap_dims();
        }
        let diameter_q = match *self {
            ShapeKey::Disk { radius_q } => 2 * u64::from(radius_q),
            ShapeKey::Ring {
                radius_q,
                thickness_q,
            } => 2 * u64::from(radius_q) + u64::from(thickness_q),
            ShapeKey::RoundedCorner { r_q } | ShapeKey::RoundedRing { r_q, .. } => u64::from(r_q),
        };
        let diameter = diameter_q as f64 * raster_scale as f64 / 4.0;
        let whole = diameter as u64;
        let d = whole
            .saturating_add(u64::from(diameter > whole as f64))
            .saturating_add(2);
        (d, d)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapeMask {
    pub rect: [u32; 4],
    pub raster_scale: f32,
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

#[derive(Debug, Clone, Copy)]
pub struct IconFont<'a> {
    pub bytes: &'a [u8],
    pub range: (u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontError {
    InvalidFont(&'static str),
    MissingHorizontalMetrics,
    AtlasTooSmall,
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontError::InvalidFont(reason) => write!(f, "invalid font bytes: {reason}"),
            FontError::MissingHorizontalMetrics => {
                write!(f, "font has no horizontal line metrics")
            }
            FontError::AtlasTooSmall => write!(f, "baked glyphs do not fit in the font atlas"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontId(pub u32);

impl FontId {
    pub const DEFAULT: FontId = FontId(0);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontInfo {
    pub px_size: f32,
    pub ascent: f32,
    pub line_height: f32,
    pub cached_glyphs: usize,
}

#[derive(Debug, Clone, Copy)]
struct GlyphInfo {
    advance: f32,
    xmin: i32,
    ymin: i32,
    rect: [u32; 4],
    source: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GlyphQuad {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

#[derive(Clone)]
struct Face {
    inner: fontdue::Font,
    source_bytes: Vec<u8>,
    indexed: BTreeMap<u16, GlyphInfo>,
    indexed_phases: BTreeMap<u16, Option<subpixel::PhaseGlyph>>,
    color_glyphs: BTreeMap<u16, Result<color::atlas::Entry, ColorFontError>>,
    has_color: bool,
    fallback: Vec<FontId>,
    icon_sources: Vec<fontdue::Font>,
    px_size: f32,
    ascent: f32,
    line_height: f32,
    glyphs: BTreeMap<char, GlyphInfo>,
    phases: BTreeMap<char, Option<subpixel::PhaseGlyph>>,
}

#[derive(Clone)]
pub struct Font {
    color_atlas: color::atlas::Atlas,
    color_texture: Option<crate::draw::TextureId>,
    layout_key: u64,
    raster_scale: f32,
    text_raster_mode: TextRasterMode,
    faces: Vec<Face>,
    atlas: Vec<u8>,
    atlas_w: u32,
    atlas_h: u32,
    svg_icons: Vec<([u32; 4], [f32; 2])>,
    svg_sources: Vec<(alloc::string::String, [f32; 2])>,
    max_atlas_size: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    shapes: BTreeMap<ShapeKey, Option<[u32; 4]>>,
    #[cfg(test)]
    raster_count: core::cell::Cell<usize>,
    generation: u64,
}

impl Font {
    pub fn new(bytes: &[u8], px_size: f32) -> Result<Font, FontError> {
        Self::new_with_icons(bytes, px_size, &[])
    }

    pub fn new_with_icons(
        bytes: &[u8],
        px_size: f32,
        icons: &[IconFont<'_>],
    ) -> Result<Font, FontError> {
        Self::new_with_icons_at_scale(bytes, px_size, icons, 1.0)
    }

    pub fn new_with_icons_at_scale(
        bytes: &[u8],
        px_size: f32,
        icons: &[IconFont<'_>],
        raster_scale: f32,
    ) -> Result<Font, FontError> {
        assert!(raster_scale.is_finite() && raster_scale > 0.0);
        let mut font = Font {
            color_atlas: color::atlas::Atlas::default(),
            color_texture: None,
            raster_scale,
            text_raster_mode: TextRasterMode::Legacy,
            faces: Vec::new(),
            layout_key: 0,
            atlas: Vec::new(),
            atlas_w: 0,
            atlas_h: 0,
            svg_icons: Vec::new(),
            svg_sources: Vec::new(),
            max_atlas_size: MAX_ATLAS_SIZE,
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            shapes: BTreeMap::new(),
            #[cfg(test)]
            raster_count: core::cell::Cell::new(0),
            generation: 0,
        };
        font.reset_atlas(INITIAL_ATLAS_SIZE);
        font.add_face_with_icons(bytes, px_size, icons)?;
        Ok(font)
    }

    pub fn rescaled(&self, scale: f32) -> Result<Self, FontError> {
        if !scale.is_finite() || scale <= 0.0 || scale > 8.0 {
            return Err(FontError::InvalidFont("invalid raster density"));
        }
        let mut rebuilt = self.clone();
        rebuilt.raster_scale = scale;
        rebuilt.text_raster_mode = TextRasterMode::Legacy;
        rebuilt.svg_icons.clear();
        rebuilt.svg_sources.clear();
        rebuilt.reset_atlas(INITIAL_ATLAS_SIZE);
        rebuilt.color_atlas = color::atlas::Atlas::default();
        for face in &mut rebuilt.faces {
            face.color_glyphs.clear();
        }
        for (face, original) in self.faces.iter().enumerate() {
            let metrics = original
                .inner
                .horizontal_line_metrics(original.px_size * scale)
                .ok_or(FontError::MissingHorizontalMetrics)?;
            rebuilt.faces[face].ascent = metrics.ascent / scale;
            rebuilt.faces[face].line_height = metrics.new_line_size / scale;
            for (&ch, glyph) in &original.glyphs {
                let Some(source) = glyph.source else {
                    continue;
                };
                let inserted = if source == 0 {
                    rebuilt.insert_glyph(face, ch)
                } else {
                    rebuilt.bake_glyph(face, source, ch)
                };
                if !inserted {
                    return Err(FontError::AtlasTooSmall);
                }
            }
        }
        for (source, size) in &self.svg_sources {
            rebuilt
                .add_svg_icon(source, *size)
                .map_err(|_| FontError::AtlasTooSmall)?;
        }
        for (face, original) in self.faces.iter().enumerate() {
            for (&ch, glyph) in &original.glyphs {
                if glyph.source.is_none() {
                    rebuilt.insert_glyph_or_fallback(face, ch);
                }
            }
        }
        rebuilt.set_text_raster_mode(self.text_raster_mode);
        for (face, original) in self.faces.iter().enumerate() {
            for &index in original.indexed.keys() {
                rebuilt.ensure_indexed(FontId(face as u32), index);
            }
        }
        rebuilt.generation = self.generation.wrapping_add(1);
        for (face, original) in self.faces.iter().enumerate() {
            for &index in original.color_glyphs.keys() {
                rebuilt.ensure_color(FontId(face as u32), index);
            }
        }
        rebuilt.color_atlas.generation = self.color_atlas.generation.wrapping_add(1);
        Ok(rebuilt)
    }

    pub fn raster_scale(&self) -> f32 {
        self.raster_scale
    }

    pub fn text_raster_mode(&self) -> TextRasterMode {
        self.text_raster_mode
    }

    pub fn set_text_raster_mode(&mut self, mode: TextRasterMode) {
        if self.text_raster_mode == mode {
            return;
        }
        self.text_raster_mode = mode;
        if mode == TextRasterMode::Subpixel {
            for face in 0..self.faces.len() {
                self.warm_phases(face);
                let indices: Vec<_> = self.faces[face].indexed.keys().copied().collect();
                for index in indices {
                    self.ensure_indexed(FontId(face as u32), index);
                }
            }
        }
    }

    pub fn add_svg_icon(
        &mut self,
        source: &str,
        size: [f32; 2],
    ) -> Result<crate::svg::SvgIconId, crate::svg::SvgError> {
        let physical = [size[0] * self.raster_scale, size[1] * self.raster_scale];
        if !physical
            .iter()
            .all(|v| v.is_finite() && *v > 0.0 && *v <= 512.0)
        {
            return Err(crate::svg::SvgError::InvalidInput);
        }
        let ceil = |v: f32| v as u32 + u32::from(v > v as u32 as f32);
        let bitmap = crate::svg::rasterize_svg(source, ceil(physical[0]), ceil(physical[1]))?;
        let (x, y) = self
            .place_bitmap(bitmap.width, bitmap.height, &bitmap.pixels)
            .ok_or(crate::svg::SvgError::AtlasFull)?;
        let id = crate::svg::SvgIconId(self.svg_icons.len() as u32);
        self.svg_icons
            .push(([x, y, bitmap.width, bitmap.height], size));
        self.svg_sources
            .push((alloc::string::String::from(source), size));
        Ok(id)
    }
    pub fn has_svg_icon(&self, id: crate::svg::SvgIconId) -> bool {
        (id.0 as usize) < self.svg_icons.len()
    }
    pub fn svg_icon_size(&self, id: crate::svg::SvgIconId) -> Option<[f32; 2]> {
        self.svg_icons.get(id.0 as usize).map(|entry| entry.1)
    }
    pub(crate) fn svg_icon_mask(&self, id: crate::svg::SvgIconId) -> Option<ShapeMask> {
        self.svg_icons
            .get(id.0 as usize)
            .map(|entry| self.shape_mask_at(entry.0))
    }

    pub fn add_face(&mut self, bytes: &[u8], px_size: f32) -> Result<FontId, FontError> {
        self.add_face_with_icons(bytes, px_size, &[])
    }

    pub fn add_face_with_icons(
        &mut self,
        bytes: &[u8],
        px_size: f32,
        icons: &[IconFont<'_>],
    ) -> Result<FontId, FontError> {
        if rustybuzz::Face::from_slice(bytes, 0).is_none() {
            return Err(FontError::InvalidFont("font cannot be shaped"));
        }
        let inner = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(FontError::InvalidFont)?;
        let line_metrics = inner
            .horizontal_line_metrics(px_size * self.raster_scale)
            .ok_or(FontError::MissingHorizontalMetrics)?;

        let index = self.faces.len();
        self.faces.push(Face {
            inner,
            source_bytes: bytes.to_vec(),
            indexed: BTreeMap::new(),
            indexed_phases: BTreeMap::new(),
            color_glyphs: BTreeMap::new(),
            has_color: rustybuzz::Face::from_slice(bytes, 0).is_some_and(|f| {
                f.raw_face()
                    .table(rustybuzz::ttf_parser::Tag::from_bytes(b"COLR"))
                    .is_some()
            }),
            fallback: Vec::new(),
            icon_sources: Vec::new(),
            px_size,
            ascent: line_metrics.ascent / self.raster_scale,
            line_height: line_metrics.new_line_size / self.raster_scale,
            glyphs: BTreeMap::new(),
            phases: BTreeMap::new(),
        });
        if let Err(err) = self.bake_face(index, icons) {
            self.faces.pop();
            return Err(err);
        }
        self.layout_key = bytes
            .iter()
            .fold(self.layout_key ^ u64::from(px_size.to_bits()), |h, b| {
                (h ^ u64::from(*b)).wrapping_mul(1099511628211)
            });
        if self.text_raster_mode == TextRasterMode::Subpixel {
            self.warm_phases(index);
        }
        Ok(FontId(index as u32))
    }

    fn bake_face(&mut self, index: usize, icons: &[IconFont<'_>]) -> Result<(), FontError> {
        for ch in ASCII_FIRST..=ASCII_LAST {
            if !self.insert_glyph(index, ch) {
                return Err(FontError::AtlasTooSmall);
            }
        }
        for icon in icons {
            let icon_font = fontdue::Font::from_bytes(icon.bytes, fontdue::FontSettings::default())
                .map_err(FontError::InvalidFont)?;
            self.faces[index].icon_sources.push(icon_font);
            let source = self.faces[index].icon_sources.len();
            for cp in icon.range.0..=icon.range.1 {
                let Some(ch) = char::from_u32(cp) else {
                    continue;
                };
                if self.faces[index].glyphs.contains_key(&ch) {
                    continue;
                }
                if !self.bake_glyph(index, source, ch) {
                    return Err(FontError::AtlasTooSmall);
                }
            }
        }
        Ok(())
    }

    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    pub fn is_valid(&self, id: FontId) -> bool {
        (id.0 as usize) < self.faces.len()
    }

    pub fn face_info(&self, id: FontId) -> Option<FontInfo> {
        self.faces.get(id.0 as usize).map(|face| FontInfo {
            px_size: face.px_size,
            ascent: face.ascent,
            line_height: face.line_height,
            cached_glyphs: face.glyphs.len(),
        })
    }

    pub fn atlas_pixels(&self) -> (&[u8], u32, u32) {
        (&self.atlas, self.atlas_w, self.atlas_h)
    }

    pub fn atlas_generation(&self) -> u64 {
        self.generation
    }

    pub fn line_height(&self) -> f32 {
        self.line_height_in(FontId::DEFAULT)
    }

    pub(crate) fn line_height_in(&self, id: FontId) -> f32 {
        self.faces[id.0 as usize].line_height
    }

    pub fn measure(&mut self, text: &str) -> [f32; 2] {
        self.measure_in(FontId::DEFAULT, text)
    }

    pub(crate) fn measure_in(&mut self, id: FontId, text: &str) -> [f32; 2] {
        let face = id.0 as usize;
        let mut width = 0.0;
        for ch in text.chars() {
            self.ensure_glyph(face, ch);
            width += self.faces[face].glyphs[&ch].advance;
        }
        [width, self.faces[face].line_height]
    }

    pub(crate) fn char_advance(&mut self, id: FontId, ch: char) -> f32 {
        let face = id.0 as usize;
        self.ensure_glyph(face, ch);
        self.faces[face].glyphs[&ch].advance
    }

    pub(crate) fn glyph_quad(
        &mut self,
        id: FontId,
        ch: char,
        pen: [f32; 2],
    ) -> ([f32; 2], Option<GlyphQuad>) {
        let face = id.0 as usize;
        self.ensure_glyph(face, ch);
        let g = self.faces[face].glyphs[&ch];
        let next_pen = [pen[0] + g.advance, pen[1]];
        let [x, y, w, h] = g.rect;
        if w == 0 || h == 0 {
            return (next_pen, None);
        }
        let baseline = pen[1] + self.faces[face].ascent;
        if self.text_raster_mode == TextRasterMode::Subpixel {
            if let Some(Some(phase)) = self.faces[face].phases.get(&ch) {
                let (px, phase_x) = subpixel::quantize(pen[0] * self.raster_scale);
                let (py, phase_y) = subpixel::quantize(baseline * self.raster_scale);
                let [x, y, w, h] = phase.rect(phase_x, phase_y);
                let min = [
                    (px + phase.origin[0] as f32) / self.raster_scale,
                    (py + phase.origin[1] as f32) / self.raster_scale,
                ];
                return (
                    next_pen,
                    Some(GlyphQuad {
                        min,
                        max: [
                            min[0] + w as f32 / self.raster_scale,
                            min[1] + h as f32 / self.raster_scale,
                        ],
                        uv_min: [
                            x as f32 / self.atlas_w as f32,
                            y as f32 / self.atlas_h as f32,
                        ],
                        uv_max: [
                            (x + w) as f32 / self.atlas_w as f32,
                            (y + h) as f32 / self.atlas_h as f32,
                        ],
                    }),
                );
            }
        }
        let x0 = pen[0] + g.xmin as f32 / self.raster_scale;
        let y1 = baseline - g.ymin as f32 / self.raster_scale;
        let y0 = y1 - h as f32 / self.raster_scale;
        let quad = GlyphQuad {
            min: [x0, y0],
            max: [x0 + w as f32 / self.raster_scale, y1],
            uv_min: [
                x as f32 / self.atlas_w as f32,
                y as f32 / self.atlas_h as f32,
            ],
            uv_max: [
                (x + w) as f32 / self.atlas_w as f32,
                (y + h) as f32 / self.atlas_h as f32,
            ],
        };
        (next_pen, Some(quad))
    }

    pub(crate) fn shape_mask(&mut self, key: ShapeKey) -> Option<ShapeMask> {
        match self.shapes.get(&key) {
            Some(&Some(rect)) => return Some(self.shape_mask_at(rect)),
            Some(None) => return None,
            None => {}
        }
        let (bw, bh) = key.bitmap_dims_at_scale(self.raster_scale);
        if bw > u64::from(MAX_SHAPE_MASK_DIM) || bh > u64::from(MAX_SHAPE_MASK_DIM) {
            return None;
        }
        #[cfg(test)]
        self.raster_count.set(self.raster_count.get() + 1);
        let (w, h, bitmap) = match key {
            ShapeKey::Disk { radius_q } => {
                raster::rasterize_disk(radius_q as f32 / 4.0 * self.raster_scale)
            }
            ShapeKey::Ring {
                radius_q,
                thickness_q,
            } => raster::rasterize_ring(
                radius_q as f32 / 4.0 * self.raster_scale,
                thickness_q as f32 / 4.0 * self.raster_scale,
            ),
            ShapeKey::RoundedCorner { r_q } => {
                raster::rasterize_rounded_corner(r_q as f32 / 4.0 * self.raster_scale)
            }
            ShapeKey::RoundedRing { r_q, t_q } => raster::rasterize_rounded_ring(
                r_q as f32 / 4.0 * self.raster_scale,
                t_q as f32 / 4.0 * self.raster_scale,
            ),
        };
        if w == 0 || h == 0 {
            return None;
        }
        let Some((x, y)) = self.place_bitmap(w, h, &bitmap) else {
            self.shapes.insert(key, None);
            return None;
        };
        let rect = [x, y, w, h];
        self.shapes.insert(key, Some(rect));
        Some(self.shape_mask_at(rect))
    }

    fn shape_mask_at(&self, rect: [u32; 4]) -> ShapeMask {
        let [x, y, w, h] = rect;
        ShapeMask {
            rect,
            raster_scale: self.raster_scale,
            uv_min: [
                x as f32 / self.atlas_w as f32,
                y as f32 / self.atlas_h as f32,
            ],
            uv_max: [
                (x + w) as f32 / self.atlas_w as f32,
                (y + h) as f32 / self.atlas_h as f32,
            ],
        }
    }

    #[cfg(test)]
    pub(crate) fn uv_samples_shape_mask(&self, uv: [f32; 2]) -> bool {
        let x = (uv[0] * self.atlas_w as f32) as i64;
        let y = (uv[1] * self.atlas_h as f32) as i64;
        self.shapes.values().flatten().any(|&[rx, ry, rw, rh]| {
            x >= i64::from(rx)
                && x < i64::from(rx + rw)
                && y >= i64::from(ry)
                && y < i64::from(ry + rh)
        })
    }

    fn ensure_glyph(&mut self, face: usize, ch: char) {
        if !self.faces[face].glyphs.contains_key(&ch) {
            self.insert_glyph_or_fallback(face, ch);
        }
        if self.text_raster_mode == TextRasterMode::Subpixel {
            self.ensure_phases(face, ch);
        }
    }

    fn warm_phases(&mut self, face: usize) {
        let chars: Vec<char> = self.faces[face].glyphs.keys().copied().collect();
        for ch in chars {
            self.ensure_phases(face, ch);
        }
    }

    fn ensure_phases(&mut self, face: usize, ch: char) {
        if self.faces[face].phases.contains_key(&ch) {
            return;
        }
        let glyph = self.faces[face].glyphs[&ch];
        let raster = glyph.source.and_then(|source| {
            let f = &self.faces[face];
            let font = if source == 0 {
                &f.inner
            } else {
                &f.icon_sources[source - 1]
            };
            subpixel::rasterize(font, ch, f.px_size * self.raster_scale, self.max_atlas_size)
        });
        let phase = raster.and_then(|(phase, pixels)| {
            let [w, h] = phase.dimensions();
            self.place_phase_bitmap(w, h, &pixels)
                .map(|(x, y)| phase.placed([x, y]))
        });
        self.faces[face].phases.insert(ch, phase);
    }

    fn reset_atlas(&mut self, size: u32) {
        self.reset_atlas_full(size, size);
    }

    fn reset_atlas_full(&mut self, w: u32, h: u32) {
        self.atlas = alloc::vec![0u8; (w * h) as usize];
        self.atlas_w = w;
        self.atlas_h = h;
        self.atlas[0] = 0xFF;
        for face in &mut self.faces {
            face.glyphs.clear();
            face.phases.clear();
            face.indexed.clear();
            face.indexed_phases.clear();
        }
        self.shapes.clear();
        self.cursor_x = 1 + PAD;
        self.cursor_y = 0;
        self.row_height = 0;
    }

    fn insert_glyph(&mut self, face: usize, ch: char) -> bool {
        let f = &self.faces[face];
        let (metrics, bitmap) = f.inner.rasterize(ch, f.px_size * self.raster_scale);
        self.place_glyph(face, ch, metrics, bitmap, 0)
    }

    fn bake_glyph(&mut self, face: usize, source: usize, ch: char) -> bool {
        let f = &self.faces[face];
        let (metrics, bitmap) =
            f.icon_sources[source - 1].rasterize(ch, f.px_size * self.raster_scale);
        self.place_glyph(face, ch, metrics, bitmap, source)
    }

    fn place_glyph(
        &mut self,
        face: usize,
        ch: char,
        metrics: fontdue::Metrics,
        bitmap: Vec<u8>,
        source: usize,
    ) -> bool {
        let (w, h) = (metrics.width as u32, metrics.height as u32);
        let Some((x, y)) = self.place_bitmap(w, h, &bitmap) else {
            return false;
        };
        self.faces[face].glyphs.insert(
            ch,
            GlyphInfo {
                advance: metrics.advance_width / self.raster_scale,
                xmin: metrics.xmin,
                ymin: metrics.ymin,
                rect: [x, y, w, h],
                source: Some(source),
            },
        );
        true
    }

    fn place_phase_bitmap(&mut self, w: u32, h: u32, bitmap: &[u8]) -> Option<(u32, u32)> {
        let limit = self.max_atlas_size - self.max_atlas_size / 4;
        if h > limit || self.cursor_y + h > limit {
            return None;
        }
        loop {
            let shelf = (self.cursor_x, self.cursor_y, self.row_height);
            if let Some((x, y)) = self.try_place(w, h) {
                if y + h <= limit {
                    self.blit(x, y, w, h, bitmap);
                    return Some((x, y));
                }
            }
            (self.cursor_x, self.cursor_y, self.row_height) = shelf;
            let (w2, h2) = grow_dims(self.atlas_w, self.atlas_h, self.max_atlas_size);
            if (w2, h2) == (self.atlas_w, self.atlas_h) {
                return None;
            }
            self.grow_atlas(w2, h2);
        }
    }

    fn place_bitmap(&mut self, w: u32, h: u32, bitmap: &[u8]) -> Option<(u32, u32)> {
        loop {
            if let Some((x, y)) = self.try_place(w, h) {
                self.blit(x, y, w, h, bitmap);
                return Some((x, y));
            }
            let (w2, h2) = grow_dims(self.atlas_w, self.atlas_h, self.max_atlas_size);
            if (w2, h2) == (self.atlas_w, self.atlas_h) {
                return None;
            }
            self.grow_atlas(w2, h2);
        }
    }

    fn insert_glyph_or_fallback(&mut self, face: usize, ch: char) {
        if self.insert_glyph(face, ch) {
            return;
        }
        let f = &self.faces[face];
        let advance = f
            .inner
            .metrics(ch, f.px_size * self.raster_scale)
            .advance_width
            / self.raster_scale;
        let (rect, xmin, ymin) = f
            .glyphs
            .get(&FALLBACK_CHAR)
            .map(|g| (g.rect, g.xmin, g.ymin))
            .unwrap_or(([0, 0, 0, 0], 0, 0));
        self.faces[face].glyphs.insert(
            ch,
            GlyphInfo {
                advance,
                xmin,
                ymin,
                rect,
                source: None,
            },
        );
    }

    fn try_place(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if w == 0 || h == 0 {
            return Some((0, 0));
        }
        if w + PAD > self.atlas_w {
            return None;
        }
        if self.cursor_x + w > self.atlas_w {
            self.cursor_x = PAD;
            self.cursor_y += self.row_height + PAD;
            self.row_height = 0;
        }
        if self.cursor_y + h > self.atlas_h {
            return None;
        }
        let pos = (self.cursor_x, self.cursor_y);
        self.cursor_x += w + PAD;
        self.row_height = self.row_height.max(h);
        Some(pos)
    }

    fn blit(&mut self, x: u32, y: u32, w: u32, h: u32, bitmap: &[u8]) {
        if w == 0 || h == 0 {
            return;
        }
        for row in 0..h {
            let dst = ((y + row) * self.atlas_w + x) as usize;
            let src = (row * w) as usize;
            self.atlas[dst..dst + w as usize].copy_from_slice(&bitmap[src..src + w as usize]);
        }
        self.generation += 1;
    }

    fn grow_atlas(&mut self, w2: u32, h2: u32) {
        let mut next = alloc::vec![0u8; (w2 * h2) as usize];
        for row in 0..self.atlas_h {
            let src = (row * self.atlas_w) as usize;
            let dst = (row * w2) as usize;
            next[dst..dst + self.atlas_w as usize]
                .copy_from_slice(&self.atlas[src..src + self.atlas_w as usize]);
        }
        self.atlas = next;
        self.atlas_w = w2;
        self.atlas_h = h2;
        self.generation += 1;
    }
}

fn grow_dims(w: u32, h: u32, max: u32) -> (u32, u32) {
    if w <= h && w < max {
        ((w * 2).min(max), h)
    } else if h < max {
        (w, (h * 2).min(max))
    } else {
        (w, h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEJAVU_SANS: &[u8] = include_bytes!("../tests/assets/DejaVuSans.ttf");
    const PX: f32 = 16.0;

    fn test_font() -> Font {
        Font::new(DEJAVU_SANS, PX).expect("DejaVu Sans must load")
    }

    #[test]
    fn rejected_phase_page_preserves_space_on_the_current_shelf() {
        let mut font = test_font();
        font.reset_atlas(64);
        font.max_atlas_size = 64;
        font.cursor_x = 60;
        font.cursor_y = 30;
        font.row_height = 18;
        let shelf = (font.cursor_x, font.cursor_y, font.row_height);
        assert!(font.place_phase_bitmap(10, 10, &[0; 100]).is_none());
        assert_eq!((font.cursor_x, font.cursor_y, font.row_height), shelf);
        assert_eq!(font.place_bitmap(2, 2, &[255; 4]), Some((60, 30)));
    }

    #[test]
    fn subpixel_warmup_leaves_room_for_shaped_and_unicode_glyphs() {
        let mut font = Font::new_with_icons(
            DEJAVU_SANS,
            PX,
            &[IconFont {
                bytes: FA_SOLID,
                range: (0xf005, 0xf071),
            }],
        )
        .unwrap();
        for size in [12.0, 24.0, 32.0] {
            font.add_face(DEJAVU_SANS, size).unwrap();
        }
        for _ in 0..3 {
            for size in [18.0, 24.0, 32.0] {
                font.add_svg_icon(
                    r#"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/></svg>"#,
                    [size; 2],
                )
                .unwrap();
            }
        }
        font.add_face(FA_SOLID, PX).unwrap();
        font.set_text_raster_mode(TextRasterMode::Subpixel);
        for ch in ['é', 'ö', 'א', 'س'] {
            font.glyph_quad(FontId::DEFAULT, ch, [0.0; 2]);
            assert_eq!(
                font.faces[0].glyphs[&ch].source,
                Some(0),
                "{ch} must have real coverage"
            );
        }
        let mut shaped = ShapedText::default();
        shaped
            .prepare(
                &mut font,
                FontId::DEFAULT,
                "office café سلام אבג",
                ShapedOptions::default(),
            )
            .unwrap();
        let mut draw = crate::draw::DrawList::new();
        shaped.draw(&mut font, &mut draw, [0.0; 2], [255; 4]);
        for glyph in shaped.glyphs() {
            assert_eq!(
                font.faces[glyph.font.0 as usize].indexed[&glyph.index].source,
                Some(0)
            );
        }
    }

    #[test]
    fn subpixel_mode_preserves_measurements_and_legacy_geometry() {
        let mut font = test_font();
        let pen = [10.25, 20.5];
        let measured = font.measure("Hamburgefonts café");
        let (advance, legacy) = font.glyph_quad(FontId::DEFAULT, 'H', pen);
        let legacy = legacy.unwrap();
        assert_eq!(font.text_raster_mode(), TextRasterMode::Legacy);
        font.set_text_raster_mode(TextRasterMode::Subpixel);
        assert_eq!(font.measure("Hamburgefonts café"), measured);
        let (next, phase) = font.glyph_quad(FontId::DEFAULT, 'H', pen);
        assert_eq!(next, advance);
        let phase = phase.unwrap();
        assert_eq!(phase.min[0].fract(), 0.0);
        assert_eq!(phase.min[1].fract(), 0.0);
        font.set_text_raster_mode(TextRasterMode::Legacy);
        let (_, restored) = font.glyph_quad(FontId::DEFAULT, 'H', pen);
        assert_eq!(restored.unwrap().min, legacy.min);
        assert_eq!(restored.unwrap().max, legacy.max);
    }

    #[test]
    fn subpixel_density_and_all_origin_phases_reuse_the_atlas() {
        let mut font = Font::new_with_icons_at_scale(DEJAVU_SANS, PX, &[], 1.5).unwrap();
        font.set_text_raster_mode(TextRasterMode::Subpixel);
        let generation = font.atlas_generation();
        let phase = font.faces[0].phases[&'H'].unwrap();
        let mut rects = alloc::collections::BTreeSet::new();
        for y in 0..4_u8 {
            for x in 0..4_u8 {
                let pen = [
                    (-10.0 + f32::from(x) * 0.25) / 1.5,
                    (f32::from(y) * 0.25) / 1.5 - font.faces[0].ascent,
                ];
                let (_, quad) = font.glyph_quad(FontId::DEFAULT, 'H', pen);
                let quad = quad.unwrap();
                for coordinate in quad.min.into_iter().chain(quad.max) {
                    assert!((coordinate * 1.5 - (coordinate * 1.5).round()).abs() < 0.0001);
                }
                let rect = phase.rect(x, y);
                assert!(rects.insert(rect));
                assert_eq!(
                    quad.uv_min,
                    [
                        rect[0] as f32 / font.atlas_w as f32,
                        rect[1] as f32 / font.atlas_h as f32,
                    ]
                );
                assert_eq!(font.atlas_generation(), generation);
            }
        }
        assert_eq!(rects.len(), 16);
        font.set_text_raster_mode(TextRasterMode::Legacy);
        font.set_text_raster_mode(TextRasterMode::Subpixel);
        assert_eq!(font.atlas_generation(), generation);
        let id = font.add_face(DEJAVU_SANS, 24.0).unwrap();
        assert!(font.faces[id.0 as usize].phases[&'H'].is_some());
    }

    #[test]
    fn subpixel_icon_coverage_uses_the_winning_icon_source() {
        let mut font = icon_font();
        font.set_text_raster_mode(TextRasterMode::Subpixel);
        let ch = '\u{f005}';
        assert_eq!(font.faces[0].glyphs[&ch].source, Some(1));
        let cached = font.faces[0].phases[&ch].unwrap();
        let (reference, pixels) =
            subpixel::rasterize(&font.faces[0].icon_sources[0], ch, PX, MAX_ATLAS_SIZE).unwrap();
        let reference_width = reference.dimensions()[0];
        for y in 0..4 {
            for x in 0..4 {
                let actual = cached.rect(x, y);
                let expected = reference.rect(x, y);
                for row in 0..actual[3] {
                    let a = ((actual[1] + row) * font.atlas_w + actual[0]) as usize;
                    let e = ((expected[1] + row) * reference_width + expected[0]) as usize;
                    assert_eq!(
                        &font.atlas[a..a + actual[2] as usize],
                        &pixels[e..e + expected[2] as usize]
                    );
                }
            }
        }
    }

    #[test]
    fn subpixel_atlas_full_keeps_base_glyph_and_caches_failure() {
        let mut font = test_font();
        let cap = font.atlas_w.max(font.atlas_h);
        font.grow_atlas(cap, cap);
        font.max_atlas_size = cap;
        font.cursor_x = PAD;
        font.cursor_y = cap;
        font.row_height = 0;
        let (_, before) = font.glyph_quad(FontId::DEFAULT, 'H', [10.25, 20.5]);
        font.set_text_raster_mode(TextRasterMode::Subpixel);
        assert!(font.faces[0].phases[&'H'].is_none());
        let generation = font.atlas_generation();
        for _ in 0..3 {
            let (_, after) = font.glyph_quad(FontId::DEFAULT, 'H', [10.25, 20.5]);
            assert_eq!(after, before);
            assert_eq!(font.atlas_generation(), generation);
        }
        assert_eq!(font.atlas[0], 255);
    }

    #[test]
    fn scaled_atlas_keeps_logical_layout_and_rasterizes_more_pixels() {
        let mut base = test_font();
        let mut scaled = Font::new_with_icons_at_scale(DEJAVU_SANS, PX, &[], 1.5).unwrap();
        assert_eq!(scaled.raster_scale(), 1.5);
        assert_eq!(scaled.face_info(FontId::DEFAULT).unwrap().px_size, PX);
        assert!((scaled.line_height() - base.line_height()).abs() < 0.01);
        assert!(scaled.faces[0].glyphs[&'H'].rect[2] > base.faces[0].glyphs[&'H'].rect[2]);
        let text = "Hello";
        assert!((scaled.measure(text)[0] - base.measure(text)[0]).abs() < 0.1);
        let pen = [40.0, 50.0];
        let (next, quad) = scaled.glyph_quad(FontId::DEFAULT, 'H', pen);
        let glyph = scaled.faces[0].glyphs[&'H'];
        let quad = quad.unwrap();
        assert_eq!(next, [pen[0] + glyph.advance, pen[1]]);
        assert!(((quad.max[0] - quad.min[0]) - glyph.rect[2] as f32 / 1.5).abs() < 1e-4);
        let mask = scaled.shape_mask(ShapeKey::disk(10.0)).unwrap();
        assert_eq!(mask.rect[2], 32);
        assert_eq!(mask.raster_scale, 1.5);
    }

    #[test]
    fn new_rejects_garbage_bytes() {
        assert!(Font::new(b"this is not a font", PX).is_err());
        assert!(Font::new(&[], PX).is_err());
    }

    #[test]
    fn bakes_printable_ascii_and_reserves_white_pixel() {
        let font = test_font();
        let (pixels, w, h) = font.atlas_pixels();
        assert!(w > 0 && h > 0);
        assert_eq!(pixels.len(), (w * h) as usize);
        assert_eq!(pixels[0], 0xFF);
        for ch in ASCII_FIRST..=ASCII_LAST {
            assert!(
                font.faces[0].glyphs.contains_key(&ch),
                "glyph {ch:?} not baked"
            );
        }
        for g in font.faces[0].glyphs.values() {
            let [x, y, gw, gh] = g.rect;
            assert!(x + gw <= w && y + gh <= h);
            assert!(
                gw == 0 || y > 0 || x > PAD,
                "glyph overlaps white pixel gap"
            );
        }
    }

    #[test]
    fn baked_glyph_rects_do_not_overlap() {
        let font = test_font();
        let mut rects: Vec<[u32; 4]> = font.faces[0]
            .glyphs
            .values()
            .map(|g| g.rect)
            .filter(|[_, _, w, h]| *w > 0 && *h > 0)
            .collect();
        rects.sort();
        for a in 0..rects.len() {
            for b in a + 1..rects.len() {
                let [ax, ay, aw, ah] = rects[a];
                let [bx, by, bw, bh] = rects[b];
                let separated = ax + aw <= bx || bx + bw <= ax || ay + ah <= by || by + bh <= ay;
                let (ra, rb) = (rects[a], rects[b]);
                assert!(separated, "glyph rects {ra:?} and {rb:?} overlap");
            }
        }
    }

    #[test]
    fn measure_returns_advance_sum_and_line_height() {
        let mut font = test_font();
        let [w, h] = font.measure("Hello");
        assert!(w > 0.0);
        assert_eq!(h, font.line_height());
        assert_eq!(font.measure("")[0], 0.0);
        let [h_w, _] = font.measure("H");
        let [ello_w, _] = font.measure("ello");
        assert_eq!(w, h_w + ello_w);
    }

    #[test]
    fn non_ascii_glyph_is_cached_on_demand() {
        let mut font = test_font();
        assert!(!font.faces[0].glyphs.contains_key(&'é'));
        let [w, _] = font.measure("é");
        assert!(w > 0.0);
        let cached = font.faces[0].glyphs[&'é'];
        assert!(cached.rect[2] > 0 && cached.rect[3] > 0);
        let (pen, quad) = font.glyph_quad(FontId::DEFAULT, 'é', [0.0, 0.0]);
        assert!(quad.is_some());
        assert_eq!(pen[0], cached.advance);
    }

    #[test]
    fn full_atlas_falls_back_to_question_mark_rect() {
        let mut font = test_font();
        font.max_atlas_size = font.atlas_w.min(font.atlas_h);
        font.cursor_y = font.atlas_h;
        let [w, _] = font.measure("Ω");
        assert!(w > 0.0, "fallback keeps the char's own advance");
        let omega = font.faces[0].glyphs[&'Ω'];
        let question = font.faces[0].glyphs[&FALLBACK_CHAR];
        assert_eq!(omega.rect, question.rect);
        assert_eq!(omega.xmin, question.xmin);
        assert_eq!(omega.ymin, question.ymin);
        let (_, quad) = font.glyph_quad(FontId::DEFAULT, 'Ω', [10.0, 20.0]);
        assert!(quad.is_some());
    }

    #[test]
    fn growth_preserves_existing_glyphs_and_bumps_generation() {
        let mut font = test_font();
        let rect = font.faces[0].glyphs[&'H'].rect;
        let (old_w, old_h) = (font.atlas_w, font.atlas_h);
        let bytes_before: Vec<u8> = rect_bytes(&font, rect);
        let gen_before = font.atlas_generation();

        font.grow_atlas(old_w * 2, old_h * 2);
        assert!(font.atlas_generation() > gen_before);
        assert!(font.insert_glyph(0, 'é'));

        assert_eq!(font.atlas[0], 0xFF);
        assert_eq!(font.faces[0].glyphs[&'H'].rect, rect);
        assert_eq!(rect_bytes(&font, rect), bytes_before);
        for g in font.faces[0].glyphs.values() {
            assert!(g.rect[0] + g.rect[2] <= old_w * 2);
            assert!(g.rect[1] + g.rect[3] <= old_h * 2);
        }
        let cached = font.faces[0].glyphs[&'é'];
        assert!(cached.rect[0] + cached.rect[2] <= font.atlas_w);
        assert!(cached.rect[1] + cached.rect[3] <= font.atlas_h);
    }

    fn rect_bytes(font: &Font, rect: [u32; 4]) -> Vec<u8> {
        let [x, y, w, h] = rect;
        let mut out = Vec::new();
        for row in 0..h {
            let start = ((y + row) * font.atlas_w + x) as usize;
            out.extend_from_slice(&font.atlas[start..start + w as usize]);
        }
        out
    }

    #[test]
    fn glyph_quad_positions_and_uvs() {
        let mut font = test_font();
        let pen = [100.0, 50.0];
        let (next_pen, quad) = font.glyph_quad(FontId::DEFAULT, 'H', pen);
        let quad = quad.expect("'H' has ink");
        let g = font.faces[0].glyphs[&'H'];
        assert_eq!(next_pen, [pen[0] + g.advance, pen[1]]);
        let baseline = pen[1] + font.faces[0].ascent;
        assert_eq!(quad.min[0], pen[0] + g.xmin as f32);
        assert_eq!(quad.max[1], baseline - g.ymin as f32);
        assert_eq!(quad.max[0] - quad.min[0], g.rect[2] as f32);
        assert_eq!(quad.max[1] - quad.min[1], g.rect[3] as f32);
        for uv in [quad.uv_min, quad.uv_max] {
            assert!((0.0..=1.0).contains(&uv[0]));
            assert!((0.0..=1.0).contains(&uv[1]));
        }
        assert!(quad.uv_min[0] < quad.uv_max[0]);
        assert!(quad.uv_min[1] < quad.uv_max[1]);
    }

    #[test]
    fn space_advances_pen_without_a_quad() {
        let mut font = test_font();
        let (next_pen, quad) = font.glyph_quad(FontId::DEFAULT, ' ', [0.0, 0.0]);
        assert!(quad.is_none());
        assert!(next_pen[0] > 0.0);
        assert_eq!(next_pen[1], 0.0);
    }

    #[test]
    fn shape_mask_miss_inserts_and_bumps_generation() {
        let mut font = test_font();
        let gen_before = font.atlas_generation();
        let mask = font
            .shape_mask(ShapeKey::disk(10.0))
            .expect("10px disk fits");
        assert!(font.atlas_generation() > gen_before);
        assert_eq!(mask.rect[2], 22);
        assert_eq!(mask.rect[3], 22);
        let (_, aw, ah) = font.atlas_pixels();
        assert!(mask.rect[0] + mask.rect[2] <= aw);
        assert!(mask.rect[1] + mask.rect[3] <= ah);
        for uv in [mask.uv_min, mask.uv_max] {
            assert!((0.0..=1.0).contains(&uv[0]));
            assert!((0.0..=1.0).contains(&uv[1]));
        }
        assert!(mask.uv_min[0] < mask.uv_max[0]);
        assert!(mask.uv_min[1] < mask.uv_max[1]);
        assert_eq!(mask.uv_min[0], mask.rect[0] as f32 / aw as f32);
        assert_eq!(
            mask.uv_max[1],
            (mask.rect[1] + mask.rect[3]) as f32 / ah as f32
        );
    }

    #[test]
    fn shape_mask_hit_returns_same_entry_without_atlas_mutation() {
        let mut font = test_font();
        let key = ShapeKey::disk(12.5);
        let first = font.shape_mask(key).unwrap();
        let gen = font.atlas_generation();
        let atlas_before = font.atlas.clone();
        let second = font.shape_mask(key).unwrap();
        assert_eq!(first, second);
        assert_eq!(font.atlas_generation(), gen);
        assert_eq!(font.atlas, atlas_before);
    }

    #[test]
    fn shape_key_quantizes_to_quarter_pixels() {
        assert_eq!(ShapeKey::disk(10.01), ShapeKey::disk(10.02));
        assert_ne!(ShapeKey::disk(10.01), ShapeKey::disk(10.26));
        assert_eq!(ShapeKey::ring(10.01, 2.01), ShapeKey::ring(10.02, 2.02));
        assert_ne!(ShapeKey::ring(10.01, 2.0), ShapeKey::ring(10.01, 2.26));
    }

    #[test]
    fn oversized_shape_returns_none_without_atlas_mutation() {
        let mut font = test_font();
        let gen = font.atlas_generation();
        let atlas_before = font.atlas.clone();
        assert!(font.shape_mask(ShapeKey::disk(300.0)).is_none());
        assert!(font.shape_mask(ShapeKey::disk(256.0)).is_none());
        assert!(font.shape_mask(ShapeKey::ring(300.0, 10.0)).is_none());
        assert!(font.shape_mask(ShapeKey::disk(1.0e9)).is_none());
        assert_eq!(font.atlas_generation(), gen);
        assert_eq!(font.atlas, atlas_before);
        assert!(font.shape_mask(ShapeKey::disk(255.0)).is_some());
    }

    #[test]
    fn degenerate_shape_returns_none() {
        let mut font = test_font();
        let gen = font.atlas_generation();
        assert!(font.shape_mask(ShapeKey::disk(0.0)).is_none());
        assert!(font.shape_mask(ShapeKey::disk(-1.0)).is_none());
        assert!(font.shape_mask(ShapeKey::disk(f32::NAN)).is_none());
        assert!(font.shape_mask(ShapeKey::ring(10.0, 0.0)).is_none());
        assert_eq!(font.atlas_generation(), gen);
    }

    #[test]
    fn mask_insertions_preserve_glyphs_and_white_pixel() {
        let mut font = test_font();
        let h_rect = font.faces[0].glyphs[&'H'].rect;
        let h_bytes = rect_bytes(&font, h_rect);
        font.shape_mask(ShapeKey::disk(7.0)).unwrap();
        font.shape_mask(ShapeKey::ring(20.0, 4.0)).unwrap();
        font.shape_mask(ShapeKey::disk(255.0)).unwrap();
        assert_eq!(font.atlas[0], 0xFF);
        assert_eq!(font.faces[0].glyphs[&'H'].rect, h_rect);
        assert_eq!(rect_bytes(&font, h_rect), h_bytes);
        let (_, quad) = font.glyph_quad(FontId::DEFAULT, 'H', [0.0, 0.0]);
        assert!(quad.is_some());
    }

    #[test]
    fn shape_mask_hit_after_growth_recomputes_uvs_with_same_rect() {
        let mut font = test_font();
        let key = ShapeKey::disk(10.0);
        let before = font.shape_mask(key).unwrap();
        let old_dims = (font.atlas_w, font.atlas_h);
        font.shape_mask(ShapeKey::disk(255.0)).unwrap();
        assert!((font.atlas_w, font.atlas_h) != old_dims);
        let after = font.shape_mask(key).unwrap();
        assert_eq!(before.rect, after.rect);
        assert!(after.uv_min[0] <= before.uv_min[0]);
        assert!(after.uv_min[1] <= before.uv_min[1]);
        assert!(after.uv_min[0] < before.uv_min[0] || after.uv_min[1] < before.uv_min[1]);
    }

    #[test]
    fn degenerate_ring_caches_same_bitmap_as_equivalent_disk() {
        let mut font = test_font();
        let ring = font.shape_mask(ShapeKey::ring(4.0, 10.0)).unwrap();
        let disk = font.shape_mask(ShapeKey::disk(9.0)).unwrap();
        assert_eq!(ring.rect[2..], disk.rect[2..]);
        assert_eq!(rect_bytes(&font, ring.rect), rect_bytes(&font, disk.rect));
        assert_ne!(ring.rect, disk.rect);
    }

    #[test]
    fn mask_rect_dims_match_bitmap_dim_for_all_quarter_phases() {
        let mut font = test_font();
        for radius in [10.0_f32, 10.25, 10.5, 10.75] {
            let key = ShapeKey::disk(radius);
            let mask = font.shape_mask(key).unwrap();
            let (bw, bh) = key.bitmap_dims();
            assert_eq!(u64::from(mask.rect[2]), bw, "disk r={radius}");
            assert_eq!(u64::from(mask.rect[3]), bh, "disk r={radius}");
        }
        let key = ShapeKey::ring(10.5, 3.25);
        let mask = font.shape_mask(key).unwrap();
        let (bw, bh) = key.bitmap_dims();
        assert_eq!(u64::from(mask.rect[2]), bw, "ring");
        assert_eq!(u64::from(mask.rect[3]), bh, "ring");
    }

    #[test]
    fn failed_placement_is_negative_cached_without_re_rasterizing() {
        let mut font = test_font();
        font.max_atlas_size = font.atlas_w.min(font.atlas_h);
        font.cursor_y = font.atlas_h;
        let key = ShapeKey::disk(10.0);
        assert!(font.shape_mask(key).is_none());
        assert_eq!(font.raster_count.get(), 1, "first lookup rasterizes");
        let gen = font.atlas_generation();
        assert!(font.shape_mask(key).is_none());
        assert_eq!(
            font.raster_count.get(),
            1,
            "repeat lookup must not re-rasterize"
        );
        assert_eq!(font.atlas_generation(), gen);
        assert!(font.shape_mask(ShapeKey::disk(11.0)).is_none());
        assert_eq!(font.raster_count.get(), 2);
    }

    #[test]
    fn rounded_corner_key_quantizes_to_quarter_pixels() {
        assert_eq!(
            ShapeKey::rounded_corner(4.01),
            ShapeKey::rounded_corner(4.02)
        );
        assert_ne!(
            ShapeKey::rounded_corner(4.01),
            ShapeKey::rounded_corner(4.26)
        );
    }

    #[test]
    fn rounded_corner_mask_miss_inserts_and_hit_leaves_atlas_untouched() {
        let mut font = test_font();
        let key = ShapeKey::rounded_corner(4.0);
        let gen = font.atlas_generation();
        let first = font.shape_mask(key).expect("a 4px corner fits");
        assert!(font.atlas_generation() > gen, "a miss bumps the generation");
        assert_eq!(first.rect[2], 6);
        assert_eq!(first.rect[3], 6);
        let (_, aw, ah) = font.atlas_pixels();
        assert!(first.rect[0] + first.rect[2] <= aw);
        assert!(first.rect[1] + first.rect[3] <= ah);
        let gen = font.atlas_generation();
        let atlas_before = font.atlas.clone();
        assert_eq!(font.shape_mask(key), Some(first));
        assert_eq!(font.atlas_generation(), gen);
        assert_eq!(font.atlas, atlas_before);
    }

    #[test]
    fn rounded_corner_mask_dims_match_bitmap_dims_for_all_quarter_phases() {
        let mut font = test_font();
        for r in [4.0_f32, 4.25, 4.5, 4.75] {
            let key = ShapeKey::rounded_corner(r);
            let mask = font.shape_mask(key).unwrap();
            let (bw, bh) = key.bitmap_dims();
            assert_eq!(u64::from(mask.rect[2]), bw, "r={r}");
            assert_eq!(u64::from(mask.rect[3]), bh, "r={r}");
        }
    }

    #[test]
    fn oversized_rounded_corner_returns_none_before_rasterizing() {
        let mut font = test_font();
        let gen = font.atlas_generation();
        assert!(font.shape_mask(ShapeKey::rounded_corner(600.0)).is_none());
        assert!(font.shape_mask(ShapeKey::rounded_corner(1.0e9)).is_none());
        assert_eq!(font.atlas_generation(), gen);
        assert!(font.shape_mask(ShapeKey::rounded_corner(510.0)).is_some());
    }

    #[test]
    fn degenerate_rounded_corner_returns_none() {
        let mut font = test_font();
        let gen = font.atlas_generation();
        assert!(font.shape_mask(ShapeKey::rounded_corner(0.0)).is_none());
        assert!(font.shape_mask(ShapeKey::rounded_corner(-1.0)).is_none());
        assert!(font
            .shape_mask(ShapeKey::rounded_corner(f32::NAN))
            .is_none());
        assert_eq!(font.atlas_generation(), gen);
    }

    #[test]
    fn rounded_ring_key_quantizes_to_quarter_pixels() {
        assert_eq!(
            ShapeKey::rounded_ring(4.01, 1.01),
            ShapeKey::rounded_ring(4.02, 1.02)
        );
        assert_ne!(
            ShapeKey::rounded_ring(4.01, 1.0),
            ShapeKey::rounded_ring(4.26, 1.0)
        );
        assert_ne!(
            ShapeKey::rounded_ring(4.0, 1.01),
            ShapeKey::rounded_ring(4.0, 1.26)
        );
    }

    #[test]
    fn rounded_ring_mask_miss_inserts_and_hit_reuses_the_patch() {
        let mut font = test_font();
        let key = ShapeKey::rounded_ring(4.0, 1.0);
        let gen = font.atlas_generation();
        let first = font.shape_mask(key).expect("a 4px/1px ring corner fits");
        assert!(font.atlas_generation() > gen, "a miss bumps the generation");
        assert_eq!(first.rect[2], 6);
        assert_eq!(first.rect[3], 6);
        let (_, aw, ah) = font.atlas_pixels();
        assert!(first.rect[0] + first.rect[2] <= aw);
        assert!(first.rect[1] + first.rect[3] <= ah);
        let gen = font.atlas_generation();
        let atlas_before = font.atlas.clone();
        assert_eq!(font.shape_mask(key), Some(first));
        assert_eq!(font.atlas_generation(), gen);
        assert_eq!(font.atlas, atlas_before);
        assert!(font.shape_mask(ShapeKey::rounded_ring(5.0, 1.0)).is_some());
        assert!(font.atlas_generation() > gen);
        let gen = font.atlas_generation();
        assert!(font.shape_mask(ShapeKey::rounded_ring(4.0, 2.0)).is_some());
        assert!(font.atlas_generation() > gen);
    }

    #[test]
    fn rounded_ring_mask_dims_match_bitmap_dims_for_all_quarter_phases() {
        let mut font = test_font();
        for r in [4.0_f32, 4.25, 4.5, 4.75] {
            let key = ShapeKey::rounded_ring(r, 1.25);
            let mask = font.shape_mask(key).unwrap();
            let (bw, bh) = key.bitmap_dims();
            assert_eq!(u64::from(mask.rect[2]), bw, "r={r}");
            assert_eq!(u64::from(mask.rect[3]), bh, "r={r}");
        }
    }

    const FA_SOLID: &[u8] = include_bytes!("../tests/assets/fa-solid-900.ttf");
    const FA_RANGE: (u32, u32) = (0xF000, 0xF005);

    fn icon_font() -> Font {
        Font::new_with_icons(
            DEJAVU_SANS,
            PX,
            &[IconFont {
                bytes: FA_SOLID,
                range: FA_RANGE,
            }],
        )
        .expect("icon bake must succeed")
    }

    #[test]
    fn bakes_icon_range_at_init_with_nonzero_coverage() {
        let font = icon_font();
        for cp in FA_RANGE.0..=FA_RANGE.1 {
            let ch = char::from_u32(cp).unwrap();
            let g = *font.faces[0]
                .glyphs
                .get(&ch)
                .unwrap_or_else(|| panic!("icon U+{cp:04X} not baked at init"));
            assert!(
                g.rect[2] > 0 && g.rect[3] > 0,
                "icon U+{cp:04X} has no area"
            );
            assert!(
                rect_bytes(&font, g.rect).iter().any(|&b| b > 0),
                "icon U+{cp:04X} has zero coverage"
            );
        }
        let (_, w, h) = font.atlas_pixels();
        for g in font.faces[0].glyphs.values() {
            assert!(g.rect[0] + g.rect[2] <= w && g.rect[1] + g.rect[3] <= h);
        }
    }

    #[test]
    fn icon_range_boundaries_respected() {
        let mut font = icon_font();
        assert!(font.faces[0].glyphs.contains_key(&'\u{f000}'));
        assert!(font.faces[0].glyphs.contains_key(&'\u{f005}'));
        assert!(!font.faces[0].glyphs.contains_key(&'\u{f006}'));
        assert!(!font.faces[0].glyphs.contains_key(&'\u{efff}'));
        font.measure("\u{f006}");
        assert!(font.faces[0].glyphs.contains_key(&'\u{f006}'));
    }

    #[test]
    fn duplicate_codepoint_first_source_wins() {
        let star = '\u{f005}';
        let fa_first = Font::new_with_icons(
            DEJAVU_SANS,
            PX,
            &[
                IconFont {
                    bytes: FA_SOLID,
                    range: FA_RANGE,
                },
                IconFont {
                    bytes: DEJAVU_SANS,
                    range: FA_RANGE,
                },
            ],
        )
        .unwrap();
        let dejavu_first = Font::new_with_icons(
            DEJAVU_SANS,
            PX,
            &[
                IconFont {
                    bytes: DEJAVU_SANS,
                    range: FA_RANGE,
                },
                IconFont {
                    bytes: FA_SOLID,
                    range: FA_RANGE,
                },
            ],
        )
        .unwrap();
        let fa_only = icon_font();
        let (a, reference) = (
            fa_first.faces[0].glyphs[&star],
            fa_only.faces[0].glyphs[&star],
        );
        assert_eq!(a.rect, reference.rect);
        assert_eq!(a.advance, reference.advance);
        assert_eq!(
            rect_bytes(&fa_first, a.rect),
            rect_bytes(&fa_only, reference.rect)
        );
        let b = dejavu_first.faces[0].glyphs[&star];
        assert_ne!(
            rect_bytes(&dejavu_first, b.rect),
            rect_bytes(&fa_only, reference.rect)
        );
    }

    #[test]
    fn new_with_icons_rejects_invalid_icon_bytes() {
        let icons = [IconFont {
            bytes: b"this is not a font",
            range: FA_RANGE,
        }];
        assert!(matches!(
            Font::new_with_icons(DEJAVU_SANS, PX, &icons),
            Err(FontError::InvalidFont(_))
        ));
    }

    #[test]
    fn empty_icons_list_matches_font_new() {
        let plain = Font::new(DEJAVU_SANS, PX).unwrap();
        let sugar = Font::new_with_icons(DEJAVU_SANS, PX, &[]).unwrap();
        let (pa, wa, ha) = plain.atlas_pixels();
        let (pb, wb, hb) = sugar.atlas_pixels();
        assert_eq!((wa, ha), (wb, hb));
        assert_eq!(pa, pb);
        assert_eq!(plain.atlas_generation(), sugar.atlas_generation());
        assert_eq!(plain.faces[0].glyphs.len(), sugar.faces[0].glyphs.len());
    }

    #[test]
    fn oversized_or_degenerate_rounded_ring_returns_none() {
        let mut font = test_font();
        let gen = font.atlas_generation();
        assert!(font
            .shape_mask(ShapeKey::rounded_ring(600.0, 2.0))
            .is_none());
        assert!(font
            .shape_mask(ShapeKey::rounded_ring(1.0e9, 2.0))
            .is_none());
        assert!(font.shape_mask(ShapeKey::rounded_ring(0.0, 1.0)).is_none());
        assert!(font.shape_mask(ShapeKey::rounded_ring(4.0, 0.0)).is_none());
        assert!(font.shape_mask(ShapeKey::rounded_ring(4.0, 0.1)).is_none());
        assert_eq!(font.atlas_generation(), gen);
        assert!(font
            .shape_mask(ShapeKey::rounded_ring(510.0, 600.0))
            .is_some());
    }

    #[test]
    fn two_faces_at_different_sizes_rasterize_independent_glyphs() {
        let mut font = test_font();
        let big = font
            .add_face(DEJAVU_SANS, 32.0)
            .expect("a second face must register");
        assert_eq!(font.face_count(), 2);
        assert_eq!(big, FontId(1));
        let small_h = font.faces[0].glyphs[&'H'];
        let big_h = font.faces[1].glyphs[&'H'];
        assert_ne!(big_h.rect[2..], small_h.rect[2..]);
        assert!(big_h.rect[2] > small_h.rect[2]);
        let [small_w, small_lh] = font.measure("Hello");
        let [big_w, big_lh] = font.measure_in(big, "Hello");
        assert!(big_w > small_w);
        assert!(big_lh > small_lh);
        let (_, aw, ah) = font.atlas_pixels();
        for g in [small_h, big_h] {
            assert!(g.rect[0] + g.rect[2] <= aw && g.rect[1] + g.rect[3] <= ah);
        }
        let (_, quad) = font.glyph_quad(big, 'H', [0.0, 0.0]);
        let quad = quad.expect("'H' has ink");
        assert_eq!(quad.max[0] - quad.min[0], big_h.rect[2] as f32);
        assert_eq!(quad.max[1] - quad.min[1], big_h.rect[3] as f32);
    }

    #[test]
    fn per_face_icon_ranges_resolve_overlapping_codepoints_independently() {
        let star = '\u{f005}';
        let mut font = icon_font();
        let face0_star = font.faces[0].glyphs[&star];
        let face0_bytes = rect_bytes(&font, face0_star.rect);
        let second = font
            .add_face_with_icons(
                DEJAVU_SANS,
                PX,
                &[IconFont {
                    bytes: DEJAVU_SANS,
                    range: FA_RANGE,
                }],
            )
            .expect("the icon face must register");
        let face1_star = font.faces[second.0 as usize].glyphs[&star];
        assert_ne!(
            rect_bytes(&font, face1_star.rect),
            face0_bytes,
            "each face resolves the overlapping codepoint from its own source"
        );
        assert_eq!(font.faces[0].glyphs[&star].rect, face0_star.rect);
        assert_eq!(rect_bytes(&font, face0_star.rect), face0_bytes);
    }

    #[test]
    fn face_info_reports_registered_face_metrics() {
        let mut font = test_font();
        let big = font.add_face(DEJAVU_SANS, 24.0).unwrap();
        let default = font.face_info(FontId::DEFAULT).expect("face 0 exists");
        assert_eq!(default.px_size, PX);
        assert_eq!(default.ascent, font.faces[0].ascent);
        assert_eq!(default.line_height, font.faces[0].line_height);
        assert_eq!(default.cached_glyphs, font.faces[0].glyphs.len());
        let big_info = font.face_info(big).expect("face 1 exists");
        assert_eq!(big_info.px_size, 24.0);
        assert_eq!(big_info.cached_glyphs, font.faces[1].glyphs.len());
        assert!(big_info.line_height > default.line_height);
        assert!(font.face_info(FontId(2)).is_none());
        assert!(font.is_valid(FontId::DEFAULT));
        assert!(font.is_valid(big));
        assert!(!font.is_valid(FontId(2)));
    }

    #[test]
    fn add_face_rejects_garbage_bytes_without_registering() {
        let mut font = test_font();
        assert!(matches!(
            font.add_face(b"this is not a font", PX),
            Err(FontError::InvalidFont(_))
        ));
        assert_eq!(font.face_count(), 1);
    }

    #[test]
    fn add_face_failing_the_bake_does_not_register_the_face() {
        let mut font = test_font();
        font.max_atlas_size = font.atlas_w.min(font.atlas_h);
        font.cursor_y = font.atlas_h;
        assert!(matches!(
            font.add_face(DEJAVU_SANS, PX),
            Err(FontError::AtlasTooSmall)
        ));
        assert_eq!(font.face_count(), 1);
        assert!(!font.is_valid(FontId(1)));
    }
}
