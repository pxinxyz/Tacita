use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
struct Count;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: Count = Count;
use tacita::{
    context::Context,
    id::Id,
    input::{InputState, Key, KeyEvent, Modifiers},
    interaction::{Navigation, Shortcut, ShortcutScope},
    ScrollFlags,
};
fn frame(c: &mut Context, step: usize) {
    c.navigate(Navigation::Next);
    let keys = if step % 4 == 2 {
        &[KeyEvent::Press(Key::K)][..]
    } else {
        &[]
    };
    c.set_input(InputState {
        keys,
        viewport_size: [900., 700.],
        ..Default::default()
    });
    c.new_frame(0.016);
    c.ui().window("W", ScrollFlags::NONE, |ui| {
        if !ui.is_popup_open(Id(1)) {
            ui.open_popup(Id(1));
        }
        ui.popup(Id(1), [100., 100., 300., 300.], |ui| {
            ui.button("parent");
            if step % 4 != 3 && !ui.is_popup_open(Id(2)) {
                ui.open_modal(Id(2));
            }
            ui.modal(Id(2), [350., 200., 650., 500.], |ui| {
                ui.button("child");
                ui.shortcut(
                    Id(3),
                    Shortcut {
                        key: Key::K,
                        modifiers: Modifiers::default(),
                        scope: ShortcutScope::Popup(Id(2)),
                    },
                );
                if step % 4 == 3 {
                    ui.close_popup();
                }
            });
        });
    });
    c.render();
}
#[test]
fn repeated_nesting_close_reopen_navigation_and_shortcuts_allocate_zero() {
    let mut c = Context::new(include_bytes!("assets/DejaVuSans.ttf")).unwrap();
    c.set_motion_settings(tacita::animation::MotionSettings {
        enabled: true,
        ..Default::default()
    });
    for i in 0..128 {
        frame(&mut c, i);
    }
    let before = ALLOCS.load(Ordering::Relaxed);
    for i in 0..128 {
        frame(&mut c, i);
    }
    let allocated = ALLOCS.load(Ordering::Relaxed) - before;
    assert_eq!(allocated, 0);
}
