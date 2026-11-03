use restricted_gui::{
    context::Context,
    diagnostics::InputRecording,
    draw::TextureId,
    images::{ImageFit, ImageRegion, NineSlice},
    input::{InputState, KeyEvent},
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

fn frame(ctx: &mut Context, recording: &InputRecording, name: &mut String) {
    ctx.replay_frame(recording.frame(0).unwrap());
    let image = ImageRegion {
        texture: TextureId(5),
        uv_min: [0.; 2],
        uv_max: [1.; 2],
        size: [32.; 2],
    };
    ctx.ui().window("trace", ScrollFlags::NONE, |ui| {
        ui.label("label");
        ui.image_button("sprite", image, [96., 32.], ImageFit::Contain, [255; 4])
            .unwrap();
        ui.draw_nine_slice(
            image,
            [120., 180., 240., 220.],
            NineSlice {
                source: [6.; 4],
                destination: [6.; 4],
            },
            [255; 4],
        )
        .unwrap();
    });
    ctx.render();
    for i in 0..ctx.inspection_count() {
        let node = ctx.inspection_node(i).unwrap();
        ctx.inspection_name(node.id, name);
    }
}
#[test]
fn warmed_replay_images_and_inspection_allocate_nothing() {
    let mut ctx = Context::new(include_bytes!("assets/DejaVuSans.ttf")).unwrap();
    ctx.set_inspection_enabled(true);
    let mut recording = InputRecording::new();
    let keys = [KeyEvent::Char('x')];
    recording
        .push(
            InputState {
                mouse_pos: [-100.; 2],
                keys: &keys,
                ..Default::default()
            },
            0.016,
            &[],
        )
        .unwrap();
    let mut name = String::with_capacity(256);
    for _ in 0..8 {
        frame(&mut ctx, &recording, &mut name);
    }
    let before = ALLOCS.load(Ordering::Relaxed);
    for _ in 0..64 {
        frame(&mut ctx, &recording, &mut name);
    }
    let after = ALLOCS.load(Ordering::Relaxed);
    assert_eq!(
        after - before,
        0,
        "warmed diagnostics and image drawing must not allocate"
    );
}
