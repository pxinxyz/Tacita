use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::draw::{DrawCmd, DrawData, DrawList, DrawVert};
use crate::font::{Font, FontError, FontId, IconFont};
use crate::id::{Id, IdStack};
use crate::input::{InputState, Key, KeyEvent, Modifiers};
use crate::layout::Cursor;
use crate::scroll::{ScrollContainerEntry, ScrollTarget};
use crate::style::Style;
use crate::ui::point_in_rect;

pub(crate) struct Toast {
    pub text: String,
    pub remaining: f32,
    pub duration: f32,
}

fn toast_ease(value: f32) -> f32 {
    crate::animation::Easing::SmoothStep.sample(value)
}

fn toast_animation(remaining: f32, duration: f32) -> (f32, f32) {
    let enter_time = 0.22_f32.min(duration * 0.4).max(f32::EPSILON);
    let exit_time = 0.28_f32.min(duration * 0.4).max(f32::EPSILON);
    let enter = toast_ease((duration - remaining) / enter_time);
    let exit = toast_ease(remaining / exit_time);
    let opacity = enter * exit;
    let slide_x = (1.0 - enter) * 24.0 + (1.0 - exit) * 12.0;
    (slide_x, opacity)
}

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
    MenuBar {
        open: Option<Id>,
    },
    TextInput(TextInputState),
    TextEdit,
    DragValue {
        start_x: f32,
        start_value: f64,
    },
    Scroll {
        offset: f32,
        content_height: f32,
    },
    TabBar {
        active: usize,
    },
    SearchCombo {
        highlighted: usize,
    },
    CollapsingHeader {
        open: bool,
    },
    TreeNode {
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
    pub editor: Option<alloc::boxed::Box<crate::editing::TextEditor>>,
    pub text_hash: u64,
    pub cursor: usize,
    pub scroll_x: f32,
    pub blink: f32,
}

#[derive(Default)]
pub(crate) struct FrameInput {
    pub(crate) mouse_pos: [f32; 2],
    pub(crate) mouse_down: [bool; 5],
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
    pub(crate) scroll_offset: f32,
    pub(crate) content_height: f32,
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
            scroll_offset: 0.0,
            content_height: 0.0,
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

pub struct WindowInspection<'a> {
    pub title: &'a str,
    pub rect: [f32; 4],
    pub vertices: usize,
    pub indices: usize,
    pub commands: &'a [DrawCmd],
}

