#![no_main]

use libfuzzer_sys::fuzz_target;
use tacita::font::Font;

const VALID_FONT: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

fuzz_target!(|data: &[u8]| {
    if data.len() > 4096 {
        return;
    }

    let _ = Font::new(data, 16.0);
    if data.len() < 2 || data[0] & 1 == 0 {
        return;
    }

    let mut bytes = VALID_FONT.to_vec();
    for chunk in data[1..].as_chunks::<5>().0.iter().take(16) {
        let offset =
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as usize % bytes.len();
        bytes[offset] = chunk[4];
    }
    let _ = Font::new(&bytes, 16.0);
});
