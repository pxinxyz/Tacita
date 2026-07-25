use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

use crate::raster;

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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapeMask {
    pub rect: [u32; 4],
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

#[derive(Debug, Clone, Copy)]
struct GlyphInfo {
    advance: f32,
    xmin: i32,
    ymin: i32,
    rect: [u32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GlyphQuad {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

pub struct Font {
    // fontdue hands us anti-aliased glyph bitmaps; we blit them into an atlas and pretend we invented typography.
    inner: fontdue::Font,
    px_size: f32,
    ascent: f32,
    line_height: f32,
    atlas: Vec<u8>,
    atlas_w: u32,
    atlas_h: u32,
    max_atlas_size: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    glyphs: BTreeMap<char, GlyphInfo>,
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
        let inner = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(FontError::InvalidFont)?;
        let line_metrics = inner
            .horizontal_line_metrics(px_size)
            .ok_or(FontError::MissingHorizontalMetrics)?;

        let mut font = Font {
            inner,
            px_size,
            ascent: line_metrics.ascent,
            line_height: line_metrics.new_line_size,
            atlas: Vec::new(),
            atlas_w: 0,
            atlas_h: 0,
            max_atlas_size: MAX_ATLAS_SIZE,
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            glyphs: BTreeMap::new(),
            shapes: BTreeMap::new(),
            #[cfg(test)]
            raster_count: core::cell::Cell::new(0),
            generation: 0,
        };
        font.reset_atlas(INITIAL_ATLAS_SIZE);

        for ch in ASCII_FIRST..=ASCII_LAST {
            if !font.insert_glyph(ch) {
                return Err(FontError::AtlasTooSmall);
            }
        }
        for icon in icons {
            // Merging icon glyphs into the same atlas as the text. Two fonts walk into one texture; the GPU never finds out.
            let icon_font = fontdue::Font::from_bytes(icon.bytes, fontdue::FontSettings::default())
                .map_err(FontError::InvalidFont)?;
            for cp in icon.range.0..=icon.range.1 {
                let Some(ch) = char::from_u32(cp) else {
                    continue;
                };
                if font.glyphs.contains_key(&ch) {
                    continue;
                }
                if !font.bake_glyph(&icon_font, ch) {
                    return Err(FontError::AtlasTooSmall);
                }
            }
        }
        Ok(font)
    }

    pub fn atlas_pixels(&self) -> (&[u8], u32, u32) {
        (&self.atlas, self.atlas_w, self.atlas_h)
    }

    pub fn atlas_generation(&self) -> u64 {
        self.generation
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn measure(&mut self, text: &str) -> [f32; 2] {
        let mut width = 0.0;
        for ch in text.chars() {
            self.ensure_glyph(ch);
            width += self.glyphs[&ch].advance;
        }
        [width, self.line_height]
    }

    pub(crate) fn glyph_quad(&mut self, ch: char, pen: [f32; 2]) -> ([f32; 2], Option<GlyphQuad>) {
        self.ensure_glyph(ch);
        let g = self.glyphs[&ch];
        let next_pen = [pen[0] + g.advance, pen[1]];
        let [x, y, w, h] = g.rect;
        if w == 0 || h == 0 {
            return (next_pen, None);
        }
        let baseline = pen[1] + self.ascent;
        let x0 = pen[0] + g.xmin as f32;
        let y1 = baseline - g.ymin as f32;
        let y0 = y1 - h as f32;
        let quad = GlyphQuad {
            min: [x0, y0],
            max: [x0 + w as f32, y1],
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
        let (bw, bh) = key.bitmap_dims();
        if bw > u64::from(MAX_SHAPE_MASK_DIM) || bh > u64::from(MAX_SHAPE_MASK_DIM) {
            return None;
        }
        #[cfg(test)]
        self.raster_count.set(self.raster_count.get() + 1);
        let (w, h, bitmap) = match key {
            ShapeKey::Disk { radius_q } => raster::rasterize_disk(radius_q as f32 / 4.0),
            ShapeKey::Ring {
                radius_q,
                thickness_q,
            } => raster::rasterize_ring(radius_q as f32 / 4.0, thickness_q as f32 / 4.0),
            ShapeKey::RoundedCorner { r_q } => raster::rasterize_rounded_corner(r_q as f32 / 4.0),
            ShapeKey::RoundedRing { r_q, t_q } => {
                raster::rasterize_rounded_ring(r_q as f32 / 4.0, t_q as f32 / 4.0)
            }
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

    fn ensure_glyph(&mut self, ch: char) {
        if !self.glyphs.contains_key(&ch) {
            self.insert_glyph_or_fallback(ch);
        }
    }

    fn reset_atlas(&mut self, size: u32) {
        self.reset_atlas_full(size, size);
    }

    fn reset_atlas_full(&mut self, w: u32, h: u32) {
        self.atlas = alloc::vec![0u8; (w * h) as usize];
        self.atlas_w = w;
        self.atlas_h = h;
        self.atlas[0] = 0xFF;
        self.glyphs.clear();
        self.shapes.clear();
        self.cursor_x = 1 + PAD;
        self.cursor_y = 0;
        self.row_height = 0;
    }

    fn insert_glyph(&mut self, ch: char) -> bool {
        let (metrics, bitmap) = self.inner.rasterize(ch, self.px_size);
        self.place_glyph(ch, metrics, bitmap)
    }

    fn bake_glyph(&mut self, source: &fontdue::Font, ch: char) -> bool {
        let (metrics, bitmap) = source.rasterize(ch, self.px_size);
        self.place_glyph(ch, metrics, bitmap)
    }

    fn place_glyph(&mut self, ch: char, metrics: fontdue::Metrics, bitmap: Vec<u8>) -> bool {
        let (w, h) = (metrics.width as u32, metrics.height as u32);
        let Some((x, y)) = self.place_bitmap(w, h, &bitmap) else {
            return false;
        };
        self.glyphs.insert(
            ch,
            GlyphInfo {
                advance: metrics.advance_width,
                xmin: metrics.xmin,
                ymin: metrics.ymin,
                rect: [x, y, w, h],
            },
        );
        true
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

    fn insert_glyph_or_fallback(&mut self, ch: char) {
        if self.insert_glyph(ch) {
            return;
        }
        let advance = self.inner.metrics(ch, self.px_size).advance_width;
        let (rect, xmin, ymin) = self
            .glyphs
            .get(&FALLBACK_CHAR)
            .map(|g| (g.rect, g.xmin, g.ymin))
            .unwrap_or(([0, 0, 0, 0], 0, 0));
        self.glyphs.insert(
            ch,
            GlyphInfo {
                advance,
                xmin,
                ymin,
                rect,
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
            assert!(font.glyphs.contains_key(&ch), "glyph {ch:?} not baked");
        }
        for g in font.glyphs.values() {
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
        let mut rects: Vec<[u32; 4]> = font
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
        assert!(!font.glyphs.contains_key(&'é'));
        let [w, _] = font.measure("é");
        assert!(w > 0.0);
        let cached = font.glyphs[&'é'];
        assert!(cached.rect[2] > 0 && cached.rect[3] > 0);
        let (pen, quad) = font.glyph_quad('é', [0.0, 0.0]);
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
        let omega = font.glyphs[&'Ω'];
        let question = font.glyphs[&FALLBACK_CHAR];
        assert_eq!(omega.rect, question.rect);
        assert_eq!(omega.xmin, question.xmin);
        assert_eq!(omega.ymin, question.ymin);
        let (_, quad) = font.glyph_quad('Ω', [10.0, 20.0]);
        assert!(quad.is_some());
    }

    #[test]
    fn growth_preserves_existing_glyphs_and_bumps_generation() {
        let mut font = test_font();
        let rect = font.glyphs[&'H'].rect;
        let (old_w, old_h) = (font.atlas_w, font.atlas_h);
        let bytes_before: Vec<u8> = rect_bytes(&font, rect);
        let gen_before = font.atlas_generation();

        font.grow_atlas(old_w * 2, old_h * 2);
        assert!(font.atlas_generation() > gen_before);
        assert!(font.insert_glyph('é'));

        assert_eq!(font.atlas[0], 0xFF);
        assert_eq!(font.glyphs[&'H'].rect, rect);
        assert_eq!(rect_bytes(&font, rect), bytes_before);
        for g in font.glyphs.values() {
            assert!(g.rect[0] + g.rect[2] <= old_w * 2);
            assert!(g.rect[1] + g.rect[3] <= old_h * 2);
        }
        let cached = font.glyphs[&'é'];
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
        let (next_pen, quad) = font.glyph_quad('H', pen);
        let quad = quad.expect("'H' has ink");
        let g = font.glyphs[&'H'];
        assert_eq!(next_pen, [pen[0] + g.advance, pen[1]]);
        let baseline = pen[1] + font.ascent;
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
        let (next_pen, quad) = font.glyph_quad(' ', [0.0, 0.0]);
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
        let h_rect = font.glyphs[&'H'].rect;
        let h_bytes = rect_bytes(&font, h_rect);
        font.shape_mask(ShapeKey::disk(7.0)).unwrap();
        font.shape_mask(ShapeKey::ring(20.0, 4.0)).unwrap();
        font.shape_mask(ShapeKey::disk(255.0)).unwrap();
        assert_eq!(font.atlas[0], 0xFF);
        assert_eq!(font.glyphs[&'H'].rect, h_rect);
        assert_eq!(rect_bytes(&font, h_rect), h_bytes);
        let (_, quad) = font.glyph_quad('H', [0.0, 0.0]);
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
            let g = *font
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
        for g in font.glyphs.values() {
            assert!(g.rect[0] + g.rect[2] <= w && g.rect[1] + g.rect[3] <= h);
        }
    }

    #[test]
    fn icon_range_boundaries_respected() {
        let mut font = icon_font();
        assert!(font.glyphs.contains_key(&'\u{f000}'));
        assert!(font.glyphs.contains_key(&'\u{f005}'));
        assert!(!font.glyphs.contains_key(&'\u{f006}'));
        assert!(!font.glyphs.contains_key(&'\u{efff}'));
        font.measure("\u{f006}");
        assert!(font.glyphs.contains_key(&'\u{f006}'));
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
        let (a, reference) = (fa_first.glyphs[&star], fa_only.glyphs[&star]);
        assert_eq!(a.rect, reference.rect);
        assert_eq!(a.advance, reference.advance);
        assert_eq!(
            rect_bytes(&fa_first, a.rect),
            rect_bytes(&fa_only, reference.rect)
        );
        let b = dejavu_first.glyphs[&star];
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
        assert_eq!(plain.glyphs.len(), sugar.glyphs.len());
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
}
