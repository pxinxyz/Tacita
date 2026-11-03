use super::*;

fn solid_area(draw: &DrawList) -> f64 {
    let data = draw.as_draw_data();
    data.indices
        .chunks_exact(3)
        .filter_map(|t| {
            let v = [
                data.vertices[t[0] as usize],
                data.vertices[t[1] as usize],
                data.vertices[t[2] as usize],
            ];
            v.iter()
                .all(|v| v.col[3] == 255)
                .then(|| cross(sub(v[1].pos, v[0].pos), sub(v[2].pos, v[0].pos)).abs() as f64 * 0.5)
        })
        .sum()
}
fn covers(draw: &DrawList, p: [f32; 2]) -> bool {
    let data = draw.as_draw_data();
    data.indices.chunks_exact(3).any(|t| {
        let a = data.vertices[t[0] as usize].pos;
        let b = data.vertices[t[1] as usize].pos;
        let c = data.vertices[t[2] as usize].pos;
        let x = cross(sub(b, a), sub(p, a));
        let y = cross(sub(c, b), sub(p, b));
        let z = cross(sub(a, c), sub(p, c));
        (x >= 0. && y >= 0. && z >= 0.) || (x <= 0. && y <= 0. && z <= 0.)
    })
}
fn raster(aa: bool) -> RasterOptions {
    RasterOptions { scale: 1., aa }
}
#[test]
fn joined_body_has_no_overlapping_corner_and_miter_limit_bevels() {
    let points = [[10., 10.], [60., 10.], [60., 60.]];
    let mut draw = DrawList::new();
    let style = StrokeStyle {
        width: 10.,
        ..Default::default()
    };
    draw.path_stroke(&points, false, style, None, [255; 4], raster(false))
        .unwrap();
    assert!((solid_area(&draw) - 1000.).abs() < 0.01);
    assert!(covers(&draw, [64., 6.]));
    let mut bevel = DrawList::new();
    bevel
        .path_stroke(
            &points,
            false,
            StrokeStyle {
                miter_limit: 1.,
                ..style
            },
            None,
            [255; 4],
            raster(false),
        )
        .unwrap();
    assert!(!covers(&bevel, [64., 6.]));
    assert!((solid_area(&bevel) - 987.5).abs() < 0.01);
}
#[test]
fn caps_have_requested_extent_and_round_body_area() {
    for (cap, expected) in [
        (LineCap::Butt, 100.),
        (LineCap::Square, 200.),
        (LineCap::Round, 100. + core::f64::consts::PI * 25.),
    ] {
        let mut draw = DrawList::new();
        draw.path_stroke(
            &[[0., 0.], [10., 0.]],
            false,
            StrokeStyle {
                width: 10.,
                cap,
                ..Default::default()
            },
            None,
            [255; 4],
            raster(false),
        )
        .unwrap();
        assert!((solid_area(&draw) - expected).abs() < 1.1, "{cap:?}");
        assert_eq!(covers(&draw, [-4., 0.]), cap != LineCap::Butt);
    }
}
#[test]
fn concave_fill_preserves_notch_in_either_winding() {
    let mut points = alloc::vec![
        [0., 0.],
        [60., 0.],
        [60., 20.],
        [20., 20.],
        [20., 60.],
        [0., 60.]
    ];
    for _ in 0..2 {
        let mut draw = DrawList::new();
        draw.path_polygon(&points, [255; 4], raster(false)).unwrap();
        assert!((solid_area(&draw) - 2000.).abs() < 0.001);
        assert!(!covers(&draw, [35., 35.]));
        assert!(covers(&draw, [5., 50.]));
        points.reverse();
    }
}
#[test]
fn invalid_shapes_and_subdivision_failures_emit_nothing() {
    let mut draw = DrawList::new();
    assert_eq!(
        draw.path_polygon(
            &[[0., 0.], [20., 20.], [0., 20.], [20., 0.]],
            [255; 4],
            raster(true)
        ),
        Err(PathError::NotSimple)
    );
    assert_eq!(
        draw.path_polygon(&[[0., 0.], [10., 0.], [5., 0.]], [255; 4], raster(true)),
        Err(PathError::NotSimple)
    );
    assert_eq!(
        draw.path_stroke(
            &[[0., 0.], [f32::NAN, 2.]],
            false,
            Default::default(),
            None,
            [255; 4],
            raster(true)
        ),
        Err(PathError::InvalidGeometry)
    );
    assert!(draw
        .path_stroke(
            &[[0., 0.], [1_000_000., 0.]],
            false,
            Default::default(),
            Some(DashPattern {
                lengths: &[0.01, 0.01],
                phase: 0.
            }),
            [255; 4],
            raster(true)
        )
        .is_err());
    assert_eq!(draw.as_draw_data().indices.len(), 0);
}
#[test]
fn repeated_close_and_collinear_vertices_are_canonical() {
    let points = [
        [0., 0.],
        [10., 0.],
        [20., 0.],
        [20., 20.],
        [0., 20.],
        [0., 0.],
    ];
    let mut draw = DrawList::new();
    draw.path_polygon(&points, [255; 4], raster(false)).unwrap();
    assert!((solid_area(&draw) - 400.).abs() < 0.001);
}
#[test]
fn adaptive_curves_keep_endpoints_and_detect_collinear_overshoot() {
    let mut points = Vec::new();
    curves::flatten(
        [[0., 0.], [100., 0.], [-100., 0.], [10., 0.]],
        0.25,
        &mut points,
    )
    .unwrap();
    assert_eq!(points[0], [0., 0.]);
    assert_eq!(*points.last().unwrap(), [10., 0.]);
    assert!(points.len() > 4);
    assert!(points.iter().any(|p| p[0] < 0.));
    let mut coarse = Vec::new();
    let control = [[0., 0.], [0., 100.], [100., 100.], [100., 0.]];
    curves::flatten(control, 1., &mut coarse).unwrap();
    curves::flatten(control, 0.1, &mut points).unwrap();
    assert!(points.len() > coarse.len());
}
#[test]
fn unattainable_curve_budget_rejects_before_drawing() {
    let mut draw = DrawList::new();
    let points = [
        [-1_000_000., -1_000_000.],
        [-1_000_000., 1_000_000.],
        [1_000_000., 1_000_000.],
        [1_000_000., -1_000_000.],
    ];
    assert_eq!(
        draw.path_curve(
            points,
            StrokeStyle {
                tolerance: 0.05,
                ..Default::default()
            },
            [255; 4],
            RasterOptions {
                scale: 4.,
                aa: true
            }
        ),
        Err(PathError::TooComplex)
    );
    assert!(draw.as_draw_data().indices.is_empty());
}

