use super::geometry::MAX_POINTS;
use crate::draw::DrawVert;

const CAPACITY: usize = MAX_POINTS + 3;
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    col: [f32; 4],
}
const ZERO: Vertex = Vertex {
    pos: [0.0; 2],
    uv: [0.0; 2],
    col: [0.0; 4],
};
impl From<DrawVert> for Vertex {
    fn from(v: DrawVert) -> Self {
        Self {
            pos: v.pos,
            uv: v.uv,
            col: v.col.map(f32::from),
        }
    }
}
impl Vertex {
    fn mix(self, other: Self, t: f32) -> Self {
        Self {
            pos: core::array::from_fn(|i| self.pos[i] + (other.pos[i] - self.pos[i]) * t),
            uv: core::array::from_fn(|i| self.uv[i] + (other.uv[i] - self.uv[i]) * t),
            col: core::array::from_fn(|i| self.col[i] + (other.col[i] - self.col[i]) * t),
        }
    }
    fn output(self) -> DrawVert {
        DrawVert {
            pos: self.pos,
            uv: self.uv,
            col: self.col.map(|v| (v.clamp(0.0, 255.0) + 0.5) as u8),
        }
    }
}
fn side(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

pub(crate) fn triangle(
    triangle: [DrawVert; 3],
    boundary: &[[f32; 2]],
    mut emit: impl FnMut([DrawVert; 3]),
) {
    let mut a = [ZERO; CAPACITY];
    let mut b = [ZERO; CAPACITY];
    for (dst, src) in a.iter_mut().zip(triangle) {
        *dst = src.into();
    }
    let mut n = 3;
    for i in 0..boundary.len() {
        let edge = [boundary[i], boundary[(i + 1) % boundary.len()]];
        if edge[0] == edge[1] {
            continue;
        }
        if n < 3 {
            return;
        }
        let mut count = 0;
        let mut previous = a[n - 1];
        let mut pd = side(edge[0], edge[1], previous.pos);
        for current in &a[..n] {
            let cd = side(edge[0], edge[1], current.pos);
            if (cd >= 0.0) != (pd >= 0.0) {
                b[count] = previous.mix(*current, (pd / (pd - cd)).clamp(0.0, 1.0));
                count += 1;
            }
            if cd >= 0.0 {
                b[count] = *current;
                count += 1;
            }
            previous = *current;
            pd = cd;
        }
        core::mem::swap(&mut a, &mut b);
        n = count;
    }
    for i in 1..n.saturating_sub(1) {
        emit([a[0].output(), a[i].output(), a[i + 1].output()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipping_interpolates_uv_and_color_without_texture_changes() {
        let v = |pos, uv, col| DrawVert { pos, uv, col };
        let input = [
            v([-10.0, 0.0], [0.0, 0.0], [0; 4]),
            v([10.0, 0.0], [1.0, 0.0], [200; 4]),
            v([10.0, 20.0], [1.0, 1.0], [200; 4]),
        ];
        let boundary = [[0.0, 0.0], [20.0, 0.0], [20.0, 30.0], [0.0, 30.0]];
        let mut found = false;
        triangle(input, &boundary, |tri| {
            for v in tri {
                assert!(v.pos[0] >= 0.0);
                if v.pos[0] == 0.0 {
                    assert_eq!(v.uv[0], 0.5);
                    assert_eq!(v.col, [100; 4]);
                    found = true;
                }
            }
        });
        assert!(found);
    }
}
