use crate::{
    ClipboardRequest, EditError, FontError, FontId, Gui, Id, ShapedOptions, ShapedText, TextCaret,
    TextEditOptions, TextEditResponse, TextEditor, TextEventKind,
};
impl Gui {
    pub fn set_color_atlas_texture(
        &mut self,
        texture: Option<crate::TextureId>,
    ) -> Result<(), crate::ColorFontError> {
        self.ctx.set_color_atlas_texture(texture)
    }
    pub fn color_atlas_texture(&self) -> Option<crate::TextureId> {
        self.ctx.font().color_atlas_texture()
    }
    pub fn color_atlas_pixels(&self) -> (&[u8], u32, u32) {
        self.ctx.font().color_atlas_pixels()
    }
    pub fn color_atlas_generation(&self) -> u64 {
        self.ctx.font().color_atlas_generation()
    }
    pub fn text_edit(
        &mut self,
        label: &str,
        editor: &mut TextEditor,
        options: TextEditOptions,
    ) -> Result<Option<TextEditResponse>, EditError> {
        if !self.emitting() {
            return Ok(None);
        }
        self.ctx.ui().text_edit(label, editor, options).map(Some)
    }
    pub fn shaped_text(
        &mut self,
        layout: &mut ShapedText,
        text: &str,
        options: ShapedOptions,
    ) -> Result<Option<[f32; 4]>, EditError> {
        if !self.emitting() {
            return Ok(None);
        }
        self.ctx.ui().shaped_text(layout, text, options).map(Some)
    }
    pub fn submit_text_event(
        &mut self,
        owner: Id,
        kind: TextEventKind,
        text: &str,
        cursor: core::ops::Range<usize>,
    ) -> Result<(), EditError> {
        self.ctx.submit_text_event(owner, kind, text, cursor)
    }
    pub fn text_caret(&self) -> Option<TextCaret> {
        self.ctx.text_caret()
    }
    pub fn clipboard_request(&self) -> Option<ClipboardRequest<'_>> {
        self.ctx.clipboard_request()
    }
    pub fn complete_clipboard(&mut self, id: u64, text: Option<&str>) -> Result<bool, EditError> {
        self.ctx.complete_clipboard(id, text)
    }
    pub fn set_font_fallback_chain(
        &mut self,
        primary: FontId,
        chain: &[FontId],
    ) -> Result<(), FontError> {
        self.ctx.set_font_fallback_chain(primary, chain)
    }
    pub fn font_fallback_chain(&self, primary: FontId) -> Option<&[FontId]> {
        self.ctx.font_fallback_chain(primary)
    }
}
