use super::geometry::MAX_POINTS;
use super::{BorderPlacement, RoundedRect, Shadow};
use crate::draw::{DrawList, DrawVert, TextureId, WHITE_PIXEL_UV};

fn vertex(pos: [f32; 2], mut col: [u8; 4], opacity: f32) -> DrawVert {
    col[3] = (col[3] as f32 * opacity.clamp(0.0, 1.0) + 0.5) as u8;
    DrawVert {
        pos,
        uv: WHITE_PIXEL_UV,
        col,
    }
}
pub(super) fn band(
    draw: &mut DrawList,
    shape: RoundedRect,
    steps: usize,
    distances: [f32; 2],
    alpha: [f32; 2],
    color: [u8; 4],
) {
    let mut outer = [[0.0; 2]; MAX_POINTS];
    let mut inner = [[0.0; 2]; MAX_POINTS];
    let n = shape.contour(distances[0], steps, &mut outer);
    shape.contour(distances[1], steps, &mut inner);
    for i in 0..n {
        let j = (i + 1) % n;
        draw.add_mesh_triangle(
            TextureId::WHITE,
            [
                vertex(outer[i], color, alpha[0]),
                vertex(outer[j], color, alpha[0]),
                vertex(inner[j], color, alpha[1]),
            ],
        );
        draw.add_mesh_triangle(
            TextureId::WHITE,
            [
                vertex(outer[i], color, alpha[0]),
                vertex(inner[j], color, alpha[1]),
                vertex(inner[i], color, alpha[1]),
            ],
        );
    }
}
fn body(draw: &mut DrawList, shape: RoundedRect, steps: usize, distance: f32, color: [u8; 4]) {
    let mut points = [[0.0; 2]; MAX_POINTS];
    let n = shape.contour(distance, steps, &mut points);
    let center = [
        (shape.rect[0] + shape.rect[2]) * 0.5,
        (shape.rect[1] + shape.rect[3]) * 0.5,
    ];
    for i in 0..n {
        draw.add_mesh_triangle(
            TextureId::WHITE,
            [
                vertex(center, color, 1.0),
                vertex(points[i], color, 1.0),
                vertex(points[(i + 1) % n], color, 1.0),
            ],
        );
    }
}
pub(super) fn fill(draw: &mut DrawList, shape: RoundedRect, scale: f32, aa: bool, color: [u8; 4]) {
    let steps = shape.steps(scale, 0.5 / scale);
    let half = if aa { 0.5 / scale } else { 0.0 };
    body(draw, shape, steps, -half, color);
    if aa {
        band(draw, shape, steps, [half, -half], [0.0, 1.0], color);
    }
}
pub(super) fn border(
    draw: &mut DrawList,
    shape: RoundedRect,
    scale: f32,
    aa: bool,
    color: [u8; 4],
    width: f32,
    placement: BorderPlacement,
) {
    let (outer, inner) = match placement {
        BorderPlacement::Inner => (0.0, -width),
        BorderPlacement::Center => (width * 0.5, -width * 0.5),
        BorderPlacement::Outer => (width, 0.0),
    };
    let half = if aa { 0.5 / scale } else { 0.0 };
    let steps = shape.steps(scale, outer + half);
    if inner <= -(shape.rect[2] - shape.rect[0]).min(shape.rect[3] - shape.rect[1]) * 0.5 {
        body(draw, shape, steps, outer - half, color);
        if aa {
            band(
                draw,
                shape,
                steps,
                [outer + half, outer - half],
                [0.0, 1.0],
                color,
            );
        }
        return;
    }
    if !aa {
        band(draw, shape, steps, [outer, inner], [1.0, 1.0], color);
        return;
    }
    let coverage = |d: f32| {
        ((outer - d) * scale + 0.5).clamp(0.0, 1.0) - ((inner - d) * scale + 0.5).clamp(0.0, 1.0)
    };
    let mut distances = [outer + half, outer - half, inner + half, inner - half];
    distances.sort_unstable_by(|a, b| b.total_cmp(a));
    for pair in distances.windows(2) {
        if pair[0] > pair[1] {
            band(
                draw,
                shape,
                steps,
                [pair[0], pair[1]],
                [coverage(pair[0]), coverage(pair[1])],
                color,
            );
        }
    }
}
pub(super) fn shadow(draw: &mut DrawList, shape: RoundedRect, scale: f32, spec: Shadow) {
    let shifted = shape.shifted(spec.offset);
    let steps = shifted.steps(scale, spec.spread + spec.blur);
    let mark = if spec.inset {
        draw.paint_mark()
    } else {
        [0; 2]
    };
    if spec.inset {
        let start = -spec.spread;
        let cover = spec.offset[0].abs() + spec.offset[1].abs();
        if cover > 0.0 || spec.spread > 0.0 {
            band(draw, shifted, steps, [cover, start], [1.0, 1.0], spec.color);
        }
        for i in 0..16 {
            let t = i as f32 / 16.0;
            let u = (i + 1) as f32 / 16.0;
            let opacity = |v: f32| {
                let q = 1.0 - v;
                q * q * (3.0 - 2.0 * q)
            };
            band(
                draw,
                shifted,
                steps,
                [start - spec.blur * t, start - spec.blur * u],
                [opacity(t), opacity(u)],
                spec.color,
            );
        }
        draw.clip_rounded_tail(mark, shape, scale);
    } else {
        body(draw, shifted, steps, spec.spread, spec.color);
        for i in 0..16 {
            let t = i as f32 / 16.0;
            let u = (i + 1) as f32 / 16.0;
            let opacity = |v: f32| {
                let q = 1.0 - v;
                q * q * (3.0 - 2.0 * q)
            };
            band(
                draw,
                shifted,
                steps,
                [spec.spread + spec.blur * u, spec.spread + spec.blur * t],
                [opacity(u), opacity(t)],
                spec.color,
            );
        }
    }
}
