use super::*;
use alloc::string::String;
#[test]
fn history_groups_and_truncates_redo_at_capacity() {
    let mut h = History::new(0, 3).unwrap();
    h.record(1);
    h.begin_group();
    h.record(2);
    h.record(3);
    h.end_group();
    assert_eq!(h.undo(), Some(&1));
    assert_eq!(h.redo(), Some(&3));
    h.record(4);
    assert_eq!(h.undo(), Some(&3));
    assert_eq!(h.undo(), Some(&1));
    assert_eq!(h.undo(), None);
    h.record(7);
    assert_eq!(h.redo(), None);
}
#[test]
fn config_roundtrip_is_transactional_and_schema_checked() {
    let mut c = Config::new();
    c.register("enabled", Value::Bool(true)).unwrap();
    c.register("name", Value::Text(String::from("é"))).unwrap();
    let b = c.encode();
    let mut other = c.clone();
    other.set("enabled", Value::Bool(false)).unwrap();
    for end in 0..b.len() {
        let before = other.clone();
        assert!(other.load(&b[..end]).is_err());
        assert_eq!(other, before);
    }
    other.load(&b).unwrap();
    assert_eq!(other, c);
    assert!(other.set("enabled", Value::Number(f64::NAN)).is_err());
    let mut extra = b.clone();
    extra.push(0);
    assert!(other.load(&extra).is_err());
    let empty = Config::new().encode();
    assert!(other.load(&empty).is_err());
}
#[test]
fn theme_text_rejects_duplicates_unknowns_missing_and_bad_numbers() {
    let t = crate::theme::ThemeTokens::default();
    let text = encode_theme(t).unwrap();
    assert_eq!(decode_theme(text.as_bytes()).unwrap(), t);
    assert!(decode_theme(format_text(&text, "\ncorner_rounding=2\n").as_bytes()).is_err());
    assert!(decode_theme(text.replace("border_size=1", "border_size=NaN").as_bytes()).is_err());
    assert!(decode_theme(b"version=2").is_err());
    assert!(decode_theme(b"version=1").is_err());
}
fn format_text(a: &str, b: &str) -> String {
    let mut s = String::from(a);
    s.push_str(b);
    s
}
#[test]
fn layout_roundtrips_windows_and_named_panel_values() {
    let mut c = crate::widgets::test_support::test_context();
    c.set_window_rect("gallery", [80., 90.], [500., 600.]);
    let mut layout = c.save_layout();
    layout
        .set_panel(
            "split",
            PanelState::Split {
                axis: crate::panels::SplitAxis::Horizontal,
                ratio: 0.4,
            },
        )
        .unwrap();
    layout
        .set_panel(
            "dock",
            PanelState::Dock(crate::panels::DockState::default()),
        )
        .unwrap();
    let bytes = layout.encode().unwrap();
    let decoded = LayoutSnapshot::decode(&bytes).unwrap();
    assert_eq!(layout, decoded);
    c.set_window_rect("gallery", [0., 0.], [200., 200.]);
    c.load_layout(&decoded).unwrap();
    assert_eq!(c.save_layout().windows, layout.windows);
    for end in 0..bytes.len() {
        assert!(LayoutSnapshot::decode(&bytes[..end]).is_err());
    }
    let mut invalid = decoded;
    invalid.windows[0].size[0] = f32::NAN;
    let before = c.save_layout();
    assert!(c.load_layout(&invalid).is_err());
    assert_eq!(c.save_layout(), before);
}
