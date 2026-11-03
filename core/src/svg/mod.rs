mod parser;
pub(crate) mod path;
use crate::draw::sqrt_approx;
use alloc::vec::Vec;
use parser::parse;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SvgError {
    InvalidInput,
    Unsupported,
    TooComplex,
    AtlasFull,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SvgIconId(pub u32);
impl SvgIconId {
    pub const INVALID: Self = Self(u32::MAX);
}

pub struct SvgBitmap {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub(crate) const MAX_EDGES: usize = 8192;
pub(crate) type Matrix = [f32; 6];
pub(crate) const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
pub(crate) fn multiply(a: Matrix, b: Matrix) -> Matrix {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}
pub(crate) fn point(m: Matrix, p: [f32; 2]) -> [f32; 2] {
    [
        m[0] * p[0] + m[2] * p[1] + m[4],
        m[1] * p[0] + m[3] * p[1] + m[5],
    ]
}
pub(crate) fn inverse(m: Matrix) -> Result<Matrix, SvgError> {
    let d = m[0] * m[3] - m[1] * m[2];
    if !d.is_finite() || d.abs() < 1e-12 {
        return Err(SvgError::InvalidInput);
    }
    let r = [
        m[3] / d,
        -m[1] / d,
        -m[2] / d,
        m[0] / d,
        (m[2] * m[5] - m[3] * m[4]) / d,
        (m[1] * m[4] - m[0] * m[5]) / d,
    ];
    if r.iter().any(|v| !v.is_finite()) {
        return Err(SvgError::InvalidInput);
    }
    Ok(r)
}
#[derive(Clone, Copy)]
pub(crate) struct Edge {
    pub(crate) a: [f32; 2],
    pub(crate) b: [f32; 2],
    stroke: bool,
    first: bool,
    last: bool,
}
impl Edge {
    pub(crate) fn winding_at(&self, p: [f32; 2]) -> i32 {
        if (self.a[1] <= p[1] && self.b[1] > p[1]) || (self.b[1] <= p[1] && self.a[1] > p[1]) {
            let x =
                self.a[0] + (p[1] - self.a[1]) * (self.b[0] - self.a[0]) / (self.b[1] - self.a[1]);
            if x > p[0] {
                return if self.b[1] > self.a[1] { 1 } else { -1 };
            }
        }
        0
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Paint {
    fill: bool,
    stroke: bool,
    width: f32,
    fill_alpha: f32,
    stroke_alpha: f32,
    opacity: f32,
    even_odd: bool,
    round_caps: bool,
}
impl Default for Paint {
    fn default() -> Self {
        Self {
            fill: true,
            stroke: false,
            width: 1.0,
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            opacity: 1.0,
            even_odd: false,
            round_caps: false,
        }
    }
}
pub(crate) struct Shape {
    edges: Vec<Edge>,
    paint: Paint,
    inverse: Matrix,
}

fn coverage(shape: &Shape, p: [f32; 2]) -> f32 {
    let p = point(shape.inverse, p);
    let mut winding = 0_i32;
    let mut stroked = false;
    let radius = shape.paint.width * 0.5;
    for e in &shape.edges {
        winding += e.winding_at(p);
        if shape.paint.stroke && e.stroke && !stroked {
            let d = [e.b[0] - e.a[0], e.b[1] - e.a[1]];
            let length = d[0] * d[0] + d[1] * d[1];
            if length == 0.0 && !shape.paint.round_caps {
                continue;
            }
            let t = if length > 0.0 {
                ((p[0] - e.a[0]) * d[0] + (p[1] - e.a[1]) * d[1]) / length
            } else {
                0.0
            };
            if !shape.paint.round_caps && ((e.first && t < 0.0) || (e.last && t > 1.0)) {
                continue;
            }
            let q = [
                e.a[0] + t.clamp(0.0, 1.0) * d[0],
                e.a[1] + t.clamp(0.0, 1.0) * d[1],
            ];
            stroked =
                (p[0] - q[0]) * (p[0] - q[0]) + (p[1] - q[1]) * (p[1] - q[1]) <= radius * radius;
        }
    }
    let filled = shape.paint.fill
        && if shape.paint.even_odd {
            winding & 1 != 0
        } else {
            winding != 0
        };
    let fill = if filled { shape.paint.fill_alpha } else { 0.0 };
    let stroke = if stroked {
        shape.paint.stroke_alpha
    } else {
        0.0
    };
    (fill + stroke * (1.0 - fill)) * shape.paint.opacity
}

pub fn rasterize_svg(source: &str, width: u32, height: u32) -> Result<SvgBitmap, SvgError> {
    if width == 0 || height == 0 || width > 512 || height > 512 || source.len() > 262_144 {
        return Err(SvgError::InvalidInput);
    }
    let shapes = parse(source, width, height)?;
    let edges = shapes.iter().map(|s| s.edges.len()).sum::<usize>();
    if edges > MAX_EDGES
        || u64::from(width) * u64::from(height) * (edges + shapes.len()) as u64 * 16 > 128_000_000
    {
        return Err(SvgError::TooComplex);
    }
    let mut pixels = alloc::vec![0;width as usize*height as usize];
    for y in 0..height {
        for x in 0..width {
            let mut alpha = 0.0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let p = [
                        x as f32 + (sx as f32 + 0.5) * 0.25,
                        y as f32 + (sy as f32 + 0.5) * 0.25,
                    ];
                    let mut sample = 0.0;
                    for shape in &shapes {
                        let a = coverage(shape, p);
                        sample += a * (1.0 - sample);
                    }
                    alpha += sample;
                }
            }
            pixels[(y * width + x) as usize] = (alpha * 255.0 / 16.0 + 0.5) as u8;
        }
    }
    Ok(SvgBitmap {
        pixels,
        width,
        height,
    })
}

pub(crate) fn length(p: [f32; 2]) -> f32 {
    sqrt_approx(p[0] * p[0] + p[1] * p[1])
}

impl crate::context::Context {
    pub fn add_svg_icon(&mut self, source: &str, size: [f32; 2]) -> Result<SvgIconId, SvgError> {
        assert!(
            !self.frame_active,
            "SVG registration must run between frames"
        );
        self.font.add_svg_icon(source, size)
    }
}
impl crate::ui::Ui<'_> {
    pub fn draw_svg_icon(
        &mut self,
        icon: SvgIconId,
        rect: [f32; 4],
        tint: [u8; 4],
    ) -> Result<(), SvgError> {
        if !crate::animation::layout::valid_rect(rect) {
            return Err(SvgError::InvalidInput);
        }
        let mask = self
            .ctx
            .font
            .svg_icon_mask(icon)
            .ok_or(SvgError::InvalidInput)?;
        if rect[0] == rect[2] || rect[1] == rect[3] {
            return Ok(());
        }
        crate::widgets::draw_parts(self.ctx).1.add_textured_quad(
            crate::draw::TextureId::FONT,
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            mask.uv_min,
            mask.uv_max,
            tint,
        );
        Ok(())
    }
    pub fn svg_icon(
        &mut self,
        icon: SvgIconId,
        size: [f32; 2],
        tint: [u8; 4],
    ) -> Result<[f32; 4], SvgError> {
        if !size.iter().all(|v| v.is_finite() && *v >= 0.0) || !self.ctx.font.has_svg_icon(icon) {
            return Err(SvgError::InvalidInput);
        }
        let rect = self.allocate(size);
        self.draw_svg_icon(icon, rect, tint)?;
        Ok(rect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hole_transform_and_fractional_coverage() {
        let b=rasterize_svg(r#"<svg viewBox="0 0 16 16"><g transform="translate(1 1)"><path fill-rule="evenodd" d="M0 0h12v12h-12z M3 3h6v6h-6z"/></g></svg>"#,16,16).unwrap();
        assert_eq!(b.pixels[2 * 16 + 2], 255);
        assert_eq!(b.pixels[6 * 16 + 6], 0);
        let b = rasterize_svg(
            r#"<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="5.3"/></svg>"#,
            16,
            16,
        )
        .unwrap();
        assert!(b.pixels.iter().any(|a| *a > 0 && *a < 255));
    }
    #[test]
    fn curves_arcs_and_stroke_only() {
        let b=rasterize_svg(r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12 C3 3 21 3 21 12 S3 21 3 12 M8 12 Q12 6 16 12 T8 12 M12 3 A9 9 0 0 1 21 12"/></svg>"#,24,24).unwrap();
        assert!(b.pixels.iter().any(|a| *a > 200));
    }
    #[test]
    fn round_caps_extend_open_lines_and_preserve_zero_length_dots() {
        let butt = rasterize_svg(
            "<svg viewBox='0 0 16 16' stroke='white' stroke-width='4'><line x1='4' y1='8' x2='12' y2='8'/></svg>",
            16, 16,
        ).unwrap();
        let round = rasterize_svg(
            "<svg viewBox='0 0 16 16' stroke='white' stroke-width='4' stroke-linecap='round'><line x1='4' y1='8' x2='12' y2='8'/></svg>",
            16, 16,
        ).unwrap();
        assert_eq!(butt.pixels[8 * 16 + 2], 0);
        assert!(round.pixels[8 * 16 + 2] > 0);
        let dot = rasterize_svg(
            "<svg viewBox='0 0 16 16' stroke='white' stroke-width='4' stroke-linecap='round'><line x1='8' y1='8' x2='8' y2='8'/></svg>",
            16, 16,
        ).unwrap();
        assert_eq!(dot.pixels[8 * 16 + 8], 255);
    }
    #[test]
    fn rejects_malformed_external_and_oversized_input() {
        for s in [
            "<svg><path d='M0'/></svg>",
            "<svg><image href='x'/></svg>",
            "<svg><script/></svg>",
            "<svg><path d='MNaN 0'/></svg>",
            "<svg><g></svg>",
            "<svg style='filter:blur(3px)'/>",
        ] {
            assert!(rasterize_svg(s, 16, 16).is_err(), "{s}");
        }
        assert!(rasterize_svg("<svg/>", 513, 16).is_err());
    }
    #[test]
    fn atlas_registration_respects_density_and_growth() {
        let source = r#"<svg viewBox="0 0 16 16"><rect width="16" height="16"/></svg>"#;
        let mut font =
            crate::font::Font::new(include_bytes!("../../tests/assets/DejaVuSans.ttf"), 16.0)
                .unwrap();
        let generation = font.atlas_generation();
        let id = font.add_svg_icon(source, [16.0; 2]).unwrap();
        assert!(font.atlas_generation() > generation);
        let before = font.svg_icon_mask(id).unwrap();
        font.add_svg_icon(source, [512.0; 2]).unwrap();
        let after = font.svg_icon_mask(id).unwrap();
        let (_, width, height) = font.atlas_pixels();
        assert_eq!(after.rect, before.rect);
        assert_eq!(
            after.uv_min,
            [
                after.rect[0] as f32 / width as f32,
                after.rect[1] as f32 / height as f32
            ]
        );
        assert!(after.uv_min != before.uv_min);
        assert!(!font.has_svg_icon(SvgIconId::INVALID));
        let mut scaled = crate::font::Font::new_with_icons_at_scale(
            include_bytes!("../../tests/assets/DejaVuSans.ttf"),
            16.0,
            &[],
            1.5,
        )
        .unwrap();
        let id = scaled.add_svg_icon(source, [16.0; 2]).unwrap();
        assert_eq!(scaled.svg_icon_mask(id).unwrap().rect[2], 24);
        assert_eq!(scaled.svg_icon_size(id), Some([16.0; 2]));
    }
}
