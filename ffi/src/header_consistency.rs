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

fn header_defines(header: &str) -> BTreeMap<String, u64> {
    let mut defines = BTreeMap::new();
    for line in header.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("#define") {
            let tokens: Vec<&str> = rest.split_whitespace().collect();
            if tokens.len() < 2 || !tokens[0].starts_with("GUI_") {
                continue;
            }
            let value: u64 = tokens[1]
                .parse()
                .unwrap_or_else(|_| panic!("non-integer #define value: {line}"));
            defines.insert(tokens[0].to_string(), value);
        }
    }
    defines
}

#[test]
fn header_declares_exactly_the_exported_functions() {
    let header = strip_comments(&read(
        &PathBuf::from(manifest_dir()).join("restricted_gui.h"),
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
        47,
        "the export surface changed; update the header"
    );
}

#[test]
fn header_constants_match_the_implementation_values() {
    let header = strip_comments(&read(
        &PathBuf::from(manifest_dir()).join("restricted_gui.h"),
    ));
    let defines = header_defines(&header);

    let mut expected: BTreeMap<String, u64> = BTreeMap::new();
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
    for (code, key) in ALL_KEYS.iter().enumerate() {
        expected.insert(format!("GUI_KEY_{:?}", key).to_uppercase(), code as u64);
    }

    assert_eq!(
        defines, expected,
        "header constant drift (left: header, right: implementation)"
    );
}

#[test]
// This test exists because the header and the Rust WILL drift. They always drift. Two sides assert the same layout so the drift has nowhere to hide.
fn struct_layouts_match_the_header_static_asserts() {
    use crate::{
        GuiCmd, GuiColor, GuiDrawData, GuiInput, GuiKeyBinding, GuiKeyEvent, GuiRect, GuiStr,
        GuiVec2, GuiVert,
    };
    use std::mem::{offset_of, size_of};

    assert_eq!(size_of::<GuiVec2>(), 8);
    assert_eq!(offset_of!(GuiVec2, y), 4);
    assert_eq!(size_of::<GuiRect>(), 16);
    assert_eq!(offset_of!(GuiRect, max), 8);
    assert_eq!(size_of::<GuiColor>(), 4);
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
    assert_eq!(size_of::<GuiInput>(), 40);
    assert_eq!(offset_of!(GuiInput, mouse_down), 8);
    assert_eq!(offset_of!(GuiInput, mouse_wheel), 12);
    assert_eq!(offset_of!(GuiInput, viewport_size), 16);
    assert_eq!(offset_of!(GuiInput, keys), 24);
    assert_eq!(offset_of!(GuiInput, key_count), 32);
}
