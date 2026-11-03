#![no_main]
use libfuzzer_sys::fuzz_target;
use restricted_gui::persistence::Config;
fuzz_target!(|data: &[u8]| {
    if data.len() > 65536 {
        return;
    }
    if let Ok(config) = Config::decode(data) {
        let encoded = config.encode();
        let again = Config::decode(&encoded).expect("encoder emits valid config");
        assert_eq!(again.encode(), encoded);
    }
    let mut registered = Config::new();
    registered
        .register("enabled", restricted_gui::persistence::Value::Bool(true))
        .unwrap();
    let before = registered.encode();
    if registered.load(data).is_err() {
        assert_eq!(registered.encode(), before);
    }
});
