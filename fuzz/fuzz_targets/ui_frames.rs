#![no_main]

use libfuzzer_sys::fuzz_target;
use tacita::context::Context;
use tacita::input::{InputState, Key, KeyEvent};
use tacita::ScrollFlags;

const FONT: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");
const ITEMS: [&str; 12] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
];

fuzz_target!(|data: &[u8]| {
    if data.len() > 256 {
        return;
    }
    let Ok(mut ctx) = Context::new(FONT) else {
        return;
    };
    let mut checked = false;
    let mut selected = 0;
    let mut text = String::new();

    for frame in data.chunks(8).take(16) {
        let byte = |i: usize| frame.get(i).copied().unwrap_or_default();
        let keys = [
            if byte(0) & 0x10 != 0 {
                KeyEvent::Press(Key::Tab)
            } else {
                KeyEvent::Release(Key::Tab)
            },
            KeyEvent::Char(char::from(b'a' + byte(1) % 26)),
        ];
        ctx.set_input(InputState {
            mouse_pos: [byte(2) as f32 * 2.0, byte(3) as f32 * 2.0],
            mouse_down: [byte(0) & 1 != 0, false, false, false, false],
            mouse_wheel: (byte(4) as i8 as f32) / 32.0,
            keys: &keys,
            viewport_size: [640.0, 480.0],
        });
        ctx.new_frame(1.0 / 144.0);
        ctx.ui().window("fuzz", ScrollFlags::NONE, |ui| {
            ui.checkbox("check", &mut checked);
            ui.text_input("text", &mut text);
            ui.list_box("items", &ITEMS, &mut selected, [180.0, 100.0]);
            ui.draw_line(
                [byte(5) as f32, byte(6) as f32],
                [byte(6) as f32, byte(7) as f32],
                [byte(0), byte(1), byte(2), 255],
                1.0 + (byte(3) % 8) as f32,
            );
        });
        let _ = ctx.render();
    }
});
