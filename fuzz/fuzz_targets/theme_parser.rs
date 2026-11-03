#![no_main]
use libfuzzer_sys::fuzz_target;
use restricted_gui::persistence::{decode_theme, encode_theme};
fuzz_target!(|data: &[u8]| {
    if data.len() > 16384 {
        return;
    }
    if let Ok(tokens) = decode_theme(data) {
        assert!(tokens.is_valid());
        let encoded = encode_theme(tokens).unwrap();
        assert_eq!(decode_theme(encoded.as_bytes()).unwrap(), tokens);
    }
});