#[derive(Clone, Copy)]
pub(crate) struct WindowScopeState {
    pub(crate) flags: crate::scroll::ScrollFlags,
    pub(crate) body_rect: [f32; 4],
    pub(crate) content_origin_y: f32,
    pub(crate) item_spacing_y: f32,
    pub(crate) scrollbar_rect: Option<[f32; 4]>,
    pub(crate) saved_interaction_clip: Option<[f32; 4]>,
    pub(crate) saved_prev_left: bool,
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
    pub(crate) accessibility_actions: crate::accessibility::ActionQueue,
    pub(crate) host_requests: crate::host_requests::HostRequests,
    pub(crate) duplicate_ids: crate::diagnostics::duplicate_ids::DuplicateIds,
    pub(crate) theme: Option<crate::theme::ThemeTokens>,
    pub(crate) motion: crate::animation::MotionSettings,
    pub(crate) animations: crate::animation::AnimationCache,
    pub(crate) motion_frame: u64,
    pub(crate) active_in_popup: bool,
    pub(crate) collapse_heights: BTreeMap<Id, f32>,
    pub(crate) focus_visuals: Vec<([f32; 4], [f32; 4], [u8; 4])>,
    pub(crate) focus_mask_visuals: Vec<(usize, crate::effects::MaskStack)>,
    pub(crate) focused_masks: crate::effects::MaskStack,
    pub(crate) popup_animation: crate::animation::PopupAnimation,
    pub(crate) routing: crate::interaction::Routing,
    pub(crate) font: Font,
    pub(crate) font_stack: Vec<FontId>,
    pub style: Style,
    pub(crate) id_stack: IdStack,
    pub(crate) input: FrameInput,
    pending_input: FrameInput,
    pub(crate) text_host: crate::editing::TextHost,
    pub(crate) prev_mouse_down: [bool; 5],
    pub(crate) modifiers: Modifiers,
    pub(crate) hot: Option<Id>,
    pub(crate) focused: Option<Id>,
    pub(crate) focused_in_modal: bool,
    pub(crate) focused_in_popup: bool,
    pub(crate) focus_order: Vec<Id>,
    pub(crate) focus_order_prev: Vec<Id>,
    pub(crate) focus_rect: Option<[f32; 4]>,
    pub(crate) focus_clip: Option<[f32; 4]>,
    pub(crate) keyboard_activate: Option<Id>,
    pub(crate) drag_state: Option<crate::drag_drop::DragState>,
    pub(crate) active: Option<Id>,
    pub(crate) active_list_index: Option<usize>,
    pub(crate) disabled_groups: Vec<Id>,
    pub(crate) active_groups: Vec<Id>,
    pub(crate) disabled_depth: usize,
    pub(crate) active_drag_offset: [f32; 2],
    pub(crate) scrollbar_skin: crate::scroll::ScrollbarSkin,
    pub(crate) press_target: PressTarget,
    pub(crate) hover_top_window: Option<Id>,
    pub(crate) widget_states: BTreeMap<Id, WidgetState>,
    pub(crate) windows: Vec<Window>,
    pub(crate) overlay: DrawList,
    pub(crate) open_popup: Option<Id>,
    pub(crate) popup_modal: bool,
    pub(crate) popup_anchor: [f32; 2],
    pub(crate) popup_rect: Option<[f32; 4]>,
    pub(crate) overlay_cursor: Cursor,
    pub(crate) popup_scope_active: bool,
    pub(crate) toasts: Vec<Toast>,
    pub(crate) interaction_clip: Option<[f32; 4]>,
    pub(crate) rounded_clips: Vec<crate::shapes::ClipEntry>,
    pub(crate) mask_depth: usize,
    pub(crate) stencil_depth: usize,
    pub(crate) rounded_capture_path: Vec<Id>,
    pub(crate) focus_rounding: Vec<(usize, crate::shapes::RoundedRect)>,
    pub(crate) focused_rounding: Vec<crate::shapes::RoundedRect>,
    pub(crate) scroll_containers: Vec<ScrollContainerEntry>,
    pub(crate) tab_bar_scopes: Vec<TabBarScopeState>,
    pub(crate) last_item: Option<(Id, [f32; 4])>,
    pub(crate) current_window: Option<Id>,
    pub(crate) window_scope: Option<WindowScopeState>,
    pub(crate) dt: f32,
    created_windows: usize,
    pub(crate) frame_active: bool,
    draw_vertices: Vec<DrawVert>,
    draw_indices: Vec<u32>,
    draw_commands: Vec<DrawCmd>,
    draw_effects: Vec<crate::effects::DrawEffects>,
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
            theme: None,
            accessibility_actions: Default::default(),
            host_requests: Default::default(),
            duplicate_ids: Default::default(),
            motion: crate::animation::MotionSettings::default(),
            animations: BTreeMap::new(),
            motion_frame: 0,
            active_in_popup: false,
            collapse_heights: BTreeMap::new(),
            focus_visuals: Vec::new(),
            focus_mask_visuals: Vec::new(),
            focused_masks: Default::default(),
            popup_animation: crate::animation::PopupAnimation::new(),
            routing: crate::interaction::Routing::new(),
            font,
            font_stack: Vec::with_capacity(8),
            style: Style::default(),
            id_stack: IdStack::new(),
            input: FrameInput::default(),
            pending_input: FrameInput::default(),
            text_host: crate::editing::TextHost::default(),
            prev_mouse_down: [false; 5],
            modifiers: Modifiers::default(),
            hot: None,
            focused: None,
            focused_in_modal: false,
            focused_in_popup: false,
            focus_order: Vec::new(),
            focus_order_prev: Vec::new(),
            focus_rect: None,
            focus_clip: None,
            keyboard_activate: None,
            drag_state: None,
            active: None,
            active_list_index: None,
            disabled_groups: Vec::with_capacity(8),
            active_groups: Vec::with_capacity(8),
            disabled_depth: 0,
            active_drag_offset: [0.0; 2],
            scrollbar_skin: crate::scroll::ScrollbarSkin::default(),
            press_target: PressTarget::None,
            hover_top_window: None,
            widget_states: BTreeMap::new(),
            windows: Vec::new(),
            overlay: DrawList::new(),
            open_popup: None,
            popup_modal: false,
            popup_anchor: [0.0; 2],
            popup_rect: None,
            overlay_cursor: Cursor::new([0.0; 2], 0.0, [0.0; 2], [0.0; 2]),
            popup_scope_active: false,
            toasts: Vec::new(),
            last_item: None,
            interaction_clip: None,
            rounded_clips: Vec::new(),
            mask_depth: 0,
            stencil_depth: 0,
            rounded_capture_path: Vec::new(),
            focus_rounding: Vec::new(),
            focused_rounding: Vec::new(),
            scroll_containers: Vec::new(),
            tab_bar_scopes: Vec::new(),
            current_window: None,
            window_scope: None,
            dt: 0.0,
            created_windows: 0,
            frame_active: false,
            draw_vertices: Vec::new(),
            draw_indices: Vec::new(),
            draw_commands: Vec::new(),
            draw_effects: Vec::new(),
        }
    }

    pub fn font(&self) -> &Font {
        &self.font
    }

    pub fn text_raster_mode(&self) -> crate::font::TextRasterMode {
        self.font.text_raster_mode()
    }

    pub fn set_color_atlas_texture(
        &mut self,
        texture: Option<crate::draw::TextureId>,
    ) -> Result<(), crate::font::ColorFontError> {
        assert!(
            !self.frame_active,
            "color font texture must change between frames"
        );
        let old = self.font.color_atlas_texture();
        self.font.set_color_atlas_texture(texture)?;
        if old != texture {
            self.popup_animation.invalidate();
        }
        Ok(())
    }

    pub fn set_text_raster_mode(&mut self, mode: crate::font::TextRasterMode) {
        assert!(
            !self.frame_active,
            "text raster mode must change between frames"
        );
        self.font.set_text_raster_mode(mode);
    }

    pub fn replace_font(&mut self, mut font: Font) {
        assert!(!self.frame_active, "replace_font must run between frames");
        assert_eq!(
            font.face_count(),
            self.font.face_count(),
            "replacement font must preserve face IDs"
        );
        font.set_text_raster_mode(self.font.text_raster_mode());
        font.set_color_atlas_texture(self.font.color_atlas_texture())
            .expect("existing color texture ID is valid");
        self.font = font;
        self.popup_animation.invalidate();
        for slot in &mut self.routing.popups {
            slot.motion.invalidate();
        }
    }

    pub fn add_font(&mut self, bytes: &[u8], px_size: f32) -> Result<FontId, FontError> {
        self.font.add_face(bytes, px_size)
    }

    pub fn add_font_with_icons(
        &mut self,
        bytes: &[u8],
        px_size: f32,
        icons: &[IconFont<'_>],
    ) -> Result<FontId, FontError> {
        self.font.add_face_with_icons(bytes, px_size, icons)
    }

    pub fn active_font(&self) -> FontId {
        self.font_stack.last().copied().unwrap_or(FontId::DEFAULT)
    }

    pub fn set_input(&mut self, input: InputState) {
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
        self.id_stack.inspector.reset();
        self.id_stack.semantics.reset();
        self.duplicate_ids.reset();
        self.reset_host_requests();
        core::mem::swap(
            &mut self.accessibility_actions.pending,
            &mut self.accessibility_actions.current,
        );
        self.accessibility_actions.pending.clear();
        self.text_host.caret = None;
        self.dt = dt;
        self.motion_frame = self.motion_frame.wrapping_add(1);
        self.prev_mouse_down = self.input.mouse_down;
        core::mem::swap(&mut self.input, &mut self.pending_input);
        self.pending_input.keys.clear();
        self.pending_input.mouse_pos = [0.0; 2];
        self.pending_input.mouse_down = [false; 5];
        self.pending_input.mouse_wheel = 0.0;
        self.pending_input.viewport_size = [0.0; 2];
        if self.input.mouse_down != self.prev_mouse_down
            || self.input.mouse_wheel != 0.
            || !self.input.keys.is_empty()
            || !self.accessibility_actions.current.is_empty()
        {
            self.invalidate();
        }
        self.routing.initial_modifiers = self.modifiers;
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
        self.route_input();
        core::mem::swap(&mut self.focus_order, &mut self.focus_order_prev);
        self.focus_order.clear();
        self.focus_rect = None;
        self.focus_clip = None;
        self.focus_visuals.clear();
        self.focus_mask_visuals.clear();
        self.focused_masks = Default::default();
        self.focus_rounding.clear();
        self.focused_rounding.clear();
        if self.input.mouse_down[0] && !self.prev_mouse_down[0] {
            self.focused = None;
            self.focused_in_modal = false;
            self.focused_in_popup = false;
        }
        if self
            .input
            .keys
            .iter()
            .any(|event| matches!(event, KeyEvent::Press(crate::input::Key::Tab)))
        {
            if self.focused.is_some() && self.active == self.focused {
                self.clear_active();
            }
            let order = &self.focus_order_prev;
            if !order.is_empty() {
                let next = self
                    .focused
                    .and_then(|id| order.iter().position(|&item| item == id))
                    .map(|index| {
                        if self.modifiers.shift {
                            (index + order.len() - 1) % order.len()
                        } else {
                            (index + 1) % order.len()
                        }
                    })
                    .unwrap_or(if self.modifiers.shift {
                        order.len() - 1
                    } else {
                        0
                    });
                self.focused = Some(order[next]);
                self.focused_in_modal = self.popup_modal;
                self.focused_in_popup = self.open_popup.is_some();
            }
        }
        self.keyboard_activate = if (self.open_popup.is_none() || self.focused_in_popup)
            && self.input.keys.iter().any(|event| {
                matches!(
                    event,
                    KeyEvent::Press(crate::input::Key::Enter | crate::input::Key::Space)
                )
            }) {
            self.focused
        } else {
            None
        };
        self.last_item = None;
        self.scroll_containers.clear();
        self.font_stack.clear();
        self.disabled_groups.clear();
        self.disabled_depth = 0;
        if let Some(index) = self.routing.popups[..self.routing.count]
            .iter()
            .position(|slot| self.motion_frame.wrapping_sub(slot.seen) > 1)
        {
            while self.routing.count > index {
                self.dismiss_popup();
            }
        }
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
            if escape || (outside_press && !self.routing.popups[self.routing.count - 1].modal) {
                let consume_press = outside_press;
                self.dismiss_popup();
                if escape {
                    self.input
                        .keys
                        .retain(|event| !matches!(event, KeyEvent::Press(Key::Escape)));
                }
                if consume_press {
                    self.prev_mouse_down[0] = true;
                }
            }
        }
        self.route_navigation();
        self.tick_popup_motion();
        self.press_target = PressTarget::None;
        if self.input.mouse_down[0] && !self.prev_mouse_down[0] {
            let inside_popup = self.open_popup.is_some()
                && (self.popup_modal
                    || self
                        .popup_rect
                        .is_some_and(|rect| point_in_rect(self.input.mouse_pos, rect)));
            self.press_target = if inside_popup {
                PressTarget::Popup
            } else {
                self.front_window_at(self.input.mouse_pos)
                    .map_or(PressTarget::None, PressTarget::Window)
            };
        }
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
        let step = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        for toast in &mut self.toasts {
            toast.remaining -= step;
        }
        self.toasts.retain(|toast| toast.remaining > 0.0);
        for index in 0..self.toasts.len() {
            self.draw_toast(index);
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

    pub(crate) fn push_toast(&mut self, text: &str, seconds: f32) {
        if !seconds.is_finite() || seconds <= 0.0 {
            return;
        }
        if self.toasts.len() == 6 {
            self.toasts.remove(0);
        }
        self.toasts.push(Toast {
            text: String::from(text),
            remaining: seconds,
            duration: seconds,
        });
    }

    fn draw_toast(&mut self, index: usize) {
        let toast = &self.toasts[index];
        let face = self.active_font();
        let style = self.style;
        let measured = self.font.measure_in(face, &toast.text);
        let width = measured[0] + 2.0 * style.window_padding[0];
        let height = measured[1] + 2.0 * style.window_padding[1];
        let viewport = self.input.viewport_size;
        let (slide_x, opacity) = toast_animation(toast.remaining, toast.duration);
        let fade = |mut color: [u8; 4]| {
            color[3] = (color[3] as f32 * opacity + 0.5) as u8;
            color
        };
        let base_x = if viewport[0] > 0.0 {
            (viewport[0] - width - 16.0).max(0.0)
        } else {
            16.0
        };
        let x = base_x + slide_x;
        let y = 56.0 + index as f32 * (height + 8.0);
        let rect = [x, y, x + width, y + height];
        crate::widgets::chrome_fill(
            &mut self.font,
            &mut self.overlay,
            style.corner_rounding,
            rect,
            fade(style.window_bg),
        );
        if style.border_size > 0.0 {
            crate::widgets::draw_border(
                &mut self.font,
                &mut self.overlay,
                style.corner_rounding,
                rect,
                style.border_size,
                fade(style.widget_bg_active),
            );
        }
        self.overlay.add_text(
            &mut self.font,
            face,
            [x + style.window_padding[0], y + style.window_padding[1]],
            &toast.text,
            fade(style.text),
        );
    }

    pub fn render(&mut self) -> DrawData<'_> {
        assert!(
            self.frame_active,
            "Context::render called without a preceding new_frame"
        );
        assert!(
            self.font_stack.is_empty(),
            "Context::render called with an unbalanced font stack: every \
             Ui::push_font must be matched by a Ui::pop_font before render"
        );
        assert!(
            self.disabled_groups.is_empty(),
            "Context::render called with an unbalanced disabled scope"
        );
        assert!(
            self.rounded_clips.is_empty(),
            "Context::render called with an unbalanced rounded clip"
        );
        assert_eq!(
            self.mask_depth, 0,
            "Context::render called with an unbalanced mask"
        );
        assert_eq!(
            self.stencil_depth, 0,
            "Context::render called with an unbalanced stencil clip"
        );
        assert_eq!(self.routing.depth, 0, "unbalanced popup scope");
        self.frame_active = false;
        self.text_host.events.clear();
        self.route_wheel();
        self.finish_host_requests();
        self.draw_popup_exit();
        if !self.input.mouse_down[0] && self.prev_mouse_down[0] {
            if let Some(state) = self.drag_state.take() {
                if self.active == Some(state.source) {
                    self.clear_active();
                }
            }
        }
        if self.motion.active() {
            for (index, &(rect, clip, color)) in self.focus_visuals.iter().enumerate() {
                self.overlay.push_clip_rect(clip);
                let previous_effects = self.overlay.effects_state();
                if let Some((_, masks)) = self.focus_mask_visuals.iter().find(|(i, _)| *i == index)
                {
                    self.overlay.set_effects(crate::effects::DrawEffects {
                        masks: *masks,
                        ..Default::default()
                    });
                }
                let rounded = self.focus_rounding.iter().any(|(i, _)| *i == index);
                let mark = if rounded {
                    self.overlay.paint_mark()
                } else {
                    [0; 2]
                };
                self.overlay.add_rect_outline(rect, 1.0, color, false);
                for (_, shape) in self.focus_rounding.iter().filter(|(i, _)| *i == index) {
                    self.overlay
                        .clip_rounded_tail(mark, *shape, self.font.raster_scale());
                }
                self.overlay.pop_clip_rect();
                self.overlay.set_effects(previous_effects);
            }
        } else if let (Some(id), Some(rect), Some(clip)) =
            (self.focused, self.focus_rect, self.focus_clip)
        {
            if self.focus_order.contains(&id)
                && rect[2] > rect[0]
                && rect[3] > rect[1]
                && clip[2] > clip[0]
                && clip[3] > clip[1]
                && (self.open_popup.is_none() || self.focused_in_popup)
            {
                self.overlay.push_clip_rect(clip);
                let previous_effects = self.overlay.effects_state();
                if self.focused_masks.count > 0 {
                    self.overlay.set_effects(crate::effects::DrawEffects {
                        masks: self.focused_masks,
                        ..Default::default()
                    });
                }
                let mark = if self.focused_rounding.is_empty() {
                    [0; 2]
                } else {
                    self.overlay.paint_mark()
                };
                self.overlay
                    .add_rect_outline(rect, 1.0, self.style.widget_bg_hovered, false);
                for shape in &self.focused_rounding {
                    self.overlay
                        .clip_rounded_tail(mark, *shape, self.font.raster_scale());
                }
                self.overlay.pop_clip_rect();
                self.overlay.set_effects(previous_effects);
            }
        }
        self.draw_vertices.clear();
        self.draw_indices.clear();
        self.draw_commands.clear();
        self.draw_effects.clear();
        for window in &self.windows {
            if !window.open {
                continue;
            }
            let data = window.draw_list.as_draw_data();
            let vertex_base = self.draw_vertices.len() as u32;
            self.draw_vertices.extend_from_slice(data.vertices);
            self.draw_indices
                .extend(data.indices.iter().map(|i| i + vertex_base));
            let base = self.draw_effects.len() as u32;
            self.draw_effects.extend_from_slice(data.effects);
            self.draw_commands.extend(data.commands.iter().map(|cmd| {
                let mut cmd = cmd.clone();
                if cmd.effects != 0 {
                    cmd.effects += base;
                }
                cmd
            }));
        }
        let data = self.overlay.as_draw_data();
        let vertex_base = self.draw_vertices.len() as u32;
        self.draw_vertices.extend_from_slice(data.vertices);
        self.draw_indices
            .extend(data.indices.iter().map(|i| i + vertex_base));
        let base = self.draw_effects.len() as u32;
        self.draw_effects.extend_from_slice(data.effects);
        self.draw_commands.extend(data.commands.iter().map(|cmd| {
            let mut cmd = cmd.clone();
            if cmd.effects != 0 {
                cmd.effects += base;
            }
            cmd
        }));
        DrawData {
            effects: &self.draw_effects,
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
            .scroll_containers
            .iter()
            .rev()
            .find(|entry| entry.layer_owns_point && point_in_rect(mouse, entry.rect))
            .map(|entry| (entry.id, entry.target, entry.max_offset, entry.line_height));
        let Some((id, target, max_offset, line_height)) = winner else {
            return;
        };
        let delta = self.input.mouse_wheel * crate::ui::WHEEL_LINES_PER_NOTCH * line_height;
        match target {
            ScrollTarget::Widget => {
                if let Some(WidgetState::Scroll { offset, .. }) = self.widget_states.get(&id) {
                    let old = *offset;
                    let next = self.scroll_target(id, old, delta, max_offset);
                    if let Some(WidgetState::Scroll { offset, .. }) =
                        self.widget_states.get_mut(&id)
                    {
                        *offset = next;
                    }
                }
            }
            ScrollTarget::Window => {
                if let Some(index) = self.windows.iter().position(|w| w.id == id) {
                    let old = self.windows[index].scroll_offset;
                    self.windows[index].scroll_offset =
                        self.scroll_target(id, old, delta, max_offset);
                }
            }
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
        self.active_in_popup = self.popup_scope_active;
        self.active_list_index = None;
        self.active_groups.clone_from(&self.disabled_groups);
        self.rounded_capture_path.clear();
        self.rounded_capture_path.extend(
            self.rounded_clips
                .iter()
                .filter(|clip| {
                    clip.popup == self.popup_scope_active
                        && clip.popup_id == self.routing.scope_id()
                        && clip.window == self.current_window
                })
                .map(|clip| clip.id),
        );
    }

    pub(crate) fn clear_active(&mut self) {
        self.active = None;
        self.active_in_popup = false;
        self.active_list_index = None;
        self.active_groups.clear();
        self.rounded_capture_path.clear();
    }

    pub(crate) fn release_popup_capture(&mut self) {
        if self.active_in_popup {
            if let Some(id) = self.active {
                if let Some(WidgetState::Keybind { armed }) = self.widget_states.get_mut(&id) {
                    *armed = false;
                }
            }
            self.clear_active();
        }
        if self.focused_in_popup {
            self.focused = None;
            self.focused_in_popup = false;
            self.focused_in_modal = false;
            self.focus_rect = None;
            self.focus_clip = None;
            self.keyboard_activate = None;
        }
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

    pub fn set_window_rect(&mut self, title: &str, pos: [f32; 2], size: [f32; 2]) {
        if !pos.iter().chain(size.iter()).all(|v| v.is_finite()) {
            return;
        }
        let index = self.find_or_create_window(title);
        let window = &mut self.windows[index];
        window.pos = pos;
        window.size = [
            size[0].max(MIN_WINDOW_SIZE[0]),
            size[1].max(MIN_WINDOW_SIZE[1]),
        ];
    }

    pub fn set_scrollbar_skin(&mut self, skin: crate::scroll::ScrollbarSkin) {
        let values = [
            skin.track_inset,
            skin.thumb_inset,
            skin.track_rounding,
            skin.thumb_rounding,
            skin.min_thumb,
        ];
        if values.iter().any(|v| !v.is_finite())
            || skin.track_inset < 0.0
            || skin.thumb_inset < 0.0
            || skin.track_rounding < 0.0
            || skin.thumb_rounding < -1.0
            || skin.min_thumb <= 0.0
        {
            return;
        }
        self.scrollbar_skin = skin;
    }

    pub fn scrollbar_skin(&self) -> crate::scroll::ScrollbarSkin {
        self.scrollbar_skin
    }

    pub fn hovered_widget_id(&self) -> Option<Id> {
        self.hot
    }

    pub fn captured_widget_id(&self) -> Option<Id> {
        self.active
    }

    pub fn focused_widget_id(&self) -> Option<Id> {
        self.focused
    }

    pub fn inspect_windows(&self) -> impl Iterator<Item = WindowInspection<'_>> + '_ {
        self.windows
            .iter()
            .filter(|window| window.open)
            .map(|window| {
                let data = window.draw_list.as_draw_data();
                WindowInspection {
                    title: &window.title,
                    rect: window.rect(),
                    vertices: data.vertices.len(),
                    indices: data.indices.len(),
                    commands: data.commands,
                }
            })
    }

    pub fn inspect_overlay_commands(&self) -> &[DrawCmd] {
        self.overlay.as_draw_data().commands
    }

    pub fn wants_mouse_capture(&self) -> bool {
        if self.active.is_some() || self.hover_top_window.is_some() {
            return true;
        }
        self.routing.popups[..self.routing.count]
            .iter()
            .any(|slot| {
                slot.rect
                    .is_some_and(|rect| point_in_rect(self.input.mouse_pos, rect))
            })
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
    #[test]
    fn tab_focuses_visible_buttons_and_enter_activates_once() {
        use crate::input::{InputState, Key, KeyEvent};
        use crate::widgets::test_support::{frame, frame_with_input, test_context};
        let mut ctx = test_context();
        let mut first = None;
        frame(&mut ctx, [-50.0, -50.0], false, |ui| {
            first = Some(ui.widget_id("First"));
            ui.button("First");
            ui.button("Second");
        });
        frame_with_input(
            &mut ctx,
            0.016,
            InputState {
                keys: &[KeyEvent::Press(Key::Tab)],
                ..Default::default()
            },
            |ui| {
                ui.button("First");
                ui.button("Second");
            },
        );
        assert_eq!(ctx.focused_widget_id(), first);
        let mut activated = false;
        frame_with_input(
            &mut ctx,
            0.016,
            InputState {
                keys: &[KeyEvent::Press(Key::Enter)],
                ..Default::default()
            },
            |ui| {
                activated = ui.button("First");
                ui.button("Second");
            },
        );
        assert!(activated);
        frame(&mut ctx, [-50.0, -50.0], false, |ui| {
            activated = ui.button("First");
            ui.button("Second");
        });
        assert!(!activated);
    }

    #[test]
    fn fully_clipped_custom_control_is_not_a_tab_stop() {
        use crate::widgets::test_support::{frame, test_context};
        let mut ctx = test_context();
        frame(&mut ctx, [-50.0, -50.0], false, |ui| {
            let id = ui.widget_id("offscreen");
            ui.focusable(id, [1000.0, 1000.0, 1030.0, 1030.0]);
        });
        assert!(ctx.focus_order.is_empty());
    }

    #[test]
    fn modal_focus_order_excludes_background_controls() {
        use crate::widgets::test_support::{frame, test_context};
        let mut ctx = test_context();
        frame(&mut ctx, [-50.0, -50.0], false, |ui| {
            let modal = ui.widget_id("modal");
            ui.open_modal(modal);
            ui.button("Background");
            if ui.begin_modal(modal, [40.0, 40.0, 220.0, 140.0]) {
                ui.button("Inside");
                ui.end_popup();
            }
        });
        assert_eq!(ctx.focus_order.len(), 1);
    }
    use super::*;
    use crate::input::Key;
    use crate::scroll::ScrollFlags;

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
            mouse_down: [true, false, false, false, false],
            mouse_wheel: 1.0,
            keys: &[KeyEvent::Char('a'), KeyEvent::Press(Key::Enter)],
            viewport_size: [800.0, 600.0],
        });
        ctx.new_frame(0.016);
        assert_eq!(ctx.input.mouse_pos, [10.0, 20.0]);
        assert_eq!(ctx.input.mouse_down, [true, false, false, false, false]);
        assert_eq!(ctx.input.mouse_wheel, 1.0);
        assert_eq!(ctx.input.viewport_size, [800.0, 600.0]);
        assert_eq!(
            ctx.input.keys,
            alloc::vec![KeyEvent::Char('a'), KeyEvent::Press(Key::Enter)]
        );
        assert_eq!(ctx.prev_mouse_down, [false; 5]);
        ctx.render();

        ctx.set_input(InputState {
            mouse_pos: [11.0, 21.0],
            mouse_down: [false, false, false, false, false],
            mouse_wheel: 0.0,
            keys: &[],
            viewport_size: [800.0, 600.0],
        });
        ctx.new_frame(0.016);
        assert_eq!(ctx.input.mouse_down, [false; 5]);
        assert_eq!(ctx.prev_mouse_down, [true, false, false, false, false]);
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
        ui.window("Win", ScrollFlags::NONE, |_| {});
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
        ui.window("A", ScrollFlags::NONE, |_| {});
        ui.window("B", ScrollFlags::NONE, |_| {});
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
        ui.window("A", ScrollFlags::NONE, |_| {});
        let _ = ui;
        assert_eq!(ctx.windows.len(), 2);
        ctx.render();
    }

    #[test]
    fn host_can_place_windows_before_the_first_frame() {
        let mut ctx = test_context();
        ctx.set_window_rect("Gallery", [72.0, 86.0], [720.0, 640.0]);
        ctx.set_window_rect("Inspector", [816.0, 86.0], [462.0, 300.0]);
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.window("Gallery", ScrollFlags::NONE, |_| {});
        ui.window("Inspector", ScrollFlags::NONE, |_| {});
        let _ = ui;
        assert_eq!(ctx.windows[0].pos, [72.0, 86.0]);
        assert_eq!(ctx.windows[0].size, [720.0, 640.0]);
        assert_eq!(ctx.windows[1].pos, [816.0, 86.0]);
        assert_eq!(ctx.windows[1].size, [462.0, 300.0]);
        assert_eq!(ctx.windows.len(), 2);
        ctx.render();
    }

    fn test_context_at(px_size: f32) -> Context {
        let font = Font::new(DEJAVU_SANS, px_size).expect("DejaVu Sans must load");
        Context::with_font(font)
    }

    #[test]
    fn replacing_font_for_dpi_keeps_window_state() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        ctx.ui().window("Persistent", ScrollFlags::NONE, |_| {});
        let _ = ctx.render();
        ctx.set_window_open("Persistent", false);
        let before = (ctx.windows[0].pos, ctx.windows[0].size, ctx.windows[0].open);

        let font = Font::new_with_icons_at_scale(DEJAVU_SANS, DEFAULT_FONT_SIZE, &[], 1.5).unwrap();
        ctx.replace_font(font);
        assert_eq!(
            (ctx.windows[0].pos, ctx.windows[0].size, ctx.windows[0].open),
            before
        );
        assert_eq!(ctx.font().raster_scale(), 1.5);
    }

    #[test]
    fn subpixel_mode_survives_dpi_replacement() {
        let mut ctx = test_context();
        ctx.set_text_raster_mode(crate::font::TextRasterMode::Subpixel);
        let font = Font::new_with_icons_at_scale(DEJAVU_SANS, DEFAULT_FONT_SIZE, &[], 1.5).unwrap();
        ctx.replace_font(font);
        assert_eq!(
            ctx.text_raster_mode(),
            crate::font::TextRasterMode::Subpixel
        );
        assert_eq!(ctx.font().raster_scale(), 1.5);
        ctx.new_frame(0.016);
        ctx.ui().window("phase", ScrollFlags::NONE, |ui| {
            ui.draw_text_at([100.25, 90.5], "Fractional text", [255; 4]);
        });
        assert!(!ctx.render().vertices.is_empty());
    }

    fn content_start_y(ctx: &mut Context) -> f32 {
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.window("Win", ScrollFlags::NONE, |_| {});
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
        ui.window("Win", ScrollFlags::NONE, |ui| {
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
        ui.window("Title Text", ScrollFlags::NONE, |_| {});
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
        ui.window("Win", ScrollFlags::NONE, |ui| {
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
