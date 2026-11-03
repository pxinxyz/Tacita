use restricted_gui::{
    context::Context,
    layout::{GridCell, LayoutAlign, LayoutItem, LayoutKind, LayoutOptions, LayoutSpec},
    paths::{DashPattern, LineCap, LineJoin, StrokeStyle},
    ScrollFlags,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counter;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;

fn frame(ctx: &mut Context) {
    ctx.new_frame(0.016);
    ctx.ui().window("paths/layout", ScrollFlags::NONE, |ui| {
        let points = [[100., 100.], [160., 100.], [190., 145.], [140., 180.]];
        for join in [LineJoin::Miter, LineJoin::Bevel, LineJoin::Round] {
            for cap in [LineCap::Butt, LineCap::Square, LineCap::Round] {
                let style = StrokeStyle {
                    width: 6.,
                    join,
                    cap,
                    ..Default::default()
                };
                ui.draw_polyline(&points, false, style, [255; 4]).unwrap();
                ui.draw_dashed_polyline(
                    &points,
                    true,
                    style,
                    DashPattern {
                        lengths: &[15., 7., 3.],
                        phase: -5.,
                    },
                    [255; 4],
                )
                .unwrap();
                ui.draw_quadratic_bezier([points[0], points[2], points[1]], style, [255; 4])
                    .unwrap();
                ui.draw_cubic_bezier(points, style, [255; 4]).unwrap();
            }
        }
        ui.draw_concave_polygon(
            &[
                [100., 200.],
                [180., 200.],
                [180., 230.],
                [130., 230.],
                [130., 280.],
                [100., 280.],
            ],
            [255; 4],
        )
        .unwrap();
        let items = [LayoutItem {
            preferred: [80., 30.],
            grow: 1.,
            ..Default::default()
        }; 3];
        let wrap = LayoutSpec {
            kind: LayoutKind::Row,
            size: [180., 80.],
            align: LayoutAlign::Stretch,
            ..Default::default()
        };
        let cells = [
            GridCell::at(0, 0, 1, 2),
            GridCell::at(0, 2, 2, 1),
            GridCell::default(),
        ];
        for (spec, options, cells) in [
            (
                wrap,
                LayoutOptions {
                    wrap: true,
                    ..Default::default()
                },
                &[][..],
            ),
            (
                LayoutSpec {
                    kind: LayoutKind::Grid,
                    columns: 3,
                    ..wrap
                },
                LayoutOptions::default(),
                &cells[..],
            ),
        ] {
            ui.layout_extended(
                if spec.kind == LayoutKind::Grid {
                    "grid"
                } else {
                    "wrap"
                },
                spec,
                &items,
                options,
                cells,
                |ui, layout| {
                    for i in 0..3 {
                        ui.layout_item(layout, i, |ui, _| {
                            ui.button("item");
                        })
                        .unwrap();
                    }
                },
            )
            .unwrap();
        }
    });
    ctx.render();
}
#[test]
fn warmed_paths_and_extended_scopes_allocate_nothing() {
    let mut ctx = Context::new(include_bytes!("assets/DejaVuSans.ttf")).unwrap();
    for _ in 0..8 {
        frame(&mut ctx);
    }
    let before = ALLOCS.load(Ordering::Relaxed);
    for _ in 0..64 {
        frame(&mut ctx);
    }
    assert_eq!(
        ALLOCS.load(Ordering::Relaxed) - before,
        0,
        "warmed path scratch and layout declarations must retain capacity"
    );
}
