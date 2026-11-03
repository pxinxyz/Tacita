use super::{add, dot, mul, normal, sub, triangle, unit, PathError, MAX_POLYGON_POINTS};
use crate::draw::DrawList;
use alloc::vec::Vec;
fn turn(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f64 {
    (b[0] as f64 - a[0] as f64) * (c[1] as f64 - a[1] as f64)
        - (b[1] as f64 - a[1] as f64) * (c[0] as f64 - a[0] as f64)
}
fn on_segment(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> bool {
    turn(a, b, p).abs() <= 0.00000001
        && p[0] >= a[0].min(b[0])
        && p[0] <= a[0].max(b[0])
        && p[1] >= a[1].min(b[1])
        && p[1] <= a[1].max(b[1])
}
fn intersects(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> bool {
    let ac = turn(a, b, c);
    let ad = turn(a, b, d);
    let ca = turn(c, d, a);
    let cb = turn(c, d, b);
    (ac * ad < 0. && ca * cb < 0.)
        || on_segment(c, a, b)
        || on_segment(d, a, b)
        || on_segment(a, c, d)
        || on_segment(b, c, d)
}
fn inside(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    turn(a, b, p) >= -0.00000001 && turn(b, c, p) >= -0.00000001 && turn(c, a, p) >= -0.00000001
}
fn ear(points: &[[f32; 2]], i: usize, prev: &[usize], next: &[usize], live: &[bool]) -> bool {
    let a = prev[i];
    let c = next[i];
    if turn(points[a], points[i], points[c]) <= 0.00000001 {
        return false;
    }
    !(0..points.len()).any(|j| {
        live[j] && j != a && j != i && j != c && inside(points[j], points[a], points[i], points[c])
    })
}
pub(super) fn prepare(
    points: &mut Vec<[f32; 2]>,
    triangles: &mut Vec<[usize; 3]>,
) -> Result<(), PathError> {
    triangles.clear();
    if points.len() > MAX_POLYGON_POINTS {
        return Err(PathError::TooManyPoints);
    }
    if points.len() < 3 {
        return Err(PathError::Degenerate);
    }
    for i in 0..points.len() {
        let ni = (i + 1) % points.len();
        let pi = (i + points.len() - 1) % points.len();
        if turn(points[pi], points[i], points[ni]).abs() <= 0.00000001
            && dot(sub(points[i], points[pi]), sub(points[ni], points[i])) < 0.
        {
            return Err(PathError::NotSimple);
        }
        for j in i + 1..points.len() {
            let nj = (j + 1) % points.len();
            if j == ni || nj == i {
                continue;
            }
            if intersects(points[i], points[ni], points[j], points[nj]) {
                return Err(PathError::NotSimple);
            }
        }
    }
    let mut i = 0;
    while points.len() > 3 && i < points.len() {
        let a = points[(i + points.len() - 1) % points.len()];
        let b = points[i];
        let c = points[(i + 1) % points.len()];
        if turn(a, b, c).abs() <= 0.00000001 {
            points.remove(i);
            i = i.saturating_sub(1);
        } else {
            i += 1;
        }
    }
    let area = (0..points.len())
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            a[0] as f64 * b[1] as f64 - a[1] as f64 * b[0] as f64
        })
        .sum::<f64>();
    if area.abs() <= 0.00000001 {
        return Err(PathError::Degenerate);
    }
    if area < 0. {
        points.reverse();
    }
    let n = points.len();
    let mut prev = [0; MAX_POLYGON_POINTS];
    let mut next = prev;
    let mut live = [false; MAX_POLYGON_POINTS];
    let mut ears = live;
    for i in 0..n {
        prev[i] = (i + n - 1) % n;
        next[i] = (i + 1) % n;
        live[i] = true;
    }
    for (i, e) in ears[..n].iter_mut().enumerate() {
        *e = ear(points, i, &prev, &next, &live);
    }
    let mut left = n;
    while left > 3 {
        let index = (0..n).find(|&i| live[i] && ears[i]);
        let i = if let Some(i) = index {
            i
        } else {
            for (i, e) in ears[..n].iter_mut().enumerate() {
                *e = live[i] && ear(points, i, &prev, &next, &live);
            }
            (0..n)
                .find(|&i| live[i] && ears[i])
                .ok_or(PathError::NotSimple)?
        };
        let a = prev[i];
        let c = next[i];
        triangles.push([a, i, c]);
        live[i] = false;
        next[a] = c;
        prev[c] = a;
        left -= 1;
        ears[a] = ear(points, a, &prev, &next, &live);
        ears[c] = ear(points, c, &prev, &next, &live);
    }
    let a = (0..n).find(|&i| live[i]).ok_or(PathError::Degenerate)?;
    triangles.push([a, next[a], next[next[a]]]);
    Ok(())
}
pub(super) fn emit(
    draw: &mut DrawList,
    points: &[[f32; 2]],
    triangles: &[[usize; 3]],
    color: [u8; 4],
    scale: f32,
    aa: bool,
) {
    for &indices in triangles {
        triangle(draw, indices.map(|i| points[i]), [1.; 3], color);
    }
    if !aa || color[3] == 0 {
        return;
    }
    let fringe = crate::draw::FRINGE_WIDTH / scale;
    let offset = |i: usize| {
        let p = points[i];
        let before = unit(sub(p, points[(i + points.len() - 1) % points.len()]));
        let after = unit(sub(points[(i + 1) % points.len()], p));
        let n0 = mul(normal(before), -1.);
        let n1 = mul(normal(after), -1.);
        let bisector = unit(add(n0, n1));
        add(
            p,
            mul(
                bisector,
                (fringe / dot(bisector, n1).max(0.000001)).min(fringe * 4.),
            ),
        )
    };
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        let a = points[i];
        let b = points[j];
        let oa = offset(i);
        let ob = offset(j);
        triangle(draw, [a, b, ob], [1., 1., 0.], color);
        triangle(draw, [a, ob, oa], [1., 0., 0.], color);
    }
}
