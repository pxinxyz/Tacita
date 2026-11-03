use alloc::string::String;
use core::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

pub const MAX_EDIT_BYTES: usize = 65536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditError {
    InvalidInput,
    TooLong,
}

pub(crate) fn is_line_break(ch: char) -> bool {
    unicode_bidi::bidi_class(ch) == unicode_bidi::BidiClass::B
        || matches!(ch, '\u{2028}' | '\u{000b}' | '\u{000c}')
}

pub fn previous_grapheme(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .take_while(|(i, _)| *i < offset)
        .last()
        .map_or(0, |(i, _)| i)
}
pub fn next_grapheme(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .find(|(i, _)| *i > offset)
        .map_or(text.len(), |(i, _)| i)
}
pub fn is_grapheme_boundary(text: &str, offset: usize) -> bool {
    offset == text.len() || text.grapheme_indices(true).any(|(i, _)| i == offset)
}
pub(crate) fn snap(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    if is_grapheme_boundary(text, offset) {
        offset
    } else {
        previous_grapheme(text, offset)
    }
}

pub struct TextEditor {
    pub(crate) text: String,
    pub(crate) cursor: usize,
    pub(crate) anchor: usize,
    pub(crate) preedit: String,
    pub(crate) composition_range: Option<Range<usize>>,
    pub(crate) preedit_cursor: Range<usize>,
    pub(crate) preedit_visible: bool,
    pub(crate) revision: u64,
    pub(crate) scroll: [f32; 2],
    pub(crate) blink: f32,
    pub(crate) dragging: bool,
    pub(crate) layout: crate::font::ShapedText,
    pub(crate) display: String,
    pub(crate) affinity: Option<crate::font::CaretPosition>,
    pub(crate) preferred_x: Option<f32>,
    pub(crate) reveal: bool,
}
impl Default for TextEditor {
    fn default() -> Self {
        Self::new("").expect("empty document is valid")
    }
}
impl TextEditor {
    pub fn new(text: &str) -> Result<Self, EditError> {
        validate(text)?;
        Ok(Self {
            text: String::from(text),
            cursor: text.len(),
            anchor: text.len(),
            preedit: String::new(),
            composition_range: None,
            preedit_cursor: 0..0,
            preedit_visible: true,
            revision: 0,
            scroll: [0.; 2],
            blink: 0.,
            dragging: false,
            layout: crate::font::ShapedText::default(),
            display: String::new(),
            affinity: None,
            preferred_x: None,
            reveal: true,
        })
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn anchor(&self) -> usize {
        self.anchor
    }
    pub fn selection(&self) -> Range<usize> {
        self.cursor.min(self.anchor)..self.cursor.max(self.anchor)
    }
    pub fn selected_text(&self) -> &str {
        &self.text[self.selection()]
    }
    pub fn preedit(&self) -> &str {
        &self.preedit
    }
    pub fn composing(&self) -> bool {
        self.composition_range.is_some()
    }
    pub fn replace(&mut self, text: &str) -> Result<bool, EditError> {
        validate(text)?;
        let changed = self.text != text;
        if changed {
            self.text.clear();
            self.text.push_str(text);
            self.revision = self.revision.wrapping_add(1);
        }
        self.cursor = snap(text, self.cursor);
        self.anchor = snap(text, self.anchor);
        self.cancel_composition();
        self.affinity = None;
        self.preferred_x = None;
        self.reveal = true;
        Ok(changed)
    }
    pub fn select(&mut self, anchor: usize, cursor: usize) -> Result<(), EditError> {
        if !is_grapheme_boundary(&self.text, anchor) || !is_grapheme_boundary(&self.text, cursor) {
            return Err(EditError::InvalidInput);
        }
        self.cancel_composition();
        self.anchor = anchor;
        self.cursor = cursor;
        self.affinity = None;
        self.preferred_x = None;
        self.reveal = true;
        self.blink = 0.;
        Ok(())
    }
    pub fn select_all(&mut self) {
        self.cancel_composition();
        self.anchor = 0;
        self.cursor = self.text.len();
        self.affinity = None;
        self.preferred_x = None;
        self.reveal = true;
        self.blink = 0.;
    }
    pub fn insert(&mut self, text: &str) -> Result<bool, EditError> {
        validate(text)?;
        if self.text.len() - self.selection().len() + text.len() > MAX_EDIT_BYTES {
            return Err(EditError::TooLong);
        }
        self.cancel_composition();
        self.replace_range(self.selection(), text)
    }
    fn replace_range(&mut self, range: Range<usize>, text: &str) -> Result<bool, EditError> {
        if self.text.len() - range.len() + text.len() > MAX_EDIT_BYTES {
            return Err(EditError::TooLong);
        }
        let changed = &self.text[range.clone()] != text;
        let end = range.start + text.len();
        if changed {
            self.text.replace_range(range, text);
            self.revision = self.revision.wrapping_add(1);
        }
        self.cursor = if is_grapheme_boundary(&self.text, end) {
            end
        } else {
            next_grapheme(&self.text, end)
        };
        self.anchor = self.cursor;
        self.affinity = None;
        self.preferred_x = None;
        self.reveal = true;
        self.blink = 0.;
        Ok(changed)
    }
    pub fn backspace(&mut self) -> bool {
        self.cancel_composition();
        let mut range = self.selection();
        if range.is_empty() {
            range.start = previous_grapheme(&self.text, self.cursor);
        }
        self.replace_range(range, "")
            .expect("deletion cannot grow text")
    }
    pub fn delete(&mut self) -> bool {
        self.cancel_composition();
        let mut range = self.selection();
        if range.is_empty() {
            range.end = next_grapheme(&self.text, self.cursor);
        }
        self.replace_range(range, "")
            .expect("deletion cannot grow text")
    }
    pub fn move_logical(&mut self, forward: bool, extend: bool) {
        self.cancel_composition();
        self.cursor = if !extend && !self.selection().is_empty() {
            if forward {
                self.selection().end
            } else {
                self.selection().start
            }
        } else if forward {
            next_grapheme(&self.text, self.cursor)
        } else {
            previous_grapheme(&self.text, self.cursor)
        };
        if !extend {
            self.anchor = self.cursor;
        }
        self.affinity = None;
        self.preferred_x = None;
        self.reveal = true;
        self.blink = 0.;
    }
    pub fn set_preedit(&mut self, text: &str, cursor: Range<usize>) -> Result<(), EditError> {
        validate(text)?;
        if cursor.start > cursor.end
            || cursor.end > text.len()
            || !text.is_char_boundary(cursor.start)
            || !text.is_char_boundary(cursor.end)
        {
            return Err(EditError::InvalidInput);
        }
        let replaced = self
            .composition_range
            .as_ref()
            .map_or_else(|| self.selection().len(), |r| r.len());
        if self.text.len() - replaced + text.len() > MAX_EDIT_BYTES {
            return Err(EditError::TooLong);
        }
        if self.composition_range.is_none() {
            self.composition_range = Some(self.selection());
        }
        self.preedit.clear();
        self.preedit.push_str(text);
        self.preedit_cursor = cursor;
        self.preedit_visible = true;
        self.blink = 0.;
        Ok(())
    }
    pub fn commit(&mut self, text: &str) -> Result<bool, EditError> {
        validate(text)?;
        let range = self
            .composition_range
            .clone()
            .unwrap_or_else(|| self.selection());
        if self.text.len() - range.len() + text.len() > MAX_EDIT_BYTES {
            return Err(EditError::TooLong);
        }
        self.cancel_composition();
        self.replace_range(range, text)
    }
    pub fn cancel_composition(&mut self) {
        self.preedit.clear();
        self.composition_range = None;
        self.preedit_cursor = 0..0;
    }
}
fn validate(text: &str) -> Result<(), EditError> {
    if text.len() > MAX_EDIT_BYTES {
        return Err(EditError::TooLong);
    }
    if text.contains('\0') {
        return Err(EditError::InvalidInput);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn movement_and_deletion_preserve_extended_graphemes() {
        for cluster in ["e\u{301}", "👩‍💻", "🇬🇧", "👍🏽", "\r\n"] {
            let mut editor = TextEditor::new(cluster).unwrap();
            editor.move_logical(false, false);
            assert_eq!(editor.cursor(), 0);
            editor.move_logical(true, false);
            assert_eq!(editor.cursor(), cluster.len());
            assert!(editor.backspace());
            assert_eq!(editor.text(), "");
            editor.insert(cluster).unwrap();
            editor.select(0, 0).unwrap();
            assert!(editor.delete());
            assert_eq!(editor.text(), "");
        }
    }
    #[test]
    fn reversed_selection_replace_and_invalid_offsets_are_atomic() {
        let mut editor = TextEditor::new("one e\u{301} two").unwrap();
        editor.select(7, 4).unwrap();
        assert_eq!(editor.selected_text(), "e\u{301}");
        assert!(editor.select(5, 5).is_err());
        assert_eq!(editor.selection(), 4..7);
        editor.insert("X").unwrap();
        assert_eq!(editor.text(), "one X two");
        editor.select_all();
        editor.insert("👩‍💻").unwrap();
        assert_eq!(editor.cursor(), editor.text().len());
    }
    #[test]
    fn composition_is_transient_and_commit_replaces_once() {
        let mut editor = TextEditor::new("before after").unwrap();
        editor.select(0, 6).unwrap();
        editor.set_preedit("日本", 3..6).unwrap();
        assert_eq!(editor.text(), "before after");
        editor.cancel_composition();
        assert_eq!(editor.selected_text(), "before");
        editor.set_preedit("日本", 6..6).unwrap();
        editor.commit("日本語").unwrap();
        assert_eq!(editor.text(), "日本語 after");
        assert!(!editor.composing());
        assert!(editor.set_preedit("日本", 1..2).is_err());
        assert!(!editor.composing());
    }
}
