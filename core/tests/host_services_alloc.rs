use tacita::{context::Context, input::InputState, ScrollFlags};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
struct Counting;
static COUNT: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(l) }
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
static ALLOC: Counting = Counting;
#[test]
fn warmed_semantics_diagnostics_and_host_requests_allocate_nothing() {
    let mut c = Context::new(include_bytes!("assets/DejaVuSans.ttf")).unwrap();
    c.set_accessibility_enabled(true);
    c.set_duplicate_id_diagnostics(true);
    c.set_hit_target_policy(tacita::host_requests::HitTargetPolicy { minimum: [32.; 2] });
    let mut checked = false;
    let mut value = 5.;
    let mut text = String::from("text");
    let mut name = String::with_capacity(256);
    let mut semantic_value = String::with_capacity(256);
    for frame in 0..30 {
        let before = COUNT.load(Ordering::Relaxed);
        c.set_input(InputState::default());
        c.new_frame(0.016);
        c.ui().window("root", ScrollFlags::NONE, |ui| {
            ui.pill_toggle("toggle", &mut checked);
            ui.slider_float("slider", &mut value, 0.0..=10.0);
            ui.text_input("input", &mut text);
            ui.button("duplicate");
            ui.button("duplicate");
        });
        c.render();
        for i in 0..c.accessibility_count() {
            let n = c.accessibility_node(i).unwrap();
            c.accessibility_name(n.id, &mut name);
            c.accessibility_value(n.id, &mut semantic_value);
        }
        let after = COUNT.load(Ordering::Relaxed);
        if frame > 12 {
            assert_eq!(after - before, 0, "frame {frame} allocated");
        }
        assert_eq!(c.duplicate_ids().len(), 1);
    }
}
