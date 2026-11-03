#![no_main]
use libfuzzer_sys::fuzz_target;
use restricted_gui::persistence::LayoutSnapshot;
fuzz_target!(|data: &[u8]| {
    if data.len() > 65536 {
        return;
    }
    if let Ok(layout) = LayoutSnapshot::decode(data) {
        assert!(layout.is_valid());
        let encoded = layout.encode().unwrap();
        assert_eq!(LayoutSnapshot::decode(&encoded).unwrap(), layout);
    }
});
