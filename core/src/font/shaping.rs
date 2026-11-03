use super::{subpixel, Font, FontError, FontId, GlyphInfo, GlyphQuad, TextRasterMode};
use crate::editing::{EditError, MAX_EDIT_BYTES};
use alloc::{string::String, vec::Vec};
use core::ops::Range;
use unicode_bidi::{BidiInfo, Level};
use unicode_segmentation::UnicodeSegmentation;
#[cfg(test)]
#[path = "shaping_tests.rs"]
mod tests;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextDirection {
    #[default]
    Auto = 0,
    LeftToRight = 1,
    RightToLeft = 2,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapedOptions {
    pub width: f32,
    pub wrap: bool,
    pub direction: TextDirection,
}
impl Default for ShapedOptions {
    fn default() -> Self {
        Self {
            width: 0.,
            wrap: false,
            direction: TextDirection::Auto,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaretPosition {
    pub byte: usize,
    pub position: [f32; 2],
    pub line: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangePair {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapedGlyph {
    pub font: FontId,
    pub index: u16,
    pub cluster: RangePair,
    pub line: usize,
    pub position: [f32; 2],
    pub pen_x: f32,
    pub advance: f32,
    pub rtl: bool,
}
#[derive(Clone, Copy)]
struct Segment {
    range: RangePair,
    font: FontId,
    script: rustybuzz::Script,
}
pub struct ShapedText {
    text: String,
    glyphs: Vec<ShapedGlyph>,
    carets: Vec<CaretPosition>,
    segments: Vec<Segment>,
    clusters: Vec<usize>,
    buffer: Option<rustybuzz::UnicodeBuffer>,
    font: FontId,
    key: u64,
    options: ShapedOptions,
    size: [f32; 2],
    line_height: f32,
    ascent: f32,
    lines: usize,
    valid: bool,
}
impl Default for ShapedText {
    fn default() -> Self {
        Self {
            text: String::new(),
            glyphs: Vec::new(),
            carets: Vec::new(),
            segments: Vec::new(),
            clusters: Vec::new(),
            buffer: Some(rustybuzz::UnicodeBuffer::new()),
            font: FontId::DEFAULT,
            key: 0,
            options: ShapedOptions::default(),
            size: [0.; 2],
            line_height: 0.,
            ascent: 0.,
            lines: 0,
            valid: false,
        }
    }
}
impl ShapedText {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn glyphs(&self) -> &[ShapedGlyph] {
        &self.glyphs
    }
    pub fn carets(&self) -> &[CaretPosition] {
        &self.carets
    }
    pub fn size(&self) -> [f32; 2] {
        self.size
    }
    pub fn line_height(&self) -> f32 {
        self.line_height
    }
    pub fn lines(&self) -> usize {
        self.lines
    }
    pub fn caret(&self, byte: usize) -> Option<CaretPosition> {
        self.carets.iter().find(|p| p.byte == byte).copied()
    }
    pub fn hit_test(&self, point: [f32; 2]) -> Option<CaretPosition> {
        let line = if point[1] < 0. {
            0
        } else {
            (point[1] / self.line_height) as usize
        }
        .min(self.lines.saturating_sub(1));
        self.carets
            .iter()
            .filter(|c| c.line == line)
            .min_by(|a, b| {
                (a.position[0] - point[0])
                    .abs()
                    .total_cmp(&(b.position[0] - point[0]).abs())
            })
            .copied()
    }
    pub fn visual_neighbor(&self, current: CaretPosition, right: bool) -> Option<CaretPosition> {
        let i = self.carets.iter().position(|c| *c == current)?;
        let mut n = if right {
            (i + 1).min(self.carets.len() - 1)
        } else {
            i.saturating_sub(1)
        };
        while n != i && self.carets[n].position == current.position {
            let next = if right {
                (n + 1).min(self.carets.len() - 1)
            } else {
                n.saturating_sub(1)
            };
            if next == n {
                break;
            }
            n = next;
        }
        self.carets.get(n).copied()
    }
    pub fn selection_rects(&self, range: Range<usize>, mut emit: impl FnMut([f32; 4])) {
        let mut i = 0;
        while i < self.glyphs.len() {
            let g = self.glyphs[i];
            let mut advance = 0.;
            while i < self.glyphs.len() && self.glyphs[i].cluster == g.cluster {
                advance += self.glyphs[i].advance;
                i += 1;
            }
            if range.start >= g.cluster.end || range.end <= g.cluster.start {
                continue;
            }
            let text = &self.text[g.cluster.start..g.cluster.end];
            let count = text.graphemes(true).count().max(1) as f32;
            let a = text
                .grapheme_indices(true)
                .filter(|(n, _)| g.cluster.start + *n < range.start)
                .count() as f32
                / count;
            let b = text
                .grapheme_indices(true)
                .filter(|(n, _)| g.cluster.start + *n < range.end)
                .count() as f32
                / count;
            let (a, b) = if g.rtl { (1. - b, 1. - a) } else { (a, b) };
            emit([
                g.pen_x + advance * a,
                g.line as f32 * self.line_height,
                g.pen_x + advance * b,
                (g.line + 1) as f32 * self.line_height,
            ]);
        }
        for c in &self.carets {
            if range.contains(&c.byte)
                && self.text[c.byte..]
                    .chars()
                    .next()
                    .is_some_and(crate::editing::model::is_line_break)
            {
                emit([
                    c.position[0],
                    c.position[1],
                    c.position[0] + self.line_height * 0.35,
                    c.position[1] + self.line_height,
                ]);
            }
        }
    }
    pub fn prepare(
        &mut self,
        font: &mut Font,
        face: FontId,
        text: &str,
        options: ShapedOptions,
    ) -> Result<(), EditError> {
        if text.len() > MAX_EDIT_BYTES {
            return Err(EditError::TooLong);
        }
        if text.contains('\0')
            || !options.width.is_finite()
            || options.width < 0.
            || options.wrap && options.width <= 0.
            || face.0 as usize >= font.faces.len()
        {
            return Err(EditError::InvalidInput);
        }
        if self.valid
            && self.text == text
            && self.font == face
            && self.key == font.layout_key
            && self.options == options
        {
            return Ok(());
        }
        self.glyphs.clear();
        self.carets.clear();
        self.size = [0.; 2];
        self.lines = 0;
        (self.ascent, self.line_height) = font.shaped_line_metrics(face);
        let base = match options.direction {
            TextDirection::Auto => None,
            TextDirection::LeftToRight => Some(Level::ltr()),
            TextDirection::RightToLeft => Some(Level::rtl()),
        };
        let mut start = 0;
        loop {
            let newline = text[start..]
                .char_indices()
                .find(|(_, c)| crate::editing::model::is_line_break(*c))
                .map(|(i, _)| start + i);
            let end = newline.unwrap_or(text.len());
            let paragraph = &text[start..end];
            let bidi = BidiInfo::new(paragraph, base);
            let mut line_start = 0;
            loop {
                let mut line_end = paragraph.len();
                if options.wrap {
                    let mut width = 0.;
                    let mut last_space = None;
                    for (i, cluster) in paragraph[line_start..].grapheme_indices(true) {
                        let i = line_start + i;
                        let mut probe = self.buffer.take().expect("retained shaping buffer");
                        probe.clear();
                        probe.push_str(cluster);
                        probe.guess_segment_properties();
                        let resolved = font.resolve_cluster(face, cluster);
                        let advance =
                            font.cluster_advance(resolved, cluster, probe.script(), &mut probe);
                        self.buffer = Some(probe);
                        if i > line_start && width + advance > options.width {
                            line_end = last_space.filter(|p| *p > line_start).unwrap_or(i);
                            break;
                        }
                        width += advance;
                        if cluster.chars().all(char::is_whitespace) {
                            last_space = Some(i + cluster.len());
                        }
                    }
                }
                let glyph_start = self.glyphs.len();
                let caret_start = self.carets.len();
                let mut width =
                    self.shape_line(font, face, paragraph, &bidi, line_start..line_end, start);
                while options.wrap
                    && width > options.width
                    && paragraph[line_start..line_end].graphemes(true).count() > 1
                {
                    self.glyphs.truncate(glyph_start);
                    self.carets.truncate(caret_start);
                    line_end = crate::editing::previous_grapheme(paragraph, line_end);
                    width =
                        self.shape_line(font, face, paragraph, &bidi, line_start..line_end, start);
                }
                self.size[0] = self.size[0].max(width);
                self.lines += 1;
                if line_end == paragraph.len() {
                    break;
                }
                line_start = line_end;
            }
            match newline {
                None => break,
                Some(i) => {
                    start = i + text[i..].chars().next().expect("newline found").len_utf8();
                    if text.as_bytes()[i] == b'\r' && text.as_bytes().get(start) == Some(&b'\n') {
                        start += 1;
                    }
                }
            }
        }
        self.size[1] = self.lines as f32 * self.line_height;
        self.carets.sort_unstable_by(|a, b| {
            a.line
                .cmp(&b.line)
                .then_with(|| a.position[0].total_cmp(&b.position[0]))
                .then(a.byte.cmp(&b.byte))
        });
        self.carets.dedup_by(|a, b| {
            a.byte == b.byte && a.line == b.line && (a.position[0] - b.position[0]).abs() < 0.001
        });
        self.text.clear();
        self.text.push_str(text);
        self.font = face;
        self.key = font.layout_key;
        self.options = options;
        self.valid = true;
        Ok(())
    }
    fn shape_line(
        &mut self,
        font: &mut Font,
        face: FontId,
        text: &str,
        bidi: &BidiInfo,
        line: Range<usize>,
        offset: usize,
    ) -> f32 {
        let y = self.lines as f32 * self.line_height;
        let mut pen = 0.;
        if line.is_empty() {
            self.carets.push(CaretPosition {
                byte: offset + line.start,
                position: [0., y],
                line: self.lines,
            });
            return 0.;
        }
        let para = &bidi.paragraphs[0];
        let (levels, runs) = bidi.visual_runs(para, line.clone());
        for run in runs {
            let rtl = levels[run.start].is_rtl();
            self.segments.clear();
            let mut probe = self.buffer.take().expect("retained shaping buffer");
            for (i, cluster) in text[run.clone()].grapheme_indices(true) {
                let i = run.start + i;
                let chosen = font.resolve_cluster(face, cluster);
                probe.clear();
                probe.push_str(cluster);
                probe.guess_segment_properties();
                let mut script = probe.script();
                if [
                    rustybuzz::script::COMMON,
                    rustybuzz::script::INHERITED,
                    rustybuzz::script::UNKNOWN,
                ]
                .contains(&script)
                {
                    script = self.segments.last().map_or(script, |s| s.script);
                }
                if let Some(last) = self
                    .segments
                    .last_mut()
                    .filter(|s| s.font == chosen && s.script == script)
                {
                    last.range.end = i + cluster.len();
                } else {
                    self.segments.push(Segment {
                        range: RangePair {
                            start: i,
                            end: i + cluster.len(),
                        },
                        font: chosen,
                        script,
                    });
                }
            }
            for n in 0..self.segments.len() {
                let s = self.segments[if rtl { self.segments.len() - 1 - n } else { n }];
                let f = &font.faces[s.font.0 as usize];
                let face = rustybuzz::Face::from_slice(&f.source_bytes, 0)
                    .expect("registered font parses for shaping");
                probe.clear();
                probe.push_str(&text[s.range.start..s.range.end]);
                probe.set_script(s.script);
                probe.set_direction(if rtl {
                    rustybuzz::Direction::RightToLeft
                } else {
                    rustybuzz::Direction::LeftToRight
                });
                probe.guess_segment_properties();
                let shaped = rustybuzz::shape(&face, &[], probe);
                let scale = f.px_size / face.units_per_em() as f32;
                let glyph_start = self.glyphs.len();
                let segment_x = pen;
                self.clusters.clear();
                self.clusters.extend(
                    shaped
                        .glyph_infos()
                        .iter()
                        .map(|g| s.range.start + g.cluster as usize),
                );
                self.clusters.sort_unstable();
                self.clusters.dedup();
                self.clusters.push(s.range.end);
                for (g, p) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                    let cluster = s.range.start + g.cluster as usize;
                    let cluster_end = self.clusters[self
                        .clusters
                        .binary_search(&cluster)
                        .expect("shaper cluster indexed")
                        + 1];
                    self.glyphs.push(ShapedGlyph {
                        font: s.font,
                        index: g.glyph_id as u16,
                        line: self.lines,
                        cluster: RangePair {
                            start: offset + cluster,
                            end: offset + cluster_end,
                        },
                        position: [
                            pen + p.x_offset as f32 * scale,
                            y + self.ascent - f.ascent - p.y_offset as f32 * scale,
                        ],
                        pen_x: pen,
                        advance: p.x_advance as f32 * scale,
                        rtl,
                    });
                    pen += p.x_advance as f32 * scale;
                }
                let mut x = segment_x;
                let mut g = glyph_start;
                while g < self.glyphs.len() {
                    let cluster = self.glyphs[g].cluster;
                    let mut advance = 0.;
                    while g < self.glyphs.len() && self.glyphs[g].cluster == cluster {
                        advance += self.glyphs[g].advance;
                        g += 1;
                    }
                    let logical = &text[cluster.start - offset..cluster.end - offset];
                    let count = logical.graphemes(true).count().max(1);
                    for (n, (i, _)) in logical.grapheme_indices(true).enumerate() {
                        let t = n as f32 / count as f32;
                        self.carets.push(CaretPosition {
                            byte: cluster.start + i,
                            position: [x + advance * if rtl { 1. - t } else { t }, y],
                            line: self.lines,
                        });
                    }
                    self.carets.push(CaretPosition {
                        byte: cluster.end,
                        position: [x + if rtl { 0. } else { advance }, y],
                        line: self.lines,
                    });
                    x += advance;
                }
                probe = shaped.clear();
            }
            self.buffer = Some(probe);
        }
        pen
    }
    pub(crate) fn draw(
        &self,
        font: &mut Font,
        draw: &mut crate::draw::DrawList,
        origin: [f32; 2],
        color: [u8; 4],
    ) {
        for glyph in &self.glyphs {
            if !font.ensure_color(glyph.font, glyph.index) {
                font.ensure_indexed(glyph.font, glyph.index);
            }
        }
        for glyph in &self.glyphs {
            let pen = [origin[0] + glyph.position[0], origin[1] + glyph.position[1]];
            if let Some(q) = font.color_quad(glyph.font, glyph.index, pen) {
                draw.add_textured_quad(
                    font.color_atlas_texture().unwrap(),
                    q.min,
                    q.max,
                    q.uv_min,
                    q.uv_max,
                    [255, 255, 255, color[3]],
                );
                continue;
            }
            if let Some(q) = font.indexed_quad(
                glyph.font,
                glyph.index,
                [origin[0] + glyph.position[0], origin[1] + glyph.position[1]],
            ) {
                draw.add_textured_quad(
                    crate::draw::TextureId::FONT,
                    q.min,
                    q.max,
                    q.uv_min,
                    q.uv_max,
                    color,
                );
            }
        }
    }
}
impl Font {
    pub(crate) fn shaped_line_metrics(&self, face: FontId) -> (f32, f32) {
        let primary = &self.faces[face.0 as usize];
        let mut ascent = primary.ascent;
        let mut descent = (primary.line_height - primary.ascent).max(0.);
        for fallback in &primary.fallback {
            let f = &self.faces[fallback.0 as usize];
            ascent = ascent.max(f.ascent);
            descent = descent.max(f.line_height - f.ascent);
        }
        (ascent, ascent + descent)
    }
    pub fn set_fallback_chain(
        &mut self,
        primary: FontId,
        chain: &[FontId],
    ) -> Result<(), FontError> {
        if primary.0 as usize >= self.faces.len()
            || chain.len() > 16
            || chain.iter().enumerate().any(|(i, f)| {
                f.0 as usize >= self.faces.len() || *f == primary || chain[..i].contains(f)
            })
        {
            return Err(FontError::InvalidFont("invalid fallback chain"));
        }
        let f = &mut self.faces[primary.0 as usize];
        if f.fallback == chain {
            return Ok(());
        }
        f.fallback.clear();
        f.fallback.extend_from_slice(chain);
        self.layout_key = self.layout_key.wrapping_mul(1099511628211).wrapping_add(1);
        Ok(())
    }
    pub fn fallback_chain(&self, primary: FontId) -> Option<&[FontId]> {
        self.faces
            .get(primary.0 as usize)
            .map(|f| f.fallback.as_slice())
    }
    fn resolve_cluster(&mut self, primary: FontId, cluster: &str) -> FontId {
        let mut ids = [primary; 17];
        let count = self.faces[primary.0 as usize].fallback.len() + 1;
        ids[1..count].copy_from_slice(&self.faces[primary.0 as usize].fallback);
        let emoji = super::color::prefers_color(cluster);
        if emoji && self.color_texture.is_some() {
            for &id in &ids[..count] {
                if self.faces[id.0 as usize].has_color && self.color_cluster(id, cluster) {
                    return id;
                }
            }
        }
        if emoji
            && ids[1..count]
                .iter()
                .any(|id| self.faces[id.0 as usize].has_color)
        {
            ids[..count].rotate_left(1);
        }
        for &id in &ids[..count] {
            if self.faces[id.0 as usize].has_color {
                if !cluster.contains('\u{fe0e}') && self.color_cluster(id, cluster) {
                    return id;
                }
                continue;
            }
            let covers = cluster.chars().all(|ch| {
                matches!(ch, '\u{200c}' | '\u{200d}' | '\u{fe0e}' | '\u{fe0f}')
                    || ch.is_control()
                    || self.faces[id.0 as usize].inner.lookup_glyph_index(ch) != 0
            });
            if covers {
                return id;
            }
        }
        primary
    }
    fn color_cluster(&mut self, id: FontId, cluster: &str) -> bool {
        if self.color_texture.is_none() {
            return false;
        }
        let f = &self.faces[id.0 as usize];
        let face = rustybuzz::Face::from_slice(&f.source_bytes, 0).expect("registered face");
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(cluster);
        buffer.guess_segment_properties();
        let shaped = rustybuzz::shape(&face, &[], buffer);
        if shaped.glyph_infos().is_empty()
            || shaped.glyph_infos().iter().any(|g| {
                g.glyph_id == 0
                    || !face.is_color_glyph(rustybuzz::ttf_parser::GlyphId(g.glyph_id as u16))
            })
        {
            return false;
        }
        let mut indices = [0u16; 32];
        if shaped.len() > indices.len() {
            return false;
        }
        for (i, glyph) in shaped.glyph_infos().iter().enumerate() {
            indices[i] = glyph.glyph_id as u16;
        }
        let count = shaped.len();
        drop(shaped);
        indices[..count]
            .iter()
            .all(|&index| self.ensure_color(id, index))
    }
    fn cluster_advance(
        &self,
        id: FontId,
        text: &str,
        script: rustybuzz::Script,
        buffer: &mut rustybuzz::UnicodeBuffer,
    ) -> f32 {
        let f = &self.faces[id.0 as usize];
        let face = rustybuzz::Face::from_slice(&f.source_bytes, 0).expect("valid registered face");
        buffer.clear();
        buffer.push_str(text);
        buffer.set_script(script);
        buffer.guess_segment_properties();
        let shaped = rustybuzz::shape(&face, &[], core::mem::take(buffer));
        let width = shaped
            .glyph_positions()
            .iter()
            .map(|p| p.x_advance as f32)
            .sum::<f32>()
            * f.px_size
            / face.units_per_em() as f32;
        *buffer = shaped.clear();
        width
    }
    pub(super) fn ensure_indexed(&mut self, id: FontId, index: u16) {
        let i = id.0 as usize;
        if !self.faces[i].indexed.contains_key(&index) {
            let f = &self.faces[i];
            let px = f.px_size * self.raster_scale;
            let metrics = f.inner.metrics_indexed(index, px);
            let bitmap = if metrics.width <= self.max_atlas_size as usize
                && metrics.height <= self.max_atlas_size as usize
            {
                Some(f.inner.rasterize_indexed(index, px).1)
            } else {
                None
            };
            let placed = bitmap
                .as_ref()
                .and_then(|b| self.place_bitmap(metrics.width as u32, metrics.height as u32, b));
            let info = if let Some((x, y)) = placed {
                GlyphInfo {
                    advance: metrics.advance_width / self.raster_scale,
                    xmin: metrics.xmin,
                    ymin: metrics.ymin,
                    rect: [x, y, metrics.width as u32, metrics.height as u32],
                    source: Some(0),
                }
            } else {
                let mut fallback = self.faces[i].glyphs[&'?'];
                fallback.source = None;
                fallback.advance = metrics.advance_width / self.raster_scale;
                fallback
            };
            self.faces[i].indexed.insert(index, info);
        }
        if self.text_raster_mode == TextRasterMode::Subpixel
            && !self.faces[i].indexed_phases.contains_key(&index)
        {
            let raster = subpixel::rasterize_indexed(
                &self.faces[i].inner,
                index,
                self.faces[i].px_size * self.raster_scale,
                self.max_atlas_size,
            );
            let phase = raster.and_then(|(p, b)| {
                let [w, h] = p.dimensions();
                self.place_phase_bitmap(w, h, &b)
                    .map(|(x, y)| p.placed([x, y]))
            });
            self.faces[i].indexed_phases.insert(index, phase);
        }
    }
    fn indexed_quad(&mut self, id: FontId, index: u16, pen: [f32; 2]) -> Option<GlyphQuad> {
        self.ensure_indexed(id, index);
        let f = &self.faces[id.0 as usize];
        let g = f.indexed[&index];
        let baseline = pen[1] + f.ascent;
        let native = (
            [
                pen[0] + g.xmin as f32 / self.raster_scale,
                baseline - (g.ymin as f32 + g.rect[3] as f32) / self.raster_scale,
            ],
            g.rect,
        );
        let (min, rect) = if self.text_raster_mode == TextRasterMode::Subpixel {
            if let Some(Some(p)) = f.indexed_phases.get(&index) {
                let (px, x) = subpixel::quantize(pen[0] * self.raster_scale);
                let (py, y) = subpixel::quantize(baseline * self.raster_scale);
                (
                    [
                        (px + p.origin[0] as f32) / self.raster_scale,
                        (py + p.origin[1] as f32) / self.raster_scale,
                    ],
                    p.rect(x, y),
                )
            } else {
                native
            }
        } else {
            native
        };
        let [x, y, w, h] = rect;
        if w == 0 || h == 0 {
            return None;
        }
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
        })
    }
}
