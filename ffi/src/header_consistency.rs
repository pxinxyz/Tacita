use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{GuiErrorCode, ALL_KEYS};

fn manifest_dir() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn strip_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let end = text[i + 2..]
                .find("*/")
                .unwrap_or_else(|| panic!("unterminated block comment at byte {i}"));
            i += 2 + end + 2;
        } else if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let end = text[i..].find('\n').unwrap_or(text.len() - i);
            i += end;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn declared_functions(header: &str) -> Vec<String> {
    let bytes = header.as_bytes();
    let mut found: Vec<String> = Vec::new();
    let mut i = 0;
    while i + 4 < bytes.len() {
        if &bytes[i..i + 4] == b"gui_" && (i == 0 || !is_ident_char(bytes[i - 1])) {
            let mut end = i + 4;
            while end < bytes.len() && is_ident_char(bytes[end]) {
                end += 1;
            }
            let mut next = end;
            while next < bytes.len() && (bytes[next] as char).is_whitespace() {
                next += 1;
            }
            if next < bytes.len() && bytes[next] == b'(' {
                let name = header[i..end].to_string();
                if !found.contains(&name) {
                    found.push(name);
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    found
}

fn exported_functions() -> Vec<String> {
    let src = PathBuf::from(manifest_dir()).join("src");
    let mut files: Vec<PathBuf> = fs::read_dir(&src)
        .expect("src dir readable")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    let mut found = Vec::new();
    for file in files {
        if file
            .file_name()
            .is_some_and(|name| name == "header_consistency.rs")
        {
            continue;
        }
        let text = read(&file);
        for needle in text.match_indices("extern \"C\" fn ") {
            let rest = &text[needle.0 + "extern \"C\" fn ".len()..];
            let name: String = rest
                .chars()
                .take_while(|&c| c.is_ascii_alphanumeric() || c == '_')
                .collect();
            assert!(
                name.starts_with("gui_"),
                "{name} in {} is not an export",
                file.display()
            );
            found.push(name);
        }
    }
    found
}

fn parse_integer_literal(token: &str, line: &str) -> u64 {
    let digits = token.trim().trim_end_matches(['u', 'U', 'l', 'L']);
    digits
        .parse()
        .unwrap_or_else(|_| panic!("non-integer #define value: {line}"))
}

fn parse_define_value(expr: &str, line: &str) -> u64 {
    let expr = expr.trim();
    let expr = expr
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .unwrap_or(expr);
    match expr.split_once("<<") {
        Some((lhs, rhs)) => parse_integer_literal(lhs, line) << parse_integer_literal(rhs, line),
        None => parse_integer_literal(expr, line),
    }
}

fn header_defines(header: &str) -> BTreeMap<String, u64> {
    let mut defines = BTreeMap::new();
    for line in header.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("#define") {
            let tokens: Vec<&str> = rest.split_whitespace().collect();
            if tokens.len() < 2 || !tokens[0].starts_with("GUI_") {
                continue;
            }
            let value = parse_define_value(&tokens[1..].join(" "), line);
            defines.insert(tokens[0].to_string(), value);
        }
    }
    defines
}

#[test]
fn header_declares_exactly_the_exported_functions() {
    let header = strip_comments(&read(
        &PathBuf::from(manifest_dir()).join("tacita.h"),
    ));
    let mut declared = declared_functions(&header);
    let mut exported = exported_functions();
    declared.sort();
    exported.sort();
    let missing_from_impl: Vec<_> = declared
        .iter()
        .filter(|name| !exported.contains(name))
        .collect();
    let missing_from_header: Vec<_> = exported
        .iter()
        .filter(|name| !declared.contains(name))
        .collect();
    assert!(
        missing_from_impl.is_empty() && missing_from_header.is_empty(),
        "header/impl drift — declared but not exported: {missing_from_impl:?}; \
         exported but not declared: {missing_from_header:?}"
    );
    assert_eq!(
        declared.len(),
        329,
        "the export surface changed; update the header"
    );
}

#[test]
fn header_constants_match_the_implementation_values() {
    let header = strip_comments(&read(
        &PathBuf::from(manifest_dir()).join("tacita.h"),
    ));
    let defines = header_defines(&header);

    let mut expected: BTreeMap<String, u64> = BTreeMap::new();
    for (name, value) in [
        ("GUI_RENDER_SORT_DISJOINT", crate::GUI_RENDER_SORT_DISJOINT),
        (
            "GUI_RENDER_INSTANCE_QUADS",
            crate::GUI_RENDER_INSTANCE_QUADS,
        ),
        ("GUI_GEOMETRY_TRIANGLES", crate::GUI_GEOMETRY_TRIANGLES),
        ("GUI_GEOMETRY_QUADS", crate::GUI_GEOMETRY_QUADS),
        (
            "GUI_ALPHA_INPUT_PREMULTIPLIED",
            crate::GUI_ALPHA_INPUT_PREMULTIPLIED,
        ),
        (
            "GUI_ALPHA_OUTPUT_PREMULTIPLIED",
            crate::GUI_ALPHA_OUTPUT_PREMULTIPLIED,
        ),
        (
            "GUI_ALPHA_SECONDARY_PREMULTIPLIED",
            crate::GUI_ALPHA_SECONDARY_PREMULTIPLIED,
        ),
    ] {
        expected.insert(name.into(), value as u64);
    }
    expected.insert(
        "GUI_CURVE_MAX_KEYS".into(),
        crate::GUI_CURVE_MAX_KEYS as u64,
    );
    expected.insert(
        "GUI_GRADIENT_EDITOR_MAX_STOPS".into(),
        crate::GUI_GRADIENT_EDITOR_MAX_STOPS as u64,
    );
    expected.insert("GUI_CURVE_STEP".into(), crate::GUI_CURVE_STEP as u64);
    expected.insert("GUI_CURVE_LINEAR".into(), crate::GUI_CURVE_LINEAR as u64);
    expected.insert("GUI_CURVE_CUBIC".into(), crate::GUI_CURVE_CUBIC as u64);
    expected.insert(
        "GUI_PLOT_AUTO_RANGE".into(),
        crate::GUI_PLOT_AUTO_RANGE as u64,
    );
    expected.insert("GUI_PLOT_AXES".into(), crate::GUI_PLOT_AXES as u64);
    expected.insert(
        "GUI_DATA_MAX_COLUMNS".into(),
        crate::GUI_DATA_MAX_COLUMNS as u64,
    );
    expected.insert("GUI_DATA_MAX_ROWS".into(), crate::GUI_DATA_MAX_ROWS as u64);
    expected.insert("GUI_SORT_NONE".into(), crate::GUI_SORT_NONE as u64);
    expected.insert(
        "GUI_SORT_ASCENDING".into(),
        crate::GUI_SORT_ASCENDING as u64,
    );
    expected.insert(
        "GUI_SORT_DESCENDING".into(),
        crate::GUI_SORT_DESCENDING as u64,
    );
    expected.insert("GUI_SURFACE_NOISE".into(), crate::GUI_SURFACE_NOISE as u64);
    expected.insert("GUI_SURFACE_GRAIN".into(), crate::GUI_SURFACE_GRAIN as u64);
    expected.insert(
        "GUI_SURFACE_DITHER".into(),
        crate::GUI_SURFACE_DITHER as u64,
    );
    expected.insert("GUI_SURFACE_GRADE".into(), crate::GUI_SURFACE_GRADE as u64);
    expected.insert(
        "GUI_SURFACE_DESATURATE".into(),
        crate::GUI_SURFACE_DESATURATE as u64,
    );
    expected.insert(
        "GUI_SURFACE_VIGNETTE".into(),
        crate::GUI_SURFACE_VIGNETTE as u64,
    );
    expected.insert(
        "GUI_SURFACE_CHROMATIC".into(),
        crate::GUI_SURFACE_CHROMATIC as u64,
    );
    expected.insert(
        "GUI_SURFACE_SCANLINES".into(),
        crate::GUI_SURFACE_SCANLINES as u64,
    );
    expected.insert("GUI_SURFACE_GRID".into(), crate::GUI_SURFACE_GRID as u64);
    expected.insert("GUI_SURFACE_DOTS".into(), crate::GUI_SURFACE_DOTS as u64);
    expected.insert(
        "GUI_SURFACE_SHIMMER".into(),
        crate::GUI_SURFACE_SHIMMER as u64,
    );
    expected.insert("GUI_SURFACE_SWEEP".into(), crate::GUI_SURFACE_SWEEP as u64);
    expected.insert(
        "GUI_SURFACE_DISTORT".into(),
        crate::GUI_SURFACE_DISTORT as u64,
    );
    expected.insert("GUI_SURFACE_GLASS".into(), crate::GUI_SURFACE_GLASS as u64);
    expected.insert("GUI_SURFACE_ALL".into(), crate::GUI_SURFACE_ALL as u64);
    expected.insert("GUI_DITHER_BAYER".into(), crate::GUI_DITHER_BAYER as u64);
    expected.insert(
        "GUI_DITHER_BLUE_NOISE".into(),
        crate::GUI_DITHER_BLUE_NOISE as u64,
    );
    expected.insert(
        "GUI_BLUR_SEPARABLE".into(),
        crate::GUI_BLUR_SEPARABLE as u64,
    );
    expected.insert(
        "GUI_BLUR_DUAL_KAWASE".into(),
        crate::GUI_BLUR_DUAL_KAWASE as u64,
    );
    expected.insert(
        "GUI_MAX_FILTER_PASSES".into(),
        crate::GUI_MAX_FILTER_PASSES as u64,
    );
    expected.insert("GUI_FILTER_COPY".into(), crate::GUI_FILTER_COPY as u64);
    expected.insert(
        "GUI_FILTER_GAUSSIAN_X".into(),
        crate::GUI_FILTER_GAUSSIAN_X as u64,
    );
    expected.insert(
        "GUI_FILTER_GAUSSIAN_Y".into(),
        crate::GUI_FILTER_GAUSSIAN_Y as u64,
    );
    expected.insert(
        "GUI_FILTER_KAWASE_DOWN".into(),
        crate::GUI_FILTER_KAWASE_DOWN as u64,
    );
    expected.insert(
        "GUI_FILTER_KAWASE_UP".into(),
        crate::GUI_FILTER_KAWASE_UP as u64,
    );
    expected.insert(
        "GUI_FILTER_THRESHOLD".into(),
        crate::GUI_FILTER_THRESHOLD as u64,
    );
    expected.insert(
        "GUI_FILTER_BLOOM_COMPOSE".into(),
        crate::GUI_FILTER_BLOOM_COMPOSE as u64,
    );
    expected.insert("GUI_ERROR_NONE".into(), GuiErrorCode::None as u64);
    expected.insert(
        "GUI_ERROR_CONTRACT_VIOLATION".into(),
        GuiErrorCode::ContractViolation as u64,
    );
    expected.insert("GUI_ERROR_PANIC".into(), GuiErrorCode::Panic as u64);
    expected.insert(
        "GUI_ERROR_INVALID_INPUT".into(),
        GuiErrorCode::InvalidInput as u64,
    );
    expected.insert(
        "GUI_KEY_EVENT_PRESS".into(),
        crate::GUI_KEY_EVENT_PRESS as u64,
    );
    expected.insert(
        "GUI_KEY_EVENT_RELEASE".into(),
        crate::GUI_KEY_EVENT_RELEASE as u64,
    );
    expected.insert(
        "GUI_KEY_EVENT_CHAR".into(),
        crate::GUI_KEY_EVENT_CHAR as u64,
    );
    expected.insert(
        "GUI_KEY_BINDING_NONE".into(),
        crate::GUI_KEY_BINDING_NONE as u64,
    );
    expected.insert(
        "GUI_KEY_BINDING_KEY".into(),
        crate::GUI_KEY_BINDING_KEY as u64,
    );
    expected.insert(
        "GUI_KEY_BINDING_MOUSE".into(),
        crate::GUI_KEY_BINDING_MOUSE as u64,
    );
    expected.insert(
        "GUI_KEY_BINDING_CHORD".into(),
        crate::GUI_KEY_BINDING_CHORD as u64,
    );
    expected.insert(
        "GUI_FONT_ID_INVALID".into(),
        crate::GUI_FONT_ID_INVALID as u64,
    );
    expected.insert("GUI_NAV_NEXT".into(), crate::GUI_NAV_NEXT as u64);
    expected.insert("GUI_NAV_PREVIOUS".into(), crate::GUI_NAV_PREVIOUS as u64);
    expected.insert("GUI_NAV_LEFT".into(), crate::GUI_NAV_LEFT as u64);
    expected.insert("GUI_NAV_RIGHT".into(), crate::GUI_NAV_RIGHT as u64);
    expected.insert("GUI_NAV_UP".into(), crate::GUI_NAV_UP as u64);
    expected.insert("GUI_NAV_DOWN".into(), crate::GUI_NAV_DOWN as u64);
    expected.insert("GUI_NAV_ACTIVATE".into(), crate::GUI_NAV_ACTIVATE as u64);
    expected.insert("GUI_NAV_CANCEL".into(), crate::GUI_NAV_CANCEL as u64);
    expected.insert(
        "GUI_SHORTCUT_GLOBAL".into(),
        crate::GUI_SHORTCUT_GLOBAL as u64,
    );
    expected.insert(
        "GUI_SHORTCUT_WINDOW".into(),
        crate::GUI_SHORTCUT_WINDOW as u64,
    );
    expected.insert(
        "GUI_SHORTCUT_POPUP".into(),
        crate::GUI_SHORTCUT_POPUP as u64,
    );
    expected.insert("GUI_MOD_CTRL".into(), crate::GUI_MOD_CTRL as u64);
    expected.insert("GUI_MOD_SHIFT".into(), crate::GUI_MOD_SHIFT as u64);
    expected.insert("GUI_MOD_ALT".into(), crate::GUI_MOD_ALT as u64);
    expected.insert("GUI_VALUE_BOOL".into(), crate::GUI_VALUE_BOOL as u64);
    expected.insert("GUI_VALUE_INTEGER".into(), crate::GUI_VALUE_INTEGER as u64);
    expected.insert("GUI_VALUE_NUMBER".into(), crate::GUI_VALUE_NUMBER as u64);
    expected.insert("GUI_VALUE_COLOR".into(), crate::GUI_VALUE_COLOR as u64);
    expected.insert("GUI_VALUE_TEXT".into(), crate::GUI_VALUE_TEXT as u64);
    expected.insert("GUI_PANEL_RECT".into(), crate::GUI_PANEL_RECT as u64);
    expected.insert("GUI_PANEL_SIZE".into(), crate::GUI_PANEL_SIZE as u64);
    expected.insert("GUI_PANEL_SPLIT".into(), crate::GUI_PANEL_SPLIT as u64);
    expected.insert("GUI_PANEL_DOCK".into(), crate::GUI_PANEL_DOCK as u64);
    expected.insert("GUI_PANEL_SCROLL".into(), crate::GUI_PANEL_SCROLL as u64);
    expected.insert(
        "GUI_RELOAD_WAITING".into(),
        crate::GUI_RELOAD_WAITING as u64,
    );
    expected.insert(
        "GUI_RELOAD_UNCHANGED".into(),
        crate::GUI_RELOAD_UNCHANGED as u64,
    );
    expected.insert(
        "GUI_RELOAD_APPLIED".into(),
        crate::GUI_RELOAD_APPLIED as u64,
    );
    expected.insert(
        "GUI_RELOAD_INVALID".into(),
        crate::GUI_RELOAD_INVALID as u64,
    );
    expected.insert("GUI_RELOAD_IO".into(), crate::GUI_RELOAD_IO as u64);
    expected.insert("GUI_HISTORY_BEGIN".into(), crate::GUI_HISTORY_BEGIN as u64);
    expected.insert("GUI_HISTORY_END".into(), crate::GUI_HISTORY_END as u64);
    expected.insert(
        "GUI_HISTORY_CANCEL".into(),
        crate::GUI_HISTORY_CANCEL as u64,
    );
    expected.insert(
        "GUI_TEXT_RASTER_LEGACY".into(),
        crate::GUI_TEXT_RASTER_LEGACY as u64,
    );
    expected.insert(
        "GUI_TEXT_RASTER_SUBPIXEL".into(),
        crate::GUI_TEXT_RASTER_SUBPIXEL as u64,
    );
    expected.insert("GUI_BORDER_INNER".into(), crate::GUI_BORDER_INNER as u64);
    expected.insert("GUI_BORDER_CENTER".into(), crate::GUI_BORDER_CENTER as u64);
    expected.insert("GUI_BORDER_OUTER".into(), crate::GUI_BORDER_OUTER as u64);
    expected.insert("GUI_PAINT_LINEAR".into(), crate::GUI_PAINT_LINEAR as u64);
    expected.insert("GUI_PAINT_RADIAL".into(), crate::GUI_PAINT_RADIAL as u64);
    expected.insert("GUI_PAINT_CONIC".into(), crate::GUI_PAINT_CONIC as u64);
    expected.insert(
        "GUI_MAX_PAINT_STOPS".into(),
        crate::GUI_MAX_PAINT_STOPS as u64,
    );
    expected.insert("GUI_MAX_MASKS".into(), crate::GUI_MAX_MASKS as u64);
    expected.insert("GUI_SDF_FILL".into(), crate::GUI_SDF_FILL as u64);
    expected.insert("GUI_SDF_BORDER".into(), crate::GUI_SDF_BORDER as u64);
    expected.insert("GUI_SDF_GLOW".into(), crate::GUI_SDF_GLOW as u64);
    expected.insert("GUI_SDF_EDGE_GLOW".into(), crate::GUI_SDF_EDGE_GLOW as u64);
    expected.insert("GUI_SDF_HIGHLIGHT".into(), crate::GUI_SDF_HIGHLIGHT as u64);
    expected.insert(
        "GUI_REVEAL_LEFT_TO_RIGHT".into(),
        crate::GUI_REVEAL_LEFT_TO_RIGHT as u64,
    );
    expected.insert(
        "GUI_REVEAL_RIGHT_TO_LEFT".into(),
        crate::GUI_REVEAL_RIGHT_TO_LEFT as u64,
    );
    expected.insert(
        "GUI_REVEAL_TOP_TO_BOTTOM".into(),
        crate::GUI_REVEAL_TOP_TO_BOTTOM as u64,
    );
    expected.insert(
        "GUI_REVEAL_BOTTOM_TO_TOP".into(),
        crate::GUI_REVEAL_BOTTOM_TO_TOP as u64,
    );
    expected.insert("GUI_REVEAL_RADIAL".into(), crate::GUI_REVEAL_RADIAL as u64);
    expected.insert(
        "GUI_EFFECT_UNIFORM_SIZE".into(),
        crate::GUI_EFFECT_UNIFORM_SIZE as u64,
    );
    for (name, value) in [
        ("GUI_SVG_ICON_INVALID", crate::GUI_SVG_ICON_INVALID),
        ("GUI_LAYOUT_ROW", crate::GUI_LAYOUT_ROW),
        ("GUI_LAYOUT_COLUMN", crate::GUI_LAYOUT_COLUMN),
        ("GUI_LAYOUT_GRID", crate::GUI_LAYOUT_GRID),
        ("GUI_LAYOUT_START", crate::GUI_LAYOUT_START),
        ("GUI_LAYOUT_CENTER", crate::GUI_LAYOUT_CENTER),
        ("GUI_LAYOUT_END", crate::GUI_LAYOUT_END),
        ("GUI_LAYOUT_STRETCH", crate::GUI_LAYOUT_STRETCH),
        ("GUI_LAYOUT_BASELINE", crate::GUI_LAYOUT_BASELINE),
        ("GUI_LAYOUT_JUSTIFY_START", crate::GUI_LAYOUT_JUSTIFY_START),
        (
            "GUI_LAYOUT_JUSTIFY_CENTER",
            crate::GUI_LAYOUT_JUSTIFY_CENTER,
        ),
        ("GUI_LAYOUT_JUSTIFY_END", crate::GUI_LAYOUT_JUSTIFY_END),
        ("GUI_LAYOUT_SPACE_BETWEEN", crate::GUI_LAYOUT_SPACE_BETWEEN),
        ("GUI_LAYOUT_MAX_ITEMS", crate::GUI_LAYOUT_MAX_ITEMS),
    ] {
        expected.insert(name.into(), value as u64);
    }
    expected.insert("GUI_JOIN_MITER".into(), crate::GUI_JOIN_MITER as u64);
    expected.insert("GUI_JOIN_BEVEL".into(), crate::GUI_JOIN_BEVEL as u64);
    expected.insert("GUI_JOIN_ROUND".into(), crate::GUI_JOIN_ROUND as u64);
    expected.insert("GUI_CAP_BUTT".into(), crate::GUI_CAP_BUTT as u64);
    expected.insert("GUI_CAP_SQUARE".into(), crate::GUI_CAP_SQUARE as u64);
    expected.insert("GUI_CAP_ROUND".into(), crate::GUI_CAP_ROUND as u64);
    expected.insert(
        "GUI_PATH_MAX_POINTS".into(),
        crate::GUI_PATH_MAX_POINTS as u64,
    );
    expected.insert(
        "GUI_POLYGON_MAX_POINTS".into(),
        crate::GUI_POLYGON_MAX_POINTS as u64,
    );
    expected.insert(
        "GUI_DASH_MAX_LENGTHS".into(),
        crate::GUI_DASH_MAX_LENGTHS as u64,
    );
    expected.insert("GUI_GRID_AUTO".into(), crate::GUI_GRID_AUTO as u64);
    expected.insert("GUI_CONTENT_START".into(), crate::GUI_CONTENT_START as u64);
    expected.insert(
        "GUI_CONTENT_CENTER".into(),
        crate::GUI_CONTENT_CENTER as u64,
    );
    expected.insert("GUI_CONTENT_END".into(), crate::GUI_CONTENT_END as u64);
    expected.insert(
        "GUI_CONTENT_STRETCH".into(),
        crate::GUI_CONTENT_STRETCH as u64,
    );
    expected.insert(
        "GUI_CONTENT_SPACE_BETWEEN".into(),
        crate::GUI_CONTENT_SPACE_BETWEEN as u64,
    );
    expected.insert("GUI_SCROLL_NONE".into(), crate::GUI_SCROLL_NONE as u64);
    expected.insert(
        "GUI_SCROLL_SCROLLABLE".into(),
        crate::GUI_SCROLL_SCROLLABLE as u64,
    );
    expected.insert(
        "GUI_SCROLL_SCROLLBAR".into(),
        crate::GUI_SCROLL_SCROLLBAR as u64,
    );
    expected.insert("GUI_VALIDATION_SUCCESS".into(), 0);
    expected.insert("GUI_VALIDATION_WARNING".into(), 1);
    expected.insert("GUI_VALIDATION_ERROR".into(), 2);
    expected.insert(
        "GUI_DOCK_LEFT".into(),
        tacita::panels::DockZone::Left as u64,
    );
    expected.insert(
        "GUI_DOCK_RIGHT".into(),
        tacita::panels::DockZone::Right as u64,
    );
    expected.insert(
        "GUI_DOCK_FLOATING".into(),
        tacita::panels::DockZone::Floating as u64,
    );
    expected.insert("GUI_SPLIT_HORIZONTAL".into(), 0);
    expected.insert("GUI_SPLIT_VERTICAL".into(), 1);
    for (name, value) in [
        ("GUI_EASING_LINEAR", crate::GUI_EASING_LINEAR),
        ("GUI_EASING_SMOOTH_STEP", crate::GUI_EASING_SMOOTH_STEP),
        ("GUI_EASING_SMOOTHER_STEP", crate::GUI_EASING_SMOOTHER_STEP),
        ("GUI_EASING_QUAD_IN", crate::GUI_EASING_QUAD_IN),
        ("GUI_EASING_QUAD_OUT", crate::GUI_EASING_QUAD_OUT),
        ("GUI_EASING_CUBIC_IN_OUT", crate::GUI_EASING_CUBIC_IN_OUT),
        ("GUI_EASING_CUBIC_OUT", crate::GUI_EASING_CUBIC_OUT),
    ] {
        expected.insert(name.into(), value as u64);
    }
    for (name, value) in [
        ("GUI_PLAYBACK_ONCE", crate::GUI_PLAYBACK_ONCE),
        ("GUI_PLAYBACK_LOOP", crate::GUI_PLAYBACK_LOOP),
        ("GUI_PLAYBACK_PING_PONG", crate::GUI_PLAYBACK_PING_PONG),
        ("GUI_GRADIENT_CLAMP", crate::GUI_GRADIENT_CLAMP),
        ("GUI_GRADIENT_REPEAT", crate::GUI_GRADIENT_REPEAT),
        ("GUI_GRADIENT_MIRROR", crate::GUI_GRADIENT_MIRROR),
    ] {
        expected.insert(name.into(), value as u64);
    }
    for (name, role) in [
        "BACKGROUND",
        "PANEL",
        "INPUT",
        "HOVER",
        "ACCENT",
        "TEXT",
        "TITLE_TEXT",
        "SCROLL_TRACK",
        "SCROLL_THUMB",
        "SCROLL_HOVER",
        "SCROLL_ACTIVE",
        "SECONDARY",
        "MUTED",
        "SUCCESS",
        "WARNING",
        "DANGER",
    ]
    .iter()
    .zip(tacita::theme::THEME_ROLES)
    {
        expected.insert(format!("GUI_THEME_{name}"), role as u64);
    }
    for (name, value) in [
        ("GUI_TEXT_ALIGN_START", crate::GUI_TEXT_ALIGN_START),
        ("GUI_TEXT_ALIGN_CENTER", crate::GUI_TEXT_ALIGN_CENTER),
        ("GUI_TEXT_ALIGN_END", crate::GUI_TEXT_ALIGN_END),
        ("GUI_TEXT_CLIP", crate::GUI_TEXT_CLIP),
        ("GUI_TEXT_TRUNCATE", crate::GUI_TEXT_TRUNCATE),
        ("GUI_TEXT_ELLIPSIS", crate::GUI_TEXT_ELLIPSIS),
    ] {
        expected.insert(name.into(), value as u64);
    }
    expected.insert(
        "GUI_EDIT_MAX_BYTES".into(),
        crate::GUI_EDIT_MAX_BYTES as u64,
    );
    expected.insert(
        "GUI_EDIT_MULTILINE".into(),
        crate::GUI_EDIT_MULTILINE as u64,
    );
    expected.insert("GUI_EDIT_WRAP".into(), crate::GUI_EDIT_WRAP as u64);
    expected.insert(
        "GUI_EDIT_READ_ONLY".into(),
        crate::GUI_EDIT_READ_ONLY as u64,
    );
    expected.insert(
        "GUI_DIRECTION_AUTO".into(),
        crate::GUI_DIRECTION_AUTO as u64,
    );
    expected.insert("GUI_DIRECTION_LTR".into(), crate::GUI_DIRECTION_LTR as u64);
    expected.insert("GUI_DIRECTION_RTL".into(), crate::GUI_DIRECTION_RTL as u64);
    expected.insert("GUI_IME_PREEDIT".into(), crate::GUI_IME_PREEDIT as u64);
    expected.insert("GUI_IME_COMMIT".into(), crate::GUI_IME_COMMIT as u64);
    expected.insert("GUI_IME_CANCEL".into(), crate::GUI_IME_CANCEL as u64);
    expected.insert(
        "GUI_CLIPBOARD_COPY".into(),
        crate::GUI_CLIPBOARD_COPY as u64,
    );
    expected.insert("GUI_CLIPBOARD_CUT".into(), crate::GUI_CLIPBOARD_CUT as u64);
    expected.insert(
        "GUI_CLIPBOARD_PASTE".into(),
        crate::GUI_CLIPBOARD_PASTE as u64,
    );
    expected.insert("GUI_EDIT_DOCUMENT".into(), crate::GUI_EDIT_DOCUMENT as u64);
    expected.insert(
        "GUI_EDIT_SELECTION".into(),
        crate::GUI_EDIT_SELECTION as u64,
    );
    expected.insert("GUI_EDIT_PREEDIT".into(), crate::GUI_EDIT_PREEDIT as u64);
    expected.insert(
        "GUI_IME_PREEDIT_HIDDEN".into(),
        crate::GUI_IME_PREEDIT_HIDDEN as u64,
    );
    for (code, key) in ALL_KEYS.iter().enumerate() {
        expected.insert(format!("GUI_KEY_{:?}", key).to_uppercase(), code as u64);
    }

    assert_eq!(
        defines, expected,
        "header constant drift (left: header, right: implementation)"
    );
}

#[test]
fn struct_layouts_match_the_header_static_asserts() {
    use std::mem::{align_of, offset_of, size_of};
    use crate::{
        GuiCmd, GuiColor, GuiDockState, GuiDragPayload, GuiDrawData, GuiInput, GuiKeyBinding,
        GuiKeyEvent, GuiRect, GuiScrollbarSkin, GuiStr, GuiVec2, GuiVert,
    };

    assert_eq!(
        (
            size_of::<crate::GuiPanelState>(),
            align_of::<crate::GuiPanelState>()
        ),
        (24, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiPanelState, kind),
            offset_of!(crate::GuiPanelState, mode),
            offset_of!(crate::GuiPanelState, values)
        ],
        [0, 4, 8]
    );
    assert_eq!(
        (
            size_of::<crate::GuiTextEditOptions>(),
            align_of::<crate::GuiTextEditOptions>()
        ),
        (16, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiTextEditOptions, size),
            offset_of!(crate::GuiTextEditOptions, flags),
            offset_of!(crate::GuiTextEditOptions, direction)
        ],
        [0, 8, 12]
    );
    assert_eq!(
        (
            size_of::<crate::GuiShapedOptions>(),
            align_of::<crate::GuiShapedOptions>()
        ),
        (12, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiShapedOptions, width),
            offset_of!(crate::GuiShapedOptions, wrap),
            offset_of!(crate::GuiShapedOptions, reserved),
            offset_of!(crate::GuiShapedOptions, direction)
        ],
        [0, 4, 5, 8]
    );
    assert_eq!(
        (
            size_of::<crate::GuiTextSelection>(),
            align_of::<crate::GuiTextSelection>()
        ),
        (16, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiTextSelection, anchor),
            offset_of!(crate::GuiTextSelection, cursor)
        ],
        [0, 8]
    );
    assert_eq!(
        (
            size_of::<crate::GuiTextEditResponse>(),
            align_of::<crate::GuiTextEditResponse>()
        ),
        (32, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiTextEditResponse, id),
            offset_of!(crate::GuiTextEditResponse, rect),
            offset_of!(crate::GuiTextEditResponse, changed),
            offset_of!(crate::GuiTextEditResponse, focused),
            offset_of!(crate::GuiTextEditResponse, reserved)
        ],
        [0, 8, 24, 25, 26]
    );
    assert_eq!(
        (
            size_of::<crate::GuiTextCaret>(),
            align_of::<crate::GuiTextCaret>()
        ),
        (32, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiTextCaret, owner),
            offset_of!(crate::GuiTextCaret, rect),
            offset_of!(crate::GuiTextCaret, composing),
            offset_of!(crate::GuiTextCaret, reserved)
        ],
        [0, 8, 24, 25]
    );
    assert_eq!(
        (
            size_of::<crate::GuiClipboardRequest>(),
            align_of::<crate::GuiClipboardRequest>()
        ),
        (40, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiClipboardRequest, request_id),
            offset_of!(crate::GuiClipboardRequest, owner),
            offset_of!(crate::GuiClipboardRequest, action),
            offset_of!(crate::GuiClipboardRequest, reserved),
            offset_of!(crate::GuiClipboardRequest, text)
        ],
        [0, 8, 16, 20, 24]
    );
    assert_eq!(
        (
            size_of::<crate::GuiShapedCaret>(),
            align_of::<crate::GuiShapedCaret>()
        ),
        (24, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiShapedCaret, byte),
            offset_of!(crate::GuiShapedCaret, position),
            offset_of!(crate::GuiShapedCaret, line)
        ],
        [0, 8, 16]
    );
    assert_eq!(size_of::<GuiVec2>(), 8);
    assert_eq!(
        (
            size_of::<crate::GuiGradientPaint>(),
            align_of::<crate::GuiGradientPaint>()
        ),
        (28, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiGradientPaint, kind),
            offset_of!(crate::GuiGradientPaint, origin),
            offset_of!(crate::GuiGradientPaint, vector),
            offset_of!(crate::GuiGradientPaint, spread),
            offset_of!(crate::GuiGradientPaint, phase)
        ],
        [0, 4, 12, 20, 24]
    );
    assert_eq!(
        (
            size_of::<crate::GuiAlphaMask>(),
            align_of::<crate::GuiAlphaMask>()
        ),
        (40, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiAlphaMask, texture),
            offset_of!(crate::GuiAlphaMask, rect),
            offset_of!(crate::GuiAlphaMask, uv)
        ],
        [0, 8, 24]
    );
    assert_eq!(
        (
            size_of::<crate::GuiGradientMask>(),
            align_of::<crate::GuiGradientMask>()
        ),
        (24, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiGradientMask, start),
            offset_of!(crate::GuiGradientMask, end),
            offset_of!(crate::GuiGradientMask, opacity)
        ],
        [0, 8, 16]
    );
    assert_eq!(
        (
            size_of::<crate::GuiMaskOp>(),
            align_of::<crate::GuiMaskOp>()
        ),
        (48, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiMaskOp, texture),
            offset_of!(crate::GuiMaskOp, kind),
            offset_of!(crate::GuiMaskOp, reserved),
            offset_of!(crate::GuiMaskOp, rect),
            offset_of!(crate::GuiMaskOp, params)
        ],
        [0, 8, 12, 16, 32]
    );
    assert_eq!(
        (
            size_of::<crate::GuiMaskStack>(),
            align_of::<crate::GuiMaskStack>()
        ),
        (200, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiMaskStack, ops),
            offset_of!(crate::GuiMaskStack, count),
            offset_of!(crate::GuiMaskStack, reserved)
        ],
        [0, 192, 196]
    );
    assert_eq!(
        (
            size_of::<crate::GuiGradientFill>(),
            align_of::<crate::GuiGradientFill>()
        ),
        (176, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiGradientFill, rect),
            offset_of!(crate::GuiGradientFill, origin),
            offset_of!(crate::GuiGradientFill, vector),
            offset_of!(crate::GuiGradientFill, kind),
            offset_of!(crate::GuiGradientFill, spread),
            offset_of!(crate::GuiGradientFill, phase),
            offset_of!(crate::GuiGradientFill, stop_count),
            offset_of!(crate::GuiGradientFill, stops)
        ],
        [0, 16, 24, 32, 36, 40, 44, 48]
    );
    assert_eq!(
        (
            size_of::<crate::GuiSdfStyle>(),
            align_of::<crate::GuiSdfStyle>()
        ),
        (28, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiSdfStyle, kind),
            offset_of!(crate::GuiSdfStyle, width),
            offset_of!(crate::GuiSdfStyle, placement),
            offset_of!(crate::GuiSdfStyle, reach),
            offset_of!(crate::GuiSdfStyle, light_direction),
            offset_of!(crate::GuiSdfStyle, light_power)
        ],
        [0, 4, 8, 12, 16, 24]
    );
    assert_eq!(
        (
            size_of::<crate::GuiRevealMask>(),
            align_of::<crate::GuiRevealMask>()
        ),
        (32, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiRevealMask, rect),
            offset_of!(crate::GuiRevealMask, direction),
            offset_of!(crate::GuiRevealMask, progress),
            offset_of!(crate::GuiRevealMask, softness),
            offset_of!(crate::GuiRevealMask, inverted),
            offset_of!(crate::GuiRevealMask, reserved)
        ],
        [0, 16, 20, 24, 28, 29]
    );
    assert_eq!(
        (
            size_of::<crate::GuiSdfShape>(),
            align_of::<crate::GuiSdfShape>()
        ),
        (64, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiSdfShape, rect),
            offset_of!(crate::GuiSdfShape, radii),
            offset_of!(crate::GuiSdfShape, parameters),
            offset_of!(crate::GuiSdfShape, light)
        ],
        [0, 16, 32, 48]
    );
    assert_eq!(
        (
            size_of::<crate::GuiSurfaceStyle>(),
            align_of::<crate::GuiSurfaceStyle>()
        ),
        (
            size_of::<tacita::effects::SurfaceStyle>(),
            align_of::<tacita::effects::SurfaceStyle>()
        )
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, flags),
        offset_of!(tacita::effects::SurfaceStyle, flags)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, phase),
        offset_of!(tacita::effects::SurfaceStyle, phase)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, pixel_scale),
        offset_of!(tacita::effects::SurfaceStyle, pixel_scale)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, noise),
        offset_of!(tacita::effects::SurfaceStyle, noise)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, grain),
        offset_of!(tacita::effects::SurfaceStyle, grain)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, dither),
        offset_of!(tacita::effects::SurfaceStyle, dither)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, dither_kind),
        offset_of!(tacita::effects::SurfaceStyle, dither_kind)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, exposure),
        offset_of!(tacita::effects::SurfaceStyle, exposure)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, contrast),
        offset_of!(tacita::effects::SurfaceStyle, contrast)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, saturation),
        offset_of!(tacita::effects::SurfaceStyle, saturation)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, desaturation),
        offset_of!(tacita::effects::SurfaceStyle, desaturation)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, tint),
        offset_of!(tacita::effects::SurfaceStyle, tint)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, scanlines),
        offset_of!(tacita::effects::SurfaceStyle, scanlines)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, spacing),
        offset_of!(tacita::effects::SurfaceStyle, spacing)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, grid_width),
        offset_of!(tacita::effects::SurfaceStyle, grid_width)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, dot_radius),
        offset_of!(tacita::effects::SurfaceStyle, dot_radius)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, vignette),
        offset_of!(tacita::effects::SurfaceStyle, vignette)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, shimmer),
        offset_of!(tacita::effects::SurfaceStyle, shimmer)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, sweep_width),
        offset_of!(tacita::effects::SurfaceStyle, sweep_width)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, sweep_speed),
        offset_of!(tacita::effects::SurfaceStyle, sweep_speed)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, distortion),
        offset_of!(tacita::effects::SurfaceStyle, distortion)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, frequency),
        offset_of!(tacita::effects::SurfaceStyle, frequency)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, chromatic),
        offset_of!(tacita::effects::SurfaceStyle, chromatic)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, refraction),
        offset_of!(tacita::effects::SurfaceStyle, refraction)
    );
    assert_eq!(
        offset_of!(crate::GuiSurfaceStyle, rect),
        offset_of!(tacita::effects::SurfaceStyle, rect)
    );
    assert_eq!(
        (
            size_of::<crate::GuiImageFilter>(),
            align_of::<crate::GuiImageFilter>()
        ),
        (
            size_of::<tacita::effects::ImageFilter>(),
            align_of::<tacita::effects::ImageFilter>()
        )
    );
    assert_eq!(
        offset_of!(crate::GuiImageFilter, kind),
        offset_of!(tacita::effects::ImageFilter, kind)
    );
    assert_eq!(
        offset_of!(crate::GuiImageFilter, overwrite),
        offset_of!(tacita::effects::ImageFilter, overwrite)
    );
    assert_eq!(
        offset_of!(crate::GuiImageFilter, radius),
        offset_of!(tacita::effects::ImageFilter, radius)
    );
    assert_eq!(
        offset_of!(crate::GuiImageFilter, threshold),
        offset_of!(tacita::effects::ImageFilter, threshold)
    );
    assert_eq!(
        offset_of!(crate::GuiImageFilter, intensity),
        offset_of!(tacita::effects::ImageFilter, intensity)
    );
    assert_eq!(
        offset_of!(crate::GuiImageFilter, texel),
        offset_of!(tacita::effects::ImageFilter, texel)
    );
    assert_eq!(
        offset_of!(crate::GuiImageFilter, reserved),
        offset_of!(tacita::effects::ImageFilter, reserved)
    );
    assert_eq!(
        offset_of!(crate::GuiImageFilter, secondary),
        offset_of!(tacita::effects::ImageFilter, secondary)
    );
    assert_eq!(
        offset_of!(crate::GuiImageFilter, params),
        offset_of!(tacita::effects::ImageFilter, params)
    );
    assert_eq!(
        (
            size_of::<crate::GuiDrawEffects>(),
            align_of::<crate::GuiDrawEffects>()
        ),
        (
            size_of::<tacita::effects::DrawEffects>(),
            align_of::<tacita::effects::DrawEffects>()
        )
    );
    assert_eq!(
        offset_of!(crate::GuiDrawEffects, gradient),
        offset_of!(tacita::effects::DrawEffects, gradient)
    );
    assert_eq!(
        offset_of!(crate::GuiDrawEffects, masks),
        offset_of!(tacita::effects::DrawEffects, masks)
    );
    assert_eq!(
        offset_of!(crate::GuiDrawEffects, sdf),
        offset_of!(tacita::effects::DrawEffects, sdf)
    );
    assert_eq!(
        offset_of!(crate::GuiDrawEffects, surface),
        offset_of!(tacita::effects::DrawEffects, surface)
    );
    assert_eq!(
        offset_of!(crate::GuiDrawEffects, filter),
        offset_of!(tacita::effects::DrawEffects, filter)
    );
    assert_eq!(
        offset_of!(crate::GuiDrawEffects, stencil),
        offset_of!(tacita::effects::DrawEffects, stencil)
    );
    assert_eq!(
        (
            size_of::<crate::GuiSurfaceStyle>(),
            align_of::<crate::GuiSurfaceStyle>()
        ),
        (128, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiSurfaceStyle, flags),
            offset_of!(crate::GuiSurfaceStyle, phase),
            offset_of!(crate::GuiSurfaceStyle, pixel_scale),
            offset_of!(crate::GuiSurfaceStyle, noise),
            offset_of!(crate::GuiSurfaceStyle, grain),
            offset_of!(crate::GuiSurfaceStyle, dither),
            offset_of!(crate::GuiSurfaceStyle, dither_kind),
            offset_of!(crate::GuiSurfaceStyle, exposure),
            offset_of!(crate::GuiSurfaceStyle, contrast),
            offset_of!(crate::GuiSurfaceStyle, saturation),
            offset_of!(crate::GuiSurfaceStyle, desaturation),
            offset_of!(crate::GuiSurfaceStyle, tint),
            offset_of!(crate::GuiSurfaceStyle, scanlines),
            offset_of!(crate::GuiSurfaceStyle, spacing),
            offset_of!(crate::GuiSurfaceStyle, grid_width),
            offset_of!(crate::GuiSurfaceStyle, dot_radius),
            offset_of!(crate::GuiSurfaceStyle, vignette),
            offset_of!(crate::GuiSurfaceStyle, shimmer),
            offset_of!(crate::GuiSurfaceStyle, sweep_width),
            offset_of!(crate::GuiSurfaceStyle, sweep_speed),
            offset_of!(crate::GuiSurfaceStyle, distortion),
            offset_of!(crate::GuiSurfaceStyle, frequency),
            offset_of!(crate::GuiSurfaceStyle, chromatic),
            offset_of!(crate::GuiSurfaceStyle, refraction),
            offset_of!(crate::GuiSurfaceStyle, rect)
        ],
        [
            0, 4, 8, 16, 20, 24, 28, 32, 36, 40, 44, 48, 64, 68, 72, 76, 80, 84, 88, 92, 96, 100,
            104, 108, 112
        ]
    );
    assert_eq!(
        (
            size_of::<crate::GuiImageFilter>(),
            align_of::<crate::GuiImageFilter>()
        ),
        (64, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiImageFilter, kind),
            offset_of!(crate::GuiImageFilter, overwrite),
            offset_of!(crate::GuiImageFilter, radius),
            offset_of!(crate::GuiImageFilter, threshold),
            offset_of!(crate::GuiImageFilter, intensity),
            offset_of!(crate::GuiImageFilter, texel),
            offset_of!(crate::GuiImageFilter, reserved),
            offset_of!(crate::GuiImageFilter, secondary),
            offset_of!(crate::GuiImageFilter, params)
        ],
        [0, 4, 8, 12, 16, 20, 28, 32, 40]
    );
    assert_eq!(
        (size_of::<crate::GuiBlur>(), align_of::<crate::GuiBlur>()),
        (12, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiBlur, method),
            offset_of!(crate::GuiBlur, radius),
            offset_of!(crate::GuiBlur, levels)
        ],
        [0, 4, 8]
    );
    assert_eq!(
        (
            size_of::<crate::GuiFilterStep>(),
            align_of::<crate::GuiFilterStep>()
        ),
        (80, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiFilterStep, source),
            offset_of!(crate::GuiFilterStep, size),
            offset_of!(crate::GuiFilterStep, reserved),
            offset_of!(crate::GuiFilterStep, filter)
        ],
        [0, 4, 12, 16]
    );
    assert_eq!(
        [
            offset_of!(crate::GuiDrawEffects, surface),
            offset_of!(crate::GuiDrawEffects, filter),
            offset_of!(crate::GuiDrawEffects, stencil)
        ],
        [440, 568, 632]
    );
    assert_eq!(
        (
            size_of::<crate::GuiDrawEffects>(),
            align_of::<crate::GuiDrawEffects>()
        ),
        (640, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiDrawEffects, gradient),
            offset_of!(crate::GuiDrawEffects, masks),
            offset_of!(crate::GuiDrawEffects, sdf)
        ],
        [0, 176, 376]
    );
    assert_eq!(
        (size_of::<crate::GuiVec2>(), align_of::<crate::GuiVec2>()),
        (size_of::<[f32; 2]>(), align_of::<[f32; 2]>())
    );
    assert_eq!(
        [offset_of!(crate::GuiVec2, x), offset_of!(crate::GuiVec2, y)],
        [0, 4]
    );
    assert_eq!(
        (
            size_of::<crate::GuiStrokeStyle>(),
            align_of::<crate::GuiStrokeStyle>()
        ),
        (20, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiStrokeStyle, width),
            offset_of!(crate::GuiStrokeStyle, join),
            offset_of!(crate::GuiStrokeStyle, cap),
            offset_of!(crate::GuiStrokeStyle, miter_limit),
            offset_of!(crate::GuiStrokeStyle, tolerance)
        ],
        [0, 4, 8, 12, 16]
    );
    assert_eq!(
        (
            size_of::<crate::GuiLayoutOptions>(),
            align_of::<crate::GuiLayoutOptions>()
        ),
        (8, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiLayoutOptions, wrap),
            offset_of!(crate::GuiLayoutOptions, align_content)
        ],
        [0, 4]
    );
    assert_eq!(
        (
            size_of::<crate::GuiGridCell>(),
            align_of::<crate::GuiGridCell>()
        ),
        (16, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiGridCell, row),
            offset_of!(crate::GuiGridCell, column),
            offset_of!(crate::GuiGridCell, row_span),
            offset_of!(crate::GuiGridCell, column_span)
        ],
        [0, 4, 8, 12]
    );
    assert_eq!(
        (
            size_of::<crate::GuiLayoutSpec>(),
            align_of::<crate::GuiLayoutSpec>()
        ),
        (64, 4)
    );
    assert_eq!(
        (
            size_of::<crate::GuiRoundedRect>(),
            align_of::<crate::GuiRoundedRect>()
        ),
        (32, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiRoundedRect, rect),
            offset_of!(crate::GuiRoundedRect, radii)
        ],
        [0, 16]
    );
    assert_eq!(
        (
            size_of::<crate::GuiShadow>(),
            align_of::<crate::GuiShadow>()
        ),
        (24, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiShadow, offset),
            offset_of!(crate::GuiShadow, spread),
            offset_of!(crate::GuiShadow, blur),
            offset_of!(crate::GuiShadow, color),
            offset_of!(crate::GuiShadow, inset),
            offset_of!(crate::GuiShadow, reserved)
        ],
        [0, 8, 12, 16, 20, 21]
    );
    assert_eq!(
        [
            offset_of!(crate::GuiLayoutSpec, kind),
            offset_of!(crate::GuiLayoutSpec, size),
            offset_of!(crate::GuiLayoutSpec, min),
            offset_of!(crate::GuiLayoutSpec, max),
            offset_of!(crate::GuiLayoutSpec, padding),
            offset_of!(crate::GuiLayoutSpec, gap),
            offset_of!(crate::GuiLayoutSpec, align),
            offset_of!(crate::GuiLayoutSpec, justify),
            offset_of!(crate::GuiLayoutSpec, columns)
        ],
        [0, 4, 12, 20, 28, 44, 52, 56, 60]
    );
    assert_eq!(
        (
            size_of::<crate::GuiLayoutItem>(),
            align_of::<crate::GuiLayoutItem>()
        ),
        (76, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiLayoutItem, preferred),
            offset_of!(crate::GuiLayoutItem, min),
            offset_of!(crate::GuiLayoutItem, max),
            offset_of!(crate::GuiLayoutItem, grow),
            offset_of!(crate::GuiLayoutItem, shrink),
            offset_of!(crate::GuiLayoutItem, margin),
            offset_of!(crate::GuiLayoutItem, baseline),
            offset_of!(crate::GuiLayoutItem, absolute),
            offset_of!(crate::GuiLayoutItem, hit_test),
            offset_of!(crate::GuiLayoutItem, reserved),
            offset_of!(crate::GuiLayoutItem, anchor),
            offset_of!(crate::GuiLayoutItem, offset),
            offset_of!(crate::GuiLayoutItem, layer)
        ],
        [0, 8, 16, 24, 28, 32, 48, 52, 53, 54, 56, 64, 72]
    );
    assert_eq!(
        (
            size_of::<crate::GuiTextOptions>(),
            align_of::<crate::GuiTextOptions>()
        ),
        (24, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiTextOptions, wrap),
            offset_of!(crate::GuiTextOptions, reserved),
            offset_of!(crate::GuiTextOptions, overflow),
            offset_of!(crate::GuiTextOptions, horizontal),
            offset_of!(crate::GuiTextOptions, vertical),
            offset_of!(crate::GuiTextOptions, line_spacing),
            offset_of!(crate::GuiTextOptions, max_lines)
        ],
        [0, 1, 4, 8, 12, 16, 20]
    );
    assert_eq!(
        (
            size_of::<crate::GuiTextMetrics>(),
            align_of::<crate::GuiTextMetrics>()
        ),
        (24, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiTextMetrics, size),
            offset_of!(crate::GuiTextMetrics, lines),
            offset_of!(crate::GuiTextMetrics, truncated),
            offset_of!(crate::GuiTextMetrics, reserved)
        ],
        [0, 8, 16, 17]
    );
    assert_eq!(
        (
            size_of::<crate::GuiTextSpan>(),
            align_of::<crate::GuiTextSpan>()
        ),
        (24, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiTextSpan, text),
            offset_of!(crate::GuiTextSpan, font),
            offset_of!(crate::GuiTextSpan, color)
        ],
        [0, 16, 20]
    );
    assert_eq!(
        (
            size_of::<crate::GuiKeyframe>(),
            align_of::<crate::GuiKeyframe>()
        ),
        (12, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiKeyframe, time),
            offset_of!(crate::GuiKeyframe, value),
            offset_of!(crate::GuiKeyframe, easing)
        ],
        [0, 4, 8]
    );
    assert_eq!(
        (
            size_of::<crate::GuiTimelineOptions>(),
            align_of::<crate::GuiTimelineOptions>()
        ),
        (24, 8)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiTimelineOptions, mode),
            offset_of!(crate::GuiTimelineOptions, speed),
            offset_of!(crate::GuiTimelineOptions, playing),
            offset_of!(crate::GuiTimelineOptions, reserved),
            offset_of!(crate::GuiTimelineOptions, revision)
        ],
        [0, 4, 8, 9, 16]
    );
    assert_eq!(
        (
            size_of::<crate::GuiGradientStop>(),
            align_of::<crate::GuiGradientStop>()
        ),
        (8, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiGradientStop, position),
            offset_of!(crate::GuiGradientStop, color)
        ],
        [0, 4]
    );
    assert_eq!(
        (
            size_of::<crate::GuiGradientOptions>(),
            align_of::<crate::GuiGradientOptions>()
        ),
        (16, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiGradientOptions, axis),
            offset_of!(crate::GuiGradientOptions, spread),
            offset_of!(crate::GuiGradientOptions, resolution)
        ],
        [0, 8, 12]
    );
    assert_eq!(
        (
            size_of::<crate::GuiThemeTokens>(),
            align_of::<crate::GuiThemeTokens>()
        ),
        (108, 4)
    );
    assert_eq!(
        [
            offset_of!(crate::GuiThemeTokens, colors),
            offset_of!(crate::GuiThemeTokens, frame_padding),
            offset_of!(crate::GuiThemeTokens, item_spacing),
            offset_of!(crate::GuiThemeTokens, window_padding),
            offset_of!(crate::GuiThemeTokens, slider_grab_size),
            offset_of!(crate::GuiThemeTokens, scrollbar_size),
            offset_of!(crate::GuiThemeTokens, corner_rounding),
            offset_of!(crate::GuiThemeTokens, border_size),
            offset_of!(crate::GuiThemeTokens, anti_aliasing),
            offset_of!(crate::GuiThemeTokens, reserved)
        ],
        [0, 64, 72, 80, 88, 92, 96, 100, 104, 105]
    );
    assert_eq!(size_of::<crate::GuiMotionSettings>(), 32);
    assert_eq!(align_of::<crate::GuiMotionSettings>(), 4);
    assert_eq!(offset_of!(crate::GuiMotionSettings, enabled), 0);
    assert_eq!(offset_of!(crate::GuiMotionSettings, reduced_motion), 1);
    assert_eq!(offset_of!(crate::GuiMotionSettings, reserved), 2);
    assert_eq!(offset_of!(crate::GuiMotionSettings, easing), 4);
    assert_eq!(offset_of!(crate::GuiMotionSettings, hover), 8);
    assert_eq!(offset_of!(crate::GuiMotionSettings, focus), 12);
    assert_eq!(offset_of!(crate::GuiMotionSettings, indicator), 16);
    assert_eq!(offset_of!(crate::GuiMotionSettings, popup), 20);
    assert_eq!(offset_of!(crate::GuiMotionSettings, collapse), 24);
    assert_eq!(offset_of!(crate::GuiMotionSettings, scroll), 28);
    assert_eq!(offset_of!(GuiVec2, y), 4);
    assert_eq!(size_of::<GuiRect>(), 16);
    assert_eq!(offset_of!(GuiRect, max), 8);
    assert_eq!(size_of::<GuiDockState>(), 12);
    assert_eq!(offset_of!(GuiDockState, float_pos), 4);
    assert_eq!(size_of::<GuiColor>(), 4);
    assert_eq!(size_of::<GuiScrollbarSkin>(), 24);
    assert_eq!(offset_of!(GuiScrollbarSkin, track_click), 20);
    assert_eq!(size_of::<GuiDragPayload>(), 48);
    assert_eq!(offset_of!(GuiDragPayload, len), 8);
    assert_eq!(offset_of!(GuiDragPayload, bytes), 9);
    assert_eq!(size_of::<GuiStr>(), 16);
    assert_eq!(offset_of!(GuiStr, len), 8);
    assert_eq!(size_of::<GuiKeyBinding>(), 8);
    assert_eq!(offset_of!(GuiKeyBinding, key), 2);
    assert_eq!(offset_of!(GuiKeyBinding, mouse_button), 4);
    assert_eq!(offset_of!(GuiKeyBinding, ctrl), 5);
    assert_eq!(offset_of!(GuiKeyBinding, alt), 7);
    assert_eq!(size_of::<GuiVert>(), 20);
    assert_eq!(offset_of!(GuiVert, uv), 8);
    assert_eq!(offset_of!(GuiVert, col), 16);
    assert_eq!(size_of::<GuiCmd>(), 32);
    assert_eq!(offset_of!(GuiCmd, clip_rect), 4);
    assert_eq!(offset_of!(GuiCmd, effects), 20);
    assert_eq!(offset_of!(GuiCmd, texture), 24);
    assert_eq!(size_of::<GuiDrawData>(), 48);
    assert_eq!(offset_of!(GuiDrawData, vertex_count), 8);
    assert_eq!(offset_of!(GuiDrawData, indices), 16);
    assert_eq!(offset_of!(GuiDrawData, index_count), 24);
    assert_eq!(offset_of!(GuiDrawData, commands), 32);
    assert_eq!(offset_of!(GuiDrawData, command_count), 40);
    assert_eq!(size_of::<GuiKeyEvent>(), 8);
    assert_eq!(offset_of!(GuiKeyEvent, key), 2);
    assert_eq!(offset_of!(GuiKeyEvent, ch), 4);
    assert_eq!(size_of::<GuiInput>(), 48);
    assert_eq!(offset_of!(GuiInput, mouse_down), 8);
    assert_eq!(offset_of!(GuiInput, mouse_wheel), 16);
    assert_eq!(offset_of!(GuiInput, viewport_size), 20);
    assert_eq!(offset_of!(GuiInput, keys), 32);
    assert_eq!(offset_of!(GuiInput, key_count), 40);
}
