use alloc::vec::Vec;

use crate::font::{Font, FontId};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawVert {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub col: [u8; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TextureId(pub u64);

impl TextureId {
    pub const FONT: TextureId = TextureId(0);
    pub const WHITE: TextureId = TextureId(1);
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawCmd {
    pub effects: u32,
    pub elem_count: u32,
    pub clip_rect: [f32; 4],
    pub texture: TextureId,
}

#[derive(Debug)]
pub struct DrawData<'a> {
    pub effects: &'a [crate::effects::DrawEffects],
    pub vertices: &'a [DrawVert],
    pub indices: &'a [u32],
    pub commands: &'a [DrawCmd],
}

pub(crate) const WHITE_PIXEL_UV: [f32; 2] = [0.0, 0.0];

pub(crate) const FRINGE_WIDTH: f32 = 1.38;

const UNBOUNDED_CLIP_RECT: [f32; 4] = [
    f32::MIN / 2.0,
    f32::MIN / 2.0,
    f32::MAX / 2.0,
    f32::MAX / 2.0,
];

pub(crate) struct DrawList {
    pub(crate) path_scratch: crate::paths::PathScratch,
    effects: Vec<crate::effects::DrawEffects>,
    current_effects: crate::effects::DrawEffects,
    current_effect_index: Option<u32>,
    vertices: Vec<DrawVert>,
    indices: Vec<u32>,
    commands: Vec<DrawCmd>,
    split_command: bool,
    layer_indices: Vec<u32>,
    layer_commands: Vec<DrawCmd>,
    clip_stack: Vec<[f32; 4]>,
}

impl DrawList {
    pub(crate) fn add_mesh_triangle(&mut self, texture: TextureId, triangle: [DrawVert; 3]) {
        let [a, b, c] = triangle.map(|v| v.pos);
        let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        if area.abs() <= 0.00000001 || triangle.iter().all(|v| v.col[3] == 0) {
            return;
        }
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&triangle);
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
        self.push_indices(texture, 3);
    }

    pub(crate) fn clip_rounded_tail(
        &mut self,
        mark: [usize; 2],
        shape: crate::shapes::RoundedRect,
        scale: f32,
    ) {
        let mut boundary = [[0.0; 2]; crate::shapes::geometry::MAX_POINTS];
        let count = shape.contour(0.0, shape.steps(scale, 0.0), &mut boundary);
        let radius = shape.radii.into_iter().fold(0.0_f32, f32::max);
        let safe = [
            shape.rect[0] + radius,
            shape.rect[1] + radius,
            shape.rect[2] - radius,
            shape.rect[3] - radius,
        ];
        self.layer_indices.clear();
        self.layer_commands.clear();
        let mut offset = mark[0];
        let command_end = self.commands.len();
        for i in mark[1]..command_end {
            let mut command = self.commands[i].clone();
            let end = offset + command.elem_count as usize;
            let start = self.layer_indices.len();
            for index in (offset..end).step_by(3) {
                let indices = [
                    self.indices[index],
                    self.indices[index + 1],
                    self.indices[index + 2],
                ];
                let triangle = indices.map(|n| self.vertices[n as usize]);
                if triangle.iter().all(|v| {
                    v.pos[0] >= safe[0]
                        && v.pos[0] <= safe[2]
                        && v.pos[1] >= safe[1]
                        && v.pos[1] <= safe[3]
                }) {
                    self.layer_indices.extend_from_slice(&indices);
                } else {
                    crate::shapes::clipping::triangle(triangle, &boundary[..count], |triangle| {
                        let [a, b, c] = triangle.map(|v| v.pos);
                        if ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs()
                            <= 0.00000001
                        {
                            return;
                        }
                        let base = self.vertices.len() as u32;
                        self.vertices.extend_from_slice(&triangle);
                        self.layer_indices
                            .extend_from_slice(&[base, base + 1, base + 2]);
                    });
                }
            }
            command.elem_count = (self.layer_indices.len() - start) as u32;
            if command.elem_count > 0 {
                self.layer_commands.push(command);
            }
            offset = end;
        }
        self.indices.truncate(mark[0]);
        self.indices.extend_from_slice(&self.layer_indices);
        self.commands.truncate(mark[1]);
        self.commands.extend_from_slice(&self.layer_commands);
        self.split_command = true;
    }

    pub(crate) fn new() -> Self {
        Self {
            path_scratch: Default::default(),
            effects: Vec::new(),
            current_effects: Default::default(),
            current_effect_index: None,
            vertices: Vec::new(),
            indices: Vec::new(),
            commands: Vec::new(),
            split_command: false,
            layer_indices: Vec::new(),
            layer_commands: Vec::new(),
            clip_stack: alloc::vec![UNBOUNDED_CLIP_RECT],
        }
    }

    pub(crate) fn as_draw_data(&self) -> DrawData<'_> {
        DrawData {
            effects: &self.effects,
            vertices: &self.vertices,
            indices: &self.indices,
            commands: &self.commands,
        }
    }

    pub(crate) fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    pub(crate) fn index_count(&self) -> usize {
        self.indices.len()
    }
    pub(crate) fn paint_mark(&mut self) -> [usize; 2] {
        self.split_command = true;
        [self.indices.len(), self.commands.len()]
    }
    pub(crate) fn order_paint(
        &mut self,
        ranges: &[PaintRange],
        start: [usize; 2],
        end: [usize; 2],
    ) {
        if ranges.is_empty() {
            return;
        }
        let contiguous = ranges[0].start == start
            && ranges[ranges.len() - 1].end == end
            && ranges.windows(2).all(|r| r[0].end == r[1].start);
        if contiguous && ranges.windows(2).all(|r| r[0].layer <= r[1].layer) {
            return;
        }
        let mut order = [0_usize; crate::layout::MAX_LAYOUT_ITEMS];
        for (i, slot) in order[..ranges.len()].iter_mut().enumerate() {
            *slot = i;
        }
        order[..ranges.len()].sort_unstable_by_key(|i| (ranges[*i].layer, *i));
        self.layer_indices.clear();
        self.layer_commands.clear();
        let mut gap = start;
        for range in ranges {
            self.layer_indices
                .extend_from_slice(&self.indices[gap[0]..range.start[0]]);
            self.layer_commands
                .extend_from_slice(&self.commands[gap[1]..range.start[1]]);
            gap = range.end;
        }
        self.layer_indices
            .extend_from_slice(&self.indices[gap[0]..end[0]]);
        self.layer_commands
            .extend_from_slice(&self.commands[gap[1]..end[1]]);
        for &i in &order[..ranges.len()] {
            let range = ranges[i];
            self.layer_indices
                .extend_from_slice(&self.indices[range.start[0]..range.end[0]]);
            self.layer_commands
                .extend_from_slice(&self.commands[range.start[1]..range.end[1]]);
        }
        assert_eq!(
            end[0] - start[0],
            self.layer_indices.len(),
            "paint ranges must partition their tail"
        );
        assert_eq!(end[1] - start[1], self.layer_commands.len());
        self.indices[start[0]..end[0]].copy_from_slice(&self.layer_indices);
        self.commands[start[1]..end[1]].clone_from_slice(&self.layer_commands);
    }

    pub(crate) fn fade_since(&mut self, start: usize, opacity: f32) {
        if opacity >= 1.0 {
            return;
        }
        for vertex in &mut self.vertices[start..] {
            vertex.col[3] = (vertex.col[3] as f32 * opacity.clamp(0.0, 1.0) + 0.5) as u8;
        }
    }
    pub(crate) fn copy_tail(&mut self, source: &Self, vertex_start: usize, index_start: usize) {
        self.clear();
        self.effects.extend_from_slice(&source.effects);
        self.vertices
            .extend_from_slice(&source.vertices[vertex_start..]);
        self.indices.extend(
            source.indices[index_start..]
                .iter()
                .map(|i| i - vertex_start as u32),
        );
        let mut offset = 0;
        for command in &source.commands {
            let end = offset + command.elem_count as usize;
            if end > index_start {
                let mut command = command.clone();
                command.elem_count = (end - offset.max(index_start)) as u32;
                self.commands.push(command);
            }
            offset = end;
        }
    }
    pub(crate) fn append_faded(&mut self, source: &Self, opacity: f32) {
        let base = self.vertices.len() as u32;
        let effect_base = self.effects.len() as u32;
        self.effects.extend_from_slice(&source.effects);
        let clip = self.current_clip();
        self.vertices.extend(source.vertices.iter().map(|v| {
            let mut v = *v;
            v.col[3] = (v.col[3] as f32 * opacity.clamp(0.0, 1.0) + 0.5) as u8;
            v
        }));
        self.indices.extend(source.indices.iter().map(|i| i + base));
        self.commands.extend(source.commands.iter().map(|cmd| {
            let mut cmd = cmd.clone();
            if cmd.effects != 0 {
                cmd.effects += effect_base;
            }
            cmd.clip_rect = intersect_rects(cmd.clip_rect, clip);
            cmd
        }));
    }

    pub(crate) fn add_filled_rect(&mut self, min: [f32; 2], max: [f32; 2], col: [u8; 4]) {
        self.add_textured_quad(
            TextureId::WHITE,
            min,
            max,
            WHITE_PIXEL_UV,
            WHITE_PIXEL_UV,
            col,
        );
    }

    pub(crate) fn add_filled_rect_fringe(&mut self, min: [f32; 2], max: [f32; 2], col: [u8; 4]) {
        self.add_rect_fringe_ring([min[0], min[1], max[0], max[1]], false, col);
    }

    fn add_rect_fringe_ring(&mut self, rect: [f32; 4], inside: bool, col: [u8; 4]) {
        let [x0, y0, x1, y1] = rect;
        let f = if inside {
            FRINGE_WIDTH.min((x1 - x0) * 0.5).min((y1 - y0) * 0.5)
        } else {
            FRINGE_WIDTH
        };
        let offset = if inside { f } else { -f };
        let edge = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
        let fade_edge = [
            [x0 + offset, y0 + offset],
            [x1 - offset, y0 + offset],
            [x1 - offset, y1 - offset],
            [x0 + offset, y1 - offset],
        ];
        self.add_quad_fringe_ring(edge, fade_edge, inside, col);
    }

    fn add_quad_fringe_ring(
        &mut self,
        edge: [[f32; 2]; 4],
        fade_edge: [[f32; 2]; 4],
        inside: bool,
        col: [u8; 4],
    ) {
        let fade = [col[0], col[1], col[2], 0];
        for i in 0..4 {
            let j = (i + 1) % 4;
            let corners = if inside {
                [
                    (edge[i], col),
                    (edge[j], col),
                    (fade_edge[j], fade),
                    (fade_edge[i], fade),
                ]
            } else {
                [
                    (edge[i], col),
                    (fade_edge[i], fade),
                    (fade_edge[j], fade),
                    (edge[j], col),
                ]
            };
            let base = self.vertices.len() as u32;
            self.vertices.extend(corners.map(|(pos, col)| DrawVert {
                pos,
                uv: WHITE_PIXEL_UV,
                col,
            }));
            self.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        self.push_indices(TextureId::WHITE, 24);
    }

    pub(crate) fn add_solid_quad(&mut self, corners: [[f32; 2]; 4], col: [u8; 4]) {
        let base = self.vertices.len() as u32;
        self.vertices
            .extend_from_slice(&corners.map(|pos| DrawVert {
                pos,
                uv: WHITE_PIXEL_UV,
                col,
            }));
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        self.push_indices(TextureId::WHITE, 6);
    }

    pub(crate) fn add_gradient_grid(
        &mut self,
        min: [f32; 2],
        max: [f32; 2],
        cols: u32,
        rows: u32,
        color_at: impl Fn(u32, u32) -> [u8; 4],
    ) {
        if cols == 0 || rows == 0 {
            return;
        }
        let x_at = |cx: u32| min[0] + (max[0] - min[0]) * cx as f32 / cols as f32;
        let y_at = |cy: u32| min[1] + (max[1] - min[1]) * cy as f32 / rows as f32;
        for cy in 0..rows {
            for cx in 0..cols {
                let base = self.vertices.len() as u32;
                let (x0, x1) = (x_at(cx), x_at(cx + 1));
                let (y0, y1) = (y_at(cy), y_at(cy + 1));
                self.vertices.extend_from_slice(&[
                    DrawVert {
                        pos: [x0, y0],
                        uv: WHITE_PIXEL_UV,
                        col: color_at(cx, cy),
                    },
                    DrawVert {
                        pos: [x1, y0],
                        uv: WHITE_PIXEL_UV,
                        col: color_at(cx + 1, cy),
                    },
                    DrawVert {
                        pos: [x1, y1],
                        uv: WHITE_PIXEL_UV,
                        col: color_at(cx + 1, cy + 1),
                    },
                    DrawVert {
                        pos: [x0, y1],
                        uv: WHITE_PIXEL_UV,
                        col: color_at(cx, cy + 1),
                    },
                ]);
                self.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base,
                    base + 2,
                    base + 3,
                ]);
            }
        }
        self.push_indices(TextureId::WHITE, 6 * cols * rows);
    }

    pub(crate) fn add_sv_gradient(&mut self, min: [f32; 2], max: [f32; 2], hue: f32) {
        self.add_gradient_grid(min, max, SV_GRID_RES, SV_GRID_RES, |cx, cy| {
            crate::color::hsv_to_rgba_u8(
                hue,
                cx as f32 / SV_GRID_RES as f32,
                1.0 - cy as f32 / SV_GRID_RES as f32,
                255,
            )
        });
    }

    pub(crate) fn add_hue_strip(&mut self, min: [f32; 2], max: [f32; 2]) {
        self.add_gradient_grid(min, max, 1, 6, |_cx, cy| {
            HUE_STRIP_BOUNDARY_COLORS[cy as usize]
        });
    }
    pub(crate) fn add_thick_line(
        &mut self,
        p0: [f32; 2],
        p1: [f32; 2],
        col: [u8; 4],
        thickness: f32,
        aa: bool,
    ) {
        self.add_thick_line_impl(p0, p1, col, thickness, aa, false);
    }

    pub(crate) fn add_thick_line_with_caps(
        &mut self,
        p0: [f32; 2],
        p1: [f32; 2],
        col: [u8; 4],
        thickness: f32,
        aa: bool,
    ) {
        self.add_thick_line_impl(p0, p1, col, thickness, aa, true);
    }

    fn add_thick_line_impl(
        &mut self,
        p0: [f32; 2],
        p1: [f32; 2],
        col: [u8; 4],
        thickness: f32,
        aa: bool,
        soften_caps: bool,
    ) {
        if thickness <= 0.0 {
            return;
        }
        let d = [p1[0] - p0[0], p1[1] - p0[1]];
        let len = sqrt_approx(d[0] * d[0] + d[1] * d[1]);
        if len == 0.0 {
            return;
        }
        let half = thickness / 2.0;
        let perp = [-d[1] / len * half, d[0] / len * half];
        self.add_solid_quad(
            [
                [p0[0] + perp[0], p0[1] + perp[1]],
                [p1[0] + perp[0], p1[1] + perp[1]],
                [p1[0] - perp[0], p1[1] - perp[1]],
                [p0[0] - perp[0], p0[1] - perp[1]],
            ],
            col,
        );
        if aa && soften_caps {
            let n = [-d[1] / len, d[0] / len];
            let t = [d[0] / len, d[1] / len];
            let edge = [
                [p0[0] - perp[0], p0[1] - perp[1]],
                [p1[0] - perp[0], p1[1] - perp[1]],
                [p1[0] + perp[0], p1[1] + perp[1]],
                [p0[0] + perp[0], p0[1] + perp[1]],
            ];
            let f = FRINGE_WIDTH;
            let fade_edge = [
                [
                    edge[0][0] - (n[0] + t[0]) * f,
                    edge[0][1] - (n[1] + t[1]) * f,
                ],
                [
                    edge[1][0] - (n[0] - t[0]) * f,
                    edge[1][1] - (n[1] - t[1]) * f,
                ],
                [
                    edge[2][0] + (n[0] + t[0]) * f,
                    edge[2][1] + (n[1] + t[1]) * f,
                ],
                [
                    edge[3][0] + (n[0] - t[0]) * f,
                    edge[3][1] + (n[1] - t[1]) * f,
                ],
            ];
            self.add_quad_fringe_ring(edge, fade_edge, false, col);
        } else if aa {
            let n = [-d[1] / len, d[0] / len];
            self.add_fringe_strip(
                [p0[0] + perp[0], p0[1] + perp[1]],
                [p1[0] + perp[0], p1[1] + perp[1]],
                n,
                n,
                col,
            );
            self.add_fringe_strip(
                [p1[0] - perp[0], p1[1] - perp[1]],
                [p0[0] - perp[0], p0[1] - perp[1]],
                [-n[0], -n[1]],
                [-n[0], -n[1]],
                col,
            );
        }
    }

    pub(crate) fn add_fringe_strip(
        &mut self,
        p0: [f32; 2],
        p1: [f32; 2],
        n0: [f32; 2],
        n1: [f32; 2],
        col: [u8; 4],
    ) {
        self.add_fringe_quad(
            p0,
            p1,
            [p1[0] + n1[0] * FRINGE_WIDTH, p1[1] + n1[1] * FRINGE_WIDTH],
            [p0[0] + n0[0] * FRINGE_WIDTH, p0[1] + n0[1] * FRINGE_WIDTH],
            col,
        );
    }

    fn add_fringe_quad(
        &mut self,
        p0: [f32; 2],
        p1: [f32; 2],
        fade1: [f32; 2],
        fade0: [f32; 2],
        col: [u8; 4],
    ) {
        let fade = [col[0], col[1], col[2], 0];
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&[
            DrawVert {
                pos: p0,
                uv: WHITE_PIXEL_UV,
                col,
            },
            DrawVert {
                pos: p1,
                uv: WHITE_PIXEL_UV,
                col,
            },
            DrawVert {
                pos: fade1,
                uv: WHITE_PIXEL_UV,
                col: fade,
            },
            DrawVert {
                pos: fade0,
                uv: WHITE_PIXEL_UV,
                col: fade,
            },
        ]);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        self.push_indices(TextureId::WHITE, 6);
    }

    pub(crate) fn add_rect_outline(
        &mut self,
        rect: [f32; 4],
        thickness: f32,
        col: [u8; 4],
        aa: bool,
    ) {
        let [x0, y0, x1, y1] = rect;
        self.add_filled_rect([x0, y0], [x1, y0 + thickness], col);
        self.add_filled_rect([x0, y1 - thickness], [x1, y1], col);
        self.add_filled_rect([x0, y0 + thickness], [x0 + thickness, y1 - thickness], col);
        self.add_filled_rect([x1 - thickness, y0 + thickness], [x1, y1 - thickness], col);
        if !aa {
            return;
        }
        self.add_rect_fringe_ring(rect, false, col);
        let inner = [
            x0 + thickness,
            y0 + thickness,
            x1 - thickness,
            y1 - thickness,
        ];
        if inner[0] < inner[2] && inner[1] < inner[3] {
            self.add_rect_fringe_ring(inner, true, col);
        }
    }

    pub(crate) fn add_filled_circle(
        &mut self,
        center: [f32; 2],
        radius: f32,
        col: [u8; 4],
        aa: bool,
    ) {
        if radius <= 0.0 {
            return;
        }
        let n = circle_segment_count(radius);
        let base = self.vertices.len() as u32;
        self.vertices.push(DrawVert {
            pos: center,
            uv: WHITE_PIXEL_UV,
            col,
        });
        let step = core::f32::consts::TAU / n as f32;
        for i in 0..n {
            let (sin, cos) = sin_cos_approx(i as f32 * step);
            self.vertices.push(DrawVert {
                pos: [center[0] + radius * cos, center[1] + radius * sin],
                uv: WHITE_PIXEL_UV,
                col,
            });
        }
        for i in 0..n as u32 {
            let next = if i + 1 == n as u32 { 1 } else { i + 2 };
            self.indices
                .extend_from_slice(&[base, base + 1 + i, base + next]);
        }
        self.push_indices(TextureId::WHITE, 3 * n as u32);
        if aa {
            for i in 0..n {
                let j = (i + 1) % n;
                let (s0, c0) = sin_cos_approx(i as f32 * step);
                let (s1, c1) = sin_cos_approx(j as f32 * step);
                self.add_fringe_strip(
                    [center[0] + radius * c1, center[1] + radius * s1],
                    [center[0] + radius * c0, center[1] + radius * s0],
                    [c1, s1],
                    [c0, s0],
                    col,
                );
            }
        }
    }

    pub(crate) fn add_filled_polygon(&mut self, points: &[[f32; 2]], col: [u8; 4], aa: bool) {
        if points.len() < 3 {
            return;
        }
        let base = self.vertices.len() as u32;
        self.vertices.extend(points.iter().map(|&pos| DrawVert {
            pos,
            uv: WHITE_PIXEL_UV,
            col,
        }));
        for i in 1..points.len() as u32 - 1 {
            self.indices
                .extend_from_slice(&[base, base + i, base + i + 1]);
        }
        self.push_indices(TextureId::WHITE, 3 * (points.len() as u32 - 2));
        if aa {
            self.add_polygon_fringe(points, col);
        }
    }

    fn add_polygon_fringe(&mut self, points: &[[f32; 2]], col: [u8; 4]) {
        if points.len() < 3 {
            return;
        }
        let n = points.len();
        let mut area2 = 0.0_f32;
        for i in 0..n {
            let (p, q) = (points[i], points[(i + 1) % n]);
            area2 += p[0] * q[1] - q[0] * p[1];
        }
        let clockwise = area2 >= 0.0;
        let side = if clockwise { 1.0 } else { -1.0 };
        let edge_normal = |i: usize| {
            let (p, q) = (points[i], points[(i + 1) % n]);
            let d = [q[0] - p[0], q[1] - p[1]];
            let len = sqrt_approx(d[0] * d[0] + d[1] * d[1]);
            if len == 0.0 {
                [0.0, 0.0]
            } else {
                [side * d[1] / len, side * -d[0] / len]
            }
        };
        let corner_normal = |prev: [f32; 2], next: [f32; 2]| {
            let sum = [prev[0] + next[0], prev[1] + next[1]];
            let len = sqrt_approx(sum[0] * sum[0] + sum[1] * sum[1]);
            [sum[0] / len, sum[1] / len]
        };
        let mut prev_normal = edge_normal(n - 1);
        let mut cur_normal = edge_normal(0);
        for i in 0..n {
            let j = (i + 1) % n;
            let next_normal = edge_normal(j);
            if points[i] != points[j] {
                let n_i = corner_normal(prev_normal, cur_normal);
                let n_j = corner_normal(cur_normal, next_normal);
                if clockwise {
                    self.add_fringe_strip(points[j], points[i], n_j, n_i, col);
                } else {
                    self.add_fringe_strip(points[i], points[j], n_i, n_j, col);
                }
            }
            prev_normal = cur_normal;
            cur_normal = next_normal;
        }
    }

    pub(crate) fn add_circle_outline(
        &mut self,
        center: [f32; 2],
        radius: f32,
        col: [u8; 4],
        thickness: f32,
        aa: bool,
    ) {
        self.add_arc(
            center,
            radius,
            0.0,
            core::f32::consts::TAU,
            col,
            thickness,
            aa,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_arc(
        &mut self,
        center: [f32; 2],
        radius: f32,
        start: f32,
        end: f32,
        col: [u8; 4],
        thickness: f32,
        aa: bool,
    ) {
        if radius <= 0.0 || thickness <= 0.0 {
            return;
        }
        let span = end - start;
        if span <= 0.0 {
            return;
        }
        let outer = radius + thickness / 2.0;
        let inner = radius - thickness / 2.0;
        let base = self.vertices.len() as u32;
        let full_count = circle_segment_count(outer);
        if span >= core::f32::consts::TAU {
            let n = full_count;
            for i in 0..n {
                let angle = start + core::f32::consts::TAU * i as f32 / n as f32;
                push_ring_pair(&mut self.vertices, center, outer, inner, angle, col);
            }
            for i in 0..n as u32 {
                let j = (i + 1) % n as u32;
                let (o0, i0) = (base + 2 * i, base + 2 * i + 1);
                let (o1, i1) = (base + 2 * j, base + 2 * j + 1);
                self.indices.extend_from_slice(&[o0, o1, i1, o0, i1, i0]);
            }
            self.push_indices(TextureId::WHITE, 6 * n as u32);
            if aa {
                for i in 0..n {
                    let j = (i + 1) % n;
                    let a0 = start + core::f32::consts::TAU * i as f32 / n as f32;
                    let a1 = start + core::f32::consts::TAU * j as f32 / n as f32;
                    self.add_ring_segment_fringe(
                        center,
                        outer,
                        inner,
                        [a0, a1],
                        col,
                        [false, false],
                    );
                }
            }
        } else {
            let n = ceil_to_usize(full_count as f32 * span / core::f32::consts::TAU).max(1);
            for i in 0..=n {
                let angle = start + span * i as f32 / n as f32;
                push_ring_pair(&mut self.vertices, center, outer, inner, angle, col);
            }
            for i in 0..n as u32 {
                let (o0, i0) = (base + 2 * i, base + 2 * i + 1);
                let (o1, i1) = (base + 2 * i + 2, base + 2 * i + 3);
                self.indices.extend_from_slice(&[o0, o1, i1, o0, i1, i0]);
            }
            self.push_indices(TextureId::WHITE, 6 * n as u32);
            if aa {
                for i in 0..n {
                    let a0 = start + span * i as f32 / n as f32;
                    let a1 = start + span * (i + 1) as f32 / n as f32;
                    self.add_ring_segment_fringe(
                        center,
                        outer,
                        inner,
                        [a0, a1],
                        col,
                        [i == 0, i + 1 == n],
                    );
                }
                self.add_arc_cap_fringe(center, outer, inner, start, col, true);
                self.add_arc_cap_fringe(center, outer, inner, end, col, false);
            }
        }
    }

    fn add_ring_segment_fringe(
        &mut self,
        center: [f32; 2],
        outer: f32,
        inner: f32,
        angles: [f32; 2],
        col: [u8; 4],
        caps: [bool; 2],
    ) {
        let [a0, a1] = angles;
        let [start_cap, end_cap] = caps;
        let (s0, c0) = sin_cos_approx(a0);
        let (s1, c1) = sin_cos_approx(a1);
        let f = FRINGE_WIDTH;
        let (outer0, outer1) = (
            [center[0] + outer * c0, center[1] + outer * s0],
            [center[0] + outer * c1, center[1] + outer * s1],
        );
        let (inner0, inner1) = (
            [center[0] + inner * c0, center[1] + inner * s0],
            [center[0] + inner * c1, center[1] + inner * s1],
        );
        let start_tangent = if start_cap { [-s0, c0] } else { [0.0, 0.0] };
        let end_tangent = if end_cap { [-s1, c1] } else { [0.0, 0.0] };
        self.add_fringe_quad(
            outer1,
            outer0,
            [
                outer0[0] + (c0 - start_tangent[0]) * f,
                outer0[1] + (s0 - start_tangent[1]) * f,
            ],
            [
                outer1[0] + (c1 + end_tangent[0]) * f,
                outer1[1] + (s1 + end_tangent[1]) * f,
            ],
            col,
        );
        self.add_fringe_quad(
            inner0,
            inner1,
            [
                inner1[0] + (-c1 + end_tangent[0]) * f,
                inner1[1] + (-s1 + end_tangent[1]) * f,
            ],
            [
                inner0[0] + (-c0 - start_tangent[0]) * f,
                inner0[1] + (-s0 - start_tangent[1]) * f,
            ],
            col,
        );
    }

    fn add_arc_cap_fringe(
        &mut self,
        center: [f32; 2],
        outer: f32,
        inner: f32,
        angle: f32,
        col: [u8; 4],
        start: bool,
    ) {
        let (s, c) = sin_cos_approx(angle);
        let (radial, tangent) = ([c, s], [-s, c]);
        let outer_pos = [center[0] + outer * c, center[1] + outer * s];
        let inner_pos = [center[0] + inner * c, center[1] + inner * s];
        let direction = if start { -1.0 } else { 1.0 };
        let fade_outer = [
            outer_pos[0] + (radial[0] + direction * tangent[0]) * FRINGE_WIDTH,
            outer_pos[1] + (radial[1] + direction * tangent[1]) * FRINGE_WIDTH,
        ];
        let fade_inner = [
            inner_pos[0] + (-radial[0] + direction * tangent[0]) * FRINGE_WIDTH,
            inner_pos[1] + (-radial[1] + direction * tangent[1]) * FRINGE_WIDTH,
        ];
        if start {
            self.add_fringe_quad(outer_pos, inner_pos, fade_inner, fade_outer, col);
        } else {
            self.add_fringe_quad(inner_pos, outer_pos, fade_outer, fade_inner, col);
        }
    }

    pub(crate) fn add_text(
        &mut self,
        font: &mut Font,
        face: FontId,
        pos: [f32; 2],
        text: &str,
        col: [u8; 4],
    ) {
        let mut pen = pos;
        for ch in text.chars() {
            let (next_pen, quad) = font.glyph_quad(face, ch, pen);
            pen = next_pen;
            if let Some(quad) = quad {
                self.add_textured_quad(
                    TextureId::FONT,
                    quad.min,
                    quad.max,
                    quad.uv_min,
                    quad.uv_max,
                    col,
                );
            }
        }
    }

    pub(crate) fn add_textured_quad(
        &mut self,
        texture: TextureId,
        min: [f32; 2],
        max: [f32; 2],
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        col: [u8; 4],
    ) {
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&[
            DrawVert {
                pos: min,
                uv: uv_min,
                col,
            },
            DrawVert {
                pos: [max[0], min[1]],
                uv: [uv_max[0], uv_min[1]],
                col,
            },
            DrawVert {
                pos: max,
                uv: uv_max,
                col,
            },
            DrawVert {
                pos: [min[0], max[1]],
                uv: [uv_min[0], uv_max[1]],
                col,
            },
        ]);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        self.push_indices(texture, 6);
    }

    pub(crate) fn push_clip_rect(&mut self, rect: [f32; 4]) {
        let current = self.current_clip();
        self.clip_stack.push(intersect_rects(current, rect));
    }

    pub(crate) fn clip_depth(&self) -> usize {
        self.clip_stack.len()
    }

    pub(crate) fn pop_clip_rect(&mut self) {
        assert!(
            self.clip_stack.len() > 1,
            "DrawList::pop_clip_rect without a matching push"
        );
        self.clip_stack.pop();
    }

    pub(crate) fn current_clip(&self) -> [f32; 4] {
        *self
            .clip_stack
            .last()
            .expect("root clip rect is never removed")
    }

    pub(crate) fn replace_base_clip(&mut self, rect: [f32; 4]) {
        debug_assert!(
            self.clip_stack.len() >= 2,
            "DrawList::replace_base_clip requires the per-frame base clip"
        );
        self.clip_stack[1] = rect;
    }

    pub(crate) fn clear(&mut self) {
        self.effects.clear();
        self.current_effects = Default::default();
        self.current_effect_index = None;
        self.vertices.clear();
        self.indices.clear();
        self.commands.clear();
        self.split_command = false;
        self.layer_indices.clear();
        self.layer_commands.clear();
        self.clip_stack.truncate(1);
    }

    fn push_indices(&mut self, texture: TextureId, count: u32) {
        let clip_rect = self.current_clip();
        let effects = self.effect_index();
        match self.commands.last_mut() {
            Some(cmd)
                if !self.split_command
                    && cmd.texture == texture
                    && cmd.clip_rect == clip_rect
                    && cmd.effects == effects =>
            {
                cmd.elem_count += count;
            }
            _ => self.commands.push(DrawCmd {
                effects,
                elem_count: count,
                clip_rect,
                texture,
            }),
        }
        self.split_command = false;
    }

    pub(crate) fn effects_state(&self) -> crate::effects::DrawEffects {
        self.current_effects
    }
    pub(crate) fn set_effects(&mut self, effects: crate::effects::DrawEffects) {
        self.current_effects = effects;
        self.current_effect_index = None;
    }
    fn effect_index(&mut self) -> u32 {
        if let Some(index) = self.current_effect_index {
            return index;
        }
        let index = if self.current_effects.identity() {
            0
        } else {
            match self
                .effects
                .iter()
                .position(|value| *value == self.current_effects)
            {
                Some(index) => index as u32 + 1,
                None => {
                    self.effects.push(self.current_effects);
                    self.effects.len() as u32
                }
            }
        };
        self.current_effect_index = Some(index);
        index
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct PaintRange {
    pub start: [usize; 2],
    pub end: [usize; 2],
    pub layer: i32,
}

pub(crate) const SV_GRID_RES: u32 = 16;

pub(crate) const HUE_STRIP_BOUNDARY_COLORS: [[u8; 4]; 7] = [
    [255, 0, 0, 255],
    [255, 255, 0, 255],
    [0, 255, 0, 255],
    [0, 255, 255, 255],
    [0, 0, 255, 255],
    [255, 0, 255, 255],
    [255, 0, 0, 255],
];

pub(crate) fn intersect_rects(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ]
}

pub(crate) fn sqrt_approx(x: f32) -> f32 {
    if x > 0.0 {
        let guess = f32::from_bits(0x1fbd_1df5 + (x.to_bits() >> 1));
        let y = 0.5 * (guess + x / guess);
        0.5 * (y + x / y)
    } else {
        0.0
    }
}

pub(crate) fn circle_segment_count(radius: f32) -> usize {
    let n = ceil_to_usize(core::f32::consts::PI * sqrt_approx(2.0 * radius));
    n.clamp(48, 512)
}

pub(crate) fn sin_cos_approx(angle: f32) -> (f32, f32) {
    use core::f32::consts::{FRAC_2_PI, FRAC_PI_2, TAU};
    let rem = angle % TAU;
    let t = if rem < 0.0 { rem + TAU } else { rem };
    let quadrant = ((t * FRAC_2_PI) as usize).min(3);
    let r = t - quadrant as f32 * FRAC_PI_2;
    let s = sin_poly(r);
    let c = sin_poly(FRAC_PI_2 - r);
    match quadrant {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

const SIN_POLY: [f32; 4] = [0.999_996_6, -0.166_648_28, 0.008_306_325, -0.000_183_636_54];

fn sin_poly(x: f32) -> f32 {
    let x2 = x * x;
    x * (SIN_POLY[0] + x2 * (SIN_POLY[1] + x2 * (SIN_POLY[2] + x2 * SIN_POLY[3])))
}

fn ceil_to_usize(x: f32) -> usize {
    let truncated = x as usize;
    if x > truncated as f32 {
        truncated + 1
    } else {
        truncated
    }
}

fn push_ring_pair(
    vertices: &mut Vec<DrawVert>,
    center: [f32; 2],
    outer: f32,
    inner: f32,
    angle: f32,
    col: [u8; 4],
) {
    let (sin, cos) = sin_cos_approx(angle);
    vertices.push(DrawVert {
        pos: [center[0] + outer * cos, center[1] + outer * sin],
        uv: WHITE_PIXEL_UV,
        col,
    });
    vertices.push(DrawVert {
        pos: [center[0] + inner * cos, center[1] + inner * sin],
        uv: WHITE_PIXEL_UV,
        col,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_texture_ids_are_zero_and_one() {
        assert_eq!(TextureId::FONT, TextureId(0));
        assert_eq!(TextureId::WHITE, TextureId(1));
    }

    #[test]
    fn host_texture_id_passes_through_to_draw_data() {
        let mut list = DrawList::new();
        list.add_textured_quad(
            TextureId(42),
            [0.0, 0.0],
            [10.0, 10.0],
            [0.0, 0.0],
            [1.0, 1.0],
            [255, 255, 255, 255],
        );

        let data = list.as_draw_data();
        assert_eq!(data.commands.len(), 1);
        assert_eq!(data.commands[0].texture, TextureId(42));
        assert_eq!(data.commands[0].elem_count, 6);
    }

    #[test]
    fn filled_rect_emits_four_verts_six_indices() {
        let mut list = DrawList::new();
        list.add_filled_rect([10.0, 10.0], [20.0, 30.0], [255, 0, 0, 255]);

        assert_eq!(list.vertices.len(), 4);
        assert_eq!(list.indices.len(), 6);

        let positions: Vec<[f32; 2]> = list.vertices.iter().map(|v| v.pos).collect();
        assert_eq!(
            positions,
            alloc::vec![[10.0, 10.0], [20.0, 10.0], [20.0, 30.0], [10.0, 30.0],]
        );
        for v in &list.vertices {
            assert_eq!(v.uv, WHITE_PIXEL_UV);
            assert_eq!(v.col, [255, 0, 0, 255]);
        }
        assert_eq!(list.indices, alloc::vec![0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn nested_clip_pushes_intersect_and_pop_restores() {
        let mut list = DrawList::new();
        let root = list.current_clip();

        list.push_clip_rect([0.0, 0.0, 100.0, 100.0]);
        assert_eq!(list.current_clip(), [0.0, 0.0, 100.0, 100.0]);

        list.push_clip_rect([50.0, 50.0, 200.0, 200.0]);
        assert_eq!(list.current_clip(), [50.0, 50.0, 100.0, 100.0]);

        list.pop_clip_rect();
        assert_eq!(list.current_clip(), [0.0, 0.0, 100.0, 100.0]);

        list.pop_clip_rect();
        assert_eq!(list.current_clip(), root);
    }

    #[test]
    fn clear_retains_capacity_and_resets_contents_and_clip_to_root() {
        let mut list = DrawList::new();
        list.push_clip_rect([10.0, 10.0, 100.0, 100.0]);
        list.push_clip_rect([20.0, 20.0, 50.0, 50.0]);
        list.add_filled_rect([20.0, 20.0], [40.0, 40.0], [255, 0, 0, 255]);
        assert!(!list.vertices.is_empty() && !list.indices.is_empty());
        let (verts, indices, commands, clips) = (
            list.vertices.capacity(),
            list.indices.capacity(),
            list.commands.capacity(),
            list.clip_stack.capacity(),
        );

        list.clear();

        assert!(list.vertices.is_empty());
        assert!(list.indices.is_empty());
        assert!(list.commands.is_empty());
        assert_eq!(list.vertices.capacity(), verts);
        assert_eq!(list.indices.capacity(), indices);
        assert_eq!(list.commands.capacity(), commands);
        assert_eq!(list.clip_stack.capacity(), clips);
        assert_eq!(list.clip_stack.len(), 1);
        assert_eq!(list.current_clip(), UNBOUNDED_CLIP_RECT);

        list.push_clip_rect([5.0, 5.0, 25.0, 25.0]);
        assert_eq!(list.current_clip(), [5.0, 5.0, 25.0, 25.0]);
        list.add_filled_rect([6.0, 6.0], [24.0, 24.0], [0, 255, 0, 255]);
        assert_eq!(list.vertices.len(), 4);
        assert_eq!(list.indices.len(), 6);
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].clip_rect, [5.0, 5.0, 25.0, 25.0]);
    }

    #[test]
    fn identical_texture_and_clip_merge_into_one_command() {
        let mut list = DrawList::new();
        list.add_filled_rect([0.0, 0.0], [10.0, 10.0], [255, 255, 255, 255]);
        list.add_filled_rect([20.0, 20.0], [30.0, 30.0], [0, 0, 0, 255]);

        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].elem_count, 12);
        assert_eq!(list.commands[0].texture, TextureId::WHITE);
        assert_eq!(list.vertices.len(), 8);
        assert_eq!(list.indices.len(), 12);
    }

    #[test]
    fn changed_clip_splits_commands() {
        let mut list = DrawList::new();
        list.add_filled_rect([0.0, 0.0], [10.0, 10.0], [255, 255, 255, 255]);

        list.push_clip_rect([0.0, 0.0, 50.0, 50.0]);
        list.add_filled_rect([20.0, 20.0], [30.0, 30.0], [0, 0, 0, 255]);

        assert_eq!(list.commands.len(), 2);
        assert_eq!(list.commands[0].elem_count, 6);
        assert_eq!(list.commands[1].elem_count, 6);
        assert_eq!(list.commands[1].clip_rect, [0.0, 0.0, 50.0, 50.0]);

        list.pop_clip_rect();
        list.add_filled_rect([40.0, 40.0], [50.0, 50.0], [1, 2, 3, 255]);
        assert_eq!(list.commands.len(), 3);
        assert_eq!(list.commands[2].clip_rect, list.commands[0].clip_rect);
    }

    #[test]
    fn textured_quad_emits_requested_uvs() {
        let mut list = DrawList::new();
        list.add_textured_quad(
            TextureId::FONT,
            [5.0, 6.0],
            [15.0, 26.0],
            [0.25, 0.5],
            [0.5, 0.75],
            [200, 201, 202, 255],
        );

        let uvs: Vec<[f32; 2]> = list.vertices.iter().map(|v| v.uv).collect();
        assert_eq!(
            uvs,
            alloc::vec![[0.25, 0.5], [0.5, 0.5], [0.5, 0.75], [0.25, 0.75],]
        );
        assert_eq!(list.vertices.len(), 4);
        assert_eq!(list.indices.len(), 6);
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].texture, TextureId::FONT);
        assert_eq!(list.commands[0].elem_count, 6);
    }

    #[test]
    fn changed_texture_splits_commands() {
        let mut list = DrawList::new();
        list.add_filled_rect([0.0, 0.0], [10.0, 10.0], [255, 255, 255, 255]);
        list.add_textured_quad(
            TextureId::FONT,
            [0.0, 0.0],
            [10.0, 10.0],
            [0.0, 0.0],
            [0.5, 0.5],
            [255, 255, 255, 255],
        );
        list.add_filled_rect([20.0, 20.0], [30.0, 30.0], [255, 255, 255, 255]);

        assert_eq!(list.commands.len(), 3);
        assert_eq!(list.commands[0].texture, TextureId::WHITE);
        assert_eq!(list.commands[1].texture, TextureId::FONT);
        assert_eq!(list.commands[2].texture, TextureId::WHITE);
        for cmd in &list.commands {
            assert_eq!(cmd.elem_count, 6);
        }
    }

    #[test]
    #[should_panic]
    fn pop_clip_rect_without_matching_push_panics() {
        DrawList::new().pop_clip_rect();
    }

    fn assert_pos_near(actual: [f32; 2], expected: [f32; 2]) {
        let eps = 1e-3;
        assert!(
            (actual[0] - expected[0]).abs() < eps && (actual[1] - expected[1]).abs() < eps,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn sqrt_approx_tracks_std_sqrt() {
        assert_eq!(sqrt_approx(0.0), 0.0);
        assert_eq!(sqrt_approx(-4.0), 0.0);
        assert_eq!(sqrt_approx(f32::NAN), 0.0);
        for &x in &[
            0.25_f32, 1.0, 2.0, 4.0, 25.0, 400.0, 123_456.79, 1.0e6, 1.0e-20, 1.0e30,
        ] {
            let expected = x.sqrt();
            let rel = (sqrt_approx(x) - expected).abs() / expected;
            assert!(
                rel < 1e-5,
                "sqrt_approx({x}) = {}, std::sqrt = {expected}",
                sqrt_approx(x)
            );
        }
    }

    #[test]
    fn thick_horizontal_line_emits_butt_cap_quad() {
        let mut list = DrawList::new();
        list.add_thick_line([10.0, 20.0], [30.0, 20.0], [9, 8, 7, 255], 4.0, false);

        assert_eq!(list.vertices.len(), 4);
        assert_eq!(list.indices, alloc::vec![0, 1, 2, 0, 2, 3]);
        let expected = [[10.0, 22.0], [30.0, 22.0], [30.0, 18.0], [10.0, 18.0]];
        for (v, e) in list.vertices.iter().zip(expected.iter()) {
            assert_pos_near(v.pos, *e);
            assert_eq!(v.uv, WHITE_PIXEL_UV);
            assert_eq!(v.col, [9, 8, 7, 255]);
        }
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].texture, TextureId::WHITE);
        assert_eq!(list.commands[0].elem_count, 6);
    }

    #[test]
    fn thick_vertical_line_emits_butt_cap_quad() {
        let mut list = DrawList::new();
        list.add_thick_line([5.0, 10.0], [5.0, 30.0], [1, 2, 3, 255], 2.0, false);

        let expected = [[4.0, 10.0], [4.0, 30.0], [6.0, 30.0], [6.0, 10.0]];
        assert_eq!(list.vertices.len(), 4);
        for (v, e) in list.vertices.iter().zip(expected.iter()) {
            assert_pos_near(v.pos, *e);
        }
    }

    #[test]
    fn thick_diagonal_line_offsets_along_unit_perpendicular() {
        let mut list = DrawList::new();
        list.add_thick_line([0.0, 0.0], [30.0, 40.0], [255, 255, 255, 255], 10.0, false);

        let expected = [[-4.0, 3.0], [26.0, 43.0], [34.0, 37.0], [4.0, -3.0]];
        assert_eq!(list.vertices.len(), 4);
        for (v, e) in list.vertices.iter().zip(expected.iter()) {
            assert_pos_near(v.pos, *e);
        }
    }

    #[test]
    fn fringe_strip_emits_fade_quad_outside_the_edge() {
        let mut list = DrawList::new();
        let col = [9, 8, 7, 200];
        list.add_fringe_strip([10.0, 20.0], [30.0, 20.0], [0.0, 1.0], [0.0, 1.0], col);

        assert_eq!(list.vertices.len(), 4);
        assert_eq!(list.indices, alloc::vec![0, 1, 2, 0, 2, 3]);
        let f = FRINGE_WIDTH;
        let expected_pos = [
            [10.0, 20.0],
            [30.0, 20.0],
            [30.0, 20.0 + f],
            [10.0, 20.0 + f],
        ];
        let expected_col = [col, col, [9, 8, 7, 0], [9, 8, 7, 0]];
        for ((v, p), c) in list
            .vertices
            .iter()
            .zip(expected_pos.iter())
            .zip(expected_col.iter())
        {
            assert_eq!(v.pos, *p);
            assert_eq!(v.col, *c);
            assert_eq!(v.uv, WHITE_PIXEL_UV);
        }
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].texture, TextureId::WHITE);
        assert_eq!(list.commands[0].elem_count, 6);
    }

    #[test]
    fn fringe_strip_offsets_along_per_endpoint_normals() {
        let mut list = DrawList::new();
        list.add_fringe_strip(
            [0.0, 0.0],
            [10.0, 0.0],
            [0.0, 1.0],
            [3.0, 4.0],
            [1, 2, 3, 255],
        );

        assert_pos_near(
            list.vertices[2].pos,
            [10.0 + 3.0 * FRINGE_WIDTH, 4.0 * FRINGE_WIDTH],
        );
        assert_pos_near(list.vertices[3].pos, [0.0, FRINGE_WIDTH]);
        assert_eq!(list.vertices[2].col, [1, 2, 3, 0]);
        assert_eq!(list.vertices[3].col, [1, 2, 3, 0]);
    }

    #[test]
    fn fringe_strip_merges_into_the_body_command() {
        let mut list = DrawList::new();
        list.add_solid_quad(
            [[10.0, 20.0], [30.0, 20.0], [30.0, 24.0], [10.0, 24.0]],
            [5, 6, 7, 255],
        );
        list.add_fringe_strip(
            [10.0, 20.0],
            [30.0, 20.0],
            [0.0, 1.0],
            [0.0, 1.0],
            [5, 6, 7, 255],
        );

        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].elem_count, 12);
    }

    #[test]
    fn fringe_strip_winds_positive_like_the_quad_convention() {
        let mut list = DrawList::new();
        list.add_fringe_strip([10.0, 20.0], [30.0, 20.0], [0.0, 1.0], [0.0, 1.0], [255; 4]);

        for tri in list.indices.chunks_exact(3) {
            let area = cross2(
                list.vertices[tri[0] as usize].pos,
                list.vertices[tri[1] as usize].pos,
                list.vertices[tri[2] as usize].pos,
            );
            assert!(area > 0.0, "fringe triangle {tri:?} wound opposite");
        }
    }

    #[test]
    fn degenerate_thick_lines_emit_nothing() {
        let mut list = DrawList::new();
        list.add_thick_line([10.0, 10.0], [10.0, 10.0], [255, 0, 0, 255], 4.0, true);
        list.add_thick_line([0.0, 0.0], [10.0, 10.0], [255, 0, 0, 255], 0.0, true);
        list.add_thick_line([0.0, 0.0], [10.0, 10.0], [255, 0, 0, 255], -2.0, true);

        assert!(list.vertices.is_empty());
        assert!(list.indices.is_empty());
        assert!(list.commands.is_empty());
    }

    #[test]
    fn sin_cos_approx_tracks_std_across_quadrants() {
        let mut max_err = 0.0_f32;
        for i in -6284..=12_566 {
            let angle = i as f32 * 0.001;
            let (s, c) = sin_cos_approx(angle);
            let err = (s - angle.sin()).abs().max((c - angle.cos()).abs());
            max_err = max_err.max(err);
            assert!(
                err < 2e-6,
                "sin_cos_approx({angle}) = ({s}, {c}), std = ({}, {})",
                angle.sin(),
                angle.cos()
            );
        }
        assert!(
            max_err < 1.5e-6,
            "measured max error {max_err} exceeds the expected ~1.3e-6"
        );
    }

    #[test]
    fn sin_cos_approx_large_angle_error_grows_linearly() {
        for &angle in &[100.0_f32, -250.0, 1000.0, -9999.0, 10_000.0] {
            let (s, c) = sin_cos_approx(angle);
            let eps = 2e-6 + angle.abs() * 5e-8;
            let err = (s - angle.sin()).abs().max((c - angle.cos()).abs());
            assert!(
                err < eps,
                "sin_cos_approx({angle}) error {err} exceeds {eps}"
            );
        }
    }

    #[test]
    fn sin_cos_approx_special_values() {
        let (s, c) = sin_cos_approx(0.0);
        assert_eq!(s, 0.0, "sin(0) is exact (odd polynomial at 0)");
        assert!((c - 1.0).abs() < 1e-6);
        assert!(sin_cos_approx(f32::NAN).0.is_nan());
        assert!(sin_cos_approx(f32::INFINITY).1.is_nan());
    }

    #[test]
    fn circle_segment_count_is_stable_monotone_and_bounded() {
        assert_eq!(circle_segment_count(50.0), circle_segment_count(50.0));
        let mut prev = circle_segment_count(1.0);
        for r in 2..500 {
            let n = circle_segment_count(r as f32);
            assert!(n >= prev, "count({r}) = {n} < count({}) = {prev}", r - 1);
            prev = n;
        }
        assert_eq!(circle_segment_count(0.5), 48);
        assert!(circle_segment_count(100.0) >= 48);
        assert_eq!(circle_segment_count(1.0e6), 512);
        assert!(circle_segment_count(13_000.0) < 512);
        assert_eq!(circle_segment_count(13_229.0), 512);
    }

    fn cross2(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    }

    #[test]
    fn filled_circle_emits_consistently_wound_fan() {
        let center = [100.0, 100.0];
        let radius = 50.0;
        let n = circle_segment_count(radius);
        let mut list = DrawList::new();
        list.add_filled_circle(center, radius, [7, 8, 9, 255], false);

        assert_eq!(list.vertices.len(), n + 1);
        assert_eq!(list.indices.len(), 3 * n);
        assert_eq!(list.vertices[0].pos, center);
        for i in 0..n {
            let wrap = if i + 1 == n { 1 } else { i + 2 };
            assert_eq!(
                list.indices[3 * i..3 * i + 3],
                [0, 1 + i as u32, wrap as u32]
            );
        }

        assert_pos_near(list.vertices[1].pos, [center[0] + radius, center[1]]);
        for v in &list.vertices[1..] {
            let d = [v.pos[0] - center[0], v.pos[1] - center[1]];
            let dist = (d[0] * d[0] + d[1] * d[1]).sqrt();
            assert!(
                (dist - radius).abs() < 1e-3,
                "vertex {:?} off circle",
                v.pos
            );
            assert_eq!(v.uv, WHITE_PIXEL_UV);
            assert_eq!(v.col, [7, 8, 9, 255]);
        }
        for i in 0..n {
            let (ia, ib, ic) = (
                list.indices[3 * i] as usize,
                list.indices[3 * i + 1] as usize,
                list.indices[3 * i + 2] as usize,
            );
            assert!(ia == 0);
            let area = cross2(
                list.vertices[ia].pos,
                list.vertices[ib].pos,
                list.vertices[ic].pos,
            );
            assert!(area > 0.0, "fan triangle {i} wound opposite");
        }
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].elem_count, 3 * n as u32);
        assert_eq!(list.commands[0].texture, TextureId::WHITE);
    }

    #[test]
    fn filled_polygon_emits_consistently_wound_fan() {
        let mut list = DrawList::new();
        let points = [[10.0, 10.0], [30.0, 10.0], [40.0, 30.0], [20.0, 40.0]];
        list.add_filled_polygon(&points, [5, 6, 7, 255], false);

        assert_eq!(list.vertices.len(), 4);
        for (v, p) in list.vertices.iter().zip(points.iter()) {
            assert_eq!(v.pos, *p);
            assert_eq!(v.uv, WHITE_PIXEL_UV);
            assert_eq!(v.col, [5, 6, 7, 255]);
        }
        assert_eq!(list.indices, alloc::vec![0, 1, 2, 0, 2, 3]);
        for tri in list.indices.chunks_exact(3) {
            assert_eq!(tri[0], 0);
            let area = cross2(
                list.vertices[tri[0] as usize].pos,
                list.vertices[tri[1] as usize].pos,
                list.vertices[tri[2] as usize].pos,
            );
            assert!(area > 0.0, "fan triangle {tri:?} wound opposite");
        }
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].texture, TextureId::WHITE);
        assert_eq!(list.commands[0].elem_count, 6);
    }

    #[test]
    fn filled_polygon_triangle_emits_one_triangle() {
        let mut list = DrawList::new();
        list.add_filled_polygon(&[[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]], [255; 4], false);

        assert_eq!(list.vertices.len(), 3);
        assert_eq!(list.indices, alloc::vec![0, 1, 2]);
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].elem_count, 3);
    }

    #[test]
    fn degenerate_polygons_emit_nothing() {
        let mut list = DrawList::new();
        list.add_filled_polygon(&[], [255; 4], true);
        list.add_filled_polygon(&[[1.0, 1.0]], [255; 4], true);
        list.add_filled_polygon(&[[1.0, 1.0], [2.0, 2.0]], [255; 4], true);

        assert!(list.vertices.is_empty());
        assert!(list.indices.is_empty());
        assert!(list.commands.is_empty());
    }

    #[test]
    fn arc_emits_only_the_requested_range() {
        let center = [100.0, 100.0];
        let radius = 50.0;
        let thickness = 4.0;
        let (ro, ri) = (radius + thickness / 2.0, radius - thickness / 2.0);
        let span = core::f32::consts::FRAC_PI_2;
        let n = ((circle_segment_count(radius + thickness / 2.0) as f32 * span
            / core::f32::consts::TAU)
            .ceil() as usize)
            .max(1);
        let mut list = DrawList::new();
        list.add_arc(center, radius, 0.0, span, [1, 2, 3, 255], thickness, false);

        assert_eq!(list.vertices.len(), 2 * (n + 1));
        assert_eq!(list.indices.len(), 6 * n);
        for i in 0..=n {
            let angle = span * i as f32 / n as f32;
            let (s, c) = (angle.sin(), angle.cos());
            assert_pos_near(
                list.vertices[2 * i].pos,
                [center[0] + ro * c, center[1] + ro * s],
            );
            assert_pos_near(
                list.vertices[2 * i + 1].pos,
                [center[0] + ri * c, center[1] + ri * s],
            );
        }
        assert_pos_near(list.vertices[0].pos, [center[0] + ro, center[1]]);
        assert_pos_near(list.vertices[2 * n].pos, [center[0], center[1] + ro]);
    }

    #[test]
    fn ring_outline_tessellates_from_the_outer_radius() {
        let (center, radius, thickness) = ([100.0, 100.0], 200.0, 8.0);
        let n_outer = circle_segment_count(radius + thickness / 2.0);
        let n_center = circle_segment_count(radius);
        assert!(
            n_outer > n_center,
            "fixture radii must yield different counts: {n_outer} vs {n_center}"
        );

        let mut list = DrawList::new();
        list.add_circle_outline(center, radius, [1, 2, 3, 255], thickness, false);
        assert_eq!(list.vertices.len(), 2 * n_outer);
        assert_eq!(list.indices.len(), 6 * n_outer);

        let span = core::f32::consts::FRAC_PI_2;
        let n = ((n_outer as f32 * span / core::f32::consts::TAU).ceil() as usize).max(1);
        let mut list = DrawList::new();
        list.add_arc(center, radius, 0.0, span, [1, 2, 3, 255], thickness, false);
        assert_eq!(list.vertices.len(), 2 * (n + 1));
        assert_eq!(list.indices.len(), 6 * n);
    }

    #[test]
    fn full_range_arc_equals_circle_outline() {
        let (center, radius, thickness) = ([100.0, 100.0], 50.0, 4.0);
        for aa in [false, true] {
            let mut outline = DrawList::new();
            outline.add_circle_outline(center, radius, [9, 9, 9, 255], thickness, aa);

            for span in [core::f32::consts::TAU, 3.0 * core::f32::consts::PI] {
                let mut arc = DrawList::new();
                arc.add_arc(center, radius, 0.0, span, [9, 9, 9, 255], thickness, aa);
                assert_eq!(
                    outline.vertices, arc.vertices,
                    "span {span} (aa={aa}) differs from circle outline"
                );
                assert_eq!(outline.indices, arc.indices);
            }
        }
    }

    #[test]
    fn degenerate_circles_and_arcs_emit_nothing() {
        let mut list = DrawList::new();
        list.add_filled_circle([0.0, 0.0], 0.0, [255, 0, 0, 255], true);
        list.add_filled_circle([0.0, 0.0], -10.0, [255, 0, 0, 255], true);
        list.add_circle_outline([0.0, 0.0], 0.0, [255, 0, 0, 255], 2.0, true);
        list.add_arc([0.0, 0.0], -5.0, 0.0, 1.0, [255, 0, 0, 255], 2.0, true);
        list.add_circle_outline([0.0, 0.0], 10.0, [255, 0, 0, 255], 0.0, true);
        list.add_arc([0.0, 0.0], 10.0, 0.0, 1.0, [255, 0, 0, 255], -1.0, true);
        list.add_arc([0.0, 0.0], 10.0, 1.0, 1.0, [255, 0, 0, 255], 2.0, true);

        assert!(list.vertices.is_empty());
        assert!(list.indices.is_empty());
        assert!(list.commands.is_empty());
    }

    #[test]
    fn aa_thick_line_softens_sides_and_both_caps_without_corner_gaps() {
        let mut list = DrawList::new();
        let col = [9, 8, 7, 255];
        list.add_thick_line_with_caps([10.0, 20.0], [30.0, 20.0], col, 4.0, true);

        assert_eq!(list.vertices.len(), 4 + 4 * 4);
        assert_eq!(list.indices.len(), 6 + 4 * 6);
        let expected_body = [[10.0, 22.0], [30.0, 22.0], [30.0, 18.0], [10.0, 18.0]];
        for (v, e) in list.vertices[..4].iter().zip(expected_body.iter()) {
            assert_pos_near(v.pos, *e);
            assert_eq!(v.col, col);
        }
        let fade = [9, 8, 7, 0];
        let f = FRINGE_WIDTH;
        let edge = [[10.0, 18.0], [30.0, 18.0], [30.0, 22.0], [10.0, 22.0]];
        let transparent = [
            [10.0 - f, 18.0 - f],
            [30.0 + f, 18.0 - f],
            [30.0 + f, 22.0 + f],
            [10.0 - f, 22.0 + f],
        ];
        for i in 0..4 {
            let j = (i + 1) % 4;
            let quad = &list.vertices[4 + i * 4..][..4];
            assert_eq!((quad[0].pos, quad[0].col), (edge[i], col));
            assert_eq!((quad[1].pos, quad[1].col), (transparent[i], fade));
            assert_eq!((quad[2].pos, quad[2].col), (transparent[j], fade));
            assert_eq!((quad[3].pos, quad[3].col), (edge[j], col));
            assert!(quad.iter().all(|v| v.uv == WHITE_PIXEL_UV));
        }
        for tri in list.indices[6..].chunks_exact(3) {
            let area = cross2(
                list.vertices[tri[0] as usize].pos,
                list.vertices[tri[1] as usize].pos,
                list.vertices[tri[2] as usize].pos,
            );
            assert!(area > 0.0, "fringe triangle {tri:?} wound opposite");
        }
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].elem_count, 30);
    }

    #[test]
    fn aa_thick_line_keeps_the_body_byte_identical_to_crisp() {
        let (p0, p1, col, thickness) = ([3.0, 7.0], [40.0, 25.0], [1, 2, 3, 200], 3.0);
        let mut crisp = DrawList::new();
        crisp.add_thick_line(p0, p1, col, thickness, false);
        let mut aa = DrawList::new();
        aa.add_thick_line_with_caps(p0, p1, col, thickness, true);

        assert_eq!(crisp.vertices.len(), 4);
        assert_eq!(crisp.indices.len(), 6);
        assert_eq!(&aa.vertices[..4], &crisp.vertices[..]);
        assert_eq!(&aa.indices[..6], &crisp.indices[..]);
    }

    #[test]
    fn aa_circle_outline_appends_fringe_rings_outside_both_edges() {
        let (center, radius, thickness) = ([100.0, 100.0], 50.0, 4.0);
        let (ro, ri) = (radius + thickness / 2.0, radius - thickness / 2.0);
        let col = [5, 6, 7, 255];
        let fade = [5, 6, 7, 0];
        let n = circle_segment_count(ro);

        let mut crisp = DrawList::new();
        crisp.add_circle_outline(center, radius, col, thickness, false);
        let mut list = DrawList::new();
        list.add_circle_outline(center, radius, col, thickness, true);

        assert_eq!(crisp.vertices.len(), 2 * n);
        assert_eq!(list.vertices.len(), 2 * n + 8 * n);
        assert_eq!(list.indices.len(), 6 * n + 12 * n);
        assert_eq!(&list.vertices[..2 * n], &crisp.vertices[..]);
        assert_eq!(&list.indices[..6 * n], &crisp.indices[..]);

        let dist = |p: [f32; 2]| {
            let d = [p[0] - center[0], p[1] - center[1]];
            (d[0] * d[0] + d[1] * d[1]).sqrt()
        };
        for i in 0..n {
            let q = 2 * n + 8 * i;
            let (o_i, o_j) = (2 * i, 2 * ((i + 1) % n));
            assert_eq!(list.vertices[q].pos, list.vertices[o_j].pos);
            assert_eq!(list.vertices[q + 1].pos, list.vertices[o_i].pos);
            assert!((dist(list.vertices[q + 2].pos) - (ro + FRINGE_WIDTH)).abs() < 1e-3);
            assert!((dist(list.vertices[q + 3].pos) - (ro + FRINGE_WIDTH)).abs() < 1e-3);
            assert_eq!(list.vertices[q].col, col);
            assert_eq!(list.vertices[q + 1].col, col);
            assert_eq!(list.vertices[q + 2].col, fade);
            assert_eq!(list.vertices[q + 3].col, fade);
            assert_eq!(list.vertices[q + 4].pos, list.vertices[o_i + 1].pos);
            assert_eq!(list.vertices[q + 5].pos, list.vertices[o_j + 1].pos);
            assert!((dist(list.vertices[q + 6].pos) - (ri - FRINGE_WIDTH)).abs() < 1e-3);
            assert!((dist(list.vertices[q + 7].pos) - (ri - FRINGE_WIDTH)).abs() < 1e-3);
            assert_eq!(list.vertices[q + 4].col, col);
            assert_eq!(list.vertices[q + 5].col, col);
            assert_eq!(list.vertices[q + 6].col, fade);
            assert_eq!(list.vertices[q + 7].col, fade);
        }
        for tri in list.indices[6 * n..].chunks_exact(3) {
            let area = cross2(
                list.vertices[tri[0] as usize].pos,
                list.vertices[tri[1] as usize].pos,
                list.vertices[tri[2] as usize].pos,
            );
            assert!(area > 0.0, "fringe triangle {tri:?} wound opposite");
        }
    }

    #[test]
    fn aa_partial_arc_fringe_follows_the_span_exactly() {
        let (center, radius, thickness) = ([100.0, 100.0], 50.0, 4.0);
        let (ro, ri) = (radius + thickness / 2.0, radius - thickness / 2.0);
        let col = [1, 2, 3, 255];
        let span = core::f32::consts::FRAC_PI_2;
        let n =
            ceil_to_usize(circle_segment_count(ro) as f32 * span / core::f32::consts::TAU).max(1);

        let mut list = DrawList::new();
        list.add_arc(center, radius, 0.0, span, col, thickness, true);

        assert_eq!(list.vertices.len(), 2 * (n + 1) + 8 * n + 8);
        assert_eq!(list.indices.len(), 6 * n + 12 * n + 12);

        let base = 2 * (n + 1);
        assert_eq!(list.vertices[base + 1].pos, list.vertices[0].pos);
        assert_pos_near(list.vertices[0].pos, [center[0] + ro, center[1]]);
        let last = base + 8 * (n - 1);
        assert_eq!(list.vertices[last].pos, list.vertices[2 * n].pos);
        assert_pos_near(list.vertices[2 * n].pos, [center[0], center[1] + ro]);
        assert_eq!(list.vertices[base + 4].pos, list.vertices[1].pos);
        assert_pos_near(list.vertices[1].pos, [center[0] + ri, center[1]]);
        assert_eq!(list.vertices[last + 5].pos, list.vertices[2 * n + 1].pos);

        let start_cap = base + 8 * n;
        let end_cap = start_cap + 4;
        assert_eq!(list.vertices[start_cap].pos, list.vertices[0].pos);
        assert_eq!(list.vertices[start_cap + 1].pos, list.vertices[1].pos);
        assert_eq!(
            list.vertices[start_cap + 3].pos,
            list.vertices[base + 2].pos
        );
        assert_eq!(
            list.vertices[start_cap + 2].pos,
            list.vertices[base + 7].pos
        );
        assert_eq!(list.vertices[end_cap].pos, list.vertices[2 * n + 1].pos);
        assert_eq!(list.vertices[end_cap + 1].pos, list.vertices[2 * n].pos);
        assert_eq!(list.vertices[end_cap + 2].pos, list.vertices[last + 3].pos);
        assert_eq!(list.vertices[end_cap + 3].pos, list.vertices[last + 6].pos);
        for tri in list.indices[6 * n..].chunks_exact(3) {
            let area = cross2(
                list.vertices[tri[0] as usize].pos,
                list.vertices[tri[1] as usize].pos,
                list.vertices[tri[2] as usize].pos,
            );
            assert!(area > 0.0, "fringe triangle {tri:?} wound opposite");
        }

        let mut crisp = DrawList::new();
        crisp.add_arc(center, radius, 0.0, span, col, thickness, false);
        assert_eq!(&list.vertices[..2 * (n + 1)], &crisp.vertices[..]);
        assert_eq!(&list.indices[..6 * n], &crisp.indices[..]);
    }

    #[test]
    fn aa_single_segment_arc_joins_both_caps_at_nonzero_angles() {
        let center = [40.0, 50.0];
        let start = 0.3;
        let end = 0.42;
        let col = [90, 180, 220, 255];
        let mut list = DrawList::new();
        list.add_arc(center, 24.0, start, end, col, 4.0, true);

        assert_eq!(list.vertices.len(), 4 + 8 + 8);
        assert_eq!(list.indices.len(), 6 + 12 + 12);
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].elem_count, 30);
        let (start_cap, end_cap) = (12, 16);
        assert_eq!(list.vertices[start_cap + 3].pos, list.vertices[6].pos);
        assert_eq!(list.vertices[start_cap + 2].pos, list.vertices[11].pos);
        assert_eq!(list.vertices[end_cap + 2].pos, list.vertices[7].pos);
        assert_eq!(list.vertices[end_cap + 3].pos, list.vertices[10].pos);
        for tri in list.indices.chunks_exact(3) {
            assert!(
                cross2(
                    list.vertices[tri[0] as usize].pos,
                    list.vertices[tri[1] as usize].pos,
                    list.vertices[tri[2] as usize].pos,
                ) > 0.0
            );
        }
    }

    #[test]
    fn aa_rect_outline_appends_fringe_on_all_perimeter_edges() {
        let col = [7, 7, 7, 255];
        let fade = [7, 7, 7, 0];
        let mut list = DrawList::new();
        list.add_rect_outline([10.0, 10.0, 30.0, 26.0], 2.0, col, true);

        assert_eq!(list.vertices.len(), 16 + 8 * 4);
        assert_eq!(list.indices.len(), 4 * 6 + 8 * 6);
        let f = FRINGE_WIDTH;
        for (ring, edge, transparent) in [
            (
                0,
                [[10.0, 10.0], [30.0, 10.0], [30.0, 26.0], [10.0, 26.0]],
                [
                    [10.0 - f, 10.0 - f],
                    [30.0 + f, 10.0 - f],
                    [30.0 + f, 26.0 + f],
                    [10.0 - f, 26.0 + f],
                ],
            ),
            (
                1,
                [[12.0, 12.0], [28.0, 12.0], [28.0, 24.0], [12.0, 24.0]],
                [
                    [12.0 + f, 12.0 + f],
                    [28.0 - f, 12.0 + f],
                    [28.0 - f, 24.0 - f],
                    [12.0 + f, 24.0 - f],
                ],
            ),
        ] {
            for i in 0..4 {
                let j = (i + 1) % 4;
                let quad = &list.vertices[16 + ring * 16 + i * 4..][..4];
                if ring == 0 {
                    assert_eq!((quad[0].pos, quad[0].col), (edge[i], col));
                    assert_eq!((quad[1].pos, quad[1].col), (transparent[i], fade));
                    assert_eq!((quad[2].pos, quad[2].col), (transparent[j], fade));
                    assert_eq!((quad[3].pos, quad[3].col), (edge[j], col));
                } else {
                    assert_eq!((quad[0].pos, quad[0].col), (edge[i], col));
                    assert_eq!((quad[1].pos, quad[1].col), (edge[j], col));
                    assert_eq!((quad[2].pos, quad[2].col), (transparent[j], fade));
                    assert_eq!((quad[3].pos, quad[3].col), (transparent[i], fade));
                }
                assert!(quad.iter().all(|v| v.uv == WHITE_PIXEL_UV));
            }
        }
        for tri in list.indices[24..].chunks_exact(3) {
            let area = cross2(
                list.vertices[tri[0] as usize].pos,
                list.vertices[tri[1] as usize].pos,
                list.vertices[tri[2] as usize].pos,
            );
            assert!(area > 0.0, "fringe triangle {tri:?} wound opposite");
        }
    }

    #[test]
    fn aa_off_rect_outline_is_byte_identical_to_the_crisp_bars() {
        let rect = [10.0, 10.0, 30.0, 26.0];
        let (col, thickness) = ([7, 7, 7, 255], 2.0);
        let mut reference = DrawList::new();
        let [x0, y0, x1, y1] = rect;
        reference.add_filled_rect([x0, y0], [x1, y0 + thickness], col);
        reference.add_filled_rect([x0, y1 - thickness], [x1, y1], col);
        reference.add_filled_rect([x0, y0 + thickness], [x0 + thickness, y1 - thickness], col);
        reference.add_filled_rect([x1 - thickness, y0 + thickness], [x1, y1 - thickness], col);

        let mut off = DrawList::new();
        off.add_rect_outline(rect, thickness, col, false);
        assert_eq!(off.vertices, reference.vertices);
        assert_eq!(off.indices, reference.indices);

        let mut on = DrawList::new();
        on.add_rect_outline(rect, thickness, col, true);
        assert_eq!(&on.vertices[..16], &reference.vertices[..]);
        assert_eq!(&on.indices[..24], &reference.indices[..]);
    }

    #[test]
    fn aa_filled_circle_appends_an_outer_fringe_ring() {
        let (center, radius) = ([100.0, 100.0], 50.0);
        let col = [7, 8, 9, 255];
        let fade = [7, 8, 9, 0];
        let n = circle_segment_count(radius);

        let mut crisp = DrawList::new();
        crisp.add_filled_circle(center, radius, col, false);
        let mut list = DrawList::new();
        list.add_filled_circle(center, radius, col, true);

        assert_eq!(crisp.vertices.len(), n + 1);
        assert_eq!(crisp.indices.len(), 3 * n);
        assert_eq!(list.vertices.len(), n + 1 + 4 * n);
        assert_eq!(list.indices.len(), 3 * n + 6 * n);
        assert_eq!(&list.vertices[..n + 1], &crisp.vertices[..]);
        assert_eq!(&list.indices[..3 * n], &crisp.indices[..]);

        let dist = |p: [f32; 2]| {
            let d = [p[0] - center[0], p[1] - center[1]];
            (d[0] * d[0] + d[1] * d[1]).sqrt()
        };
        for i in 0..n {
            let q = n + 1 + 4 * i;
            let j = (i + 1) % n;
            assert_eq!(list.vertices[q].pos, list.vertices[1 + j].pos);
            assert_eq!(list.vertices[q + 1].pos, list.vertices[1 + i].pos);
            assert!((dist(list.vertices[q + 2].pos) - (radius + FRINGE_WIDTH)).abs() < 1e-3);
            assert!((dist(list.vertices[q + 3].pos) - (radius + FRINGE_WIDTH)).abs() < 1e-3);
            assert_eq!(list.vertices[q].col, col);
            assert_eq!(list.vertices[q + 1].col, col);
            assert_eq!(list.vertices[q + 2].col, fade);
            assert_eq!(list.vertices[q + 3].col, fade);
        }
        for tri in list.indices[3 * n..].chunks_exact(3) {
            let area = cross2(
                list.vertices[tri[0] as usize].pos,
                list.vertices[tri[1] as usize].pos,
                list.vertices[tri[2] as usize].pos,
            );
            assert!(area > 0.0, "fringe triangle {tri:?} wound opposite");
        }
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].elem_count, 9 * n as u32);
    }

    #[test]
    fn aa_filled_polygon_fringes_each_edge_with_averaged_corner_normals() {
        let points = [[10.0, 10.0], [30.0, 10.0], [30.0, 30.0], [10.0, 30.0]];
        let col = [5, 6, 7, 255];
        let fade = [5, 6, 7, 0];

        let mut crisp = DrawList::new();
        crisp.add_filled_polygon(&points, col, false);
        let mut list = DrawList::new();
        list.add_filled_polygon(&points, col, true);

        assert_eq!(list.vertices.len(), 4 + 4 * 4);
        assert_eq!(list.indices.len(), 6 + 4 * 6);
        assert_eq!(&list.vertices[..4], &crisp.vertices[..]);
        assert_eq!(&list.indices[..6], &crisp.indices[..]);

        let h = FRINGE_WIDTH * core::f32::consts::FRAC_1_SQRT_2;
        let expected_fringe = [
            ([30.0, 10.0], col),
            ([10.0, 10.0], col),
            ([10.0 - h, 10.0 - h], fade),
            ([30.0 + h, 10.0 - h], fade),
            ([30.0, 30.0], col),
            ([30.0, 10.0], col),
            ([30.0 + h, 10.0 - h], fade),
            ([30.0 + h, 30.0 + h], fade),
            ([10.0, 30.0], col),
            ([30.0, 30.0], col),
            ([30.0 + h, 30.0 + h], fade),
            ([10.0 - h, 30.0 + h], fade),
            ([10.0, 10.0], col),
            ([10.0, 30.0], col),
            ([10.0 - h, 30.0 + h], fade),
            ([10.0 - h, 10.0 - h], fade),
        ];
        for (v, (p, c)) in list.vertices[4..].iter().zip(expected_fringe.iter()) {
            assert_pos_near(v.pos, *p);
            assert_eq!(v.col, *c);
            assert_eq!(v.uv, WHITE_PIXEL_UV);
        }
        for i in 0..4 {
            let q = 4 + 4 * i;
            assert_eq!(list.vertices[q].pos, points[(i + 1) % 4]);
            assert_eq!(list.vertices[q + 1].pos, points[i]);
        }
        for tri in list.indices[6..].chunks_exact(3) {
            let area = cross2(
                list.vertices[tri[0] as usize].pos,
                list.vertices[tri[1] as usize].pos,
                list.vertices[tri[2] as usize].pos,
            );
            assert!(area > 0.0, "fringe triangle {tri:?} wound opposite");
        }
    }

    #[test]
    fn aa_filled_polygon_fringe_faces_outward_for_both_windings() {
        let cw = [[10.0, 10.0], [30.0, 10.0], [30.0, 30.0], [10.0, 30.0]];
        let ccw = [[10.0, 10.0], [10.0, 30.0], [30.0, 30.0], [30.0, 10.0]];
        for points in [cw, ccw] {
            let mut list = DrawList::new();
            list.add_filled_polygon(&points, [1, 2, 3, 255], true);
            assert_eq!(list.vertices.len(), 4 + 16);
            let mut sides = [0usize; 4];
            for quad in list.vertices[4..].chunks_exact(4) {
                let (o0, o1) = (quad[2].pos, quad[3].pos);
                if o0[1] < 10.0 && o1[1] < 10.0 {
                    sides[0] += 1;
                } else if o0[1] > 30.0 && o1[1] > 30.0 {
                    sides[1] += 1;
                } else if o0[0] < 10.0 && o1[0] < 10.0 {
                    sides[2] += 1;
                } else if o0[0] > 30.0 && o1[0] > 30.0 {
                    sides[3] += 1;
                } else {
                    panic!("fringe quad {quad:?} does not fade outward");
                }
            }
            assert_eq!(sides, [1, 1, 1, 1], "points {points:?}");
        }
    }

    #[test]
    fn aa_filled_polygon_skips_zero_length_edges_without_nan() {
        let points = [
            [10.0, 10.0],
            [30.0, 10.0],
            [30.0, 10.0],
            [30.0, 30.0],
            [10.0, 30.0],
        ];
        let mut list = DrawList::new();
        list.add_filled_polygon(&points, [1, 2, 3, 255], true);

        assert_eq!(list.vertices.len(), 5 + 4 * 4);
        for v in &list.vertices {
            assert!(!v.pos[0].is_nan() && !v.pos[1].is_nan(), "NaN vertex {v:?}");
        }
        let mut sides = [0usize; 4];
        for quad in list.vertices[5..].chunks_exact(4) {
            let (o0, o1) = (quad[2].pos, quad[3].pos);
            if o0[1] < 10.0 && o1[1] < 10.0 {
                sides[0] += 1;
            } else if o0[1] > 30.0 && o1[1] > 30.0 {
                sides[1] += 1;
            } else if o0[0] < 10.0 && o1[0] < 10.0 {
                sides[2] += 1;
            } else if o0[0] > 30.0 && o1[0] > 30.0 {
                sides[3] += 1;
            } else {
                panic!("fringe quad {quad:?} does not fade outward");
            }
        }
        assert_eq!(sides, [1, 1, 1, 1]);
    }

    #[test]
    fn aa_off_fills_are_byte_identical_and_on_appends_after_the_body() {
        let (center, radius) = ([100.0, 100.0], 50.0);
        let points = [[10.0, 10.0], [30.0, 10.0], [40.0, 30.0], [20.0, 40.0]];
        let col = [1, 2, 3, 200];

        let mut circle_off = DrawList::new();
        circle_off.add_filled_circle(center, radius, col, false);
        let n = circle_segment_count(radius);
        assert_eq!(circle_off.vertices.len(), n + 1);
        assert_eq!(circle_off.indices.len(), 3 * n);
        let mut polygon_off = DrawList::new();
        polygon_off.add_filled_polygon(&points, col, false);
        assert_eq!(polygon_off.vertices.len(), 4);
        assert_eq!(polygon_off.indices.len(), 6);

        let mut circle_on = DrawList::new();
        circle_on.add_filled_circle(center, radius, col, true);
        assert_eq!(&circle_on.vertices[..n + 1], &circle_off.vertices[..]);
        assert_eq!(&circle_on.indices[..3 * n], &circle_off.indices[..]);
        let mut polygon_on = DrawList::new();
        polygon_on.add_filled_polygon(&points, col, true);
        assert_eq!(&polygon_on.vertices[..4], &polygon_off.vertices[..]);
        assert_eq!(&polygon_on.indices[..6], &polygon_off.indices[..]);
    }

    #[test]
    fn filled_rect_fringe_is_continuous_at_corners() {
        let col = [7, 7, 7, 255];
        let fade = [7, 7, 7, 0];
        let mut list = DrawList::new();
        list.add_filled_rect([10.0, 10.0], [30.0, 26.0], col);
        list.add_filled_rect_fringe([10.0, 10.0], [30.0, 26.0], col);

        assert_eq!(list.vertices.len(), 4 + 16);
        assert_eq!(list.indices.len(), 6 + 24);
        let f = FRINGE_WIDTH;
        let edge = [[10.0, 10.0], [30.0, 10.0], [30.0, 26.0], [10.0, 26.0]];
        let transparent = [
            [10.0 - f, 10.0 - f],
            [30.0 + f, 10.0 - f],
            [30.0 + f, 26.0 + f],
            [10.0 - f, 26.0 + f],
        ];
        for i in 0..4 {
            let j = (i + 1) % 4;
            let quad = &list.vertices[4 + i * 4..][..4];
            assert_eq!((quad[0].pos, quad[0].col), (edge[i], col));
            assert_eq!((quad[1].pos, quad[1].col), (transparent[i], fade));
            assert_eq!((quad[2].pos, quad[2].col), (transparent[j], fade));
            assert_eq!((quad[3].pos, quad[3].col), (edge[j], col));
            assert!(quad.iter().all(|v| v.uv == WHITE_PIXEL_UV));
        }
        for tri in list.indices[6..].chunks_exact(3) {
            let area = cross2(
                list.vertices[tri[0] as usize].pos,
                list.vertices[tri[1] as usize].pos,
                list.vertices[tri[2] as usize].pos,
            );
            assert!(area > 0.0, "fringe triangle {tri:?} wound opposite");
        }
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].elem_count, 30);
    }

    #[test]
    fn gradient_grid_subdivides_the_rect_into_exact_cells() {
        let mut list = DrawList::new();
        list.add_gradient_grid([10.0, 20.0], [50.0, 80.0], 2, 3, |_cx, _cy| [9, 9, 9, 200]);

        assert_eq!(list.vertices.len(), 2 * 3 * 4);
        assert_eq!(list.indices.len(), 2 * 3 * 6);
        for row in 0..3u32 {
            for col in 0..2u32 {
                let cell = (row * 2 + col) as usize;
                let (x0, y0) = (10.0 + 20.0 * col as f32, 20.0 + 20.0 * row as f32);
                let (x1, y1) = (x0 + 20.0, y0 + 20.0);
                let quad = &list.vertices[4 * cell..4 * cell + 4];
                assert_eq!(quad[0].pos, [x0, y0]);
                assert_eq!(quad[1].pos, [x1, y0]);
                assert_eq!(quad[2].pos, [x1, y1]);
                assert_eq!(quad[3].pos, [x0, y1]);
                let base = 4 * cell as u32;
                assert_eq!(
                    list.indices[6 * cell..6 * cell + 6],
                    [base, base + 1, base + 2, base, base + 2, base + 3]
                );
            }
        }
        for v in &list.vertices {
            assert_eq!(v.uv, WHITE_PIXEL_UV);
            assert_eq!(v.col, [9, 9, 9, 200]);
        }
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].texture, TextureId::WHITE);
        assert_eq!(list.commands[0].elem_count, 36);
    }

    #[test]
    fn gradient_grid_cell_corners_take_the_four_intersection_colors() {
        let color_at = |cx: u32, cy: u32| [cx as u8, cy as u8, 0, 255];
        let mut list = DrawList::new();
        list.add_gradient_grid([0.0, 0.0], [30.0, 20.0], 3, 2, color_at);

        for row in 0..2u32 {
            for col in 0..3u32 {
                let cell = (row * 3 + col) as usize;
                let quad = &list.vertices[4 * cell..4 * cell + 4];
                assert_eq!(quad[0].col, color_at(col, row), "cell ({col}, {row}) TL");
                assert_eq!(
                    quad[1].col,
                    color_at(col + 1, row),
                    "cell ({col}, {row}) TR"
                );
                assert_eq!(
                    quad[2].col,
                    color_at(col + 1, row + 1),
                    "cell ({col}, {row}) BR"
                );
                assert_eq!(
                    quad[3].col,
                    color_at(col, row + 1),
                    "cell ({col}, {row}) BL"
                );
            }
        }
    }

    #[test]
    fn degenerate_gradient_grids_emit_nothing() {
        let mut list = DrawList::new();
        list.add_gradient_grid([0.0, 0.0], [10.0, 10.0], 0, 4, |_, _| [255; 4]);
        list.add_gradient_grid([0.0, 0.0], [10.0, 10.0], 4, 0, |_, _| [255; 4]);

        assert!(list.vertices.is_empty());
        assert!(list.indices.is_empty());
        assert!(list.commands.is_empty());
    }

    #[test]
    fn sv_gradient_outer_corners_are_white_hue_black_black() {
        use crate::color::hsv_to_rgba_u8;
        let hue = 240.0;
        let mut list = DrawList::new();
        list.add_sv_gradient([5.0, 7.0], [165.0, 167.0], hue);

        let res = SV_GRID_RES as usize;
        let quad = |cell: usize| &list.vertices[4 * cell..4 * cell + 4];
        let top_left = quad(0);
        assert_eq!(top_left[0].pos, [5.0, 7.0]);
        assert_eq!(top_left[0].col, [255, 255, 255, 255]);
        let top_right = quad(res - 1);
        assert_eq!(top_right[1].pos, [165.0, 7.0]);
        assert_eq!(top_right[1].col, hsv_to_rgba_u8(hue, 1.0, 1.0, 255));
        assert_eq!(top_right[1].col, [0, 0, 255, 255]);
        let bottom_left = quad((res - 1) * res);
        assert_eq!(bottom_left[3].pos, [5.0, 167.0]);
        assert_eq!(bottom_left[3].col, [0, 0, 0, 255]);
        let bottom_right = quad(res * res - 1);
        assert_eq!(bottom_right[2].pos, [165.0, 167.0]);
        assert_eq!(bottom_right[2].col, [0, 0, 0, 255]);
    }

    #[test]
    fn sv_gradient_top_edge_interpolates_from_white_to_the_hue() {
        use crate::color::hsv_to_rgba_u8;
        let hue = 240.0;
        let mut list = DrawList::new();
        list.add_sv_gradient([0.0, 0.0], [160.0, 160.0], hue);

        let res = SV_GRID_RES;
        for cx in 0..=res {
            let expected = hsv_to_rgba_u8(hue, cx as f32 / res as f32, 1.0, 255);
            let actual = if cx < res {
                list.vertices[4 * cx as usize].col
            } else {
                list.vertices[4 * (res - 1) as usize + 1].col
            };
            assert_eq!(actual, expected, "top-edge intersection cx={cx}");
        }
        assert_eq!(list.vertices[4 * 8].col, [128, 128, 255, 255]);
    }

    #[test]
    fn sv_gradient_emits_the_documented_grid_and_no_fringe() {
        let mut list = DrawList::new();
        list.add_sv_gradient([0.0, 0.0], [160.0, 160.0], 120.0);

        let res = SV_GRID_RES as usize;
        assert_eq!(list.vertices.len(), 4 * res * res);
        assert_eq!(list.indices.len(), 6 * res * res);
        for v in &list.vertices {
            assert_eq!(v.uv, WHITE_PIXEL_UV);
            assert_eq!(v.col[3], 255, "gradient interior is opaque, un-fringed");
        }
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].texture, TextureId::WHITE);
        assert_eq!(list.commands[0].elem_count, (6 * res * res) as u32);
    }

    #[test]
    fn hue_strip_stacks_six_quads_with_the_seven_boundary_colors() {
        let mut list = DrawList::new();
        list.add_hue_strip([10.0, 20.0], [26.0, 140.0]);

        let boundary = [
            [255, 0, 0, 255],
            [255, 255, 0, 255],
            [0, 255, 0, 255],
            [0, 255, 255, 255],
            [0, 0, 255, 255],
            [255, 0, 255, 255],
            [255, 0, 0, 255],
        ];
        assert_eq!(list.vertices.len(), 6 * 4);
        assert_eq!(list.indices.len(), 6 * 6);
        for i in 0..6 {
            let quad = &list.vertices[4 * i..4 * i + 4];
            let (y0, y1) = (20.0 + 20.0 * i as f32, 40.0 + 20.0 * i as f32);
            assert_eq!(quad[0].pos, [10.0, y0]);
            assert_eq!(quad[1].pos, [26.0, y0]);
            assert_eq!(quad[2].pos, [26.0, y1]);
            assert_eq!(quad[3].pos, [10.0, y1]);
            assert_eq!(quad[0].col, boundary[i]);
            assert_eq!(quad[1].col, boundary[i]);
            assert_eq!(quad[2].col, boundary[i + 1]);
            assert_eq!(quad[3].col, boundary[i + 1]);
            for v in quad {
                assert_eq!(v.uv, WHITE_PIXEL_UV);
            }
        }
        assert_eq!(list.commands.len(), 1);
        assert_eq!(list.commands[0].texture, TextureId::WHITE);
        assert_eq!(list.commands[0].elem_count, 36);
    }
}
