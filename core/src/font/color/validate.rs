use super::{ColorFontError as Error, MAX_DEPTH, MAX_NODES};
use rustybuzz::ttf_parser::{Face, GlyphId, Tag};

struct Graph<'a> {
    bytes: &'a [u8],
    face: &'a Face<'a>,
    base: usize,
    layers: usize,
    palette: rustybuzz::ttf_parser::cpal::Table<'a>,
    palette_entries: u16,
    nodes: usize,
    ancestors: [usize; MAX_DEPTH],
}
impl Graph<'_> {
    fn bytes(&self, p: usize, len: usize) -> Result<&[u8], Error> {
        self.bytes
            .get(p..p.checked_add(len).ok_or(Error::InvalidFont)?)
            .ok_or(Error::InvalidFont)
    }
    fn u16(&self, p: usize) -> Result<u16, Error> {
        let b = self.bytes(p, 2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    fn offset(&self, p: usize, len: usize) -> Result<usize, Error> {
        Ok(self
            .bytes(p, len)?
            .iter()
            .fold(0usize, |n, v| n * 256 + usize::from(*v)))
    }
    fn relative(&self, base: usize, p: usize, len: usize) -> Result<usize, Error> {
        let offset = self.offset(p, len)?;
        if offset == 0 {
            return Err(Error::InvalidFont);
        }
        base.checked_add(offset).ok_or(Error::InvalidFont)
    }
    fn glyph(&self, glyph: u16) -> Result<usize, Error> {
        let count = self.offset(self.base, 4)?;
        self.bytes(
            self.base + 4,
            count.checked_mul(6).ok_or(Error::InvalidFont)?,
        )?;
        let mut lo = 0;
        let mut hi = count;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let p = self.base + 4 + mid * 6;
            match self.u16(p)?.cmp(&glyph) {
                core::cmp::Ordering::Less => lo = mid + 1,
                core::cmp::Ordering::Greater => hi = mid,
                core::cmp::Ordering::Equal => return self.relative(self.base, p + 2, 4),
            }
        }
        Err(Error::Unsupported)
    }
    fn palette(&self, p: usize) -> Result<(), Error> {
        let index = self.u16(p)?;
        if index == 0xffff {
            return Err(Error::Unsupported);
        }
        if index >= self.palette_entries {
            return Err(Error::InvalidFont);
        }
        self.palette.get(0, index).ok_or(Error::InvalidFont)?;
        if self.u16(p + 2)? > 16384 {
            return Err(Error::InvalidFont);
        }
        Ok(())
    }
    fn line(&self, p: usize) -> Result<(), Error> {
        if self.bytes(p, 1)?[0] > 2 {
            return Err(Error::InvalidFont);
        }
        let count = self.u16(p + 1)? as usize;
        if count > 16 {
            return Err(Error::TooComplex);
        }
        self.bytes(p + 3, count * 6)?;
        for i in 0..count {
            self.palette(p + 3 + i * 6 + 2)?;
        }
        Ok(())
    }
    fn visit(&mut self, p: usize, depth: usize) -> Result<(), Error> {
        if depth >= MAX_DEPTH || self.nodes >= MAX_NODES {
            return Err(Error::TooComplex);
        }
        if self.ancestors[..depth].contains(&p) {
            return Err(Error::InvalidFont);
        }
        self.ancestors[depth] = p;
        self.nodes += 1;
        match self.bytes(p, 1)?[0] {
            1 => {
                let count = self.bytes(p + 1, 1)?[0] as usize;
                let first = self.offset(p + 2, 4)?;
                if count == 0 {
                    return Ok(());
                }
                if self.layers == 0 {
                    return Err(Error::InvalidFont);
                }
                let total = self.offset(self.layers, 4)?;
                self.bytes(
                    self.layers + 4,
                    total.checked_mul(4).ok_or(Error::InvalidFont)?,
                )?;
                if first.checked_add(count).is_none_or(|n| n > total) {
                    return Err(Error::InvalidFont);
                }
                for i in first..first + count {
                    let child = self.relative(self.layers, self.layers + 4 + i * 4, 4)?;
                    self.visit(child, depth + 1)?;
                }
            }
            2 => self.palette(p + 1)?,
            4 | 6 => {
                self.bytes(p, 16)?;
                self.line(self.relative(p, p + 1, 3)?)?;
            }
            10 => {
                if self.u16(p + 4)? >= self.face.number_of_glyphs() {
                    return Err(Error::InvalidFont);
                }
                self.visit(self.relative(p, p + 1, 3)?, depth + 1)?;
            }
            11 => self.visit(self.glyph(self.u16(p + 1)?)?, depth + 1)?,
            12 => {
                let transform = self.relative(p, p + 4, 3)?;
                self.bytes(transform, 24)?;
                self.visit(self.relative(p, p + 1, 3)?, depth + 1)?;
            }
            14 | 16 | 18 => {
                self.bytes(p, if self.bytes(p, 1)?[0] == 18 { 12 } else { 8 })?;
                self.visit(self.relative(p, p + 1, 3)?, depth + 1)?;
            }
            32 => {
                if ![3, 5, 20].contains(&self.bytes(p + 4, 1)?[0]) {
                    return Err(Error::Unsupported);
                }
                self.visit(self.relative(p, p + 5, 3)?, depth + 1)?;
                self.visit(self.relative(p, p + 1, 3)?, depth + 1)?;
            }
            _ => return Err(Error::Unsupported),
        }
        Ok(())
    }
}

pub(super) fn validate(face: &Face<'_>, glyph: u16) -> Result<(), Error> {
    if face.is_variable() {
        return Err(Error::Unsupported);
    }
    let data = face
        .raw_face()
        .table(Tag::from_bytes(b"COLR"))
        .ok_or(Error::Unsupported)?;
    let cpal = face
        .raw_face()
        .table(Tag::from_bytes(b"CPAL"))
        .ok_or(Error::InvalidFont)?;
    let palette = rustybuzz::ttf_parser::cpal::Table::parse(cpal).ok_or(Error::InvalidFont)?;
    let mut graph = Graph {
        bytes: data,
        face,
        palette,
        palette_entries: u16::from_be_bytes(cpal[2..4].try_into().map_err(|_| Error::InvalidFont)?),
        base: 0,
        layers: 0,
        nodes: 0,
        ancestors: [0; MAX_DEPTH],
    };
    if graph.u16(0)? != 1 {
        return Err(Error::Unsupported);
    }
    graph.base = graph.offset(14, 4)?;
    graph.layers = graph.offset(18, 4)?;
    if graph.base == 0 {
        return Err(Error::InvalidFont);
    }
    let clips = graph.offset(22, 4)?;
    if clips != 0 {
        if graph.bytes(clips, 1)?[0] != 1 {
            return Err(Error::Unsupported);
        }
        let count = graph.offset(clips + 1, 4)?;
        graph.bytes(clips + 5, count.checked_mul(7).ok_or(Error::InvalidFont)?)?;
        for i in 0..count {
            let p = clips + 5 + i * 7;
            if graph.u16(p)? > graph.u16(p + 2)? {
                return Err(Error::InvalidFont);
            }
            let bbox = graph.relative(clips, p + 4, 3)?;
            if graph.bytes(bbox, 1)?[0] != 1 {
                return Err(Error::Unsupported);
            }
            graph.bytes(bbox, 9)?;
        }
    }
    if !face.is_color_glyph(GlyphId(glyph)) {
        return Err(Error::Unsupported);
    }
    graph.visit(graph.glyph(glyph)?, 0)
}
