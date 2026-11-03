use super::{EditError, MAX_EDIT_BYTES};
use crate::{
    context::Context,
    font::{FontError, FontId},
    id::Id,
};
use alloc::{string::String, vec::Vec};
use core::ops::Range;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEventKind {
    Preedit = 0,
    Commit = 1,
    Cancel = 2,
    PreeditHidden = 3,
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardAction {
    Copy = 0,
    Cut = 1,
    Paste = 2,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextCaret {
    pub owner: Id,
    pub rect: [f32; 4],
    pub composing: bool,
}
pub struct ClipboardRequest<'a> {
    pub id: u64,
    pub owner: Id,
    pub action: ClipboardAction,
    pub text: &'a str,
}
pub(crate) struct Request {
    id: u64,
    owner: Id,
    action: ClipboardAction,
    text: String,
    range: Range<usize>,
    revision: u64,
}
pub(crate) enum Event {
    Ime {
        owner: Id,
        kind: TextEventKind,
        text: String,
        cursor: Range<usize>,
    },
    Clipboard {
        owner: Id,
        action: ClipboardAction,
        text: String,
        range: Range<usize>,
        revision: u64,
    },
}
#[derive(Default)]
pub(crate) struct TextHost {
    pub events: Vec<Event>,
    pub caret: Option<TextCaret>,
    request: Option<Request>,
    next_id: u64,
}
impl TextHost {
    pub fn request(
        &mut self,
        owner: Id,
        action: ClipboardAction,
        text: &str,
        range: Range<usize>,
        revision: u64,
    ) {
        self.next_id = self.next_id.wrapping_add(1).max(1);
        if let Some(r) = &mut self.request {
            r.text.clear();
            r.text.push_str(text);
            r.id = self.next_id;
            r.owner = owner;
            r.action = action;
            r.range = range;
            r.revision = revision;
        } else {
            self.request = Some(Request {
                id: self.next_id,
                owner,
                action,
                text: String::from(text),
                range,
                revision,
            });
        }
    }
}
impl Context {
    pub fn submit_text_event(
        &mut self,
        owner: Id,
        kind: TextEventKind,
        text: &str,
        cursor: Range<usize>,
    ) -> Result<(), EditError> {
        if self.frame_active
            || text.len() > MAX_EDIT_BYTES
            || text.contains('\0')
            || cursor.start > cursor.end
            || cursor.end > text.len()
            || !text.is_char_boundary(cursor.start)
            || !text.is_char_boundary(cursor.end)
            || self.text_host.events.len() >= 32
        {
            return Err(EditError::InvalidInput);
        }
        self.text_host.events.push(Event::Ime {
            owner,
            kind,
            text: String::from(text),
            cursor,
        });
        Ok(())
    }
    pub fn text_caret(&self) -> Option<TextCaret> {
        self.text_host
            .caret
            .filter(|c| self.focused == Some(c.owner))
    }
    pub fn clipboard_request(&self) -> Option<ClipboardRequest<'_>> {
        self.text_host
            .request
            .as_ref()
            .filter(|r| r.id != 0)
            .map(|r| ClipboardRequest {
                id: r.id,
                owner: r.owner,
                action: r.action,
                text: &r.text,
            })
    }
    pub fn complete_clipboard(&mut self, id: u64, text: Option<&str>) -> Result<bool, EditError> {
        if self.frame_active || text.is_some_and(|s| s.len() > MAX_EDIT_BYTES || s.contains('\0')) {
            return Err(EditError::InvalidInput);
        }
        let Some(r) = self
            .text_host
            .request
            .as_mut()
            .filter(|r| r.id == id && id != 0)
        else {
            return Ok(false);
        };
        if let Some(text) = text {
            if self.focused == Some(r.owner) && r.action != ClipboardAction::Copy {
                if self.text_host.events.len() >= 32 {
                    return Err(EditError::InvalidInput);
                }
                self.text_host.events.push(Event::Clipboard {
                    owner: r.owner,
                    action: r.action,
                    text: String::from(text),
                    range: r.range.clone(),
                    revision: r.revision,
                });
            }
        }
        r.id = 0;
        Ok(true)
    }
    pub fn set_font_fallback_chain(
        &mut self,
        primary: FontId,
        chain: &[FontId],
    ) -> Result<(), FontError> {
        assert!(
            !self.frame_active,
            "fallback configuration is between frames"
        );
        self.font.set_fallback_chain(primary, chain)
    }
    pub fn font_fallback_chain(&self, primary: FontId) -> Option<&[FontId]> {
        self.font.fallback_chain(primary)
    }
}
