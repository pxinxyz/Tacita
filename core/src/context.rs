use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::draw::{DrawCmd, DrawData, DrawList, DrawVert};
use crate::font::{Font, FontError, IconFont};
use crate::id::{Id, IdStack};
use crate::input::{InputState, Key, KeyEvent, Modifiers};
use crate::layout::Cursor;
use crate::style::Style;
use crate::ui::point_in_rect;

pub const DEFAULT_FONT_SIZE: f32 = 16.0;

const DEFAULT_WINDOW_POS: [f32; 2] = [60.0, 60.0];

const CASCADE_OFFSET: f32 = 24.0;

const DEFAULT_WINDOW_SIZE: [f32; 2] = [300.0, 400.0];

pub(crate) const MIN_WINDOW_SIZE: [f32; 2] = [120.0, 80.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PressTarget {
    None,
    Popup,
    Window(Id),
}

pub(crate) enum WidgetState {
    TextInput(TextInputState),
    Scroll {
        offset: f32,
        content_height: f32,
    },
    TabBar {
        active: usize,
    },
    CollapsingHeader {
        open: bool,
    },
    Keybind {
        armed: bool,
    },
    Tooltip {
        hover_time: f32,
    },
    ColorPicker {
        h: f32,
        s: f32,
        v: f32,
        hex: String,
        original: [u8; 4],
    },
}

pub(crate) struct TextInputState {
    pub cursor: usize,
    pub scroll_x: f32,
    pub blink: f32,
}

#[derive(Default)]
pub(crate) struct FrameInput {
    pub(crate) mouse_pos: [f32; 2],
    pub(crate) mouse_down: [bool; 3],
    pub(crate) mouse_wheel: f32,
    pub(crate) keys: Vec<KeyEvent>,
    pub(crate) viewport_size: [f32; 2],
}

pub(crate) struct Window {
    pub(crate) id: Id,
    pub(crate) title: String,
    pub(crate) pos: [f32; 2],
    pub(crate) size: [f32; 2],
    pub(crate) open: bool,
    pub(crate) draw_list: DrawList,
    pub(crate) cursor: Cursor,
}

impl Window {
    fn new(title: &str, cascade_index: usize) -> Self {
        let offset = cascade_index as f32 * CASCADE_OFFSET;
        let pos = [
            DEFAULT_WINDOW_POS[0] + offset,
            DEFAULT_WINDOW_POS[1] + offset,
        ];
        let id = IdStack::new().id(title);
        let size = DEFAULT_WINDOW_SIZE;
        let mut draw_list = DrawList::new();
        draw_list.push_clip_rect([pos[0], pos[1], pos[0] + size[0], pos[1] + size[1]]);
        Self {
            id,
            title: String::from(title),
            pos,
            size,
            open: true,
            draw_list,
            cursor: Cursor::new(pos, size[0], [0.0; 2], [0.0; 2]),
        }
    }

    pub(crate) fn rect(&self) -> [f32; 4] {
        [
            self.pos[0],
            self.pos[1],
            self.pos[0] + self.size[0],
            self.pos[1] + self.size[1],
        ]
    }
}

pub(crate) struct ScrollRegionEntry {
    pub(crate) id: Id,
    pub(crate) rect: [f32; 4],
    pub(crate) layer_owns_point: bool,
    pub(crate) max_offset: f32,
}

pub(crate) struct TabBarScopeState {
    pub(crate) id: Id,
    pub(crate) row: [f32; 4],
    pub(crate) pen_x: f32,
    pub(crate) active: usize,
    pub(crate) clicked: Option<usize>,
    pub(crate) count: usize,
}

pub struct Context {
    pub(crate) font: Font,
    pub style: Style,
    pub(crate) id_stack: IdStack,
    pub(crate) input: FrameInput,
    // Retaining state in a stateless framework. I have become what I swore to destroy.
    pending_input: FrameInput,
    pub(crate) prev_mouse_down: [bool; 3],
    pub(crate) modifiers: Modifiers,
    pub(crate) hot: Option<Id>,
    pub(crate) active: Option<Id>,
    pub(crate) active_drag_offset: [f32; 2],
    pub(crate) press_target: PressTarget,
    pub(crate) hover_top_window: Option<Id>,
    pub(crate) widget_states: BTreeMap<Id, WidgetState>,
    pub(crate) windows: Vec<Window>,
    pub(crate) overlay: DrawList,
    pub(crate) open_popup: Option<Id>,
    pub(crate) popup_rect: Option<[f32; 4]>,
    pub(crate) overlay_cursor: Cursor,
    pub(crate) popup_scope_active: bool,
    pub(crate) interaction_clip: Option<[f32; 4]>,
    pub(crate) scroll_regions: Vec<ScrollRegionEntry>,
    pub(crate) tab_bar_scopes: Vec<TabBarScopeState>,
    pub(crate) last_item: Option<(Id, [f32; 4])>,
    pub(crate) current_window: Option<Id>,
    pub(crate) window_scope_saved_prev_left: bool,
    pub(crate) dt: f32,
    created_windows: usize,
    frame_active: bool,
    draw_vertices: Vec<DrawVert>,
    draw_indices: Vec<u32>,
    draw_commands: Vec<DrawCmd>,
}

impl Context {
    pub fn new(font_bytes: &[u8]) -> Result<Context, FontError> {
        Ok(Self::with_font(Font::new(font_bytes, DEFAULT_FONT_SIZE)?))
    }

    pub fn new_with_icons(font_bytes: &[u8], icons: &[IconFont<'_>]) -> Result<Context, FontError> {
        Ok(Self::with_font(Font::new_with_icons(
            font_bytes,
            DEFAULT_FONT_SIZE,
            icons,
        )?))
    }

    pub fn with_font(font: Font) -> Context {
        Context {
            font,
            style: Style::default(),
            id_stack: IdStack::new(),
            input: FrameInput::default(),
            pending_input: FrameInput::default(),
            prev_mouse_down: [false; 3],
            modifiers: Modifiers::default(),
            hot: None,
            active: None,
            active_drag_offset: [0.0; 2],
            press_target: PressTarget::None,
            hover_top_window: None,
            widget_states: BTreeMap::new(),
            windows: Vec::new(),
            overlay: DrawList::new(),
            open_popup: None,
            popup_rect: None,
            overlay_cursor: Cursor::new([0.0; 2], 0.0, [0.0; 2], [0.0; 2]),
            popup_scope_active: false,
            last_item: None,
            interaction_clip: None,
            scroll_regions: Vec::new(),
            tab_bar_scopes: Vec::new(),
            current_window: None,
            window_scope_saved_prev_left: false,
            dt: 0.0,
            created_windows: 0,
            frame_active: false,
            draw_vertices: Vec::new(),
            draw_indices: Vec::new(),
            draw_commands: Vec::new(),
        }
    }

    pub fn font(&self) -> &Font {
        &self.font
    }

    pub fn set_input(&mut self, input: InputState) {
        // Copy the host's frame into our own buffers. Borrowing across frames is how you meet the borrow checker's final boss.
        self.pending_input.mouse_pos = input.mouse_pos;
        self.pending_input.mouse_down = input.mouse_down;
        self.pending_input.mouse_wheel = input.mouse_wheel;
        self.pending_input.keys.clear();
        self.pending_input.keys.extend_from_slice(input.keys);
        self.pending_input.viewport_size = input.viewport_size;
    }

    pub fn new_frame(&mut self, dt: f32) {
        assert!(
            !self.frame_active,
            "Context::new_frame called while a frame is active; call render first"
        );
        self.dt = dt;
        self.prev_mouse_down = self.input.mouse_down;
        core::mem::swap(&mut self.input, &mut self.pending_input);
        self.pending_input.keys.clear();
        self.pending_input.mouse_pos = [0.0; 2];
        self.pending_input.mouse_down = [false; 3];
        self.pending_input.mouse_wheel = 0.0;
        self.pending_input.viewport_size = [0.0; 2];
        for event in &self.input.keys {
            match event {
                KeyEvent::Press(Key::Shift) => self.modifiers.shift = true,
                KeyEvent::Release(Key::Shift) => self.modifiers.shift = false,
                KeyEvent::Press(Key::Ctrl) => self.modifiers.ctrl = true,
                KeyEvent::Release(Key::Ctrl) => self.modifiers.ctrl = false,
                KeyEvent::Press(Key::Alt) => self.modifiers.alt = true,
                KeyEvent::Release(Key::Alt) => self.modifiers.alt = false,
                _ => {}
            }
        }
        self.hot = None;
        self.last_item = None;
        self.scroll_regions.clear();
        if self.open_popup.is_some() {
            let escape = self
                .input
                .keys
                .iter()
                .any(|ev| matches!(ev, KeyEvent::Press(Key::Escape)));
            let outside_press = self.popup_rect.is_some_and(|rect| {
                self.input.mouse_down[0]
                    && !self.prev_mouse_down[0]
                    && !point_in_rect(self.input.mouse_pos, rect)
            });
            if escape || outside_press {
                self.open_popup = None;
                self.popup_rect = None;
                if outside_press {
                    self.prev_mouse_down[0] = true;
                }
            }
        }
        self.press_target = PressTarget::None;
        if self.input.mouse_down[0] && !self.prev_mouse_down[0] {
            let inside_popup = self.open_popup.is_some()
                && self
                    .popup_rect
                    .is_some_and(|rect| point_in_rect(self.input.mouse_pos, rect));
            self.press_target = if inside_popup {
                PressTarget::Popup
            } else {
                self.front_window_at(self.input.mouse_pos)
                    .map_or(PressTarget::None, PressTarget::Window)
            };
        }
        // Z-order hit-testing: the front-most window at the cursor wins. Implemented in M1, fixed in H1, never questioned again.
        self.hover_top_window = self.front_window_at(self.input.mouse_pos);
        self.overlay.clear();
        if self.input.viewport_size != [0.0, 0.0] {
            self.overlay.push_clip_rect([
                0.0,
                0.0,
                self.input.viewport_size[0],
                self.input.viewport_size[1],
            ]);
        }
        for window in &mut self.windows {
            window.draw_list.clear();
            window.draw_list.push_clip_rect(window.rect());
        }
        self.frame_active = true;
    }

    pub fn ui(&mut self) -> crate::ui::Ui<'_> {
        assert!(
            self.frame_active,
            "Context::ui called outside a frame; call new_frame first"
        );
        crate::ui::Ui { ctx: self }
    }

    // One frame, one render: DrawData borrows this context, so a second render() panics. The FFI wrapper dances around that with a staging buffer and a dream.
    pub fn render(&mut self) -> DrawData<'_> {
        assert!(
            self.frame_active,
            "Context::render called without a preceding new_frame"
        );
        self.frame_active = false;
        self.route_wheel();
        self.draw_vertices.clear();
        self.draw_indices.clear();
        self.draw_commands.clear();
        for window in &self.windows {
            if !window.open {
                continue;
            }
            let data = window.draw_list.as_draw_data();
            let vertex_base = self.draw_vertices.len() as u32;
            self.draw_vertices.extend_from_slice(data.vertices);
            self.draw_indices
                .extend(data.indices.iter().map(|i| i + vertex_base));
            self.draw_commands.extend_from_slice(data.commands);
        }
        let data = self.overlay.as_draw_data();
        let vertex_base = self.draw_vertices.len() as u32;
        self.draw_vertices.extend_from_slice(data.vertices);
        self.draw_indices
            .extend(data.indices.iter().map(|i| i + vertex_base));
        self.draw_commands.extend_from_slice(data.commands);
        DrawData {
            vertices: &self.draw_vertices,
            indices: &self.draw_indices,
            commands: &self.draw_commands,
        }
    }

    fn route_wheel(&mut self) {
        if self.input.mouse_wheel == 0.0 {
            return;
        }
        let mouse = self.input.mouse_pos;
        let winner = self
            .scroll_regions
            .iter()
            .rev()
            .find(|entry| entry.layer_owns_point && point_in_rect(mouse, entry.rect))
            .map(|entry| (entry.id, entry.max_offset));
        let Some((id, max_offset)) = winner else {
            return;
        };
        let delta =
            self.input.mouse_wheel * crate::ui::WHEEL_LINES_PER_NOTCH * self.font.line_height();
        if let Some(WidgetState::Scroll { offset, .. }) = self.widget_states.get_mut(&id) {
            *offset = (*offset - delta).clamp(0.0, max_offset);
        }
    }

    pub(crate) fn find_or_create_window(&mut self, title: &str) -> usize {
        if let Some(index) = self.windows.iter().position(|w| w.title == title) {
            return index;
        }
        let index = self.windows.len();
        self.windows.push(Window::new(title, self.created_windows));
        self.created_windows += 1;
        index
    }

    pub(crate) fn bring_window_to_front(&mut self, index: usize) {
        let window = self.windows.remove(index);
        self.windows.push(window);
    }

    pub(crate) fn front_window_at(&self, point: [f32; 2]) -> Option<Id> {
        self.windows
            .iter()
            .rev()
            .find(|w| w.open && point_in_rect(point, w.rect()))
            .map(|w| w.id)
    }

    pub(crate) fn window_index(&self, id: Id) -> usize {
        self.windows
            .iter()
            .position(|w| w.id == id)
            .expect("window ID must resolve to a registered window")
    }

    pub(crate) fn current_window_mut(&mut self) -> &mut Window {
        let id = self
            .current_window
            .expect("current_window_mut called outside a window declaration");
        let index = self.window_index(id);
        &mut self.windows[index]
    }

    pub(crate) fn set_active(&mut self, id: Id) {
        self.active = Some(id);
    }

    pub(crate) fn clear_active(&mut self) {
        self.active = None;
    }

    pub fn set_window_open(&mut self, title: &str, open: bool) {
        let Some(index) = self.windows.iter().position(|w| w.title == title) else {
            return;
        };
        self.windows[index].open = open;
        if !open {
            let mut stack = IdStack::new();
            stack.push(title);
            let chrome_id = stack.id(crate::ui::TITLE_BAR_TAG);
            let close_id = stack.id(crate::ui::CLOSE_BUTTON_TAG);
            let resize_id = stack.id(crate::ui::RESIZE_GRIP_TAG);
            if self.active == Some(chrome_id)
                || self.active == Some(close_id)
                || self.active == Some(resize_id)
            {
                self.clear_active();
            }
        }
    }

    pub fn wants_mouse_capture(&self) -> bool {
        if self.active.is_some() || self.hover_top_window.is_some() {
            return true;
        }
        self.open_popup.is_some()
            && self
                .popup_rect
                .is_some_and(|rect| point_in_rect(self.input.mouse_pos, rect))
    }

    pub fn wants_keyboard_capture(&self) -> bool {
        self.active.is_some()
            || self
                .widget_states
                .values()
                .any(|state| matches!(state, WidgetState::Keybind { armed: true }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Key;

    const DEJAVU_SANS: &[u8] = include_bytes!("../tests/assets/DejaVuSans.ttf");

    fn test_context() -> Context {
        Context::new(DEJAVU_SANS).expect("DejaVu Sans must load")
    }

    #[test]
    fn new_with_test_font_succeeds() {
        let ctx = test_context();
        let (pixels, w, h) = ctx.font().atlas_pixels();
        assert_eq!(pixels.len(), (w * h) as usize);
        assert!(w > 0 && h > 0);
    }

    #[test]
    fn new_with_garbage_bytes_returns_err() {
        assert!(Context::new(b"this is not a font").is_err());
        assert!(Context::new(&[]).is_err());
    }

    #[test]
    #[should_panic]
    fn render_without_new_frame_panics() {
        let mut ctx = test_context();
        let _ = ctx.render();
    }

    #[test]
    #[should_panic]
    fn new_frame_without_render_panics() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        ctx.new_frame(0.016);
    }

    #[test]
    fn new_frame_transplants_input_and_keeps_previous_mouse_down() {
        let mut ctx = test_context();

        ctx.set_input(InputState {
            mouse_pos: [10.0, 20.0],
            mouse_down: [true, false, false],
            mouse_wheel: 1.0,
            keys: &[KeyEvent::Char('a'), KeyEvent::Press(Key::Enter)],
            viewport_size: [800.0, 600.0],
        });
        ctx.new_frame(0.016);
        assert_eq!(ctx.input.mouse_pos, [10.0, 20.0]);
        assert_eq!(ctx.input.mouse_down, [true, false, false]);
        assert_eq!(ctx.input.mouse_wheel, 1.0);
        assert_eq!(ctx.input.viewport_size, [800.0, 600.0]);
        assert_eq!(
            ctx.input.keys,
            alloc::vec![KeyEvent::Char('a'), KeyEvent::Press(Key::Enter)]
        );
        assert_eq!(ctx.prev_mouse_down, [false; 3]);
        ctx.render();

        ctx.set_input(InputState {
            mouse_pos: [11.0, 21.0],
            mouse_down: [false, false, false],
            mouse_wheel: 0.0,
            keys: &[],
            viewport_size: [800.0, 600.0],
        });
        ctx.new_frame(0.016);
        assert_eq!(ctx.input.mouse_down, [false; 3]);
        assert_eq!(ctx.prev_mouse_down, [true, false, false]);
        assert!(ctx.input.keys.is_empty());
        ctx.render();
    }

    #[test]
    fn modifier_state_folds_the_shift_ctrl_alt_press_release_history() {
        let mut ctx = test_context();
        assert_eq!(ctx.modifiers, Modifiers::default());

        ctx.set_input(InputState {
            keys: &[KeyEvent::Press(Key::Shift)],
            ..Default::default()
        });
        ctx.new_frame(0.016);
        assert!(ctx.modifiers.shift);
        assert!(!ctx.modifiers.ctrl);
        ctx.render();

        ctx.set_input(InputState {
            keys: &[KeyEvent::Press(Key::Ctrl)],
            ..Default::default()
        });
        ctx.new_frame(0.016);
        assert!(ctx.modifiers.shift);
        assert!(ctx.modifiers.ctrl);
        ctx.render();

        ctx.set_input(InputState {
            keys: &[KeyEvent::Release(Key::Shift), KeyEvent::Press(Key::Alt)],
            ..Default::default()
        });
        ctx.new_frame(0.016);
        assert!(!ctx.modifiers.shift);
        assert!(ctx.modifiers.ctrl);
        assert!(ctx.modifiers.alt);
        ctx.render();

        ctx.set_input(InputState {
            keys: &[KeyEvent::Press(Key::G), KeyEvent::Char('g')],
            ..Default::default()
        });
        ctx.new_frame(0.016);
        assert!(ctx.modifiers.ctrl);
        assert!(ctx.modifiers.alt);
        ctx.render();
    }

    #[test]
    fn new_frame_resets_window_draw_lists_with_window_rect_root_clip() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.window("Win", |_| {});
        let _ = ui;
        ctx.render();

        ctx.new_frame(0.016);
        let window = &ctx.windows[0];
        assert_eq!(window.draw_list.as_draw_data().indices.len(), 0);
        assert_eq!(window.draw_list.current_clip(), window.rect());
        ctx.render();
    }

    #[test]
    fn first_windows_cascade_from_default_position() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.window("A", |_| {});
        ui.window("B", |_| {});
        let _ = ui;

        assert_eq!(ctx.windows[0].pos, DEFAULT_WINDOW_POS);
        assert_eq!(ctx.windows[0].size, DEFAULT_WINDOW_SIZE);
        assert_eq!(
            ctx.windows[1].pos,
            [
                DEFAULT_WINDOW_POS[0] + CASCADE_OFFSET,
                DEFAULT_WINDOW_POS[1] + CASCADE_OFFSET,
            ]
        );
        let mut ui = ctx.ui();
        ui.window("A", |_| {});
        let _ = ui;
        assert_eq!(ctx.windows.len(), 2);
        ctx.render();
    }

    fn test_context_at(px_size: f32) -> Context {
        let font = Font::new(DEJAVU_SANS, px_size).expect("DejaVu Sans must load");
        Context::with_font(font)
    }

    fn content_start_y(ctx: &mut Context) -> f32 {
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.window("Win", |_| {});
        let _ = ui;
        let y = ctx.windows[0].cursor.pos_y();
        ctx.render();
        y
    }

    #[test]
    fn with_font_at_custom_size_completes_frame_cycle_and_emits_geometry() {
        let mut ctx = test_context_at(24.0);
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.window("Win", |ui| {
            ui.label("hello");
        });
        let _ = ui;
        let data = ctx.render();
        assert!(!data.vertices.is_empty());
        assert!(!data.indices.is_empty());
        assert!(!data.commands.is_empty());
    }

    #[test]
    fn title_bar_height_derives_from_the_font_line_height() {
        let mut ctx_default = test_context();
        let mut ctx_large = test_context_at(24.0);
        let y_default = content_start_y(&mut ctx_default);
        let y_large = content_start_y(&mut ctx_large);

        let style = ctx_default.style;
        let pad = style.window_padding[1];
        let tb_default = y_default - DEFAULT_WINDOW_POS[1] - pad;
        let tb_large = y_large - DEFAULT_WINDOW_POS[1] - pad;
        assert_eq!(
            tb_default,
            ctx_default.font().line_height() + 2.0 * style.frame_padding[1]
        );
        assert_eq!(
            tb_large,
            ctx_large.font().line_height() + 2.0 * style.frame_padding[1]
        );
        assert!(tb_large > tb_default);
    }

    #[test]
    fn title_text_stays_within_the_title_bar_at_custom_size() {
        let mut ctx = test_context_at(24.0);
        let title_bar_bottom =
            DEFAULT_WINDOW_POS[1] + ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1];
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.window("Title Text", |_| {});
        let _ = ui;
        let data = ctx.render();

        let mut title_text_verts = 0;
        let mut index_offset = 0usize;
        for cmd in data.commands {
            if cmd.texture == crate::draw::TextureId::FONT {
                let end = index_offset + cmd.elem_count as usize;
                for &i in &data.indices[index_offset..end] {
                    let v = data.vertices[i as usize];
                    if v.pos[1] < title_bar_bottom {
                        title_text_verts += 1;
                        assert!(
                            v.pos[1] >= DEFAULT_WINDOW_POS[1],
                            "title text above the title bar: y = {}",
                            v.pos[1]
                        );
                    }
                }
                index_offset = end;
            } else {
                index_offset += cmd.elem_count as usize;
            }
        }
        assert!(title_text_verts > 0, "title text was emitted");
    }

    #[test]
    fn label_advance_matches_the_font_line_height_at_custom_size() {
        let mut ctx = test_context_at(24.0);
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.window("Win", |ui| {
            ui.label("hello");
        });
        let _ = ui;
        let after_label_y = ctx.windows[0].cursor.pos_y();
        ctx.render();

        let style = ctx.style;
        let line_height = ctx.font().line_height();
        let title_bar_height = line_height + 2.0 * style.frame_padding[1];
        let content_start = DEFAULT_WINDOW_POS[1] + title_bar_height + style.window_padding[1];
        assert_eq!(
            after_label_y,
            content_start + line_height + style.item_spacing[1]
        );
    }

    #[test]
    fn context_new_keeps_the_default_font_size() {
        let ctx = test_context();
        let reference = Font::new(DEJAVU_SANS, DEFAULT_FONT_SIZE).expect("DejaVu Sans must load");
        assert_eq!(ctx.font().line_height(), reference.line_height());
    }

    const FA_SOLID: &[u8] = include_bytes!("../tests/assets/fa-solid-900.ttf");

    fn icon_context() -> Context {
        let star = crate::icon::STAR as u32;
        Context::new_with_icons(
            DEJAVU_SANS,
            &[IconFont {
                bytes: FA_SOLID,
                range: (star, star),
            }],
        )
        .expect("icon bake must succeed")
    }

    #[test]
    fn new_with_icons_builds_and_runs_a_full_frame_cycle() {
        let mut ctx = icon_context();
        for _ in 0..2 {
            crate::widgets::test_support::frame(&mut ctx, [0.0, 0.0], false, |ui| {
                ui.label("Aim \u{f005}");
            });
        }
    }

    #[test]
    fn new_with_icons_rejects_invalid_icon_bytes() {
        let icons = [IconFont {
            bytes: b"this is not a font",
            range: (0xF005, 0xF005),
        }];
        assert!(matches!(
            Context::new_with_icons(DEJAVU_SANS, &icons),
            Err(FontError::InvalidFont(_))
        ));
    }

    #[test]
    fn new_with_icons_keeps_the_default_font_size() {
        let ctx = icon_context();
        let reference = Font::new(DEJAVU_SANS, DEFAULT_FONT_SIZE).expect("DejaVu Sans must load");
        assert_eq!(ctx.font().line_height(), reference.line_height());
    }

    #[test]
    fn label_mixing_text_and_icon_measures_wider_and_emits_both() {
        use crate::widgets::test_support::{frame, glyph_colors_in};
        let mut ctx = icon_context();
        let text = alloc::format!("Aim{}", crate::icon::STAR);
        let mut text_only = [0.0; 2];
        let mut mixed = [0.0; 2];
        let mut label_rect = [0.0; 4];
        frame(&mut ctx, [0.0, 0.0], false, |ui| {
            text_only = ui.ctx.font.measure("Aim");
            mixed = ui.ctx.font.measure(&text);
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            ui.label(&text);
            label_rect = [pos[0], pos[1], pos[0] + mixed[0], pos[1] + mixed[1]];
        });
        assert!(mixed[0] > text_only[0], "the icon adds advance width");
        assert_eq!(mixed[1], text_only[1], "the icon shares the line height");
        let text_rect = [
            label_rect[0],
            label_rect[1],
            label_rect[0] + text_only[0],
            label_rect[3],
        ];
        let icon_rect = [text_rect[2], label_rect[1], label_rect[2], label_rect[3]];
        assert!(
            !glyph_colors_in(&ctx, text_rect).is_empty(),
            "text glyphs emitted"
        );
        assert!(
            !glyph_colors_in(&ctx, icon_rect).is_empty(),
            "icon glyph emitted"
        );
    }

    #[test]
    fn icon_glyph_recolors_with_vertex_color_like_any_glyph() {
        use crate::widgets::test_support::{frame, glyph_colors_in};
        let mut ctx = icon_context();
        let accent = [250, 180, 20, 255];
        let text = alloc::format!("{}", crate::icon::STAR);
        let mut rect = [0.0; 4];
        frame(&mut ctx, [0.0, 0.0], false, |ui| {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let size = ui.ctx.font.measure(&text);
            ui.draw_text_at(pos, &text, accent);
            rect = [pos[0], pos[1], pos[0] + size[0], pos[1] + size[1]];
        });
        let colors = glyph_colors_in(&ctx, rect);
        assert!(!colors.is_empty(), "icon glyph quads emitted");
        assert!(
            colors.iter().all(|&c| c == accent),
            "every icon vertex takes the draw call's color"
        );
    }
}
