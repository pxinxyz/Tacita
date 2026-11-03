mod curves;
mod polygon;
mod stroke;
#[cfg(test)]
mod tests;

use crate::draw::{DrawList, DrawVert, TextureId, WHITE_PIXEL_UV};
use crate::ui::Ui;
use alloc::vec::Vec;

pub const MAX_PATH_POINTS: usize = 2048;
pub const MAX_POLYGON_POINTS: usize = 512;
pub const MAX_DASH_LENGTHS: usize = 16;
const MAX_SECTIONS: usize = 16384;
const LIMIT: f32 = 1_000_000.0;
#[derive(Clone, Copy)]
pub(crate) struct RasterOptions {
    scale: f32,
    aa: bool,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineJoin {
    #[default]
    Miter = 0,
    Bevel = 1,
    Round = 2,
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineCap {
    #[default]
    Butt = 0,
    Square = 1,
    Round = 2,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    pub width: f32,
    pub join: LineJoin,
    pub cap: LineCap,
    pub miter_limit: f32,
    pub tolerance: f32,
}
impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            width: 1.,
            join: LineJoin::Miter,
            cap: LineCap::Butt,
            miter_limit: 4.,
            tolerance: 0.25,
        }
    }
}
impl StrokeStyle {
    pub fn is_valid(self) -> bool {
        self.width.is_finite()
            && (0.0..=4096.).contains(&self.width)
            && self.miter_limit.is_finite()
            && (1.0..=64.).contains(&self.miter_limit)
            && self.tolerance.is_finite()
            && (0.05..=4.).contains(&self.tolerance)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct DashPattern<'a> {
    pub lengths: &'a [f32],
    pub phase: f32,
}
impl DashPattern<'_> {
    pub fn is_valid(self) -> bool {
        !self.lengths.is_empty()
            && self.lengths.len() <= MAX_DASH_LENGTHS
            && self
                .lengths
                .iter()
                .all(|v| v.is_finite() && *v >= 0.01 && *v <= LIMIT)
            && self.phase.is_finite()
            && self.phase.abs() <= LIMIT
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathError {
    InvalidGeometry,
    TooManyPoints,
    TooComplex,
    NotSimple,
    Degenerate,
}

#[derive(Default)]
pub(crate) struct PathScratch {
    points: Vec<[f32; 2]>,
    curve: Vec<[f32; 2]>,
    dashed: Vec<[f32; 2]>,
    runs: Vec<core::ops::Range<usize>>,
    sections: Vec<stroke::Section>,
    meshes: Vec<stroke::StrokeRun>,
    triangles: Vec<[usize; 3]>,
}
fn validate_points(points: &[[f32; 2]]) -> Result<(), PathError> {
    if points.len() > MAX_PATH_POINTS {
        return Err(PathError::TooManyPoints);
    }
    if points
        .iter()
        .flatten()
        .any(|v| !v.is_finite() || v.abs() > LIMIT)
    {
        return Err(PathError::InvalidGeometry);
    }
    Ok(())
}
fn canonical(points: &[[f32; 2]], closed: bool, out: &mut Vec<[f32; 2]>) -> Result<(), PathError> {
    validate_points(points)?;
    out.clear();
    for &point in points {
        if out
            .last()
            .is_none_or(|&last| length(sub(point, last)) > 0.00001)
        {
            out.push(point);
        }
    }
    if closed && out.len() > 1 && length(sub(out[0], *out.last().unwrap())) <= 0.00001 {
        out.pop();
    }
    Ok(())
}
fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}
fn mul(a: [f32; 2], v: f32) -> [f32; 2] {
    [a[0] * v, a[1] * v]
}
fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}
fn length(a: [f32; 2]) -> f32 {
    crate::draw::sqrt_approx(dot(a, a))
}
fn unit(a: [f32; 2]) -> [f32; 2] {
    mul(a, 1. / length(a).max(0.000001))
}
fn normal(a: [f32; 2]) -> [f32; 2] {
    [-a[1], a[0]]
}
fn vertex(pos: [f32; 2], mut color: [u8; 4], alpha: f32) -> DrawVert {
    color[3] = (color[3] as f32 * alpha.clamp(0., 1.) + 0.5) as u8;
    DrawVert {
        pos,
        uv: WHITE_PIXEL_UV,
        col: color,
    }
}
fn triangle(draw: &mut DrawList, points: [[f32; 2]; 3], alpha: [f32; 3], color: [u8; 4]) {
    draw.add_mesh_triangle(
        TextureId::WHITE,
        core::array::from_fn(|i| vertex(points[i], color, alpha[i])),
    );
}
impl DrawList {
    fn path_work(
        &mut self,
        work: impl FnOnce(&mut Self, &mut PathScratch) -> Result<(), PathError>,
    ) -> Result<(), PathError> {
        let mut scratch = core::mem::take(&mut self.path_scratch);
        let result = work(self, &mut scratch);
        self.path_scratch = scratch;
        result
    }
    pub(crate) fn path_stroke(
        &mut self,
        points: &[[f32; 2]],
        closed: bool,
        style: StrokeStyle,
        dash: Option<DashPattern<'_>>,
        color: [u8; 4],
        raster: RasterOptions,
    ) -> Result<(), PathError> {
        if !style.is_valid() || dash.is_some_and(|d| !d.is_valid()) {
            return Err(PathError::InvalidGeometry);
        }
        self.path_work(|draw, scratch| {
            canonical(points, closed, &mut scratch.points)?;
            if scratch.points.len() < 2 || style.width == 0. || color[3] == 0 {
                return Ok(());
            }
            stroke::prepare(scratch, closed, style, dash, raster)?;
            stroke::emit(draw, scratch, color);
            Ok(())
        })
    }
    pub(crate) fn path_curve(
        &mut self,
        points: [[f32; 2]; 4],
        style: StrokeStyle,
        color: [u8; 4],
        raster: RasterOptions,
    ) -> Result<(), PathError> {
        if !style.is_valid() {
            return Err(PathError::InvalidGeometry);
        }
        validate_points(&points)?;
        self.path_work(|draw, scratch| {
            curves::flatten(points, style.tolerance / raster.scale, &mut scratch.curve)?;
            let closed = points[0] == points[3];
            canonical(&scratch.curve, closed, &mut scratch.points)?;
            if scratch.points.len() < 2 || style.width == 0. || color[3] == 0 {
                return Ok(());
            }
            stroke::prepare(scratch, closed, style, None, raster)?;
            stroke::emit(draw, scratch, color);
            Ok(())
        })
    }
    pub(crate) fn path_polygon(
        &mut self,
        points: &[[f32; 2]],
        color: [u8; 4],
        raster: RasterOptions,
    ) -> Result<(), PathError> {
        if points.len() > MAX_POLYGON_POINTS {
            return Err(PathError::TooManyPoints);
        }
        self.path_work(|draw, scratch| {
            canonical(points, true, &mut scratch.points)?;
            polygon::prepare(&mut scratch.points, &mut scratch.triangles)?;
            polygon::emit(
                draw,
                &scratch.points,
                &scratch.triangles,
                color,
                raster.scale,
                raster.aa,
            );
            Ok(())
        })
    }
}
impl Ui<'_> {
    pub fn draw_polyline(
        &mut self,
        points: &[[f32; 2]],
        closed: bool,
        style: StrokeStyle,
        color: [u8; 4],
    ) -> Result<(), PathError> {
        if self.ctx.current_window.is_none() {
            return Ok(());
        }
        let scale = self.ctx.font.raster_scale();
        let aa = self.ctx.style.anti_aliasing;
        crate::widgets::draw_parts(self.ctx).1.path_stroke(
            points,
            closed,
            style,
            None,
            color,
            RasterOptions { scale, aa },
        )
    }
    pub fn draw_dashed_polyline(
        &mut self,
        points: &[[f32; 2]],
        closed: bool,
        style: StrokeStyle,
        dash: DashPattern<'_>,
        color: [u8; 4],
    ) -> Result<(), PathError> {
        if self.ctx.current_window.is_none() {
            return Ok(());
        }
        let scale = self.ctx.font.raster_scale();
        let aa = self.ctx.style.anti_aliasing;
        crate::widgets::draw_parts(self.ctx).1.path_stroke(
            points,
            closed,
            style,
            Some(dash),
            color,
            RasterOptions { scale, aa },
        )
    }
    pub fn draw_quadratic_bezier(
        &mut self,
        points: [[f32; 2]; 3],
        style: StrokeStyle,
        color: [u8; 4],
    ) -> Result<(), PathError> {
        validate_points(&points)?;
        self.draw_cubic_bezier(
            [
                points[0],
                add(points[0], mul(sub(points[1], points[0]), 2. / 3.)),
                add(points[2], mul(sub(points[1], points[2]), 2. / 3.)),
                points[2],
            ],
            style,
            color,
        )
    }
    pub fn draw_cubic_bezier(
        &mut self,
        points: [[f32; 2]; 4],
        style: StrokeStyle,
        color: [u8; 4],
    ) -> Result<(), PathError> {
        if self.ctx.current_window.is_none() {
            return Ok(());
        }
        let scale = self.ctx.font.raster_scale();
        let aa = self.ctx.style.anti_aliasing;
        crate::widgets::draw_parts(self.ctx).1.path_curve(
            points,
            style,
            color,
            RasterOptions { scale, aa },
        )
    }
    pub fn draw_concave_polygon(
        &mut self,
        points: &[[f32; 2]],
        color: [u8; 4],
    ) -> Result<(), PathError> {
        if self.ctx.current_window.is_none() {
            return Ok(());
        }
        let scale = self.ctx.font.raster_scale();
        let aa = self.ctx.style.anti_aliasing;
        crate::widgets::draw_parts(self.ctx).1.path_polygon(
            points,
            color,
            RasterOptions { scale, aa },
        )
    }
}