#[test]
fn path_canvas_calls_without_a_window_are_inert() {
    let mut ctx =
        crate::context::Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap();
    ctx.new_frame(0.016);
    let mut ui = ctx.ui();
    let line = [[0., 0.], [20., 0.], [20., 20.]];
    ui.draw_polyline(&line, false, Default::default(), [255; 4])
        .unwrap();
    ui.draw_dashed_polyline(
        &line,
        false,
        Default::default(),
        DashPattern {
            lengths: &[5., 5.],
            phase: 0.,
        },
        [255; 4],
    )
    .unwrap();
    ui.draw_quadratic_bezier(line, Default::default(), [255; 4])
        .unwrap();
    ui.draw_cubic_bezier(
        [line[0], line[1], line[2], [0., 20.]],
        Default::default(),
        [255; 4],
    )
    .unwrap();
    ui.draw_concave_polygon(&line, [255; 4]).unwrap();
    assert!(ctx.render().indices.is_empty());
}

#[test]
fn odd_patterns_and_negative_phase_agree_with_repeated_patterns() {
    let points = [[0., 0.], [40., 0.], [40., 40.]];
    let mut a = DrawList::new();
    let mut b = DrawList::new();
    a.path_stroke(
        &points,
        false,
        Default::default(),
        Some(DashPattern {
            lengths: &[8., 4., 2.],
            phase: -3.,
        }),
        [255; 4],
        raster(true),
    )
    .unwrap();
    b.path_stroke(
        &points,
        false,
        Default::default(),
        Some(DashPattern {
            lengths: &[8., 4., 2., 8., 4., 2.],
            phase: 25.,
        }),
        [255; 4],
        raster(true),
    )
    .unwrap();
    assert_eq!(a.as_draw_data().vertices, b.as_draw_data().vertices);
    assert_eq!(a.as_draw_data().indices, b.as_draw_data().indices);
}
#[test]
fn dash_run_crosses_corner_and_closed_seam_without_extra_caps() {
    let mut scratch = PathScratch::default();
    scratch
        .points
        .extend_from_slice(&[[0., 0.], [10., 0.], [10., 10.]]);
    stroke::prepare(
        &mut scratch,
        false,
        Default::default(),
        Some(DashPattern {
            lengths: &[15., 5.],
            phase: 0.,
        }),
        raster(false),
    )
    .unwrap();
    assert_eq!(scratch.runs.len(), 1);
    assert_eq!(
        &scratch.dashed[scratch.runs[0].clone()],
        &[[0., 0.], [10., 0.], [10., 5.]]
    );
    scratch.points.clear();
    scratch
        .points
        .extend_from_slice(&[[0., 0.], [10., 0.], [10., 10.], [0., 10.]]);
    stroke::prepare(
        &mut scratch,
        true,
        Default::default(),
        Some(DashPattern {
            lengths: &[15., 5.],
            phase: 5.,
        }),
        raster(false),
    )
    .unwrap();
    assert_eq!(scratch.runs.len(), 2);
    let crossing = &scratch.dashed[scratch.runs[1].clone()];
    assert!(crossing.contains(&[0., 0.]));
    assert_ne!(crossing[0], [0., 0.]);
    assert_ne!(*crossing.last().unwrap(), [0., 0.]);
}
#[test]
fn aa_has_finite_coverage_and_leaves_solid_stroke_area_unchanged() {
    let points = [[0., 0.], [30., 0.], [30., 40.]];
    let style = StrokeStyle {
        width: 8.,
        join: LineJoin::Round,
        cap: LineCap::Round,
        ..Default::default()
    };
    let mut a = DrawList::new();
    let mut b = DrawList::new();
    a.path_stroke(&points, false, style, None, [255; 4], raster(false))
        .unwrap();
    b.path_stroke(&points, false, style, None, [255; 4], raster(true))
        .unwrap();
    assert!((solid_area(&a) - solid_area(&b)).abs() < 0.01);
    assert!(b.as_draw_data().vertices.iter().any(|v| v.col[3] == 0));
    assert!(b
        .as_draw_data()
        .vertices
        .iter()
        .all(|v| v.pos.iter().all(|p| p.is_finite())));
}
