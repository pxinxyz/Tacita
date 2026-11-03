use super::*;

fn wrapping() -> LayoutOptions {
    LayoutOptions {
        wrap: true,
        ..Default::default()
    }
}
#[test]
fn default_extension_matches_legacy_rectangles_exactly() {
    let items = [
        LayoutItem::fixed([20., 10.]),
        LayoutItem::fixed([30., 20.]),
        LayoutItem::fixed([40., 15.]),
    ];
    for kind in [LayoutKind::Row, LayoutKind::Column, LayoutKind::Grid] {
        let spec = LayoutSpec {
            kind,
            columns: 2,
            ..Default::default()
        };
        let a = resolve_layout(spec, &items).unwrap();
        let b = resolve_layout_extended(spec, &items, Default::default(), &[]).unwrap();
        assert_eq!(a.size, b.size);
        for i in 0..items.len() {
            assert_eq!(a.rect(i), b.rect(i));
        }
    }
}
#[test]
fn rows_wrap_at_outer_size_and_keep_per_line_flex() {
    let items = [LayoutItem {
        preferred: [40., 10.],
        min: [20., 10.],
        grow: 1.,
        ..Default::default()
    }; 3];
    let spec = LayoutSpec {
        kind: LayoutKind::Row,
        size: [100., 0.],
        gap: [10.; 2],
        ..Default::default()
    };
    let g = resolve_layout_extended(spec, &items, wrapping(), &[]).unwrap();
    assert_eq!(g.size, [100., 30.]);
    assert_eq!(g.rect(0), Some([0., 0., 45., 10.]));
    assert_eq!(g.rect(1), Some([55., 0., 100., 10.]));
    assert_eq!(g.rect(2), Some([0., 20., 100., 30.]));
}
#[test]
fn columns_wrap_and_content_distribution_stretches_lines() {
    let items = [LayoutItem {
        preferred: [20., 30.],
        ..Default::default()
    }; 3];
    let spec = LayoutSpec {
        kind: LayoutKind::Column,
        size: [100., 70.],
        gap: [10.; 2],
        align: LayoutAlign::Stretch,
        ..Default::default()
    };
    let g = resolve_layout_extended(
        spec,
        &items,
        LayoutOptions {
            wrap: true,
            align_content: ContentAlign::Stretch,
        },
        &[],
    )
    .unwrap();
    assert_eq!(g.rect(0), Some([0., 0., 45., 30.]));
    assert_eq!(g.rect(1), Some([0., 40., 45., 70.]));
    assert_eq!(g.rect(2), Some([55., 0., 100., 30.]));
}
#[test]
fn wrapping_baselines_margins_and_absolute_items_remain_independent() {
    let items = [
        LayoutItem {
            preferred: [40., 20.],
            baseline: 15.,
            margin: [2., 3., 2., 4.],
            ..Default::default()
        },
        LayoutItem {
            preferred: [40., 10.],
            baseline: 8.,
            ..Default::default()
        },
        LayoutItem {
            preferred: [40., 10.],
            baseline: 8.,
            ..Default::default()
        },
        LayoutItem {
            absolute: true,
            anchor: [1.; 2],
            ..LayoutItem::fixed([5.; 2])
        },
    ];
    let g = resolve_layout_extended(
        LayoutSpec {
            kind: LayoutKind::Row,
            size: [100., 0.],
            gap: [10.; 2],
            align: LayoutAlign::Baseline,
            ..Default::default()
        },
        &items,
        wrapping(),
        &[],
    )
    .unwrap();
    assert_eq!(g.rect(0).unwrap()[1] + 15., g.rect(1).unwrap()[1] + 8.);
    assert!(g.rect(2).unwrap()[1] > g.rect(0).unwrap()[3]);
    let r = g.rect(3).unwrap();
    assert_eq!([r[2], r[3]], g.size);
}
#[test]
fn spanning_tracks_include_minima_and_automatic_placement_reserves_explicit_cells() {
    let items = [
        LayoutItem::fixed([100., 30.]),
        LayoutItem::fixed([20., 70.]),
        LayoutItem::fixed([40., 20.]),
    ];
    let cells = [
        GridCell::spanning(1, 2),
        GridCell::at(0, 2, 2, 1),
        GridCell::default(),
    ];
    let g = resolve_layout_extended(
        LayoutSpec {
            kind: LayoutKind::Grid,
            columns: 3,
            gap: [5.; 2],
            ..Default::default()
        },
        &items,
        Default::default(),
        &cells,
    )
    .unwrap();
    let a = g.rect(0).unwrap();
    let b = g.rect(1).unwrap();
    let c = g.rect(2).unwrap();
    assert_eq!([a[2] - a[0], a[3] - a[1]], [100., 30.]);
    assert_eq!(b[3] - b[1], 70.);
    assert!(b[0] >= a[2] + 5.);
    assert!(c[1] >= a[3] + 5.);
    assert_eq!(c[0], 0.);
}
#[test]
fn overlapping_and_invalid_spans_are_rejected() {
    let items = [LayoutItem::fixed([10.; 2]); 2];
    let spec = LayoutSpec {
        kind: LayoutKind::Grid,
        columns: 2,
        ..Default::default()
    };
    for cells in [
        [GridCell::at(0, 0, 1, 2), GridCell::at(0, 1, 1, 1)],
        [GridCell::spanning(0, 1), GridCell::default()],
        [GridCell::at(63, 0, 2, 1), GridCell::default()],
    ] {
        assert!(matches!(
            resolve_layout_extended(spec, &items, Default::default(), &cells),
            Err(LayoutError::InvalidGeometry)
        ));
    }
    assert!(resolve_layout_extended(spec, &items, wrapping(), &[]).is_err());
    assert!(
        resolve_layout_extended(spec, &items, Default::default(), &[GridCell::default()]).is_err()
    );
}
#[test]
fn wrapping_respects_an_intrinsic_maximum_and_oversized_minimum() {
    let items = [LayoutItem::fixed([80., 20.]); 3];
    let g = resolve_layout_extended(
        LayoutSpec {
            kind: LayoutKind::Row,
            max: [120., 1000.],
            gap: [10.; 2],
            ..Default::default()
        },
        &items,
        wrapping(),
        &[],
    )
    .unwrap();
    assert_eq!(g.size, [120., 80.]);
    assert_eq!(g.rect(2), Some([0., 60., 80., 80.]));
    let g = resolve_layout_extended(
        LayoutSpec {
            kind: LayoutKind::Row,
            size: [40., 0.],
            ..Default::default()
        },
        &items[..1],
        wrapping(),
        &[],
    )
    .unwrap();
    assert_eq!(g.size, [40., 20.]);
    assert_eq!(g.rect(0), Some([0., 0., 80., 20.]));
}
