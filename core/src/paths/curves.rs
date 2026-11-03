use super::{add, dot, length, mul, sub, PathError, MAX_PATH_POINTS};
use alloc::vec::Vec;

#[derive(Clone, Copy)]
struct Segment {
    points: [[f32; 2]; 4],
    depth: u8,
}
fn distance(point: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let d = sub(b, a);
    let l = dot(d, d);
    let t = if l > 0. {
        (dot(sub(point, a), d) / l).clamp(0., 1.)
    } else {
        0.
    };
    length(sub(point, add(a, mul(d, t))))
}
pub(super) fn flatten(
    points: [[f32; 2]; 4],
    tolerance: f32,
    out: &mut Vec<[f32; 2]>,
) -> Result<(), PathError> {
    out.clear();
    out.push(points[0]);
    let empty = Segment {
        points: [[0.; 2]; 4],
        depth: 0,
    };
    let mut stack = [empty; 20];
    stack[0] = Segment { points, depth: 0 };
    let mut n = 1;
    while n > 0 {
        n -= 1;
        let segment = stack[n];
        let p = segment.points;
        if distance(p[1], p[0], p[3]).max(distance(p[2], p[0], p[3])) <= tolerance {
            if out.len() >= MAX_PATH_POINTS {
                return Err(PathError::TooComplex);
            }
            out.push(p[3]);
            continue;
        }
        if segment.depth >= 18 || n + 2 > stack.len() {
            return Err(PathError::TooComplex);
        }
        let a = mul(add(p[0], p[1]), 0.5);
        let b = mul(add(p[1], p[2]), 0.5);
        let c = mul(add(p[2], p[3]), 0.5);
        let d = mul(add(a, b), 0.5);
        let e = mul(add(b, c), 0.5);
        let middle = mul(add(d, e), 0.5);
        stack[n] = Segment {
            points: [middle, e, c, p[3]],
            depth: segment.depth + 1,
        };
        stack[n + 1] = Segment {
            points: [p[0], a, d, middle],
            depth: segment.depth + 1,
        };
        n += 2;
    }
    Ok(())
}
