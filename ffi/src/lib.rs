use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use restricted_gui::context::Context;
mod host_services;
pub use host_services::*;
#[path = "../../host/src/accessibility/mod.rs"]
pub mod accessibility_host;
use restricted_gui::font::FontId;
use restricted_gui::input::{InputState, Key, KeyEvent};
use restricted_gui::panels::{
    DockPanelScope, DockspaceScope, PanelCellScope, ResizableChildScope, SplitScope, TableScope,
};
use restricted_gui::ui::{ChildScope, CollapsingHeaderScope, DisabledScope, TreeNodeScope};

mod animation;
mod canvas;
#[path = "../../host/src/clipboard.rs"]
mod clipboard;
mod custom;
mod data;
mod data_editors;
mod diagnostics;
mod editing;
mod effects;
mod images;
mod layout;
mod panels;
mod paths;
mod rendering;
mod shapes;
mod svg;
mod text;
mod text_raster;
mod theme;
pub use data::*;
pub use data_editors::*;
pub use editing::*;
mod input_state;
#[path = "../../host/src/style_watch.rs"]
pub mod style_watch;
mod types;
mod widgets;
mod windows;
pub use input_state::*;

#[cfg(test)]
mod header_consistency;
#[cfg(test)]
mod test_support;

pub use animation::*;
pub use canvas::*;
pub use custom::*;
pub use diagnostics::*;
pub use effects::*;
pub use images::*;
pub use layout::*;
pub use panels::*;
pub use paths::*;
pub use rendering::*;
pub use shapes::*;
pub use svg::*;
pub use text::*;
pub use text_raster::*;
pub use theme::*;
pub use types::*;
pub use widgets::*;
pub use windows::*;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiErrorCode {
    None = 0,
    ContractViolation = 1,
    Panic = 2,
    InvalidInput = 3,
}

thread_local! {
    static LAST_ERROR: Cell<u8> = const { Cell::new(GuiErrorCode::None as u8) };
}

fn set_last_error(code: GuiErrorCode) {
    let _ = LAST_ERROR.try_with(|e| e.set(code as u8));
}

