use crate::draw::{DrawList, TextureId};
use crate::font::{Font, FontId};
use crate::ui::Ui;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Start,
    Center,
    End,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextOverflow {
    Clip,
    Truncate,
    Ellipsis,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextOptions {
    pub wrap: bool,
    pub overflow: TextOverflow,
    pub horizontal: TextAlign,
    pub vertical: TextAlign,
    pub line_spacing: f32,
    pub max_lines: usize,
}
impl Default for TextOptions {
    fn default() -> Self {
        Self {
            wrap: false,
            overflow: TextOverflow::Clip,
            horizontal: TextAlign::Start,
            vertical: TextAlign::Start,
            line_spacing: 0.0,
            max_lines: 0,
        }
    }
}
impl TextOptions {
    pub fn is_valid(&self) -> bool {
        self.line_spacing.is_finite() && self.line_spacing >= 0.0
    }
}
#[derive(Clone, Copy, Debug)]
pub struct TextSpan<'a> {
    pub text: &'a str,
    pub font: FontId,
    pub color: [u8; 4],
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMetrics {
    pub size: [f32; 2],
    pub lines: usize,
    pub truncated: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Position {
    span: usize,
    byte: usize,
}
#[derive(Clone, Copy)]
struct Unit {
    start: Position,
    end: Position,
    ch: char,
    font: FontId,
    color: [u8; 4],
}
fn raw<'a>(count: usize, get: &impl Fn(usize) -> TextSpan<'a>, mut p: Position) -> Option<Unit> {
    while p.span < count {
        let span = get(p.span);
        if let Some(ch) = span.text[p.byte..].chars().next() {
            return Some(Unit {
                start: p,
                end: Position {
                    byte: p.byte + ch.len_utf8(),
                    ..p
                },
                ch,
                font: span.font,
                color: span.color,
            });
        }
        p = Position {
            span: p.span + 1,
            byte: 0,
        };
    }
    None
}
fn unit<'a>(count: usize, get: &impl Fn(usize) -> TextSpan<'a>, p: Position) -> Option<Unit> {
    let mut u = raw(count, get, p)?;
    if u.ch == '\r' {
        match raw(count, get, u.end) {
            Some(next) if next.ch == '\n' => u.end = next.end,
            _ => {}
        }
        u.ch = '\n';
    }
    Some(u)
}
fn boundary<'a>(count: usize, get: &impl Fn(usize) -> TextSpan<'a>, p: Position) -> bool {
    use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};
    let mut offset = p.byte;
    let mut total = 0;
    for i in 0..count {
        let len = get(i).text.len();
        total += len;
        if i < p.span {
            offset += len;
        }
    }
    if offset == 0 || offset == total {
        return true;
    }
    let mut start = offset - p.byte;
    let mut span = p.span;
    while span < count && get(span).text.len() <= offset - start {
        start += get(span).text.len();
        span += 1;
    }
    let chunk = get(span).text;
    if offset > start
        && chunk.as_bytes()[offset - start].is_ascii()
        && chunk.as_bytes()[offset - start - 1].is_ascii()
    {
        return !(chunk.as_bytes()[offset - start - 1] == b'\r'
            && chunk.as_bytes()[offset - start] == b'\n');
    }
    let mut cursor = GraphemeCursor::new(offset, total, true);
    loop {
        match cursor.is_boundary(chunk, start) {
            Ok(result) => return result,
            Err(GraphemeIncomplete::PreContext(end)) => {
                let mut begin = 0;
                for i in 0..count {
                    let text = get(i).text;
                    if begin < end && end <= begin + text.len() {
                        cursor.provide_context(&text[..end - begin], begin);
                        break;
                    }
                    begin += text.len();
                }
            }
            _ => unreachable!("validated span position must have boundary context"),
        }
    }
}

