use alloc::vec;
use alloc::vec::Vec;

pub(crate) fn rasterize_disk(radius: f32) -> (u32, u32, Vec<u8>) {
    if radius.is_finite() && radius > 0.0 {
        rasterize_annulus(radius as f64, 0.0)
    } else {
        (0, 0, Vec::new())
    }
}

pub(crate) fn rasterize_ring(radius: f32, thickness: f32) -> (u32, u32, Vec<u8>) {
    if radius > 0.0 && thickness > 0.0 {
        let outer = radius as f64 + thickness as f64 / 2.0;
        let inner = radius as f64 - thickness as f64 / 2.0;
        if outer.is_finite() {
            return rasterize_annulus(outer, inner.max(0.0));
        }
    }
    (0, 0, Vec::new())
}

fn rasterize_annulus(r_out: f64, r_in: f64) -> (u32, u32, Vec<u8>) {
    let diameter = 2.0 * r_out;
    if diameter + 2.0 > u32::MAX as f64 {
        return (0, 0, Vec::new());
    }
    let mut dim = diameter as u64;
    if (dim as f64) < diameter {
        dim += 1;
    }
    let w = (dim + 2) as usize;
    let center = w as f64 / 2.0;

    let disk = DiskTables::new(r_out, w, center);
    let hole = (r_in > 0.0).then(|| DiskTables::new(r_in, w, center));

    let mut bitmap = vec![0u8; w * w];
    for y in 0..w {
        for x in 0..w {
            let x0 = x as f64 - center;
            let x1 = x0 + 1.0;
            let mut cov = disk.pixel_coverage(x, x0, x1, y);
            if let Some(hole) = &hole {
                cov -= hole.pixel_coverage(x, x0, x1, y);
            }
            bitmap[y * w + x] = (cov.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        }
    }
    (w as u32, w as u32, bitmap)
}

pub(crate) fn rasterize_rounded_corner(radius: f32) -> (u32, u32, Vec<u8>) {
    if radius.is_finite() && radius > 0.0 {
        return rounded_ring_impl(radius as f64, 0.0);
    }
    (0, 0, Vec::new())
}

pub(crate) fn rasterize_rounded_ring(radius: f32, thickness: f32) -> (u32, u32, Vec<u8>) {
    if radius.is_finite() && radius > 0.0 && thickness > 0.0 {
        let inner = (radius as f64 - thickness as f64).max(0.0);
        return rounded_ring_impl(radius as f64, inner);
    }
    (0, 0, Vec::new())
}

fn rounded_ring_impl(r: f64, r_in: f64) -> (u32, u32, Vec<u8>) {
    if r + 2.0 > u32::MAX as f64 {
        return (0, 0, Vec::new());
    }
    let mut dim = r as u64;
    if (dim as f64) < r {
        dim += 1;
    }
    let w = (dim + 2) as usize;
    let ox = (w as f64 - r) / 2.0;
    let (cx, cy) = (ox + r, ox + r);

    let mut bitmap = vec![0u8; w * w];
    for py in 0..w {
        for px in 0..w {
            let (x0, y0) = (px as f64, py as f64);
            let (x1, y1) = (x0 + 1.0, y0 + 1.0);
            let (ix0, iy0) = (x0.max(ox), y0.max(ox));
            let (ix1, iy1) = (x1.min(ox + r), y1.min(ox + r));
            if ix0 < ix1 && iy0 < iy1 {
                let mut cov = disk_rect_coverage(ix0, iy0, ix1, iy1, cx, cy, r);
                if r_in > 0.0 {
                    cov -= disk_rect_coverage(ix0, iy0, ix1, iy1, cx, cy, r_in);
                }
                bitmap[py * w + px] = (cov.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }
    }
    (w as u32, w as u32, bitmap)
}

#[allow(clippy::too_many_arguments)]
fn disk_rect_coverage(x0: f64, y0: f64, x1: f64, y1: f64, cx: f64, cy: f64, r: f64) -> f64 {
    (disk_corner_area(x1 - cx, y1 - cy, r) - disk_corner_area(x0 - cx, y1 - cy, r))
        - (disk_corner_area(x1 - cx, y0 - cy, r) - disk_corner_area(x0 - cx, y0 - cy, r))
}

fn disk_corner_area(x: f64, y: f64, r: f64) -> f64 {
    corner_area(x, half_chord_integral(x, r), &RowEdge::new(y, r), r)
}

struct DiskTables {
    r: f64,
    hx: Vec<f64>,
    edges: Vec<RowEdge>,
}

impl DiskTables {
    fn new(r: f64, w: usize, center: f64) -> Self {
        let hx = (0..=w)
            .map(|i| half_chord_integral(i as f64 - center, r))
            .collect();
        let edges = (0..=w)
            .map(|j| RowEdge::new(j as f64 - center, r))
            .collect();
        Self { r, hx, edges }
    }

    fn pixel_coverage(&self, x: usize, x0: f64, x1: f64, y: usize) -> f64 {
        let (e0, e1) = (&self.edges[y], &self.edges[y + 1]);
        let (hx0, hx1) = (self.hx[x], self.hx[x + 1]);
        (corner_area(x1, hx1, e1, self.r) - corner_area(x0, hx0, e1, self.r))
            - (corner_area(x1, hx1, e0, self.r) - corner_area(x0, hx0, e0, self.r))
    }
}

enum RowEdge {
    Zero,
    Full,
    Pos { v: f64, uy: f64, huy: f64 },
    Neg { v: f64, uy: f64, huy: f64 },
}

impl RowEdge {
    fn new(v: f64, r: f64) -> Self {
        if v <= -r {
            Self::Zero
        } else if v >= r {
            Self::Full
        } else {
            let uy = sqrt_f64(r * r - v * v);
            let huy = half_chord_integral(uy, r);
            if v >= 0.0 {
                Self::Pos { v, uy, huy }
            } else {
                Self::Neg { v, uy, huy }
            }
        }
    }
}

fn corner_area(x: f64, hx: f64, edge: &RowEdge, r: f64) -> f64 {
    let q = core::f64::consts::FRAC_PI_4 * r * r;
    match *edge {
        RowEdge::Zero => 0.0,
        RowEdge::Full => 2.0 * hx + 2.0 * q,
        RowEdge::Pos { v, uy, huy } => {
            let minint = if x <= -uy {
                hx + q
            } else if x >= uy {
                q - 2.0 * huy + 2.0 * uy * v + hx
            } else {
                q - huy + v * (x + uy)
            };
            minint + hx + q
        }
        RowEdge::Neg { v, uy, huy } => {
            if x <= -uy {
                0.0
            } else if x >= uy {
                2.0 * uy * v + 2.0 * huy
            } else {
                v * (x + uy) + hx + huy
            }
        }
    }
}

fn half_chord_integral(x: f64, r: f64) -> f64 {
    let u = (x / r).clamp(-1.0, 1.0);
    let s = sqrt_f64(1.0 - u * u);
    0.5 * r * r * (u * s + 2.0 * atan_unit(u / (1.0 + s)))
}

fn sqrt_f64(x: f64) -> f64 {
    if x > 0.0 {
        let guess = f64::from_bits(0x1ff7_a3be_a91d_9b1b + (x.to_bits() >> 1));
        let y = 0.5 * (guess + x / guess);
        let y = 0.5 * (y + x / y);
        let y = 0.5 * (y + x / y);
        0.5 * (y + x / y)
    } else {
        0.0
    }
}

#[allow(clippy::approx_constant)]
pub(crate) fn atan_unit(x: f64) -> f64 {
    const ATAN_HI: [f64; 2] = [4.636_476_090_008_061e-1, 7.853_981_633_974_483e-1];
    const ATAN_LO: [f64; 2] = [2.269_877_745_296_168_7e-17, 3.061_616_997_868_383e-17];
    const P: [f64; 11] = [
        3.333_333_333_333_293e-1,
        -1.999_999_999_987_648_3e-1,
        1.428_571_427_250_346_6e-1,
        -1.111_111_040_546_235_6e-1,
        9.090_887_133_436_507e-2,
        -7.691_876_205_044_83e-2,
        6.661_073_137_387_531e-2,
        -5.833_570_133_790_573_5e-2,
        4.976_877_994_615_932_4e-2,
        -3.653_157_274_421_691_6e-2,
        1.628_582_011_536_578_2e-2,
    ];
    let ax = x.abs();
    let (t, id) = if ax < 0.4375 {
        (ax, None)
    } else if ax < 0.6875 {
        ((ax - 0.5) / (1.0 + 0.5 * ax), Some(0))
    } else {
        ((ax - 1.0) / (1.0 + ax), Some(1))
    };
    let z = t * t;
    let w = z * z;
    let s1 = z * (P[0] + w * (P[2] + w * (P[4] + w * (P[6] + w * (P[8] + w * P[10])))));
    let s2 = w * (P[1] + w * (P[3] + w * (P[5] + w * (P[7] + w * P[9]))));
    let r = match id {
        None => t - t * (s1 + s2),
        Some(i) => ATAN_HI[i] - ((t * (s1 + s2) - ATAN_LO[i]) - t),
    };
    if x < 0.0 {
        -r
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn sum_coverage(bitmap: &[u8]) -> f64 {
        bitmap.iter().map(|&b| b as f64).sum::<f64>() / 255.0
    }

    #[test]
    fn sqrt_f64_tracks_std() {
        assert_eq!(sqrt_f64(0.0), 0.0);
        assert_eq!(sqrt_f64(-4.0), 0.0);
        assert_eq!(sqrt_f64(f64::NAN), 0.0);
        for &x in &[
            0.25_f64, 1.0, 2.0, 3.0, 162.5, 400.0, 65_025.0, 1.0e6, 1.0e-20, 1.0e30,
        ] {
            let expected = x.sqrt();
            let rel = (sqrt_f64(x) - expected).abs() / expected;
            assert!(
                rel < 1e-13,
                "sqrt_f64({x}) = {}, std = {expected}",
                sqrt_f64(x)
            );
        }
    }

    #[test]
    fn atan_unit_tracks_std() {
        for i in -1000..=1000 {
            let x = i as f64 / 1000.0;
            let err = (atan_unit(x) - x.atan()).abs();
            assert!(
                err < 1e-13,
                "atan_unit({x}) = {}, std = {}",
                atan_unit(x),
                x.atan()
            );
        }
    }

    #[test]
    fn disk_dimensions_are_ceil_diameter_plus_padding() {
        assert_eq!(rasterize_disk(4.0).0, 10);
        assert_eq!(rasterize_disk(4.3).0, 11);
        assert_eq!(rasterize_disk(0.5).0, 3);
        let (w, h, px) = rasterize_disk(7.3);
        assert_eq!(w, h);
        assert_eq!(px.len(), (w * h) as usize);
    }

    #[test]
    fn disk_interior_pixels_fully_covered() {
        let (w, _, px) = rasterize_disk(4.0);
        let w = w as usize;
        for &(x, y) in &[(4, 4), (5, 5), (4, 5), (5, 4), (5, 6)] {
            assert_eq!(px[y * w + x], 255, "pixel ({x}, {y})");
        }
    }

    #[test]
    fn disk_exterior_pixels_uncovered() {
        let (w, _, px) = rasterize_disk(4.0);
        let w = w as usize;
        for &(x, y) in &[(0, 0), (9, 0), (0, 9), (9, 9), (0, 5), (5, 0)] {
            assert_eq!(px[y * w + x], 0, "pixel ({x}, {y})");
        }
    }

    #[test]
    fn disk_edge_pixel_centered_on_boundary_is_half_covered() {
        let r = 162.5_f32.sqrt();
        let (w, _, px) = rasterize_disk(r);
        assert_eq!(w, 28);
        let cov = px[26 * w as usize + 16] as f64 / 255.0;
        assert!((cov - 0.5).abs() <= 0.01, "edge pixel coverage {cov}");
    }

    #[test]
    fn disk_coverage_conserves_area() {
        for &(r, rel_tol) in &[(2.5_f32, 1e-2), (32.5, 1e-3), (100.0, 1e-3)] {
            let (_, _, px) = rasterize_disk(r);
            let exact = PI * (r as f64) * (r as f64);
            let got = sum_coverage(&px);
            assert!(
                (got - exact).abs() / exact < rel_tol,
                "r = {r}: sum {got}, π r² = {exact}"
            );
        }
    }

    #[test]
    fn disk_bitmap_is_symmetric_across_both_axes() {
        let (w, _, px) = rasterize_disk(7.3);
        let w = w as usize;
        for y in 0..w {
            for x in 0..w {
                assert_eq!(
                    px[y * w + x],
                    px[y * w + (w - 1 - x)],
                    "h-mirror ({x}, {y})"
                );
                assert_eq!(
                    px[y * w + x],
                    px[(w - 1 - y) * w + x],
                    "v-mirror ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn ring_bitmap_is_symmetric_across_both_axes() {
        let (w, _, px) = rasterize_ring(7.3, 3.4);
        let w = w as usize;
        for y in 0..w {
            for x in 0..w {
                assert_eq!(
                    px[y * w + x],
                    px[y * w + (w - 1 - x)],
                    "h-mirror ({x}, {y})"
                );
                assert_eq!(
                    px[y * w + x],
                    px[(w - 1 - y) * w + x],
                    "v-mirror ({x}, {y})"
                );
            }
        }
    }

    fn ref_corner_area(x: f64, y: f64, r: f64) -> f64 {
        if y <= -r || x <= -r {
            return 0.0;
        }
        let x = x.clamp(-r, r);
        let q = std::f64::consts::FRAC_PI_4 * r * r;
        let h = |xx: f64| {
            let xc = xx.clamp(-r, r);
            0.5 * (xc * (r * r - xc * xc).max(0.0).sqrt()
                + r * r * (xc / r).clamp(-1.0, 1.0).asin())
        };
        if y >= r {
            return 2.0 * h(x) + 2.0 * q;
        }
        let uy = (r * r - y * y).max(0.0).sqrt();
        let huy = h(uy);
        if y >= 0.0 {
            let minint = if x <= -uy {
                h(x) + q
            } else if x >= uy {
                q - 2.0 * huy + 2.0 * uy * y + h(x)
            } else {
                q - huy + y * (x + uy)
            };
            minint + h(x) + q
        } else if x <= -uy {
            0.0
        } else if x >= uy {
            2.0 * uy * y + 2.0 * huy
        } else {
            y * (x + uy) + h(x) + huy
        }
    }

    #[test]
    fn disk_matches_std_analytic_reference() {
        let r = 5.3_f64;
        let (w, _, px) = rasterize_disk(r as f32);
        let w = w as usize;
        let c = w as f64 / 2.0;
        for y in 0..w {
            for x in 0..w {
                let (x0, y0) = (x as f64 - c, y as f64 - c);
                let cov = ref_corner_area(x0 + 1.0, y0 + 1.0, r)
                    - ref_corner_area(x0, y0 + 1.0, r)
                    - ref_corner_area(x0 + 1.0, y0, r)
                    + ref_corner_area(x0, y0, r);
                let got = px[y * w + x] as f64 / 255.0;
                assert!(
                    (got - cov).abs() <= 0.51 / 255.0 + 1e-9,
                    "pixel ({x}, {y}): got {got}, reference {cov}"
                );
            }
        }
    }

    #[test]
    fn ring_hole_is_uncovered_and_band_is_full() {
        let (w, _, px) = rasterize_ring(10.0, 4.0);
        assert_eq!(w, 26);
        let w = w as usize;
        assert_eq!(px[13 * w + 13], 0, "hole pixel");
        assert_eq!(px[12 * w + 12], 0, "hole pixel (diag, dist √2·2 < 8)");
        assert_eq!(px[3 * w + 13], 255, "band pixel");
    }

    #[test]
    fn ring_coverage_conserves_area() {
        let (r, t) = (10.0_f32, 4.0_f32);
        let (_, _, px) = rasterize_ring(r, t);
        let (ro, ri) = ((r + t / 2.0) as f64, (r - t / 2.0) as f64);
        let exact = PI * (ro * ro - ri * ri);
        let got = sum_coverage(&px);
        assert!(
            (got - exact).abs() / exact < 1e-3,
            "ring sum {got}, π(ro²−ri²) = {exact}"
        );
    }

    #[test]
    fn ring_with_nonpositive_inner_radius_equals_disk() {
        let disk = rasterize_disk(9.0);
        assert_eq!(rasterize_ring(4.0, 10.0), disk);
        let disk20 = rasterize_disk(20.0);
        assert_eq!(rasterize_ring(10.0, 20.0), disk20);
    }

    #[test]
    fn degenerate_inputs_return_empty_without_panic() {
        let empty: (u32, u32, Vec<u8>) = (0, 0, Vec::new());
        assert_eq!(rasterize_disk(0.0), empty);
        assert_eq!(rasterize_disk(-2.0), empty);
        assert_eq!(rasterize_disk(f32::NAN), empty);
        assert_eq!(rasterize_disk(f32::INFINITY), empty);
        assert_eq!(rasterize_disk(3.0e9), empty);
        assert_eq!(rasterize_ring(0.0, 2.0), empty);
        assert_eq!(rasterize_ring(-1.0, 2.0), empty);
        assert_eq!(rasterize_ring(f32::NAN, 2.0), empty);
        assert_eq!(rasterize_ring(10.0, 0.0), empty);
        assert_eq!(rasterize_ring(10.0, -1.0), empty);
        assert_eq!(rasterize_ring(10.0, f32::NAN), empty);
        assert_eq!(rasterize_ring(f32::MAX, 2.0), empty);
        assert_eq!(rasterize_ring(f32::INFINITY, 2.0), empty);
    }

    #[test]
    fn rounded_corner_dimensions_are_ceil_radius_plus_padding() {
        let (w, h, px) = rasterize_rounded_corner(4.0);
        assert_eq!((w, h), (6, 6));
        assert_eq!(px.len(), (w * h) as usize);
        let (w, h, _) = rasterize_rounded_corner(4.3);
        assert_eq!((w, h), (7, 7));
        let (w, h, _) = rasterize_rounded_corner(0.5);
        assert_eq!((w, h), (3, 3));
    }

    #[test]
    fn rounded_corner_inner_full_outer_empty_edge_partial() {
        let (w, _, px) = rasterize_rounded_corner(4.0);
        let w = w as usize;
        let at = |x: usize, y: usize| px[y * w + x];
        assert_eq!(at(4, 4), 255, "inner corner pixel");
        assert_eq!(at(4, 2), 255, "pixel near the inner corner");
        assert_eq!(at(1, 1), 0, "beyond the arc");
        assert_eq!(at(0, 0), 0, "padding");
        assert_eq!(at(0, 3), 0, "padding on the arc's axis");
        assert_eq!(at(3, 0), 0, "padding on the arc's other axis");
        let edge = at(2, 1);
        assert!(
            edge > 0 && edge < 255,
            "arc pixel must be partial, got {edge}"
        );
        let edge = at(1, 2);
        assert!(
            edge > 0 && edge < 255,
            "arc pixel (mirrored) must be partial, got {edge}"
        );
    }

    #[test]
    fn rounded_corner_coverage_conserves_area() {
        for &(r, rel_tol) in &[(2.5_f32, 1e-2), (32.5, 1e-3)] {
            let (_, _, px) = rasterize_rounded_corner(r);
            let exact = PI * (r as f64) * (r as f64) / 4.0;
            let got = sum_coverage(&px);
            assert!(
                (got - exact).abs() / exact < rel_tol,
                "r = {r}: sum {got}, π r²/4 = {exact}"
            );
        }
    }

    #[test]
    fn rounded_corner_matches_std_analytic_reference() {
        let r = 5.3_f64;
        let (w, _, px) = rasterize_rounded_corner(r as f32);
        let w = w as usize;
        let ox = (w as f64 - r) / 2.0;
        let c = ox + r;
        for y in 0..w {
            for x in 0..w {
                let (x0, y0) = (x as f64, y as f64);
                let (ix0, iy0) = (x0.max(ox), y0.max(ox));
                let (ix1, iy1) = ((x0 + 1.0).min(ox + r), (y0 + 1.0).min(ox + r));
                let cov = if ix0 < ix1 && iy0 < iy1 {
                    ref_corner_area(ix1 - c, iy1 - c, r)
                        - ref_corner_area(ix0 - c, iy1 - c, r)
                        - ref_corner_area(ix1 - c, iy0 - c, r)
                        + ref_corner_area(ix0 - c, iy0 - c, r)
                } else {
                    0.0
                };
                let got = px[y * w + x] as f64 / 255.0;
                assert!(
                    (got - cov).abs() <= 0.51 / 255.0 + 1e-9,
                    "pixel ({x}, {y}): got {got}, reference {cov}"
                );
            }
        }
    }

    #[test]
    fn rounded_ring_dimensions_are_ceil_radius_plus_padding() {
        let (w, h, px) = rasterize_rounded_ring(4.0, 1.0);
        assert_eq!((w, h), (6, 6));
        assert_eq!(px.len(), (w * h) as usize);
        let (w, h, _) = rasterize_rounded_ring(4.3, 1.0);
        assert_eq!((w, h), (7, 7));
        let (w, h, _) = rasterize_rounded_ring(4.0, 4.0);
        assert_eq!((w, h), (6, 6));
    }

    #[test]
    fn rounded_ring_inner_empty_band_full_outer_empty() {
        let (w, _, px) = rasterize_rounded_ring(6.0, 2.0);
        let w = w as usize;
        let at = |x: usize, y: usize| px[y * w + x];
        assert_eq!(at(5, 5), 0, "inside the inner radius");
        assert_eq!(at(3, 3), 255, "fully inside the band");
        assert_eq!(at(1, 1), 0, "beyond the outer arc");
        assert_eq!(at(0, 0), 0, "padding");
        let outer_edge = at(2, 2);
        assert!(
            outer_edge > 0 && outer_edge < 255,
            "outer arc pixel must be partial, got {outer_edge}"
        );
        let inner_edge = at(4, 4);
        assert!(
            inner_edge > 0 && inner_edge < 255,
            "inner arc pixel must be partial, got {inner_edge}"
        );
    }

    #[test]
    fn rounded_ring_coverage_conserves_area() {
        for &(r, t, rel_tol) in &[(6.0_f32, 2.0_f32, 1e-2), (32.5, 4.25, 1e-3)] {
            let (_, _, px) = rasterize_rounded_ring(r, t);
            let (r, ri) = (r as f64, (r - t) as f64);
            let exact = PI * (r * r - ri * ri) / 4.0;
            let got = sum_coverage(&px);
            assert!(
                (got - exact).abs() / exact < rel_tol,
                "r = {r}, t = {t}: sum {got}, π(r²−ri²)/4 = {exact}"
            );
        }
    }

    #[test]
    fn rounded_ring_thickness_at_or_above_radius_is_the_full_corner() {
        let corner = rasterize_rounded_corner(5.0);
        assert_eq!(rasterize_rounded_ring(5.0, 5.0), corner);
        assert_eq!(rasterize_rounded_ring(5.0, 8.0), corner);
    }

    #[test]
    fn rounded_ring_matches_std_analytic_reference() {
        let (r, t) = (5.3_f64, 1.7_f64);
        let (w, _, px) = rasterize_rounded_ring(r as f32, t as f32);
        let w = w as usize;
        let ox = (w as f64 - r) / 2.0;
        let c = ox + r;
        for y in 0..w {
            for x in 0..w {
                let (x0, y0) = (x as f64, y as f64);
                let (ix0, iy0) = (x0.max(ox), y0.max(ox));
                let (ix1, iy1) = ((x0 + 1.0).min(ox + r), (y0 + 1.0).min(ox + r));
                let disk_cov = |rad: f64| {
                    if ix0 < ix1 && iy0 < iy1 {
                        ref_corner_area(ix1 - c, iy1 - c, rad)
                            - ref_corner_area(ix0 - c, iy1 - c, rad)
                            - ref_corner_area(ix1 - c, iy0 - c, rad)
                            + ref_corner_area(ix0 - c, iy0 - c, rad)
                    } else {
                        0.0
                    }
                };
                let cov = disk_cov(r) - disk_cov(r - t);
                let got = px[y * w + x] as f64 / 255.0;
                assert!(
                    (got - cov).abs() <= 0.51 / 255.0 + 1e-9,
                    "pixel ({x}, {y}): got {got}, reference {cov}"
                );
            }
        }
    }

    #[test]
    fn rounded_ring_degenerate_inputs_return_empty_without_panic() {
        let empty: (u32, u32, Vec<u8>) = (0, 0, Vec::new());
        assert_eq!(rasterize_rounded_ring(0.0, 1.0), empty);
        assert_eq!(rasterize_rounded_ring(-2.0, 1.0), empty);
        assert_eq!(rasterize_rounded_ring(f32::NAN, 1.0), empty);
        assert_eq!(rasterize_rounded_ring(f32::INFINITY, 1.0), empty);
        assert_eq!(rasterize_rounded_ring(4.0, 0.0), empty);
        assert_eq!(rasterize_rounded_ring(4.0, -1.0), empty);
        assert_eq!(rasterize_rounded_ring(4.0, f32::NAN), empty);
        assert_eq!(rasterize_rounded_ring(5.0e9, 1.0), empty);
    }

    #[test]
    fn rounded_corner_degenerate_inputs_return_empty_without_panic() {
        let empty: (u32, u32, Vec<u8>) = (0, 0, Vec::new());
        assert_eq!(rasterize_rounded_corner(0.0), empty);
        assert_eq!(rasterize_rounded_corner(-2.0), empty);
        assert_eq!(rasterize_rounded_corner(f32::NAN), empty);
        assert_eq!(rasterize_rounded_corner(f32::INFINITY), empty);
        assert_eq!(rasterize_rounded_corner(5.0e9), empty);
    }
}