#[no_mangle]
pub extern "C" fn gui_last_error_code() -> u8 {
    catch_unwind(|| LAST_ERROR.with(Cell::get)).unwrap_or(GuiErrorCode::Panic as u8)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiVert {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub col: [u8; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiCmd {
    pub elem_count: u32,
    pub clip_rect: [f32; 4],
    pub effects: u32,
    pub texture: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiDrawData {
    pub vertices: *const GuiVert,
    pub vertex_count: u32,
    pub indices: *const u32,
    pub index_count: u32,
    pub commands: *const GuiCmd,
    pub command_count: u32,
}

impl GuiDrawData {
    const ZERO: Self = Self {
        vertices: ptr::null(),
        vertex_count: 0,
        indices: ptr::null(),
        index_count: 0,
        commands: ptr::null(),
        command_count: 0,
    };
}

pub const GUI_KEY_EVENT_PRESS: u8 = 0;
pub const GUI_KEY_EVENT_RELEASE: u8 = 1;
pub const GUI_KEY_EVENT_CHAR: u8 = 2;

pub const GUI_FONT_ID_INVALID: u32 = u32::MAX;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiKeyEvent {
    pub kind: u8,
    pub key: u16,
    pub ch: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuiInput {
    pub mouse_pos: [f32; 2],
    pub mouse_down: [u8; 5],
    pub mouse_wheel: f32,
    pub viewport_size: [f32; 2],
    pub keys: *const GuiKeyEvent,
    pub key_count: usize,
}

const ALL_KEYS: [Key; 64] = [
    Key::Backspace,
    Key::Delete,
    Key::Left,
    Key::Right,
    Key::Home,
    Key::End,
    Key::Enter,
    Key::Escape,
    Key::Insert,
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
    Key::Num0,
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
    Key::Space,
    Key::Tab,
    Key::Shift,
    Key::Ctrl,
    Key::Alt,
    Key::Up,
    Key::Down,
];

pub(crate) fn key_to_code(key: Key) -> u16 {
    match key {
        Key::Backspace => 0,
        Key::Delete => 1,
        Key::Left => 2,
        Key::Right => 3,
        Key::Home => 4,
        Key::End => 5,
        Key::Enter => 6,
        Key::Escape => 7,
        Key::Insert => 8,
        Key::A => 9,
        Key::B => 10,
        Key::C => 11,
        Key::D => 12,
        Key::E => 13,
        Key::F => 14,
        Key::G => 15,
        Key::H => 16,
        Key::I => 17,
        Key::J => 18,
        Key::K => 19,
        Key::L => 20,
        Key::M => 21,
        Key::N => 22,
        Key::O => 23,
        Key::P => 24,
        Key::Q => 25,
        Key::R => 26,
        Key::S => 27,
        Key::T => 28,
        Key::U => 29,
        Key::V => 30,
        Key::W => 31,
        Key::X => 32,
        Key::Y => 33,
        Key::Z => 34,
        Key::Num0 => 35,
        Key::Num1 => 36,
        Key::Num2 => 37,
        Key::Num3 => 38,
        Key::Num4 => 39,
        Key::Num5 => 40,
        Key::Num6 => 41,
        Key::Num7 => 42,
        Key::Num8 => 43,
        Key::Num9 => 44,
        Key::F1 => 45,
        Key::F2 => 46,
        Key::F3 => 47,
        Key::F4 => 48,
        Key::F5 => 49,
        Key::F6 => 50,
        Key::F7 => 51,
        Key::F8 => 52,
        Key::F9 => 53,
        Key::F10 => 54,
        Key::F11 => 55,
        Key::F12 => 56,
        Key::Space => 57,
        Key::Tab => 58,
        Key::Shift => 59,
        Key::Ctrl => 60,
        Key::Alt => 61,
        Key::Up => 62,
        Key::Down => 63,
    }
}

fn key_from_code(code: u16) -> Option<Key> {
    ALL_KEYS.get(code as usize).copied()
}

fn translate_key_event(event: &GuiKeyEvent) -> Option<KeyEvent> {
    match event.kind {
        GUI_KEY_EVENT_PRESS => key_from_code(event.key).map(KeyEvent::Press),
        GUI_KEY_EVENT_RELEASE => key_from_code(event.key).map(KeyEvent::Release),
        GUI_KEY_EVENT_CHAR => char::from_u32(event.ch).map(KeyEvent::Char),
        _ => None,
    }
}

macro_rules! gui_export {
    ($body:expr, $fallback:expr) => {{
        $crate::set_last_error($crate::GuiErrorCode::None);
        match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe($body)) {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(_) => {
                $crate::set_last_error($crate::GuiErrorCode::Panic);
                $fallback
            }
        }
    }};
}
pub(crate) use gui_export;

pub(crate) unsafe fn str_arg<'a>(ptr: *const u8, len: usize) -> Option<&'a str> {
    let bytes = unsafe { slice_arg(ptr, len) }?;
    std::str::from_utf8(bytes).ok()
}

pub(crate) unsafe fn slice_arg<'a, T>(ptr: *const T, len: usize) -> Option<&'a [T]> {
    if len == 0 {
        return Some(&[]);
    }
    let element_size = std::mem::size_of::<T>();
    if ptr.is_null() || element_size == 0 || !ptr.is_aligned() {
        return None;
    }
    let byte_len = len.checked_mul(element_size)?;
    if byte_len > isize::MAX as usize || (ptr as usize).checked_add(byte_len).is_none() {
        return None;
    }
    Some(unsafe { std::slice::from_raw_parts(ptr, len) })
}

pub(crate) unsafe fn active_frame_ctx<'a>(ctx: *mut GuiContext) -> Option<&'a mut GuiContext> {
    if ctx.is_null() {
        set_last_error(GuiErrorCode::InvalidInput);
        return None;
    }
    let gui = unsafe { &mut *ctx };
    if !gui.frame_active {
        set_last_error(GuiErrorCode::ContractViolation);
        return None;
    }
    Some(gui)
}

pub struct GuiContext {
    pub(crate) ctx: Context,
    pub(crate) state: input_state::StateBridge,
    pub(crate) frame_active: bool,
    last_draw: GuiDrawData,
    last_effects: *const GuiDrawEffects,
    last_effect_count: usize,
    commands_staging: Vec<GuiCmd>,
    key_events_staging: Vec<KeyEvent>,
    pub(crate) text_input_staging: String,
    pub(crate) window: WindowState,
    pub(crate) containers: Vec<ContainerScope>,
    pub(crate) font_scope_depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContainerKind {
    Rows,
    DataTable,
    DataCell,
    MenuBar,
    PropertyGrid,
    PropertyRow,
    DataMenu,

    Stencil,
    Mask,
    RoundedClip,
    Layout,
    LayoutItem,
    AnimatedLayout,
    Header,
    Tree,
    ScrollRegion,
    Child,
    GroupBox,
    PreviewCanvas,
    Disabled,
    TabBar,
    Popup,
    Table,
    Split,
    PanelCell,
    ResizableChild,
    Dockspace,
    DockPanel,
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum ContainerScope {
    Rows(restricted_gui::data::RowClipperScope),
    DataTable(restricted_gui::data::DataTableScope),
    DataCell(restricted_gui::data::DataCellScope),
    MenuBar(restricted_gui::data::MenuBarScope),
    PropertyGrid(restricted_gui::data::PropertyGridScope),
    PropertyRow(restricted_gui::data::DataCellScope),
    DataMenu,

    Stencil(restricted_gui::effects::StencilScope),
    Mask(restricted_gui::effects::MaskScope),
    RoundedClip(restricted_gui::shapes::RoundedClipScope),
    Layout(restricted_gui::layout::LayoutScope),
    LayoutItem(restricted_gui::layout::LayoutItemScope),
    AnimatedLayout(restricted_gui::animation::AnimatedLayoutScope),
    Header(Option<CollapsingHeaderScope>),
    Tree(Option<TreeNodeScope>),
    ScrollRegion(ChildScope),
    Child(ChildScope),
    GroupBox(ChildScope),
    PreviewCanvas(ChildScope),
    Disabled(DisabledScope),
    TabBar,
    Popup(bool),
    Table(TableScope),
    Split(SplitScope),
    PanelCell(PanelCellScope),
    ResizableChild(ResizableChildScope),
    Dockspace(DockspaceScope),
    DockPanel(DockPanelScope),
}

impl ContainerScope {
    pub(crate) fn kind(&self) -> ContainerKind {
        match self {
            Self::Rows(_) => ContainerKind::Rows,
            Self::DataTable(_) => ContainerKind::DataTable,
            Self::DataCell(_) => ContainerKind::DataCell,
            Self::MenuBar(_) => ContainerKind::MenuBar,
            Self::PropertyGrid(_) => ContainerKind::PropertyGrid,
            Self::PropertyRow(_) => ContainerKind::PropertyRow,
            Self::DataMenu => ContainerKind::DataMenu,

            Self::Stencil(_) => ContainerKind::Stencil,
            Self::Mask(_) => ContainerKind::Mask,
            Self::RoundedClip(_) => ContainerKind::RoundedClip,
            Self::Layout(_) => ContainerKind::Layout,
            Self::LayoutItem(_) => ContainerKind::LayoutItem,
            Self::AnimatedLayout(_) => ContainerKind::AnimatedLayout,
            Self::Header(_) => ContainerKind::Header,
            Self::Tree(_) => ContainerKind::Tree,
            Self::ScrollRegion(_) => ContainerKind::ScrollRegion,
            Self::Child(_) => ContainerKind::Child,
            Self::GroupBox(_) => ContainerKind::GroupBox,
            Self::PreviewCanvas(_) => ContainerKind::PreviewCanvas,
            Self::Disabled(_) => ContainerKind::Disabled,
            Self::TabBar => ContainerKind::TabBar,
            Self::Popup(_) => ContainerKind::Popup,
            Self::Table(_) => ContainerKind::Table,
            Self::Split(_) => ContainerKind::Split,
            Self::PanelCell(_) => ContainerKind::PanelCell,
            Self::ResizableChild(_) => ContainerKind::ResizableChild,
            Self::Dockspace(_) => ContainerKind::Dockspace,
            Self::DockPanel(_) => ContainerKind::DockPanel,
        }
    }
}

pub(crate) enum WindowState {
    None,
    Open,
    Closed,
}

#[no_mangle]
pub unsafe extern "C" fn gui_context_new(font_bytes: *const u8, len: usize) -> *mut GuiContext {
    set_last_error(GuiErrorCode::None);
    catch_unwind(AssertUnwindSafe(|| {
        if font_bytes.is_null() || len == 0 {
            set_last_error(GuiErrorCode::InvalidInput);
            return ptr::null_mut();
        }
        let Some(bytes) = (unsafe { slice_arg(font_bytes, len) }) else {
            set_last_error(GuiErrorCode::InvalidInput);
            return ptr::null_mut();
        };
        match Context::new(bytes) {
            Ok(ctx) => Box::into_raw(Box::new(GuiContext {
                ctx,
                state: input_state::StateBridge::new(),
                frame_active: false,
                last_draw: GuiDrawData::ZERO,
                last_effects: ptr::null(),
                last_effect_count: 0,
                commands_staging: Vec::new(),
                key_events_staging: Vec::new(),
                text_input_staging: String::new(),
                window: WindowState::None,
                containers: Vec::new(),
                font_scope_depth: 0,
            })),
            Err(_) => {
                set_last_error(GuiErrorCode::InvalidInput);
                ptr::null_mut()
            }
        }
    }))
    .unwrap_or_else(|_| {
        set_last_error(GuiErrorCode::Panic);
        ptr::null_mut()
    })
}

#[no_mangle]
pub unsafe extern "C" fn gui_context_free(ctx: *mut GuiContext) {
    set_last_error(GuiErrorCode::None);
    let result = catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() {
            return;
        }
        drop(unsafe { Box::from_raw(ctx) });
    }));
    if result.is_err() {
        set_last_error(GuiErrorCode::Panic);
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_new_frame(ctx: *mut GuiContext, input: *const GuiInput, dt: f32) {
    set_last_error(GuiErrorCode::None);
    let result = catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() {
            set_last_error(GuiErrorCode::InvalidInput);
            return;
        }
        let raw = if input.is_null() {
            None
        } else {
            Some(unsafe { &*input })
        };
        let events = if let Some(raw) = raw {
            if !raw.keys.is_null() && raw.key_count > 0 {
                let Some(events) = (unsafe { slice_arg(raw.keys, raw.key_count) }) else {
                    set_last_error(GuiErrorCode::InvalidInput);
                    return;
                };
                events
            } else {
                &[]
            }
        } else {
            &[]
        };
        let gui = unsafe { &mut *ctx };
        if events.len() > gui.key_events_staging.capacity()
            && gui
                .key_events_staging
                .try_reserve(events.len() - gui.key_events_staging.len())
                .is_err()
        {
            set_last_error(GuiErrorCode::InvalidInput);
            return;
        }
        gui.window = WindowState::None;
        gui.containers.clear();
        gui.last_effects = std::ptr::null();
        gui.last_effect_count = 0;
        gui.font_scope_depth = 0;
        gui.key_events_staging.clear();
        let state = if let Some(raw) = raw {
            gui.key_events_staging
                .extend(events.iter().filter_map(translate_key_event));
            InputState {
                mouse_pos: raw.mouse_pos,
                mouse_down: raw.mouse_down.map(|b| b != 0),
                mouse_wheel: raw.mouse_wheel,
                keys: &gui.key_events_staging,
                viewport_size: raw.viewport_size,
            }
        } else {
            InputState::default()
        };
        gui.ctx.set_input(state);
        gui.ctx.new_frame(dt);
        gui.frame_active = true;
    }));
    if result.is_err() {
        set_last_error(GuiErrorCode::Panic);
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_render(ctx: *mut GuiContext) -> GuiDrawData {
    set_last_error(GuiErrorCode::None);
    match catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() {
            set_last_error(GuiErrorCode::InvalidInput);
            return GuiDrawData::ZERO;
        }
        let gui = unsafe { &mut *ctx };
        if gui.frame_active {
            let unbalanced = !matches!(gui.window, WindowState::None)
                || !gui.containers.is_empty()
                || gui.font_scope_depth > 0;
            if unbalanced {
                while gui.font_scope_depth > 0 {
                    gui.font_scope_depth -= 1;
                    gui.ctx.ui().pop_font();
                }
                if matches!(gui.window, WindowState::Open) {
                    while let Some(scope) = gui.containers.pop() {
                        match scope {
                            ContainerScope::Rows(token) => gui.ctx.ui().end_rows(token),
                            ContainerScope::DataTable(token) => {
                                gui.ctx.ui().end_data_table(token);
                            }
                            ContainerScope::DataCell(token) => gui.ctx.ui().end_data_cell(token),
                            ContainerScope::MenuBar(token) => gui.ctx.ui().end_menu_bar(token),
                            ContainerScope::PropertyGrid(token) => {
                                gui.ctx.ui().end_property_grid(token)
                            }
                            ContainerScope::PropertyRow(token) => {
                                gui.ctx.ui().end_property_row(token)
                            }
                            ContainerScope::DataMenu => gui.ctx.ui().end_menu(),
                            ContainerScope::Stencil(token) => gui.ctx.ui().end_stencil_clip(token),
                            ContainerScope::Mask(token) => gui.ctx.ui().end_mask(token),
                            ContainerScope::RoundedClip(token) => {
                                gui.ctx.ui().end_rounded_clip(token)
                            }
                            ContainerScope::Layout(token) => gui.ctx.ui().end_layout(token),
                            ContainerScope::LayoutItem(token) => {
                                if let Some(ContainerScope::Layout(parent)) =
                                    gui.containers.last_mut()
                                {
                                    gui.ctx.ui().end_layout_item(parent, token);
                                }
                            }
                            ContainerScope::AnimatedLayout(token) => {
                                gui.ctx.ui().end_animated_layout(token)
                            }
                            ContainerScope::Header(Some(token)) => {
                                gui.ctx.ui().end_collapsing_header(token)
                            }
                            ContainerScope::Tree(Some(token)) => gui.ctx.ui().end_tree_node(token),
                            ContainerScope::ScrollRegion(token) | ContainerScope::Child(token) => {
                                gui.ctx.ui().end_child(token)
                            }
                            ContainerScope::GroupBox(token) => gui.ctx.ui().end_group_box(token),
                            ContainerScope::PreviewCanvas(token) => {
                                gui.ctx.ui().end_preview_canvas(token)
                            }
                            ContainerScope::Disabled(token) => gui.ctx.ui().end_disabled(token),
                            ContainerScope::TabBar => gui.ctx.ui().end_tab_bar(),
                            ContainerScope::Table(token) => gui.ctx.ui().end_table(token),
                            ContainerScope::Split(token) => gui.ctx.ui().end_split(token),
                            ContainerScope::PanelCell(token) => gui.ctx.ui().end_panel_cell(token),
                            ContainerScope::ResizableChild(token) => {
                                gui.ctx.ui().end_resizable_child(token)
                            }
                            ContainerScope::Dockspace(token) => gui.ctx.ui().end_dockspace(token),
                            ContainerScope::DockPanel(token) => gui.ctx.ui().end_dock_panel(token),
                            ContainerScope::Popup(true) => gui.ctx.ui().end_popup(),
                            ContainerScope::Popup(false) => {}
                            ContainerScope::Header(None) | ContainerScope::Tree(None) => {}
                        }
                    }
                    gui.ctx.ui().end_window();
                } else {
                    gui.containers.clear();
                }
                gui.window = WindowState::None;
            }
            let data = gui.ctx.render();
            gui.frame_active = false;
            gui.commands_staging.clear();
            gui.commands_staging
                .extend(data.commands.iter().map(|cmd| GuiCmd {
                    elem_count: cmd.elem_count,
                    clip_rect: cmd.clip_rect,
                    effects: cmd.effects,
                    texture: cmd.texture.0,
                }));
            gui.last_draw = GuiDrawData {
                vertices: data.vertices.as_ptr().cast::<GuiVert>(),
                vertex_count: data.vertices.len() as u32,
                indices: data.indices.as_ptr(),
                index_count: data.indices.len() as u32,
                commands: gui.commands_staging.as_ptr(),
                command_count: gui.commands_staging.len() as u32,
            };
            gui.last_effects = data.effects.as_ptr().cast();
            gui.last_effect_count = data.effects.len();
            if unbalanced {
                set_last_error(GuiErrorCode::ContractViolation);
            }
        }
        gui.last_draw
    })) {
        Ok(draw) => draw,
        Err(_) => {
            set_last_error(GuiErrorCode::Panic);
            GuiDrawData::ZERO
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_atlas_pixels(
    ctx: *mut GuiContext,
    w: *mut u32,
    h: *mut u32,
) -> *const u8 {
    set_last_error(GuiErrorCode::None);
    match catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() || w.is_null() || h.is_null() {
            set_last_error(GuiErrorCode::InvalidInput);
            return ptr::null();
        }
        let gui = unsafe { &*ctx };
        let (pixels, atlas_w, atlas_h) = gui.ctx.font().atlas_pixels();
        unsafe {
            *w = atlas_w;
            *h = atlas_h;
        }
        pixels.as_ptr()
    })) {
        Ok(pixels) => pixels,
        Err(_) => {
            set_last_error(GuiErrorCode::Panic);
            ptr::null()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_atlas_generation(ctx: *mut GuiContext) -> u64 {
    set_last_error(GuiErrorCode::None);
    match catch_unwind(AssertUnwindSafe(|| {
        if ctx.is_null() {
            set_last_error(GuiErrorCode::InvalidInput);
            return 0;
        }
        let gui = unsafe { &*ctx };
        gui.ctx.font().atlas_generation()
    })) {
        Ok(generation) => generation,
        Err(_) => {
            set_last_error(GuiErrorCode::Panic);
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn gui_add_font(
    ctx: *mut GuiContext,
    font_bytes: *const u8,
    len: usize,
    px_size: f32,
) -> u32 {
    gui_export!(
        || {
            if ctx.is_null() || font_bytes.is_null() || len == 0 {
                set_last_error(GuiErrorCode::InvalidInput);
                return GUI_FONT_ID_INVALID;
            }
            let gui = unsafe { &mut *ctx };
            let Some(bytes) = (unsafe { slice_arg(font_bytes, len) }) else {
                set_last_error(GuiErrorCode::InvalidInput);
                return GUI_FONT_ID_INVALID;
            };
            match gui.ctx.add_font(bytes, px_size) {
                Ok(id) => id.0,
                Err(_) => {
                    set_last_error(GuiErrorCode::InvalidInput);
                    GUI_FONT_ID_INVALID
                }
            }
        },
        GUI_FONT_ID_INVALID
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_push_font(ctx: *mut GuiContext, id: u32) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if !gui.ctx.font().is_valid(FontId(id)) {
                set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            gui.ctx.ui().push_font(FontId(id));
            gui.font_scope_depth += 1;
        },
        ()
    )
}

#[no_mangle]
pub unsafe extern "C" fn gui_pop_font(ctx: *mut GuiContext) {
    gui_export!(
        || {
            let Some(gui) = (unsafe { active_frame_ctx(ctx) }) else {
                return;
            };
            if gui.font_scope_depth == 0 {
                set_last_error(GuiErrorCode::ContractViolation);
                return;
            }
            gui.ctx.ui().pop_font();
            gui.font_scope_depth -= 1;
        },
        ()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use restricted_gui::context::Context;
    use restricted_gui::draw::{DrawVert, TextureId};

    const DEJAVU_SANS: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

    fn new_context() -> *mut GuiContext {
        let ctx = unsafe { gui_context_new(DEJAVU_SANS.as_ptr(), DEJAVU_SANS.len()) };
        assert!(!ctx.is_null(), "valid font must produce a context");
        ctx
    }

    fn neutral_input() -> GuiInput {
        GuiInput {
            mouse_pos: [0.0; 2],
            mouse_down: [0; 5],
            mouse_wheel: 0.0,
            viewport_size: [800.0, 600.0],
            keys: std::ptr::null(),
            key_count: 0,
        }
    }

    #[test]
    fn null_font_ptr_returns_null_and_reports_invalid_input() {
        let ctx = unsafe { gui_context_new(std::ptr::null(), 0) };
        assert!(ctx.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
    }

    #[test]
    fn garbage_font_bytes_return_null_and_report_invalid_input() {
        let garbage = [0xABu8; 64];
        let ctx = unsafe { gui_context_new(garbage.as_ptr(), garbage.len()) };
        assert!(ctx.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
    }

    #[test]
    fn foreign_slice_bounds_are_checked_before_reading() {
        let oversized = isize::MAX as usize + 1;
        assert!(unsafe { slice_arg(DEJAVU_SANS.as_ptr(), oversized) }.is_none());
        let item = GuiStr {
            ptr: std::ptr::null(),
            len: 0,
        };
        let misaligned = (&item as *const GuiStr)
            .cast::<u8>()
            .wrapping_add(1)
            .cast::<GuiStr>();
        assert!(unsafe { slice_arg(misaligned, 1) }.is_none());
        assert!(unsafe { slice_arg::<u8>(std::ptr::null(), 0) }
            .unwrap()
            .is_empty());

        let ctx = unsafe { gui_context_new(DEJAVU_SANS.as_ptr(), oversized) };
        assert!(ctx.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
    }

    #[test]
    fn oversized_key_count_rejects_frame_without_mutating_context() {
        let ctx = new_context();
        let mut input = neutral_input();
        let event = GuiKeyEvent {
            kind: GUI_KEY_EVENT_PRESS,
            key: 0,
            ch: 0,
        };
        input.keys = &event;
        input.key_count = isize::MAX as usize / std::mem::size_of::<GuiKeyEvent>() + 1;
        unsafe { gui_new_frame(ctx, &input, 1.0 / 60.0) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);

        input.key_count = 1;
        unsafe { gui_new_frame(ctx, &input, 1.0 / 60.0) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_render(ctx) };
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn valid_font_produces_a_context_and_free_is_safe() {
        let ctx = new_context();
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
        unsafe { gui_context_free(std::ptr::null_mut()) };
    }

    #[test]
    fn atlas_accessors_match_core_values() {
        let reference = Context::new(DEJAVU_SANS).expect("reference context");
        let (ref_pixels, ref_w, ref_h) = reference.font().atlas_pixels();

        let ctx = new_context();
        let mut w = 0u32;
        let mut h = 0u32;
        let pixels = unsafe { gui_atlas_pixels(ctx, &mut w, &mut h) };
        assert!(!pixels.is_null());
        assert_eq!((w, h), (ref_w, ref_h));
        let ffi_pixels = unsafe { std::slice::from_raw_parts(pixels, (w * h) as usize) };
        assert_eq!(ffi_pixels, ref_pixels);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let generation = unsafe { gui_atlas_generation(ctx) };
        assert_eq!(generation, reference.font().atlas_generation());
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn atlas_null_arguments_report_invalid_input() {
        let ctx = new_context();
        let mut w = 0u32;
        let mut h = 0u32;
        assert!(unsafe { gui_atlas_pixels(ctx, &mut w, std::ptr::null_mut()) }.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        assert!(unsafe { gui_atlas_pixels(std::ptr::null_mut(), &mut w, &mut h) }.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        assert_eq!(unsafe { gui_atlas_generation(std::ptr::null_mut()) }, 0);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn render_after_new_frame_yields_geometry_and_stable_slices() {
        let ctx = new_context();
        let input = neutral_input();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe {
            (&mut *ctx)
                .ctx
                .ui()
                .window("ffi", restricted_gui::ScrollFlags::NONE, |ui| {
                    ui.label("hello ffi")
                })
        };

        let first = unsafe { gui_render(ctx) };
        assert!(first.vertex_count > 0);
        assert!(first.index_count > 0);
        assert!(first.command_count > 0);
        assert!(!first.vertices.is_null());
        assert!(!first.indices.is_null());
        assert!(!first.commands.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let second = unsafe { gui_render(ctx) };
        assert_eq!(first.vertices, second.vertices);
        assert_eq!(first.indices, second.indices);
        assert_eq!(first.commands, second.commands);
        assert_eq!(first.vertex_count, second.vertex_count);
        assert_eq!(first.index_count, second.index_count);
        assert_eq!(first.command_count, second.command_count);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn null_input_means_neutral_input() {
        let ctx = new_context();
        unsafe { gui_new_frame(ctx, std::ptr::null(), 0.016) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        let data = unsafe { gui_render(ctx) };
        assert_eq!(data.vertex_count, 0);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn null_context_calls_report_invalid_input() {
        let input = neutral_input();
        unsafe { gui_new_frame(std::ptr::null_mut(), &input, 0.016) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        let data = unsafe { gui_render(std::ptr::null_mut()) };
        assert!(data.vertices.is_null());
        assert_eq!(data.vertex_count, 0);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
    }

    #[test]
    fn panic_is_contained_reported_and_the_host_continues() {
        let ctx = new_context();
        let input = neutral_input();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);

        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        let _ = std::panic::take_hook();
        std::panic::set_hook(hook);

        assert_eq!(gui_last_error_code(), GuiErrorCode::Panic as u8);

        let data = unsafe { gui_render(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        let _ = data;
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        let data = unsafe { gui_render(ctx) };
        let _ = data;
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn last_error_code_is_always_callable_without_unwinding() {
        let code = std::thread::spawn(|| gui_last_error_code())
            .join()
            .expect("reader thread must not panic");
        assert_eq!(code, GuiErrorCode::None as u8);
        let ctx = unsafe { gui_context_new(std::ptr::null(), 0) };
        assert!(ctx.is_null());
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
    }

    #[test]
    fn every_key_round_trips_through_u16() {
        for (code, key) in ALL_KEYS.iter().enumerate() {
            let code = code as u16;
            assert_eq!(key_to_code(*key), code, "wrong code for {key:?}");
            assert_eq!(
                key_from_code(code),
                Some(*key),
                "code {code} does not map back"
            );
        }
        assert_eq!(key_to_code(Key::Down) as usize + 1, ALL_KEYS.len());
        assert_eq!(key_from_code(ALL_KEYS.len() as u16), None);
        assert_eq!(key_from_code(u16::MAX), None);
    }

    #[test]
    fn key_event_translation_accepts_valid_and_ignores_invalid() {
        let press = GuiKeyEvent {
            kind: GUI_KEY_EVENT_PRESS,
            key: 0,
            ch: 0,
        };
        assert_eq!(
            translate_key_event(&press),
            Some(KeyEvent::Press(Key::Backspace))
        );
        let release = GuiKeyEvent {
            kind: GUI_KEY_EVENT_RELEASE,
            key: 61,
            ch: 0,
        };
        assert_eq!(
            translate_key_event(&release),
            Some(KeyEvent::Release(Key::Alt))
        );
        let character = GuiKeyEvent {
            kind: GUI_KEY_EVENT_CHAR,
            key: 0,
            ch: 'x' as u32,
        };
        assert_eq!(translate_key_event(&character), Some(KeyEvent::Char('x')));
        let bad_key = GuiKeyEvent {
            kind: GUI_KEY_EVENT_PRESS,
            key: 64,
            ch: 0,
        };
        assert_eq!(translate_key_event(&bad_key), None);
        let bad_kind = GuiKeyEvent {
            kind: 3,
            key: 0,
            ch: 0,
        };
        assert_eq!(translate_key_event(&bad_kind), None);
        let bad_char = GuiKeyEvent {
            kind: GUI_KEY_EVENT_CHAR,
            key: 0,
            ch: u32::MAX,
        };
        assert_eq!(translate_key_event(&bad_char), None);
    }

    #[test]
    fn new_frame_accepts_a_full_key_stream() {
        let ctx = new_context();
        let mut events = Vec::new();
        for code in 0..64u16 {
            events.push(GuiKeyEvent {
                kind: GUI_KEY_EVENT_PRESS,
                key: code,
                ch: 0,
            });
            events.push(GuiKeyEvent {
                kind: GUI_KEY_EVENT_RELEASE,
                key: code,
                ch: 0,
            });
        }
        events.push(GuiKeyEvent {
            kind: GUI_KEY_EVENT_CHAR,
            key: 0,
            ch: 'A' as u32,
        });
        events.push(GuiKeyEvent {
            kind: 7,
            key: 999,
            ch: u32::MAX,
        });
        let mut input = neutral_input();
        input.keys = events.as_ptr();
        input.key_count = events.len();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        let data = unsafe { gui_render(ctx) };
        let _ = data;
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    fn add_second_face(ctx: *mut GuiContext) -> u32 {
        let id = unsafe { gui_add_font(ctx, DEJAVU_SANS.as_ptr(), DEJAVU_SANS.len(), 24.0) };
        assert_ne!(
            id, GUI_FONT_ID_INVALID,
            "valid face registration must succeed"
        );
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        id
    }

    #[test]
    fn add_font_registers_a_face_and_push_pop_scopes_text() {
        let ctx = new_context();
        let face = add_second_face(ctx);
        assert_eq!(face, 1, "the second face follows the default face 0");

        let input = neutral_input();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        unsafe { gui_push_font(ctx, face) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_label(ctx, b"big text".as_ptr(), 8) };
        unsafe { gui_pop_font(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_end_window(ctx) };
        let data = unsafe { gui_render(ctx) };
        assert!(data.vertex_count > 0);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn add_font_failures_return_invalid_id_and_report_invalid_input() {
        let ctx = new_context();
        let garbage = [0xABu8; 64];
        let id = unsafe { gui_add_font(ctx, garbage.as_ptr(), garbage.len(), 16.0) };
        assert_eq!(id, GUI_FONT_ID_INVALID);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);

        let id = unsafe { gui_add_font(ctx, std::ptr::null(), 8, 16.0) };
        assert_eq!(id, GUI_FONT_ID_INVALID);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);

        let id = unsafe { gui_add_font(ctx, DEJAVU_SANS.as_ptr(), 0, 16.0) };
        assert_eq!(id, GUI_FONT_ID_INVALID);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);

        let id = unsafe {
            gui_add_font(
                std::ptr::null_mut(),
                DEJAVU_SANS.as_ptr(),
                DEJAVU_SANS.len(),
                16.0,
            )
        };
        assert_eq!(id, GUI_FONT_ID_INVALID);
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);

        assert_eq!(add_second_face(ctx), 1);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn push_font_with_unknown_id_is_contract_violation_not_panic() {
        let ctx = new_context();
        let input = neutral_input();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        unsafe { gui_push_font(ctx, 1) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { gui_push_font(ctx, GUI_FONT_ID_INVALID) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        let data = unsafe { gui_render(ctx) };
        let _ = data;
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn pop_font_with_no_scope_open_is_contract_violation_not_panic() {
        let ctx = new_context();
        let input = neutral_input();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        unsafe { gui_pop_font(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        let data = unsafe { gui_render(ctx) };
        let _ = data;
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn font_scope_calls_without_a_frame_are_contract_violations() {
        let ctx = new_context();
        let face = add_second_face(ctx);
        unsafe { gui_push_font(ctx, face) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { gui_pop_font(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { gui_push_font(std::ptr::null_mut(), face) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        unsafe { gui_pop_font(std::ptr::null_mut()) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::InvalidInput as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn unbalanced_font_scope_is_violation_at_render_not_panic() {
        let ctx = new_context();
        let face = add_second_face(ctx);
        let input = neutral_input();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        unsafe { gui_push_font(ctx, face) };
        unsafe { gui_label(ctx, b"leaky".as_ptr(), 5) };
        unsafe { gui_end_window(ctx) };
        let data = unsafe { gui_render(ctx) };
        assert!(data.vertex_count > 0, "the frame still renders");
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);

        let input = neutral_input();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        let data = unsafe { gui_render(ctx) };
        let _ = data;
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn nested_font_scopes_unwind_in_order() {
        let ctx = new_context();
        let face = add_second_face(ctx);
        let input = neutral_input();
        unsafe { gui_new_frame(ctx, &input, 0.016) };
        assert!(unsafe { gui_begin_window(ctx, b"W".as_ptr(), 1, GUI_SCROLL_NONE) });
        unsafe { gui_push_font(ctx, face) };
        unsafe { gui_push_font(ctx, 0) };
        unsafe { gui_pop_font(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_pop_font(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_pop_font(ctx) };
        assert_eq!(gui_last_error_code(), GuiErrorCode::ContractViolation as u8);
        unsafe { gui_end_window(ctx) };
        let data = unsafe { gui_render(ctx) };
        assert!(data.vertex_count > 0);
        assert_eq!(gui_last_error_code(), GuiErrorCode::None as u8);
        unsafe { gui_context_free(ctx) };
    }

    #[test]
    fn ffi_struct_layouts_match_core_layouts() {
        use std::mem::{align_of, offset_of, size_of};
        assert_eq!(size_of::<GuiVert>(), size_of::<DrawVert>());
        assert_eq!(align_of::<GuiVert>(), align_of::<DrawVert>());
        assert_eq!(offset_of!(GuiVert, pos), offset_of!(DrawVert, pos));
        assert_eq!(offset_of!(GuiVert, uv), offset_of!(DrawVert, uv));
        assert_eq!(offset_of!(GuiVert, col), offset_of!(DrawVert, col));
        assert_eq!(size_of::<DrawVert>(), 20);
        assert_eq!(align_of::<DrawVert>(), 4);
        assert_eq!(offset_of!(DrawVert, pos), 0);
        assert_eq!(offset_of!(DrawVert, uv), 8);
        assert_eq!(offset_of!(DrawVert, col), 16);
        assert_eq!(size_of::<TextureId>(), size_of::<u64>());
        assert_eq!(align_of::<TextureId>(), align_of::<u64>());
    }
}
