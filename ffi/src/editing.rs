use crate::{
    active_frame_ctx,
    data::{invalid, target},
    gui_export, slice_arg, str_arg, ContainerScope, GuiContext, GuiErrorCode, GuiStr, GuiVec2,
    WindowState,
};
use tacita::{
    editing::{TextEditOptions, TextEditor, TextEventKind, MAX_EDIT_BYTES},
    font::{CaretPosition, FontId, ShapedOptions, ShapedText, TextDirection},
    id::Id,
};
pub const GUI_EDIT_MAX_BYTES: u32 = MAX_EDIT_BYTES as u32;
#[no_mangle]
pub unsafe extern "C" fn gui_set_color_font_texture(ctx: *mut GuiContext, texture: u64) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { between(ctx) }) else {
                return false;
            };
            let texture = if texture == 0 {
                None
            } else {
                Some(tacita::draw::TextureId(texture))
            };
            if gui.ctx.set_color_atlas_texture(texture).is_err() {
                invalid();
                return false;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_color_atlas_pixels(
    ctx: *mut GuiContext,
    w: *mut u32,
    h: *mut u32,
    len: *mut usize,
    generation: *mut u64,
) -> *const u8 {
    gui_export!(
        || {
            let Some(gui) = (unsafe { target(ctx) }) else {
                return core::ptr::null();
            };
            if w.is_null()
                || !w.is_aligned()
                || h.is_null()
                || !h.is_aligned()
                || len.is_null()
                || !len.is_aligned()
                || generation.is_null()
                || !generation.is_aligned()
            {
                invalid();
                return core::ptr::null();
            }
            let (pixels, width, height) = gui.ctx.font().color_atlas_pixels();
            unsafe {
                *w = width;
                *h = height;
                *len = pixels.len();
                *generation = gui.ctx.font().color_atlas_generation();
            }
            if pixels.is_empty() {
                core::ptr::null()
            } else {
                pixels.as_ptr()
            }
        },
        core::ptr::null()
    )
}
pub const GUI_EDIT_MULTILINE: u32 = 1;
pub const GUI_EDIT_WRAP: u32 = 2;
pub const GUI_EDIT_READ_ONLY: u32 = 4;
pub const GUI_DIRECTION_AUTO: u32 = 0;
pub const GUI_DIRECTION_LTR: u32 = 1;
pub const GUI_DIRECTION_RTL: u32 = 2;
pub const GUI_IME_PREEDIT: u32 = 0;
pub const GUI_IME_COMMIT: u32 = 1;
pub const GUI_IME_CANCEL: u32 = 2;
pub const GUI_IME_PREEDIT_HIDDEN: u32 = 3;
pub const GUI_CLIPBOARD_COPY: u32 = 0;
pub const GUI_CLIPBOARD_CUT: u32 = 1;
pub const GUI_CLIPBOARD_PASTE: u32 = 2;
pub const GUI_EDIT_DOCUMENT: u32 = 0;
pub const GUI_EDIT_SELECTION: u32 = 1;
pub const GUI_EDIT_PREEDIT: u32 = 2;
pub struct GuiTextEditor(TextEditor);
pub struct GuiShapedText(ShapedText);
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiTextEditOptions {
    pub size: GuiVec2,
    pub flags: u32,
    pub direction: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiShapedOptions {
    pub width: f32,
    pub wrap: u8,
    pub reserved: [u8; 3],
    pub direction: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiTextSelection {
    pub anchor: usize,
    pub cursor: usize,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiTextEditResponse {
    pub id: u64,
    pub rect: [f32; 4],
    pub changed: u8,
    pub focused: u8,
    pub reserved: [u8; 6],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiTextCaret {
    pub owner: u64,
    pub rect: [f32; 4],
    pub composing: u8,
    pub reserved: [u8; 7],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiClipboardRequest {
    pub request_id: u64,
    pub owner: u64,
    pub action: u32,
    pub reserved: u32,
    pub text: GuiStr,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GuiShapedCaret {
    pub byte: usize,
    pub position: GuiVec2,
    pub line: usize,
}
impl From<CaretPosition> for GuiShapedCaret {
    fn from(c: CaretPosition) -> Self {
        Self {
            byte: c.byte,
            position: c.position.into(),
            line: c.line,
        }
    }
}
fn direction(code: u32) -> Option<TextDirection> {
    match code {
        0 => Some(TextDirection::Auto),
        1 => Some(TextDirection::LeftToRight),
        2 => Some(TextDirection::RightToLeft),
        _ => None,
    }
}
impl GuiTextEditOptions {
    fn decode(self) -> Option<TextEditOptions> {
        if self.flags & !7 != 0
            || [self.size.x, self.size.y]
                .iter()
                .any(|v| !v.is_finite() || *v < 0.)
            || self.flags & 2 != 0 && self.flags & 1 == 0
        {
            return None;
        }
        Some(TextEditOptions {
            size: self.size.into(),
            multiline: self.flags & 1 != 0,
            wrap: self.flags & 2 != 0,
            read_only: self.flags & 4 != 0,
            direction: direction(self.direction)?,
        })
    }
}
impl GuiShapedOptions {
    fn decode(self) -> Option<ShapedOptions> {
        if self.wrap > 1
            || self.reserved != [0; 3]
            || !self.width.is_finite()
            || self.width < 0.
            || self.wrap != 0 && self.width == 0.
        {
            return None;
        }
        Some(ShapedOptions {
            width: self.width,
            wrap: self.wrap != 0,
            direction: direction(self.direction)?,
        })
    }
}
unsafe fn text<'a>(s: GuiStr) -> Option<&'a str> {
    if s.len > MAX_EDIT_BYTES {
        invalid();
        return None;
    }
    let result = unsafe { str_arg(s.ptr, s.len) }.filter(|s| !s.contains('\0'));
    if result.is_none() {
        invalid();
    }
    result
}
unsafe fn between<'a>(ctx: *mut GuiContext) -> Option<&'a mut GuiContext> {
    let gui = unsafe { target(ctx) }?;
    if gui.frame_active {
        crate::set_last_error(GuiErrorCode::ContractViolation);
        return None;
    }
    Some(gui)
}
unsafe fn declaring<'a>(ctx: *mut GuiContext) -> Option<&'a mut GuiContext> {
    if ctx.is_null() || !ctx.is_aligned() {
        invalid();
        return None;
    }
    unsafe { active_frame_ctx(ctx) }
}
fn suppressed(gui: &GuiContext) -> bool {
    matches!(gui.window, WindowState::Closed)
        || gui.containers.iter().any(|s| {
            matches!(
                s,
                ContainerScope::Header(None)
                    | ContainerScope::Tree(None)
                    | ContainerScope::Popup(false)
            )
        })
}
unsafe fn copy_bytes(source: &str, out: *mut u8, capacity: usize) -> usize {
    let required = source.len();
    if out.is_null() && capacity == 0 {
        return required;
    }
    if out.is_null()
        || capacity < required
        || capacity > isize::MAX as usize
        || (out as usize).checked_add(capacity).is_none()
    {
        invalid();
        return required;
    }
    unsafe { core::ptr::copy_nonoverlapping(source.as_ptr(), out, required) };
    required
}
#[no_mangle]
pub unsafe extern "C" fn gui_text_editor_new(value: GuiStr) -> *mut GuiTextEditor {
    gui_export!(
        || {
            let Some(s) = (unsafe { text(value) }) else {
                return core::ptr::null_mut();
            };
            match TextEditor::new(s) {
                Ok(e) => Box::into_raw(Box::new(GuiTextEditor(e))),
                Err(_) => {
                    invalid();
                    core::ptr::null_mut()
                }
            }
        },
        core::ptr::null_mut()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_text_editor_free(editor: *mut GuiTextEditor) {
    gui_export!(
        || {
            if !editor.is_null() {
                if !editor.is_aligned() {
                    invalid();
                    return;
                }
                unsafe {
                    drop(Box::from_raw(editor));
                }
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_text_editor_set(editor: *mut GuiTextEditor, value: GuiStr) -> bool {
    gui_export!(
        || {
            let Some(e) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some(s) = (unsafe { text(value) }) else {
                return false;
            };
            if e.0.replace(s).is_err() {
                invalid();
                return false;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_text_editor_copy(
    editor: *mut GuiTextEditor,
    kind: u32,
    out: *mut u8,
    capacity: usize,
) -> usize {
    gui_export!(
        || {
            let Some(e) = (unsafe { target(editor) }) else {
                return 0;
            };
            let s = match kind {
                0 => e.0.text(),
                1 => e.0.selected_text(),
                2 => e.0.preedit(),
                _ => {
                    invalid();
                    return 0;
                }
            };
            unsafe { copy_bytes(s, out, capacity) }
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_text_editor_select(
    editor: *mut GuiTextEditor,
    anchor: usize,
    cursor: usize,
) -> bool {
    gui_export!(
        || {
            let Some(e) = (unsafe { target(editor) }) else {
                return false;
            };
            if e.0.select(anchor, cursor).is_err() {
                invalid();
                return false;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_text_editor_selection(
    editor: *mut GuiTextEditor,
    out: *mut GuiTextSelection,
) -> bool {
    gui_export!(
        || {
            let Some(e) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            *out = GuiTextSelection {
                anchor: e.0.anchor(),
                cursor: e.0.cursor(),
            };
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_text_edit(
    ctx: *mut GuiContext,
    label: GuiStr,
    editor: *mut GuiTextEditor,
    options: GuiTextEditOptions,
    out: *mut GuiTextEditResponse,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { declaring(ctx) }) else {
                return false;
            };
            let Some(label) = (unsafe { text(label) }) else {
                return false;
            };
            let Some(e) = (unsafe { target(editor) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            let Some(options) = options.decode() else {
                invalid();
                return false;
            };
            if suppressed(gui) {
                return true;
            }
            match gui.ctx.ui().text_edit(label, &mut e.0, options) {
                Ok(r) => {
                    *out = GuiTextEditResponse {
                        id: r.id.0,
                        rect: r.rect,
                        changed: r.changed as u8,
                        focused: r.focused as u8,
                        reserved: [0; 6],
                    };
                    true
                }
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_text_event(
    ctx: *mut GuiContext,
    owner: u64,
    kind: u32,
    value: GuiStr,
    anchor: usize,
    cursor: usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { between(ctx) }) else {
                return false;
            };
            let Some(value) = (unsafe { text(value) }) else {
                return false;
            };
            let kind = match kind {
                0 => TextEventKind::Preedit,
                1 => TextEventKind::Commit,
                2 => TextEventKind::Cancel,
                3 => TextEventKind::PreeditHidden,
                _ => {
                    invalid();
                    return false;
                }
            };
            if gui
                .ctx
                .submit_text_event(Id(owner), kind, value, anchor..cursor)
                .is_err()
            {
                invalid();
                return false;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_text_caret(ctx: *mut GuiContext, out: *mut GuiTextCaret) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { target(ctx) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            let Some(c) = gui.ctx.text_caret() else {
                return false;
            };
            *out = GuiTextCaret {
                owner: c.owner.0,
                rect: c.rect,
                composing: c.composing as u8,
                reserved: [0; 7],
            };
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_clipboard_request(
    ctx: *mut GuiContext,
    out: *mut GuiClipboardRequest,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { target(ctx) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            let Some(r) = gui.ctx.clipboard_request() else {
                return false;
            };
            *out = GuiClipboardRequest {
                request_id: r.id,
                owner: r.owner.0,
                action: r.action as u32,
                reserved: 0,
                text: GuiStr {
                    ptr: r.text.as_ptr(),
                    len: r.text.len(),
                },
            };
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_clipboard_complete(
    ctx: *mut GuiContext,
    id: u64,
    success: u8,
    value: GuiStr,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { between(ctx) }) else {
                return false;
            };
            if success > 1 {
                invalid();
                return false;
            }
            let Some(s) = (unsafe { text(value) }) else {
                return false;
            };
            match gui
                .ctx
                .complete_clipboard(id, if success == 1 { Some(s) } else { None })
            {
                Ok(v) => v,
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_set_font_fallback(
    ctx: *mut GuiContext,
    primary: u32,
    ids: *const u32,
    count: usize,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { between(ctx) }) else {
                return false;
            };
            if count > 16 {
                invalid();
                return false;
            }
            let Some(ids) = (unsafe { slice_arg(ids, count) }) else {
                invalid();
                return false;
            };
            let mut chain = [FontId::DEFAULT; 16];
            for (i, id) in ids.iter().enumerate() {
                chain[i] = FontId(*id);
            }
            if gui
                .ctx
                .set_font_fallback_chain(FontId(primary), &chain[..count])
                .is_err()
            {
                invalid();
                return false;
            }
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_font_fallback(
    ctx: *mut GuiContext,
    primary: u32,
    out: *mut u32,
    capacity: usize,
) -> usize {
    gui_export!(
        || {
            let Some(gui) = (unsafe { target(ctx) }) else {
                return 0;
            };
            let Some(ids) = gui.ctx.font_fallback_chain(FontId(primary)) else {
                invalid();
                return 0;
            };
            let n = ids.len();
            if out.is_null() && capacity == 0 {
                return n;
            }
            if out.is_null()
                || !out.is_aligned()
                || capacity < n
                || capacity > isize::MAX as usize / 4
                || (out as usize)
                    .checked_add(capacity.saturating_mul(4))
                    .is_none()
            {
                invalid();
                return n;
            }
            for (i, id) in ids.iter().enumerate() {
                unsafe {
                    out.add(i).write(id.0);
                }
            }
            n
        },
        0
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_shaped_text_new() -> *mut GuiShapedText {
    gui_export!(
        || Box::into_raw(Box::new(GuiShapedText(ShapedText::default()))),
        core::ptr::null_mut()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_shaped_text_free(layout: *mut GuiShapedText) {
    gui_export!(
        || {
            if !layout.is_null() {
                if !layout.is_aligned() {
                    invalid();
                    return;
                }
                unsafe {
                    drop(Box::from_raw(layout));
                }
            }
        },
        ()
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_shaped_text(
    ctx: *mut GuiContext,
    layout: *mut GuiShapedText,
    value: GuiStr,
    options: GuiShapedOptions,
    size: *mut GuiVec2,
) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { declaring(ctx) }) else {
                return false;
            };
            let Some(l) = (unsafe { target(layout) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(size) }) else {
                return false;
            };
            let Some(s) = (unsafe { text(value) }) else {
                return false;
            };
            let Some(options) = options.decode() else {
                invalid();
                return false;
            };
            if suppressed(gui) {
                return true;
            }
            match gui.ctx.ui().shaped_text(&mut l.0, s, options) {
                Ok(_) => {
                    *out = l.0.size().into();
                    true
                }
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_shaped_caret(
    layout: *mut GuiShapedText,
    byte: usize,
    out: *mut GuiShapedCaret,
) -> bool {
    gui_export!(
        || {
            let Some(l) = (unsafe { target(layout) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            let Some(c) = l.0.caret(byte) else {
                return false;
            };
            *out = c.into();
            true
        },
        false
    )
}
#[no_mangle]
pub unsafe extern "C" fn gui_shaped_hit_test(
    layout: *mut GuiShapedText,
    point: GuiVec2,
    out: *mut GuiShapedCaret,
) -> bool {
    gui_export!(
        || {
            let Some(l) = (unsafe { target(layout) }) else {
                return false;
            };
            let Some(out) = (unsafe { target(out) }) else {
                return false;
            };
            if !point.x.is_finite() || !point.y.is_finite() {
                invalid();
                return false;
            }
            let Some(c) = l.0.hit_test(point.into()) else {
                return false;
            };
            *out = c.into();
            true
        },
        false
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_clipboard_process_native(ctx: *mut GuiContext) -> bool {
    gui_export!(
        || {
            let Some(gui) = (unsafe { between(ctx) }) else {
                return false;
            };
            match crate::clipboard::process(&mut gui.ctx) {
                Ok(v) => v,
                Err(_) => {
                    invalid();
                    false
                }
            }
        },
        false
    )
}

#[cfg(test)]
#[path = "editing_tests.rs"]
mod tests;
