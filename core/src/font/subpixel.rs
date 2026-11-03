use alloc::vec::Vec;

const PHASES: i64 = 4;
const GUARD: u32 = 1;

#[derive(Clone, Copy, Debug)]
pub(super) struct PhaseGlyph {
    pub origin: [i32; 2],
    size: [u32; 2],
    stride: [u32; 2],
    page: [u32; 2],
}

impl PhaseGlyph {
    pub fn dimensions(&self) -> [u32; 2] {
        [self.stride[0] * 4, self.stride[1] * 4]
    }

    pub fn placed(mut self, page: [u32; 2]) -> Self {
        self.page = page;
        self
    }

    pub fn rect(&self, x: u8, y: u8) -> [u32; 4] {
        [
            self.page[0] + u32::from(x) * self.stride[0] + GUARD,
            self.page[1] + u32::from(y) * self.stride[1] + GUARD,
            self.size[0],
            self.size[1],
        ]
    }
}

pub(super) fn quantize(value: f32) -> (f32, u8) {
    let remainder = value % 1.0;
    let fraction = if remainder < 0.0 {
        remainder + 1.0
    } else {
        remainder
    };
    let phase = (fraction * 4.0 + 0.5) as u8;
    (value - fraction + f32::from(phase / 4), phase % 4)
}

pub(super) fn rasterize(
    font: &fontdue::Font,
    ch: char,
    physical_size: f32,
    max_page: u32,
) -> Option<(PhaseGlyph, Vec<u8>)> {
    rasterize_indexed(font, font.lookup_glyph_index(ch), physical_size, max_page)
}

pub(super) fn rasterize_indexed(
    font: &fontdue::Font,
    index: u16,
    physical_size: f32,
    max_page: u32,
) -> Option<(PhaseGlyph, Vec<u8>)> {
    let px = physical_size * 4.0;
    if !px.is_finite() || px <= 0.0 {
        return None;
    }
    let metrics = font.metrics_indexed(index, px);
    if metrics.width == 0
        || metrics.height == 0
        || metrics.width > max_page as usize
        || metrics.height > max_page as usize
    {
        return None;
    }
    let high_origin = [
        i64::from(metrics.xmin),
        -i64::from(metrics.ymin) - metrics.height as i64,
    ];
    let origin = high_origin.map(|v| v.div_euclid(PHASES));
    let extent = [metrics.width as i64, metrics.height as i64];
    let size = core::array::from_fn::<_, 2, _>(|a| {
        (high_origin[a] + extent[a] + (PHASES - 1) * 2).div_euclid(PHASES) - origin[a]
    });
    let glyph = PhaseGlyph {
        origin: [
            i32::try_from(origin[0]).ok()?,
            i32::try_from(origin[1]).ok()?,
        ],
        size: [u32::try_from(size[0]).ok()?, u32::try_from(size[1]).ok()?],
        stride: [
            u32::try_from(size[0]).ok()?.checked_add(GUARD * 2)?,
            u32::try_from(size[1]).ok()?.checked_add(GUARD * 2)?,
        ],
        page: [0; 2],
    };
    let [width, height] = glyph.dimensions();
    if width > max_page || height > max_page {
        return None;
    }
    let (_, fine) = font.rasterize_indexed(index, px);
    let mut pixels = alloc::vec![0; width as usize * height as usize];
    for py in 0..4_u8 {
        for px in 0..4_u8 {
            let tile = glyph.rect(px, py);
            for y in 0..glyph.size[1] {
                for x in 0..glyph.size[0] {
                    let sx = (origin[0] + i64::from(x)) * PHASES - i64::from(px) - high_origin[0];
                    let sy = (origin[1] + i64::from(y)) * PHASES - i64::from(py) - high_origin[1];
                    let mut sum = 0_u32;
                    for dy in 0..PHASES {
                        for dx in 0..PHASES {
                            let xx = sx + dx;
                            let yy = sy + dy;
                            if xx >= 0 && xx < extent[0] && yy >= 0 && yy < extent[1] {
                                sum += u32::from(fine[yy as usize * metrics.width + xx as usize]);
                            }
                        }
                    }
                    pixels[((tile[1] + y) * width + tile[0] + x) as usize] = ((sum + 8) / 16) as u8;
                }
            }
        }
    }
    Some((glyph, pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subpixel_quantization_carries_across_positive_and_negative_pixels() {
        assert_eq!(quantize(10.26), (10.0, 1));
        assert_eq!(quantize(10.99), (11.0, 0));
        assert_eq!(quantize(-0.25), (-1.0, 3));
        assert_eq!(quantize(-0.01), (0.0, 0));
        assert_eq!(quantize(-10.75), (-11.0, 1));
    }

    #[test]
    fn subpixel_tiles_shift_coverage_without_changing_total_ink() {
        let font = fontdue::Font::from_bytes(
            &include_bytes!("../../tests/assets/DejaVuSans.ttf")[..],
            fontdue::FontSettings::default(),
        )
        .unwrap();
        let (glyph, pixels) = rasterize(&font, 'H', 16.0, 2048).unwrap();
        let width = glyph.dimensions()[0] as usize;
        let moments = |phase| {
            let [ox, oy, w, h] = glyph.rect(phase, phase);
            let mut sum = 0_f64;
            let mut mx = 0_f64;
            let mut my = 0_f64;
            for y in 0..h {
                for x in 0..w {
                    let a = f64::from(pixels[(oy + y) as usize * width + (ox + x) as usize]);
                    sum += a;
                    mx += a * (f64::from(x) + 0.5);
                    my += a * (f64::from(y) + 0.5);
                }
            }
            (sum, mx / sum, my / sum)
        };
        let a = moments(0);
        let b = moments(3);
        assert!((b.0 - a.0).abs() <= f64::from(glyph.size[0] * glyph.size[1]));
        assert!((b.1 - a.1 - 0.75).abs() < 0.1);
        assert!((b.2 - a.2 - 0.75).abs() < 0.1);
        for py in 0..4 {
            for px in 0..4 {
                let [x, y, w, h] = glyph.rect(px, py);
                for row in y - 1..=y + h {
                    assert_eq!(pixels[row as usize * width + (x - 1) as usize], 0);
                    assert_eq!(pixels[row as usize * width + (x + w) as usize], 0);
                }
                for col in x - 1..=x + w {
                    assert_eq!(pixels[(y - 1) as usize * width + col as usize], 0);
                    assert_eq!(pixels[(y + h) as usize * width + col as usize], 0);
                }
            }
        }
        assert!(rasterize(&font, 'H', 16.0, 8).is_none());
    }
}
