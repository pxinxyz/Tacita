use tacita::{
    context::Context,
    editing::{TextEditOptions, TextEditor},
    font::{ShapedOptions, ShapedText, TextRasterMode},
    input::InputState,
    ScrollFlags,
};
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
fn warmed_unicode_editor_selection_and_display_allocate_nothing() {
    let mut c = Context::new(include_bytes!("assets/DejaVuSans.ttf")).unwrap();
    let emoji = c
        .add_font(include_bytes!("assets/Noto-COLRv1.ttf"), 16.)
        .unwrap();
    c.set_font_fallback_chain(tacita::font::FontId::DEFAULT, &[emoji])
        .unwrap();
    c.set_color_atlas_texture(Some(tacita::draw::TextureId(90)))
        .unwrap();
    c.set_text_raster_mode(TextRasterMode::Subpixel);
    let mut e = TextEditor::new(
        "office e\u{301} سلام abc אבג xyz 👩‍💻\nSecond line 🇬🇧\nThird line 👍🏽\nFourth line",
    )
    .unwrap();
    e.select_all();
    let mut layout = ShapedText::default();
    for frame in 0..40 {
        let before = COUNT.load(Ordering::Relaxed);
        c.set_input(InputState::default());
        c.new_frame(0.016);
        {
            let mut ui = c.ui();
            ui.window("text", ScrollFlags::NONE, |ui| {
                let id = ui.widget_id("editor");
                ui.focus(id);
                ui.text_edit(
                    "editor",
                    &mut e,
                    TextEditOptions {
                        size: [220., 70.],
                        multiline: true,
                        wrap: true,
                        ..Default::default()
                    },
                )
                .unwrap();
                ui.shaped_text(
                    &mut layout,
                    "office — سلام — אבג 😀 🚀",
                    ShapedOptions::default(),
                )
                .unwrap();
            });
        }
        let data = c.render();
        let nonempty = !data.vertices.is_empty();
        let after = COUNT.load(Ordering::Relaxed);
        assert!(nonempty);
        if frame >= 12 {
            assert_eq!(after - before, 0, "frame {frame} allocated");
        }
    }
}