fn skip_spaces<'a>(
    count: usize,
    get: &impl Fn(usize) -> TextSpan<'a>,
    mut p: Position,
) -> Position {
    while let Some(u) = unit(count, get, p) {
        if u.ch == '\n' || !u.ch.is_whitespace() {
            break;
        }
        p = u.end;
    }
    p
}
fn advance(font: &mut Font, u: Unit) -> f32 {
    if u.ch == '\t' {
        font.char_advance(u.font, ' ') * 4.0
    } else {
        font.char_advance(u.font, u.ch)
    }
}
#[derive(Clone, Copy)]
struct Line {
    start: Position,
    end: Position,
    next: Position,
    width: f32,
    ascent: f32,
    descent: f32,
    terminal: bool,
}
impl Line {
    fn height(&self) -> f32 {
        self.ascent + self.descent
    }
}
fn line<'a>(
    font: &mut Font,
    fallback: FontId,
    count: usize,
    get: &impl Fn(usize) -> TextSpan<'a>,
    start: Position,
    width: f32,
    wrap: bool,
) -> Line {
    let mut l = Line {
        start,
        end: start,
        next: start,
        width: 0.0,
        ascent: 0.0,
        descent: 0.0,
        terminal: false,
    };
    let mut seen = false;
    let mut break_at = None;
    let mut previous_space = false;
    loop {
        let Some(u) = unit(count, get, l.end) else {
            l.terminal = true;
            break;
        };
        let info = font.face_info(u.font).unwrap();
        if u.ch == '\n' {
            l.next = u.end;
            if !seen {
                l.ascent = info.ascent;
                l.descent = (info.line_height - info.ascent).max(0.0);
            }
            return l;
        }
        let space = u.ch.is_whitespace();
        if wrap && space && seen && !previous_space && boundary(count, get, u.end) {
            let mut candidate = l;
            candidate.next = skip_spaces(count, get, u.end);
            match unit(count, get, candidate.next) {
                Some(next) if next.ch == '\n' => candidate.next = next.end,
                None => candidate.terminal = true,
                _ => {}
            }
            break_at = Some(candidate);
        }
        let a = advance(font, u);
        if wrap && seen && l.width + a > width && boundary(count, get, u.start) {
            if let Some(candidate) = break_at {
                return candidate;
            }
            l.next = u.start;
            return l;
        }
        l.width += a;
        l.ascent = l.ascent.max(info.ascent);
        l.descent = l.descent.max((info.line_height - info.ascent).max(0.0));
        l.end = u.end;
        l.next = u.end;
        seen = true;
        previous_space = space;
    }
    if !seen {
        let info = font.face_info(fallback).unwrap();
        l.ascent = info.ascent;
        l.descent = (info.line_height - info.ascent).max(0.0);
    }
    l
}
fn validate<'a>(
    font: &mut Font,
    count: usize,
    get: &impl Fn(usize) -> TextSpan<'a>,
    options: TextOptions,
    width: f32,
) {
    assert!(
        options.is_valid() && width.is_finite() && width >= 0.0,
        "invalid text options or width"
    );
    for i in 0..count {
        let s = get(i);
        assert!(font.is_valid(s.font), "unregistered text span font");
    }
    if options.overflow == TextOverflow::Ellipsis {
        for i in 0..count {
            font.char_advance(get(i).font, '…');
        }
    }
}
fn measure<'a>(
    font: &mut Font,
    fallback: FontId,
    count: usize,
    get: &impl Fn(usize) -> TextSpan<'a>,
    width: f32,
    height: f32,
    options: TextOptions,
) -> TextMetrics {
    let mut result = TextMetrics::default();
    let mut p = Position::default();
    loop {
        let l = line(font, fallback, count, get, p, width, options.wrap);
        let spacing = if result.lines > 0 {
            options.line_spacing
        } else {
            0.0
        };
        if result.size[1] + spacing + l.height() > height {
            result.truncated = true;
            break;
        }
        result.lines += 1;
        result.size[1] += spacing + l.height();
        result.truncated |= l.width > width;
        if l.terminal {
            break;
        }
        if options.max_lines != 0 && result.lines >= options.max_lines {
            result.truncated = true;
            break;
        }
        p = l.next;
    }
    p = Position::default();
    for i in 0..result.lines {
        let l = line(font, fallback, count, get, p, width, options.wrap);
        let fit = fit_line(
            font,
            count,
            get,
            l,
            width,
            options.overflow,
            i + 1 == result.lines && !l.terminal,
        );
        result.size[0] = result.size[0].max(fit.width.min(width));
        p = l.next;
    }
    result
}
fn offset(align: TextAlign, room: f32) -> f32 {
    match align {
        TextAlign::Start => 0.0,
        TextAlign::Center => room * 0.5,
        TextAlign::End => room,
    }
}
fn glyph(draw: &mut DrawList, font: &mut Font, u: Unit, pen: [f32; 2]) {
    if u.ch == '\t' {
        return;
    }
    if let Some(q) = font.glyph_quad(u.font, u.ch, pen).1 {
        draw.add_textured_quad(TextureId::FONT, q.min, q.max, q.uv_min, q.uv_max, u.color);
    }
}
struct Fit {
    end: Position,
    width: f32,
    marker: Option<Unit>,
}
fn fit_line<'a>(
    font: &mut Font,
    count: usize,
    get: &impl Fn(usize) -> TextSpan<'a>,
    l: Line,
    width: f32,
    overflow: TextOverflow,
    hidden_tail: bool,
) -> Fit {
    let first = unit(count, get, l.start);
    let mut marker =
        first.filter(|_| overflow == TextOverflow::Ellipsis && (l.width > width || hidden_tail));
    let marker_width = marker.map_or(0.0, |u| font.char_advance(u.font, '…'));
    let mut end = l.end;
    let mut visible_width = l.width;
    if overflow != TextOverflow::Clip && (l.width > width || marker.is_some()) {
        visible_width = 0.0;
        let mut p = l.start;
        let mut fitted_width = 0.;
        end = p;
        while let Some(u) = unit(count, get, p).filter(|u| u.start < l.end) {
            let a = advance(font, u);
            if visible_width + a + marker_width > width {
                break;
            }
            visible_width += a;
            p = u.end;
            if boundary(count, get, p) {
                end = p;
                fitted_width = visible_width;
            }
        }
        visible_width = fitted_width;
    }
    if marker_width > width {
        marker = None;
        end = l.start;
        visible_width = 0.0;
    }
    let displayed_width = visible_width + if marker.is_some() { marker_width } else { 0.0 };
    Fit {
        end,
        width: displayed_width,
        marker,
    }
}
fn emit_line<'a>(
    parts: (&mut Font, &mut DrawList),
    count: usize,
    get: &impl Fn(usize) -> TextSpan<'a>,
    l: Line,
    rect: [f32; 4],
    options: TextOptions,
    hidden_tail: bool,
) {
    let (font, draw) = parts;
    let width = rect[2] - rect[0];
    let fit = fit_line(font, count, get, l, width, options.overflow, hidden_tail);
    let mut x = rect[0] + offset(options.horizontal, width - fit.width);
    let baseline = rect[1] + l.ascent;
    let mut p = l.start;
    while let Some(u) = unit(count, get, p).filter(|u| u.start < fit.end) {
        let ascent = font.face_info(u.font).unwrap().ascent;
        glyph(draw, font, u, [x, baseline - ascent]);
        x += advance(font, u);
        p = u.end;
    }
    if let Some(mut u) = fit.marker {
        u.ch = '…';
        let ascent = font.face_info(u.font).unwrap().ascent;
        glyph(draw, font, u, [x, baseline - ascent]);
    }
}
fn draw<'a>(
    parts: (&mut Font, &mut DrawList),
    fallback: FontId,
    count: usize,
    get: &impl Fn(usize) -> TextSpan<'a>,
    rect: [f32; 4],
    options: TextOptions,
    metrics: TextMetrics,
) {
    let (font, draw) = parts;
    draw.push_clip_rect(rect);
    let mut p = Position::default();
    let mut y = rect[1] + offset(options.vertical, rect[3] - rect[1] - metrics.size[1]);
    for i in 0..metrics.lines {
        let l = line(
            font,
            fallback,
            count,
            get,
            p,
            rect[2] - rect[0],
            options.wrap,
        );
        let hidden = i + 1 == metrics.lines && !l.terminal;
        emit_line(
            (font, draw),
            count,
            get,
            l,
            [rect[0], y, rect[2], rect[3]],
            options,
            hidden,
        );
        y += l.height() + options.line_spacing;
        p = l.next;
    }
    draw.pop_clip_rect();
}
impl Ui<'_> {
    pub fn active_font(&self) -> FontId {
        self.ctx.active_font()
    }
    pub fn measure_rich_text(
        &mut self,
        spans: &[TextSpan<'_>],
        width: f32,
        options: TextOptions,
    ) -> TextMetrics {
        self.measure_rich_text_with(spans.len(), |i| spans[i], width, options)
    }
    pub fn measure_rich_text_with<'a>(
        &mut self,
        count: usize,
        get: impl Fn(usize) -> TextSpan<'a>,
        width: f32,
        options: TextOptions,
    ) -> TextMetrics {
        let face = self.ctx.active_font();
        validate(&mut self.ctx.font, count, &get, options, width);
        measure(
            &mut self.ctx.font,
            face,
            count,
            &get,
            width,
            f32::INFINITY,
            options,
        )
    }
    pub fn measure_text_block(
        &mut self,
        text: &str,
        width: f32,
        options: TextOptions,
    ) -> TextMetrics {
        let span = TextSpan {
            text,
            font: self.ctx.active_font(),
            color: self.ctx.style.text,
        };
        self.measure_rich_text(&[span], width, options)
    }
    pub fn rich_text(
        &mut self,
        spans: &[TextSpan<'_>],
        width: f32,
        options: TextOptions,
    ) -> TextMetrics {
        self.rich_text_with(spans.len(), |i| spans[i], width, options)
    }
    pub fn rich_text_with<'a>(
        &mut self,
        count: usize,
        get: impl Fn(usize) -> TextSpan<'a>,
        width: f32,
        options: TextOptions,
    ) -> TextMetrics {
        let face = self.ctx.active_font();
        validate(&mut self.ctx.font, count, &get, options, width);
        let metrics = measure(
            &mut self.ctx.font,
            face,
            count,
            &get,
            width,
            f32::INFINITY,
            options,
        );
        let rect = crate::widgets::item_rect(self.ctx, [width, metrics.size[1]]);
        let (font, list) = crate::widgets::draw_parts(self.ctx);
        draw((font, list), face, count, &get, rect, options, metrics);
        metrics
    }
    pub fn text_block(&mut self, text: &str, width: f32, options: TextOptions) -> TextMetrics {
        let span = TextSpan {
            text,
            font: self.ctx.active_font(),
            color: self.ctx.style.text,
        };
        self.rich_text(&[span], width, options)
    }
    pub fn draw_rich_text_box(
        &mut self,
        rect: [f32; 4],
        spans: &[TextSpan<'_>],
        options: TextOptions,
    ) -> TextMetrics {
        self.draw_rich_text_box_with(rect, spans.len(), |i| spans[i], options)
    }
    pub fn draw_rich_text_box_with<'a>(
        &mut self,
        rect: [f32; 4],
        count: usize,
        get: impl Fn(usize) -> TextSpan<'a>,
        options: TextOptions,
    ) -> TextMetrics {
        assert!(
            crate::animation::layout::valid_rect(rect),
            "invalid text box"
        );
        let face = self.ctx.active_font();
        validate(&mut self.ctx.font, count, &get, options, rect[2] - rect[0]);
        let (font, list) = crate::widgets::draw_parts(self.ctx);
        let metrics = measure(
            font,
            face,
            count,
            &get,
            rect[2] - rect[0],
            rect[3] - rect[1],
            options,
        );
        draw((font, list), face, count, &get, rect, options, metrics);
        metrics
    }
    pub fn draw_text_box(
        &mut self,
        rect: [f32; 4],
        text: &str,
        color: [u8; 4],
        options: TextOptions,
    ) -> TextMetrics {
        let span = TextSpan {
            text,
            font: self.ctx.active_font(),
            color,
        };
        self.draw_rich_text_box(rect, &[span], options)
    }
}
