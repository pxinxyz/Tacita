use super::{
    add, cross, dot, length, mul, normal, sub, triangle, unit, DashPattern, LineCap, LineJoin,
    PathError, PathScratch, RasterOptions, StrokeStyle, MAX_SECTIONS,
};
use crate::draw::DrawList;
use alloc::vec::Vec;
use core::{
    f32::consts::{PI, TAU},
    ops::Range,
};

#[derive(Clone, Copy)]
pub(super) struct Section {
    left: [f32; 2],
    right: [f32; 2],
    outer_left: [f32; 2],
    outer_right: [f32; 2],
}
pub(super) struct StrokeRun {
    sections: Range<usize>,
    closed: bool,
    cap: LineCap,
    start: [f32; 2],
    end: [f32; 2],
    start_dir: [f32; 2],
    end_dir: [f32; 2],
    radius: f32,
    fringe: f32,
    cap_steps: usize,
}
fn angle(v: [f32; 2]) -> f32 {
    let x = v[0].abs() as f64;
    let y = v[1].abs() as f64;
    let a = if x >= y {
        crate::raster::atan_unit(if x == 0. { 0. } else { y / x }) as f32
    } else {
        PI * 0.5 - crate::raster::atan_unit(x / y) as f32
    };
    let a = if v[0] < 0. { PI - a } else { a };
    if v[1] < 0. {
        -a
    } else {
        a
    }
}
fn arc_steps(sweep: f32, radius: f32, scale: f32, tolerance: f32) -> Result<usize, PathError> {
    let n = (sweep.abs() * crate::draw::sqrt_approx(radius * scale / (8. * tolerance)))
        .max(sweep.abs() / PI * 12.)
        .max(1.);
    let steps = n as usize + usize::from(n > n as usize as f32);
    if steps > 2048 {
        return Err(PathError::TooComplex);
    }
    Ok(steps)
}
fn section(center: [f32; 2], left: [f32; 2], right: [f32; 2], radius: f32, fringe: f32) -> Section {
    Section {
        left: add(center, mul(left, radius)),
        right: add(center, mul(right, radius)),
        outer_left: add(center, mul(left, radius + fringe)),
        outer_right: add(center, mul(right, radius + fringe)),
    }
}
fn push_section(out: &mut Vec<Section>, value: Section) -> Result<(), PathError> {
    if out.len() >= MAX_SECTIONS {
        return Err(PathError::TooComplex);
    }
    out.push(value);
    Ok(())
}
fn join(
    out: &mut Vec<Section>,
    point: [f32; 2],
    before: [f32; 2],
    after: [f32; 2],
    style: StrokeStyle,
    fringe: f32,
    scale: f32,
) -> Result<(), PathError> {
    let n0 = normal(before);
    let n1 = normal(after);
    let turn = cross(before, after);
    let radius = style.width * 0.5;
    if dot(before, after) < -0.99999 {
        push_section(out, section(point, n0, mul(n0, -1.), radius, fringe))?;
        return push_section(out, section(point, n1, mul(n1, -1.), radius, fringe));
    }
    let bisector = unit(add(n0, n1));
    let factor = 1. / dot(bisector, n1).max(0.000001);
    if turn.abs() < 0.000001 || (style.join == LineJoin::Miter && factor <= style.miter_limit) {
        let miter = mul(bisector, factor);
        return push_section(out, section(point, miter, mul(miter, -1.), radius, fringe));
    }
    let outer_sign = if turn > 0. { -1. } else { 1. };
    let inner = mul(bisector, -outer_sign * factor.min(64.));
    let first = mul(n0, outer_sign);
    let last = mul(n1, outer_sign);
    let start = angle(first);
    let mut sweep = angle(last) - start;
    if sweep > PI {
        sweep -= TAU;
    }
    if sweep < -PI {
        sweep += TAU;
    }
    let steps = if style.join == LineJoin::Round {
        arc_steps(sweep, radius + fringe, scale, style.tolerance)?
    } else {
        1
    };
    for i in 0..=steps {
        let outer = if i == 0 {
            first
        } else if i == steps {
            last
        } else {
            let (s, c) = crate::draw::sin_cos_approx(start + sweep * i as f32 / steps as f32);
            [c, s]
        };
        let (left, right) = if outer_sign < 0. {
            (inner, outer)
        } else {
            (outer, inner)
        };
        push_section(out, section(point, left, right, radius, fringe))?;
    }
    Ok(())
}
fn build_run(
    points: &[[f32; 2]],
    closed: bool,
    style: StrokeStyle,
    raster: RasterOptions,
    sections: &mut Vec<Section>,
    meshes: &mut Vec<StrokeRun>,
    budget: &mut usize,
) -> Result<(), PathError> {
    let RasterOptions { scale, aa } = raster;
    if points.len() < 2 {
        return Ok(());
    }
    let points = if closed && points[0] == points[points.len() - 1] {
        &points[..points.len() - 1]
    } else {
        points
    };
    if points.len() < 2 {
        return Ok(());
    }
    let start = sections.len();
    let radius = style.width * 0.5;
    let fringe = if aa {
        crate::draw::FRINGE_WIDTH / scale
    } else {
        0.
    };
    let start_dir = unit(sub(points[1], points[0]));
    let end_dir = unit(sub(points[points.len() - 1], points[points.len() - 2]));
    for i in 0..points.len() {
        if !closed && (i == 0 || i == points.len() - 1) {
            let dir = if i == 0 { start_dir } else { end_dir };
            let sign = if i == 0 { -1. } else { 1. };
            let shift = if style.cap == LineCap::Square {
                radius * sign
            } else {
                0.
            };
            let center = add(points[i], mul(dir, shift));
            let n = normal(dir);
            let mut s = section(center, n, mul(n, -1.), radius, fringe);
            if style.cap != LineCap::Round {
                s.outer_left = add(s.outer_left, mul(dir, sign * fringe));
                s.outer_right = add(s.outer_right, mul(dir, sign * fringe));
            }
            push_section(sections, s)?;
        } else {
            let before = unit(sub(
                points[i],
                points[(i + points.len() - 1) % points.len()],
            ));
            let after = unit(sub(points[(i + 1) % points.len()], points[i]));
            join(sections, points[i], before, after, style, fringe, scale)?;
        }
    }
    let cap_steps = if !closed && style.cap == LineCap::Round {
        arc_steps(PI, radius + fringe, scale, style.tolerance)?
    } else {
        0
    };
    *budget += sections.len() - start + cap_steps * 2;
    if *budget > MAX_SECTIONS {
        return Err(PathError::TooComplex);
    }
    meshes.push(StrokeRun {
        sections: start..sections.len(),
        closed,
        cap: style.cap,
        start: points[0],
        end: points[points.len() - 1],
        start_dir,
        end_dir,
        radius,
        fringe,
        cap_steps,
    });
    Ok(())
}
fn append_point(out: &mut Vec<[f32; 2]>, point: [f32; 2]) -> Result<(), PathError> {
    if out.last() == Some(&point) {
        return Ok(());
    }
    if out.len() >= MAX_SECTIONS {
        return Err(PathError::TooComplex);
    }
    out.push(point);
    Ok(())
}
fn finish_run(start: &mut Option<usize>, out: &[[f32; 2]], runs: &mut Vec<Range<usize>>) {
    if let Some(i) = start.take() {
        if out.len() >= i + 2 {
            runs.push(i..out.len());
        }
    }
}
fn split_dashes(
    points: &[[f32; 2]],
    closed: bool,
    dash: DashPattern<'_>,
    out: &mut Vec<[f32; 2]>,
    runs: &mut Vec<Range<usize>>,
) -> Result<(), PathError> {
    let count = dash.lengths.len() * if dash.lengths.len() % 2 == 1 { 2 } else { 1 };
    let period = dash.lengths.iter().map(|&v| v as f64).sum::<f64>()
        * if dash.lengths.len() % 2 == 1 { 2. } else { 1. };
    let mut phase = (dash.phase as f64 % period + period) % period;
    let mut index = 0;
    while phase >= dash.lengths[index % dash.lengths.len()] as f64 {
        phase -= dash.lengths[index % dash.lengths.len()] as f64;
        index = (index + 1) % count;
    }
    let mut remaining = dash.lengths[index % dash.lengths.len()] as f64 - phase;
    let mut on = index % 2 == 0;
    let mut start = None;
    let mut steps = 0;
    for i in 0..points.len() - usize::from(!closed) {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let delta = sub(b, a);
        let len = length(delta) as f64;
        let mut at = 0.;
        while at < len {
            steps += 1;
            if steps > MAX_SECTIONS {
                return Err(PathError::TooComplex);
            }
            if on && start.is_none() {
                start = Some(out.len());
                out.push(add(a, mul(delta, (at / len) as f32)));
            }
            let amount = remaining.min(len - at);
            at += amount;
            remaining -= amount;
            if on {
                let end = if at == len {
                    b
                } else {
                    add(a, mul(delta, (at / len) as f32))
                };
                append_point(out, end)?;
            }
            if remaining <= 0.000000001 {
                if on {
                    finish_run(&mut start, out, runs);
                }
                index = (index + 1) % count;
                on = index % 2 == 0;
                remaining = dash.lengths[index % dash.lengths.len()] as f64;
            }
        }
    }
    finish_run(&mut start, out, runs);
    if closed && runs.len() > 1 {
        let first = runs[0].clone();
        let last = runs[runs.len() - 1].clone();
        if out[first.start] == points[0] && out[last.end - 1] == points[0] {
            let begin = out.len();
            for i in last.clone() {
                let p = out[i];
                append_point(out, p)?;
            }
            for i in first.start + 1..first.end {
                let p = out[i];
                append_point(out, p)?;
            }
            runs.pop();
            runs.remove(0);
            runs.push(begin..out.len());
        }
    }
    Ok(())
}
pub(super) fn prepare(
    scratch: &mut PathScratch,
    closed: bool,
    style: StrokeStyle,
    dash: Option<DashPattern<'_>>,
    raster: RasterOptions,
) -> Result<(), PathError> {
    scratch.sections.clear();
    scratch.meshes.clear();
    scratch.dashed.clear();
    scratch.runs.clear();
    let mut budget = 0;
    if closed && scratch.points.len() < 3 {
        return Err(PathError::Degenerate);
    }
    if let Some(dash) = dash {
        split_dashes(
            &scratch.points,
            closed,
            dash,
            &mut scratch.dashed,
            &mut scratch.runs,
        )?;
        for run in &scratch.runs {
            let points = &scratch.dashed[run.clone()];
            let is_closed = closed && points[0] == points[points.len() - 1];
            build_run(
                points,
                is_closed,
                style,
                raster,
                &mut scratch.sections,
                &mut scratch.meshes,
                &mut budget,
            )?;
        }
    } else {
        build_run(
            &scratch.points,
            closed,
            style,
            raster,
            &mut scratch.sections,
            &mut scratch.meshes,
            &mut budget,
        )?;
    }
    Ok(())
}
fn band(draw: &mut DrawList, a: [f32; 2], b: [f32; 2], oa: [f32; 2], ob: [f32; 2], color: [u8; 4]) {
    triangle(draw, [a, b, ob], [1., 1., 0.], color);
    triangle(draw, [a, ob, oa], [1., 0., 0.], color);
}
fn round_cap(
    draw: &mut DrawList,
    center: [f32; 2],
    direction: [f32; 2],
    start: bool,
    run: &StrokeRun,
    color: [u8; 4],
) {
    let first = mul(normal(direction), if start { 1. } else { -1. });
    let begin = angle(first);
    let mut last = first;
    for i in 1..=run.cap_steps {
        let next = if i == run.cap_steps {
            mul(first, -1.)
        } else {
            let (s, c) = crate::draw::sin_cos_approx(begin + PI * i as f32 / run.cap_steps as f32);
            [c, s]
        };
        let a = add(center, mul(last, run.radius));
        let b = add(center, mul(next, run.radius));
        triangle(draw, [center, a, b], [1.; 3], color);
        if run.fringe > 0. {
            band(
                draw,
                a,
                b,
                add(center, mul(last, run.radius + run.fringe)),
                add(center, mul(next, run.radius + run.fringe)),
                color,
            );
        }
        last = next;
    }
}
pub(super) fn emit(draw: &mut DrawList, scratch: &PathScratch, color: [u8; 4]) {
    for run in &scratch.meshes {
        let sections = &scratch.sections[run.sections.clone()];
        let count = sections.len() - usize::from(!run.closed);
        for i in 0..count {
            let a = sections[i];
            let b = sections[(i + 1) % sections.len()];
            triangle(draw, [a.left, a.right, b.right], [1.; 3], color);
            triangle(draw, [a.left, b.right, b.left], [1.; 3], color);
            if run.fringe > 0. {
                band(draw, a.left, b.left, a.outer_left, b.outer_left, color);
                band(draw, a.right, b.right, a.outer_right, b.outer_right, color);
            }
        }
        if !run.closed {
            if run.cap == LineCap::Round {
                round_cap(draw, run.start, run.start_dir, true, run, color);
                round_cap(draw, run.end, run.end_dir, false, run, color);
            } else if run.fringe > 0. {
                for s in [sections[0], sections[sections.len() - 1]] {
                    band(draw, s.left, s.right, s.outer_left, s.outer_right, color);
                }
            }
        }
    }
}
