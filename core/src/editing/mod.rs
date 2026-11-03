mod host;
pub(crate) mod model;
pub use host::{ClipboardAction, ClipboardRequest, TextCaret, TextEventKind};
pub(crate) use host::{Event, TextHost};
pub use model::{
    is_grapheme_boundary, next_grapheme, previous_grapheme, EditError, TextEditor, MAX_EDIT_BYTES,
};

mod widget;
pub use widget::{TextEditOptions, TextEditResponse};

#[cfg(test)]
mod tests;
