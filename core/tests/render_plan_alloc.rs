use tacita::{
    draw::{DrawCmd, DrawData, DrawVert, TextureId},
    rendering::{CommandPlan, DrawOptimization},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Counter;
static COUNT: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOC: Counter = Counter;
#[test]
fn warmed_planning_and_quad_extraction_reuse_all_storage() {
    let vertices: Vec<_> = (0..128)
        .flat_map(|i| {
            let x = i as f32 * 16.;
            [[x, 0.], [x + 8., 0.], [x + 8., 8.], [x, 8.]].map(|pos| DrawVert {
                pos,
                uv: [0.; 2],
                col: [255; 4],
            })
        })
        .collect();
    let indices: Vec<_> = (0..128)
        .flat_map(|i| [0, 1, 2, 0, 2, 3].map(|v| i * 4 + v))
        .collect();
    let commands: Vec<_> = (0..128)
        .map(|i| DrawCmd {
            elem_count: 6,
            clip_rect: [0., 0., 4096., 128.],
            texture: TextureId(2 + i % 3),
            effects: 0,
        })
        .collect();
    let draw = DrawData {
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
        effects: &[],
    };
    let mut plan = CommandPlan::default();
    let options = DrawOptimization {
        sort_disjoint: true,
        instance_quads: true,
    };
    plan.prepare(&draw, options, 1.5).unwrap();
    let before = COUNT.load(Ordering::Relaxed);
    for _ in 0..64 {
        plan.prepare(&draw, options, 1.5).unwrap();
    }
    assert_eq!(COUNT.load(Ordering::Relaxed) - before, 0);
    assert_eq!(plan.instances().len(), 128);
}
