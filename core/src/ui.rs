use crate::context::{Context, TabBarScopeState, WidgetState, MIN_WINDOW_SIZE};
use crate::draw::TextureId;
use crate::font::FontId;
use crate::id::{split_label, Id};
use crate::input::{Key, KeyBinding, KeyEvent};
use crate::layout::Cursor;
use crate::scroll::{ScrollContainerEntry, ScrollFlags};
use crate::style::Style;
use alloc::vec::Vec;

pub(crate) const TITLE_BAR_TAG: &str = "##titlebar";

pub(crate) const CLOSE_BUTTON_TAG: &str = "##close";

pub(crate) const RESIZE_GRIP_TAG: &str = "##resize";

pub(crate) const RESIZE_GRIP_SIZE: f32 = 16.0;

const CLOSE_GLYPH_INSET: f32 = 5.0;

const DRAG_VIEWPORT_MARGIN: f32 = 32.0;

const COLOR_EDIT_CHECKER_CELL: f32 = 4.0;

const COLOR_EDIT_CHECKER_LIGHT: [u8; 4] = [160, 160, 160, 255];
const COLOR_EDIT_CHECKER_DARK: [u8; 4] = [96, 96, 96, 255];

const COLOR_EDIT_POPUP_MIN_WIDTH: f32 = 160.0;

const COLOR_EDIT_PREVIEW_ROWS: f32 = 2.0;

pub(crate) const COLOR_PICKER_SV_WIDTH_RATIO: f32 = 1.1;

pub(crate) fn color_picker_column_height(row_h: f32, spacing_y: f32) -> f32 {
    (COLOR_EDIT_PREVIEW_ROWS + 5.0) * row_h + 5.0 * spacing_y
}

#[cfg(test)]
mod overlay_contract_tests {
    use crate::{context::Context, id::Id, input::InputState, scroll::ScrollFlags};

    const FONT: &[u8] = include_bytes!("../tests/assets/DejaVuSans.ttf");

    fn frame(
        ctx: &mut Context,
        mouse: [f32; 2],
        buttons: [bool; 5],
        dt: f32,
        declare: impl FnOnce(&mut crate::ui::Ui<'_>),
    ) {
        ctx.set_input(InputState {
            mouse_pos: mouse,
            mouse_down: buttons,
            viewport_size: [500.0, 400.0],
            ..InputState::default()
        });
        ctx.new_frame(dt);
        declare(&mut ctx.ui());
        ctx.render();
    }

    #[test]
    fn context_menu_anchors_at_right_press_and_keeps_popup_scope_balanced() {
        let mut ctx = Context::new(FONT).expect("font");
        let menu = Id(88);
        frame(
            &mut ctx,
            [120.0, 100.0],
            [false, true, false, false, false],
            0.016,
            |ui| {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    assert!(ui.begin_context_menu(menu, [100.0, 80.0, 180.0, 130.0], [90.0, 70.0]));
                    ui.label("Menu item");
                    ui.end_popup();
                });
            },
        );
        assert_eq!(ctx.popup_anchor, [120.0, 100.0]);
        assert_eq!(ctx.popup_rect, Some([120.0, 100.0, 210.0, 170.0]));
        assert!(!ctx.popup_scope_active);
    }

    #[test]
    fn modal_blocks_outside_press_but_accepts_content_click() {
        let mut ctx = Context::new(FONT).expect("font");
        ctx.set_window_rect("W", [0.0, 0.0], [400.0, 350.0]);
        let modal = Id(91);
        frame(&mut ctx, [0.0, 0.0], [false; 5], 0.016, |ui| {
            ui.window("W", ScrollFlags::NONE, |ui| ui.open_modal(modal));
        });
        frame(
            &mut ctx,
            [50.0, 100.0],
            [true, false, false, false, false],
            0.016,
            |ui| {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    assert!(!ui.mouse_pressed(0));
                    assert!(!ui.is_hovered([40.0, 90.0, 90.0, 120.0]));
                    ui.modal(modal, [220.0, 160.0, 380.0, 280.0], |ui| {
                        ui.button("Accept");
                    });
                });
            },
        );
        assert_eq!(ctx.open_popup, Some(modal));
        frame(&mut ctx, [230.0, 175.0], [false; 5], 0.016, |ui| {
            ui.window("W", ScrollFlags::NONE, |ui| {
                ui.modal(modal, [220.0, 160.0, 380.0, 280.0], |ui| {
                    ui.button("Accept");
                })
            });
        });
        frame(
            &mut ctx,
            [230.0, 175.0],
            [true, false, false, false, false],
            0.016,
            |ui| {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    ui.modal(modal, [220.0, 160.0, 380.0, 280.0], |ui| {
                        assert!(!ui.button("Accept"));
                    })
                });
            },
        );
        frame(&mut ctx, [230.0, 175.0], [false; 5], 0.016, |ui| {
            ui.window("W", ScrollFlags::NONE, |ui| {
                ui.modal(modal, [220.0, 160.0, 380.0, 280.0], |ui| {
                    assert!(ui.button("Accept"));
                })
            });
        });
    }

    #[test]
    fn toast_is_inert_and_expires_after_elapsed_frame_time() {
        let mut ctx = Context::new(FONT).expect("font");
        frame(&mut ctx, [0.0, 0.0], [false; 5], 0.016, |ui| {
            ui.toast("Saved", 0.2)
        });
        assert_eq!(ctx.toasts.len(), 1);
        frame(&mut ctx, [0.0, 0.0], [false; 5], 0.1, |_| {});
        assert_eq!(ctx.toasts.len(), 1);
        assert!(!ctx.overlay.as_draw_data().vertices.is_empty());
        frame(&mut ctx, [0.0, 0.0], [false; 5], 0.11, |_| {});
        frame(&mut ctx, [0.0, 0.0], [false; 5], 0.01, |_| {});
        assert!(ctx.toasts.is_empty());
        assert!(ctx.overlay.as_draw_data().vertices.is_empty());
    }
}

pub(crate) const COLOR_PICKER_HUE_STRIP_WIDTH: f32 = 16.0;

pub(crate) const COLOR_PICKER_COLUMN_WIDTH: f32 = 200.0;

const SV_MARKER_RADIUS: f32 = 4.0;

const DRAG_MARKER_CORE: [u8; 4] = [255, 255, 255, 255];
const DRAG_MARKER_EDGE: [u8; 4] = [0, 0, 0, 255];

const KEYBIND_PROMPT: &str = "press any key...";

pub(crate) const TOOLTIP_DELAY: f32 = 0.5;

pub(crate) const TOOLTIP_OFFSET: [f32; 2] = [12.0, 16.0];

const TOOLTIP_STATE_SALT: u64 = 0x9e37_79b9_7f4a_7c15;

pub const WHEEL_LINES_PER_NOTCH: f32 = 3.0;

pub struct Ui<'a> {
    pub(crate) ctx: &'a mut Context,
}

pub struct DisabledScope {
    id: Id,
    disabled: bool,
    masked: bool,
    style: Style,
    mouse_pos: [f32; 2],
    mouse_down: [bool; 5],
    prev_mouse_down: [bool; 5],
    mouse_wheel: f32,
    keys: Option<Vec<KeyEvent>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationState {
    Success,
    Warning,
    Error,
}

impl Ui<'_> {
    pub fn disabled(&mut self, label: &str, disabled: bool, f: impl FnOnce(&mut Ui)) {
        let scope = self.begin_disabled(label, disabled);
        f(self);
        self.end_disabled(scope);
    }

    pub fn begin_disabled(&mut self, label: &str, disabled: bool) -> DisabledScope {
        let ctx = &mut *self.ctx;
        let id = ctx.id_stack.id(label);
        ctx.id_stack.push_id(id);
        ctx.disabled_groups.push(id);
        if disabled && ctx.active_groups.contains(&id) {
            ctx.clear_active();
        }
        let masked = disabled && ctx.disabled_depth == 0;
        let mut scope = DisabledScope {
            id,
            disabled,
            masked,
            style: ctx.style,
            mouse_pos: ctx.input.mouse_pos,
            mouse_down: ctx.input.mouse_down,
            prev_mouse_down: ctx.prev_mouse_down,
            mouse_wheel: ctx.input.mouse_wheel,
            keys: None,
        };
        if disabled {
            ctx.disabled_depth += 1;
        }
        if masked {
            let fade = |c: &mut [u8; 4]| c[3] = ((c[3] as u16 * 110) / 255) as u8;
            fade(&mut ctx.style.window_bg);
            fade(&mut ctx.style.title_bg);
            fade(&mut ctx.style.title_text);
            fade(&mut ctx.style.widget_bg);
            fade(&mut ctx.style.widget_bg_hovered);
            fade(&mut ctx.style.widget_bg_active);
            fade(&mut ctx.style.text);
            fade(&mut ctx.style.scrollbar_bg);
            fade(&mut ctx.style.scrollbar_thumb);
            fade(&mut ctx.style.scrollbar_thumb_hovered);
            fade(&mut ctx.style.scrollbar_thumb_active);
            ctx.input.mouse_pos = [f32::NAN; 2];
            ctx.input.mouse_down = [false; 5];
            ctx.prev_mouse_down = [false; 5];
            ctx.input.mouse_wheel = 0.0;
            scope.keys = Some(core::mem::take(&mut ctx.input.keys));
        }
        scope
    }

    pub fn end_disabled(&mut self, scope: DisabledScope) {
        let ctx = &mut *self.ctx;
        assert_eq!(
            ctx.disabled_groups.pop(),
            Some(scope.id),
            "disabled scopes must close in reverse order"
        );
        ctx.id_stack.pop();
        if scope.masked {
            ctx.style = scope.style;
        }
        if scope.masked {
            ctx.input.mouse_pos = scope.mouse_pos;
            ctx.input.mouse_down = scope.mouse_down;
            ctx.prev_mouse_down = scope.prev_mouse_down;
            ctx.input.mouse_wheel = scope.mouse_wheel;
            ctx.input.keys = scope.keys.expect("masked disabled scope owns input keys");
        }
        if scope.disabled {
            ctx.disabled_depth -= 1;
        }
    }
    pub fn window(&mut self, title: &str, flags: ScrollFlags, f: impl FnOnce(&mut Ui)) {
        if self.begin_window(title, flags) {
            f(self);
            self.end_window();
        }
    }

    pub fn begin_window(&mut self, title: &str, flags: ScrollFlags) -> bool {
        let ctx = &mut *self.ctx;
        assert!(
            ctx.current_window.is_none(),
            "Ui::begin_window called while a window is already open: \
             nested windows are not supported"
        );
        let mut index = ctx.find_or_create_window(title);
        if !ctx.windows[index].open {
            return false;
        }
        let window_id = ctx.windows[index].id;
        ctx.id_stack.push(title);
        ctx.id_stack.inspector.scope_rect(ctx.windows[index].rect());
        ctx.id_stack.semantics.scope_rect(ctx.windows[index].rect());
        let chrome_id = ctx.id_stack.id(TITLE_BAR_TAG);
        let close_id = ctx.id_stack.id(CLOSE_BUTTON_TAG);
        let resize_id = ctx.id_stack.id(RESIZE_GRIP_TAG);

        let style = ctx.style;
        let face = ctx.active_font();
        let title_bar_height = ctx.font().line_height_in(face) + 2.0 * style.frame_padding[1];
        let mouse = ctx.input.mouse_pos;
        let viewport_size = ctx.input.viewport_size;
        let left_down = ctx.input.mouse_down[0];
        let left_pressed = left_down && !ctx.prev_mouse_down[0];

        let is_press_target = ctx.press_target == crate::context::PressTarget::Window(window_id);
        if is_press_target {
            ctx.bring_window_to_front(index);
            index = ctx.window_index(window_id);
        }

        ctx.current_window = Some(window_id);
        ctx.last_item = None;

        let title_bar_rect = {
            let window = &ctx.windows[index];
            [
                window.pos[0],
                window.pos[1],
                window.pos[0] + window.size[0],
                window.pos[1] + title_bar_height,
            ]
        };
        let close_rect = [
            title_bar_rect[2] - title_bar_height,
            title_bar_rect[1],
            title_bar_rect[2],
            title_bar_rect[3],
        ];
        let grip_rect = [
            title_bar_rect[2] - RESIZE_GRIP_SIZE,
            title_bar_rect[1] + ctx.windows[index].size[1] - RESIZE_GRIP_SIZE,
            title_bar_rect[2],
            title_bar_rect[1] + ctx.windows[index].size[1],
        ];
        let grip_pressed = left_pressed && is_press_target && point_in_rect(mouse, grip_rect);
        let close_held = ctx.active == Some(close_id);
        let close_clicked = close_held && !left_down && point_in_rect(mouse, close_rect);
        if left_pressed && is_press_target && point_in_rect(mouse, close_rect) {
            ctx.set_active(close_id);
        } else if left_pressed && is_press_target && point_in_rect(mouse, title_bar_rect) {
            ctx.set_active(chrome_id);
            ctx.active_drag_offset = [mouse[0] - title_bar_rect[0], mouse[1] - title_bar_rect[1]];
        } else if grip_pressed {
            ctx.set_active(resize_id);
            ctx.active_drag_offset = [mouse[0] - grip_rect[2], mouse[1] - grip_rect[3]];
        }
        if ctx.active == Some(chrome_id) {
            if left_down {
                let offset = ctx.active_drag_offset;
                let window = &mut ctx.windows[index];
                let mut new_pos = [mouse[0] - offset[0], mouse[1] - offset[1]];
                if viewport_size != [0.0, 0.0] {
                    let min_x = DRAG_VIEWPORT_MARGIN - window.size[0];
                    new_pos[0] = new_pos[0]
                        .clamp(min_x, (viewport_size[0] - DRAG_VIEWPORT_MARGIN).max(min_x));
                    let max_y = viewport_size[1] - title_bar_height;
                    new_pos[1] = new_pos[1].clamp(0.0, max_y.max(0.0));
                }
                if new_pos != window.pos {
                    window.pos = new_pos;
                    window.draw_list.replace_base_clip(window.rect());
                }
            } else {
                ctx.clear_active();
            }
        }
        if ctx.active == Some(resize_id) {
            if left_down {
                let offset = ctx.active_drag_offset;
                let window = &mut ctx.windows[index];
                let new_size = [
                    (mouse[0] - offset[0] - window.pos[0]).max(MIN_WINDOW_SIZE[0]),
                    (mouse[1] - offset[1] - window.pos[1]).max(MIN_WINDOW_SIZE[1]),
                ];
                if new_size != window.size {
                    window.size = new_size;
                    window.draw_list.replace_base_clip(window.rect());
                }
            } else {
                ctx.clear_active();
            }
        }
        if close_held && !left_down {
            ctx.clear_active();
            if close_clicked {
                ctx.windows[index].open = false;
            }
        }
        let close_hovered = hovered(ctx, close_rect);
        let close_pressed = ctx.active == Some(close_id) && point_in_rect(mouse, close_rect);

        let body_rect = {
            let window = &ctx.windows[index];
            [
                window.pos[0],
                window.pos[1] + title_bar_height,
                window.pos[0] + window.size[0],
                window.pos[1] + window.size[1],
            ]
        };
        ctx.id_stack.semantics.scope_rect(ctx.windows[index].rect());
        let content_height = ctx.windows[index].content_height;
        let geometry = crate::scroll::geometry(&style, body_rect, 0.0, flags, content_height);
        let content_rect = geometry.content_rect;
        let scrollbar_rect = geometry.scrollbar_rect;
        if flags.scrollable {
            ctx.scroll_containers.push(ScrollContainerEntry {
                id: window_id,
                target: crate::scroll::ScrollTarget::Window,
                rect: body_rect,
                layer_owns_point: hover_layer_gate(ctx),
                max_offset: geometry.max_offset,
                line_height: ctx.font.line_height_in(face),
            });
        }
        let mut offset = ctx.windows[index]
            .scroll_offset
            .clamp(0.0, geometry.max_offset);
        if flags.scrollable {
            offset = ctx.scroll_position(ctx.windows[index].id, offset, geometry.max_offset);
            ctx.windows[index].scroll_offset = offset;
        }
        let content_width = match scrollbar_rect {
            Some(bar) => bar[0] - body_rect[0],
            None => ctx.windows[index].size[0],
        };

        {
            let font = &mut ctx.font;
            let window = &mut ctx.windows[index];
            window.draw_list.push_clip_rect(window.rect());
            window.cursor = Cursor::new(
                [window.pos[0], window.pos[1] + title_bar_height - offset],
                content_width,
                style.window_padding,
                style.item_spacing,
            );
            let rect = window.rect();
            crate::widgets::chrome_fill(
                font,
                &mut window.draw_list,
                style.corner_rounding,
                rect,
                style.window_bg,
            );
            crate::widgets::chrome_fill_top_rounded(
                font,
                &mut window.draw_list,
                style.corner_rounding,
                [rect[0], rect[1], rect[2], rect[1] + title_bar_height],
                style.title_bg,
            );
            if close_pressed {
                window.draw_list.add_filled_rect(
                    [close_rect[0], close_rect[1]],
                    [close_rect[2], close_rect[3]],
                    style.widget_bg_active,
                );
            } else if close_hovered {
                window.draw_list.add_filled_rect(
                    [close_rect[0], close_rect[1]],
                    [close_rect[2], close_rect[3]],
                    style.widget_bg_hovered,
                );
            }
            let text_pos = [
                rect[0] + style.window_padding[0],
                rect[1] + style.frame_padding[1],
            ];
            let display = split_label(title).0;
            let max_text_width = close_rect[0] - text_pos[0];
            let mut text = display;
            if font.measure_in(face, text)[0] > max_text_width {
                let mut len = text.len();
                while len > 0 && font.measure_in(face, &text[..len])[0] > max_text_width {
                    len = text[..len].char_indices().next_back().map_or(0, |(i, _)| i);
                }
                text = &text[..len];
            }
            window
                .draw_list
                .add_text(font, face, text_pos, text, style.title_text);
            let g0 = [
                close_rect[0] + CLOSE_GLYPH_INSET,
                close_rect[1] + CLOSE_GLYPH_INSET,
            ];
            let g1 = [
                close_rect[2] - CLOSE_GLYPH_INSET,
                close_rect[3] - CLOSE_GLYPH_INSET,
            ];
            let aa = style.anti_aliasing;
            window
                .draw_list
                .add_thick_line(g0, g1, style.title_text, 1.0, aa);
            window.draw_list.add_thick_line(
                [g0[0], g1[1]],
                [g1[0], g0[1]],
                style.title_text,
                1.0,
                aa,
            );
        }
        let suppress_press = ctx.popup_modal
            || (!is_press_target && ctx.press_target != crate::context::PressTarget::None)
            || grip_pressed
            || (flags.scrollable && !point_in_rect(mouse, content_rect));

        if flags.scrollable {
            ctx.windows[index].draw_list.push_clip_rect(content_rect);
        }

        let saved_interaction_clip = ctx.interaction_clip;
        if flags.scrollable {
            ctx.interaction_clip = Some(match saved_interaction_clip {
                Some(outer) => crate::draw::intersect_rects(outer, content_rect),
                None => content_rect,
            });
        }

        ctx.window_scope = Some(crate::context::WindowScopeState {
            flags,
            body_rect,
            content_origin_y: body_rect[1] - offset,
            item_spacing_y: style.item_spacing[1],
            scrollbar_rect,
            saved_interaction_clip,
            saved_prev_left: ctx.prev_mouse_down[0],
        });
        if suppress_press {
            ctx.prev_mouse_down[0] = true;
        }
        true
    }

    pub fn end_window(&mut self) {
        let ctx = &mut *self.ctx;
        let window_id = ctx.current_window.expect(
            "Ui::end_window called with no window open: every end_window \
             must follow a begin_window that returned true",
        );
        let scope = ctx
            .window_scope
            .take()
            .expect("an open window scope always carries its begin-half state");
        let end_y = ctx.current_window_mut().cursor.pos_y();
        let content_height = (end_y - scope.content_origin_y - scope.item_spacing_y).max(0.0);
        ctx.prev_mouse_down[0] = scope.saved_prev_left;
        ctx.interaction_clip = scope.saved_interaction_clip;
        let index = ctx.window_index(window_id);
        ctx.windows[index].content_height = content_height;
        if scope.flags.scrollable {
            ctx.windows[index].draw_list.pop_clip_rect();
        }
        match scope.scrollbar_rect {
            Some(track) if scope.flags.scrollable && scope.flags.scrollbar => {
                let bar_id = ctx.id_stack.id(crate::scroll::SCROLLBAR_TAG);
                let visible_height = (scope.body_rect[3] - scope.body_rect[1]).max(0.0);
                let offset = ctx.windows[index].scroll_offset;
                let offset = crate::scroll::scrollbar(
                    ctx,
                    bar_id,
                    track,
                    offset,
                    (content_height - visible_height).max(0.0),
                    visible_height,
                    content_height,
                );
                if offset != ctx.windows[index].scroll_offset
                    || ctx.active == Some(bar_id)
                    || (ctx.hot == Some(bar_id)
                        && ctx.input.mouse_down[0]
                        && !ctx.prev_mouse_down[0])
                {
                    ctx.scroll_snap(ctx.windows[index].id, offset);
                }
                ctx.windows[index].scroll_offset = offset;
            }
            _ => {}
        }
        ctx.current_window = None;
        let style = ctx.style;
        let window = &mut ctx.windows[index];
        let rect = window.rect();
        let r = if style.corner_rounding.is_finite() {
            style
                .corner_rounding
                .min((rect[2] - rect[0]).min(rect[3] - rect[1]) / 2.0)
        } else {
            0.0
        };
        let inset = if r > 0.0 {
            r * (1.0 - core::f32::consts::FRAC_1_SQRT_2)
        } else {
            0.0
        };
        window.draw_list.add_filled_polygon(
            &[
                [rect[2] - inset, rect[3] - RESIZE_GRIP_SIZE],
                [rect[2] - inset, rect[3] - inset],
                [rect[2] - RESIZE_GRIP_SIZE, rect[3] - inset],
            ],
            style.title_text,
            style.anti_aliasing,
        );
        window.draw_list.pop_clip_rect();
        ctx.id_stack.pop();
    }

    pub fn push_font(&mut self, id: FontId) {
        assert!(
            self.ctx.font.is_valid(id),
            "Ui::push_font called with an unknown FontId: faces come from \
             Context::add_font / Context::add_font_with_icons"
        );
        self.ctx.font_stack.push(id);
    }

    pub fn pop_font(&mut self) {
        self.ctx.font_stack.pop().expect(
            "Ui::pop_font called with no font scope open: every pop_font \
             must follow a Ui::push_font",
        );
    }

    pub fn with_font(&mut self, id: FontId, f: impl FnOnce(&mut Ui)) {
        self.push_font(id);
        f(self);
        self.pop_font();
    }

    pub fn label(&mut self, text: &str) {
        crate::widgets::label::label(self, text);
    }

    pub fn button(&mut self, label: &str) -> bool {
        crate::widgets::button::button(self, label)
    }

    pub fn icon_button(&mut self, label: &str, side: f32) -> bool {
        crate::widgets::button::icon_button(self, label, side)
    }

    pub fn separator(&mut self) {
        crate::widgets::decorations::separator(self);
    }

    pub fn color_swatch(&mut self, label: &str, color: [u8; 4], size: [f32; 2]) {
        crate::widgets::decorations::color_swatch(self, label, color, size);
    }

    pub fn status_badge(&mut self, label: &str, color: [u8; 4]) {
        crate::widgets::decorations::status_badge(self, label, color);
    }

    pub fn hotkey_hint(&mut self, label: &str) {
        crate::widgets::decorations::hotkey_hint(self, label);
    }

    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> bool {
        crate::widgets::checkbox::checkbox(self, label, value)
    }

    pub fn pill_toggle(&mut self, label: &str, value: &mut bool) -> bool {
        crate::widgets::choice_controls::pill_toggle(self, label, value)
    }

    pub fn segmented_control(&mut self, label: &str, items: &[&str], selected: &mut usize) -> bool {
        crate::widgets::choice_controls::segmented_control(self, label, items, selected)
    }

    pub fn radio_button(&mut self, label: &str, selected: &mut usize, index: usize) -> bool {
        crate::widgets::radio::radio_button(self, label, selected, index)
    }

    pub fn validation_message(&mut self, state: ValidationState, message: &str) {
        crate::widgets::form_nav::validation_message(self, state, message);
    }

    pub fn nav_row(&mut self, label: &str, selected: &mut usize, index: usize) -> bool {
        crate::widgets::form_nav::nav_row(self, label, selected, index)
    }

    pub fn breadcrumbs(&mut self, label: &str, items: &[&str], selected: &mut usize) -> bool {
        crate::widgets::form_nav::breadcrumbs(self, label, items, selected)
    }

    pub fn search_box(&mut self, label: &str, query: &mut alloc::string::String) -> bool {
        let ctx = &mut *self.ctx;
        let id = ctx.id_stack.id(label);
        let style = ctx.style;
        let face = ctx.active_font();
        let width = crate::widgets::available_width(ctx).max(0.0);
        let height = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
        let rect = crate::widgets::item_rect(ctx, [width, height]);
        {
            let (font, draw) = crate::widgets::draw_parts(ctx);
            crate::widgets::chrome_fill(font, draw, style.corner_rounding, rect, style.widget_bg);
            if style.border_size > 0.0 {
                crate::widgets::draw_border(
                    font,
                    draw,
                    style.corner_rounding,
                    rect,
                    style.border_size,
                    crate::widgets::lighten(style.widget_bg, crate::widgets::BORDER_LIGHTEN),
                );
            }
        }
        ctx.id_stack.push_id(id);
        let query_id = ctx.id_stack.id("##query");
        let popup_scope = ctx.popup_scope_active;
        let saved_cursor = if popup_scope {
            ctx.overlay_cursor
        } else {
            ctx.current_window_mut().cursor
        };
        let inner_width = (width - 46.0).max(0.0);
        if popup_scope {
            ctx.overlay_cursor
                .enter_region([rect[0] + 22.0, rect[1]], inner_width);
        } else {
            ctx.current_window_mut()
                .cursor
                .enter_region([rect[0] + 22.0, rect[1]], inner_width);
        }
        let changed = crate::widgets::text_input::embedded_text_input(self, "##query", query);
        let ctx = &mut *self.ctx;
        if popup_scope {
            ctx.overlay_cursor = saved_cursor;
        } else {
            ctx.current_window_mut().cursor = saved_cursor;
        }
        let search_col = style.text;
        let icon_x = rect[0] + 10.0;
        let icon_y = (rect[1] + rect[3]) * 0.5 - 1.5;
        let center = [icon_x, icon_y];
        let (font, draw) = crate::widgets::draw_parts(ctx);
        if !crate::widgets::radio::circle_mask(
            font,
            draw,
            crate::font::ShapeKey::ring(5.0, 1.5),
            center,
            search_col,
        ) {
            draw.add_circle_outline(center, 5.0, search_col, 1.5, style.anti_aliasing);
        }
        draw.add_thick_line_with_caps(
            [icon_x + 3.7, icon_y + 3.7],
            [icon_x + 7.5, icon_y + 7.5],
            search_col,
            1.5,
            style.anti_aliasing,
        );
        if query.is_empty() && ctx.active != Some(query_id) {
            let display = split_label(label).0;
            let mut placeholder = style.text;
            placeholder[3] /= 2;
            let (font, draw) = crate::widgets::draw_parts(ctx);
            draw.add_text(
                font,
                face,
                [
                    rect[0] + 22.0 + style.frame_padding[0],
                    rect[1] + style.frame_padding[1],
                ],
                display,
                placeholder,
            );
        }
        let clear_id = ctx.id_stack.id("##clear");
        let clear_rect = [rect[2] - 24.0, rect[1], rect[2], rect[3]];
        let inside = !query.is_empty()
            && point_in_rect(ctx.input.mouse_pos, clear_rect)
            && hover_layer_gate(ctx);
        let down = ctx.input.mouse_down[0];
        if inside && down && !ctx.prev_mouse_down[0] {
            ctx.set_active(clear_id);
        }
        let held = ctx.active == Some(clear_id);
        let clear = held && !down && inside;
        if held && !down {
            ctx.clear_active();
        }
        if !query.is_empty() && hovered(ctx, clear_rect) {
            ctx.hot = Some(clear_id);
        }
        if !query.is_empty() {
            let (font, draw) = crate::widgets::draw_parts(ctx);
            draw.add_text(
                font,
                face,
                [clear_rect[0] + 6.0, rect[1] + style.frame_padding[1]],
                "×",
                search_col,
            );
        }
        if clear {
            query.clear();
        }
        ctx.id_stack.pop();
        crate::widgets::register_item(ctx, id, rect);
        changed || clear
    }

    pub fn progress_bar(&mut self, label: &str, fraction: f32, size: [f32; 2]) {
        crate::widgets::progress::progress_bar(self, label, fraction, size)
    }

    pub fn spinner(&mut self, label: &str, diameter: f32, phase: f32) {
        crate::widgets::spinner::spinner(self, label, diameter, phase)
    }

    pub fn slider_float(
        &mut self,
        label: &str,
        value: &mut f32,
        range: core::ops::RangeInclusive<f32>,
    ) -> bool {
        crate::widgets::slider::slider_float(self, label, value, range)
    }

    pub fn filled_slider_float(
        &mut self,
        label: &str,
        value: &mut f32,
        range: core::ops::RangeInclusive<f32>,
    ) -> bool {
        crate::widgets::slider::filled_slider_float(self, label, value, range)
    }

    pub fn slider_int(
        &mut self,
        label: &str,
        value: &mut i32,
        range: core::ops::RangeInclusive<i32>,
    ) -> bool {
        crate::widgets::slider::slider_int(self, label, value, range)
    }

    pub fn filled_slider_int(
        &mut self,
        label: &str,
        value: &mut i32,
        range: core::ops::RangeInclusive<i32>,
    ) -> bool {
        crate::widgets::slider::filled_slider_int(self, label, value, range)
    }

    pub fn range_slider_float(
        &mut self,
        label: &str,
        lower: &mut f32,
        upper: &mut f32,
        range: core::ops::RangeInclusive<f32>,
    ) -> bool {
        crate::widgets::range_slider::range_slider_float(self, label, lower, upper, range)
    }

    pub fn range_slider_int(
        &mut self,
        label: &str,
        lower: &mut i32,
        upper: &mut i32,
        range: core::ops::RangeInclusive<i32>,
    ) -> bool {
        crate::widgets::range_slider::range_slider_int(self, label, lower, upper, range)
    }

    pub fn drag_float(
        &mut self,
        label: &str,
        value: &mut f32,
        speed: f32,
        range: core::ops::RangeInclusive<f32>,
    ) -> bool {
        crate::widgets::drag_value::drag_float(self, label, value, speed, range)
    }

    pub fn drag_int(
        &mut self,
        label: &str,
        value: &mut i32,
        speed: f32,
        range: core::ops::RangeInclusive<i32>,
    ) -> bool {
        crate::widgets::drag_value::drag_int(self, label, value, speed, range)
    }

    pub fn text_input(&mut self, label: &str, buf: &mut alloc::string::String) -> bool {
        crate::widgets::text_input::text_input(self, label, buf)
    }

    pub fn tab_bar(&mut self, label: &str, f: impl FnOnce(&mut TabBar<'_>)) {
        self.begin_tab_bar(label);
        f(&mut TabBar {
            ctx: &mut *self.ctx,
        });
        self.end_tab_bar();
    }

    pub fn begin_tab_bar(&mut self, label: &str) {
        let ctx = &mut *self.ctx;
        let id = crate::widgets::widget_id(ctx, label);
        let active = match ctx.widget_states.get(&id) {
            Some(WidgetState::TabBar { active }) => *active,
            _ => 0,
        };
        let style = ctx.style;
        let header_h = ctx.font.line_height_in(ctx.active_font()) + 2.0 * style.frame_padding[1];
        let width = crate::widgets::available_width(ctx);
        let row = crate::widgets::item_rect(ctx, [width, header_h]);
        ctx.id_stack.push_id(id);
        if style.border_size > 0.0 {
            let (_, draw_list) = crate::widgets::draw_parts(ctx);
            draw_list.add_filled_rect(
                [row[0], row[3] - style.border_size],
                [row[2], row[3]],
                crate::widgets::lighten(style.widget_bg, crate::widgets::BORDER_LIGHTEN),
            );
        }
        ctx.tab_bar_scopes.push(TabBarScopeState {
            id,
            row,
            pen_x: row[0],
            active,
            clicked: None,
            count: 0,
        });
    }

    pub fn tab(&mut self, title: &str) -> bool {
        tab_header(self.ctx, title)
    }

    pub fn end_tab_bar(&mut self) {
        let ctx = &mut *self.ctx;
        let state = ctx.tab_bar_scopes.pop().expect(
            "Ui::end_tab_bar called with no tab bar open: every end_tab_bar \
             must follow a Ui::begin_tab_bar",
        );
        ctx.id_stack.pop();
        if state.count > 0 {
            let active = state.clicked.unwrap_or(state.active).min(state.count - 1);
            ctx.widget_states
                .insert(state.id, WidgetState::TabBar { active });
        }
    }

    pub fn tree_node(&mut self, label: &str, f: impl FnOnce(&mut Ui)) {
        if let Some(scope) = self.begin_tree_node(label) {
            f(self);
            self.end_tree_node(scope);
        }
    }

    pub fn begin_tree_node(&mut self, label: &str) -> Option<TreeNodeScope> {
        let ctx = &mut *self.ctx;
        let id = crate::widgets::widget_id(ctx, label);
        let display = split_label(label).0;
        let open = match ctx.widget_states.get(&id) {
            Some(WidgetState::TreeNode { open }) => *open,
            _ => false,
        };
        let style = ctx.style;
        let face = ctx.active_font();
        let line_height = ctx.font.line_height_in(face);
        let arrow = line_height * 0.5;
        let text_size = ctx.font.measure_in(face, display);
        let row_height = line_height + 2.0 * style.frame_padding[1];
        let row_width = 2.0 * style.frame_padding[0] + arrow + text_size[0];
        let rect = crate::widgets::interactive_rect(ctx, [row_width, row_height]);
        if ctx.accessibility_enabled() {
            crate::widgets::register_focusable_item(ctx, id, rect);
        } else {
            crate::widgets::register_item(ctx, id, rect);
        }
        ctx.id_stack.semantics.describe(
            id,
            crate::accessibility::AccessibilityRole::TreeItem,
            display,
            if open {
                crate::accessibility::AccessibilityState::EXPANDED
            } else {
                0
            },
            3,
        );

        let inside = point_in_rect(ctx.input.mouse_pos, ctx.target_rect(rect));
        let down = ctx.input.mouse_down[0];
        if inside && hover_layer_gate(ctx) && down && !ctx.prev_mouse_down[0] {
            ctx.set_active(id);
        }
        let held = ctx.active == Some(id);
        let clicked =
            (held && !down && inside && hover_layer_gate(ctx)) || ctx.keyboard_activate == Some(id);
        if held && !down {
            ctx.clear_active();
        }
        let over = hovered(ctx, rect);
        if over {
            ctx.hot = Some(id);
        }
        let bg = if ctx.active == Some(id) && inside {
            Some(style.widget_bg_active)
        } else if over {
            Some(style.widget_bg_hovered)
        } else {
            None
        };
        let ax = rect[0] + style.frame_padding[0];
        let ay = rect[1] + (row_height - arrow) / 2.0;
        let amount = ctx.motion_tween(
            id,
            crate::animation::COLLAPSE,
            0.0,
            if open { 1.0 } else { 0.0 },
            ctx.motion.collapse,
            ctx.motion.easing,
        );
        let tri = crate::animation::arrow_triangle(ax, ay, arrow, amount);
        let text_pos = [
            ax + arrow + style.frame_padding[0],
            rect[1] + (row_height - text_size[1]) / 2.0,
        ];
        let bg = if ctx.motion.active() {
            let mut idle = style.widget_bg_hovered;
            idle[3] = 0;
            Some(ctx.widget_color(id, bg.unwrap_or(idle)))
        } else {
            bg
        };
        let (font, draw_list) = crate::widgets::draw_parts(ctx);
        if let Some(bg) = bg {
            draw_list.add_filled_rect([rect[0], rect[1]], [rect[2], rect[3]], bg);
        }
        draw_list.add_filled_polygon(&tri, style.text, style.anti_aliasing);
        draw_list.add_text(font, face, text_pos, display, style.text);

        let indent = 2.0 * style.frame_padding[0] + arrow;
        if open || amount > 0.0 {
            ctx.id_stack.push_id(id);
            if ctx.popup_scope_active {
                ctx.overlay_cursor.indent(indent);
            } else {
                ctx.current_window_mut().cursor.indent(indent);
            }
            let motion = self.begin_collapse_motion(id, amount, !open || clicked);
            Some(TreeNodeScope {
                id,
                open,
                clicked,
                indent,
                motion,
            })
        } else {
            ctx.widget_states
                .insert(id, WidgetState::TreeNode { open: clicked });
            None
        }
    }

    pub fn end_tree_node(&mut self, scope: TreeNodeScope) {
        self.end_collapse_motion(scope.motion);
        let ctx = &mut *self.ctx;
        if ctx.popup_scope_active {
            ctx.overlay_cursor.unindent(scope.indent);
        } else {
            ctx.current_window_mut().cursor.unindent(scope.indent);
        }
        ctx.id_stack.pop();
        ctx.widget_states.insert(
            scope.id,
            WidgetState::TreeNode {
                open: scope.open ^ scope.clicked,
            },
        );
    }

    pub fn collapsing_header(&mut self, title: &str, f: impl FnOnce(&mut Ui)) {
        if let Some(scope) = self.begin_collapsing_header(title) {
            f(self);
            self.end_collapsing_header(scope);
        }
    }

    pub fn begin_collapsing_header(&mut self, title: &str) -> Option<CollapsingHeaderScope> {
        let ctx = &mut *self.ctx;
        let id = crate::widgets::widget_id(ctx, title);
        let display = split_label(title).0;
        let open = match ctx.widget_states.get(&id) {
            Some(WidgetState::CollapsingHeader { open }) => *open,
            _ => false,
        };
        let style = ctx.style;
        let face = ctx.active_font();
        let line_height = ctx.font.line_height_in(face);
        let header_h = line_height + 2.0 * style.frame_padding[1];
        let width = crate::widgets::available_width(ctx);
        let rect = crate::widgets::interactive_rect(ctx, [width, header_h]);
        if ctx.accessibility_enabled() {
            crate::widgets::register_focusable_item(ctx, id, rect);
        } else {
            crate::widgets::register_item(ctx, id, rect);
        }
        ctx.id_stack.semantics.describe(
            id,
            crate::accessibility::AccessibilityRole::TreeItem,
            display,
            if open {
                crate::accessibility::AccessibilityState::EXPANDED
            } else {
                0
            },
            3,
        );

        let mouse = ctx.input.mouse_pos;
        let inside = point_in_rect(mouse, ctx.target_rect(rect));
        let left_down = ctx.input.mouse_down[0];
        let left_pressed = left_down && !ctx.prev_mouse_down[0];
        if inside && left_pressed {
            ctx.set_active(id);
        }
        let held = ctx.active == Some(id);
        let mut clicked = ctx.keyboard_activate == Some(id);
        if held && !left_down {
            if inside {
                clicked = true;
            }
            ctx.clear_active();
        }
        let hovered = hovered(ctx, rect);
        if hovered {
            ctx.hot = Some(id);
        }
        let active = ctx.active == Some(id);
        let bg = if active && inside {
            style.widget_bg_active
        } else if hovered {
            style.widget_bg_hovered
        } else {
            style.widget_bg
        };

        let arrow = line_height * 0.5;
        let ax = rect[0] + style.frame_padding[0];
        let ay = rect[1] + (header_h - arrow) / 2.0;
        let amount = ctx.motion_tween(
            id,
            crate::animation::COLLAPSE,
            0.0,
            if open { 1.0 } else { 0.0 },
            ctx.motion.collapse,
            ctx.motion.easing,
        );
        let tri = crate::animation::arrow_triangle(ax, ay, arrow, amount);
        let text_size = ctx.font.measure_in(face, display);
        let text_pos = [
            rect[0] + 2.0 * style.frame_padding[0] + arrow,
            rect[1] + (header_h - text_size[1]) / 2.0,
        ];
        let bg = ctx.widget_color(id, bg);
        {
            let (font, draw_list) = crate::widgets::draw_parts(ctx);
            draw_list.add_filled_rect([rect[0], rect[1]], [rect[2], rect[3]], bg);
            if style.border_size > 0.0 {
                crate::widgets::draw_border(
                    font,
                    draw_list,
                    0.0,
                    rect,
                    style.border_size,
                    crate::widgets::lighten(bg, crate::widgets::BORDER_LIGHTEN),
                );
            }
            draw_list.add_filled_polygon(&tri, style.text, style.anti_aliasing);
            draw_list.add_text(font, face, text_pos, display, style.text);
        }

        let indent = 2.0 * style.frame_padding[0] + arrow;
        if open || amount > 0.0 {
            ctx.id_stack.push_id(id);
            if ctx.popup_scope_active {
                ctx.overlay_cursor.indent(indent);
            } else {
                ctx.current_window_mut().cursor.indent(indent);
            }
            let motion = self.begin_collapse_motion(id, amount, !open || clicked);
            Some(CollapsingHeaderScope {
                id,
                open,
                clicked,
                indent,
                motion,
            })
        } else {
            ctx.widget_states.insert(
                id,
                WidgetState::CollapsingHeader {
                    open: open ^ clicked,
                },
            );
            None
        }
    }

    pub fn end_collapsing_header(&mut self, scope: CollapsingHeaderScope) {
        self.end_collapse_motion(scope.motion);
        let ctx = &mut *self.ctx;
        if ctx.popup_scope_active {
            ctx.overlay_cursor.unindent(scope.indent);
        } else {
            ctx.current_window_mut().cursor.unindent(scope.indent);
        }
        ctx.id_stack.pop();
        ctx.widget_states.insert(
            scope.id,
            WidgetState::CollapsingHeader {
                open: scope.open ^ scope.clicked,
            },
        );
    }

    pub fn child(
        &mut self,
        label: &str,
        size: [f32; 2],
        flags: ScrollFlags,
        f: impl FnOnce(&mut Ui),
    ) {
        let scope = self.begin_child(label, size, flags);
        f(self);
        self.end_child(scope);
    }

    pub fn group_box(&mut self, label: &str, size: [f32; 2], f: impl FnOnce(&mut Ui)) {
        let scope = self.begin_group_box(label, size);
        f(self);
        self.end_group_box(scope);
    }

    pub fn begin_group_box(&mut self, label: &str, size: [f32; 2]) -> ChildScope {
        let scope = self.begin_child(label, size, ScrollFlags::NONE);
        self.label(label);
        self.separator();
        scope
    }

    pub fn end_group_box(&mut self, scope: ChildScope) {
        self.end_child(scope);
    }

    pub fn preview_canvas(
        &mut self,
        label: &str,
        size: [f32; 2],
        f: impl FnOnce(&mut Ui, [f32; 4]),
    ) {
        let (scope, rect) = self.begin_preview_canvas(label, size);
        f(self, rect);
        self.end_preview_canvas(scope);
    }

    pub fn begin_preview_canvas(&mut self, label: &str, size: [f32; 2]) -> (ChildScope, [f32; 4]) {
        let scope = self.begin_child(label, size, ScrollFlags::NONE);
        let rect = scope.content_rect;
        let col = self.ctx.style.window_bg;
        self.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], col);
        (scope, rect)
    }

    pub fn end_preview_canvas(&mut self, scope: ChildScope) {
        self.end_child(scope);
    }

    pub fn begin_child(&mut self, label: &str, size: [f32; 2], flags: ScrollFlags) -> ChildScope {
        let ctx = &mut *self.ctx;
        let id = crate::widgets::widget_id(ctx, label);
        let style = ctx.style;
        let rect = crate::widgets::item_rect(ctx, size);
        crate::widgets::register_item(ctx, id, rect);

        let (prev_offset, content_height) = match ctx.widget_states.get(&id) {
            Some(WidgetState::Scroll {
                offset,
                content_height,
            }) => (*offset, *content_height),
            _ => (0.0, 0.0),
        };
        let inset = crate::scroll::content_inset(&style, rect);
        let geometry = crate::scroll::geometry(&style, rect, inset, flags, content_height);
        let content_rect = geometry.content_rect;
        let scrollbar_rect = geometry.scrollbar_rect;
        let max_offset = geometry.max_offset;

        if flags.scrollable {
            ctx.scroll_containers.push(ScrollContainerEntry {
                id,
                target: crate::scroll::ScrollTarget::Widget,
                rect,
                layer_owns_point: hover_layer_gate(ctx),
                max_offset,
                line_height: ctx.font.line_height_in(ctx.active_font()),
            });
        }
        let offset = if flags.scrollable {
            ctx.scroll_position(id, prev_offset.clamp(0.0, max_offset), max_offset)
        } else {
            0.0
        };
        let mouse = ctx.input.mouse_pos;

        {
            let (font, draw_list) = crate::widgets::draw_parts(ctx);
            crate::widgets::chrome_fill(
                font,
                draw_list,
                style.corner_rounding,
                rect,
                style.widget_bg,
            );
            if style.border_size > 0.0 {
                crate::widgets::draw_border(
                    font,
                    draw_list,
                    style.corner_rounding,
                    rect,
                    style.border_size,
                    crate::widgets::lighten(style.widget_bg, crate::widgets::BORDER_LIGHTEN),
                );
            }
            draw_list.push_clip_rect(content_rect);
        }

        ctx.id_stack.push_id(id);
        let saved_clip = ctx.interaction_clip;
        let clip = match saved_clip {
            Some(outer) => crate::draw::intersect_rects(outer, content_rect),
            None => content_rect,
        };
        ctx.interaction_clip = Some(clip);
        let popup_scope = ctx.popup_scope_active;
        let saved_cursor = if popup_scope {
            ctx.overlay_cursor
        } else {
            ctx.current_window_mut().cursor
        };
        let content_origin = [content_rect[0], content_rect[1] - offset];
        let content_width = content_rect[2] - content_rect[0];
        if popup_scope {
            ctx.overlay_cursor
                .enter_region(content_origin, content_width);
        } else {
            ctx.current_window_mut()
                .cursor
                .enter_region(content_origin, content_width);
        }
        let saved_prev_left = ctx.prev_mouse_down[0];
        if !point_in_rect(mouse, clip) {
            ctx.prev_mouse_down[0] = true;
        }
        ChildScope {
            id,
            offset,
            prev_offset,
            content_origin_y: content_origin[1],
            item_spacing_y: style.item_spacing[1],
            popup_scope,
            saved_cursor,
            saved_prev_left,
            saved_clip,
            flags,
            content_rect,
            scrollbar_rect,
        }
    }

    pub fn end_child(&mut self, scope: ChildScope) {
        let ctx = &mut *self.ctx;
        let end_y = if scope.popup_scope {
            ctx.overlay_cursor.pos_y()
        } else {
            ctx.current_window_mut().cursor.pos_y()
        };
        let content_height = (end_y - scope.content_origin_y - scope.item_spacing_y).max(0.0);
        if scope.popup_scope {
            ctx.overlay_cursor = scope.saved_cursor;
            ctx.overlay.pop_clip_rect();
        } else {
            ctx.current_window_mut().cursor = scope.saved_cursor;
            ctx.current_window_mut().draw_list.pop_clip_rect();
        }
        ctx.prev_mouse_down[0] = scope.saved_prev_left;
        ctx.interaction_clip = scope.saved_clip;
        let offset = match scope.scrollbar_rect {
            Some(track) if scope.flags.scrollable && scope.flags.scrollbar => {
                let bar_id = ctx.id_stack.id(crate::scroll::SCROLLBAR_TAG);
                let visible_height = (scope.content_rect[3] - scope.content_rect[1]).max(0.0);
                let offset = crate::scroll::scrollbar(
                    ctx,
                    bar_id,
                    track,
                    scope.offset,
                    (content_height - visible_height).max(0.0),
                    visible_height,
                    content_height,
                );
                if offset != scope.offset
                    || ctx.active == Some(bar_id)
                    || (ctx.hot == Some(bar_id)
                        && ctx.input.mouse_down[0]
                        && !ctx.prev_mouse_down[0])
                {
                    ctx.scroll_snap(scope.id, offset);
                }
                offset
            }
            _ => scope.offset,
        };
        ctx.id_stack.pop();
        let persisted_offset = if scope.flags.scrollable {
            offset
        } else {
            scope.prev_offset
        };
        ctx.widget_states.insert(
            scope.id,
            WidgetState::Scroll {
                offset: persisted_offset,
                content_height,
            },
        );
    }

    pub fn scroll_region(&mut self, label: &str, size: [f32; 2], f: impl FnOnce(&mut Ui)) {
        self.child(label, size, ScrollFlags::WHEEL, f)
    }

    pub fn begin_scroll_region(&mut self, label: &str, size: [f32; 2]) -> ScrollRegionScope {
        self.begin_child(label, size, ScrollFlags::WHEEL)
    }

    pub fn end_scroll_region(&mut self, scope: ScrollRegionScope) {
        self.end_child(scope)
    }

    pub fn combo(&mut self, label: &str, items: &[&str], selected: &mut usize) -> bool {
        let id = crate::widgets::widget_id(self.ctx, label);
        if items.is_empty() && self.is_popup_open(id) {
            self.close_popup();
        }
        let ctx = &mut *self.ctx;
        let display = split_label(label).0;
        let style = ctx.style;
        let face = ctx.active_font();
        let line_height = ctx.font.line_height_in(face);
        let arrow = line_height * 0.5;
        let row_h = line_height + 2.0 * style.frame_padding[1];
        let current = items.get(*selected).copied().unwrap_or("");
        let current_size = ctx.font.measure_in(face, current);
        let label_size = if display.is_empty() {
            [0.0; 2]
        } else {
            ctx.font.measure_in(face, display)
        };
        let button_w = current_size[0] + arrow + 3.0 * style.frame_padding[0];
        let item_w = if display.is_empty() {
            button_w
        } else {
            button_w + style.item_spacing[0] + label_size[0]
        };
        let rect = crate::widgets::interactive_rect(ctx, [item_w, row_h]);
        let button = [rect[0], rect[1], rect[0] + button_w, rect[3]];
        if ctx.accessibility_enabled() && !items.is_empty() {
            crate::widgets::register_focusable_item(ctx, id, button);
        } else {
            crate::widgets::register_item(ctx, id, button);
        }
        ctx.id_stack.semantics.describe(
            id,
            crate::accessibility::AccessibilityRole::ComboBox,
            current,
            if items.is_empty() {
                crate::accessibility::AccessibilityState::DISABLED
            } else {
                0
            },
            if items.is_empty() { 0 } else { 3 },
        );

        let mouse = ctx.input.mouse_pos;
        let left_down = ctx.input.mouse_down[0];
        let mut clicked = !items.is_empty() && ctx.keyboard_activate == Some(id);
        let mut hover = false;
        let mut active_inside = false;
        if !items.is_empty() {
            let inside = point_in_rect(mouse, ctx.target_rect(button));
            let left_pressed = left_down && !ctx.prev_mouse_down[0];
            if inside && left_pressed {
                ctx.set_active(id);
            }
            let held = ctx.active == Some(id);
            if held && !left_down {
                if inside {
                    clicked = true;
                }
                ctx.clear_active();
            }
            hover = hovered(ctx, button);
            if hover {
                ctx.hot = Some(id);
            }
            active_inside = ctx.active == Some(id) && inside;
        }

        let bg = if active_inside {
            style.widget_bg_active
        } else if hover {
            style.widget_bg_hovered
        } else {
            style.widget_bg
        };
        let bg = ctx.widget_color(id, bg);
        {
            let (font, draw_list) = crate::widgets::draw_parts(ctx);
            crate::widgets::chrome_fill(font, draw_list, style.corner_rounding, button, bg);
            if style.border_size > 0.0 {
                crate::widgets::draw_border(
                    font,
                    draw_list,
                    style.corner_rounding,
                    button,
                    style.border_size,
                    crate::widgets::lighten(bg, crate::widgets::BORDER_LIGHTEN),
                );
            }
            let ax = button[2] - style.frame_padding[0] - arrow;
            let ay = button[1] + (row_h - arrow) / 2.0;
            let tri = [[ax, ay], [ax + arrow, ay], [ax + arrow / 2.0, ay + arrow]];
            draw_list.add_filled_polygon(&tri, style.text, style.anti_aliasing);
            let text_pos = [
                button[0] + style.frame_padding[0],
                button[1] + (row_h - current_size[1]) / 2.0,
            ];
            draw_list.add_text(font, face, text_pos, current, style.text);
            if !display.is_empty() {
                let label_pos = [
                    button[2] + style.item_spacing[0],
                    button[1] + (row_h - label_size[1]) / 2.0,
                ];
                draw_list.add_text(font, face, label_pos, display, style.text);
            }
        }
        if clicked {
            self.open_popup(id);
        }

        let ctx = &mut *self.ctx;
        let mut widest = 0.0f32;
        for item in items {
            widest = widest.max(ctx.font.measure_in(face, item)[0]);
        }
        let rows_height = |rows: usize| -> f32 {
            let mut h = 2.0 * style.window_padding[1];
            if rows > 0 {
                h += rows as f32 * row_h + (rows as f32 - 1.0) * style.item_spacing[1];
            }
            h
        };
        let viewport = ctx.input.viewport_size;
        let clamped = viewport != [0.0, 0.0];
        let mut popup_w =
            button_w.max(widest + 2.0 * style.frame_padding[0] + 2.0 * style.window_padding[0]);
        let mut popup_h = rows_height(items.len());
        if clamped {
            popup_w = popup_w.min(viewport[0]);
            popup_h = popup_h.min((viewport[1] - button[3]).max(0.0));
        }
        let content_h = (popup_h - 2.0 * style.window_padding[1]).max(0.0);
        let scrollable = rows_height(items.len()) > popup_h;
        let mut origin = [button[0], button[3]];
        if clamped {
            origin[0] = origin[0].clamp(0.0, (viewport[0] - popup_w).max(0.0));
            origin[1] = origin[1].clamp(0.0, (viewport[1] - popup_h).max(0.0));
        }
        let popup_rect = [
            origin[0],
            origin[1],
            origin[0] + popup_w,
            origin[1] + popup_h,
        ];

        let mut picked = None;
        self.popup(id, popup_rect, |ui| {
            let content_w = crate::widgets::available_width(ui.ctx);
            let mut rows = |ui: &mut Ui| {
                let ctx = &mut *ui.ctx;
                let row_w = crate::widgets::available_width(ctx);
                for (i, item) in items.iter().enumerate() {
                    let row = crate::widgets::item_rect(ctx, [row_w, row_h]);
                    ctx.id_stack.push_id(Id(i as u64));
                    let row_id = ctx.id_stack.id("##combo_item");
                    ctx.id_stack.pop();

                    let mouse = ctx.input.mouse_pos;
                    let inside = point_in_rect(mouse, row);
                    let left_down = ctx.input.mouse_down[0];
                    let left_pressed = left_down && !ctx.prev_mouse_down[0];
                    if inside && left_pressed {
                        ctx.set_active(row_id);
                    }
                    let held = ctx.active == Some(row_id);
                    if held && !left_down {
                        if inside {
                            picked = Some(i);
                        }
                        ctx.clear_active();
                    }
                    let hover = hovered(ctx, row);
                    if hover {
                        ctx.hot = Some(row_id);
                    }
                    let bg = if held && inside {
                        Some(style.widget_bg_active)
                    } else if hover {
                        Some(style.widget_bg_hovered)
                    } else if i == *selected {
                        Some(style.widget_bg_active)
                    } else {
                        None
                    };

                    let text_size = ctx.font.measure_in(face, item);
                    let (font, draw_list) = crate::widgets::draw_parts(ctx);
                    if let Some(col) = bg {
                        draw_list.add_filled_rect([row[0], row[1]], [row[2], row[3]], col);
                    }
                    let text_pos = [
                        row[0] + style.frame_padding[0],
                        row[1] + (row_h - text_size[1]) / 2.0,
                    ];
                    draw_list.add_text(font, face, text_pos, item, style.text);
                }
            };
            if scrollable {
                ui.scroll_region("##combo_list", [content_w, content_h], rows);
            } else {
                rows(ui);
            }
        });
        if let Some(i) = picked {
            *selected = i;
            self.close_popup();
            return true;
        }
        false
    }

    pub fn list_box(
        &mut self,
        label: &str,
        items: &[&str],
        selected: &mut usize,
        size: [f32; 2],
    ) -> bool {
        self.list_box_with(
            label,
            items.len(),
            |index| Some(items[index]),
            selected,
            size,
        )
        .unwrap_or(false)
    }

    pub fn list_box_with<'item>(
        &mut self,
        label: &str,
        item_count: usize,
        mut item_at: impl FnMut(usize) -> Option<&'item str>,
        selected: &mut usize,
        size: [f32; 2],
    ) -> Option<bool> {
        let id = crate::widgets::widget_id(self.ctx, label);
        let display = split_label(label).0;
        if !display.is_empty() {
            self.label(display);
        }
        self.ctx.id_stack.push_id(id);
        let scope = self.begin_child("##list_panel", size, ScrollFlags::BAR);
        let panel_item = self.ctx.last_item;
        let style = self.ctx.style;
        let face = self.ctx.active_font();
        let row_h = self.ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
        let row_w = crate::widgets::available_width(self.ctx);
        let clip = self.ctx.interaction_clip.unwrap_or(scope.content_rect);
        let visible = crate::data::VisibleRows::in_rect(
            item_count,
            row_h,
            style.item_spacing[1],
            scope.content_origin_y,
            clip,
        )
        .unwrap_or_default();
        let (first, end) = (visible.start, visible.end);
        for index in first..end {
            if item_at(index).is_none() {
                if scope.popup_scope {
                    self.ctx
                        .overlay_cursor
                        .skip_rows(item_count, [row_w, row_h]);
                } else {
                    self.ctx
                        .current_window_mut()
                        .cursor
                        .skip_rows(item_count, [row_w, row_h]);
                }
                self.end_child(scope);
                self.ctx.last_item = panel_item;
                self.ctx.id_stack.pop();
                return None;
            }
        }
        let captured_index = self.ctx.active_list_index.filter(|&index| {
            self.ctx.id_stack.push_id(Id(index as u64));
            let row_id = self.ctx.id_stack.id("##list_row");
            self.ctx.id_stack.pop();
            self.ctx.active == Some(row_id)
        });
        let released = captured_index.is_some() && !self.ctx.input.mouse_down[0];
        if scope.popup_scope {
            self.ctx.overlay_cursor.skip_rows(first, [row_w, row_h]);
        } else {
            self.ctx
                .current_window_mut()
                .cursor
                .skip_rows(first, [row_w, row_h]);
        }
        let mut picked = None;
        let mut invalid_at = None;
        for index in first..end {
            let Some(item) = item_at(index) else {
                invalid_at = Some(index);
                break;
            };
            let ctx = &mut *self.ctx;
            let row = crate::widgets::item_rect(ctx, [row_w, row_h]);
            ctx.id_stack.push_id(Id(index as u64));
            let row_id = ctx.id_stack.id("##list_row");
            ctx.id_stack.pop();
            crate::widgets::register_item(ctx, row_id, row);

            let mouse = ctx.input.mouse_pos;
            let inside = point_in_rect(mouse, row)
                && ctx
                    .interaction_clip
                    .is_some_and(|clip| point_in_rect(mouse, clip));
            let left_down = ctx.input.mouse_down[0];
            if inside && left_down && !ctx.prev_mouse_down[0] {
                ctx.set_active(row_id);
                ctx.active_list_index = Some(index);
            }
            let held = ctx.active == Some(row_id);
            if released && captured_index == Some(index) && inside && *selected != index {
                picked = Some(index);
            }
            let hover = hovered(ctx, row);
            if hover {
                ctx.hot = Some(row_id);
            }
            let bg = if held && inside || *selected == index || picked == Some(index) {
                Some(style.widget_bg_active)
            } else if hover {
                Some(style.widget_bg_hovered)
            } else {
                None
            };
            let text_size = ctx.font.measure_in(face, item);
            let (font, draw_list) = crate::widgets::draw_parts(ctx);
            if let Some(bg) = bg {
                draw_list.add_filled_rect([row[0], row[1]], [row[2], row[3]], bg);
            }
            draw_list.add_text(
                font,
                face,
                [
                    row[0] + style.frame_padding[0],
                    row[1] + (row_h - text_size[1]) * 0.5,
                ],
                item,
                style.text,
            );
        }
        if released {
            self.ctx.clear_active();
        }
        let remaining = item_count - invalid_at.unwrap_or(end);
        if scope.popup_scope {
            self.ctx.overlay_cursor.skip_rows(remaining, [row_w, row_h]);
        } else {
            self.ctx
                .current_window_mut()
                .cursor
                .skip_rows(remaining, [row_w, row_h]);
        }
        self.end_child(scope);
        self.ctx.last_item = panel_item;
        self.ctx.id_stack.pop();
        if invalid_at.is_some() {
            return None;
        }
        if let Some(index) = picked {
            *selected = index;
            Some(true)
        } else {
            Some(false)
        }
    }

    pub fn multi_select_list_with<'item>(
        &mut self,
        label: &str,
        item_count: usize,
        mut item_at: impl FnMut(usize) -> Option<&'item str>,
        selected: &mut crate::data::Selection,
        size: [f32; 2],
    ) -> Option<bool> {
        if item_count != selected.len() {
            return None;
        }
        let id = crate::widgets::widget_id(self.ctx, label);
        let display = split_label(label).0;
        if !display.is_empty() {
            self.label(display);
        }
        self.ctx.id_stack.push_id(id);
        let scope = self.begin_child("##list_panel", size, ScrollFlags::BAR);
        let panel_item = self.ctx.last_item;
        let style = self.ctx.style;
        let face = self.ctx.active_font();
        let row_h = self.ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
        let row_w = crate::widgets::available_width(self.ctx);
        let clip = self.ctx.interaction_clip.unwrap_or(scope.content_rect);
        let visible = crate::data::VisibleRows::in_rect(
            item_count,
            row_h,
            style.item_spacing[1],
            scope.content_origin_y,
            clip,
        )
        .unwrap_or_default();
        let (first, end) = (visible.start, visible.end);
        for index in first..end {
            if item_at(index).is_none() {
                if scope.popup_scope {
                    self.ctx
                        .overlay_cursor
                        .skip_rows(item_count, [row_w, row_h]);
                } else {
                    self.ctx
                        .current_window_mut()
                        .cursor
                        .skip_rows(item_count, [row_w, row_h]);
                }
                self.end_child(scope);
                self.ctx.last_item = panel_item;
                self.ctx.id_stack.pop();
                return None;
            }
        }
        let captured_index = self.ctx.active_list_index.filter(|&index| {
            self.ctx.id_stack.push_id(Id(index as u64));
            let row_id = self.ctx.id_stack.id("##list_row");
            self.ctx.id_stack.pop();
            self.ctx.active == Some(row_id)
        });
        let released = captured_index.is_some() && !self.ctx.input.mouse_down[0];
        if scope.popup_scope {
            self.ctx.overlay_cursor.skip_rows(first, [row_w, row_h]);
        } else {
            self.ctx
                .current_window_mut()
                .cursor
                .skip_rows(first, [row_w, row_h]);
        }
        let mut picked = None;
        let mut invalid_at = None;
        for index in first..end {
            let Some(item) = item_at(index) else {
                invalid_at = Some(index);
                break;
            };
            let ctx = &mut *self.ctx;
            let row = crate::widgets::item_rect(ctx, [row_w, row_h]);
            ctx.id_stack.push_id(Id(index as u64));
            let row_id = ctx.id_stack.id("##list_row");
            ctx.id_stack.pop();
            crate::widgets::register_item(ctx, row_id, row);

            let mouse = ctx.input.mouse_pos;
            let inside = point_in_rect(mouse, row)
                && ctx
                    .interaction_clip
                    .is_some_and(|clip| point_in_rect(mouse, clip));
            let left_down = ctx.input.mouse_down[0];
            if inside
                && hover_layer_gate(ctx)
                && ctx.disabled_depth == 0
                && left_down
                && !ctx.prev_mouse_down[0]
            {
                ctx.set_active(row_id);
                ctx.active_list_index = Some(index);
            }
            let held = ctx.active == Some(row_id);
            if released
                && captured_index == Some(index)
                && inside
                && ctx.disabled_depth == 0
                && ctx.popup_input_allowed()
            {
                picked = Some(index);
            }
            let hover = hovered(ctx, row);
            if hover {
                ctx.hot = Some(row_id);
            }
            let bg = if held && inside || selected.is_selected(index) || picked == Some(index) {
                Some(style.widget_bg_active)
            } else if hover {
                Some(style.widget_bg_hovered)
            } else {
                None
            };
            let text_size = ctx.font.measure_in(face, item);
            let (font, draw_list) = crate::widgets::draw_parts(ctx);
            if let Some(bg) = bg {
                draw_list.add_filled_rect([row[0], row[1]], [row[2], row[3]], bg);
            }
            draw_list.add_text(
                font,
                face,
                [
                    row[0] + style.frame_padding[0],
                    row[1] + (row_h - text_size[1]) * 0.5,
                ],
                item,
                style.text,
            );
        }
        if released {
            self.ctx.clear_active();
        }
        let remaining = item_count - invalid_at.unwrap_or(end);
        if scope.popup_scope {
            self.ctx.overlay_cursor.skip_rows(remaining, [row_w, row_h]);
        } else {
            self.ctx
                .current_window_mut()
                .cursor
                .skip_rows(remaining, [row_w, row_h]);
        }
        self.end_child(scope);
        self.ctx.last_item = panel_item;
        self.ctx.id_stack.pop();
        if invalid_at.is_some() {
            return None;
        }
        if let Some(index) = picked {
            Some(selected.select(index, self.ctx.modifiers.ctrl, self.ctx.modifiers.shift))
        } else {
            Some(false)
        }
    }

    pub fn searchable_combo(
        &mut self,
        label: &str,
        items: &[&str],
        selected: &mut usize,
        query: &mut alloc::string::String,
    ) -> bool {
        self.searchable_combo_with(
            label,
            items.len(),
            |index| Some(items[index]),
            selected,
            query,
        )
        .unwrap_or(false)
    }

    pub fn searchable_combo_with<'item>(
        &mut self,
        label: &str,
        item_count: usize,
        mut item_at: impl FnMut(usize) -> Option<&'item str>,
        selected: &mut usize,
        query: &mut alloc::string::String,
    ) -> Option<bool> {
        let id = crate::widgets::widget_id(self.ctx, label);
        self.ctx.id_stack.push_id(id);
        let search_id = self.ctx.id_stack.id("##search");
        self.ctx.id_stack.pop();
        if item_count == 0 && self.is_popup_open(id) {
            self.close_popup();
        }
        if !self.is_popup_open(id) && self.ctx.active == Some(search_id) {
            self.ctx.clear_active();
        }

        let ctx = &mut *self.ctx;
        let display = split_label(label).0;
        let style = ctx.style;
        let face = ctx.active_font();
        let line_h = ctx.font.line_height_in(face);
        let row_h = line_h + 2.0 * style.frame_padding[1];
        let arrow = line_h * 0.5;
        let current = if *selected < item_count {
            item_at(*selected)?
        } else {
            ""
        };
        let current_size = ctx.font.measure_in(face, current);
        let label_size = ctx.font.measure_in(face, display);
        let available = crate::widgets::available_width(ctx).max(0.0);
        let button_w = (current_size[0] + arrow + 3.0 * style.frame_padding[0])
            .max(160.0)
            .min(available);
        let item_w = if display.is_empty() {
            button_w
        } else {
            button_w + style.item_spacing[0] + label_size[0]
        };
        let rect = crate::widgets::item_rect(ctx, [item_w, row_h]);
        let button = [rect[0], rect[1], rect[0] + button_w, rect[3]];
        crate::widgets::register_item(ctx, id, button);

        let inside = point_in_rect(ctx.input.mouse_pos, button);
        let left_down = ctx.input.mouse_down[0];
        let left_pressed = left_down && !ctx.prev_mouse_down[0];
        let mut clicked = false;
        if item_count > 0 && inside && left_pressed {
            ctx.set_active(id);
        }
        let held = ctx.active == Some(id);
        if held && !left_down {
            clicked = inside;
            ctx.clear_active();
        }
        let hover = item_count > 0 && hovered(ctx, button);
        if hover {
            ctx.hot = Some(id);
        }
        let bg = if held && inside {
            style.widget_bg_active
        } else if hover {
            style.widget_bg_hovered
        } else {
            style.widget_bg
        };
        let bg = ctx.widget_color(id, bg);
        {
            let (font, draw_list) = crate::widgets::draw_parts(ctx);
            crate::widgets::chrome_fill(font, draw_list, style.corner_rounding, button, bg);
            if style.border_size > 0.0 {
                crate::widgets::draw_border(
                    font,
                    draw_list,
                    style.corner_rounding,
                    button,
                    style.border_size,
                    crate::widgets::lighten(bg, crate::widgets::BORDER_LIGHTEN),
                );
            }
            let ax = button[2] - style.frame_padding[0] - arrow;
            let ay = button[1] + (row_h - arrow) * 0.5;
            draw_list.add_filled_polygon(
                &[[ax, ay], [ax + arrow, ay], [ax + arrow * 0.5, ay + arrow]],
                style.text,
                style.anti_aliasing,
            );
            draw_list.add_text(
                font,
                face,
                [
                    button[0] + style.frame_padding[0],
                    button[1] + (row_h - current_size[1]) * 0.5,
                ],
                current,
                style.text,
            );
            if !display.is_empty() {
                draw_list.add_text(
                    font,
                    face,
                    [
                        button[2] + style.item_spacing[0],
                        button[1] + (row_h - label_size[1]) * 0.5,
                    ],
                    display,
                    style.text,
                );
            }
        }
        let mut highlighted = match ctx.widget_states.get(&id) {
            Some(WidgetState::SearchCombo { highlighted }) => *highlighted,
            _ => 0,
        };
        if !clicked && !self.is_popup_open(id) {
            return Some(false);
        }

        let mut widest = 0.0f32;
        let mut matches = 0usize;
        let effective_query = if clicked { "" } else { query.as_str() };
        for index in 0..item_count {
            let item = item_at(index)?;
            widest = widest.max(self.ctx.font.measure_in(face, item)[0]);
            if search_matches(item, effective_query) {
                matches += 1;
            }
        }
        if clicked {
            query.clear();
            highlighted = 0;
            self.open_popup(id);
            self.ctx.set_active(search_id);
        }

        let ctx = &mut *self.ctx;
        let visible_rows = matches.clamp(1, 8);
        let search_h = row_h;
        let rows_h = visible_rows as f32 * row_h
            + visible_rows.saturating_sub(1) as f32 * style.item_spacing[1];
        let natural_h = 2.0 * style.window_padding[1] + search_h + style.item_spacing[1] + rows_h;
        let viewport = ctx.input.viewport_size;
        let clamped = viewport != [0.0, 0.0];
        let mut popup_w =
            button_w.max(widest + 2.0 * style.frame_padding[0] + 2.0 * style.window_padding[0]);
        let mut popup_h = natural_h;
        if clamped {
            popup_w = popup_w.min(viewport[0]);
            popup_h = popup_h.min((viewport[1] - button[3]).max(0.0));
        }
        let mut origin = [button[0], button[3]];
        if clamped {
            origin[0] = origin[0].clamp(0.0, (viewport[0] - popup_w).max(0.0));
            origin[1] = origin[1].clamp(0.0, (viewport[1] - popup_h).max(0.0));
        }
        let popup_rect = [
            origin[0],
            origin[1],
            origin[0] + popup_w,
            origin[1] + popup_h,
        ];
        let results_h =
            (popup_h - 2.0 * style.window_padding[1] - search_h - style.item_spacing[1]).max(0.0);
        let scrollable = matches > visible_rows || natural_h > popup_h;

        let mut picked = None;
        let mut invalid_item = false;
        self.popup(id, popup_rect, |ui| {
            let changed = ui.text_input("##search", query);
            let match_count = (0..item_count)
                .filter(|&index| match item_at(index) {
                    Some(item) => search_matches(item, query),
                    None => {
                        invalid_item = true;
                        false
                    }
                })
                .count();
            if invalid_item {
                return;
            }
            if changed {
                highlighted = 0;
            }
            highlighted = highlighted.min(match_count.saturating_sub(1));
            let mut keyboard_moved = false;
            for event in ui.ctx.input.keys.iter().copied() {
                match event {
                    KeyEvent::Press(Key::Down) if match_count > 0 => {
                        highlighted = (highlighted + 1).min(match_count - 1);
                        keyboard_moved = true;
                    }
                    KeyEvent::Press(Key::Up) => {
                        highlighted = highlighted.saturating_sub(1);
                        keyboard_moved = true;
                    }
                    KeyEvent::Press(Key::Enter) if match_count > 0 => {
                        picked = (0..item_count)
                            .filter(|&index| match item_at(index) {
                                Some(item) => search_matches(item, query),
                                None => {
                                    invalid_item = true;
                                    false
                                }
                            })
                            .nth(highlighted);
                    }
                    _ => {}
                }
            }
            let content_w = crate::widgets::available_width(ui.ctx);
            if scrollable && keyboard_moved {
                let scroll_id = ui.widget_id("##search_results");
                if let Some(WidgetState::Scroll { offset, .. }) =
                    ui.ctx.widget_states.get_mut(&scroll_id)
                {
                    let step = row_h + style.item_spacing[1];
                    let top = highlighted as f32 * step;
                    let bottom = top + row_h;
                    let view_h = (results_h - 4.0).max(row_h);
                    if top < *offset {
                        *offset = top;
                    } else if bottom > *offset + view_h {
                        *offset = bottom - view_h;
                    }
                }
            }
            let mut rows = |ui: &mut Ui| {
                let ctx = &mut *ui.ctx;
                let row_w = crate::widgets::available_width(ctx);
                let mut match_index = 0;
                for index in 0..item_count {
                    let Some(item) = item_at(index) else {
                        invalid_item = true;
                        break;
                    };
                    if !search_matches(item, query) {
                        continue;
                    }
                    let row = crate::widgets::item_rect(ctx, [row_w, row_h]);
                    ctx.id_stack.push_id(Id(index as u64));
                    let row_id = ctx.id_stack.id("##search_result");
                    ctx.id_stack.pop();
                    let clip = ctx.interaction_clip;
                    let visible = clip.is_none_or(|clip| row[3] > clip[1] && row[1] < clip[3]);
                    let left_down = ctx.input.mouse_down[0];
                    if !visible {
                        if ctx.active == Some(row_id) && !left_down {
                            ctx.clear_active();
                        }
                        match_index += 1;
                        continue;
                    }
                    let inside = point_in_rect(ctx.input.mouse_pos, row)
                        && clip.is_none_or(|clip| point_in_rect(ctx.input.mouse_pos, clip));
                    if inside && left_down && !ctx.prev_mouse_down[0] {
                        ctx.set_active(row_id);
                    }
                    let held = ctx.active == Some(row_id);
                    if held && !left_down {
                        if inside {
                            picked = Some(index);
                        }
                        ctx.clear_active();
                    }
                    let hover = hovered(ctx, row);
                    if hover {
                        ctx.hot = Some(row_id);
                    }
                    let bg = if held && inside || match_index == highlighted {
                        Some(style.widget_bg_active)
                    } else if hover {
                        Some(style.widget_bg_hovered)
                    } else {
                        None
                    };
                    let text_size = ctx.font.measure_in(face, item);
                    let (font, draw_list) = crate::widgets::draw_parts(ctx);
                    if let Some(bg) = bg {
                        draw_list.add_filled_rect([row[0], row[1]], [row[2], row[3]], bg);
                    }
                    draw_list.add_text(
                        font,
                        face,
                        [
                            row[0] + style.frame_padding[0],
                            row[1] + (row_h - text_size[1]) * 0.5,
                        ],
                        item,
                        style.text,
                    );
                    match_index += 1;
                }
                if match_index == 0 {
                    ui.label("No matches");
                }
            };
            if scrollable {
                ui.scroll_region("##search_results", [content_w, results_h], rows);
            } else {
                rows(ui);
            }
        });
        if invalid_item {
            return None;
        }
        self.ctx
            .widget_states
            .insert(id, WidgetState::SearchCombo { highlighted });
        if let Some(index) = picked {
            *selected = index;
            self.close_popup();
            if self.ctx.active == Some(search_id) {
                self.ctx.clear_active();
            }
            return Some(true);
        }
        Some(false)
    }

    pub fn color_edit_rgba(&mut self, label: &str, value: &mut [u8; 4]) -> bool {
        let (id, swatch, item_w, clicked) = self.color_swatch_button(label, value);
        if clicked {
            self.open_popup(id);
        }

        let ctx = &mut *self.ctx;
        let style = ctx.style;
        let row_h = ctx.font.line_height_in(ctx.active_font()) + 2.0 * style.frame_padding[1];
        let preview_h = COLOR_EDIT_PREVIEW_ROWS * row_h;
        let mut popup_w = item_w.max(COLOR_EDIT_POPUP_MIN_WIDTH);
        let mut popup_h =
            2.0 * style.window_padding[1] + 4.0 * row_h + preview_h + 4.0 * style.item_spacing[1];
        let mut origin = [swatch[0], swatch[3]];
        let viewport = ctx.input.viewport_size;
        if viewport != [0.0, 0.0] {
            popup_w = popup_w.min(viewport[0]);
            popup_h = popup_h.min(viewport[1]);
            origin[0] = origin[0].clamp(0.0, (viewport[0] - popup_w).max(0.0));
            origin[1] = origin[1].clamp(0.0, (viewport[1] - popup_h).max(0.0));
        }
        let popup_rect = [
            origin[0],
            origin[1],
            origin[0] + popup_w,
            origin[1] + popup_h,
        ];

        let mut changed = false;
        self.popup(id, popup_rect, |ui| {
            const CHANNELS: [&str; 4] = ["R", "G", "B", "A"];
            for (i, channel) in CHANNELS.iter().enumerate() {
                let mut v = value[i] as i32;
                if ui.slider_int(channel, &mut v, 0..=255) {
                    value[i] = v as u8;
                    changed = true;
                }
            }
            let ctx = &mut *ui.ctx;
            let width = crate::widgets::available_width(ctx);
            let preview = crate::widgets::item_rect(ctx, [width, preview_h]);
            let (_, draw_list) = crate::widgets::draw_parts(ctx);
            draw_checkerboard(draw_list, preview);
            draw_list.add_filled_rect([preview[0], preview[1]], [preview[2], preview[3]], *value);
        });
        changed
    }

    fn color_swatch_button(&mut self, label: &str, value: &[u8; 4]) -> (Id, [f32; 4], f32, bool) {
        let ctx = &mut *self.ctx;
        let id = crate::widgets::widget_id(ctx, label);
        let display = split_label(label).0;
        let style = ctx.style;
        let face = ctx.active_font();
        let row_h = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
        let label_size = if display.is_empty() {
            [0.0; 2]
        } else {
            ctx.font.measure_in(face, display)
        };
        let item_w = if display.is_empty() {
            row_h
        } else {
            row_h + style.item_spacing[0] + label_size[0]
        };
        let rect = crate::widgets::item_rect(ctx, [item_w, row_h]);
        let swatch = [rect[0], rect[1], rect[0] + row_h, rect[3]];
        crate::widgets::register_item(ctx, id, swatch);

        let mouse = ctx.input.mouse_pos;
        let left_down = ctx.input.mouse_down[0];
        let inside = point_in_rect(mouse, swatch);
        let left_pressed = left_down && !ctx.prev_mouse_down[0];
        if inside && left_pressed {
            ctx.set_active(id);
        }
        let held = ctx.active == Some(id);
        let mut clicked = false;
        if held && !left_down {
            if inside {
                clicked = true;
            }
            ctx.clear_active();
        }
        let hover = hovered(ctx, swatch);
        if hover {
            ctx.hot = Some(id);
        }
        let state_bg = if ctx.active == Some(id) && inside {
            style.widget_bg_active
        } else if hover {
            style.widget_bg_hovered
        } else {
            style.widget_bg
        };

        {
            let (font, draw_list) = crate::widgets::draw_parts(ctx);
            draw_checkerboard(draw_list, swatch);
            draw_list.add_filled_rect([swatch[0], swatch[1]], [swatch[2], swatch[3]], *value);
            if style.border_size > 0.0 {
                crate::widgets::draw_border(
                    font,
                    draw_list,
                    0.0,
                    swatch,
                    style.border_size,
                    crate::widgets::lighten(state_bg, crate::widgets::BORDER_LIGHTEN),
                );
            }
            if !display.is_empty() {
                let label_pos = [
                    swatch[2] + style.item_spacing[0],
                    rect[1] + (row_h - label_size[1]) / 2.0,
                ];
                draw_list.add_text(font, face, label_pos, display, style.text);
            }
        }
        (id, swatch, item_w, clicked)
    }

    pub fn color_picker_rgba(&mut self, label: &str, value: &mut [u8; 4]) -> bool {
        let (id, swatch, item_w, clicked) = self.color_swatch_button(label, value);
        if clicked {
            let (h, s, v) = crate::color::rgba_u8_to_hsv(*value);
            self.ctx.widget_states.insert(
                id,
                WidgetState::ColorPicker {
                    h,
                    s,
                    v,
                    hex: crate::color::format_rgba_hex(*value),
                    original: *value,
                },
            );
            self.open_popup(id);
        }

        let ctx = &mut *self.ctx;
        let style = ctx.style;
        let face = ctx.active_font();
        let row_h = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
        let preview_h = COLOR_EDIT_PREVIEW_ROWS * row_h;
        let sv_h = color_picker_column_height(row_h, style.item_spacing[1]);
        let sv_w = COLOR_PICKER_SV_WIDTH_RATIO * sv_h;
        let mut popup_w = item_w.max(
            sv_w + style.item_spacing[0]
                + COLOR_PICKER_HUE_STRIP_WIDTH
                + style.item_spacing[0]
                + COLOR_PICKER_COLUMN_WIDTH
                + 2.0 * style.window_padding[0],
        );
        let mut popup_h = 2.0 * style.window_padding[1] + sv_h;
        let mut origin = [swatch[0], swatch[3]];
        let viewport = ctx.input.viewport_size;
        if viewport != [0.0, 0.0] {
            popup_w = popup_w.min(viewport[0]);
            popup_h = popup_h.min(viewport[1]);
            origin[0] = origin[0].clamp(0.0, (viewport[0] - popup_w).max(0.0));
            origin[1] = origin[1].clamp(0.0, (viewport[1] - popup_h).max(0.0));
        }
        let popup_rect = [
            origin[0],
            origin[1],
            origin[0] + popup_w,
            origin[1] + popup_h,
        ];

        let mut changed = false;
        self.popup(id, popup_rect, |ui| {
            let hex_id = crate::widgets::widget_id(ui.ctx, "##hex");
            let hex_was_focused = ui.ctx.active == Some(hex_id);
            let (mut h, mut s, mut v, mut hex, original) = match ui.ctx.widget_states.get_mut(&id) {
                Some(WidgetState::ColorPicker {
                    h,
                    s,
                    v,
                    hex,
                    original,
                }) => (*h, *s, *v, core::mem::take(hex), *original),
                _ => {
                    let (h, s, v) = crate::color::rgba_u8_to_hsv(*value);
                    (h, s, v, crate::color::format_rgba_hex(*value), *value)
                }
            };
            let mut hsv_moved = false;

            let mut sv = [s, v];
            if ui.sv_field("##sv", [sv_w, sv_h], h, &mut sv) {
                s = sv[0];
                v = sv[1];
                hsv_moved = true;
            }
            ui.same_line();
            if ui.hue_strip("##hue", [COLOR_PICKER_HUE_STRIP_WIDTH, sv_h], &mut h) {
                hsv_moved = true;
            }

            let content_origin = [
                popup_rect[0] + style.window_padding[0],
                popup_rect[1] + style.window_padding[1],
            ];
            let column_x = content_origin[0]
                + sv_w
                + style.item_spacing[0]
                + COLOR_PICKER_HUE_STRIP_WIDTH
                + style.item_spacing[0];
            let narrow_w = (COLOR_PICKER_COLUMN_WIDTH - style.item_spacing[0]) / 2.0;
            let outer_cursor = ui.ctx.overlay_cursor;

            ui.ctx
                .overlay_cursor
                .enter_region([column_x, content_origin[1]], COLOR_PICKER_COLUMN_WIDTH);
            let preview = crate::widgets::item_rect(ui.ctx, [COLOR_PICKER_COLUMN_WIDTH, preview_h]);
            let slider_top = preview[3] + style.item_spacing[1];

            let h_w = ui.ctx.font.measure_in(face, "H")[0];
            let s_w = ui.ctx.font.measure_in(face, "S")[0];
            let b_w = ui.ctx.font.measure_in(face, "B")[0];
            let r_w = ui.ctx.font.measure_in(face, "R")[0];
            let g_w = ui.ctx.font.measure_in(face, "G")[0];
            let gutter = h_w.max(s_w).max(b_w).max(r_w).max(g_w);
            let narrow_track_w = narrow_w - gutter - style.item_spacing[0];
            let right_x = column_x + narrow_w + style.item_spacing[0];
            let row_step = row_h + style.item_spacing[1];

            ui.ctx.overlay_cursor.enter_region(
                [right_x, slider_top],
                narrow_track_w + h_w + style.item_spacing[0],
            );
            let mut hi = (h + 0.5) as i32;
            if ui.slider_int("H", &mut hi, 0..=360) {
                h = hi as f32;
                hsv_moved = true;
            }
            ui.ctx.overlay_cursor.enter_region(
                [right_x, slider_top + row_step],
                narrow_track_w + s_w + style.item_spacing[0],
            );
            let mut si = (s * 100.0 + 0.5) as i32;
            if ui.slider_int("S", &mut si, 0..=100) {
                s = si as f32 / 100.0;
                hsv_moved = true;
            }
            ui.ctx.overlay_cursor.enter_region(
                [right_x, slider_top + 2.0 * row_step],
                narrow_track_w + b_w + style.item_spacing[0],
            );
            let mut bi = (v * 100.0 + 0.5) as i32;
            if ui.slider_int("B##bright", &mut bi, 0..=100) {
                v = bi as f32 / 100.0;
                hsv_moved = true;
            }

            if hsv_moved {
                let new = crate::color::hsv_to_rgba_u8(h, s, v, value[3]);
                if new != *value {
                    *value = new;
                    changed = true;
                }
            }

            const CHANNELS: [&str; 4] = ["R", "G", "B##blue", "A"];
            let label_ws = [r_w, g_w, b_w, 0.0];
            let mut rgb_moved = false;
            for (i, channel) in CHANNELS.iter().enumerate() {
                let w = if i < 3 {
                    narrow_track_w + label_ws[i] + style.item_spacing[0]
                } else {
                    COLOR_PICKER_COLUMN_WIDTH
                };
                let y = slider_top + i as f32 * row_step;
                ui.ctx.overlay_cursor.enter_region([column_x, y], w);
                let mut ch = value[i] as i32;
                if ui.slider_int(channel, &mut ch, 0..=255) {
                    value[i] = ch as u8;
                    changed = true;
                    rgb_moved = true;
                }
            }
            if rgb_moved {
                let (h2, s2, v2) = crate::color::rgba_u8_to_hsv(*value);
                h = if h == 360.0 && h2 == 0.0 { 360.0 } else { h2 };
                s = s2;
                v = v2;
            }

            let hex_y = slider_top + 4.0 * (row_h + style.item_spacing[1]);
            ui.ctx
                .overlay_cursor
                .enter_region([column_x, hex_y], COLOR_PICKER_COLUMN_WIDTH);
            if !hex_was_focused {
                crate::color::format_rgba_hex_into(&mut hex, *value);
            }
            ui.text_input("##hex", &mut hex);
            if hex_was_focused && ui.ctx.active != Some(hex_id) {
                if let Some(rgba) = crate::color::parse_rgba_hex(&hex) {
                    if rgba != *value {
                        *value = rgba;
                        changed = true;
                        let (h2, s2, v2) = crate::color::rgba_u8_to_hsv(*value);
                        h = if h == 360.0 && h2 == 0.0 { 360.0 } else { h2 };
                        s = s2;
                        v = v2;
                    }
                }
                crate::color::format_rgba_hex_into(&mut hex, *value);
            }

            let mid_x = (preview[0] + preview[2]) / 2.0;
            let current = [preview[0], preview[1], mid_x, preview[3]];
            let original_rect = [mid_x, preview[1], preview[2], preview[3]];
            let (_, draw_list) = crate::widgets::draw_parts(ui.ctx);
            draw_checkerboard(draw_list, current);
            draw_checkerboard(draw_list, original_rect);
            draw_list.add_filled_rect([current[0], current[1]], [current[2], current[3]], *value);
            draw_list.add_filled_rect(
                [original_rect[0], original_rect[1]],
                [original_rect[2], original_rect[3]],
                original,
            );

            ui.ctx.overlay_cursor = outer_cursor;

            ui.ctx.widget_states.insert(
                id,
                WidgetState::ColorPicker {
                    h,
                    s,
                    v,
                    hex,
                    original,
                },
            );
        });
        changed
    }

    pub(crate) fn sv_field(
        &mut self,
        label: &str,
        size: [f32; 2],
        hue: f32,
        sv: &mut [f32; 2],
    ) -> bool {
        let ctx = &mut *self.ctx;
        let id = crate::widgets::widget_id(ctx, label);
        let style = ctx.style;
        let rect = crate::widgets::item_rect(ctx, size);
        crate::widgets::register_item(ctx, id, rect);

        let mouse = ctx.input.mouse_pos;
        let left_down = ctx.input.mouse_down[0];
        let left_pressed = left_down && !ctx.prev_mouse_down[0];
        if point_in_rect(mouse, rect) && left_pressed {
            ctx.set_active(id);
        }
        let held = ctx.active == Some(id);
        let mut changed = false;
        if held && left_down {
            let w = rect[2] - rect[0];
            let h = rect[3] - rect[1];
            let s = if w > 0.0 {
                ((mouse[0] - rect[0]) / w).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let v = if h > 0.0 {
                (1.0 - (mouse[1] - rect[1]) / h).clamp(0.0, 1.0)
            } else {
                0.0
            };
            if s != sv[0] || v != sv[1] {
                sv[0] = s;
                sv[1] = v;
                changed = true;
            }
        }
        if held && !left_down {
            ctx.clear_active();
        }
        if hovered(ctx, rect) {
            ctx.hot = Some(id);
        }

        let marker = [
            rect[0] + sv[0].clamp(0.0, 1.0) * (rect[2] - rect[0]),
            rect[1] + (1.0 - sv[1].clamp(0.0, 1.0)) * (rect[3] - rect[1]),
        ];
        {
            let (font, draw_list) = crate::widgets::draw_parts(ctx);
            draw_list.add_sv_gradient([rect[0], rect[1]], [rect[2], rect[3]], hue);
            if style.border_size > 0.0 {
                crate::widgets::draw_border(
                    font,
                    draw_list,
                    0.0,
                    rect,
                    style.border_size,
                    crate::widgets::lighten(style.widget_bg, crate::widgets::BORDER_LIGHTEN),
                );
            }
        }
        self.draw_circle_outline(marker, SV_MARKER_RADIUS + 1.0, DRAG_MARKER_EDGE, 1.0);
        self.draw_circle_outline(marker, SV_MARKER_RADIUS, DRAG_MARKER_CORE, 1.0);
        changed
    }

    pub(crate) fn hue_strip(&mut self, label: &str, size: [f32; 2], h: &mut f32) -> bool {
        let ctx = &mut *self.ctx;
        let id = crate::widgets::widget_id(ctx, label);
        let style = ctx.style;
        let rect = crate::widgets::item_rect(ctx, size);
        crate::widgets::register_item(ctx, id, rect);

        let mouse = ctx.input.mouse_pos;
        let left_down = ctx.input.mouse_down[0];
        let left_pressed = left_down && !ctx.prev_mouse_down[0];
        if point_in_rect(mouse, rect) && left_pressed {
            ctx.set_active(id);
        }
        let held = ctx.active == Some(id);
        let mut changed = false;
        if held && left_down {
            let height = rect[3] - rect[1];
            let t = if height > 0.0 {
                ((mouse[1] - rect[1]) / height).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let new_h = t * 360.0;
            if new_h != *h {
                *h = new_h;
                changed = true;
            }
        }
        if held && !left_down {
            ctx.clear_active();
        }
        if hovered(ctx, rect) {
            ctx.hot = Some(id);
        }

        let marker_y = rect[1] + (*h / 360.0).clamp(0.0, 1.0) * (rect[3] - rect[1]);
        {
            let (font, draw_list) = crate::widgets::draw_parts(ctx);
            draw_list.add_hue_strip([rect[0], rect[1]], [rect[2], rect[3]]);
            if style.border_size > 0.0 {
                crate::widgets::draw_border(
                    font,
                    draw_list,
                    0.0,
                    rect,
                    style.border_size,
                    crate::widgets::lighten(style.widget_bg, crate::widgets::BORDER_LIGHTEN),
                );
            }
        }
        self.draw_line(
            [rect[0], marker_y],
            [rect[2], marker_y],
            DRAG_MARKER_EDGE,
            3.0,
        );
        self.draw_line(
            [rect[0], marker_y],
            [rect[2], marker_y],
            DRAG_MARKER_CORE,
            1.0,
        );
        changed
    }

    pub fn keybind(&mut self, label: &str, binding: &mut KeyBinding) -> bool {
        let ctx = &mut *self.ctx;
        let id = crate::widgets::widget_id(ctx, label);
        let display = split_label(label).0;
        let style = ctx.style;
        let face = ctx.active_font();
        let row_h = ctx.font.line_height_in(face) + 2.0 * style.frame_padding[1];
        let armed = matches!(
            ctx.widget_states.get(&id),
            Some(WidgetState::Keybind { armed: true })
        );
        let mut shown_buf = crate::widgets::FmtBuf::<64>::new();
        use core::fmt::Write;
        if armed {
            shown_buf.write_str(KEYBIND_PROMPT)
        } else {
            write!(shown_buf, "{}", *binding)
        }
        .expect("keybind caption always fits 64 bytes");
        let shown = shown_buf.as_str();
        let shown_size = ctx.font.measure_in(face, shown);
        let label_size = if display.is_empty() {
            [0.0; 2]
        } else {
            ctx.font.measure_in(face, display)
        };
        let button_w = shown_size[0] + 2.0 * style.frame_padding[0];
        let item_w = if display.is_empty() {
            button_w
        } else {
            button_w + style.item_spacing[0] + label_size[0]
        };
        let rect = crate::widgets::item_rect(ctx, [item_w, row_h]);
        let button = [rect[0], rect[1], rect[0] + button_w, rect[3]];
        crate::widgets::register_item(ctx, id, button);

        let mouse = ctx.input.mouse_pos;
        let inside = point_in_rect(mouse, button);
        let left_down = ctx.input.mouse_down[0];
        let mut changed = false;
        let mut hover = false;

        if armed {
            let pressed = [
                left_down && !ctx.prev_mouse_down[0],
                ctx.input.mouse_down[1] && !ctx.prev_mouse_down[1],
                ctx.input.mouse_down[2] && !ctx.prev_mouse_down[2],
                ctx.input.mouse_down[3] && !ctx.prev_mouse_down[3],
                ctx.input.mouse_down[4] && !ctx.prev_mouse_down[4],
            ];
            enum Action {
                Unarm,
                Capture(KeyBinding),
            }
            let mut action = None;
            if pressed[0] && inside {
                action = Some(Action::Unarm);
            } else if let Some(i) = pressed.iter().position(|p| *p) {
                action = Some(Action::Capture(KeyBinding::Mouse(i as u8)));
            } else {
                for event in &ctx.input.keys {
                    match event {
                        KeyEvent::Press(Key::Escape) => {
                            action = Some(Action::Unarm);
                            break;
                        }
                        KeyEvent::Press(Key::Shift | Key::Ctrl | Key::Alt) => {}
                        KeyEvent::Press(key) => {
                            let modifiers = ctx.modifiers;
                            let captured = if modifiers.shift || modifiers.ctrl || modifiers.alt {
                                KeyBinding::Chord {
                                    modifiers,
                                    key: *key,
                                }
                            } else {
                                KeyBinding::Key(*key)
                            };
                            action = Some(Action::Capture(captured));
                            break;
                        }
                        _ => {}
                    }
                }
            }
            if let Some(action) = action {
                if let Action::Capture(captured) = action {
                    *binding = captured;
                    changed = true;
                }
                ctx.widget_states
                    .insert(id, WidgetState::Keybind { armed: false });
                ctx.clear_active();
            }
        } else {
            let left_pressed = left_down && !ctx.prev_mouse_down[0];
            if inside && left_pressed {
                ctx.set_active(id);
            }
            let held = ctx.active == Some(id);
            if held && !left_down {
                if inside {
                    ctx.widget_states
                        .insert(id, WidgetState::Keybind { armed: true });
                } else {
                    ctx.clear_active();
                }
            }
            hover = hovered(ctx, button);
            if hover {
                ctx.hot = Some(id);
            }
        }

        let bg = if armed || (ctx.active == Some(id) && inside) {
            style.widget_bg_active
        } else if hover {
            style.widget_bg_hovered
        } else {
            style.widget_bg
        };
        let bg = ctx.widget_color(id, bg);
        {
            let (font, draw_list) = crate::widgets::draw_parts(ctx);
            crate::widgets::chrome_fill(font, draw_list, style.corner_rounding, button, bg);
            if style.border_size > 0.0 {
                crate::widgets::draw_border(
                    font,
                    draw_list,
                    style.corner_rounding,
                    button,
                    style.border_size,
                    crate::widgets::lighten(bg, crate::widgets::BORDER_LIGHTEN),
                );
            }
            let text_pos = [
                button[0] + style.frame_padding[0],
                button[1] + (row_h - shown_size[1]) / 2.0,
            ];
            draw_list.add_text(font, face, text_pos, shown, style.text);
            if !display.is_empty() {
                let label_pos = [
                    button[2] + style.item_spacing[0],
                    button[1] + (row_h - label_size[1]) / 2.0,
                ];
                draw_list.add_text(font, face, label_pos, display, style.text);
            }
        }
        changed
    }

    pub fn tooltip_text(&mut self, text: &str) {
        let ctx = &mut *self.ctx;
        if ctx.current_window.is_none() && !ctx.popup_scope_active {
            return;
        }
        let Some((id, rect)) = ctx.last_item else {
            return;
        };
        let hover = hovered(ctx, rect);
        let dt = ctx.dt;
        let state_id = Id(id.0 ^ TOOLTIP_STATE_SALT);
        if !hover && !ctx.widget_states.contains_key(&state_id) {
            return;
        }
        let state = ctx
            .widget_states
            .entry(state_id)
            .or_insert(WidgetState::Tooltip { hover_time: 0.0 });
        let show = match state {
            WidgetState::Tooltip { hover_time } => {
                if hover {
                    *hover_time += dt;
                } else {
                    *hover_time = 0.0;
                }
                hover && *hover_time >= TOOLTIP_DELAY
            }
            _ => false,
        };
        if !show && hover {
            if let Some(WidgetState::Tooltip { hover_time }) = ctx.widget_states.get(&state_id) {
                ctx.request_redraw_in((TOOLTIP_DELAY - *hover_time).max(0.001));
            }
        }
        if !show {
            return;
        }

        let style = ctx.style;
        let face = ctx.active_font();
        let line_height = ctx.font.line_height_in(face);
        let mut widest = 0.0f32;
        let mut lines = 0usize;
        for line in text.split('\n') {
            widest = widest.max(ctx.font.measure_in(face, line)[0]);
            lines += 1;
        }
        let w = widest + 2.0 * style.window_padding[0];
        let h = lines as f32 * line_height + 2.0 * style.window_padding[1];
        let mouse = ctx.input.mouse_pos;
        let mut origin = [mouse[0] + TOOLTIP_OFFSET[0], mouse[1] + TOOLTIP_OFFSET[1]];
        let viewport = ctx.input.viewport_size;
        if viewport != [0.0, 0.0] {
            origin[0] = origin[0].clamp(0.0, (viewport[0] - w).max(0.0));
            origin[1] = origin[1].clamp(0.0, (viewport[1] - h).max(0.0));
        }
        let rect = [origin[0], origin[1], origin[0] + w, origin[1] + h];

        crate::widgets::chrome_fill(
            &mut ctx.font,
            &mut ctx.overlay,
            style.corner_rounding,
            rect,
            style.window_bg,
        );
        if style.border_size > 0.0 {
            crate::widgets::draw_border(
                &mut ctx.font,
                &mut ctx.overlay,
                style.corner_rounding,
                rect,
                style.border_size,
                crate::widgets::lighten(style.window_bg, crate::widgets::BORDER_LIGHTEN),
            );
        }
        let font = &mut ctx.font;
        let overlay = &mut ctx.overlay;
        for (i, line) in text.split('\n').enumerate() {
            let pos = [
                origin[0] + style.window_padding[0],
                origin[1] + style.window_padding[1] + i as f32 * line_height,
            ];
            overlay.add_text(font, face, pos, line, style.text);
        }
    }

    pub fn same_line(&mut self) {
        if self.ctx.popup_scope_active {
            self.ctx.overlay_cursor.same_line();
        } else {
            self.ctx.current_window_mut().cursor.same_line();
        }
    }

    pub fn open_popup(&mut self, id: Id) {
        self.ctx.push_popup(id, false);
    }

    pub fn close_popup(&mut self) {
        self.ctx.dismiss_popup();
    }

    pub fn popup_depth(&self) -> usize {
        self.ctx.popup_depth()
    }
    pub fn modal_depth(&self) -> usize {
        self.ctx.modal_depth()
    }

    pub fn is_popup_open(&self, id: Id) -> bool {
        self.ctx.routing.popups[..self.ctx.routing.count]
            .iter()
            .any(|p| p.id == Some(id))
    }

    pub fn open_modal(&mut self, id: Id) {
        self.ctx.push_popup(id, true);
    }

    pub fn toast(&mut self, text: &str, seconds: f32) {
        self.ctx.push_toast(text, seconds);
    }

    pub fn begin_context_menu(&mut self, id: Id, target: [f32; 4], size: [f32; 2]) -> bool {
        if self.ctx.input.mouse_down[1]
            && !self.ctx.prev_mouse_down[1]
            && hovered(self.ctx, target)
            && !self.ctx.popup_modal
        {
            let anchor = self.ctx.input.mouse_pos;
            self.open_popup(id);
            self.ctx.popup_anchor = anchor;
        }
        let anchor = self.ctx.popup_anchor;
        let viewport = self.ctx.input.viewport_size;
        let width = size[0].max(0.0);
        let height = size[1].max(0.0);
        let x = if viewport[0] > 0.0 {
            anchor[0].clamp(0.0, (viewport[0] - width).max(0.0))
        } else {
            anchor[0]
        };
        let y = if viewport[1] > 0.0 {
            anchor[1].clamp(0.0, (viewport[1] - height).max(0.0))
        } else {
            anchor[1]
        };
        self.begin_popup(id, [x, y, x + width, y + height])
    }

    pub fn begin_modal(&mut self, id: Id, rect: [f32; 4]) -> bool {
        if !self.ctx.routing.popups[..self.ctx.routing.count]
            .iter()
            .any(|p| p.id == Some(id) && p.modal)
        {
            return false;
        }
        self.begin_popup(id, rect)
    }

    pub fn popup(&mut self, id: Id, rect: [f32; 4], f: impl FnOnce(&mut Ui)) {
        if self.begin_popup(id, rect) {
            f(self);
            self.end_popup();
        }
    }

    pub fn modal(&mut self, id: Id, rect: [f32; 4], f: impl FnOnce(&mut Ui)) {
        if self.begin_modal(id, rect) {
            f(self);
            self.end_popup();
        }
    }

    pub fn context_menu(
        &mut self,
        id: Id,
        target: [f32; 4],
        size: [f32; 2],
        f: impl FnOnce(&mut Ui),
    ) {
        if self.begin_context_menu(id, target, size) {
            f(self);
            self.end_popup();
        }
    }

    pub fn begin_popup(&mut self, id: Id, rect: [f32; 4]) -> bool {
        let Some(index) = self.ctx.routing.popups[..self.ctx.routing.count]
            .iter()
            .position(|p| p.id == Some(id))
        else {
            return false;
        };
        assert!(
            rect.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                && rect[2] > rect[0]
                && rect[3] > rect[1],
            "invalid popup rectangle"
        );
        let ctx = &mut *self.ctx;
        assert!(
            ctx.routing.depth < crate::interaction::MAX_POPUPS,
            "popup scope limit"
        );
        assert!(
            !ctx.routing.scopes[..ctx.routing.depth]
                .iter()
                .any(|s| s.index == index),
            "recursive popup declaration"
        );
        let depth = ctx.routing.depth;
        let scope = &mut ctx.routing.scopes[depth];
        scope.id = id;
        scope.index = index;
        scope.cursor = ctx.overlay_cursor;
        scope.prev = ctx.prev_mouse_down;
        scope.mouse = ctx.input.mouse_pos;
        scope.down = ctx.input.mouse_down;
        scope.wheel = ctx.input.mouse_wheel;
        scope.clip = ctx.interaction_clip;
        scope.keys.clear();
        scope.keys.extend_from_slice(&ctx.input.keys);
        core::mem::swap(&mut ctx.overlay, &mut ctx.routing.popups[index].draw);
        core::mem::swap(
            &mut ctx.popup_animation,
            &mut ctx.routing.popups[index].motion,
        );
        ctx.routing.depth += 1;
        ctx.popup_scope_active = true;
        ctx.input.mouse_pos = ctx.routing.scopes[0].mouse;
        ctx.input.mouse_down = ctx.routing.scopes[0].down;
        ctx.input.mouse_wheel = ctx.routing.scopes[0].wheel;
        ctx.input.keys.clear();
        ctx.input
            .keys
            .extend_from_slice(&ctx.routing.scopes[0].keys);
        ctx.prev_mouse_down = ctx.routing.scopes[0].prev;
        if index + 1 < ctx.routing.count {
            ctx.input.mouse_pos = [-f32::MAX; 2];
            ctx.input.mouse_down = [false; 5];
            ctx.input.mouse_wheel = 0.;
            ctx.input.keys.clear();
            ctx.prev_mouse_down = [true; 5];
        } else if ctx.press_target == crate::context::PressTarget::Popup {
            ctx.prev_mouse_down[0] = false;
        }
        ctx.routing.popups[index].rect = Some(rect);
        ctx.routing.popups[index].seen = ctx.motion_frame;
        ctx.sync_popup_top();
        ctx.mark_popup_motion(id);
        if ctx.routing.popups[index].modal
            && ctx.input.viewport_size[0] > 0.
            && ctx.input.viewport_size[1] > 0.
        {
            ctx.overlay
                .add_filled_rect([0.; 2], ctx.input.viewport_size, [0, 0, 0, 150]);
        }
        let style = ctx.style;
        crate::widgets::chrome_fill(
            &mut ctx.font,
            &mut ctx.overlay,
            style.corner_rounding,
            rect,
            style.window_bg,
        );
        if style.border_size > 0.0 {
            crate::widgets::draw_border(
                &mut ctx.font,
                &mut ctx.overlay,
                style.corner_rounding,
                rect,
                style.border_size,
                crate::widgets::lighten(style.window_bg, crate::widgets::BORDER_LIGHTEN),
            );
        }
        ctx.overlay.push_clip_rect(rect);
        ctx.overlay_cursor = Cursor::new(
            [rect[0], rect[1]],
            rect[2] - rect[0],
            style.window_padding,
            style.item_spacing,
        );
        ctx.id_stack.push_id(id);
        ctx.interaction_clip = None;
        true
    }

    pub fn end_popup(&mut self) {
        let ctx = &mut *self.ctx;
        assert!(ctx.routing.depth > 0, "Ui::end_popup without a popup scope");
        ctx.finish_popup_motion();
        ctx.overlay.pop_clip_rect();
        ctx.id_stack.pop();
        ctx.routing.depth -= 1;
        let scope = &mut ctx.routing.scopes[ctx.routing.depth];
        ctx.interaction_clip = scope.clip;
        ctx.prev_mouse_down = scope.prev;
        ctx.input.mouse_pos = scope.mouse;
        ctx.input.mouse_down = scope.down;
        ctx.input.mouse_wheel = scope.wheel;
        ctx.input.keys.clear();
        ctx.input.keys.extend_from_slice(&scope.keys);
        ctx.overlay_cursor = scope.cursor;
        core::mem::swap(&mut ctx.overlay, &mut ctx.routing.popups[scope.index].draw);
        core::mem::swap(
            &mut ctx.popup_animation,
            &mut ctx.routing.popups[scope.index].motion,
        );
        ctx.popup_scope_active = ctx.routing.depth > 0;
        if ctx.routing.depth == 0 {
            for slot in &ctx.routing.popups[..ctx.routing.count] {
                ctx.overlay.append_faded(&slot.draw, 1.0);
            }
        }
        if let Some((id, modal, level)) = ctx.routing.pending_popup {
            if !ctx.routing.scopes[..ctx.routing.depth]
                .iter()
                .any(|scope| scope.index == level)
            {
                ctx.routing.pending_popup = None;
                ctx.push_popup(id, modal);
            }
        }
    }

    pub fn draw_line(&mut self, p0: [f32; 2], p1: [f32; 2], col: [u8; 4], thickness: f32) {
        if self.ctx.current_window.is_none() {
            return;
        }
        let aa = self.ctx.style.anti_aliasing;
        routed_draw_list(self.ctx).add_thick_line_with_caps(p0, p1, col, thickness, aa);
    }

    pub fn draw_rect_filled(&mut self, min: [f32; 2], max: [f32; 2], col: [u8; 4]) {
        let Some((min, max)) = normalize_rect(min, max) else {
            return;
        };
        if self.ctx.current_window.is_none() {
            return;
        }
        let aa = self.ctx.style.anti_aliasing;
        let draw_list = routed_draw_list(self.ctx);
        draw_list.add_filled_rect(min, max, col);
        if aa {
            draw_list.add_filled_rect_fringe(min, max, col);
        }
    }

    pub fn draw_rect_outline(
        &mut self,
        min: [f32; 2],
        max: [f32; 2],
        col: [u8; 4],
        thickness: f32,
    ) {
        if thickness <= 0.0 {
            return;
        }
        let Some((min, max)) = normalize_rect(min, max) else {
            return;
        };
        if self.ctx.current_window.is_none() {
            return;
        }
        let aa = self.ctx.style.anti_aliasing;
        routed_draw_list(self.ctx).add_rect_outline(
            [min[0], min[1], max[0], max[1]],
            thickness,
            col,
            aa,
        );
    }

    fn try_draw_mask_quad(
        &mut self,
        key: crate::font::ShapeKey,
        center: [f32; 2],
        col: [u8; 4],
    ) -> bool {
        let (font, draw_list) = crate::widgets::draw_parts(self.ctx);
        let Some(mask) = font.shape_mask(key) else {
            return false;
        };
        let w = mask.rect[2] as f32 / mask.raster_scale;
        let h = mask.rect[3] as f32 / mask.raster_scale;
        let min = [center[0] - w / 2.0, center[1] - h / 2.0];
        let max = [center[0] + w / 2.0, center[1] + h / 2.0];
        draw_list.add_textured_quad(TextureId::FONT, min, max, mask.uv_min, mask.uv_max, col);
        true
    }

    pub fn draw_circle_filled(&mut self, center: [f32; 2], radius: f32, col: [u8; 4]) {
        if self.ctx.current_window.is_none() {
            return;
        }
        let aa = self.ctx.style.anti_aliasing;
        if aa && self.try_draw_mask_quad(crate::font::ShapeKey::disk(radius), center, col) {
            return;
        }
        routed_draw_list(self.ctx).add_filled_circle(center, radius, col, aa);
    }

    pub fn draw_circle_outline(
        &mut self,
        center: [f32; 2],
        radius: f32,
        col: [u8; 4],
        thickness: f32,
    ) {
        if self.ctx.current_window.is_none() {
            return;
        }
        let aa = self.ctx.style.anti_aliasing;
        if aa
            && self.try_draw_mask_quad(crate::font::ShapeKey::ring(radius, thickness), center, col)
        {
            return;
        }
        routed_draw_list(self.ctx).add_circle_outline(center, radius, col, thickness, aa);
    }

    pub fn draw_arc(
        &mut self,
        center: [f32; 2],
        radius: f32,
        angle_range: core::ops::RangeInclusive<f32>,
        col: [u8; 4],
        thickness: f32,
    ) {
        let (mut start, mut end) = (*angle_range.start(), *angle_range.end());
        if start > end {
            core::mem::swap(&mut start, &mut end);
        }
        if self.ctx.current_window.is_none() {
            return;
        }
        let aa = self.ctx.style.anti_aliasing;
        routed_draw_list(self.ctx).add_arc(center, radius, start, end, col, thickness, aa);
    }

    pub fn draw_polygon_filled(&mut self, points: &[[f32; 2]], col: [u8; 4]) {
        if self.ctx.current_window.is_none() {
            return;
        }
        let aa = self.ctx.style.anti_aliasing;
        routed_draw_list(self.ctx).add_filled_polygon(points, col, aa);
    }

    pub fn draw_image(
        &mut self,
        texture: TextureId,
        min: [f32; 2],
        max: [f32; 2],
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        tint: [u8; 4],
    ) {
        let Some((min, max)) = normalize_rect(min, max) else {
            return;
        };
        if self.ctx.current_window.is_none() {
            return;
        }
        routed_draw_list(self.ctx).add_textured_quad(texture, min, max, uv_min, uv_max, tint);
    }

    pub fn widget_id(&self, label: &str) -> Id {
        crate::widgets::widget_id(self.ctx, label)
    }

    pub fn allocate(&mut self, size: [f32; 2]) -> [f32; 4] {
        crate::widgets::item_rect(self.ctx, size)
    }

    pub fn available_width(&mut self) -> f32 {
        self.ctx
            .current_window_mut()
            .cursor
            .available_width()
            .max(0.0)
    }

    pub fn is_hovered(&self, rect: [f32; 4]) -> bool {
        hovered(self.ctx, rect)
    }

    pub fn capture(&mut self, id: Id) {
        self.ctx.set_active(id);
    }

    pub fn release_capture(&mut self, id: Id) {
        if self.ctx.active == Some(id) {
            self.ctx.clear_active();
        }
    }

    pub fn is_captured(&self, id: Id) -> bool {
        self.ctx.active == Some(id)
    }

    pub fn focusable(&mut self, id: Id, rect: [f32; 4]) {
        if rect.iter().all(|v| v.is_finite()) && rect[2] > rect[0] && rect[3] > rect[1] {
            crate::widgets::register_focusable_item(self.ctx, id, rect);
        }
    }

    pub fn is_focused(&self, id: Id) -> bool {
        self.ctx.focused == Some(id)
    }

    pub fn focus(&mut self, id: Id) {
        if self.ctx.popup_input_allowed() {
            self.ctx.focused = Some(id);
            self.ctx.focused_in_modal = self.ctx.popup_modal;
            self.ctx.focused_in_popup = self.ctx.popup_scope_active;
        }
    }

    pub fn keyboard_activated(&self, id: Id) -> bool {
        self.ctx.keyboard_activate == Some(id) && self.ctx.popup_input_allowed()
    }

    pub fn draw_text_at(&mut self, pos: [f32; 2], text: &str, col: [u8; 4]) {
        if self.ctx.current_window.is_none() {
            return;
        }
        let face = self.ctx.active_font();
        let (font, draw_list) = crate::widgets::draw_parts(self.ctx);
        draw_list.add_text(font, face, pos, text, col);
    }

    pub fn measure_text(&mut self, text: &str) -> [f32; 2] {
        let face = self.ctx.active_font();
        self.ctx.font.measure_in(face, text)
    }

    pub fn mouse_pos(&self) -> [f32; 2] {
        self.ctx.input.mouse_pos
    }

    pub fn mouse_down(&self, button: usize) -> bool {
        self.ctx.input.mouse_down[button]
    }

    pub fn mouse_pressed(&self, button: usize) -> bool {
        self.ctx.input.mouse_down[button] && !self.ctx.prev_mouse_down[button]
    }

    pub fn mouse_released(&self, button: usize) -> bool {
        !self.ctx.input.mouse_down[button] && self.ctx.prev_mouse_down[button]
    }
}

pub struct CollapsingHeaderScope {
    motion: Option<crate::animation::CollapseMotion>,
    id: Id,
    open: bool,
    clicked: bool,
    indent: f32,
}

pub struct TreeNodeScope {
    motion: Option<crate::animation::CollapseMotion>,
    id: Id,
    open: bool,
    clicked: bool,
    indent: f32,
}

pub struct ChildScope {
    id: Id,
    offset: f32,
    prev_offset: f32,
    content_origin_y: f32,
    item_spacing_y: f32,
    popup_scope: bool,
    saved_cursor: Cursor,
    saved_prev_left: bool,
    saved_clip: Option<[f32; 4]>,
    flags: ScrollFlags,
    pub(crate) content_rect: [f32; 4],
    scrollbar_rect: Option<[f32; 4]>,
}

impl ChildScope {
    pub(crate) fn scroll_offset(&self) -> f32 {
        self.offset
    }
}

pub type ScrollRegionScope = ChildScope;

pub struct TabBar<'a> {
    ctx: &'a mut Context,
}

impl TabBar<'_> {
    pub fn tab(&mut self, title: &str, f: impl FnOnce(&mut Ui)) {
        if tab_header(self.ctx, title) {
            f(&mut Ui {
                ctx: &mut *self.ctx,
            });
        }
    }
}

fn tab_header(ctx: &mut Context, title: &str) -> bool {
    let (index, row, pen_x, bar_active) = {
        let state = ctx.tab_bar_scopes.last_mut().expect(
            "Ui::tab called with no tab bar open: every tab must follow \
             Ui::begin_tab_bar",
        );
        let index = state.count;
        state.count += 1;
        (index, state.row, state.pen_x, state.active)
    };
    let id = ctx.id_stack.id(title);
    let display = split_label(title).0;
    let style = ctx.style;
    let face = ctx.active_font();
    let text_size = ctx.font.measure_in(face, display);
    let width = text_size[0] + 2.0 * style.frame_padding[0];
    let rect = [pen_x, row[1], pen_x + width, row[3]];
    let top = ctx.tab_bar_scopes.len() - 1;
    ctx.tab_bar_scopes[top].pen_x += width + style.item_spacing[0];
    crate::widgets::register_item(ctx, id, rect);

    let mouse = ctx.input.mouse_pos;
    let inside = point_in_rect(mouse, ctx.target_rect(rect));
    let left_down = ctx.input.mouse_down[0];
    let left_pressed = left_down && !ctx.prev_mouse_down[0];
    if inside && left_pressed {
        ctx.set_active(id);
    }
    let held = ctx.active == Some(id);
    if held && !left_down {
        if inside {
            ctx.tab_bar_scopes[top].clicked = Some(index);
        }
        ctx.clear_active();
    }

    if ctx.keyboard_activate == Some(id) {
        ctx.tab_bar_scopes[top].clicked = Some(index);
    }
    let is_active = index == bar_active;
    ctx.id_stack.semantics.describe(
        id,
        crate::accessibility::AccessibilityRole::Tab,
        display,
        if is_active {
            crate::accessibility::AccessibilityState::SELECTED
        } else {
            0
        },
        2,
    );
    let hovered = hovered(ctx, rect);
    if hovered {
        ctx.hot = Some(id);
    }
    let bg = if is_active {
        style.widget_bg_active
    } else if hovered {
        style.widget_bg_hovered
    } else {
        style.widget_bg
    };
    let bg = ctx.widget_color(id, bg);

    {
        let (font, draw_list) = crate::widgets::draw_parts(ctx);
        draw_list.add_filled_rect([rect[0], rect[1]], [rect[2], rect[3]], bg);
        if style.border_size > 0.0 {
            let border_col = crate::widgets::lighten(bg, crate::widgets::BORDER_LIGHTEN);
            if is_active {
                let t = style.border_size;
                draw_list.add_filled_rect([rect[0], rect[1]], [rect[2], rect[1] + t], border_col);
                draw_list.add_filled_rect(
                    [rect[0], rect[1] + t],
                    [rect[0] + t, rect[3]],
                    border_col,
                );
                draw_list.add_filled_rect(
                    [rect[2] - t, rect[1] + t],
                    [rect[2], rect[3]],
                    border_col,
                );
            } else {
                crate::widgets::draw_border(
                    font,
                    draw_list,
                    0.0,
                    rect,
                    style.border_size,
                    border_col,
                );
            }
        }
        let text_pos = [
            rect[0] + (width - text_size[0]) / 2.0,
            rect[1] + (row[3] - row[1] - text_size[1]) / 2.0,
        ];
        draw_list.add_text(font, face, text_pos, display, style.text);
    }
    is_active
}

fn search_matches(item: &str, query: &str) -> bool {
    query.is_empty()
        || item.char_indices().any(|(start, _)| {
            let mut remaining = item[start..].chars();
            query.chars().all(|needle| {
                remaining
                    .next()
                    .is_some_and(|hay| hay.eq_ignore_ascii_case(&needle))
            })
        })
}

fn normalize_rect(a: [f32; 2], b: [f32; 2]) -> Option<([f32; 2], [f32; 2])> {
    let min = [a[0].min(b[0]), a[1].min(b[1])];
    let max = [a[0].max(b[0]), a[1].max(b[1])];
    if min[0] == max[0] || min[1] == max[1] {
        return None;
    }
    Some((min, max))
}

pub(crate) fn point_in_rect(p: [f32; 2], rect: [f32; 4]) -> bool {
    p[0] >= rect[0] && p[0] < rect[2] && p[1] >= rect[1] && p[1] < rect[3]
}

fn routed_draw_list(ctx: &mut Context) -> &mut crate::draw::DrawList {
    if ctx.popup_scope_active {
        &mut ctx.overlay
    } else {
        &mut ctx.current_window_mut().draw_list
    }
}

pub(crate) fn hovered(ctx: &Context, rect: [f32; 4]) -> bool {
    if ctx.active.is_some() || !point_in_rect(ctx.input.mouse_pos, ctx.target_rect(rect)) {
        return false;
    }
    hover_layer_gate(ctx)
}

pub(crate) fn hover_layer_gate(ctx: &Context) -> bool {
    if ctx.rounded_clips.iter().any(|clip| {
        clip.popup == ctx.popup_scope_active
            && clip.popup_id == ctx.routing.scope_id()
            && clip.window == ctx.current_window
            && !clip.shape.contains(ctx.input.mouse_pos)
    }) {
        return false;
    }
    if let Some(clip) = ctx.interaction_clip {
        if !point_in_rect(ctx.input.mouse_pos, clip) {
            return false;
        }
    }
    if ctx.popup_scope_active {
        return ctx.popup_input_allowed();
    }
    if ctx.popup_modal && ctx.open_popup.is_some() {
        return false;
    }
    if ctx.routing.popups[..ctx.routing.count].iter().any(|slot| {
        slot.rect
            .is_some_and(|rect| point_in_rect(ctx.input.mouse_pos, rect))
    }) {
        return false;
    }
    match (ctx.current_window, ctx.hover_top_window) {
        (Some(current), Some(top)) => current == top,
        _ => true,
    }
}

pub(crate) fn draw_checkerboard(draw_list: &mut crate::draw::DrawList, rect: [f32; 4]) {
    let mut row = 0u32;
    let mut y = rect[1];
    while y < rect[3] {
        let cell_bottom = (y + COLOR_EDIT_CHECKER_CELL).min(rect[3]);
        let mut col = 0u32;
        let mut x = rect[0];
        while x < rect[2] {
            let cell_right = (x + COLOR_EDIT_CHECKER_CELL).min(rect[2]);
            let color = if (row + col).is_multiple_of(2) {
                COLOR_EDIT_CHECKER_LIGHT
            } else {
                COLOR_EDIT_CHECKER_DARK
            };
            draw_list.add_filled_rect([x, y], [cell_right, cell_bottom], color);
            x = cell_right;
            col += 1;
        }
        y = cell_bottom;
        row += 1;
    }
}

#[cfg(test)]
mod tests {
    use crate::context::Context;
    use crate::input::{InputState, Key, KeyEvent};
    use crate::scroll::ScrollFlags;
    use alloc::vec::Vec;

    const DEJAVU_SANS: &[u8] = include_bytes!("../tests/assets/DejaVuSans.ttf");

    fn test_context() -> Context {
        Context::new(DEJAVU_SANS).expect("DejaVu Sans must load")
    }

    #[test]
    fn full_frame_cycle_produces_consistent_draw_data() {
        let mut ctx = test_context();
        ctx.set_input(InputState::default());
        ctx.new_frame(0.016);

        let mut closure_ran = false;
        let mut ui = ctx.ui();
        ui.window("Test Window", ScrollFlags::NONE, |_ui| {
            closure_ran = true;
        });
        let _ = ui;
        assert!(closure_ran);

        let data = ctx.render();
        assert!(!data.vertices.is_empty());
        let elem_sum: u32 = data.commands.iter().map(|c| c.elem_count).sum();
        assert_eq!(elem_sum as usize, data.indices.len());
        for cmd in data.commands {
            assert!(cmd.clip_rect[0] > f32::MIN / 2.0);
            assert!(cmd.clip_rect[2] < f32::MAX / 2.0);
        }
    }

    #[test]
    fn window_closure_leaves_id_stack_and_clip_stack_balanced() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("Scoped", ScrollFlags::NONE, |_| {});
        }
        assert_eq!(
            ctx.id_stack.id("probe"),
            crate::id::IdStack::new().id("probe")
        );
        let window = &ctx.windows[0];
        assert_eq!(window.draw_list.current_clip(), window.rect());
        ctx.render();
    }

    #[test]
    fn scrollable_window_closure_leaves_id_stack_and_both_clip_levels_balanced() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("Scoped", ScrollFlags::WHEEL, |ui| {
                ui.label("row");
            });
        }
        assert_eq!(
            ctx.id_stack.id("probe"),
            crate::id::IdStack::new().id("probe")
        );
        const MARKER: [f32; 4] = [-7.0, -7.0, 7.0, 7.0];
        let window = &mut ctx.windows[0];
        window.draw_list.replace_base_clip(MARKER);
        assert_eq!(
            window.draw_list.current_clip(),
            MARKER,
            "end_window must pop the body content clip AND the window clip"
        );
        assert!(ctx.interaction_clip.is_none());
        assert!(ctx.current_window.is_none());
    }

    #[test]
    fn two_windows_emit_disjoint_geometry_with_index_fixup() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let (rect_a, rect_b, verts_a, indices_a, verts_b, indices_b) = {
            let mut ui = ctx.ui();
            ui.window("A", ScrollFlags::NONE, |_| {});
            ui.window("B", ScrollFlags::NONE, |_| {});
            let _ = ui;
            let a = ctx.windows[0].draw_list.as_draw_data();
            let b = ctx.windows[1].draw_list.as_draw_data();
            (
                ctx.windows[0].rect(),
                ctx.windows[1].rect(),
                a.vertices.len(),
                a.indices.len(),
                b.vertices.len(),
                b.indices.len(),
            )
        };
        assert_ne!(rect_a, rect_b);

        let data = ctx.render();
        assert_eq!(data.vertices.len(), verts_a + verts_b);
        assert_eq!(data.indices.len(), indices_a + indices_b);
        let inside = |v: &crate::draw::DrawVert, r: [f32; 4]| {
            let margin = crate::draw::FRINGE_WIDTH + 1e-3;
            v.pos[0] >= r[0] - margin
                && v.pos[0] <= r[2] + margin
                && v.pos[1] >= r[1] - margin
                && v.pos[1] <= r[3] + margin
        };
        assert!(data.vertices[..verts_a].iter().all(|v| inside(v, rect_a)));
        assert!(data.vertices[verts_a..].iter().all(|v| inside(v, rect_b)));

        assert!(data.indices[..indices_a]
            .iter()
            .all(|&i| (i as usize) < verts_a));
        assert!(data.indices[indices_a..]
            .iter()
            .all(|&i| (i as usize) >= verts_a));
    }

    #[test]
    fn closed_window_is_skipped_by_render() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("Open", ScrollFlags::NONE, |_| {});
            ui.window("Closed", ScrollFlags::NONE, |_| {});
        }
        ctx.windows[1].open = false;
        let rect = ctx.windows[0].rect();
        let open_list = ctx.windows[0].draw_list.as_draw_data();
        let (open_verts, open_indices) = (open_list.vertices.len(), open_list.indices.len());

        let data = ctx.render();
        assert_eq!(data.vertices.len(), open_verts);
        assert_eq!(data.indices.len(), open_indices);
        let margin = crate::draw::FRINGE_WIDTH + 1e-3;
        for v in data.vertices {
            assert!(v.pos[0] >= rect[0] - margin && v.pos[0] <= rect[2] + margin);
        }
    }

    fn declare_label_button_frame(
        use_begin_end: bool,
    ) -> (
        Vec<crate::draw::DrawVert>,
        Vec<u32>,
        Vec<crate::draw::DrawCmd>,
        [f32; 4],
        bool,
    ) {
        let mut ctx = test_context();
        ctx.set_input(InputState::default());
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            if use_begin_end {
                assert!(ui.begin_window("Pair", ScrollFlags::NONE));
                ui.label("hello");
                let _ = ui.button("OK");
                ui.end_window();
            } else {
                ui.window("Pair", ScrollFlags::NONE, |ui| {
                    ui.label("hello");
                    let _ = ui.button("OK");
                });
            }
        }
        assert_eq!(
            ctx.id_stack.id("probe"),
            crate::id::IdStack::new().id("probe")
        );
        assert!(ctx.current_window.is_none());
        let rect = ctx.windows[0].rect();
        let open = ctx.windows[0].open;
        let data = ctx.render();
        (
            data.vertices.to_vec(),
            data.indices.to_vec(),
            data.commands.to_vec(),
            rect,
            open,
        )
    }

    #[test]
    fn begin_end_pair_matches_closure_form_byte_identically() {
        let closure = declare_label_button_frame(false);
        let pair = declare_label_button_frame(true);
        assert_eq!(closure, pair);
    }

    #[test]
    #[should_panic(expected = "no window open")]
    fn end_window_without_an_open_window_panics() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.end_window();
    }

    #[test]
    #[should_panic(expected = "already open")]
    fn nested_begin_window_panics() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        assert!(ui.begin_window("Outer", ScrollFlags::NONE));
        let _ = ui.begin_window("Inner", ScrollFlags::NONE);
    }

    #[test]
    fn closed_window_begin_returns_false_and_emits_nothing() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            assert!(ui.begin_window("Win", ScrollFlags::NONE));
            ui.end_window();
        }
        ctx.render();
        ctx.windows[0].open = false;

        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            assert!(!ui.begin_window("Win", ScrollFlags::NONE));
        }
        let data = ctx.render();
        assert!(data.vertices.is_empty());
        assert!(data.indices.is_empty());
        assert_eq!(
            ctx.id_stack.id("probe"),
            crate::id::IdStack::new().id("probe")
        );
        assert!(ctx.current_window.is_none());
    }

    fn two_face_context() -> (Context, crate::font::FontId) {
        let mut ctx = test_context();
        let big = ctx
            .add_font(DEJAVU_SANS, 32.0)
            .expect("a second face must register");
        (ctx, big)
    }

    #[test]
    #[should_panic(expected = "unknown FontId")]
    fn push_font_with_an_unknown_id_panics() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.push_font(crate::font::FontId(7));
    }

    #[test]
    #[should_panic(expected = "no font scope open")]
    fn pop_font_without_a_matching_push_panics() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.pop_font();
    }

    #[test]
    #[should_panic(expected = "unbalanced font stack")]
    fn render_with_an_unbalanced_font_stack_panics() {
        let (mut ctx, big) = two_face_context();
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.push_font(big);
        }
        let _ = ctx.render();
    }

    fn declare_font_scope_frame(
        use_closure: bool,
    ) -> (
        Vec<crate::draw::DrawVert>,
        Vec<u32>,
        Vec<crate::draw::DrawCmd>,
    ) {
        let (mut ctx, big) = two_face_context();
        ctx.set_input(InputState::default());
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("Fonts", ScrollFlags::NONE, |ui| {
                if use_closure {
                    ui.with_font(big, |ui| {
                        ui.label("hello");
                        let _ = ui.button("OK");
                    });
                } else {
                    ui.push_font(big);
                    ui.label("hello");
                    let _ = ui.button("OK");
                    ui.pop_font();
                }
            });
        }
        assert!(ctx.font_stack.is_empty());
        let data = ctx.render();
        (
            data.vertices.to_vec(),
            data.indices.to_vec(),
            data.commands.to_vec(),
        )
    }

    #[test]
    fn with_font_matches_the_push_pop_pair_byte_identically() {
        let closure = declare_font_scope_frame(true);
        let pair = declare_font_scope_frame(false);
        assert_eq!(closure, pair);
    }

    #[test]
    fn pushed_face_sizes_text_and_rows_by_its_own_rasterization() {
        let (mut ctx, big) = two_face_context();
        ctx.new_frame(0.016);
        let mut default_advance = 0.0;
        let mut big_advance = 0.0;
        let mut default_size = [0.0; 2];
        let mut big_size = [0.0; 2];
        {
            let mut ui = ctx.ui();
            ui.window("W", ScrollFlags::NONE, |ui| {
                let before = ui.ctx.current_window_mut().cursor.next_pos();
                ui.label("Hello");
                default_advance = ui.ctx.current_window_mut().cursor.next_pos()[1] - before[1];
                default_size = ui.measure_text("Hello");
                ui.with_font(big, |ui| {
                    let before = ui.ctx.current_window_mut().cursor.next_pos();
                    ui.label("Hello");
                    big_advance = ui.ctx.current_window_mut().cursor.next_pos()[1] - before[1];
                    big_size = ui.measure_text("Hello");
                });
            });
        }
        ctx.render();
        assert!(big_size[0] > default_size[0]);
        assert!(big_size[1] > default_size[1]);
        assert!(big_advance > default_advance);
    }

    #[test]
    fn title_bar_height_follows_the_face_active_at_window_declaration() {
        let (mut ctx, big) = two_face_context();
        ctx.new_frame(0.016);
        let mut default_bar = 0.0;
        let mut big_bar = 0.0;
        {
            let mut ui = ctx.ui();
            ui.window("A", ScrollFlags::NONE, |ui| {
                let w = ui.ctx.current_window_mut();
                default_bar = w.cursor.next_pos()[1] - w.pos[1];
            });
            ui.with_font(big, |ui| {
                ui.window("B", ScrollFlags::NONE, |ui| {
                    let w = ui.ctx.current_window_mut();
                    big_bar = w.cursor.next_pos()[1] - w.pos[1];
                });
            });
        }
        ctx.render();
        let style = ctx.style;
        let extra = style.window_padding[1] + 2.0 * style.frame_padding[1];
        assert_eq!(default_bar, ctx.font().line_height() + extra);
        assert_eq!(big_bar, ctx.font().line_height_in(big) + extra);
        assert!(big_bar > default_bar);
    }

    #[test]
    fn wheel_scroll_step_follows_the_face_active_at_region_declaration() {
        let (mut ctx, big) = two_face_context();
        let mut region_center = [0.0; 2];
        ctx.set_input(InputState::default());
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("W", ScrollFlags::NONE, |ui| {
                ui.with_font(big, |ui| {
                    let pos = ui.ctx.current_window_mut().cursor.next_pos();
                    region_center = [pos[0] + 100.0, pos[1] + 24.0];
                    ui.scroll_region("Log", [200.0, 48.0], |ui| {
                        for _ in 0..10 {
                            ui.label("line");
                        }
                    });
                });
            });
        }
        ctx.render();
        ctx.set_input(InputState {
            mouse_pos: region_center,
            mouse_wheel: -1.0,
            ..Default::default()
        });
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("W", ScrollFlags::NONE, |ui| {
                ui.with_font(big, |ui| {
                    ui.scroll_region("Log", [200.0, 48.0], |ui| {
                        for _ in 0..10 {
                            ui.label("line");
                        }
                    });
                });
            });
        }
        ctx.render();
        let mut stack = crate::id::IdStack::new();
        stack.push("W");
        let id = stack.id("Log");
        let Some(crate::context::WidgetState::Scroll { offset, .. }) = ctx.widget_states.get(&id)
        else {
            panic!("the region must have scroll state");
        };
        let big_step = super::WHEEL_LINES_PER_NOTCH * ctx.font().line_height_in(big);
        assert_eq!(*offset, big_step);
        assert_ne!(
            big_step,
            super::WHEEL_LINES_PER_NOTCH * ctx.font().line_height()
        );
    }

    use super::Ui;

    const AWAY: [f32; 2] = [-1000.0, -1000.0];

    fn scripted_frame(
        ctx: &mut Context,
        mouse: [f32; 2],
        down: bool,
        wheel: f32,
        declare: impl FnOnce(&mut Ui),
    ) -> (
        Vec<crate::draw::DrawVert>,
        Vec<u32>,
        Vec<crate::draw::DrawCmd>,
    ) {
        ctx.set_input(InputState {
            mouse_pos: mouse,
            mouse_down: [down, false, false, false, false],
            mouse_wheel: wheel,
            keys: &[],
            viewport_size: [0.0, 0.0],
        });
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            declare(&mut ui);
        }
        let data = ctx.render();
        (
            data.vertices.to_vec(),
            data.indices.to_vec(),
            data.commands.to_vec(),
        )
    }

    fn content_origin(ctx: &Context) -> [f32; 2] {
        let style = ctx.style;
        let win = ctx.windows[0].rect();
        [
            win[0] + style.window_padding[0],
            win[1] + title_bar_height(ctx) + style.window_padding[1],
        ]
    }

    fn declare_collapsing_closure(ui: &mut Ui) {
        ui.window("W", ScrollFlags::NONE, |ui| {
            ui.collapsing_header("Settings", |ui| {
                ui.label("body");
                let _ = ui.button("apply");
            });
        });
    }

    fn declare_collapsing_pair(ui: &mut Ui) {
        assert!(ui.begin_window("W", ScrollFlags::NONE));
        if let Some(scope) = ui.begin_collapsing_header("Settings") {
            ui.label("body");
            let _ = ui.button("apply");
            ui.end_collapsing_header(scope);
        }
        ui.end_window();
    }

    fn collapsing_header_script(
        declare: impl Fn(&mut Ui),
    ) -> Vec<(
        Vec<crate::draw::DrawVert>,
        Vec<u32>,
        Vec<crate::draw::DrawCmd>,
    )> {
        let mut ctx = test_context();
        let mut frames = Vec::new();
        frames.push(scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| declare(ui)));
        let header_h = ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1];
        let origin = content_origin(&ctx);
        let at = [origin[0] + 5.0, origin[1] + header_h / 2.0];
        frames.push(scripted_frame(&mut ctx, at, true, 0.0, |ui| declare(ui)));
        frames.push(scripted_frame(&mut ctx, at, false, 0.0, |ui| declare(ui)));
        frames.push(scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| declare(ui)));
        frames
    }

    #[test]
    fn collapsing_header_pair_matches_closure_form() {
        let closure = collapsing_header_script(declare_collapsing_closure);
        assert_ne!(closure[0], closure[3]);
        assert_eq!(closure, collapsing_header_script(declare_collapsing_pair));
    }

    #[test]
    fn collapsing_header_begin_reports_the_open_state() {
        let mut ctx = test_context();
        scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
            ui.window("W", ScrollFlags::NONE, |ui| {
                assert!(ui.begin_collapsing_header("Settings").is_none());
            });
        });
        let header_h = ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1];
        let origin = content_origin(&ctx);
        let at = [origin[0] + 5.0, origin[1] + header_h / 2.0];
        let declare = |ui: &mut Ui| {
            ui.window("W", ScrollFlags::NONE, |ui| {
                if let Some(scope) = ui.begin_collapsing_header("Settings") {
                    ui.end_collapsing_header(scope);
                }
            });
        };
        scripted_frame(&mut ctx, at, true, 0.0, |ui| declare(ui));
        scripted_frame(&mut ctx, at, false, 0.0, |ui| declare(ui));
        scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
            ui.window("W", ScrollFlags::NONE, |ui| {
                let scope = ui
                    .begin_collapsing_header("Settings")
                    .expect("the header must be open after the click");
                ui.end_collapsing_header(scope);
            });
        });
    }

    fn declare_scroll_closure(ui: &mut Ui) {
        ui.window("W", ScrollFlags::NONE, |ui| {
            ui.scroll_region("Log", [200.0, 96.0], |ui| {
                for _ in 0..20 {
                    ui.label("log line");
                }
            });
        });
    }

    fn declare_scroll_pair(ui: &mut Ui) {
        assert!(ui.begin_window("W", ScrollFlags::NONE));
        let scope = ui.begin_scroll_region("Log", [200.0, 96.0]);
        for _ in 0..20 {
            ui.label("log line");
        }
        ui.end_scroll_region(scope);
        ui.end_window();
    }

    fn scroll_region_script(
        declare: impl Fn(&mut Ui),
    ) -> Vec<(
        Vec<crate::draw::DrawVert>,
        Vec<u32>,
        Vec<crate::draw::DrawCmd>,
    )> {
        let mut ctx = test_context();
        let mut frames = Vec::new();
        frames.push(scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| declare(ui)));
        let origin = content_origin(&ctx);
        let over = [origin[0] + 10.0, origin[1] + 10.0];
        for _ in 0..3 {
            frames.push(scripted_frame(&mut ctx, over, false, -1.0, |ui| {
                declare(ui)
            }));
        }
        frames
    }

    #[test]
    fn scroll_region_pair_matches_closure_form() {
        let closure = scroll_region_script(declare_scroll_closure);
        assert_ne!(closure[0], closure[2]);
        assert_eq!(closure, scroll_region_script(declare_scroll_pair));
    }

    fn declare_scrollable_window_closure(ui: &mut Ui) {
        ui.window("W", ScrollFlags::WHEEL, |ui| {
            for _ in 0..60 {
                ui.label("row");
            }
            let _ = ui.button("go");
        });
    }

    fn declare_scrollable_window_pair(ui: &mut Ui) {
        assert!(ui.begin_window("W", ScrollFlags::WHEEL));
        for _ in 0..60 {
            ui.label("row");
        }
        let _ = ui.button("go");
        ui.end_window();
    }

    fn scrollable_window_script(
        declare: impl Fn(&mut Ui),
    ) -> Vec<(
        Vec<crate::draw::DrawVert>,
        Vec<u32>,
        Vec<crate::draw::DrawCmd>,
    )> {
        let mut ctx = test_context();
        let mut frames = Vec::new();
        frames.push(scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| declare(ui)));
        let origin = content_origin(&ctx);
        let over = [origin[0] + 10.0, origin[1] + 10.0];
        for _ in 0..3 {
            frames.push(scripted_frame(&mut ctx, over, false, -1.0, |ui| {
                declare(ui)
            }));
        }
        frames
    }

    #[test]
    fn scrollable_window_pair_matches_closure_form() {
        let closure = scrollable_window_script(declare_scrollable_window_closure);
        assert_ne!(closure[0], closure[2]);
        assert_eq!(
            closure,
            scrollable_window_script(declare_scrollable_window_pair)
        );
    }

    #[test]
    fn scroll_region_draw_data_pin_matches_pre_refactor_snapshot() {
        fn declare_pin_fixture(ui: &mut Ui) {
            assert!(ui.begin_window("W", ScrollFlags::NONE));
            let scope = ui.begin_scroll_region("Log", [80.0, 24.0]);
            ui.label("x");
            ui.label("y");
            ui.label("z");
            ui.end_scroll_region(scope);
            ui.end_window();
        }

        let mut ctx = test_context();
        let _settle = scripted_frame(&mut ctx, AWAY, false, 0.0, declare_pin_fixture);
        let origin = content_origin(&ctx);
        let over = [origin[0] + 5.0, origin[1] + 5.0];
        let (verts, indices, cmds) =
            scripted_frame(&mut ctx, over, false, -1.0, declare_pin_fixture);

        assert_eq!(verts.len(), 175, "vertex count pin");
        assert_eq!(indices.len(), 261, "index count pin");
        assert_eq!(cmds.len(), 12, "command count pin");

        use crate::draw::{DrawCmd, DrawVert, TextureId};
        let expected_verts: Vec<DrawVert> = std::vec![
            DrawVert {
                pos: [f32::from_bits(0x42700000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42700000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42700000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E60000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42700000), f32::from_bits(0x43E60000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B40000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B40000), f32::from_bits(0x43E60000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E60000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E60000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E60000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42700000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42700000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B40000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E40000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [16, 16, 20, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42700000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42700000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42700000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42700000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42800000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42AD4000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42700000), f32::from_bits(0x42AD4000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [27, 27, 34, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x4285B400)],
                uv: [f32::from_bits(0x3DC00000), f32::from_bits(0x3E800000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42AC0000), f32::from_bits(0x4285B400)],
                uv: [f32::from_bits(0x3E200000), f32::from_bits(0x3E800000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42AC0000), f32::from_bits(0x429DB400)],
                uv: [f32::from_bits(0x3E200000), f32::from_bits(0x3E980000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x429DB400)],
                uv: [f32::from_bits(0x3DC00000), f32::from_bits(0x3E980000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A902BF), f32::from_bits(0x4282B505)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B152BF), f32::from_bits(0x42A3F505)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B1AD41), f32::from_bits(0x42A28AFB)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A95D41), f32::from_bits(0x42814AFB)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A902BF), f32::from_bits(0x4282B505)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B152BF), f32::from_bits(0x42A3F505)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B0D5D8), f32::from_bits(0x42A5E8A2)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A885D8), f32::from_bits(0x4284A8A2)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B1AD41), f32::from_bits(0x42A28AFB)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A95D41), f32::from_bits(0x42814AFB)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A9DA28), f32::from_bits(0x427EAEBC)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B22A28), f32::from_bits(0x42A0975E)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A95D41), f32::from_bits(0x42A3F505)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B1AD41), f32::from_bits(0x4282B505)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B152BF), f32::from_bits(0x42814AFB)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A902BF), f32::from_bits(0x42A28AFB)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A95D41), f32::from_bits(0x42A3F505)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B1AD41), f32::from_bits(0x4282B505)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B22A28), f32::from_bits(0x4284A8A2)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A9DA28), f32::from_bits(0x42A5E8A2)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B152BF), f32::from_bits(0x42814AFB)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A902BF), f32::from_bits(0x42A28AFB)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43A885D8), f32::from_bits(0x42A0975E)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B0D5D8), f32::from_bits(0x427EAEBC)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEB4000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEB4000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF34000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF34000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEB4000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEB4000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF34000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF34000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF4C000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF4C000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EECC000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EECC000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF4C000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF4C000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EECC000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EECC000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [41, 74, 122, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EEB4000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EEB4000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EF34000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EF34000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EEB4000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EEB4000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EF34000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EF34000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EF4C000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EF4C000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EECC000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EECC000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EF4C000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EF4C000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EECC000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EECC000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42C14000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42C34000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42C34000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42EF4000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42EF4000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43120000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42940000), f32::from_bits(0x42F14000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428E0000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428E0000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43150000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42C94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43160000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43150000), f32::from_bits(0x42E94000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [81, 114, 162, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x429057D8), f32::from_bits(0x42D14BD8)],
                uv: [f32::from_bits(0x3EAE0000), f32::from_bits(0x3EC60000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42A257D8), f32::from_bits(0x42D14BD8)],
                uv: [f32::from_bits(0x3EC00000), f32::from_bits(0x3EC60000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42A257D8), f32::from_bits(0x42E34BD8)],
                uv: [f32::from_bits(0x3EC00000), f32::from_bits(0x3ED80000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x429057D8), f32::from_bits(0x42E34BD8)],
                uv: [f32::from_bits(0x3EAE0000), f32::from_bits(0x3ED80000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x429057D8), f32::from_bits(0x42FE8BD8)],
                uv: [f32::from_bits(0x3EC20000), f32::from_bits(0x3EC60000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42A257D8), f32::from_bits(0x42FE8BD8)],
                uv: [f32::from_bits(0x3ED40000), f32::from_bits(0x3EC60000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42A257D8), f32::from_bits(0x430C45EC)],
                uv: [f32::from_bits(0x3ED40000), f32::from_bits(0x3EE00000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x429057D8), f32::from_bits(0x430C45EC)],
                uv: [f32::from_bits(0x3EC20000), f32::from_bits(0x3EE00000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x429057D8), f32::from_bits(0x4315E5EC)],
                uv: [f32::from_bits(0x3ED60000), f32::from_bits(0x3EC60000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42A057D8), f32::from_bits(0x4315E5EC)],
                uv: [f32::from_bits(0x3EE60000), f32::from_bits(0x3EC60000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x42A057D8), f32::from_bits(0x431EE5EC)],
                uv: [f32::from_bits(0x3EE60000), f32::from_bits(0x3ED80000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x429057D8), f32::from_bits(0x431EE5EC)],
                uv: [f32::from_bits(0x3ED60000), f32::from_bits(0x3ED80000)],
                col: [240, 240, 240, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43DE0000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43E56A0A)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43AC0000), f32::from_bits(0x43E56A0A)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43E56A0A)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43DE0000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B3ADA3), f32::from_bits(0x43DD5CCE)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B3E6F1), f32::from_bits(0x43E5E6F1)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43AC0000), f32::from_bits(0x43E56A0A)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43E56A0A)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B3E6F1), f32::from_bits(0x43E5E6F1)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43AB5CCE), f32::from_bits(0x43E5ADA3)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43DE0000)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43AC0000), f32::from_bits(0x43E56A0A)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 255]
            },
            DrawVert {
                pos: [f32::from_bits(0x43AB5CCE), f32::from_bits(0x43E5ADA3)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
            DrawVert {
                pos: [f32::from_bits(0x43B3ADA3), f32::from_bits(0x43DD5CCE)],
                uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                col: [224, 224, 224, 0]
            },
        ];
        let expected_indices: Vec<u32> = std::vec![
            0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16,
            17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23, 24, 25, 26, 24, 26, 27, 28, 29, 30, 28, 30,
            31, 32, 33, 34, 32, 34, 35, 36, 37, 38, 36, 38, 39, 40, 41, 42, 40, 42, 43, 44, 45, 46,
            44, 46, 47, 48, 49, 50, 48, 50, 51, 52, 53, 54, 52, 54, 55, 56, 57, 58, 56, 58, 59, 60,
            61, 62, 60, 62, 63, 64, 65, 66, 64, 66, 67, 68, 69, 70, 68, 70, 71, 72, 73, 74, 72, 74,
            75, 76, 77, 78, 76, 78, 79, 80, 81, 82, 80, 82, 83, 84, 85, 86, 84, 86, 87, 88, 89, 90,
            88, 90, 91, 92, 93, 94, 92, 94, 95, 96, 97, 98, 96, 98, 99, 100, 101, 102, 100, 102,
            103, 104, 105, 106, 104, 106, 107, 108, 109, 110, 108, 110, 111, 112, 113, 114, 112,
            114, 115, 116, 117, 118, 116, 118, 119, 120, 121, 122, 120, 122, 123, 124, 125, 126,
            124, 126, 127, 128, 129, 130, 128, 130, 131, 132, 133, 134, 132, 134, 135, 136, 137,
            138, 136, 138, 139, 140, 141, 142, 140, 142, 143, 144, 145, 146, 144, 146, 147, 148,
            149, 150, 148, 150, 151, 152, 153, 154, 152, 154, 155, 156, 157, 158, 156, 158, 159,
            160, 161, 162, 163, 164, 165, 163, 165, 166, 167, 168, 169, 167, 169, 170, 171, 172,
            173, 171, 173, 174,
        ];
        let expected_cmds: Vec<DrawCmd> = std::vec![
            DrawCmd {
                effects: 0,
                elem_count: 24,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(0)
            },
            DrawCmd {
                effects: 0,
                elem_count: 30,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(1)
            },
            DrawCmd {
                effects: 0,
                elem_count: 12,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(0)
            },
            DrawCmd {
                effects: 0,
                elem_count: 12,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(1)
            },
            DrawCmd {
                effects: 0,
                elem_count: 6,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(0)
            },
            DrawCmd {
                effects: 0,
                elem_count: 36,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(1)
            },
            DrawCmd {
                effects: 0,
                elem_count: 24,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(0)
            },
            DrawCmd {
                effects: 0,
                elem_count: 30,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(1)
            },
            DrawCmd {
                effects: 0,
                elem_count: 24,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(0)
            },
            DrawCmd {
                effects: 0,
                elem_count: 24,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(1)
            },
            DrawCmd {
                effects: 0,
                elem_count: 18,
                clip_rect: [
                    f32::from_bits(0x429057D8),
                    f32::from_bits(0x42C597D8),
                    f32::from_bits(0x4313D414),
                    f32::from_bits(0x42ECE828)
                ],
                texture: TextureId(0)
            },
            DrawCmd {
                effects: 0,
                elem_count: 21,
                clip_rect: [
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x42700000),
                    f32::from_bits(0x43B40000),
                    f32::from_bits(0x43E60000)
                ],
                texture: TextureId(1)
            },
        ];

        assert_eq!(
            verts, expected_verts,
            "vertex data pin (full field comparison)"
        );
        assert_eq!(indices, expected_indices, "index data pin");
        assert_eq!(
            cmds, expected_cmds,
            "command data pin (full field comparison)"
        );
    }

    fn declare_tabs_closure(ui: &mut Ui) {
        ui.window("W", ScrollFlags::NONE, |ui| {
            ui.tab_bar("Tabs", |tabs| {
                tabs.tab("Alpha", |ui| {
                    ui.label("alpha body");
                });
                tabs.tab("Beta", |ui| {
                    ui.label("beta body");
                });
            });
        });
    }

    fn declare_tabs_pair(ui: &mut Ui) {
        assert!(ui.begin_window("W", ScrollFlags::NONE));
        ui.begin_tab_bar("Tabs");
        if ui.tab("Alpha") {
            ui.label("alpha body");
        }
        if ui.tab("Beta") {
            ui.label("beta body");
        }
        ui.end_tab_bar();
        ui.end_window();
    }

    fn tab_bar_script(
        declare: impl Fn(&mut Ui),
    ) -> Vec<(
        Vec<crate::draw::DrawVert>,
        Vec<u32>,
        Vec<crate::draw::DrawCmd>,
    )> {
        let mut ctx = test_context();
        let mut frames = Vec::new();
        frames.push(scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| declare(ui)));
        let style = ctx.style;
        let origin = content_origin(&ctx);
        let header_h = ctx.font().line_height() + 2.0 * style.frame_padding[1];
        let alpha_w = ctx.font.measure("Alpha")[0] + 2.0 * style.frame_padding[0];
        let beta_w = ctx.font.measure("Beta")[0] + 2.0 * style.frame_padding[0];
        let at = [
            origin[0] + alpha_w + style.item_spacing[0] + beta_w / 2.0,
            origin[1] + header_h / 2.0,
        ];
        frames.push(scripted_frame(&mut ctx, at, true, 0.0, |ui| declare(ui)));
        frames.push(scripted_frame(&mut ctx, at, false, 0.0, |ui| declare(ui)));
        frames.push(scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| declare(ui)));
        frames
    }

    #[test]
    fn tab_bar_pair_matches_closure_form() {
        let closure = tab_bar_script(declare_tabs_closure);
        assert_ne!(closure[0], closure[3]);
        assert_eq!(closure, tab_bar_script(declare_tabs_pair));
    }

    #[test]
    fn container_pairs_leave_scopes_balanced() {
        let mut ctx = test_context();
        let declare = |ui: &mut Ui| {
            assert!(ui.begin_window("W", ScrollFlags::NONE));
            if let Some(header) = ui.begin_collapsing_header("Settings") {
                let region = ui.begin_scroll_region("Log", [200.0, 96.0]);
                ui.label("line");
                ui.end_scroll_region(region);
                ui.begin_tab_bar("Tabs");
                if ui.tab("A") {
                    ui.label("a");
                }
                if ui.tab("B") {
                    ui.label("b");
                }
                ui.end_tab_bar();
                ui.end_collapsing_header(header);
            }
            ui.end_window();
        };
        scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| declare(ui));
        let header_h = ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1];
        let origin = content_origin(&ctx);
        let at = [origin[0] + 5.0, origin[1] + header_h / 2.0];
        scripted_frame(&mut ctx, at, true, 0.0, |ui| declare(ui));
        scripted_frame(&mut ctx, at, false, 0.0, |ui| declare(ui));
        scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| declare(ui));
        assert_eq!(
            ctx.id_stack.id("probe"),
            crate::id::IdStack::new().id("probe")
        );
        assert!(ctx.current_window.is_none());
        assert!(ctx.tab_bar_scopes.is_empty());
    }

    #[test]
    #[should_panic(expected = "no tab bar open")]
    fn tab_without_an_open_tab_bar_panics() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        let _ = ui.tab("X");
    }

    #[test]
    #[should_panic(expected = "no tab bar open")]
    fn end_tab_bar_without_an_open_tab_bar_panics() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut ui = ctx.ui();
        ui.end_tab_bar();
    }

    fn title_bar_height(ctx: &Context) -> f32 {
        ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1]
    }

    fn chrome_id(title: &str) -> crate::id::Id {
        let mut stack = crate::id::IdStack::new();
        stack.push(title);
        stack.id(super::TITLE_BAR_TAG)
    }

    fn frame(ctx: &mut Context, mouse_pos: [f32; 2], left_down: bool, titles: &[&str]) {
        frame_viewport(ctx, mouse_pos, left_down, [0.0, 0.0], titles);
    }

    fn frame_viewport(
        ctx: &mut Context,
        mouse_pos: [f32; 2],
        left_down: bool,
        viewport_size: [f32; 2],
        titles: &[&str],
    ) {
        ctx.set_input(InputState {
            mouse_pos,
            mouse_down: [left_down, false, false, false, false],
            viewport_size,
            ..Default::default()
        });
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            for &title in titles {
                ui.window(title, ScrollFlags::NONE, |_| {});
            }
        }
        ctx.render();
    }

    #[test]
    fn title_bar_press_captures_chrome_and_brings_window_to_front() {
        let mut ctx = test_context();
        frame(&mut ctx, [0.0, 0.0], false, &["A", "B"]);
        assert_eq!(ctx.windows[0].title, "A");
        let pos_a = ctx.windows[0].pos;
        let pos_b = ctx.windows[1].pos;

        let grab = [pos_a[0] + 2.0, pos_a[1] + title_bar_height(&ctx) / 2.0];
        assert!(grab[0] < pos_b[0], "grab point must miss B's title bar");
        frame(&mut ctx, grab, true, &["A", "B"]);

        assert_eq!(ctx.active, Some(chrome_id("A")));
        assert_eq!(ctx.windows[0].title, "B");
        assert_eq!(ctx.windows[1].title, "A");
        assert_eq!(ctx.windows[1].pos, pos_a);
    }

    #[test]
    fn title_bar_drag_moves_window_until_release() {
        let mut ctx = test_context();
        frame(&mut ctx, [0.0, 0.0], false, &["Drag Me"]);
        let start = ctx.windows[0].pos;
        let grab = [start[0] + 20.0, start[1] + title_bar_height(&ctx) / 2.0];

        frame(&mut ctx, grab, true, &["Drag Me"]);
        assert_eq!(ctx.active, Some(chrome_id("Drag Me")));
        assert_eq!(ctx.windows[0].pos, start);

        let delta = [35.0, 17.0];
        let moved_mouse = [grab[0] + delta[0], grab[1] + delta[1]];
        frame(&mut ctx, moved_mouse, true, &["Drag Me"]);
        assert_eq!(ctx.active, Some(chrome_id("Drag Me")));
        assert_eq!(
            ctx.windows[0].pos,
            [start[0] + delta[0], start[1] + delta[1]]
        );
        let moved_rect = ctx.windows[0].rect();
        let moved_data = ctx.windows[0].draw_list.as_draw_data();
        assert!(!moved_data.commands.is_empty());
        for cmd in moved_data.commands {
            assert_eq!(cmd.clip_rect, moved_rect);
        }

        frame(&mut ctx, moved_mouse, false, &["Drag Me"]);
        assert_eq!(ctx.active, None);
        assert_eq!(
            ctx.windows[0].pos,
            [start[0] + delta[0], start[1] + delta[1]]
        );

        let stray = [moved_mouse[0] + 50.0, moved_mouse[1] + 50.0];
        frame(&mut ctx, stray, true, &["Drag Me"]);
        assert_eq!(
            ctx.windows[0].pos,
            [start[0] + delta[0], start[1] + delta[1]]
        );
    }

    #[test]
    fn drag_clamps_to_viewport_edges() {
        let mut ctx = test_context();
        let viewport = [800.0, 600.0];
        frame_viewport(&mut ctx, [0.0, 0.0], false, viewport, &["Clamp"]);
        let start = ctx.windows[0].pos;
        let size = ctx.windows[0].size;
        let tb = title_bar_height(&ctx);
        let grab = [start[0] + 20.0, start[1] + tb / 2.0];

        frame_viewport(&mut ctx, grab, true, viewport, &["Clamp"]);
        assert_eq!(ctx.active, Some(chrome_id("Clamp")));

        frame_viewport(&mut ctx, [-5000.0, grab[1]], true, viewport, &["Clamp"]);
        assert_eq!(ctx.windows[0].pos[0], 32.0 - size[0]);
        assert_eq!(ctx.windows[0].pos[1], start[1]);

        frame_viewport(&mut ctx, [5000.0, grab[1]], true, viewport, &["Clamp"]);
        assert_eq!(ctx.windows[0].pos[0], viewport[0] - 32.0);

        frame_viewport(&mut ctx, [grab[0], 5000.0], true, viewport, &["Clamp"]);
        assert_eq!(ctx.windows[0].pos[1], viewport[1] - tb);

        frame_viewport(&mut ctx, [grab[0], -5000.0], true, viewport, &["Clamp"]);
        assert_eq!(ctx.windows[0].pos[1], 0.0);
    }

    #[test]
    fn drag_is_unclamped_when_viewport_unknown() {
        let mut ctx = test_context();
        frame(&mut ctx, [0.0, 0.0], false, &["Free"]);
        let start = ctx.windows[0].pos;
        let tb = title_bar_height(&ctx);
        let grab = [start[0] + 20.0, start[1] + tb / 2.0];

        frame(&mut ctx, grab, true, &["Free"]);
        let far = [-5000.0, 5000.0];
        frame(&mut ctx, far, true, &["Free"]);
        assert_eq!(ctx.windows[0].pos, [far[0] - 20.0, far[1] - tb / 2.0]);
    }

    #[test]
    fn drag_with_degenerate_viewport_pins_to_axis_min_without_panicking() {
        let mut ctx = test_context();
        let viewport = [800.0, 10.0];
        frame_viewport(&mut ctx, [0.0, 0.0], false, viewport, &["Tiny"]);
        let start = ctx.windows[0].pos;
        let tb = title_bar_height(&ctx);
        assert!(
            viewport[1] < tb,
            "viewport must be shorter than the title bar"
        );
        let grab = [start[0] + 20.0, start[1] + tb / 2.0];

        frame_viewport(&mut ctx, grab, true, viewport, &["Tiny"]);
        frame_viewport(&mut ctx, [grab[0], 5000.0], true, viewport, &["Tiny"]);
        assert_eq!(ctx.windows[0].pos[1], 0.0);
        assert_eq!(ctx.windows[0].pos[0], start[0]);
    }

    #[test]
    fn body_press_does_not_capture_chrome_or_move_window() {
        let mut ctx = test_context();
        frame(&mut ctx, [0.0, 0.0], false, &["Body"]);
        let start = ctx.windows[0].pos;
        let press = [start[0] + 40.0, start[1] + title_bar_height(&ctx) + 30.0];

        frame(&mut ctx, press, true, &["Body"]);
        assert_eq!(ctx.active, None);

        let moved = [press[0] + 25.0, press[1] + 25.0];
        frame(&mut ctx, moved, true, &["Body"]);
        assert_eq!(ctx.windows[0].pos, start);
    }

    #[test]
    fn content_origin_starts_below_title_bar() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut first_item = [0.0; 2];
        {
            let mut ui = ctx.ui();
            ui.window("Probe", ScrollFlags::NONE, |ui| {
                first_item = ui.ctx.current_window_mut().cursor.next_pos();
            });
        }
        let window = &ctx.windows[0];
        let bar_bottom = window.pos[1] + title_bar_height(&ctx);
        assert!(first_item[1] >= bar_bottom);
        assert_eq!(
            first_item,
            [
                window.pos[0] + ctx.style.window_padding[0],
                bar_bottom + ctx.style.window_padding[1],
            ]
        );
        ctx.render();
    }

    #[test]
    fn title_text_is_emitted_inside_title_bar() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("Title Text", ScrollFlags::NONE, |_| {});
        }
        let bar = {
            let window = &ctx.windows[0];
            [
                window.pos[0],
                window.pos[1],
                window.pos[0] + window.size[0],
                window.pos[1] + title_bar_height(&ctx),
            ]
        };
        let data = ctx.windows[0].draw_list.as_draw_data();
        let mut index_offset = 0usize;
        let mut glyph_vertices = 0usize;
        for cmd in data.commands {
            let next = index_offset + cmd.elem_count as usize;
            if cmd.texture == crate::draw::TextureId::FONT {
                for &i in &data.indices[index_offset..next] {
                    let v = data.vertices[i as usize];
                    if ctx.font.uv_samples_shape_mask(v.uv) {
                        continue;
                    }
                    assert!(
                        v.pos[0] >= bar[0]
                            && v.pos[0] <= bar[2]
                            && v.pos[1] >= bar[1]
                            && v.pos[1] <= bar[3],
                        "title glyph vertex {:?} outside title bar {bar:?}",
                        v.pos
                    );
                    glyph_vertices += 1;
                }
            }
            index_offset = next;
        }
        assert!(glyph_vertices > 0, "title text must emit glyph quads");
        ctx.render();
    }

    #[test]
    fn bring_to_front_mid_closure_keeps_clip_stacks_balanced() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("A", ScrollFlags::NONE, |_| {});
            ui.window("B", ScrollFlags::NONE, |_| {});
        }
        ctx.render();

        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("A", ScrollFlags::NONE, |ui| {
                ui.ctx.bring_window_to_front(0);
            });
            ui.window("B", ScrollFlags::NONE, |_| {});
        }
        assert_eq!(ctx.windows[1].title, "A");
        assert_eq!(ctx.windows[0].title, "B");
        for window in &ctx.windows {
            assert_eq!(window.draw_list.current_clip(), window.rect());
        }
        ctx.render();
    }

    fn quads_from(ctx: &Context, start: usize, n: usize) -> alloc::vec::Vec<[[f32; 2]; 4]> {
        let data = ctx.windows[0].draw_list.as_draw_data();
        data.vertices[start..start + 4 * n]
            .chunks_exact(4)
            .map(|q| [q[0].pos, q[1].pos, q[2].pos, q[3].pos])
            .collect()
    }

    fn assert_all_commands_clipped_to_window(ctx: &Context) {
        let rect = ctx.windows[0].rect();
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert!(!data.commands.is_empty());
        for cmd in data.commands {
            assert_eq!(cmd.clip_rect, rect);
        }
    }

    fn assert_pos_near(actual: [f32; 2], expected: [f32; 2]) {
        let eps = 1e-3;
        assert!(
            (actual[0] - expected[0]).abs() < eps && (actual[1] - expected[1]).abs() < eps,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn draw_line_emits_butt_cap_quad_in_current_window() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = false;
        ctx.new_frame(0.016);
        let mut start = 0usize;
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                start = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_line([10.0, 20.0], [30.0, 20.0], [200, 0, 0, 255], 4.0);
            });
        }
        let quads = quads_from(&ctx, start, 1);
        let expected = [[10.0, 22.0], [30.0, 22.0], [30.0, 18.0], [10.0, 18.0]];
        for (a, e) in quads[0].iter().zip(expected.iter()) {
            assert_pos_near(*a, *e);
        }
        let data = ctx.windows[0].draw_list.as_draw_data();
        let last_cmd = data.commands.last().expect("line must emit a command");
        assert_eq!(last_cmd.texture, crate::draw::TextureId::WHITE);
        for v in &data.vertices[start..start + 4] {
            assert_eq!(v.col, [200, 0, 0, 255]);
        }
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn draw_rect_filled_normalizes_and_matches_internal_emission() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = false;
        ctx.new_frame(0.016);
        let mut start = 0usize;
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                start = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_rect_filled([10.0, 10.0], [20.0, 30.0], [1, 2, 3, 255]);
                ui.draw_rect_filled([20.0, 30.0], [10.0, 10.0], [1, 2, 3, 255]);
                ui.ctx.current_window_mut().draw_list.add_filled_rect(
                    [10.0, 10.0],
                    [20.0, 30.0],
                    [1, 2, 3, 255],
                );
            });
        }
        let quads = quads_from(&ctx, start, 3);
        let expected = [[10.0, 10.0], [20.0, 10.0], [20.0, 30.0], [10.0, 30.0]];
        for quad in &quads {
            assert_eq!(quad, &expected);
        }
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn draw_rect_outline_emits_four_frame_quads() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = false;
        ctx.new_frame(0.016);
        let mut start = 0usize;
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                start = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_rect_outline([10.0, 10.0], [30.0, 26.0], [0, 200, 0, 255], 2.0);
            });
        }
        let quads = quads_from(&ctx, start, 4);
        let expected = [
            [[10.0, 10.0], [30.0, 10.0], [30.0, 12.0], [10.0, 12.0]],
            [[10.0, 24.0], [30.0, 24.0], [30.0, 26.0], [10.0, 26.0]],
            [[10.0, 12.0], [12.0, 12.0], [12.0, 24.0], [10.0, 24.0]],
            [[28.0, 12.0], [30.0, 12.0], [30.0, 24.0], [28.0, 24.0]],
        ];
        assert_eq!(quads, expected);
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn degenerate_canvas_input_emits_nothing() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let (before, after) = {
            let mut counts = (0usize, 0usize);
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                counts.0 = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_line([10.0, 10.0], [10.0, 10.0], [255, 0, 0, 255], 2.0);
                ui.draw_line([0.0, 0.0], [10.0, 10.0], [255, 0, 0, 255], 0.0);
                ui.draw_rect_filled([5.0, 5.0], [5.0, 20.0], [255, 0, 0, 255]);
                ui.draw_rect_outline([5.0, 5.0], [20.0, 5.0], [255, 0, 0, 255], 2.0);
                ui.draw_rect_outline([5.0, 5.0], [20.0, 20.0], [255, 0, 0, 255], 0.0);
                ui.draw_circle_filled([50.0, 50.0], 0.0, [255, 0, 0, 255]);
                ui.draw_circle_filled([50.0, 50.0], -10.0, [255, 0, 0, 255]);
                ui.draw_circle_outline([50.0, 50.0], 0.0, [255, 0, 0, 255], 2.0);
                ui.draw_circle_outline([50.0, 50.0], 10.0, [255, 0, 0, 255], 0.0);
                ui.draw_arc([50.0, 50.0], 0.0, 0.0..=1.0, [255, 0, 0, 255], 2.0);
                ui.draw_arc([50.0, 50.0], 10.0, 0.0..=1.0, [255, 0, 0, 255], -1.0);
                ui.draw_arc([50.0, 50.0], 10.0, 1.0..=1.0, [255, 0, 0, 255], 2.0);
                ui.draw_polygon_filled(&[], [255, 0, 0, 255]);
                ui.draw_polygon_filled(&[[5.0, 5.0], [20.0, 20.0]], [255, 0, 0, 255]);
                ui.draw_image(
                    crate::draw::TextureId(42),
                    [5.0, 5.0],
                    [5.0, 20.0],
                    [0.0, 0.0],
                    [1.0, 1.0],
                    [255, 0, 0, 255],
                );
                counts.1 = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
            });
            counts
        };
        assert_eq!(before, after);
        ctx.render();
    }

    #[test]
    fn canvas_primitives_are_noops_outside_a_window() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.draw_line([0.0, 0.0], [10.0, 10.0], [255, 255, 255, 255], 1.0);
            ui.draw_rect_filled([0.0, 0.0], [10.0, 10.0], [255, 255, 255, 255]);
            ui.draw_rect_outline([0.0, 0.0], [10.0, 10.0], [255, 255, 255, 255], 1.0);
            ui.draw_circle_filled([5.0, 5.0], 10.0, [255, 255, 255, 255]);
            ui.draw_circle_outline([5.0, 5.0], 10.0, [255, 255, 255, 255], 1.0);
            ui.draw_arc([5.0, 5.0], 10.0, 0.0..=1.0, [255, 255, 255, 255], 1.0);
            ui.draw_polygon_filled(
                &[[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]],
                [255, 255, 255, 255],
            );
            ui.draw_image(
                crate::draw::TextureId(42),
                [0.0, 0.0],
                [10.0, 10.0],
                [0.0, 0.0],
                [1.0, 1.0],
                [255, 255, 255, 255],
            );
        }
        let data = ctx.render();
        assert!(data.vertices.is_empty());
        assert!(data.commands.is_empty());
    }

    #[test]
    fn canvas_primitives_do_not_advance_the_layout_cursor() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut before = [0.0; 2];
        let mut after = [0.0; 2];
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                before = ui.ctx.current_window_mut().cursor.next_pos();
                ui.draw_line([10.0, 20.0], [30.0, 20.0], [255, 0, 0, 255], 4.0);
                ui.draw_rect_filled([10.0, 10.0], [20.0, 30.0], [255, 0, 0, 255]);
                ui.draw_rect_outline([10.0, 10.0], [30.0, 26.0], [255, 0, 0, 255], 2.0);
                ui.draw_circle_filled([100.0, 100.0], 40.0, [255, 0, 0, 255]);
                ui.draw_circle_outline([100.0, 100.0], 40.0, [255, 0, 0, 255], 2.0);
                ui.draw_arc([100.0, 100.0], 40.0, 0.0..=1.0, [255, 0, 0, 255], 2.0);
                ui.draw_polygon_filled(
                    &[[10.0, 10.0], [30.0, 10.0], [40.0, 30.0]],
                    [255, 0, 0, 255],
                );
                ui.draw_image(
                    crate::draw::TextureId(42),
                    [10.0, 10.0],
                    [30.0, 30.0],
                    [0.0, 0.0],
                    [1.0, 1.0],
                    [255, 0, 0, 255],
                );
                after = ui.ctx.current_window_mut().cursor.next_pos();
            });
        }
        assert_eq!(before, after);
        ctx.render();
    }

    fn positions_from(ctx: &Context, start: usize, count: usize) -> alloc::vec::Vec<[f32; 2]> {
        ctx.windows[0].draw_list.as_draw_data().vertices[start..start + count]
            .iter()
            .map(|v| v.pos)
            .collect()
    }

    #[test]
    fn draw_circle_filled_emits_fan_in_current_window() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = false;
        ctx.new_frame(0.016);
        let mut start = 0usize;
        let mut vend = 0usize;
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                start = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_circle_filled([100.0, 100.0], 40.0, [0, 0, 200, 255]);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
            });
        }
        let n = crate::draw::circle_segment_count(40.0);
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert_eq!(vend - start, n + 1);
        assert_eq!(data.vertices[start].pos, [100.0, 100.0]);
        assert_pos_near(data.vertices[start + 1].pos, [140.0, 100.0]);
        for v in &data.vertices[start..vend] {
            assert_eq!(v.col, [0, 0, 200, 255]);
        }
        let last_cmd = data.commands.last().expect("circle must emit a command");
        assert_eq!(last_cmd.texture, crate::draw::TextureId::WHITE);
        assert_eq!(last_cmd.elem_count, 3 * n as u32 + 12 + 3);
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    fn mask_disk_dim(radius: f32) -> f32 {
        mask_ring_dim(radius, 0.0)
    }

    fn mask_ring_dim(radius: f32, thickness: f32) -> f32 {
        let radius_q = u64::from(crate::font::quantize_quarter_px(radius));
        let thickness_q = u64::from(crate::font::quantize_quarter_px(thickness));
        (2 * radius_q + thickness_q).div_ceil(4) as f32 + 2.0
    }

    #[test]
    fn aa_circle_fill_emits_one_mask_quad() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = true;
        ctx.new_frame(0.016);
        let (mut vstart, mut istart) = (0usize, 0usize);
        let (mut vend, mut iend) = (0usize, 0usize);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                vstart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                istart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
                ui.draw_circle_filled([100.0, 80.0], 20.0, [9, 8, 7, 255]);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                iend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
            });
        }
        let mask = ctx
            .font
            .shape_mask(crate::font::ShapeKey::disk(20.0))
            .expect("radius 20 fits the mask cap");
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert_eq!(vend - vstart, 4);
        assert_eq!(iend - istart, 6);
        let w = mask_disk_dim(20.0);
        assert_eq!(w, 42.0, "radius 20: ceil(2·20) + 2");
        assert_eq!(mask.rect[2] as f32, w);
        assert_eq!(mask.rect[3] as f32, w);
        let h = w / 2.0;
        let (cx, cy) = (100.0, 80.0);
        let expected = [
            [cx - h, cy - h],
            [cx + h, cy - h],
            [cx + h, cy + h],
            [cx - h, cy + h],
        ];
        assert_eq!(positions_from(&ctx, vstart, 4), expected);
        let expected_uvs = [
            mask.uv_min,
            [mask.uv_max[0], mask.uv_min[1]],
            mask.uv_max,
            [mask.uv_min[0], mask.uv_max[1]],
        ];
        for (v, uv) in data.vertices[vstart..vend].iter().zip(expected_uvs.iter()) {
            assert_eq!(v.col, [9, 8, 7, 255]);
            assert_eq!(v.uv, *uv);
        }
        let base = vstart as u32;
        assert_eq!(
            data.indices[istart..iend],
            [base, base + 1, base + 2, base, base + 2, base + 3]
        );
        let mut offset = 0usize;
        let mut circle_cmds = 0;
        for cmd in data.commands {
            let next = offset + cmd.elem_count as usize;
            if next > istart && offset < iend {
                assert_eq!(cmd.texture, crate::draw::TextureId::FONT);
                circle_cmds += 1;
            }
            offset = next;
        }
        assert_eq!(circle_cmds, 1, "one merged FONT command covers the quad");
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn aa_circle_fill_oversized_radius_falls_back_to_fringe() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = true;
        ctx.new_frame(0.016);
        let (mut vstart, mut istart) = (0usize, 0usize);
        let (mut vend, mut iend) = (0usize, 0usize);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                vstart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                istart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
                ui.draw_circle_filled([400.0, 300.0], 300.0, [9, 8, 7, 255]);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                iend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
            });
        }
        let n = crate::draw::circle_segment_count(300.0);
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert_eq!(vend - vstart, (n + 1) + 4 * n);
        assert_eq!(iend - istart, 9 * n);
        let mut offset = 0usize;
        for cmd in data.commands {
            let next = offset + cmd.elem_count as usize;
            if next > istart && offset < iend {
                assert_eq!(cmd.texture, crate::draw::TextureId::WHITE);
            }
            offset = next;
        }
        let faded = data.vertices[vstart..vend]
            .iter()
            .filter(|v| v.col[3] == 0)
            .count();
        assert_eq!(faded, 2 * n);
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn aa_off_circle_fill_is_byte_identical_to_the_crisp_fan() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = false;
        ctx.new_frame(0.016);
        let (mut vstart, mut istart) = (0usize, 0usize);
        let (mut vend, mut iend) = (0usize, 0usize);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                vstart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                istart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
                ui.draw_circle_filled([100.0, 100.0], 40.0, [0, 0, 200, 255]);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                iend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
            });
        }
        let mut reference = crate::draw::DrawList::new();
        reference.add_filled_circle([100.0, 100.0], 40.0, [0, 0, 200, 255], false);
        let ref_data = reference.as_draw_data();
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert_eq!(vend - vstart, ref_data.vertices.len());
        assert_eq!(&data.vertices[vstart..vend], ref_data.vertices);
        assert_eq!(iend - istart, ref_data.indices.len());
        let base = vstart as u32;
        for (actual, expected) in data.indices[istart..iend]
            .iter()
            .zip(ref_data.indices.iter())
        {
            assert_eq!(*actual, expected + base);
        }
        ctx.render();
    }

    #[test]
    fn identical_circles_share_the_cached_mask() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = true;
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                ui.draw_circle_filled([100.0, 80.0], 20.0, [9, 8, 7, 255]);
            });
        }
        let gen_cached = ctx.font().atlas_generation();
        ctx.render();

        ctx.new_frame(0.016);
        let mut vstart = 0usize;
        let mut vend = 0usize;
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                vstart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_circle_filled([100.0, 80.0], 20.0, [9, 8, 7, 255]);
                ui.draw_circle_filled([200.0, 120.0], 20.0, [9, 8, 7, 255]);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
            });
        }
        assert_eq!(
            ctx.font().atlas_generation(),
            gen_cached,
            "cache hits must not touch the atlas"
        );
        assert_eq!(vend - vstart, 8);
        let h = mask_disk_dim(20.0) / 2.0;
        let mut expected = alloc::vec::Vec::new();
        for (cx, cy) in [(100.0, 80.0), (200.0, 120.0)] {
            expected.extend_from_slice(&[
                [cx - h, cy - h],
                [cx + h, cy - h],
                [cx + h, cy + h],
                [cx - h, cy + h],
            ]);
        }
        assert_eq!(positions_from(&ctx, vstart, 8), expected);
        ctx.render();
    }

    #[test]
    fn aa_circle_outline_emits_one_mask_quad() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = true;
        ctx.new_frame(0.016);
        let (mut vstart, mut istart) = (0usize, 0usize);
        let (mut vend, mut iend) = (0usize, 0usize);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                vstart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                istart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
                ui.draw_circle_outline([100.0, 80.0], 20.0, [9, 8, 7, 255], 4.0);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                iend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
            });
        }
        let mask = ctx
            .font
            .shape_mask(crate::font::ShapeKey::ring(20.0, 4.0))
            .expect("radius 20, thickness 4 fits the mask cap");
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert_eq!(vend - vstart, 4);
        assert_eq!(iend - istart, 6);
        let w = mask_ring_dim(20.0, 4.0);
        assert_eq!(w, 46.0, "radius 20, thickness 4: ceil(2·20 + 4) + 2");
        assert_eq!(mask.rect[2] as f32, w);
        assert_eq!(mask.rect[3] as f32, w);
        let h = w / 2.0;
        let (cx, cy) = (100.0, 80.0);
        let expected = [
            [cx - h, cy - h],
            [cx + h, cy - h],
            [cx + h, cy + h],
            [cx - h, cy + h],
        ];
        assert_eq!(positions_from(&ctx, vstart, 4), expected);
        let expected_uvs = [
            mask.uv_min,
            [mask.uv_max[0], mask.uv_min[1]],
            mask.uv_max,
            [mask.uv_min[0], mask.uv_max[1]],
        ];
        for (v, uv) in data.vertices[vstart..vend].iter().zip(expected_uvs.iter()) {
            assert_eq!(v.col, [9, 8, 7, 255]);
            assert_eq!(v.uv, *uv);
        }
        let base = vstart as u32;
        assert_eq!(
            data.indices[istart..iend],
            [base, base + 1, base + 2, base, base + 2, base + 3]
        );
        let mut offset = 0usize;
        let mut outline_cmds = 0;
        for cmd in data.commands {
            let next = offset + cmd.elem_count as usize;
            if next > istart && offset < iend {
                assert_eq!(cmd.texture, crate::draw::TextureId::FONT);
                outline_cmds += 1;
            }
            offset = next;
        }
        assert_eq!(outline_cmds, 1, "one merged FONT command covers the quad");
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn aa_circle_outline_oversized_falls_back_to_the_fringe_ring() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = true;
        ctx.new_frame(0.016);
        let (mut vstart, mut istart) = (0usize, 0usize);
        let (mut vend, mut iend) = (0usize, 0usize);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                vstart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                istart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
                ui.draw_circle_outline([400.0, 300.0], 300.0, [9, 8, 7, 255], 10.0);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                iend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
            });
        }
        let mut reference = crate::draw::DrawList::new();
        reference.add_circle_outline([400.0, 300.0], 300.0, [9, 8, 7, 255], 10.0, true);
        let ref_data = reference.as_draw_data();
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert_eq!(vend - vstart, ref_data.vertices.len());
        assert_eq!(&data.vertices[vstart..vend], ref_data.vertices);
        assert_eq!(iend - istart, ref_data.indices.len());
        let base = vstart as u32;
        for (actual, expected) in data.indices[istart..iend]
            .iter()
            .zip(ref_data.indices.iter())
        {
            assert_eq!(*actual, expected + base);
        }
        let mut offset = 0usize;
        for cmd in data.commands {
            let next = offset + cmd.elem_count as usize;
            if next > istart && offset < iend {
                assert_eq!(cmd.texture, crate::draw::TextureId::WHITE);
            }
            offset = next;
        }
        assert!(data.vertices[vstart..vend].iter().any(|v| v.col[3] == 0));
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn aa_off_circle_outline_is_byte_identical_to_the_crisp_ring() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = false;
        ctx.new_frame(0.016);
        let (mut vstart, mut istart) = (0usize, 0usize);
        let (mut vend, mut iend) = (0usize, 0usize);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                vstart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                istart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
                ui.draw_circle_outline([100.0, 100.0], 40.0, [0, 0, 200, 255], 3.0);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                iend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
            });
        }
        let mut reference = crate::draw::DrawList::new();
        reference.add_circle_outline([100.0, 100.0], 40.0, [0, 0, 200, 255], 3.0, false);
        let ref_data = reference.as_draw_data();
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert_eq!(vend - vstart, ref_data.vertices.len());
        assert_eq!(&data.vertices[vstart..vend], ref_data.vertices);
        assert_eq!(iend - istart, ref_data.indices.len());
        let base = vstart as u32;
        for (actual, expected) in data.indices[istart..iend]
            .iter()
            .zip(ref_data.indices.iter())
        {
            assert_eq!(*actual, expected + base);
        }
        ctx.render();
    }

    #[test]
    fn ring_and_disk_masks_are_distinct_cache_entries() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = true;
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                ui.draw_circle_outline([100.0, 80.0], 20.0, [9, 8, 7, 255], 4.0);
            });
        }
        let gen_ring = ctx.font().atlas_generation();
        ctx.render();

        ctx.new_frame(0.016);
        let mut vstart = 0usize;
        let mut vend = 0usize;
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                vstart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_circle_outline([100.0, 80.0], 20.0, [9, 8, 7, 255], 4.0);
                ui.draw_circle_outline([200.0, 120.0], 20.0, [9, 8, 7, 255], 4.0);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
            });
        }
        assert_eq!(
            ctx.font().atlas_generation(),
            gen_ring,
            "ring cache hits must not touch the atlas"
        );
        assert_eq!(vend - vstart, 8);
        ctx.render();

        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                ui.draw_circle_filled([100.0, 80.0], 20.0, [9, 8, 7, 255]);
            });
        }
        assert!(
            ctx.font().atlas_generation() > gen_ring,
            "disk(20) must not hit the ring(20, 4) entry"
        );
        ctx.render();
    }

    #[test]
    fn draw_arc_is_unaffected_by_the_mask_system() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = true;
        ctx.new_frame(0.016);
        let (mut vstart, mut istart) = (0usize, 0usize);
        let (mut vend, mut iend) = (0usize, 0usize);
        let (mut gen_before, mut gen_after) = (0u64, 0u64);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                vstart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                istart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
                gen_before = ui.ctx.font().atlas_generation();
                ui.draw_arc(
                    [100.0, 100.0],
                    40.0,
                    0.0..=core::f32::consts::FRAC_PI_2,
                    [1, 2, 3, 255],
                    3.0,
                );
                gen_after = ui.ctx.font().atlas_generation();
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                iend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
            });
        }
        assert_eq!(gen_before, gen_after);
        let mut reference = crate::draw::DrawList::new();
        reference.add_arc(
            [100.0, 100.0],
            40.0,
            0.0,
            core::f32::consts::FRAC_PI_2,
            [1, 2, 3, 255],
            3.0,
            true,
        );
        let ref_data = reference.as_draw_data();
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert_eq!(vend - vstart, ref_data.vertices.len());
        assert_eq!(&data.vertices[vstart..vend], ref_data.vertices);
        assert_eq!(iend - istart, ref_data.indices.len());
        let base = vstart as u32;
        for (actual, expected) in data.indices[istart..iend]
            .iter()
            .zip(ref_data.indices.iter())
        {
            assert_eq!(*actual, expected + base);
        }
        let mut offset = 0usize;
        for cmd in data.commands {
            let next = offset + cmd.elem_count as usize;
            if next > istart && offset < iend {
                assert_eq!(cmd.texture, crate::draw::TextureId::WHITE);
            }
            offset = next;
        }
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn full_range_arc_matches_circle_outline_through_ui() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = false;
        ctx.new_frame(0.016);
        let mut start = 0usize;
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                start = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_circle_outline([100.0, 100.0], 40.0, [9, 9, 9, 255], 3.0);
                ui.draw_arc(
                    [100.0, 100.0],
                    40.0,
                    0.0..=core::f32::consts::TAU,
                    [9, 9, 9, 255],
                    3.0,
                );
            });
        }
        let n = crate::draw::circle_segment_count(40.0 + 1.5);
        let positions = positions_from(&ctx, start, 4 * n);
        assert_eq!(positions.len(), 4 * n);
        assert_eq!(&positions[..2 * n], &positions[2 * n..]);
        assert_pos_near(positions[0], [100.0 + 40.0 + 1.5, 100.0]);
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn reversed_arc_range_is_normalized() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut start = 0usize;
        let mut vend = 0usize;
        let quarter = core::f32::consts::FRAC_PI_2;
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                start = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_arc([100.0, 100.0], 40.0, 0.0..=quarter, [1, 1, 1, 255], 2.0);
                ui.draw_arc([100.0, 100.0], 40.0, quarter..=0.0, [1, 1, 1, 255], 2.0);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
            });
        }
        let positions = positions_from(&ctx, start, vend - start);
        assert!(positions.len().is_multiple_of(2) && !positions.is_empty());
        let half = positions.len() / 2;
        assert_eq!(&positions[..half], &positions[half..]);
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn draw_polygon_filled_emits_fan_in_current_window() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = false;
        ctx.new_frame(0.016);
        let mut start = 0usize;
        let mut vend = 0usize;
        let mut istart = 0usize;
        let points = [[10.0, 10.0], [30.0, 10.0], [40.0, 30.0], [20.0, 40.0]];
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                start = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                istart = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .indices
                    .len();
                ui.draw_polygon_filled(&points, [1, 2, 3, 255]);
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
            });
        }
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert_eq!(vend - start, 4);
        for (v, p) in data.vertices[start..vend].iter().zip(points.iter()) {
            assert_eq!(v.pos, *p);
            assert_eq!(v.col, [1, 2, 3, 255]);
        }
        let base = start as u32;
        assert_eq!(
            data.indices[istart..istart + 6],
            [base, base + 1, base + 2, base, base + 2, base + 3]
        );
        let last_cmd = data.commands.last().expect("polygon must emit a command");
        assert_eq!(last_cmd.texture, crate::draw::TextureId::WHITE);
        assert_eq!(last_cmd.elem_count, 6 + 12 + 3);
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    #[test]
    fn draw_image_passes_host_texture_uvs_and_tint_through() {
        let mut ctx = test_context();
        ctx.new_frame(0.016);
        let mut start = 0usize;
        let mut vend = 0usize;
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                start = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_image(
                    crate::draw::TextureId(42),
                    [10.0, 20.0],
                    [110.0, 70.0],
                    [0.25, 0.5],
                    [0.75, 1.0],
                    [9, 8, 7, 255],
                );
                ui.draw_image(
                    crate::draw::TextureId(42),
                    [110.0, 70.0],
                    [10.0, 20.0],
                    [0.25, 0.5],
                    [0.75, 1.0],
                    [9, 8, 7, 255],
                );
                vend = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
            });
        }
        let data = ctx.windows[0].draw_list.as_draw_data();
        assert_eq!(vend - start, 8);
        let positions: alloc::vec::Vec<[f32; 2]> =
            data.vertices[start..vend].iter().map(|v| v.pos).collect();
        let quad = [[10.0, 20.0], [110.0, 20.0], [110.0, 70.0], [10.0, 70.0]];
        assert_eq!(&positions[..4], &quad);
        assert_eq!(&positions[4..], &quad, "reversed corners must normalize");
        let uvs: alloc::vec::Vec<[f32; 2]> =
            data.vertices[start..vend].iter().map(|v| v.uv).collect();
        let quad_uvs = [[0.25, 0.5], [0.75, 0.5], [0.75, 1.0], [0.25, 1.0]];
        assert_eq!(&uvs[..4], &quad_uvs);
        assert_eq!(&uvs[4..], &quad_uvs);
        for v in &data.vertices[start..vend] {
            assert_eq!(v.col, [9, 8, 7, 255]);
        }
        let img_cmd = data
            .commands
            .iter()
            .find(|c| c.texture == crate::draw::TextureId(42))
            .expect("image must emit a command");
        assert_eq!(img_cmd.elem_count, 12);
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    fn stroke_geometry_counts(aa: bool) -> (usize, usize) {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = aa;
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                ui.draw_line([10.0, 20.0], [30.0, 20.0], [255, 0, 0, 255], 4.0);
                ui.draw_rect_outline([10.0, 10.0], [30.0, 26.0], [255, 0, 0, 255], 2.0);
                ui.draw_arc([100.0, 100.0], 40.0, 0.0..=1.0, [255, 0, 0, 255], 2.0);
            });
        }
        let data = ctx.windows[0].draw_list.as_draw_data();
        let counts = (data.vertices.len(), data.indices.len());
        ctx.render();
        counts
    }

    #[test]
    fn style_aa_flag_gates_fringe_emission_for_canvas_strokes() {
        let (off_v, off_i) = stroke_geometry_counts(false);
        let (on_v, on_i) = stroke_geometry_counts(true);

        let n_full = crate::draw::circle_segment_count(40.0 + 1.0);
        let n_arc = ((n_full as f32 * 1.0 / core::f32::consts::TAU).ceil() as usize).max(1);
        let fringe_quads = 4 + 8 + 2 * n_arc + 2 + 4 + 3;
        assert_eq!(on_v - off_v, 4 * fringe_quads);
        assert_eq!(on_i - off_i, 6 * fringe_quads);
    }

    #[test]
    fn canvas_strokes_keep_their_body_byte_identical_with_aa_on() {
        let mut off = test_context();
        off.style.anti_aliasing = false;
        off.new_frame(0.016);
        let mut on = test_context();
        on.style.anti_aliasing = true;
        on.new_frame(0.016);
        {
            let mut ui = off.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                ui.draw_line([10.0, 20.0], [30.0, 20.0], [255, 0, 0, 255], 4.0);
                ui.draw_rect_outline([10.0, 10.0], [30.0, 26.0], [255, 0, 0, 255], 2.0);
                ui.draw_arc([100.0, 100.0], 40.0, 0.0..=1.0, [255, 0, 0, 255], 2.0);
            });
        }
        {
            let mut ui = on.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                ui.draw_line([10.0, 20.0], [30.0, 20.0], [255, 0, 0, 255], 4.0);
                ui.draw_rect_outline([10.0, 10.0], [30.0, 26.0], [255, 0, 0, 255], 2.0);
                ui.draw_arc([100.0, 100.0], 40.0, 0.0..=1.0, [255, 0, 0, 255], 2.0);
            });
        }
        let off_data = off.windows[0].draw_list.as_draw_data();
        let on_data = on.windows[0].draw_list.as_draw_data();
        assert!(on_data.vertices.len() > off_data.vertices.len());
        let mut it = on_data.vertices.iter();
        for v in off_data.vertices {
            assert!(
                it.by_ref().any(|w| w == v),
                "crisp vertex {v:?} missing from the AA emission"
            );
        }
        off.render();
        on.render();
    }

    #[test]
    fn aa_canvas_rect_fill_appends_fringe_outside_the_edges() {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = true;
        ctx.new_frame(0.016);
        let mut start = 0usize;
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                start = ui
                    .ctx
                    .current_window_mut()
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .len();
                ui.draw_rect_filled([10.0, 10.0], [30.0, 26.0], [9, 8, 7, 255]);
            });
        }
        let f = crate::draw::FRINGE_WIDTH;
        let quads = quads_from(&ctx, start, 5);
        let expected = [
            [[10.0, 10.0], [30.0, 10.0], [30.0, 26.0], [10.0, 26.0]],
            [
                [10.0, 10.0],
                [10.0 - f, 10.0 - f],
                [30.0 + f, 10.0 - f],
                [30.0, 10.0],
            ],
            [
                [30.0, 10.0],
                [30.0 + f, 10.0 - f],
                [30.0 + f, 26.0 + f],
                [30.0, 26.0],
            ],
            [
                [30.0, 26.0],
                [30.0 + f, 26.0 + f],
                [10.0 - f, 26.0 + f],
                [10.0, 26.0],
            ],
            [
                [10.0, 26.0],
                [10.0 - f, 26.0 + f],
                [10.0 - f, 10.0 - f],
                [10.0, 10.0],
            ],
        ];
        assert_eq!(quads, expected);
        let data = ctx.windows[0].draw_list.as_draw_data();
        for quad in data.vertices[start + 4..start + 20].chunks_exact(4) {
            assert_eq!(quad[0].col, [9, 8, 7, 255]);
            assert_eq!(quad[1].col, [9, 8, 7, 0]);
            assert_eq!(quad[2].col, [9, 8, 7, 0]);
            assert_eq!(quad[3].col, [9, 8, 7, 255]);
        }
        assert_all_commands_clipped_to_window(&ctx);
        ctx.render();
    }

    fn fill_geometry_counts(aa: bool) -> (usize, usize) {
        let mut ctx = test_context();
        ctx.style.anti_aliasing = aa;
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| {
                ui.draw_rect_filled([10.0, 10.0], [30.0, 26.0], [255, 0, 0, 255]);
                ui.draw_circle_filled([400.0, 300.0], 300.0, [255, 0, 0, 255]);
                ui.draw_polygon_filled(
                    &[[10.0, 10.0], [30.0, 10.0], [40.0, 30.0], [20.0, 40.0]],
                    [255, 0, 0, 255],
                );
            });
        }
        let data = ctx.windows[0].draw_list.as_draw_data();
        let counts = (data.vertices.len(), data.indices.len());
        ctx.render();
        counts
    }

    #[test]
    fn style_aa_flag_gates_fringe_emission_for_canvas_fills() {
        let (off_v, off_i) = fill_geometry_counts(false);
        let (on_v, on_i) = fill_geometry_counts(true);

        let n_circle = crate::draw::circle_segment_count(300.0);
        let fringe_quads = 4 + n_circle + 4 + 4 + 3;
        assert_eq!(on_v - off_v, 4 * fringe_quads);
        assert_eq!(on_i - off_i, 6 * fringe_quads);
    }

    #[test]
    fn canvas_fills_keep_their_body_byte_identical_with_aa_on() {
        let mut off = test_context();
        off.style.anti_aliasing = false;
        off.new_frame(0.016);
        let mut on = test_context();
        on.style.anti_aliasing = true;
        on.new_frame(0.016);
        let draws = |ui: &mut super::Ui| {
            ui.draw_rect_filled([10.0, 10.0], [30.0, 26.0], [255, 0, 0, 255]);
            ui.draw_circle_filled([400.0, 300.0], 300.0, [255, 0, 0, 255]);
            ui.draw_polygon_filled(
                &[[10.0, 10.0], [30.0, 10.0], [40.0, 30.0], [20.0, 40.0]],
                [255, 0, 0, 255],
            );
        };
        {
            let mut ui = off.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| draws(ui));
        }
        {
            let mut ui = on.ui();
            ui.window("Canvas", ScrollFlags::NONE, |ui| draws(ui));
        }
        let off_data = off.windows[0].draw_list.as_draw_data();
        let on_data = on.windows[0].draw_list.as_draw_data();
        assert!(on_data.vertices.len() > off_data.vertices.len());
        let mut it = on_data.vertices.iter();
        for v in off_data.vertices {
            assert!(
                it.by_ref().any(|w| w == v),
                "crisp vertex {v:?} missing from the AA emission"
            );
        }
        off.render();
        on.render();
    }

    fn popup_frame(ctx: &mut Context, input: InputState<'_>, f: impl FnOnce(&mut super::Ui)) {
        ctx.set_input(input);
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("W", ScrollFlags::NONE, f);
        }
        ctx.render();
    }

    fn window_vertex_total(ctx: &Context) -> usize {
        ctx.windows
            .iter()
            .map(|w| w.draw_list.as_draw_data().vertices.len())
            .sum()
    }

    fn window_index_total(ctx: &Context) -> usize {
        ctx.windows
            .iter()
            .map(|w| w.draw_list.as_draw_data().indices.len())
            .sum()
    }

    fn inside_rect(v: &crate::draw::DrawVert, rect: [f32; 4]) -> bool {
        v.pos[0] >= rect[0] && v.pos[0] <= rect[2] && v.pos[1] >= rect[1] && v.pos[1] <= rect[3]
    }

    #[test]
    fn overlay_geometry_follows_all_windows_in_draw_data() {
        let mut ctx = test_context();
        let popup_rect = [400.0, 300.0, 560.0, 460.0];
        ctx.set_input(InputState::default());
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("A", ScrollFlags::NONE, |_| {});
            ui.window("B", ScrollFlags::NONE, |ui| {
                let popup_id = ui.widget_id("##popup");
                ui.open_popup(popup_id);
                ui.popup(popup_id, popup_rect, |_| {});
            });
        }
        let window_vertices = window_vertex_total(&ctx);
        let window_indices = window_index_total(&ctx);
        assert!(window_vertices > 0 && window_indices > 0);

        let data = ctx.render();
        assert!(
            data.vertices.len() > window_vertices,
            "the overlay must contribute geometry after both windows"
        );
        assert!(
            data.vertices[window_vertices..]
                .iter()
                .all(|v| inside_rect(v, popup_rect)),
            "overlay vertices must be the popup's, appended after all windows"
        );
        assert!(
            data.indices[window_indices..]
                .iter()
                .all(|&i| i as usize >= window_vertices),
            "overlay indices must be offset by the windows' vertex count"
        );
    }

    #[test]
    fn overlay_commands_clip_to_the_viewport_when_known() {
        let mut ctx = test_context();
        let viewport = [800.0, 600.0];
        ctx.set_input(InputState {
            viewport_size: viewport,
            ..Default::default()
        });
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("W", ScrollFlags::NONE, |ui| {
                let popup_id = ui.widget_id("##popup");
                ui.open_popup(popup_id);
                ui.popup(popup_id, [400.0, 300.0, 560.0, 460.0], |_| {});
            });
        }
        let data = ctx.render();
        let last = data.commands.last().expect("popup chrome must emit");
        assert_eq!(last.clip_rect, [0.0, 0.0, viewport[0], viewport[1]]);
    }

    #[test]
    fn press_outside_popup_closes_it_and_the_press_is_consumed() {
        let mut ctx = test_context();
        let popup_rect = [400.0, 300.0, 560.0, 460.0];
        let mut button_rect = [0.0; 4];
        let mut clicked = true;

        popup_frame(&mut ctx, InputState::default(), |ui| {
            button_rect = crate::widgets::test_support::next_text_rect(ui, "OK");
            clicked = ui.button("OK");
            let popup_id = ui.widget_id("##popup");
            ui.open_popup(popup_id);
            ui.popup(popup_id, popup_rect, |_| {});
        });
        assert!(!clicked);
        assert!(ctx.open_popup.is_some());
        let press = [
            (button_rect[0] + button_rect[2]) / 2.0,
            (button_rect[1] + button_rect[3]) / 2.0,
        ];
        assert!(
            !super::point_in_rect(press, popup_rect),
            "the button must be outside the popup rect for this test"
        );

        popup_frame(
            &mut ctx,
            InputState {
                mouse_pos: press,
                mouse_down: [true, false, false, false, false],
                ..Default::default()
            },
            |ui| {
                clicked = ui.button("OK");
                let popup_id = ui.widget_id("##popup");
                ui.popup(popup_id, popup_rect, |_| {});
            },
        );
        assert!(!clicked, "the press frame never reports a click");
        assert_eq!(
            ctx.open_popup, None,
            "an outside press must close the popup"
        );
        assert_eq!(
            ctx.active, None,
            "the consumed press must not capture the button underneath"
        );

        popup_frame(
            &mut ctx,
            InputState {
                mouse_pos: press,
                ..Default::default()
            },
            |ui| {
                clicked = ui.button("OK");
            },
        );
        assert!(
            !clicked,
            "a button pressed under an outside-press must not fire"
        );
    }

    #[test]
    fn escape_press_closes_the_popup() {
        let mut ctx = test_context();
        let popup_rect = [400.0, 300.0, 560.0, 460.0];
        popup_frame(&mut ctx, InputState::default(), |ui| {
            let popup_id = ui.widget_id("##popup");
            ui.open_popup(popup_id);
            ui.popup(popup_id, popup_rect, |_| {});
        });
        assert!(ctx.open_popup.is_some());

        popup_frame(
            &mut ctx,
            InputState {
                keys: &[KeyEvent::Press(Key::Escape)],
                ..Default::default()
            },
            |ui| {
                let popup_id = ui.widget_id("##popup");
                assert!(
                    !ui.is_popup_open(popup_id),
                    "Escape must close the popup before declarations run"
                );
                ui.popup(popup_id, popup_rect, |_| {});
            },
        );
        assert_eq!(ctx.open_popup, None);
    }

    #[test]
    fn opening_popup_b_while_a_is_open_replaces_a() {
        let mut ctx = test_context();
        let rect_a = [400.0, 300.0, 500.0, 400.0];
        let rect_b = [520.0, 320.0, 640.0, 440.0];
        popup_frame(&mut ctx, InputState::default(), |ui| {
            let a = ui.widget_id("##a");
            let b = ui.widget_id("##b");
            ui.open_popup(a);
            ui.popup(a, rect_a, |_| {});
            assert!(ui.is_popup_open(a));
            ui.open_popup(b);
            assert!(!ui.is_popup_open(a), "opening B must replace A");
            assert!(ui.is_popup_open(b));
            ui.popup(b, rect_b, |_| {});
        });

        ctx.set_input(InputState::default());
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("W", ScrollFlags::NONE, |ui| {
                let a = ui.widget_id("##a");
                let b = ui.widget_id("##b");
                ui.popup(a, rect_a, |_| {});
                ui.popup(b, rect_b, |_| {});
            });
        }
        let window_vertices = window_vertex_total(&ctx);
        let data = ctx.render();
        assert!(data.vertices.len() > window_vertices);
        assert!(
            data.vertices[window_vertices..]
                .iter()
                .all(|v| inside_rect(v, rect_b)),
            "with A replaced, the overlay must hold B's geometry only"
        );
    }

    #[test]
    fn close_popup_closes_the_open_popup_immediately() {
        let mut ctx = test_context();
        let popup_rect = [400.0, 300.0, 560.0, 460.0];
        popup_frame(&mut ctx, InputState::default(), |ui| {
            let popup_id = ui.widget_id("##popup");
            ui.open_popup(popup_id);
            ui.popup(popup_id, popup_rect, |ui| {
                ui.close_popup();
            });
            assert!(
                !ui.is_popup_open(popup_id),
                "close_popup takes effect within the same frame"
            );
        });
        assert_eq!(ctx.open_popup, None);
        assert_eq!(ctx.popup_rect, None);

        ctx.set_input(InputState::default());
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            ui.window("W", ScrollFlags::NONE, |ui| {
                let popup_id = ui.widget_id("##popup");
                ui.popup(popup_id, popup_rect, |_| {});
            });
        }
        let window_vertices = window_vertex_total(&ctx);
        let data = ctx.render();
        assert_eq!(data.vertices.len(), window_vertices);
    }

    #[test]
    fn canvas_primitives_inside_popup_emit_into_the_overlay_with_popup_clip() {
        let mut ctx = test_context();
        let popup_rect = [400.0, 300.0, 560.0, 460.0];
        let rect_min = [420.0, 320.0];
        let rect_max = [480.0, 380.0];
        popup_frame(&mut ctx, InputState::default(), |ui| {
            let popup_id = ui.widget_id("##popup");
            ui.open_popup(popup_id);
            ui.popup(popup_id, popup_rect, |ui| {
                ui.draw_rect_filled(rect_min, rect_max, [255, 0, 0, 255]);
            });
        });
        let overlay = ctx.overlay.as_draw_data();
        let in_overlay = overlay
            .vertices
            .windows(4)
            .any(|q| q[0].pos == rect_min && q[2].pos == rect_max);
        assert!(in_overlay, "the canvas rect must emit into the overlay");
        assert!(
            overlay.commands.iter().any(|c| c.clip_rect == popup_rect),
            "canvas commands in a popup must carry the popup rect clip"
        );
        let window = ctx.windows[0].draw_list.as_draw_data();
        let in_window = window
            .vertices
            .windows(4)
            .any(|q| q[0].pos == rect_min && q[2].pos == rect_max);
        assert!(
            !in_window,
            "canvas geometry must not leak into the window list"
        );
    }

    #[test]
    fn popup_scope_interleaving_leaves_window_cursor_and_list_untouched() {
        let mut ctx = test_context();
        let popup_rect = [400.0, 300.0, 560.0, 460.0];
        let mut rect_a = [0.0; 4];
        let mut rect_b = [0.0; 4];
        popup_frame(&mut ctx, InputState::default(), |ui| {
            rect_a = crate::widgets::test_support::next_text_rect(ui, "A");
            ui.button("A");
            let popup_id = ui.widget_id("##popup");
            ui.open_popup(popup_id);
            ui.popup(popup_id, popup_rect, |ui| {
                ui.button("P");
            });
            rect_b = crate::widgets::test_support::next_text_rect(ui, "B");
            ui.button("B");
        });
        assert_eq!(rect_b[0], rect_a[0]);
        assert_eq!(rect_b[1], rect_a[3] + ctx.style.item_spacing[1]);
        assert_eq!(
            crate::widgets::test_support::fill_color_at(&ctx, rect_b),
            Some(ctx.style.widget_bg)
        );
        let overlay = ctx.overlay.as_draw_data();
        assert!(!overlay.vertices.is_empty());
        assert!(overlay.vertices.iter().all(|v| inside_rect(v, popup_rect)));
    }

    #[test]
    fn button_inside_popup_clicks_with_normal_rules() {
        let mut ctx = test_context();
        let popup_rect = [400.0, 300.0, 560.0, 460.0];
        let mut item_rect = [0.0; 4];
        let mut clicked = true;
        let mut popup_id = crate::id::Id(0);

        popup_frame(&mut ctx, InputState::default(), |ui| {
            popup_id = ui.widget_id("##popup");
            ui.open_popup(popup_id);
            ui.popup(popup_id, popup_rect, |ui| {
                let pos = ui.ctx.overlay_cursor.next_pos();
                let style = ui.ctx.style;
                let text = ui.ctx.font.measure("Item");
                let size = [
                    text[0] + 2.0 * style.frame_padding[0],
                    text[1] + 2.0 * style.frame_padding[1],
                ];
                item_rect = [pos[0], pos[1], pos[0] + size[0], pos[1] + size[1]];
                clicked = ui.button("Item");
            });
        });
        assert!(!clicked);
        let at = [
            (item_rect[0] + item_rect[2]) / 2.0,
            (item_rect[1] + item_rect[3]) / 2.0,
        ];
        assert!(super::point_in_rect(at, popup_rect));

        let scoped_item_id = {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            stack.push_id(popup_id);
            stack.id("Item")
        };

        popup_frame(
            &mut ctx,
            InputState {
                mouse_pos: at,
                mouse_down: [true, false, false, false, false],
                ..Default::default()
            },
            |ui| {
                let popup_id = ui.widget_id("##popup");
                ui.popup(popup_id, popup_rect, |ui| {
                    clicked = ui.button("Item");
                });
            },
        );
        assert!(!clicked, "the press frame must not report a click");
        assert!(
            ctx.open_popup.is_some(),
            "a press inside the popup rect must keep the popup open"
        );
        assert_eq!(ctx.active, Some(scoped_item_id));

        popup_frame(
            &mut ctx,
            InputState {
                mouse_pos: at,
                ..Default::default()
            },
            |ui| {
                let popup_id = ui.widget_id("##popup");
                ui.popup(popup_id, popup_rect, |ui| {
                    clicked = ui.button("Item");
                });
            },
        );
        assert!(clicked, "release inside the popup button must click");
        assert_eq!(ctx.active, None);
    }

    mod tab_bar {
        use crate::context::{Context, WidgetState};
        use crate::ui::Ui;
        use crate::widgets::test_support::*;
        use crate::widgets::{lighten, BORDER_LIGHTEN};

        const AWAY: [f32; 2] = [-50.0, -50.0];

        const MARK_A: ([f32; 4], [u8; 4]) = ([100.0, 400.0, 110.0, 410.0], [9, 8, 7, 255]);
        const MARK_B: ([f32; 4], [u8; 4]) = ([120.0, 400.0, 130.0, 410.0], [6, 5, 4, 255]);
        const MARK_A0: ([f32; 4], [u8; 4]) = ([100.0, 420.0, 110.0, 430.0], [1, 1, 1, 255]);
        const MARK_A1: ([f32; 4], [u8; 4]) = ([120.0, 420.0, 130.0, 430.0], [2, 2, 2, 255]);
        const MARK_B0: ([f32; 4], [u8; 4]) = ([140.0, 420.0, 150.0, 430.0], [3, 3, 3, 255]);
        const MARK_B1: ([f32; 4], [u8; 4]) = ([160.0, 420.0, 170.0, 430.0], [4, 4, 4, 255]);

        fn mark(ui: &mut Ui, m: ([f32; 4], [u8; 4])) {
            ui.draw_rect_filled([m.0[0], m.0[1]], [m.0[2], m.0[3]], m.1);
        }

        fn mark_color(ctx: &Context, m: ([f32; 4], [u8; 4])) -> Option<[u8; 4]> {
            fill_color_at(ctx, m.0)
        }

        fn declare_two_tabs(ui: &mut Ui) {
            ui.tab_bar("Tabs", |tabs| {
                tabs.tab("Alpha", |ui| mark(ui, MARK_A));
                tabs.tab("Beta", |ui| mark(ui, MARK_B));
            });
        }

        fn header_rect(ui: &mut Ui, prev: Option<[f32; 4]>, title: &str) -> [f32; 4] {
            match prev {
                None => next_text_rect(ui, title),
                Some(p) => {
                    let style = ui.ctx.style;
                    let w = ui.ctx.font.measure(title)[0] + 2.0 * style.frame_padding[0];
                    let x = p[2] + style.item_spacing[0];
                    [x, p[1], x + w, p[3]]
                }
            }
        }

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn settle_two_tabs(ctx: &mut Context) -> ([f32; 4], [f32; 4], [f32; 4]) {
            let mut alpha = [0.0; 4];
            let mut beta = [0.0; 4];
            let mut row = [0.0; 4];
            frame(ctx, AWAY, false, |ui| {
                alpha = header_rect(ui, None, "Alpha");
                beta = header_rect(ui, Some(alpha), "Beta");
                let width = ui.ctx.current_window_mut().cursor.available_width();
                row = [alpha[0], alpha[1], alpha[0] + width, alpha[3]];
                declare_two_tabs(ui);
            });
            (alpha, beta, row)
        }

        fn active_index(ctx: &Context, label: &str) -> Option<usize> {
            match ctx.widget_states.get(&widget_id_in_w(label)) {
                Some(WidgetState::TabBar { active }) => Some(*active),
                _ => None,
            }
        }

        #[test]
        fn first_tab_is_active_by_default_and_only_its_closure_runs() {
            let mut ctx = test_context();
            frame(&mut ctx, AWAY, false, declare_two_tabs);
            assert_eq!(mark_color(&ctx, MARK_A), Some(MARK_A.1));
            assert_eq!(mark_color(&ctx, MARK_B), None);
            assert_eq!(active_index(&ctx, "Tabs"), Some(0));
        }

        #[test]
        fn clicking_a_header_switches_the_active_tab_next_frame_and_persists() {
            let mut ctx = test_context();
            let (_, beta, _) = settle_two_tabs(&mut ctx);
            let at = center(beta);

            frame(&mut ctx, at, true, declare_two_tabs);
            frame(&mut ctx, at, false, declare_two_tabs);
            assert_eq!(mark_color(&ctx, MARK_A), Some(MARK_A.1));
            assert_eq!(mark_color(&ctx, MARK_B), None);
            assert_eq!(active_index(&ctx, "Tabs"), Some(1));

            frame(&mut ctx, AWAY, false, declare_two_tabs);
            assert_eq!(mark_color(&ctx, MARK_B), Some(MARK_B.1));
            assert_eq!(mark_color(&ctx, MARK_A), None);

            frame(&mut ctx, AWAY, false, declare_two_tabs);
            assert_eq!(mark_color(&ctx, MARK_B), Some(MARK_B.1));
            assert_eq!(active_index(&ctx, "Tabs"), Some(1));
        }

        #[test]
        fn tab_bars_with_same_tab_titles_do_not_share_state() {
            let mut ctx = test_context();
            let declare = |ui: &mut Ui, record: Option<(&mut [f32; 4], &mut [f32; 4])>| {
                ui.tab_bar("Tabs##a", |tabs| {
                    tabs.tab("One", |ui| mark(ui, MARK_A0));
                    tabs.tab("Two", |ui| mark(ui, MARK_A1));
                });
                let one = header_rect(ui, None, "One");
                let two = header_rect(ui, Some(one), "Two");
                if let Some((r_one, r_two)) = record {
                    *r_one = one;
                    *r_two = two;
                }
                ui.tab_bar("Tabs##b", |tabs| {
                    tabs.tab("One", |ui| mark(ui, MARK_B0));
                    tabs.tab("Two", |ui| mark(ui, MARK_B1));
                });
            };

            let mut b_one = [0.0; 4];
            let mut b_two = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                declare(ui, Some((&mut b_one, &mut b_two)));
            });
            let _ = b_one;

            let at = center(b_two);
            frame(&mut ctx, at, true, |ui| declare(ui, None));
            frame(&mut ctx, at, false, |ui| declare(ui, None));
            frame(&mut ctx, AWAY, false, |ui| declare(ui, None));

            assert_eq!(mark_color(&ctx, MARK_A0), Some(MARK_A0.1));
            assert_eq!(mark_color(&ctx, MARK_A1), None);
            assert_eq!(mark_color(&ctx, MARK_B0), None);
            assert_eq!(mark_color(&ctx, MARK_B1), Some(MARK_B1.1));
            assert_eq!(active_index(&ctx, "Tabs##a"), Some(0));
            assert_eq!(active_index(&ctx, "Tabs##b"), Some(1));
        }

        #[test]
        fn shrinking_the_tab_count_clamps_the_persisted_index() {
            let mut ctx = test_context();
            let (_, beta, _) = settle_two_tabs(&mut ctx);
            let at = center(beta);
            frame(&mut ctx, at, true, declare_two_tabs);
            frame(&mut ctx, at, false, declare_two_tabs);
            assert_eq!(active_index(&ctx, "Tabs"), Some(1));
            let one_tab = |ui: &mut Ui| {
                ui.tab_bar("Tabs", |tabs| {
                    tabs.tab("Alpha", |ui| mark(ui, MARK_A));
                });
            };

            frame(&mut ctx, AWAY, false, one_tab);
            assert_eq!(mark_color(&ctx, MARK_A), None);
            assert_eq!(active_index(&ctx, "Tabs"), Some(0));

            frame(&mut ctx, AWAY, false, one_tab);
            assert_eq!(mark_color(&ctx, MARK_A), Some(MARK_A.1));
        }

        #[test]
        fn headers_emit_title_text_and_active_inactive_styling_differs() {
            let mut ctx = test_context();
            let (alpha, beta, row) = settle_two_tabs(&mut ctx);
            let style = ctx.style;

            assert_eq!(fill_color_at(&ctx, alpha), Some(style.widget_bg_active));
            assert_eq!(fill_color_at(&ctx, beta), Some(style.widget_bg));

            for rect in [alpha, beta] {
                let glyphs = glyph_colors_in(&ctx, rect);
                assert!(!glyphs.is_empty(), "tab headers must emit their title");
                assert!(glyphs.iter().all(|&c| c == style.text));
            }

            let t = style.border_size;
            let separator = [row[0], row[3] - t, row[2], row[3]];
            assert_eq!(
                fill_color_at(&ctx, separator),
                Some(lighten(style.widget_bg, BORDER_LIGHTEN))
            );
            let alpha_bottom = [alpha[0], alpha[3] - t, alpha[2], alpha[3]];
            let beta_bottom = [beta[0], beta[3] - t, beta[2], beta[3]];
            assert_eq!(fill_color_at(&ctx, alpha_bottom), None);
            assert_eq!(
                fill_color_at(&ctx, beta_bottom),
                Some(lighten(style.widget_bg, BORDER_LIGHTEN))
            );
        }

        mod popup_scope {
            use super::super::{inside_rect, popup_frame};
            use super::*;
            use crate::input::InputState;

            const POPUP_RECT: [f32; 4] = [400.0, 300.0, 560.0, 460.0];
            const MARK_PA: ([f32; 4], [u8; 4]) = ([420.0, 430.0, 430.0, 440.0], [9, 9, 9, 255]);
            const MARK_PB: ([f32; 4], [u8; 4]) = ([440.0, 430.0, 450.0, 440.0], [8, 8, 8, 255]);

            fn declare_tabs(ui: &mut Ui) {
                ui.tab_bar("Tabs", |tabs| {
                    tabs.tab("Alpha", |ui| mark(ui, MARK_PA));
                    tabs.tab("Beta", |ui| mark(ui, MARK_PB));
                });
            }

            fn popup_header_rect(ui: &mut Ui, prev: Option<[f32; 4]>, title: &str) -> [f32; 4] {
                let style = ui.ctx.style;
                let h = ui.ctx.font.line_height() + 2.0 * style.frame_padding[1];
                let w = ui.ctx.font.measure(title)[0] + 2.0 * style.frame_padding[0];
                match prev {
                    None => {
                        let pos = ui.ctx.overlay_cursor.next_pos();
                        [pos[0], pos[1], pos[0] + w, pos[1] + h]
                    }
                    Some(p) => {
                        let x = p[2] + style.item_spacing[0];
                        [x, p[1], x + w, p[3]]
                    }
                }
            }

            fn settle(ctx: &mut Context) -> (crate::id::Id, [f32; 4], [f32; 4]) {
                let mut popup = crate::id::Id(0);
                let mut alpha = [0.0; 4];
                let mut beta = [0.0; 4];
                popup_frame(ctx, InputState::default(), |ui| {
                    popup = ui.widget_id("##popup");
                    ui.open_popup(popup);
                    ui.popup(popup, POPUP_RECT, |ui| {
                        alpha = popup_header_rect(ui, None, "Alpha");
                        beta = popup_header_rect(ui, Some(alpha), "Beta");
                        declare_tabs(ui);
                    });
                });
                (popup, alpha, beta)
            }

            fn frame_tabs(ctx: &mut Context, popup: crate::id::Id, mouse: [f32; 2], down: bool) {
                popup_frame(
                    ctx,
                    InputState {
                        mouse_pos: mouse,
                        mouse_down: [down, false, false, false, false],
                        ..Default::default()
                    },
                    |ui| {
                        ui.popup(popup, POPUP_RECT, declare_tabs);
                    },
                );
            }

            fn overlay_fill_color_at(ctx: &Context, rect: [f32; 4]) -> Option<[u8; 4]> {
                crate::widgets::test_support::chrome_color_in(ctx.overlay.as_draw_data(), rect)
            }

            fn overlay_mark(ctx: &Context, m: ([f32; 4], [u8; 4])) -> bool {
                ctx.overlay
                    .as_draw_data()
                    .vertices
                    .windows(4)
                    .any(|q| q[0].pos == [m.0[0], m.0[1]] && q[2].pos == [m.0[2], m.0[3]])
            }

            fn window_mark(ctx: &Context, m: ([f32; 4], [u8; 4])) -> bool {
                ctx.windows[0]
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .windows(4)
                    .any(|q| q[0].pos == [m.0[0], m.0[1]] && q[2].pos == [m.0[2], m.0[3]])
            }

            fn popup_active_index(
                ctx: &Context,
                popup: crate::id::Id,
                label: &str,
            ) -> Option<usize> {
                let mut stack = crate::id::IdStack::new();
                stack.push("W");
                stack.push_id(popup);
                match ctx.widget_states.get(&stack.id(label)) {
                    Some(WidgetState::TabBar { active }) => Some(*active),
                    _ => None,
                }
            }

            #[test]
            fn headers_and_content_render_in_the_overlay_with_the_popup_clip() {
                let mut ctx = test_context();
                let (_, alpha, beta) = settle(&mut ctx);
                let style = ctx.style;

                assert_eq!(
                    overlay_fill_color_at(&ctx, alpha),
                    Some(style.widget_bg_active)
                );
                assert_eq!(overlay_fill_color_at(&ctx, beta), Some(style.widget_bg));
                assert_eq!(fill_color_at(&ctx, alpha), None);
                assert_eq!(fill_color_at(&ctx, beta), None);

                assert!(overlay_mark(&ctx, MARK_PA));
                assert!(!overlay_mark(&ctx, MARK_PB));
                assert!(!window_mark(&ctx, MARK_PA));

                let overlay = ctx.overlay.as_draw_data();
                assert!(
                    overlay.commands.iter().any(|c| c.clip_rect == POPUP_RECT),
                    "tab bar content in a popup must carry the popup rect clip"
                );
                assert!(overlay.vertices.iter().all(|v| inside_rect(v, POPUP_RECT)));
            }

            #[test]
            fn clicking_a_header_switches_the_tab_next_frame_and_persists() {
                let mut ctx = test_context();
                let (popup, _, beta) = settle(&mut ctx);
                assert_eq!(popup_active_index(&ctx, popup, "Tabs"), Some(0));

                let at = center(beta);
                frame_tabs(&mut ctx, popup, at, true);
                frame_tabs(&mut ctx, popup, at, false);
                assert!(ctx.open_popup.is_some());
                assert!(overlay_mark(&ctx, MARK_PA));
                assert!(!overlay_mark(&ctx, MARK_PB));
                assert_eq!(popup_active_index(&ctx, popup, "Tabs"), Some(1));

                frame_tabs(&mut ctx, popup, AWAY, false);
                assert!(overlay_mark(&ctx, MARK_PB));
                assert!(!overlay_mark(&ctx, MARK_PA));
                assert!(!window_mark(&ctx, MARK_PB));

                frame_tabs(&mut ctx, popup, AWAY, false);
                assert!(overlay_mark(&ctx, MARK_PB));
                assert_eq!(popup_active_index(&ctx, popup, "Tabs"), Some(1));
            }
        }
    }

    mod collapsing_header {
        use crate::context::{Context, WidgetState};
        use crate::input::InputState;
        use crate::scroll::ScrollFlags;
        use crate::ui::Ui;
        use crate::widgets::test_support::*;

        const AWAY: [f32; 2] = [-50.0, -50.0];

        const MARK: ([f32; 4], [u8; 4]) = ([100.0, 400.0, 110.0, 410.0], [9, 8, 7, 255]);
        const MARK_W1: ([f32; 4], [u8; 4]) = ([100.0, 400.0, 110.0, 410.0], [9, 8, 7, 255]);
        const MARK_W2: ([f32; 4], [u8; 4]) = ([200.0, 430.0, 210.0, 440.0], [6, 5, 4, 255]);

        fn mark(ui: &mut Ui, m: ([f32; 4], [u8; 4])) {
            ui.draw_rect_filled([m.0[0], m.0[1]], [m.0[2], m.0[3]], m.1);
        }

        fn mark_color(ctx: &Context, m: ([f32; 4], [u8; 4])) -> Option<[u8; 4]> {
            fill_color_at(ctx, m.0)
        }

        fn declare(ui: &mut Ui) {
            ui.collapsing_header("Settings", |ui| mark(ui, MARK));
        }

        fn header_row(ui: &mut Ui) -> [f32; 4] {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let style = ui.ctx.style;
            let h = ui.ctx.font.line_height() + 2.0 * style.frame_padding[1];
            let w = ui.ctx.current_window_mut().cursor.available_width();
            [pos[0], pos[1], pos[0] + w, pos[1] + h]
        }

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn open_state(ctx: &Context, window: &str, label: &str) -> Option<bool> {
            let mut stack = crate::id::IdStack::new();
            stack.push(window);
            match ctx.widget_states.get(&stack.id(label)) {
                Some(WidgetState::CollapsingHeader { open }) => Some(*open),
                _ => None,
            }
        }

        fn settle(ctx: &mut Context) -> [f32; 4] {
            let mut row = [0.0; 4];
            frame(ctx, AWAY, false, |ui| {
                row = header_row(ui);
                declare(ui);
            });
            row
        }

        fn click_open(ctx: &mut Context, row: [f32; 4]) {
            let at = center(row);
            frame(ctx, at, true, declare);
            frame(ctx, at, false, declare);
        }

        #[test]
        fn closed_by_default_and_closure_does_not_run() {
            let mut ctx = test_context();
            frame(&mut ctx, AWAY, false, declare);
            frame(&mut ctx, AWAY, false, declare);
            assert_eq!(mark_color(&ctx, MARK), None);
            assert_eq!(open_state(&ctx, "W", "Settings"), Some(false));
        }

        #[test]
        fn click_opens_next_frame_and_second_click_closes() {
            let mut ctx = test_context();
            let row = settle(&mut ctx);
            click_open(&mut ctx, row);

            assert_eq!(mark_color(&ctx, MARK), None);
            assert_eq!(open_state(&ctx, "W", "Settings"), Some(true));

            frame(&mut ctx, AWAY, false, declare);
            assert_eq!(mark_color(&ctx, MARK), Some(MARK.1));

            let at = center(row);
            frame(&mut ctx, at, true, declare);
            frame(&mut ctx, at, false, declare);
            assert_eq!(open_state(&ctx, "W", "Settings"), Some(false));
            frame(&mut ctx, AWAY, false, declare);
            assert_eq!(mark_color(&ctx, MARK), None);
        }

        #[test]
        fn open_state_persists_across_frames_without_interaction() {
            let mut ctx = test_context();
            let row = settle(&mut ctx);
            click_open(&mut ctx, row);
            for _ in 0..3 {
                frame(&mut ctx, AWAY, false, declare);
                assert_eq!(mark_color(&ctx, MARK), Some(MARK.1));
                assert_eq!(open_state(&ctx, "W", "Settings"), Some(true));
            }
        }

        fn verts_in(ctx: &Context, region: [f32; 4]) -> alloc::vec::Vec<[f32; 2]> {
            let data = ctx.windows[0].draw_list.as_draw_data();
            data.vertices
                .iter()
                .map(|v| v.pos)
                .filter(|p| {
                    p[0] > region[0] && p[0] < region[2] && p[1] > region[1] && p[1] < region[3]
                })
                .collect()
        }

        fn arrow_verts(ctx: &Context, row: [f32; 4]) -> alloc::vec::Vec<[f32; 2]> {
            let style = ctx.style;
            let t = style.border_size;
            let ax = row[0] + style.frame_padding[0];
            let arrow = ctx.font.line_height() * 0.5;
            verts_in(ctx, [row[0] + t, row[1] + t, ax + arrow + 1.0, row[3] - t])
        }

        #[test]
        fn arrow_points_right_when_closed_and_down_when_open() {
            let mut ctx = test_context();
            ctx.style.anti_aliasing = false;
            let row = settle(&mut ctx);

            let closed = arrow_verts(&ctx, row);
            assert_eq!(closed.len(), 3, "closed arrow must be one triangle");
            let min_x = closed.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
            assert_eq!(closed.iter().filter(|p| p[0] == min_x).count(), 2);

            click_open(&mut ctx, row);
            frame(&mut ctx, AWAY, false, declare);

            let open = arrow_verts(&ctx, row);
            assert_eq!(open.len(), 3, "open arrow must be one triangle");
            let min_y = open.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
            assert_eq!(open.iter().filter(|p| p[1] == min_y).count(), 2);
        }

        #[test]
        fn open_content_is_indented_under_the_title_and_restored_after() {
            let mut ctx = test_context();
            let row = settle(&mut ctx);
            click_open(&mut ctx, row);

            let mut content_x = 0.0;
            let mut after_x = 0.0;
            frame(&mut ctx, AWAY, false, |ui| {
                ui.collapsing_header("Settings", |ui| {
                    content_x = ui.ctx.current_window_mut().cursor.next_pos()[0];
                    ui.label("inner");
                });
                after_x = ui.ctx.current_window_mut().cursor.next_pos()[0];
            });

            let style = ctx.style;
            let arrow = ctx.font.line_height() * 0.5;
            let indent = 2.0 * style.frame_padding[0] + arrow;
            assert_eq!(content_x, row[0] + indent);
            assert_eq!(after_x, row[0]);
        }

        fn two_window_frame(
            ctx: &mut Context,
            mouse: [f32; 2],
            down: bool,
            record_w2_row: Option<&mut [f32; 4]>,
        ) {
            ctx.set_input(InputState {
                mouse_pos: mouse,
                mouse_down: [down, false, false, false, false],
                ..Default::default()
            });
            ctx.new_frame(0.016);
            {
                let mut ui = ctx.ui();
                ui.window("W1", ScrollFlags::NONE, |ui| {
                    ui.collapsing_header("Settings", |ui| mark(ui, MARK_W1));
                });
                ui.window("W2", ScrollFlags::NONE, |ui| {
                    if let Some(r) = record_w2_row {
                        *r = header_row(ui);
                    }
                    ui.collapsing_header("Settings", |ui| mark(ui, MARK_W2));
                });
            }
            ctx.render();
        }

        fn mark_color_in(ctx: &Context, title: &str, m: ([f32; 4], [u8; 4])) -> Option<[u8; 4]> {
            let index = ctx.windows.iter().position(|w| w.title == title)?;
            let data = ctx.windows[index].draw_list.as_draw_data();
            data.vertices.windows(3).find_map(|anchor| {
                if anchor[0].pos == [m.0[0], m.0[1]] && anchor[2].pos == [m.0[2], m.0[3]] {
                    Some(anchor[0].col)
                } else {
                    None
                }
            })
        }

        #[test]
        fn same_title_in_different_windows_is_independent() {
            let mut ctx = test_context();
            let mut row_w2 = [0.0; 4];
            two_window_frame(&mut ctx, AWAY, false, Some(&mut row_w2));
            assert_eq!(mark_color_in(&ctx, "W1", MARK_W1), None);
            assert_eq!(mark_color_in(&ctx, "W2", MARK_W2), None);

            let at = center(row_w2);
            two_window_frame(&mut ctx, at, true, None);
            two_window_frame(&mut ctx, at, false, None);
            two_window_frame(&mut ctx, AWAY, false, None);

            assert_eq!(mark_color_in(&ctx, "W2", MARK_W2), Some(MARK_W2.1));
            assert_eq!(mark_color_in(&ctx, "W1", MARK_W1), None);
            assert_eq!(open_state(&ctx, "W2", "Settings"), Some(true));
            assert_eq!(open_state(&ctx, "W1", "Settings"), Some(false));
        }

        #[test]
        fn hovering_the_row_sets_hot_and_hover_color() {
            let mut ctx = test_context();
            let row = settle(&mut ctx);

            frame(&mut ctx, center(row), false, declare);
            assert_eq!(ctx.hot, Some(widget_id_in_w("Settings")));
            assert_eq!(fill_color_at(&ctx, row), Some(ctx.style.widget_bg_hovered));
        }

        const MARK_SA: ([f32; 4], [u8; 4]) = ([100.0, 400.0, 110.0, 410.0], [1, 2, 3, 255]);
        const MARK_SB: ([f32; 4], [u8; 4]) = ([120.0, 420.0, 130.0, 430.0], [4, 5, 6, 255]);

        fn declare_ab(ui: &mut Ui) {
            ui.collapsing_header("Settings##a", |ui| mark(ui, MARK_SA));
            ui.collapsing_header("Settings##b", |ui| mark(ui, MARK_SB));
        }

        #[test]
        fn hidden_id_suffixes_make_same_display_title_independent() {
            let mut ctx = test_context();
            let mut row_b = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                ui.collapsing_header("Settings##a", |ui| mark(ui, MARK_SA));
                row_b = header_row(ui);
                ui.collapsing_header("Settings##b", |ui| mark(ui, MARK_SB));
            });
            assert_eq!(mark_color(&ctx, MARK_SA), None);
            assert_eq!(mark_color(&ctx, MARK_SB), None);

            let at = center(row_b);
            frame(&mut ctx, at, true, declare_ab);
            frame(&mut ctx, at, false, declare_ab);
            frame(&mut ctx, AWAY, false, declare_ab);

            assert_eq!(mark_color(&ctx, MARK_SB), Some(MARK_SB.1));
            assert_eq!(mark_color(&ctx, MARK_SA), None);
            assert_eq!(open_state(&ctx, "W", "Settings##b"), Some(true));
            assert_eq!(open_state(&ctx, "W", "Settings##a"), Some(false));
        }

        #[test]
        fn nested_headers_gate_content_and_indent_cumulatively() {
            const MARK_INNER: ([f32; 4], [u8; 4]) = ([100.0, 400.0, 110.0, 410.0], [7, 7, 7, 255]);

            let mut ctx = test_context();
            let mut row_outer = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                row_outer = header_row(ui);
                ui.collapsing_header("Outer", |ui| {
                    ui.collapsing_header("Inner", |ui| mark(ui, MARK_INNER));
                });
            });

            let at_outer = center(row_outer);
            frame(&mut ctx, at_outer, true, |ui| {
                ui.collapsing_header("Outer", |ui| {
                    ui.collapsing_header("Inner", |ui| mark(ui, MARK_INNER));
                });
            });
            frame(&mut ctx, at_outer, false, |ui| {
                ui.collapsing_header("Outer", |ui| {
                    ui.collapsing_header("Inner", |ui| mark(ui, MARK_INNER));
                });
            });

            let mut row_inner = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                ui.collapsing_header("Outer", |ui| {
                    row_inner = header_row(ui);
                    ui.collapsing_header("Inner", |ui| mark(ui, MARK_INNER));
                });
            });
            assert_eq!(mark_color(&ctx, MARK_INNER), None);
            assert_eq!(open_state(&ctx, "W", "Outer"), Some(true));
            let style = ctx.style;
            let indent = 2.0 * style.frame_padding[0] + ctx.font.line_height() * 0.5;
            assert_eq!(row_inner[0], row_outer[0] + indent);

            let at_inner = center(row_inner);
            frame(&mut ctx, at_inner, true, |ui| {
                ui.collapsing_header("Outer", |ui| {
                    ui.collapsing_header("Inner", |ui| mark(ui, MARK_INNER));
                });
            });
            frame(&mut ctx, at_inner, false, |ui| {
                ui.collapsing_header("Outer", |ui| {
                    ui.collapsing_header("Inner", |ui| mark(ui, MARK_INNER));
                });
            });

            let mut content_x = 0.0;
            frame(&mut ctx, AWAY, false, |ui| {
                ui.collapsing_header("Outer", |ui| {
                    ui.collapsing_header("Inner", |ui| {
                        content_x = ui.ctx.current_window_mut().cursor.next_pos()[0];
                        mark(ui, MARK_INNER);
                    });
                });
            });
            assert_eq!(mark_color(&ctx, MARK_INNER), Some(MARK_INNER.1));
            assert_eq!(content_x, row_outer[0] + 2.0 * indent);
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            let outer_id = stack.id("Outer");
            stack.push_id(outer_id);
            let inner_open = matches!(
                ctx.widget_states.get(&stack.id("Inner")),
                Some(WidgetState::CollapsingHeader { open: true })
            );
            assert!(inner_open, "inner header state must exist and be open");
        }

        mod popup_scope {
            use super::super::popup_frame;
            use super::*;

            const POPUP_RECT: [f32; 4] = [400.0, 300.0, 560.0, 460.0];
            const MARK_P: ([f32; 4], [u8; 4]) = ([420.0, 430.0, 430.0, 440.0], [7, 7, 7, 255]);

            fn declare(ui: &mut Ui) {
                ui.collapsing_header("Settings", |ui| mark(ui, MARK_P));
            }

            fn popup_header_row(ui: &mut Ui) -> [f32; 4] {
                let pos = ui.ctx.overlay_cursor.next_pos();
                let style = ui.ctx.style;
                let h = ui.ctx.font.line_height() + 2.0 * style.frame_padding[1];
                let w = ui.ctx.overlay_cursor.available_width();
                [pos[0], pos[1], pos[0] + w, pos[1] + h]
            }

            fn settle(ctx: &mut Context) -> (crate::id::Id, [f32; 4]) {
                let mut popup = crate::id::Id(0);
                let mut row = [0.0; 4];
                popup_frame(ctx, InputState::default(), |ui| {
                    popup = ui.widget_id("##popup");
                    ui.open_popup(popup);
                    ui.popup(popup, POPUP_RECT, |ui| {
                        row = popup_header_row(ui);
                        declare(ui);
                    });
                });
                (popup, row)
            }

            fn frame_header(ctx: &mut Context, popup: crate::id::Id, mouse: [f32; 2], down: bool) {
                popup_frame(
                    ctx,
                    InputState {
                        mouse_pos: mouse,
                        mouse_down: [down, false, false, false, false],
                        ..Default::default()
                    },
                    |ui| {
                        ui.popup(popup, POPUP_RECT, declare);
                    },
                );
            }

            fn overlay_mark(ctx: &Context, m: ([f32; 4], [u8; 4])) -> bool {
                ctx.overlay
                    .as_draw_data()
                    .vertices
                    .windows(4)
                    .any(|q| q[0].pos == [m.0[0], m.0[1]] && q[2].pos == [m.0[2], m.0[3]])
            }

            fn window_mark(ctx: &Context, m: ([f32; 4], [u8; 4])) -> bool {
                ctx.windows[0]
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .windows(4)
                    .any(|q| q[0].pos == [m.0[0], m.0[1]] && q[2].pos == [m.0[2], m.0[3]])
            }

            fn popup_open_state(ctx: &Context, popup: crate::id::Id, label: &str) -> Option<bool> {
                let mut stack = crate::id::IdStack::new();
                stack.push("W");
                stack.push_id(popup);
                match ctx.widget_states.get(&stack.id(label)) {
                    Some(WidgetState::CollapsingHeader { open }) => Some(*open),
                    _ => None,
                }
            }

            fn overlay_verts_in(ctx: &Context, region: [f32; 4]) -> alloc::vec::Vec<[f32; 2]> {
                ctx.overlay
                    .as_draw_data()
                    .vertices
                    .iter()
                    .map(|v| v.pos)
                    .filter(|p| {
                        p[0] > region[0] && p[0] < region[2] && p[1] > region[1] && p[1] < region[3]
                    })
                    .collect()
            }

            fn arrow_verts(ctx: &Context, row: [f32; 4]) -> alloc::vec::Vec<[f32; 2]> {
                let style = ctx.style;
                let t = style.border_size;
                let ax = row[0] + style.frame_padding[0];
                let arrow = ctx.font.line_height() * 0.5;
                overlay_verts_in(ctx, [row[0] + t, row[1] + t, ax + arrow + 1.0, row[3] - t])
            }

            #[test]
            fn click_toggles_next_frame_and_content_routes_to_the_overlay() {
                let mut ctx = test_context();
                let (popup, row) = settle(&mut ctx);
                assert!(!overlay_mark(&ctx, MARK_P));
                assert_eq!(popup_open_state(&ctx, popup, "Settings"), Some(false));

                let at = center(row);
                frame_header(&mut ctx, popup, at, true);
                frame_header(&mut ctx, popup, at, false);
                assert!(ctx.open_popup.is_some());
                assert!(!overlay_mark(&ctx, MARK_P));
                assert_eq!(popup_open_state(&ctx, popup, "Settings"), Some(true));

                frame_header(&mut ctx, popup, AWAY, false);
                assert!(overlay_mark(&ctx, MARK_P));
                assert!(!window_mark(&ctx, MARK_P));
                assert!(
                    ctx.overlay
                        .as_draw_data()
                        .commands
                        .iter()
                        .any(|c| c.clip_rect == POPUP_RECT),
                    "header content in a popup must carry the popup rect clip"
                );

                frame_header(&mut ctx, popup, AWAY, false);
                assert!(overlay_mark(&ctx, MARK_P));
                assert_eq!(popup_open_state(&ctx, popup, "Settings"), Some(true));

                frame_header(&mut ctx, popup, at, true);
                frame_header(&mut ctx, popup, at, false);
                assert_eq!(popup_open_state(&ctx, popup, "Settings"), Some(false));
                frame_header(&mut ctx, popup, AWAY, false);
                assert!(!overlay_mark(&ctx, MARK_P));
            }

            #[test]
            fn arrow_flips_and_the_indent_shifts_the_overlay_cursor() {
                let mut ctx = test_context();
                ctx.style.anti_aliasing = false;
                let (popup, row) = settle(&mut ctx);

                let closed = arrow_verts(&ctx, row);
                assert_eq!(closed.len(), 3, "closed arrow must be one triangle");
                let min_x = closed.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
                assert_eq!(closed.iter().filter(|p| p[0] == min_x).count(), 2);

                let at = center(row);
                frame_header(&mut ctx, popup, at, true);
                frame_header(&mut ctx, popup, at, false);

                let mut content_x = 0.0;
                let mut after_x = 0.0;
                popup_frame(&mut ctx, InputState::default(), |ui| {
                    ui.popup(popup, POPUP_RECT, |ui| {
                        ui.collapsing_header("Settings", |ui| {
                            content_x = ui.ctx.overlay_cursor.next_pos()[0];
                            mark(ui, MARK_P);
                        });
                        after_x = ui.ctx.overlay_cursor.next_pos()[0];
                    });
                });
                let open = arrow_verts(&ctx, row);
                assert_eq!(open.len(), 3, "open arrow must be one triangle");
                let min_y = open.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
                assert_eq!(open.iter().filter(|p| p[1] == min_y).count(), 2);

                let style = ctx.style;
                let indent = 2.0 * style.frame_padding[0] + ctx.font.line_height() * 0.5;
                assert_eq!(content_x, row[0] + indent);
                assert_eq!(after_x, row[0]);
            }
        }
    }

    mod searchable_combo {
        use crate::input::{InputState, Key, KeyEvent};
        use crate::ui::{search_matches, Ui};
        use crate::widgets::test_support::{frame_with_input, test_context};
        use alloc::string::String;

        const ITEMS: [&str; 3] = ["Alpha", "Bravo", "Charlie"];

        fn declare(ui: &mut Ui, selected: &mut usize, query: &mut String) -> bool {
            ui.searchable_combo("Find", &ITEMS, selected, query)
        }

        #[test]
        fn filtering_is_case_insensitive_without_losing_original_indices() {
            assert!(search_matches("Alpha Bravo", "BRAV"));
            assert!(search_matches("café", "fé"));
            assert!(!search_matches("Alpha", "Bravo"));
            assert!(!search_matches("ab", "abc"));
        }

        #[test]
        fn getter_only_reads_selected_item_while_closed() {
            let mut ctx = test_context();
            let mut selected = 9_999;
            let mut query = String::new();
            let mut calls = 0;
            frame_with_input(&mut ctx, 0.016, InputState::default(), |ui| {
                assert_eq!(
                    ui.searchable_combo_with(
                        "Find",
                        10_000,
                        |index| {
                            calls += 1;
                            assert_eq!(index, 9_999);
                            Some("last item")
                        },
                        &mut selected,
                        &mut query
                    ),
                    Some(false)
                );
            });
            assert_eq!(calls, 1);
        }

        #[test]
        fn invalid_open_item_does_not_change_selection() {
            let mut ctx = test_context();
            let mut selected = 0;
            let mut query = String::new();
            let mut button = [0.0; 2];
            frame_with_input(&mut ctx, 0.016, InputState::default(), |ui| {
                button = ui.ctx.current_window_mut().cursor.next_pos();
                assert_eq!(
                    ui.searchable_combo_with(
                        "Find",
                        2,
                        |index| (index == 0).then_some("valid"),
                        &mut selected,
                        &mut query
                    ),
                    Some(false)
                );
            });
            let at = [button[0] + 15.0, button[1] + 10.0];
            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    mouse_pos: at,
                    mouse_down: [true, false, false, false, false],
                    ..Default::default()
                },
                |ui| {
                    assert_eq!(
                        ui.searchable_combo_with(
                            "Find",
                            2,
                            |index| (index == 0).then_some("valid"),
                            &mut selected,
                            &mut query
                        ),
                        Some(false)
                    );
                },
            );
            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    mouse_pos: at,
                    ..Default::default()
                },
                |ui| {
                    assert_eq!(
                        ui.searchable_combo_with(
                            "Find",
                            2,
                            |index| (index == 0).then_some("valid"),
                            &mut selected,
                            &mut query
                        ),
                        None
                    );
                },
            );
            assert_eq!(selected, 0);
            assert!(ctx.open_popup.is_none());
        }

        #[test]
        fn typing_filters_and_enter_selects_the_matching_item() {
            let mut ctx = test_context();
            let mut selected = 0;
            let mut query = String::new();
            let mut button = [0.0; 2];
            frame_with_input(&mut ctx, 0.016, InputState::default(), |ui| {
                button = ui.ctx.current_window_mut().cursor.next_pos();
                assert!(!declare(ui, &mut selected, &mut query));
            });
            let at = [button[0] + 15.0, button[1] + 10.0];
            for down in [true, false] {
                frame_with_input(
                    &mut ctx,
                    0.016,
                    InputState {
                        mouse_pos: at,
                        mouse_down: [down, false, false, false, false],
                        ..Default::default()
                    },
                    |ui| {
                        assert!(!declare(ui, &mut selected, &mut query));
                    },
                );
            }
            assert!(ctx.open_popup.is_some());
            let keys = [KeyEvent::Char('B'), KeyEvent::Char('r')];
            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    keys: &keys,
                    ..Default::default()
                },
                |ui| {
                    assert!(!declare(ui, &mut selected, &mut query));
                },
            );
            assert_eq!(query, "Br");
            let enter = [KeyEvent::Press(Key::Enter)];
            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    keys: &enter,
                    ..Default::default()
                },
                |ui| {
                    assert!(declare(ui, &mut selected, &mut query));
                },
            );
            assert_eq!(selected, 1);
            assert!(ctx.open_popup.is_none());
            assert!(ctx.active.is_none());
        }

        #[test]
        fn escape_closes_without_leaving_search_focused() {
            let mut ctx = test_context();
            let mut selected = 0;
            let mut query = String::new();
            let mut button = [0.0; 2];
            frame_with_input(&mut ctx, 0.016, InputState::default(), |ui| {
                button = ui.ctx.current_window_mut().cursor.next_pos();
                declare(ui, &mut selected, &mut query);
            });
            let at = [button[0] + 15.0, button[1] + 10.0];
            for down in [true, false] {
                frame_with_input(
                    &mut ctx,
                    0.016,
                    InputState {
                        mouse_pos: at,
                        mouse_down: [down, false, false, false, false],
                        ..Default::default()
                    },
                    |ui| {
                        declare(ui, &mut selected, &mut query);
                    },
                );
            }
            assert!(ctx.open_popup.is_some());
            let escape = [KeyEvent::Press(Key::Escape)];
            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    keys: &escape,
                    ..Default::default()
                },
                |ui| {
                    declare(ui, &mut selected, &mut query);
                },
            );
            assert!(ctx.open_popup.is_none());
            assert!(ctx.active.is_none());
            assert_eq!(selected, 0);
        }

        #[test]
        fn keyboard_highlight_scrolls_to_reveal_later_results() {
            const MANY: [&str; 12] = [
                "00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11",
            ];
            let mut ctx = test_context();
            let mut selected = 0;
            let mut query = String::new();
            let mut button = [0.0; 2];
            frame_with_input(&mut ctx, 0.016, InputState::default(), |ui| {
                button = ui.ctx.current_window_mut().cursor.next_pos();
                ui.searchable_combo("Find", &MANY, &mut selected, &mut query);
            });
            let at = [button[0] + 15.0, button[1] + 10.0];
            for down in [true, false] {
                frame_with_input(
                    &mut ctx,
                    0.016,
                    InputState {
                        mouse_pos: at,
                        mouse_down: [down, false, false, false, false],
                        ..Default::default()
                    },
                    |ui| {
                        ui.searchable_combo("Find", &MANY, &mut selected, &mut query);
                    },
                );
            }
            let downs = [KeyEvent::Press(Key::Down); 10];
            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    keys: &downs,
                    ..Default::default()
                },
                |ui| {
                    ui.searchable_combo("Find", &MANY, &mut selected, &mut query);
                },
            );
            let mut ids = crate::id::IdStack::new();
            ids.push("W");
            let combo = ids.id("Find");
            ids.push_id(combo);
            let scroll = ids.id("##search_results");
            assert!(
                matches!(ctx.widget_states.get(&scroll), Some(crate::context::WidgetState::Scroll { offset, .. }) if *offset > 0.0)
            );
            let enter = [KeyEvent::Press(Key::Enter)];
            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    keys: &enter,
                    ..Default::default()
                },
                |ui| {
                    assert!(ui.searchable_combo("Find", &MANY, &mut selected, &mut query));
                },
            );
            assert_eq!(selected, 10);
        }

        #[test]
        fn clicking_a_result_selects_its_original_index() {
            let mut ctx = test_context();
            let mut selected = 0;
            let mut query = String::new();
            let mut button = [0.0; 2];
            frame_with_input(&mut ctx, 0.016, InputState::default(), |ui| {
                button = ui.ctx.current_window_mut().cursor.next_pos();
                declare(ui, &mut selected, &mut query);
            });
            let at = [button[0] + 15.0, button[1] + 10.0];
            for down in [true, false] {
                frame_with_input(
                    &mut ctx,
                    0.016,
                    InputState {
                        mouse_pos: at,
                        mouse_down: [down, false, false, false, false],
                        ..Default::default()
                    },
                    |ui| {
                        declare(ui, &mut selected, &mut query);
                    },
                );
            }
            let popup = ctx.popup_rect.expect("search popup opened");
            let style = ctx.style;
            let row_h = ctx.font.line_height() + 2.0 * style.frame_padding[1];
            let row_step = row_h + style.item_spacing[1];
            let bravo = [
                popup[0] + style.window_padding[0] + 16.0,
                popup[1] + style.window_padding[1] + row_step * 2.0 + row_h * 0.5,
            ];
            for down in [true, false] {
                frame_with_input(
                    &mut ctx,
                    0.016,
                    InputState {
                        mouse_pos: bravo,
                        mouse_down: [down, false, false, false, false],
                        ..Default::default()
                    },
                    |ui| {
                        declare(ui, &mut selected, &mut query);
                    },
                );
            }
            assert_eq!(selected, 1);
            assert!(ctx.open_popup.is_none());
        }
    }

    mod combo {
        use crate::context::{Context, WidgetState};
        use crate::input::{InputState, Key, KeyEvent};
        use crate::ui::Ui;
        use crate::widgets::test_support::*;

        const AWAY: [f32; 2] = [-50.0, -50.0];
        const ITEMS: [&str; 3] = ["Red", "Green", "Blue"];
        const LONG_ITEMS: [&str; 6] = ["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot"];

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn button_rect(ui: &mut Ui, current: &str) -> [f32; 4] {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let style = ui.ctx.style;
            let text_w = ui.ctx.font.measure(current)[0];
            let line_height = ui.ctx.font.line_height();
            let h = line_height + 2.0 * style.frame_padding[1];
            let w = text_w + line_height * 0.5 + 3.0 * style.frame_padding[0];
            [pos[0], pos[1], pos[0] + w, pos[1] + h]
        }

        fn declare(ui: &mut Ui, selected: &mut usize) -> bool {
            ui.combo("Color", &ITEMS, selected)
        }

        fn settle(ctx: &mut Context, selected: &mut usize) -> [f32; 4] {
            let mut rect = [0.0; 4];
            let current = ITEMS[*selected];
            frame(ctx, AWAY, false, |ui| {
                rect = button_rect(ui, current);
                declare(ui, selected);
            });
            rect
        }

        fn click_open(ctx: &mut Context, button: [f32; 4], selected: &mut usize) -> [f32; 4] {
            let at = center(button);
            frame(ctx, at, true, |ui| {
                declare(ui, selected);
            });
            frame(ctx, at, false, |ui| {
                declare(ui, selected);
            });
            ctx.popup_rect
                .expect("the popup must be open after the click")
        }

        fn row_rect(ctx: &Context, popup: [f32; 4], i: usize) -> [f32; 4] {
            let style = ctx.style;
            let row_h = ctx.font.line_height() + 2.0 * style.frame_padding[1];
            let y = popup[1] + style.window_padding[1] + i as f32 * (row_h + style.item_spacing[1]);
            [
                popup[0] + style.window_padding[0],
                y,
                popup[2] - style.window_padding[0],
                y + row_h,
            ]
        }

        fn overlay_fill_color_at(ctx: &Context, rect: [f32; 4]) -> Option<[u8; 4]> {
            crate::widgets::test_support::chrome_color_in(ctx.overlay.as_draw_data(), rect)
        }

        fn overlay_has_glyphs_in(ctx: &Context, rect: [f32; 4]) -> bool {
            let data = ctx.overlay.as_draw_data();
            let mut offset = 0usize;
            for cmd in data.commands {
                let next = offset + cmd.elem_count as usize;
                if cmd.texture == crate::draw::TextureId::FONT {
                    for &i in &data.indices[offset..next] {
                        let v = data.vertices[i as usize];
                        if v.pos[0] >= rect[0]
                            && v.pos[0] <= rect[2]
                            && v.pos[1] >= rect[1]
                            && v.pos[1] <= rect[3]
                        {
                            return true;
                        }
                    }
                }
                offset = next;
            }
            false
        }

        fn assert_below_fold_glyphs_carry_region_clip(ctx: &Context, region: [f32; 4]) {
            let inset = crate::scroll::content_inset(&ctx.style, region);
            let content_clip = [
                region[0] + inset,
                region[1] + inset,
                region[2] - inset,
                region[3] - inset,
            ];
            let data = ctx.overlay.as_draw_data();
            let mut offset = 0usize;
            let mut below = 0usize;
            for cmd in data.commands {
                let next = offset + cmd.elem_count as usize;
                if cmd.texture == crate::draw::TextureId::FONT {
                    for &i in &data.indices[offset..next] {
                        let v = data.vertices[i as usize];
                        if v.pos[1] > region[3] && !ctx.font.uv_samples_shape_mask(v.uv) {
                            below += 1;
                            for (c, &expected) in content_clip.iter().enumerate() {
                                assert!(
                                    (cmd.clip_rect[c] - expected).abs() < 1e-3,
                                    "a glyph below the fold must carry the inset content clip, \
                                     got {:?}",
                                    cmd.clip_rect
                                );
                            }
                        }
                    }
                }
                offset = next;
            }
            assert!(below > 0, "rows below the fold must still emit vertices");
        }

        fn declare_long(ui: &mut Ui, selected: &mut usize) -> bool {
            ui.combo("Color", &LONG_ITEMS, selected)
        }

        fn settle_long(ctx: &mut Context, selected: &mut usize) -> [f32; 4] {
            let mut rect = [0.0; 4];
            let current = LONG_ITEMS[*selected];
            frame(ctx, AWAY, false, |ui| {
                rect = button_rect(ui, current);
                declare_long(ui, selected);
            });
            rect
        }

        fn viewport_fitting(ctx: &Context, button: [f32; 4], rows: usize) -> [f32; 2] {
            let style = ctx.style;
            let row_h = ctx.font.line_height() + 2.0 * style.frame_padding[1];
            let h = 2.0 * style.window_padding[1]
                + rows as f32 * row_h
                + (rows as f32 - 1.0) * style.item_spacing[1];
            [400.0, button[3] + h]
        }

        fn click_open_viewport(
            ctx: &mut Context,
            button: [f32; 4],
            viewport: [f32; 2],
            selected: &mut usize,
        ) -> [f32; 4] {
            let at = center(button);
            let input = |down: bool| InputState {
                mouse_pos: at,
                mouse_down: [down, false, false, false, false],
                viewport_size: viewport,
                ..Default::default()
            };
            frame_with_input(ctx, 0.016, input(true), |ui| {
                declare_long(ui, selected);
            });
            frame_with_input(ctx, 0.016, input(false), |ui| {
                declare_long(ui, selected);
            });
            ctx.popup_rect
                .expect("the popup must be open after the click")
        }

        fn wheel_frame(
            ctx: &mut Context,
            mouse: [f32; 2],
            wheel: f32,
            viewport: [f32; 2],
            selected: &mut usize,
        ) {
            frame_with_input(
                ctx,
                0.016,
                InputState {
                    mouse_pos: mouse,
                    mouse_wheel: wheel,
                    viewport_size: viewport,
                    ..Default::default()
                },
                |ui| {
                    declare_long(ui, selected);
                },
            );
        }

        fn list_region_rect(ctx: &Context, popup: [f32; 4]) -> [f32; 4] {
            let pad = ctx.style.window_padding;
            [
                popup[0] + pad[0],
                popup[1] + pad[1],
                popup[2] - pad[0],
                popup[3] - pad[1],
            ]
        }

        fn scrolled_row_rect(ctx: &Context, popup: [f32; 4], i: usize, offset: f32) -> [f32; 4] {
            let region = list_region_rect(ctx, popup);
            let style = ctx.style;
            let row_h = ctx.font.line_height() + 2.0 * style.frame_padding[1];
            let y = region[1] + i as f32 * (row_h + style.item_spacing[1]) - offset;
            [region[0], y, region[2], y + row_h]
        }

        fn list_region_id(combo_label: &str) -> crate::id::Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            let popup_id = stack.id(combo_label);
            stack.push_id(popup_id);
            stack.id("##combo_list")
        }

        fn list_scroll_state(ctx: &Context, combo_label: &str) -> Option<(f32, f32)> {
            match ctx.widget_states.get(&list_region_id(combo_label)) {
                Some(WidgetState::Scroll {
                    offset,
                    content_height,
                }) => Some((*offset, *content_height)),
                _ => None,
            }
        }

        fn verts_in(ctx: &Context, region: [f32; 4]) -> alloc::vec::Vec<[f32; 2]> {
            let data = ctx.windows[0].draw_list.as_draw_data();
            data.vertices
                .iter()
                .map(|v| v.pos)
                .filter(|p| {
                    p[0] > region[0] && p[0] < region[2] && p[1] > region[1] && p[1] < region[3]
                })
                .collect()
        }

        #[test]
        fn closed_combo_shows_current_item_arrow_and_label() {
            let mut ctx = test_context();
            ctx.style.anti_aliasing = false;
            let mut selected = 1usize;
            let mut rect = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                rect = button_rect(ui, "Green");
                assert!(!declare(ui, &mut selected), "no interaction, no change");
            });
            let style = ctx.style;

            assert_eq!(fill_color_at(&ctx, rect), Some(style.widget_bg));

            let pen_x = rect[0] + style.frame_padding[0];
            let (_, quad) = ctx
                .font
                .glyph_quad(crate::font::FontId::DEFAULT, 'G', [pen_x, 0.0]);
            let expected_min_x = quad.expect("'G' has ink").min[0];
            let actual_min_x =
                glyph_min_x_in(&ctx, rect).expect("the current item text must be emitted");
            assert!(
                (actual_min_x - expected_min_x).abs() < 1e-4,
                "text min x {actual_min_x} != left-aligned {expected_min_x}"
            );

            let arrow = ctx.font.line_height() * 0.5;
            let t = style.border_size;
            let region = [
                rect[2] - style.frame_padding[0] - arrow - 1.0,
                rect[1] + t,
                rect[2] - style.frame_padding[0] + 1.0,
                rect[3] - t,
            ];
            let verts = verts_in(&ctx, region);
            assert_eq!(verts.len(), 3, "the arrow must be one triangle");
            let min_y = verts.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
            assert_eq!(verts.iter().filter(|p| p[1] == min_y).count(), 2);

            let label_region = [
                rect[2] + style.item_spacing[0],
                rect[1],
                rect[2] + style.item_spacing[0] + ctx.font.measure("Color")[0] + 2.0,
                rect[3],
            ];
            assert!(
                !glyph_colors_in(&ctx, label_region).is_empty(),
                "the label text must be drawn right of the button"
            );
        }

        #[test]
        fn press_opens_popup_below_the_button_sized_to_the_items_with_selection_marked() {
            let mut ctx = test_context();
            let mut selected = 1usize;
            let button = settle(&mut ctx, &mut selected);
            let popup = click_open(&mut ctx, button, &mut selected);
            assert_eq!(ctx.open_popup, Some(widget_id_in_w("Color")));

            let style = ctx.style;
            let row_h = ctx.font.line_height() + 2.0 * style.frame_padding[1];
            assert_eq!([popup[0], popup[1]], [button[0], button[3]]);
            let widest = ctx.font.measure("Green")[0];
            let content = widest + 2.0 * style.frame_padding[0] + 2.0 * style.window_padding[0];
            assert_eq!(popup[2] - popup[0], (button[2] - button[0]).max(content));
            let expected_h =
                2.0 * style.window_padding[1] + (3.0 * row_h + 2.0 * style.item_spacing[1]);
            assert_eq!(popup[3] - popup[1], expected_h);

            for i in 0..3 {
                assert!(
                    overlay_has_glyphs_in(&ctx, row_rect(&ctx, popup, i)),
                    "row {i} must emit its item text"
                );
            }
            assert_eq!(
                overlay_fill_color_at(&ctx, row_rect(&ctx, popup, 1)),
                Some(style.widget_bg_active),
                "the selected row must be marked"
            );
            assert_eq!(overlay_fill_color_at(&ctx, row_rect(&ctx, popup, 0)), None);
            assert_eq!(overlay_fill_color_at(&ctx, row_rect(&ctx, popup, 2)), None);
        }

        #[test]
        fn clicking_an_item_selects_it_closes_the_popup_and_returns_true_on_that_frame_only() {
            let mut ctx = test_context();
            let mut selected = 0usize;
            let button = settle(&mut ctx, &mut selected);
            let popup = click_open(&mut ctx, button, &mut selected);

            let at = center(row_rect(&ctx, popup, 2));
            let mut changed = true;
            frame(&mut ctx, at, true, |ui| {
                changed = declare(ui, &mut selected)
            });
            assert!(!changed, "the press frame must not change the selection");
            assert!(
                ctx.open_popup.is_some(),
                "a press inside the popup must keep it open"
            );

            frame(&mut ctx, at, false, |ui| {
                changed = declare(ui, &mut selected)
            });
            assert!(changed, "the release frame inside the row selects the item");
            assert_eq!(selected, 2);
            assert_eq!(ctx.open_popup, None, "selecting closes the popup");

            frame(&mut ctx, AWAY, false, |ui| {
                changed = declare(ui, &mut selected)
            });
            assert!(!changed, "the change must not repeat on later frames");
            assert_eq!(selected, 2);
        }

        #[test]
        fn click_outside_closes_without_selecting_and_the_press_is_consumed() {
            let mut ctx = test_context();
            let mut selected = 1usize;
            let mut probe_rect = [0.0; 4];
            let mut button = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                probe_rect = next_text_rect(ui, "Probe");
                ui.button("Probe");
                button = button_rect(ui, "Green");
                declare(ui, &mut selected);
            });
            let mut probe_clicked = true;
            let declare_all = |ui: &mut Ui, selected: &mut usize, probe: &mut bool| {
                *probe = ui.button("Probe");
                declare(ui, selected);
            };
            let popup = {
                let at = center(button);
                frame(&mut ctx, at, true, |ui| {
                    declare_all(ui, &mut selected, &mut probe_clicked)
                });
                frame(&mut ctx, at, false, |ui| {
                    declare_all(ui, &mut selected, &mut probe_clicked)
                });
                assert!(!probe_clicked, "the combo's click is not the probe's");
                ctx.popup_rect
                    .expect("the popup must be open after the click")
            };
            let at = center(probe_rect);
            assert!(
                !crate::ui::point_in_rect(at, popup),
                "the probe must be outside the popup rect for this test"
            );

            frame(&mut ctx, at, true, |ui| {
                declare_all(ui, &mut selected, &mut probe_clicked)
            });
            assert_eq!(
                ctx.open_popup, None,
                "an outside press must close the popup"
            );
            assert_eq!(ctx.active, None, "the consumed press must capture nothing");
            frame(&mut ctx, at, false, |ui| {
                declare_all(ui, &mut selected, &mut probe_clicked)
            });
            assert!(
                !probe_clicked,
                "the probe must not click from a consumed press"
            );
            assert_eq!(selected, 1, "the selection is unchanged");
        }

        #[test]
        fn escape_closes_the_popup_without_selecting() {
            let mut ctx = test_context();
            let mut selected = 1usize;
            let button = settle(&mut ctx, &mut selected);
            click_open(&mut ctx, button, &mut selected);
            assert!(ctx.open_popup.is_some());

            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    keys: &[KeyEvent::Press(Key::Escape)],
                    ..Default::default()
                },
                |ui| {
                    declare(ui, &mut selected);
                },
            );
            assert_eq!(ctx.open_popup, None, "Escape must close the popup");
            assert_eq!(selected, 1, "the selection is unchanged");
            frame(&mut ctx, AWAY, false, |ui| {
                declare(ui, &mut selected);
            });
            assert_eq!(ctx.open_popup, None);
            assert_eq!(selected, 1);
        }

        #[test]
        fn empty_items_list_disables_the_combo() {
            let mut ctx = test_context();
            let items: [&str; 0] = [];
            let mut selected = 0usize;
            let mut button = [0.0; 4];
            let mut changed = true;
            frame(&mut ctx, AWAY, false, |ui| {
                button = button_rect(ui, "");
                changed = ui.combo("Color", &items, &mut selected);
            });
            assert!(!changed);

            let at = center(button);
            frame(&mut ctx, at, false, |ui| {
                changed = ui.combo("Color", &items, &mut selected)
            });
            assert_eq!(ctx.hot, None, "a disabled combo reports no hover");
            assert_eq!(fill_color_at(&ctx, button), Some(ctx.style.widget_bg));

            frame(&mut ctx, at, true, |ui| {
                changed = ui.combo("Color", &items, &mut selected)
            });
            assert_eq!(ctx.active, None, "a disabled combo must not capture");
            frame(&mut ctx, at, false, |ui| {
                changed = ui.combo("Color", &items, &mut selected)
            });
            assert!(!changed);
            assert_eq!(ctx.open_popup, None, "a disabled combo opens no popup");
        }

        #[test]
        fn hidden_id_suffixes_make_same_display_label_combos_independent() {
            let mut ctx = test_context();
            let mut selected_a = 0usize;
            let mut selected_b = 2usize;
            let mut button_a = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                button_a = button_rect(ui, "Red");
                ui.combo("Color##a", &ITEMS, &mut selected_a);
                ui.combo("Color##b", &ITEMS, &mut selected_b);
            });

            let at = center(button_a);
            let declare_ab = |ui: &mut Ui, a: &mut usize, b: &mut usize| {
                ui.combo("Color##a", &ITEMS, a);
                ui.combo("Color##b", &ITEMS, b);
            };
            frame(&mut ctx, at, true, |ui| {
                declare_ab(ui, &mut selected_a, &mut selected_b)
            });
            frame(&mut ctx, at, false, |ui| {
                declare_ab(ui, &mut selected_a, &mut selected_b)
            });
            assert_eq!(ctx.open_popup, Some(widget_id_in_w("Color##a")));
            assert_ne!(ctx.open_popup, Some(widget_id_in_w("Color##b")));

            let popup = ctx.popup_rect.expect("a's popup must be open");
            let at = center(row_rect(&ctx, popup, 1));
            frame(&mut ctx, at, true, |ui| {
                declare_ab(ui, &mut selected_a, &mut selected_b)
            });
            frame(&mut ctx, at, false, |ui| {
                declare_ab(ui, &mut selected_a, &mut selected_b)
            });
            assert_eq!(selected_a, 1);
            assert_eq!(selected_b, 2, "combo b is untouched by a's interaction");
            assert_eq!(ctx.open_popup, None);
        }

        #[test]
        fn popup_height_is_clamped_into_the_viewport_and_the_full_list_scrolls() {
            let mut ctx = test_context();
            let mut selected = 0usize;
            let button = settle_long(&mut ctx, &mut selected);
            let viewport = viewport_fitting(&ctx, button, 2);
            let popup = click_open_viewport(&mut ctx, button, viewport, &mut selected);

            assert_eq!(popup[1], button[3], "still anchored below the button");
            assert!(
                (popup[3] - viewport[1]).abs() < 1e-4,
                "height clamped to the space below the button: {:?}",
                popup
            );

            let style = ctx.style;
            let row_h = ctx.font.line_height() + 2.0 * style.frame_padding[1];
            let region = list_region_rect(&ctx, popup);
            assert!(
                (region[3] - region[1] - (2.0 * row_h + style.item_spacing[1])).abs() < 1e-4,
                "the scroll region fills the popup's content area (two rows)"
            );

            let (offset, content_height) = list_scroll_state(&ctx, "Color")
                .expect("an overflowing list must declare a scroll region");
            assert_eq!(offset, 0.0);
            assert!(overlay_has_glyphs_in(
                &ctx,
                scrolled_row_rect(&ctx, popup, 0, 0.0)
            ));
            assert!(overlay_has_glyphs_in(
                &ctx,
                scrolled_row_rect(&ctx, popup, 1, 0.0)
            ));
            let all_rows = 6.0 * row_h + 5.0 * style.item_spacing[1];
            assert!(
                (content_height - all_rows).abs() < 1e-4,
                "the region's content must be the FULL list: {content_height} != {all_rows}"
            );
            assert_below_fold_glyphs_carry_region_clip(&ctx, region);

            let at = center(region);
            wheel_frame(&mut ctx, at, -100.0, viewport, &mut selected);
            let (offset, content_height) = list_scroll_state(&ctx, "Color").unwrap();
            let inset = crate::scroll::content_inset(&ctx.style, region);
            let max_offset = content_height - (region[3] - region[1] - 2.0 * inset);
            assert!(offset > 0.0);
            assert!(
                (offset - max_offset).abs() < 1e-4,
                "the offset must clamp to the content bottom: {offset} != {max_offset}"
            );
            wheel_frame(&mut ctx, at, 0.0, viewport, &mut selected);
            let last = scrolled_row_rect(&ctx, popup, 5, offset);
            assert!(
                last[1] >= region[1] - 1e-4 && last[3] <= region[3] + 1e-4,
                "the last row must be fully visible at max offset: {last:?} vs {region:?}"
            );
            assert!(
                overlay_has_glyphs_in(&ctx, last),
                "the last row's text must land inside the region"
            );

            wheel_frame(&mut ctx, at, 100.0, viewport, &mut selected);
            assert_eq!(list_scroll_state(&ctx, "Color").unwrap().0, 0.0);
            wheel_frame(&mut ctx, at, 0.0, viewport, &mut selected);
            assert!(overlay_has_glyphs_in(
                &ctx,
                scrolled_row_rect(&ctx, popup, 0, 0.0)
            ));
        }

        #[test]
        fn clicking_a_scrolled_into_view_row_selects_it_closes_and_returns_true() {
            let mut ctx = test_context();
            let mut selected = 0usize;
            let button = settle_long(&mut ctx, &mut selected);
            let viewport = viewport_fitting(&ctx, button, 2);
            let popup = click_open_viewport(&mut ctx, button, viewport, &mut selected);
            let region = list_region_rect(&ctx, popup);

            wheel_frame(&mut ctx, center(region), -100.0, viewport, &mut selected);
            let (offset, _) = list_scroll_state(&ctx, "Color").unwrap();
            let last = scrolled_row_rect(&ctx, popup, 5, offset);
            assert!(
                last[1] >= region[1] - 1e-4 && last[3] <= region[3] + 1e-4,
                "the last row must be fully visible after scrolling"
            );

            let at = center(last);
            let input = |down: bool| InputState {
                mouse_pos: at,
                mouse_down: [down, false, false, false, false],
                viewport_size: viewport,
                ..Default::default()
            };
            let mut changed = true;
            frame_with_input(&mut ctx, 0.016, input(true), |ui| {
                changed = declare_long(ui, &mut selected)
            });
            assert!(!changed, "the press frame must not change the selection");
            assert!(
                ctx.open_popup.is_some(),
                "a press on a row must keep the popup open"
            );
            frame_with_input(&mut ctx, 0.016, input(false), |ui| {
                changed = declare_long(ui, &mut selected)
            });
            assert!(changed, "the release frame on the row selects the item");
            assert_eq!(selected, 5);
            assert_eq!(ctx.open_popup, None, "selecting closes the popup");
        }

        #[test]
        fn a_list_that_fits_declares_no_scroll_region_and_ignores_the_wheel() {
            let mut ctx = test_context();
            let mut selected = 1usize;
            let button = settle(&mut ctx, &mut selected);
            let viewport = viewport_fitting(&ctx, button, 4);
            let at = center(button);
            let input = |down: bool| InputState {
                mouse_pos: at,
                mouse_down: [down, false, false, false, false],
                viewport_size: viewport,
                ..Default::default()
            };
            frame_with_input(&mut ctx, 0.016, input(true), |ui| {
                declare(ui, &mut selected);
            });
            frame_with_input(&mut ctx, 0.016, input(false), |ui| {
                declare(ui, &mut selected);
            });
            let popup = ctx.popup_rect.expect("the popup must be open");

            let over = center(list_region_rect(&ctx, popup));
            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    mouse_pos: over,
                    mouse_wheel: -1.0,
                    viewport_size: viewport,
                    ..Default::default()
                },
                |ui| {
                    declare(ui, &mut selected);
                },
            );
            assert!(
                list_scroll_state(&ctx, "Color").is_none(),
                "a fitting list must not declare a scroll region"
            );
            assert!(
                ctx.open_popup.is_some(),
                "the wheel must not close the popup"
            );
            for i in 0..3 {
                assert!(
                    overlay_has_glyphs_in(&ctx, row_rect(&ctx, popup, i)),
                    "row {i} must stay at its unscrolled position"
                );
            }
        }

        #[test]
        fn the_wheel_scrolls_the_popup_list_only_while_over_the_popup() {
            let mut ctx = test_context();
            let mut selected = 0usize;
            let button = settle_long(&mut ctx, &mut selected);
            let viewport = viewport_fitting(&ctx, button, 2);
            let popup = click_open_viewport(&mut ctx, button, viewport, &mut selected);
            let region = list_region_rect(&ctx, popup);

            wheel_frame(&mut ctx, center(button), -1.0, viewport, &mut selected);
            assert_eq!(list_scroll_state(&ctx, "Color").unwrap().0, 0.0);
            assert!(ctx.open_popup.is_some());

            wheel_frame(&mut ctx, center(region), -1.0, viewport, &mut selected);
            let expected = super::super::WHEEL_LINES_PER_NOTCH * ctx.font.line_height();
            let (offset, _) = list_scroll_state(&ctx, "Color").unwrap();
            assert!(
                (offset - expected).abs() < 1e-4,
                "one notch over the popup: {offset} != {expected}"
            );
        }

        fn row_id(combo_label: &str, index: usize) -> crate::id::Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            let popup_id = stack.id(combo_label);
            stack.push_id(popup_id);
            stack.push_id(crate::id::Id(index as u64));
            stack.id("##combo_item")
        }

        #[test]
        fn hovering_a_popup_row_highlights_it_and_sets_hot_to_the_row_id() {
            let mut ctx = test_context();
            let mut selected = 2usize;
            let button = settle(&mut ctx, &mut selected);
            let popup = click_open(&mut ctx, button, &mut selected);

            let at = center(row_rect(&ctx, popup, 0));
            frame(&mut ctx, at, false, |ui| {
                declare(ui, &mut selected);
            });
            assert_eq!(
                overlay_fill_color_at(&ctx, row_rect(&ctx, popup, 0)),
                Some(ctx.style.widget_bg_hovered),
                "the row under the mouse must be highlighted"
            );
            assert_eq!(ctx.hot, Some(row_id("Color", 0)));
            assert_eq!(overlay_fill_color_at(&ctx, row_rect(&ctx, popup, 1)), None);
            assert_eq!(
                overlay_fill_color_at(&ctx, row_rect(&ctx, popup, 2)),
                Some(ctx.style.widget_bg_active)
            );
        }

        #[test]
        fn duplicate_item_strings_are_distinct_rows() {
            let mut ctx = test_context();
            let items = ["A", "A"];
            let mut selected = 0usize;
            let mut button = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                button = button_rect(ui, "A");
                ui.combo("Color", &items, &mut selected);
            });
            let at = center(button);
            frame(&mut ctx, at, true, |ui| {
                ui.combo("Color", &items, &mut selected);
            });
            frame(&mut ctx, at, false, |ui| {
                ui.combo("Color", &items, &mut selected);
            });
            let popup = ctx.popup_rect.expect("the popup must open");

            assert_ne!(row_id("Color", 0), row_id("Color", 1));
            let at = center(row_rect(&ctx, popup, 1));
            frame(&mut ctx, at, true, |ui| {
                ui.combo("Color", &items, &mut selected);
            });
            assert_eq!(ctx.active, Some(row_id("Color", 1)));
            frame(&mut ctx, at, false, |ui| {
                ui.combo("Color", &items, &mut selected);
            });
            assert_eq!(selected, 1, "the second row selects index 1, not 0");
            assert_eq!(ctx.open_popup, None);
        }

        #[test]
        fn clicking_the_button_while_open_closes_the_popup_and_a_third_click_reopens() {
            let mut ctx = test_context();
            let mut selected = 1usize;
            let button = settle(&mut ctx, &mut selected);
            click_open(&mut ctx, button, &mut selected);
            assert!(ctx.open_popup.is_some());

            let at = center(button);
            frame(&mut ctx, at, true, |ui| {
                declare(ui, &mut selected);
            });
            assert_eq!(ctx.open_popup, None, "the outside press closes the popup");
            frame(&mut ctx, at, false, |ui| {
                declare(ui, &mut selected);
            });
            assert_eq!(
                ctx.open_popup, None,
                "the consumed press must not reopen the popup on release"
            );

            frame(&mut ctx, at, true, |ui| {
                declare(ui, &mut selected);
            });
            frame(&mut ctx, at, false, |ui| {
                declare(ui, &mut selected);
            });
            assert_eq!(
                ctx.open_popup,
                Some(widget_id_in_w("Color")),
                "a fresh click reopens the popup"
            );
            assert_eq!(selected, 1, "toggling never touches the selection");
        }

        #[test]
        fn shrinking_to_an_empty_list_while_open_closes_the_popup() {
            let mut ctx = test_context();
            let mut selected = 1usize;
            let button = settle(&mut ctx, &mut selected);
            click_open(&mut ctx, button, &mut selected);
            assert!(ctx.open_popup.is_some());

            let empty: [&str; 0] = [];
            frame(&mut ctx, AWAY, false, |ui| {
                ui.combo("Color", &empty, &mut selected);
            });
            assert_eq!(
                ctx.open_popup, None,
                "a list shrunk to empty must not leave a contentless popup"
            );

            let button = settle(&mut ctx, &mut selected);
            click_open(&mut ctx, button, &mut selected);
            assert!(ctx.open_popup.is_some());
        }

        #[test]
        fn out_of_range_selected_shows_empty_text_and_marks_no_row() {
            let mut ctx = test_context();
            let mut selected = 5usize;
            let mut rect = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                rect = button_rect(ui, "");
                assert!(!declare(ui, &mut selected));
            });
            assert_eq!(
                glyph_min_x_in(&ctx, rect),
                None,
                "an out-of-range selection must draw no item text"
            );

            let popup = click_open(&mut ctx, rect, &mut selected);
            for i in 0..3 {
                let row = row_rect(&ctx, popup, i);
                assert!(overlay_has_glyphs_in(&ctx, row), "row {i} must emit");
                assert_eq!(
                    overlay_fill_color_at(&ctx, row),
                    None,
                    "no row may carry the selection marker"
                );
            }
        }

        #[test]
        fn popup_origin_is_clamped_into_the_viewport_horizontally() {
            let mut ctx = test_context();
            let mut selected = 0usize;
            let viewport = [260.0, 400.0];
            let input = |pos: [f32; 2], down: bool| InputState {
                mouse_pos: pos,
                mouse_down: [down, false, false, false, false],
                viewport_size: viewport,
                ..Default::default()
            };
            let mut button = [0.0; 4];
            frame_with_input(&mut ctx, 0.016, input(AWAY, false), |ui| {
                button = button_rect(ui, "Red");
                declare(ui, &mut selected);
            });
            ctx.windows[0].pos = [200.0, 60.0];
            frame_with_input(&mut ctx, 0.016, input(AWAY, false), |ui| {
                button = button_rect(ui, "Red");
                declare(ui, &mut selected);
            });

            let at = center(button);
            frame_with_input(&mut ctx, 0.016, input(at, true), |ui| {
                declare(ui, &mut selected);
            });
            frame_with_input(&mut ctx, 0.016, input(at, false), |ui| {
                declare(ui, &mut selected);
            });
            let popup = ctx.popup_rect.expect("the popup must open");

            let style = ctx.style;
            let widest = ctx.font.measure("Green")[0];
            let w = (button[2] - button[0])
                .max(widest + 2.0 * style.frame_padding[0] + 2.0 * style.window_padding[0]);
            assert!(
                button[0] + w > viewport[0],
                "the unclamped popup must overflow the viewport for this test"
            );
            assert_eq!(popup[2] - popup[0], w, "the width itself fits");
            assert_eq!(popup[0], viewport[0] - w, "origin shifted left to fit");
            assert_eq!(popup[1], button[3], "still anchored below the button");
            assert!(popup[3] <= viewport[1]);
        }
    }

    mod color_edit {
        use crate::context::Context;
        use crate::input::{InputState, Key, KeyEvent};
        use crate::ui::{
            Ui, COLOR_EDIT_CHECKER_DARK, COLOR_EDIT_CHECKER_LIGHT, COLOR_EDIT_POPUP_MIN_WIDTH,
            COLOR_EDIT_PREVIEW_ROWS,
        };
        use crate::widgets::test_support::*;

        const AWAY: [f32; 2] = [-50.0, -50.0];
        const CHANNELS: [&str; 4] = ["R", "G", "B", "A"];

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn swatch_rect(ui: &mut Ui) -> [f32; 4] {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let style = ui.ctx.style;
            let side = ui.ctx.font.line_height() + 2.0 * style.frame_padding[1];
            [pos[0], pos[1], pos[0] + side, pos[1] + side]
        }

        fn declare(ui: &mut Ui, value: &mut [u8; 4]) -> bool {
            ui.color_edit_rgba("Tint", value)
        }

        fn settle(ctx: &mut Context, value: &mut [u8; 4]) -> [f32; 4] {
            let mut rect = [0.0; 4];
            frame(ctx, AWAY, false, |ui| {
                rect = swatch_rect(ui);
                declare(ui, value);
            });
            rect
        }

        fn click_open(ctx: &mut Context, swatch: [f32; 4], value: &mut [u8; 4]) -> [f32; 4] {
            let at = center(swatch);
            frame(ctx, at, true, |ui| {
                declare(ui, value);
            });
            frame(ctx, at, false, |ui| {
                declare(ui, value);
            });
            ctx.popup_rect
                .expect("the popup must be open after the click")
        }

        fn row_height(ctx: &Context) -> f32 {
            ctx.font.line_height() + 2.0 * ctx.style.frame_padding[1]
        }

        fn track_rect(ctx: &mut Context, popup: [f32; 4], i: usize) -> [f32; 4] {
            let style = ctx.style;
            let h = row_height(ctx);
            let y = popup[1] + style.window_padding[1] + i as f32 * (h + style.item_spacing[1]);
            let content_w = popup[2] - popup[0] - 2.0 * style.window_padding[0];
            let track_w =
                (content_w - ctx.font.measure(CHANNELS[i])[0] - style.item_spacing[0]).max(0.0);
            let x = popup[0] + style.window_padding[0];
            [x, y, x + track_w, y + h]
        }

        fn grab_rect(ctx: &Context, track: [f32; 4], v: u8) -> [f32; 4] {
            let style = ctx.style;
            let track_w = track[2] - track[0];
            let grab_w = style.slider_grab_size.min(track_w);
            let t = (v as f32 / 255.0).clamp(0.0, 1.0);
            let x = (track[0] + t * track_w - grab_w / 2.0).clamp(track[0], track[2] - grab_w);
            [x, track[1], x + grab_w, track[3]]
        }

        fn preview_rect(ctx: &Context, popup: [f32; 4]) -> [f32; 4] {
            let style = ctx.style;
            let h = row_height(ctx);
            let y = popup[1] + style.window_padding[1] + 4.0 * (h + style.item_spacing[1]);
            [
                popup[0] + style.window_padding[0],
                y,
                popup[2] - style.window_padding[0],
                y + COLOR_EDIT_PREVIEW_ROWS * h,
            ]
        }

        fn overlay_fill_color_at(ctx: &Context, rect: [f32; 4]) -> Option<[u8; 4]> {
            crate::widgets::test_support::chrome_color_in(ctx.overlay.as_draw_data(), rect)
        }

        fn overlay_colors_in(ctx: &Context, rect: [f32; 4]) -> alloc::vec::Vec<[u8; 4]> {
            ctx.overlay
                .as_draw_data()
                .vertices
                .iter()
                .filter(|v| {
                    v.pos[0] >= rect[0]
                        && v.pos[0] <= rect[2]
                        && v.pos[1] >= rect[1]
                        && v.pos[1] <= rect[3]
                })
                .map(|v| v.col)
                .collect()
        }

        fn window_colors_in(ctx: &Context, rect: [f32; 4]) -> alloc::vec::Vec<[u8; 4]> {
            ctx.windows[0]
                .draw_list
                .as_draw_data()
                .vertices
                .iter()
                .filter(|v| {
                    v.pos[0] >= rect[0]
                        && v.pos[0] <= rect[2]
                        && v.pos[1] >= rect[1]
                        && v.pos[1] <= rect[3]
                })
                .map(|v| v.col)
                .collect()
        }

        fn overlay_has_glyphs_in(ctx: &Context, rect: [f32; 4]) -> bool {
            let data = ctx.overlay.as_draw_data();
            let mut offset = 0usize;
            for cmd in data.commands {
                let next = offset + cmd.elem_count as usize;
                if cmd.texture == crate::draw::TextureId::FONT {
                    for &i in &data.indices[offset..next] {
                        let v = data.vertices[i as usize];
                        if v.pos[0] >= rect[0]
                            && v.pos[0] <= rect[2]
                            && v.pos[1] >= rect[1]
                            && v.pos[1] <= rect[3]
                        {
                            return true;
                        }
                    }
                }
                offset = next;
            }
            false
        }

        fn channel_id(edit_label: &str, channel: &str) -> crate::id::Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            let popup_id = stack.id(edit_label);
            stack.push_id(popup_id);
            stack.id(channel)
        }

        #[test]
        fn swatch_shows_the_color_over_a_checkerboard_with_the_label_right() {
            let mut ctx = test_context();
            let mut value = [200, 30, 60, 128];
            let swatch = settle(&mut ctx, &mut value);

            let colors = window_colors_in(&ctx, swatch);
            assert!(
                colors.contains(&COLOR_EDIT_CHECKER_LIGHT)
                    && colors.contains(&COLOR_EDIT_CHECKER_DARK),
                "both checker grays must be emitted under the swatch"
            );
            assert!(
                colors.contains(&value),
                "the swatch must emit the current color over the checker"
            );

            let style = ctx.style;
            let label_region = [
                swatch[2] + style.item_spacing[0],
                swatch[1],
                swatch[2] + style.item_spacing[0] + ctx.font.measure("Tint")[0] + 2.0,
                swatch[3],
            ];
            assert!(
                !glyph_colors_in(&ctx, label_region).is_empty(),
                "the label text must be drawn right of the swatch"
            );
        }

        #[test]
        fn press_opens_popup_with_four_sliders_at_current_values_and_a_preview() {
            let mut ctx = test_context();
            let mut value = [10, 80, 150, 220];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            assert_eq!(ctx.open_popup, Some(widget_id_in_w("Tint")));
            let style = ctx.style;

            assert_eq!([popup[0], popup[1]], [swatch[0], swatch[3]]);

            for (i, channel) in CHANNELS.iter().enumerate() {
                let track = track_rect(&mut ctx, popup, i);
                assert_eq!(
                    overlay_fill_color_at(&ctx, track),
                    Some(style.widget_bg),
                    "channel {channel}'s slider track must be emitted"
                );
                assert_eq!(
                    overlay_fill_color_at(&ctx, grab_rect(&ctx, track, value[i])),
                    Some(style.widget_bg),
                    "channel {channel}'s grab must sit at the current value"
                );
                let row = [
                    track[0],
                    track[1],
                    popup[2] - style.window_padding[0],
                    track[3],
                ];
                assert!(
                    overlay_has_glyphs_in(&ctx, row),
                    "channel {channel}'s row must emit its value and label text"
                );
            }

            let preview = preview_rect(&ctx, popup);
            assert!(preview[2] - preview[0] > swatch[2] - swatch[0]);
            let colors = overlay_colors_in(&ctx, preview);
            assert!(
                colors.contains(&COLOR_EDIT_CHECKER_LIGHT)
                    && colors.contains(&COLOR_EDIT_CHECKER_DARK),
                "the preview must emit the checkerboard"
            );
            assert!(
                colors.contains(&value),
                "the preview must emit the current color"
            );
        }

        #[test]
        fn dragging_r_writes_through_returns_true_and_never_closes_the_popup() {
            let mut ctx = test_context();
            let mut value = [0, 64, 128, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let track = track_rect(&mut ctx, popup, 0);
            let r_id = channel_id("Tint", "R");
            let mut changed = false;

            frame(&mut ctx, center(track), true, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(changed, "the press frame writes through immediately");
            assert_eq!(value[0], 128);
            assert!(
                ctx.open_popup.is_some(),
                "a press inside the popup keeps it open"
            );
            assert_eq!(ctx.active, Some(r_id));

            let off = [track[0] + 0.75 * (track[2] - track[0]), popup[3] + 120.0];
            frame(&mut ctx, off, true, |ui| changed = declare(ui, &mut value));
            assert!(changed);
            assert_eq!(value[0], 191, "0.75 * 255 = 191.25 rounds to 191");
            assert!(
                ctx.open_popup.is_some(),
                "leaving the popup rect mid-drag must not close it"
            );
            assert_eq!(ctx.active, Some(r_id));

            let preview = preview_rect(&ctx, popup);
            let colors = overlay_colors_in(&ctx, preview);
            assert!(
                colors.contains(&value),
                "the preview must show the dragged color {value:?}"
            );

            frame(&mut ctx, center(preview), false, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(!changed, "the release frame changes nothing");
            assert_eq!(ctx.active, None, "the release drops the capture");
            assert!(
                ctx.open_popup.is_some(),
                "a release inside the popup must not close it"
            );
            assert_eq!(value, [191, 64, 128, 255]);
        }

        #[test]
        fn click_outside_closes_the_popup_and_keeps_the_edited_value() {
            let mut ctx = test_context();
            let mut value = [0, 64, 128, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let track = track_rect(&mut ctx, popup, 0);

            frame(&mut ctx, center(track), true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, center(track), false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value[0], 128);
            assert!(ctx.open_popup.is_some());

            let mut changed = true;
            frame(&mut ctx, AWAY, true, |ui| changed = declare(ui, &mut value));
            assert!(!changed);
            assert_eq!(
                ctx.open_popup, None,
                "an outside press must close the popup"
            );
            assert_eq!(value, [128, 64, 128, 255], "the edited value is kept");
            frame(&mut ctx, AWAY, false, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(!changed);
            assert_eq!(ctx.open_popup, None, "the consumed press must not reopen");
            assert_eq!(value, [128, 64, 128, 255]);
        }

        #[test]
        fn clicking_the_swatch_while_open_closes_the_popup_and_a_third_click_reopens() {
            let mut ctx = test_context();
            let mut value = [0, 64, 128, 255];
            let swatch = settle(&mut ctx, &mut value);
            click_open(&mut ctx, swatch, &mut value);
            assert!(ctx.open_popup.is_some());

            let at = center(swatch);
            frame(&mut ctx, at, true, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(ctx.open_popup, None, "the outside press closes the popup");
            frame(&mut ctx, at, false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(
                ctx.open_popup, None,
                "the consumed press must not reopen the popup on release"
            );

            frame(&mut ctx, at, true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, at, false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(
                ctx.open_popup,
                Some(widget_id_in_w("Tint")),
                "a fresh click reopens the popup"
            );
            assert_eq!(value, [0, 64, 128, 255], "toggling never touches the value");
        }

        #[test]
        fn escape_closes_the_popup_and_keeps_the_edited_value() {
            let mut ctx = test_context();
            let mut value = [0, 64, 128, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let track = track_rect(&mut ctx, popup, 0);

            frame(&mut ctx, center(track), true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, center(track), false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value[0], 128);
            assert!(ctx.open_popup.is_some());

            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    keys: &[KeyEvent::Press(Key::Escape)],
                    ..Default::default()
                },
                |ui| {
                    declare(ui, &mut value);
                },
            );
            assert_eq!(ctx.open_popup, None, "Escape must close the popup");
            assert_eq!(value, [128, 64, 128, 255], "the edited value is kept");
        }

        #[test]
        fn hidden_id_suffixes_make_two_color_edits_independent() {
            let mut ctx = test_context();
            let mut a = [1, 2, 3, 4];
            let mut b = [200, 201, 202, 203];
            let mut swatch_a = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                swatch_a = swatch_rect(ui);
                ui.color_edit_rgba("Tint##a", &mut a);
                ui.color_edit_rgba("Tint##b", &mut b);
            });
            let declare_ab = |ui: &mut Ui, a: &mut [u8; 4], b: &mut [u8; 4]| {
                ui.color_edit_rgba("Tint##a", a);
                ui.color_edit_rgba("Tint##b", b);
            };

            let at = center(swatch_a);
            frame(&mut ctx, at, true, |ui| {
                declare_ab(ui, &mut a, &mut b);
            });
            frame(&mut ctx, at, false, |ui| {
                declare_ab(ui, &mut a, &mut b);
            });
            assert_eq!(ctx.open_popup, Some(widget_id_in_w("Tint##a")));
            assert_ne!(ctx.open_popup, Some(widget_id_in_w("Tint##b")));

            let popup = ctx.popup_rect.expect("a's popup must be open");
            let track = track_rect(&mut ctx, popup, 0);
            let mut changed = false;
            frame(&mut ctx, center(track), true, |ui| {
                changed = ui.color_edit_rgba("Tint##a", &mut a);
                ui.color_edit_rgba("Tint##b", &mut b);
            });
            assert!(changed);
            assert_eq!(a[0], 128);
            assert_eq!(b, [200, 201, 202, 203], "color edit b is untouched");
            assert_eq!(ctx.active, Some(channel_id("Tint##a", "R")));
            frame(&mut ctx, center(track), false, |ui| {
                declare_ab(ui, &mut a, &mut b);
            });
            assert!(ctx.open_popup.is_some());
        }

        #[test]
        fn popup_is_size_capped_and_origin_shifted_into_a_known_viewport() {
            let mut ctx = test_context();
            let mut value = [10, 80, 150, 220];
            let viewport = [250.0, 180.0];
            let input = |pos: [f32; 2], down: bool| InputState {
                mouse_pos: pos,
                mouse_down: [down, false, false, false, false],
                viewport_size: viewport,
                ..Default::default()
            };
            let mut swatch = [0.0; 4];
            frame_with_input(&mut ctx, 0.016, input(AWAY, false), |ui| {
                swatch = swatch_rect(ui);
                declare(ui, &mut value);
            });
            ctx.windows[0].pos = [150.0, 120.0];
            frame_with_input(&mut ctx, 0.016, input(AWAY, false), |ui| {
                swatch = swatch_rect(ui);
                declare(ui, &mut value);
            });

            let at = center(swatch);
            frame_with_input(&mut ctx, 0.016, input(at, true), |ui| {
                declare(ui, &mut value);
            });
            frame_with_input(&mut ctx, 0.016, input(at, false), |ui| {
                declare(ui, &mut value);
            });
            let popup = ctx.popup_rect.expect("the popup must open");

            let style = ctx.style;
            let h = row_height(&ctx);
            let item_w = h + style.item_spacing[0] + ctx.font.measure("Tint")[0];
            let natural_w = item_w.max(COLOR_EDIT_POPUP_MIN_WIDTH);
            let natural_h = 2.0 * style.window_padding[1]
                + 4.0 * h
                + COLOR_EDIT_PREVIEW_ROWS * h
                + 4.0 * style.item_spacing[1];
            assert!(
                swatch[0] + natural_w > viewport[0] && swatch[3] + natural_h > viewport[1],
                "the unclamped popup must overflow the viewport on both axes"
            );

            assert_eq!(popup[2] - popup[0], natural_w, "the width itself fits");
            assert_eq!(
                popup[0],
                viewport[0] - natural_w,
                "origin shifted left to fit"
            );
            assert_eq!(
                popup[3] - popup[1],
                viewport[1],
                "the height is capped at the viewport"
            );
            assert_eq!(popup[1], 0.0, "origin shifted up to fit");
            assert!(popup[0] >= 0.0 && popup[2] <= viewport[0]);
            assert!(popup[3] <= viewport[1]);
        }
    }

    mod color_picker {
        use crate::context::{Context, WidgetState};
        use crate::input::{InputState, Key, KeyEvent};
        use crate::ui::{
            Ui, COLOR_EDIT_CHECKER_DARK, COLOR_EDIT_CHECKER_LIGHT, COLOR_EDIT_PREVIEW_ROWS,
            COLOR_PICKER_COLUMN_WIDTH, COLOR_PICKER_HUE_STRIP_WIDTH, COLOR_PICKER_SV_WIDTH_RATIO,
            DRAG_MARKER_CORE,
        };
        use crate::widgets::test_support::*;

        const AWAY: [f32; 2] = [-50.0, -50.0];
        const SLIDERS: [&str; 7] = ["H", "S", "B##bright", "R", "G", "B##blue", "A"];

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn swatch_rect(ui: &mut Ui) -> [f32; 4] {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let style = ui.ctx.style;
            let side = ui.ctx.font.line_height() + 2.0 * style.frame_padding[1];
            [pos[0], pos[1], pos[0] + side, pos[1] + side]
        }

        fn declare(ui: &mut Ui, value: &mut [u8; 4]) -> bool {
            ui.color_picker_rgba("Tint", value)
        }

        fn settle(ctx: &mut Context, value: &mut [u8; 4]) -> [f32; 4] {
            let mut rect = [0.0; 4];
            frame(ctx, AWAY, false, |ui| {
                rect = swatch_rect(ui);
                declare(ui, value);
            });
            rect
        }

        fn click_open(ctx: &mut Context, swatch: [f32; 4], value: &mut [u8; 4]) -> [f32; 4] {
            let at = center(swatch);
            frame(ctx, at, true, |ui| {
                declare(ui, value);
            });
            frame(ctx, at, false, |ui| {
                declare(ui, value);
            });
            ctx.popup_rect
                .expect("the popup must be open after the click")
        }

        fn row_height(ctx: &Context) -> f32 {
            ctx.font.line_height() + 2.0 * ctx.style.frame_padding[1]
        }

        fn content_origin(ctx: &Context, popup: [f32; 4]) -> [f32; 2] {
            let style = ctx.style;
            [
                popup[0] + style.window_padding[0],
                popup[1] + style.window_padding[1],
            ]
        }

        fn sv_height(ctx: &Context) -> f32 {
            (COLOR_EDIT_PREVIEW_ROWS + 5.0) * row_height(ctx) + 5.0 * ctx.style.item_spacing[1]
        }

        fn sv_width(ctx: &Context) -> f32 {
            COLOR_PICKER_SV_WIDTH_RATIO * sv_height(ctx)
        }

        fn sv_rect(ctx: &Context, popup: [f32; 4]) -> [f32; 4] {
            let o = content_origin(ctx, popup);
            [o[0], o[1], o[0] + sv_width(ctx), o[1] + sv_height(ctx)]
        }

        fn strip_rect(ctx: &Context, popup: [f32; 4]) -> [f32; 4] {
            let style = ctx.style;
            let o = content_origin(ctx, popup);
            let x = o[0] + sv_width(ctx) + style.item_spacing[0];
            [
                x,
                o[1],
                x + COLOR_PICKER_HUE_STRIP_WIDTH,
                o[1] + sv_height(ctx),
            ]
        }

        fn column_origin(ctx: &Context, popup: [f32; 4]) -> [f32; 2] {
            let style = ctx.style;
            let o = content_origin(ctx, popup);
            [
                o[0] + sv_width(ctx)
                    + style.item_spacing[0]
                    + COLOR_PICKER_HUE_STRIP_WIDTH
                    + style.item_spacing[0],
                o[1],
            ]
        }

        fn narrow_width(ctx: &Context) -> f32 {
            (COLOR_PICKER_COLUMN_WIDTH - ctx.style.item_spacing[0]) / 2.0
        }

        fn narrow_track_width(ctx: &mut Context) -> f32 {
            let gutter = ["R", "G", "B", "H", "S"]
                .iter()
                .map(|label| ctx.font.measure(label)[0])
                .fold(0.0_f32, f32::max);
            narrow_width(ctx) - gutter - ctx.style.item_spacing[0]
        }

        fn slider_area_top(ctx: &Context, popup: [f32; 4]) -> f32 {
            column_origin(ctx, popup)[1]
                + COLOR_EDIT_PREVIEW_ROWS * row_height(ctx)
                + ctx.style.item_spacing[1]
        }

        fn track_rect(ctx: &mut Context, popup: [f32; 4], i: usize) -> [f32; 4] {
            let style = ctx.style;
            let h = row_height(ctx);
            let narrow = narrow_width(ctx);
            let column_x = column_origin(ctx, popup)[0];
            let (x, row, track_w) = if i < 3 {
                (
                    column_x + narrow + style.item_spacing[0],
                    i,
                    narrow_track_width(ctx),
                )
            } else if i < 6 {
                (column_x, i - 3, narrow_track_width(ctx))
            } else {
                let w =
                    COLOR_PICKER_COLUMN_WIDTH - ctx.font.measure("A")[0] - style.item_spacing[0];
                (column_x, 3, w)
            };
            let y = slider_area_top(ctx, popup) + row as f32 * (h + style.item_spacing[1]);
            [x, y, x + track_w, y + h]
        }

        fn grab_at(ctx: &Context, track: [f32; 4], t: f32) -> [f32; 4] {
            let style = ctx.style;
            let track_w = track[2] - track[0];
            let grab_w = style.slider_grab_size.min(track_w);
            let x = (track[0] + t.clamp(0.0, 1.0) * track_w - grab_w / 2.0)
                .clamp(track[0], track[2] - grab_w);
            [x, track[1], x + grab_w, track[3]]
        }

        fn preview_rect(ctx: &Context, popup: [f32; 4]) -> [f32; 4] {
            let o = column_origin(ctx, popup);
            [
                o[0],
                o[1],
                o[0] + COLOR_PICKER_COLUMN_WIDTH,
                o[1] + COLOR_EDIT_PREVIEW_ROWS * row_height(ctx),
            ]
        }

        fn preview_current_rect(ctx: &Context, popup: [f32; 4]) -> [f32; 4] {
            let p = preview_rect(ctx, popup);
            [p[0], p[1], (p[0] + p[2]) / 2.0, p[3]]
        }

        fn preview_original_rect(ctx: &Context, popup: [f32; 4]) -> [f32; 4] {
            let p = preview_rect(ctx, popup);
            [(p[0] + p[2]) / 2.0, p[1], p[2], p[3]]
        }

        fn overlay_fill_color_at(ctx: &Context, rect: [f32; 4]) -> Option<[u8; 4]> {
            crate::widgets::test_support::chrome_color_in(ctx.overlay.as_draw_data(), rect)
        }

        fn overlay_colors_in(ctx: &Context, rect: [f32; 4]) -> alloc::vec::Vec<[u8; 4]> {
            ctx.overlay
                .as_draw_data()
                .vertices
                .iter()
                .filter(|v| {
                    v.pos[0] >= rect[0]
                        && v.pos[0] <= rect[2]
                        && v.pos[1] >= rect[1]
                        && v.pos[1] <= rect[3]
                })
                .map(|v| v.col)
                .collect()
        }

        fn window_colors_in(ctx: &Context, rect: [f32; 4]) -> alloc::vec::Vec<[u8; 4]> {
            ctx.windows[0]
                .draw_list
                .as_draw_data()
                .vertices
                .iter()
                .filter(|v| {
                    v.pos[0] >= rect[0]
                        && v.pos[0] <= rect[2]
                        && v.pos[1] >= rect[1]
                        && v.pos[1] <= rect[3]
                })
                .map(|v| v.col)
                .collect()
        }

        fn vertices_of_color(data: crate::draw::DrawData<'_>, col: [u8; 4]) -> usize {
            data.vertices.iter().filter(|v| v.col == col).count()
        }

        fn overlay_has_glyphs_in(ctx: &Context, rect: [f32; 4]) -> bool {
            let data = ctx.overlay.as_draw_data();
            let mut offset = 0usize;
            for cmd in data.commands {
                let next = offset + cmd.elem_count as usize;
                if cmd.texture == crate::draw::TextureId::FONT {
                    for &i in &data.indices[offset..next] {
                        let v = data.vertices[i as usize];
                        if v.pos[0] >= rect[0]
                            && v.pos[0] <= rect[2]
                            && v.pos[1] >= rect[1]
                            && v.pos[1] <= rect[3]
                        {
                            return true;
                        }
                    }
                }
                offset = next;
            }
            false
        }

        fn scoped_id(picker_label: &str, widget_label: &str) -> crate::id::Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            let popup_id = stack.id(picker_label);
            stack.push_id(popup_id);
            stack.id(widget_label)
        }

        fn hsv_state(ctx: &Context, picker_label: &str) -> (f32, f32, f32) {
            match ctx.widget_states.get(&widget_id_in_w(picker_label)) {
                Some(WidgetState::ColorPicker { h, s, v, .. }) => (*h, *s, *v),
                _ => panic!("the picker must hold ColorPicker widget state"),
            }
        }

        fn hex_buffer(ctx: &Context, picker_label: &str) -> alloc::string::String {
            match ctx.widget_states.get(&widget_id_in_w(picker_label)) {
                Some(WidgetState::ColorPicker { hex, .. }) => hex.clone(),
                _ => panic!("the picker must hold ColorPicker widget state"),
            }
        }

        fn original_state(ctx: &Context, picker_label: &str) -> [u8; 4] {
            match ctx.widget_states.get(&widget_id_in_w(picker_label)) {
                Some(WidgetState::ColorPicker { original, .. }) => *original,
                _ => panic!("the picker must hold ColorPicker widget state"),
            }
        }

        fn hex_field_rect(ctx: &mut Context, popup: [f32; 4]) -> [f32; 4] {
            let style = ctx.style;
            let h = row_height(ctx);
            let y = slider_area_top(ctx, popup) + 4.0 * (h + style.item_spacing[1]);
            let x = column_origin(ctx, popup)[0];
            [x, y, x + COLOR_PICKER_COLUMN_WIDTH, y + h]
        }

        fn keyed(keys: &[KeyEvent]) -> InputState<'_> {
            InputState {
                mouse_pos: AWAY,
                keys,
                ..Default::default()
            }
        }

        fn focus_clear_and_type(
            ctx: &mut Context,
            field: [f32; 4],
            value: &mut [u8; 4],
            text: &str,
        ) {
            let at = [field[2] - 1.0, center(field)[1]];
            frame(ctx, at, true, |ui| {
                declare(ui, value);
            });
            frame(ctx, at, false, |ui| {
                declare(ui, value);
            });
            let mut keys = alloc::vec![KeyEvent::Press(Key::Backspace); 9];
            keys.extend(text.chars().map(KeyEvent::Char));
            frame_with_input(ctx, 0.016, keyed(&keys), |ui| {
                declare(ui, value);
            });
        }

        #[test]
        fn swatch_shows_the_color_over_a_checkerboard_with_the_label_right() {
            let mut ctx = test_context();
            let mut value = [200, 30, 60, 128];
            let swatch = settle(&mut ctx, &mut value);

            let colors = window_colors_in(&ctx, swatch);
            assert!(
                colors.contains(&COLOR_EDIT_CHECKER_LIGHT)
                    && colors.contains(&COLOR_EDIT_CHECKER_DARK),
                "both checker grays must be emitted under the swatch"
            );
            assert!(
                colors.contains(&value),
                "the swatch must emit the current color over the checker"
            );

            let style = ctx.style;
            let label_region = [
                swatch[2] + style.item_spacing[0],
                swatch[1],
                swatch[2] + style.item_spacing[0] + ctx.font.measure("Tint")[0] + 2.0,
                swatch[3],
            ];
            assert!(
                !glyph_colors_in(&ctx, label_region).is_empty(),
                "the label text must be drawn right of the swatch"
            );
        }

        #[test]
        fn press_opens_popup_with_all_sections_at_seeded_values() {
            let mut ctx = test_context();
            let mut value = [0, 255, 0, 200];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            assert_eq!(ctx.open_popup, Some(widget_id_in_w("Tint")));
            let style = ctx.style;

            assert_eq!([popup[0], popup[1]], [swatch[0], swatch[3]]);

            let sv = sv_rect(&ctx, popup);
            let colors = overlay_colors_in(&ctx, sv);
            for corner in [[255, 255, 255, 255], [0, 255, 0, 255], [0, 0, 0, 255]] {
                assert!(
                    colors.contains(&corner),
                    "the SV gradient must emit its corner color {corner:?}"
                );
            }

            let strip = strip_rect(&ctx, popup);
            let colors = overlay_colors_in(&ctx, strip);
            assert!(
                colors.contains(&[255, 0, 0, 255]) && colors.contains(&[255, 0, 255, 255]),
                "the hue strip must emit its boundary colors"
            );

            let ts = [120.0 / 360.0, 1.0, 1.0, 0.0, 1.0, 0.0, 200.0 / 255.0];
            for (i, t) in ts.iter().enumerate() {
                let track = track_rect(&mut ctx, popup, i);
                assert_eq!(
                    overlay_fill_color_at(&ctx, track),
                    Some(style.widget_bg),
                    "slider {i}'s track must be emitted"
                );
                assert_eq!(
                    overlay_fill_color_at(&ctx, grab_at(&ctx, track, *t)),
                    Some(style.widget_bg),
                    "slider {i}'s grab must sit at the seeded value (t = {t})"
                );
                let row = [
                    track[0],
                    track[1],
                    popup[2] - style.window_padding[0],
                    track[3],
                ];
                assert!(
                    overlay_has_glyphs_in(&ctx, row),
                    "slider {i}'s row must emit its value and label text"
                );
            }

            let preview = preview_rect(&ctx, popup);
            assert!(preview[2] - preview[0] > swatch[2] - swatch[0]);
            let colors = overlay_colors_in(&ctx, preview);
            assert!(
                colors.contains(&COLOR_EDIT_CHECKER_LIGHT)
                    && colors.contains(&COLOR_EDIT_CHECKER_DARK),
                "the preview must emit the checkerboard"
            );
            assert!(
                colors.contains(&value),
                "the preview must emit the current color"
            );
        }

        #[test]
        fn popup_is_sized_to_the_three_sections_per_the_contract() {
            let mut ctx = test_context();
            let mut value = [0, 255, 0, 200];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let style = ctx.style;
            let h = row_height(&ctx);

            let item_w = h + style.item_spacing[0] + ctx.font.measure("Tint")[0];
            let sections_w = sv_width(&ctx)
                + style.item_spacing[0]
                + COLOR_PICKER_HUE_STRIP_WIDTH
                + style.item_spacing[0]
                + COLOR_PICKER_COLUMN_WIDTH
                + 2.0 * style.window_padding[0];
            assert_eq!(popup[2] - popup[0], item_w.max(sections_w));

            let expected_h = 2.0 * style.window_padding[1] + sv_height(&ctx);
            assert_eq!(popup[3] - popup[1], expected_h);

            let o = content_origin(&ctx, popup);
            let strip = strip_rect(&ctx, popup);
            let column = column_origin(&ctx, popup);
            assert_eq!(column[0], strip[2] + style.item_spacing[0]);
            assert_eq!(column[1], o[1]);

            let sv = sv_rect(&ctx, popup);
            assert_eq!(strip[0], sv[2] + style.item_spacing[0]);
            assert_eq!(
                column[0] + COLOR_PICKER_COLUMN_WIDTH,
                popup[2] - style.window_padding[0],
                "the column's right edge is the content right edge"
            );

            let hex = hex_field_rect(&mut ctx, popup);
            assert!(
                (sv[3] - hex[3]).abs() < 1e-4 && (strip[3] - hex[3]).abs() < 1e-4,
                "SV bottom {}, strip bottom {}, hex bottom {} must coincide",
                sv[3],
                strip[3],
                hex[3]
            );
        }

        #[test]
        fn slider_rows_pair_two_narrow_sliders_side_by_side() {
            let mut ctx = test_context();
            let mut value = [10, 20, 30, 40];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let style = ctx.style;
            let narrow = narrow_width(&ctx);
            let narrow_track = narrow_track_width(&mut ctx);
            let column_x = column_origin(&ctx, popup)[0];

            for row in 0..3 {
                let left = track_rect(&mut ctx, popup, row + 3);
                let right = track_rect(&mut ctx, popup, row);
                assert_eq!(left[1], right[1], "row {row}: the pair must share its y");
                assert_eq!(
                    left[3], right[3],
                    "row {row}: the pair must share its height"
                );
                assert_eq!(
                    left[0], column_x,
                    "row {row}: the left slider starts the column"
                );
                assert_eq!(
                    right[0],
                    column_x + narrow + style.item_spacing[0],
                    "row {row}: the right slider sits one spacing past the left sub-region"
                );
                assert!(
                    ((left[2] - left[0]) - (right[2] - right[0])).abs() < 1e-4,
                    "row {row}: paired tracks must be equal width ({} vs {})",
                    left[2] - left[0],
                    right[2] - right[0]
                );
                for (i, track) in [(row + 3, left), (row, right)] {
                    assert!(
                        ((track[2] - track[0]) - narrow_track).abs() < 1e-4,
                        "slider {}'s track must be the uniform narrow width, got {}",
                        SLIDERS[i],
                        track[2] - track[0]
                    );
                    assert_eq!(
                        overlay_fill_color_at(&ctx, track),
                        Some(style.widget_bg),
                        "slider {}'s track must be emitted at its narrow rect",
                        SLIDERS[i]
                    );
                }
            }

            let a = track_rect(&mut ctx, popup, 6);
            let b = track_rect(&mut ctx, popup, 2);
            let h = row_height(&ctx);
            assert_eq!(a[0], column_x, "A starts the column");
            assert_eq!(
                a[1],
                b[1] + h + style.item_spacing[1],
                "A takes the row below B|B"
            );
            let a_item_w = (a[2] - a[0]) + style.item_spacing[0] + ctx.font.measure("A")[0];
            assert!(
                (a_item_w - COLOR_PICKER_COLUMN_WIDTH).abs() < 1e-4,
                "A's item must span the full column width, got {a_item_w}"
            );
            assert_eq!(
                overlay_fill_color_at(&ctx, a),
                Some(style.widget_bg),
                "A's track must be emitted at its full-width rect"
            );
        }

        #[test]
        fn all_narrow_sliders_have_exactly_equal_track_widths() {
            let mut ctx = test_context();
            let mut value = [10, 20, 30, 40];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            let mut widths = [0.0; 6];
            for (i, w) in widths.iter_mut().enumerate() {
                let t = track_rect(&mut ctx, popup, i);
                assert_eq!(
                    overlay_fill_color_at(&ctx, t),
                    Some(ctx.style.widget_bg),
                    "slider {}'s track must be emitted at its predicted rect",
                    SLIDERS[i]
                );
                *w = t[2] - t[0];
            }
            for (i, w) in widths.iter().enumerate() {
                assert!(
                    (w - widths[0]).abs() < 1e-4,
                    "slider {}'s track width {w} must equal slider {}'s {}",
                    SLIDERS[i],
                    SLIDERS[0],
                    widths[0]
                );
            }
        }

        #[test]
        fn sv_field_and_strip_match_the_column_content_height() {
            let mut ctx = test_context();
            let mut value = [0, 255, 0, 200];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            let sv = sv_rect(&ctx, popup);
            let strip = strip_rect(&ctx, popup);
            let column = column_origin(&ctx, popup);
            let hex = hex_field_rect(&mut ctx, popup);

            let content_h = hex[3] - column[1];
            assert!(
                ((sv[3] - sv[1]) - content_h).abs() < 1e-4,
                "SV height {} must be the column content height {content_h}",
                sv[3] - sv[1]
            );
            assert!(
                ((strip[3] - strip[1]) - content_h).abs() < 1e-4,
                "strip height {} must be the column content height {content_h}",
                strip[3] - strip[1]
            );
            assert_eq!(sv[1], column[1], "the SV field starts at the content top");
            assert_eq!(strip[1], column[1], "the strip starts at the content top");
            assert!(
                (sv[3] - hex[3]).abs() < 1e-4 && (strip[3] - hex[3]).abs() < 1e-4,
                "the SV field and the strip end flush with the hex row"
            );

            assert!(
                ((sv[2] - sv[0]) - COLOR_PICKER_SV_WIDTH_RATIO * (sv[3] - sv[1])).abs() < 1e-4,
                "SV width {} must be the ratio times its height {}",
                sv[2] - sv[0],
                sv[3] - sv[1]
            );

            let data = ctx.overlay.as_draw_data();
            for (name, rect) in [("SV field", sv), ("hue strip", strip)] {
                for corner in [[rect[0], rect[1]], [rect[2], rect[3]]] {
                    assert!(
                        data.vertices.iter().any(|v| v.pos == corner),
                        "the {name} gradient must emit a vertex at its corner {corner:?}"
                    );
                }
            }
        }

        #[test]
        fn a_narrow_slider_maps_drags_across_its_own_track() {
            let mut ctx = test_context();
            let mut value = [128, 128, 128, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            let track = track_rect(&mut ctx, popup, 1);
            let mut changed = false;
            frame(&mut ctx, center(track), true, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(changed);
            assert_eq!(
                hsv_state(&ctx, "Tint").1,
                0.5,
                "a midpoint drag must set the range midpoint"
            );
            assert_eq!(
                value,
                [128, 64, 64, 255],
                "hsv(0, 0.5, 128/255) rounds to [128, 64, 64]"
            );
            frame(&mut ctx, center(track), false, |ui| {
                declare(ui, &mut value);
            });
        }

        #[test]
        fn hsv_is_seeded_from_the_value_on_open() {
            let mut ctx = test_context();
            let mut value = [128, 128, 128, 77];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let style = ctx.style;

            let (h, s, v) = hsv_state(&ctx, "Tint");
            assert_eq!(h, 0.0);
            assert_eq!(s, 0.0);
            assert_eq!(v, 128.0 / 255.0);

            let ts = [0.0, 0.0, 0.5];
            for (i, t) in ts.iter().enumerate() {
                let track = track_rect(&mut ctx, popup, i);
                assert_eq!(
                    overlay_fill_color_at(&ctx, grab_at(&ctx, track, *t)),
                    Some(style.widget_bg),
                    "HSB slider {i}'s grab must sit at the seeded t = {t}"
                );
            }

            frame(&mut ctx, AWAY, true, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(ctx.open_popup, None, "the outside press closes the popup");
            frame(&mut ctx, AWAY, false, |ui| {
                declare(ui, &mut value);
            });
            value = [255, 0, 0, 255];
            click_open(&mut ctx, swatch, &mut value);
            assert_eq!(
                hsv_state(&ctx, "Tint"),
                (0.0, 1.0, 1.0),
                "a reopen must re-seed from the current value"
            );
        }

        #[test]
        fn rgb_slider_edit_rederives_the_hsb_sliders() {
            let mut ctx = test_context();
            let mut value = [0, 0, 0, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let style = ctx.style;

            let track = track_rect(&mut ctx, popup, 3);
            let mut changed = false;
            frame(&mut ctx, center(track), true, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(changed);
            frame(&mut ctx, [track[2] + 10.0, center(track)[1]], true, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(changed);
            assert_eq!(value, [255, 0, 0, 255]);
            frame(&mut ctx, [track[2] + 10.0, center(track)[1]], false, |ui| {
                declare(ui, &mut value);
            });

            frame(&mut ctx, AWAY, false, |ui| {
                declare(ui, &mut value);
            });
            let ts = [0.0, 1.0, 1.0];
            for (i, t) in ts.iter().enumerate() {
                let track = track_rect(&mut ctx, popup, i);
                assert_eq!(
                    overlay_fill_color_at(&ctx, grab_at(&ctx, track, *t)),
                    Some(style.widget_bg),
                    "HSB slider {i} must reflect the RGB edit (t = {t})"
                );
            }
            assert_eq!(hsv_state(&ctx, "Tint"), (0.0, 1.0, 1.0));
        }

        #[test]
        fn hsb_slider_edit_updates_rgb_sliders_and_value_same_frame() {
            let mut ctx = test_context();
            let mut value = [255, 0, 0, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let style = ctx.style;

            let track = track_rect(&mut ctx, popup, 0);
            let at = [track[0] + (track[2] - track[0]) / 3.0, center(track)[1]];
            let mut changed = false;
            frame(&mut ctx, at, true, |ui| changed = declare(ui, &mut value));
            assert!(changed, "the H edit must report a change on its frame");
            assert_eq!(value, [0, 255, 0, 255], "h = 120° must write pure green");
            assert_eq!(hsv_state(&ctx, "Tint"), (120.0, 1.0, 1.0));

            for (i, t) in [(3, 0.0), (4, 1.0), (5, 0.0)] {
                let track = track_rect(&mut ctx, popup, i);
                assert_eq!(
                    overlay_fill_color_at(&ctx, grab_at(&ctx, track, t)),
                    Some(style.widget_bg),
                    "RGBA slider {i} must reflect the HSB edit same-frame (t = {t})"
                );
            }

            let preview = preview_rect(&ctx, popup);
            assert!(
                overlay_colors_in(&ctx, preview).contains(&[0, 255, 0, 255]),
                "the preview must show the edited color same-frame"
            );
        }

        #[test]
        fn sv_field_drag_updates_value_and_preview_same_frame() {
            let mut ctx = test_context();
            let mut value = [128, 128, 128, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            let sv = sv_rect(&ctx, popup);
            let mid = center(sv);
            let mut changed = false;
            frame(&mut ctx, mid, true, |ui| changed = declare(ui, &mut value));
            assert!(changed, "the press frame writes through immediately");
            assert_eq!(value, [128, 64, 64, 255], "alpha must pass through");
            assert_eq!(ctx.active, Some(scoped_id("Tint", "##sv")));
            assert!(
                ctx.open_popup.is_some(),
                "a press inside the popup keeps it open"
            );

            let preview = preview_rect(&ctx, popup);
            assert!(
                overlay_colors_in(&ctx, preview).contains(&value),
                "the preview must track the SV drag same-frame"
            );

            frame(&mut ctx, mid, true, |ui| changed = declare(ui, &mut value));
            assert!(!changed);
            frame(&mut ctx, mid, false, |ui| changed = declare(ui, &mut value));
            assert!(!changed);
            assert_eq!(ctx.active, None);
            assert_eq!(value, [128, 64, 64, 255]);
        }

        #[test]
        fn original_color_is_snapshotted_on_open() {
            let mut ctx = test_context();
            let mut value = [10, 200, 90, 128];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            assert_eq!(
                original_state(&ctx, "Tint"),
                value,
                "the snapshot must equal the value at open"
            );
            let current = preview_current_rect(&ctx, popup);
            assert!(
                overlay_colors_in(&ctx, current).contains(&value),
                "the current swatch shows the value at open"
            );
            let original = preview_original_rect(&ctx, popup);
            assert!(
                overlay_colors_in(&ctx, original).contains(&value),
                "the original swatch shows the value at open"
            );
        }

        #[test]
        fn both_preview_swatches_render_over_the_checkerboard() {
            let mut ctx = test_context();
            let mut value = [200, 30, 60, 128];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            for (name, rect) in [
                ("current", preview_current_rect(&ctx, popup)),
                ("original", preview_original_rect(&ctx, popup)),
            ] {
                let colors = overlay_colors_in(&ctx, rect);
                assert!(
                    colors.contains(&COLOR_EDIT_CHECKER_LIGHT)
                        && colors.contains(&COLOR_EDIT_CHECKER_DARK),
                    "the {name} swatch must render over the checkerboard"
                );
            }
        }

        #[test]
        fn original_swatch_is_fixed_while_the_current_tracks_edits() {
            let mut ctx = test_context();
            let mut value = [10, 200, 90, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let original = value;

            let track = track_rect(&mut ctx, popup, 3);
            frame(&mut ctx, center(track), true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, [track[2] + 10.0, center(track)[1]], true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, [track[2] + 10.0, center(track)[1]], false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value, [255, 200, 90, 255]);
            assert_eq!(
                original_state(&ctx, "Tint"),
                original,
                "an RGB slider edit must not move the snapshot"
            );
            let current = preview_current_rect(&ctx, popup);
            let current = [current[0], current[1], current[2] - 1.0, current[3]];
            let colors = overlay_colors_in(&ctx, current);
            assert!(
                colors.contains(&value),
                "the current swatch tracks the slider edit"
            );
            assert!(
                !colors.contains(&original),
                "the current swatch no longer shows the original"
            );
            let other = preview_original_rect(&ctx, popup);
            let other = [other[0] + 1.0, other[1], other[2], other[3]];
            let colors = overlay_colors_in(&ctx, other);
            assert!(
                colors.contains(&original),
                "the original swatch keeps the snapshot"
            );
            assert!(
                !colors.contains(&value),
                "the original swatch does not track the edit"
            );

            let sv = sv_rect(&ctx, popup);
            frame(&mut ctx, center(sv), true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, center(sv), false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(
                original_state(&ctx, "Tint"),
                original,
                "an SV edit must not move the snapshot"
            );
            assert!(
                overlay_colors_in(&ctx, preview_current_rect(&ctx, popup)).contains(&value),
                "the current swatch tracks the SV edit"
            );
            assert!(
                overlay_colors_in(&ctx, preview_original_rect(&ctx, popup)).contains(&original),
                "the original swatch still keeps the snapshot"
            );

            let field = hex_field_rect(&mut ctx, popup);
            focus_clear_and_type(&mut ctx, field, &mut value, "#12345678");
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(&[KeyEvent::Press(Key::Enter)]),
                |ui| {
                    declare(ui, &mut value);
                },
            );
            assert_eq!(value, [0x12, 0x34, 0x56, 0x78]);
            assert_eq!(
                original_state(&ctx, "Tint"),
                original,
                "a hex commit must not move the snapshot"
            );
            assert!(
                overlay_colors_in(&ctx, preview_current_rect(&ctx, popup)).contains(&value),
                "the current swatch tracks the hex commit"
            );
            assert!(
                overlay_colors_in(&ctx, preview_original_rect(&ctx, popup)).contains(&original),
                "the original swatch still keeps the snapshot"
            );
        }

        #[test]
        fn reopen_resnapshots_the_original_color() {
            let mut ctx = test_context();
            let mut value = [10, 200, 90, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            let track = track_rect(&mut ctx, popup, 3);
            frame(&mut ctx, center(track), true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, [track[2] + 10.0, center(track)[1]], true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, [track[2] + 10.0, center(track)[1]], false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value, [255, 200, 90, 255]);
            assert_eq!(original_state(&ctx, "Tint"), [10, 200, 90, 255]);

            frame(&mut ctx, AWAY, true, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(ctx.open_popup, None, "the outside press closes the popup");
            frame(&mut ctx, AWAY, false, |ui| {
                declare(ui, &mut value);
            });
            let popup = click_open(&mut ctx, swatch, &mut value);
            assert_eq!(
                original_state(&ctx, "Tint"),
                value,
                "a reopen must re-snapshot from the current value"
            );
            assert!(
                overlay_colors_in(&ctx, preview_original_rect(&ctx, popup)).contains(&value),
                "the original swatch shows the re-snapshotted value"
            );
        }

        #[test]
        fn hue_360_is_preserved_across_rgb_rederivation() {
            let mut ctx = test_context();
            let mut value = [255, 0, 0, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            let strip = strip_rect(&ctx, popup);
            let x = center(strip)[0];
            frame(&mut ctx, [x, strip[3] - 1.0], true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, [x, strip[3] + 10.0], true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, [x, strip[3] + 10.0], false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value, [255, 0, 0, 255]);
            assert_eq!(hsv_state(&ctx, "Tint").0, 360.0, "the drag clamps to 360°");

            let a_track = track_rect(&mut ctx, popup, 6);
            frame(&mut ctx, center(a_track), true, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value[3], 128, "t = 0.5 maps alpha to 128");
            assert_eq!(
                hsv_state(&ctx, "Tint"),
                (360.0, 1.0, 1.0),
                "360 ≡ 0: re-derivation must not teleport the hue"
            );
            frame(&mut ctx, center(a_track), false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(hsv_state(&ctx, "Tint"), (360.0, 1.0, 1.0));
        }

        #[test]
        fn sub_u8_state_moves_that_round_back_to_the_same_rgba_report_no_change() {
            let mut ctx = test_context();
            let mut value = [10, 20, 30, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            let sv = sv_rect(&ctx, popup);
            let mut changed = false;
            frame(&mut ctx, center(sv), true, |ui| {
                changed = declare(ui, &mut value)
            });
            frame(&mut ctx, [center(sv)[0], sv[3] + 10.0], true, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(changed, "reaching v = 0 is a real change to black");
            assert_eq!(value, [0, 0, 0, 255]);
            let (_, s_before, v_before) = hsv_state(&ctx, "Tint");
            assert_eq!(v_before, 0.0, "the off-edge drag clamps v to exactly 0");

            frame(&mut ctx, [sv[2] + 20.0, sv[3] + 10.0], true, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(
                !changed,
                "a state move that rounds back to the same RGBA must report no change"
            );
            assert_eq!(value, [0, 0, 0, 255]);
            let (_, s_after, _) = hsv_state(&ctx, "Tint");
            assert_ne!(
                s_after, s_before,
                "the working state really moved (the case is not vacuous)"
            );
            frame(&mut ctx, [sv[2] + 20.0, sv[3] + 10.0], false, |ui| {
                declare(ui, &mut value);
            });

            let s_track = track_rect(&mut ctx, popup, 1);
            let at = [
                s_track[0] + 0.75 * (s_track[2] - s_track[0]),
                center(s_track)[1],
            ];
            frame(&mut ctx, at, true, |ui| changed = declare(ui, &mut value));
            assert!(
                !changed,
                "an S-slider nudge at v = 0 rounds back to the same RGBA"
            );
            assert_eq!(value, [0, 0, 0, 255]);
            assert_eq!(
                hsv_state(&ctx, "Tint").1,
                0.75,
                "the slider moved the working state"
            );
            frame(&mut ctx, at, false, |ui| {
                declare(ui, &mut value);
            });
        }

        #[test]
        fn alternating_edits_do_not_accumulate_drift() {
            let mut ctx = test_context();
            let mut value = [200, 100, 50, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            let r_track = track_rect(&mut ctx, popup, 3);
            let at = [
                r_track[0] + (100.0 / 255.0) * (r_track[2] - r_track[0]),
                center(r_track)[1],
            ];
            frame(&mut ctx, at, true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, at, false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value, [100, 100, 50, 255]);

            let (h, _, v) = crate::color::rgba_u8_to_hsv(value);
            let s_track = track_rect(&mut ctx, popup, 1);
            let at = [
                s_track[0] + 0.75 * (s_track[2] - s_track[0]),
                center(s_track)[1],
            ];
            frame(&mut ctx, at, true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, at, false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(
                value,
                crate::color::hsv_to_rgba_u8(h, 0.75, v, 255),
                "an HSB edit writes the exact u8 rounding of the HSV state"
            );

            let g_track = track_rect(&mut ctx, popup, 4);
            let at = [
                g_track[0] + (200.0 / 255.0) * (g_track[2] - g_track[0]),
                center(g_track)[1],
            ];
            frame(&mut ctx, at, true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, at, false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value[1], 200);

            let sv = sv_rect(&ctx, popup);
            let (_, s, v) = hsv_state(&ctx, "Tint");
            let marker = [
                sv[0] + s * (sv[2] - sv[0]),
                sv[1] + (1.0 - v) * (sv[3] - sv[1]),
            ];
            let before = value;
            frame(&mut ctx, marker, true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, marker, false, |ui| {
                declare(ui, &mut value);
            });
            let first = value;
            for i in 0..3 {
                assert!(
                    (first[i] as i32 - before[i] as i32).abs() <= 1,
                    "channel {i} drifted past the documented ±1: {before:?} → {first:?}"
                );
            }
            frame(&mut ctx, marker, true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, marker, false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(
                value, first,
                "repeating the cycle must hit a fixed point, not drift"
            );
        }

        #[test]
        fn changed_flag_is_true_exactly_on_frames_where_the_value_changed() {
            let mut ctx = test_context();
            let mut value = [200, 100, 50, 255];
            let mut changed = true;

            frame(&mut ctx, AWAY, false, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(!changed, "an idle frame changes nothing");
            let swatch = swatch_rect_ref(&mut ctx);

            let at = center(swatch);
            frame(&mut ctx, at, true, |ui| changed = declare(ui, &mut value));
            assert!(!changed, "opening the popup changes nothing");
            frame(&mut ctx, at, false, |ui| changed = declare(ui, &mut value));
            assert!(!changed, "the opening release changes nothing");

            let popup = ctx.popup_rect.expect("the popup must be open");
            let sv = sv_rect(&ctx, popup);
            let mid = center(sv);
            frame(&mut ctx, mid, false, |ui| changed = declare(ui, &mut value));
            assert!(!changed, "a hover frame changes nothing");

            frame(&mut ctx, mid, true, |ui| changed = declare(ui, &mut value));
            assert!(changed, "the SV press changes the value");
            frame(&mut ctx, mid, true, |ui| changed = declare(ui, &mut value));
            assert!(!changed, "holding still changes nothing");
            frame(&mut ctx, [mid[0] + 12.0, mid[1]], true, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(changed, "the drag changes the value");
            frame(&mut ctx, [mid[0] + 12.0, mid[1]], false, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(!changed, "the release frame changes nothing");
        }

        fn swatch_rect_ref(ctx: &mut Context) -> [f32; 4] {
            ctx.last_item
                .expect("a picker declaration registers its swatch")
                .1
        }

        #[test]
        fn hidden_id_suffixes_make_two_pickers_independent() {
            let mut ctx = test_context();
            let mut a = [1, 2, 3, 4];
            let mut b = [200, 201, 202, 203];
            let mut swatch_a = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                swatch_a = swatch_rect(ui);
                ui.color_picker_rgba("Tint##a", &mut a);
                ui.color_picker_rgba("Tint##b", &mut b);
            });
            let declare_ab = |ui: &mut Ui, a: &mut [u8; 4], b: &mut [u8; 4]| {
                ui.color_picker_rgba("Tint##a", a);
                ui.color_picker_rgba("Tint##b", b);
            };

            let at = center(swatch_a);
            frame(&mut ctx, at, true, |ui| {
                declare_ab(ui, &mut a, &mut b);
            });
            frame(&mut ctx, at, false, |ui| {
                declare_ab(ui, &mut a, &mut b);
            });
            assert_eq!(ctx.open_popup, Some(widget_id_in_w("Tint##a")));
            assert_ne!(ctx.open_popup, Some(widget_id_in_w("Tint##b")));

            let popup = ctx.popup_rect.expect("a's popup must be open");
            let track = track_rect(&mut ctx, popup, 3);
            let mut changed = false;
            frame(&mut ctx, center(track), true, |ui| {
                changed = ui.color_picker_rgba("Tint##a", &mut a);
                ui.color_picker_rgba("Tint##b", &mut b);
            });
            assert!(changed);
            assert_eq!(a[0], 128);
            assert_eq!(b, [200, 201, 202, 203], "picker b is untouched");
            assert_eq!(ctx.active, Some(scoped_id("Tint##a", "R")));
            let (h, s, v) = hsv_state(&ctx, "Tint##a");
            assert_eq!(
                (h, s, v),
                crate::color::rgba_u8_to_hsv(a),
                "a's state re-derived from a's edited RGB"
            );
            assert!(
                !ctx.widget_states.contains_key(&widget_id_in_w("Tint##b")),
                "b never opened, so b holds no picker state"
            );
            frame(&mut ctx, center(track), false, |ui| {
                declare_ab(ui, &mut a, &mut b);
            });
            assert!(ctx.open_popup.is_some());
        }

        #[test]
        fn click_outside_closes_the_popup_and_keeps_the_edited_value() {
            let mut ctx = test_context();
            let mut value = [0, 64, 128, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let track = track_rect(&mut ctx, popup, 3);

            frame(&mut ctx, center(track), true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, center(track), false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value[0], 128);
            assert!(ctx.open_popup.is_some());

            let mut changed = true;
            frame(&mut ctx, AWAY, true, |ui| changed = declare(ui, &mut value));
            assert!(!changed);
            assert_eq!(
                ctx.open_popup, None,
                "an outside press must close the popup"
            );
            assert_eq!(value, [128, 64, 128, 255], "the edited value is kept");
            frame(&mut ctx, AWAY, false, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(!changed);
            assert_eq!(ctx.open_popup, None, "the consumed press must not reopen");
            assert_eq!(value, [128, 64, 128, 255]);
        }

        #[test]
        fn escape_closes_the_popup_and_keeps_the_edited_value() {
            let mut ctx = test_context();
            let mut value = [0, 64, 128, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let track = track_rect(&mut ctx, popup, 3);

            frame(&mut ctx, center(track), true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, center(track), false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value[0], 128);
            assert!(ctx.open_popup.is_some());

            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    keys: &[KeyEvent::Press(Key::Escape)],
                    ..Default::default()
                },
                |ui| {
                    declare(ui, &mut value);
                },
            );
            assert_eq!(ctx.open_popup, None, "Escape must close the popup");
            assert_eq!(value, [128, 64, 128, 255], "the edited value is kept");
        }

        #[test]
        fn popup_content_routes_to_the_overlay_not_the_window() {
            let mut ctx = test_context();
            let mut value = [0, 255, 0, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            let sv = sv_rect(&ctx, popup);
            assert!(
                !overlay_colors_in(&ctx, sv).is_empty(),
                "the SV field must emit into the overlay"
            );
            assert!(
                window_colors_in(&ctx, sv).is_empty(),
                "no picker geometry may leak into the window draw list"
            );

            let overlay_whites = vertices_of_color(ctx.overlay.as_draw_data(), DRAG_MARKER_CORE);
            assert!(
                overlay_whites >= 8,
                "the marker rings/lines must emit into the overlay, got {overlay_whites} white vertices"
            );
            let window_whites =
                vertices_of_color(ctx.windows[0].draw_list.as_draw_data(), DRAG_MARKER_CORE);
            assert_eq!(
                window_whites, 0,
                "no marker geometry may leak into the window draw list"
            );

            let mid = center(sv);
            frame(&mut ctx, mid, true, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(ctx.active, Some(scoped_id("Tint", "##sv")));
            frame(&mut ctx, mid, false, |ui| {
                declare(ui, &mut value);
            });
        }

        #[test]
        fn typing_a_valid_hex_and_enter_updates_the_value_and_rederives_hsv() {
            let mut ctx = test_context();
            let mut value = [255, 0, 0, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let field = hex_field_rect(&mut ctx, popup);

            focus_clear_and_type(&mut ctx, field, &mut value, "00ff00");
            assert_eq!(ctx.active, Some(scoped_id("Tint", "##hex")));
            assert!(
                ctx.open_popup.is_some(),
                "clicking into the hex field is a press inside the popup: it stays open"
            );
            assert_eq!(value, [255, 0, 0, 255], "typing alone must not commit");

            let mut changed = false;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(&[KeyEvent::Press(Key::Enter)]),
                |ui| changed = declare(ui, &mut value),
            );
            assert!(changed, "a valid commit reports a change");
            assert_eq!(
                value,
                [0, 255, 0, 255],
                "lowercase six digits parse; alpha is implied 255"
            );
            assert_eq!(
                hsv_state(&ctx, "Tint"),
                (120.0, 1.0, 1.0),
                "the HSV state re-derives through the RGB path"
            );
            assert_eq!(
                hex_buffer(&ctx, "Tint"),
                "#00FF00FF",
                "the committed field restores the canonical display"
            );
            assert_eq!(ctx.active, None, "Enter unfocuses the field");

            frame(&mut ctx, AWAY, false, |ui| {
                declare(ui, &mut value);
            });
            let style = ctx.style;
            for (i, t) in [(0, 120.0 / 360.0), (1, 1.0), (2, 1.0)] {
                let track = track_rect(&mut ctx, popup, i);
                assert_eq!(
                    overlay_fill_color_at(&ctx, grab_at(&ctx, track, t)),
                    Some(style.widget_bg),
                    "HSB slider {i} must reflect the hex commit (t = {t})"
                );
            }
        }

        #[test]
        fn an_invalid_hex_commit_leaves_the_value_unchanged_and_restores_the_display() {
            let mut ctx = test_context();
            let mut value = [16, 32, 48, 200];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let field = hex_field_rect(&mut ctx, popup);
            let seeded_state = hsv_state(&ctx, "Tint");

            focus_clear_and_type(&mut ctx, field, &mut value, "GGGG");
            let mut changed = true;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(&[KeyEvent::Press(Key::Enter)]),
                |ui| changed = declare(ui, &mut value),
            );
            assert!(!changed, "an invalid commit is not a change");
            assert_eq!(
                value,
                [16, 32, 48, 200],
                "invalid input leaves the value unchanged"
            );
            assert_eq!(
                hsv_state(&ctx, "Tint"),
                seeded_state,
                "invalid input leaves the working state untouched"
            );
            assert_eq!(
                hex_buffer(&ctx, "Tint"),
                "#102030C8",
                "the display restores the current value's canonical format"
            );
        }

        #[test]
        fn the_unfocused_hex_display_reformats_after_an_rgb_slider_edit() {
            let mut ctx = test_context();
            let mut value = [0, 0, 0, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            assert_eq!(hex_buffer(&ctx, "Tint"), "#000000FF");

            let track = track_rect(&mut ctx, popup, 3);
            frame(&mut ctx, center(track), true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, [track[2] + 10.0, center(track)[1]], true, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value[0], 255);
            assert_eq!(
                hex_buffer(&ctx, "Tint"),
                "#FF0000FF",
                "the unfocused field must reformat when the value moves"
            );
            frame(&mut ctx, [track[2] + 10.0, center(track)[1]], false, |ui| {
                declare(ui, &mut value);
            });
        }

        #[test]
        fn the_edit_buffer_survives_the_end_of_frame_state_write_back() {
            let mut ctx = test_context();
            let mut value = [255, 0, 0, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let field = hex_field_rect(&mut ctx, popup);

            focus_clear_and_type(&mut ctx, field, &mut value, "00");
            assert_eq!(hex_buffer(&ctx, "Tint"), "00");
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(&[
                    KeyEvent::Char('F'),
                    KeyEvent::Char('F'),
                    KeyEvent::Char('0'),
                    KeyEvent::Char('0'),
                ]),
                |ui| {
                    declare(ui, &mut value);
                },
            );
            assert_eq!(
                hex_buffer(&ctx, "Tint"),
                "00FF00",
                "the buffer must survive the per-frame state write-back"
            );
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(&[KeyEvent::Char('F'), KeyEvent::Char('F')]),
                |ui| {
                    declare(ui, &mut value);
                },
            );
            assert_eq!(hex_buffer(&ctx, "Tint"), "00FF00FF");

            let mut changed = false;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(&[KeyEvent::Press(Key::Enter)]),
                |ui| changed = declare(ui, &mut value),
            );
            assert!(changed);
            assert_eq!(value, [0, 255, 0, 255]);
        }

        #[test]
        fn pressing_a_slider_defocuses_and_commits_the_hex_field() {
            let mut ctx = test_context();
            let mut value = [255, 0, 0, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let field = hex_field_rect(&mut ctx, popup);
            focus_clear_and_type(&mut ctx, field, &mut value, "00FF00");

            let track = track_rect(&mut ctx, popup, 3);
            let mut changed = false;
            frame(&mut ctx, center(track), true, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(changed);
            assert_eq!(
                value,
                [0, 255, 0, 255],
                "the hex commit must land despite the slider stealing the capture"
            );
            assert_eq!(
                hsv_state(&ctx, "Tint"),
                (120.0, 1.0, 1.0),
                "the commit re-derives the working state"
            );
            assert_eq!(ctx.active, Some(scoped_id("Tint", "R")));

            frame(&mut ctx, center(track), true, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(changed);
            assert_eq!(
                value,
                [128, 255, 0, 255],
                "the slider drag applies after the commit frame"
            );
            frame(&mut ctx, center(track), false, |ui| {
                declare(ui, &mut value);
            });
        }

        #[test]
        fn committing_the_current_value_keeps_the_sub_u8_working_state() {
            let mut ctx = test_context();
            let mut value = [10, 20, 30, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);

            let sv = sv_rect(&ctx, popup);
            frame(&mut ctx, center(sv), true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, [center(sv)[0], sv[3] + 10.0], true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, [center(sv)[0], sv[3] + 10.0], false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(value, [0, 0, 0, 255]);
            let s_track = track_rect(&mut ctx, popup, 1);
            let at = [
                s_track[0] + 0.75 * (s_track[2] - s_track[0]),
                center(s_track)[1],
            ];
            frame(&mut ctx, at, true, |ui| {
                declare(ui, &mut value);
            });
            frame(&mut ctx, at, false, |ui| {
                declare(ui, &mut value);
            });
            assert_eq!(hsv_state(&ctx, "Tint").1, 0.75, "the state really moved");

            let field = hex_field_rect(&mut ctx, popup);
            focus_clear_and_type(&mut ctx, field, &mut value, "000000");
            let mut changed = true;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(&[KeyEvent::Press(Key::Enter)]),
                |ui| changed = declare(ui, &mut value),
            );
            assert!(!changed, "an equal commit is not a change");
            assert_eq!(value, [0, 0, 0, 255]);
            assert_eq!(
                hsv_state(&ctx, "Tint").1,
                0.75,
                "an equal commit must not re-derive (and flatten) the working state"
            );
            assert_eq!(hex_buffer(&ctx, "Tint"), "#000000FF");
        }

        #[test]
        fn defocusing_the_field_commits_like_enter() {
            let mut ctx = test_context();
            let mut value = [255, 0, 0, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let field = hex_field_rect(&mut ctx, popup);

            focus_clear_and_type(&mut ctx, field, &mut value, "0000FF80");
            let preview = preview_rect(&ctx, popup);
            let mut changed = false;
            frame(&mut ctx, center(preview), true, |ui| {
                changed = declare(ui, &mut value)
            });
            assert!(changed, "defocus commits the field");
            assert_eq!(
                value,
                [0, 0, 255, 128],
                "eight digits carry an explicit alpha"
            );
            assert!(
                ctx.open_popup.is_some(),
                "a press inside the popup keeps it open"
            );
            frame(&mut ctx, center(preview), false, |ui| {
                declare(ui, &mut value);
            });
        }

        #[test]
        fn escape_closes_the_popup_before_the_field_sees_it_so_nothing_commits() {
            let mut ctx = test_context();
            let mut value = [255, 0, 0, 255];
            let swatch = settle(&mut ctx, &mut value);
            let popup = click_open(&mut ctx, swatch, &mut value);
            let field = hex_field_rect(&mut ctx, popup);
            focus_clear_and_type(&mut ctx, field, &mut value, "00FF00");

            let mut changed = true;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(&[KeyEvent::Press(Key::Escape)]),
                |ui| changed = declare(ui, &mut value),
            );
            assert!(!changed);
            assert_eq!(ctx.open_popup, None, "Escape must close the popup");
            assert_eq!(
                value,
                [255, 0, 0, 255],
                "the uncommitted text never reaches the value"
            );
            frame(&mut ctx, AWAY, false, |ui| {
                declare(ui, &mut value);
            });

            assert_eq!(hex_buffer(&ctx, "Tint"), "00FF00");
            click_open(&mut ctx, swatch, &mut value);
            assert_eq!(hex_buffer(&ctx, "Tint"), "#FF0000FF");
        }

        #[test]
        fn closing_a_by_pressing_b_then_opening_b_seeds_b_and_keeps_as_edits() {
            let mut ctx = test_context();
            let mut a = [0, 0, 0, 255];
            let mut b = [200, 201, 202, 203];
            let (mut sw_a, mut sw_b) = ([0.0; 4], [0.0; 4]);
            frame(&mut ctx, AWAY, false, |ui| {
                sw_a = swatch_rect(ui);
                ui.color_picker_rgba("Tint##a", &mut a);
                ui.same_line();
                sw_b = swatch_rect(ui);
                ui.color_picker_rgba("Tint##b", &mut b);
            });
            let declare_ab = |ui: &mut Ui, a: &mut [u8; 4], b: &mut [u8; 4]| {
                ui.color_picker_rgba("Tint##a", a);
                ui.same_line();
                ui.color_picker_rgba("Tint##b", b);
            };

            let at_a = center(sw_a);
            frame(&mut ctx, at_a, true, |ui| declare_ab(ui, &mut a, &mut b));
            frame(&mut ctx, at_a, false, |ui| declare_ab(ui, &mut a, &mut b));
            assert_eq!(ctx.open_popup, Some(widget_id_in_w("Tint##a")));
            let popup = ctx.popup_rect.expect("a's popup must be open");
            let track = track_rect(&mut ctx, popup, 3);
            frame(&mut ctx, center(track), true, |ui| {
                declare_ab(ui, &mut a, &mut b)
            });
            frame(&mut ctx, center(track), false, |ui| {
                declare_ab(ui, &mut a, &mut b)
            });
            assert_eq!(a, [128, 0, 0, 255], "a's edit landed");

            let at_b = center(sw_b);
            frame(&mut ctx, at_b, true, |ui| declare_ab(ui, &mut a, &mut b));
            assert_eq!(ctx.open_popup, None, "the outside press closes A's popup");
            frame(&mut ctx, at_b, false, |ui| declare_ab(ui, &mut a, &mut b));
            assert_eq!(ctx.open_popup, None, "the consumed press must not open B");

            frame(&mut ctx, at_b, true, |ui| declare_ab(ui, &mut a, &mut b));
            frame(&mut ctx, at_b, false, |ui| declare_ab(ui, &mut a, &mut b));
            assert_eq!(ctx.open_popup, Some(widget_id_in_w("Tint##b")));
            assert_eq!(
                hsv_state(&ctx, "Tint##b"),
                crate::color::rgba_u8_to_hsv(b),
                "B's working state is seeded from B's value"
            );
            assert_eq!(a, [128, 0, 0, 255], "A's edits are retained");
            assert_eq!(b, [200, 201, 202, 203], "B's value is untouched");
        }
    }

    mod keybind {
        use alloc::string::ToString;

        use crate::context::Context;
        use crate::input::{InputState, Key, KeyBinding, KeyEvent, Modifiers};
        use crate::ui::{Ui, KEYBIND_PROMPT};
        use crate::widgets::test_support::*;

        const AWAY: [f32; 2] = [-50.0, -50.0];

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn keyed(mouse_pos: [f32; 2], keys: &[KeyEvent]) -> InputState<'_> {
            InputState {
                mouse_pos,
                keys,
                ..Default::default()
            }
        }

        fn button_rect(ui: &mut Ui, shown: &str) -> [f32; 4] {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let style = ui.ctx.style;
            let text_w = ui.ctx.font.measure(shown)[0];
            let h = ui.ctx.font.line_height() + 2.0 * style.frame_padding[1];
            let w = text_w + 2.0 * style.frame_padding[0];
            [pos[0], pos[1], pos[0] + w, pos[1] + h]
        }

        fn settle(ctx: &mut Context, binding: &mut KeyBinding) -> [f32; 4] {
            let mut rect = [0.0; 4];
            frame(ctx, AWAY, false, |ui| {
                rect = button_rect(ui, &binding.to_string());
                ui.keybind("Jump", binding);
            });
            rect
        }

        fn arm(ctx: &mut Context, button: [f32; 4], binding: &mut KeyBinding) {
            let at = center(button);
            frame(ctx, at, true, |ui| {
                ui.keybind("Jump", binding);
            });
            frame(ctx, at, false, |ui| {
                ui.keybind("Jump", binding);
            });
        }

        fn armed_button_rect(ctx: &mut Context, binding: &mut KeyBinding) -> [f32; 4] {
            let mut rect = [0.0; 4];
            frame(ctx, AWAY, false, |ui| {
                rect = button_rect(ui, KEYBIND_PROMPT);
                ui.keybind("Jump", binding);
            });
            rect
        }

        #[test]
        fn pressing_a_key_while_armed_binds_it_on_the_capture_frame() {
            let mut ctx = test_context();
            let mut binding = KeyBinding::None;
            let button = settle(&mut ctx, &mut binding);
            arm(&mut ctx, button, &mut binding);
            assert_eq!(binding, KeyBinding::None, "arming alone must not rebind");

            let mut changed = false;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(AWAY, &[KeyEvent::Press(Key::F)]),
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(changed, "the capture frame must return true");
            assert_eq!(binding, KeyBinding::Key(Key::F));

            let mut changed = true;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(AWAY, &[KeyEvent::Press(Key::G)]),
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(!changed, "an unarmed keybind must not capture");
            assert_eq!(binding, KeyBinding::Key(Key::F));
        }

        #[test]
        fn a_mouse_button_press_while_armed_binds_the_button_index() {
            let mut ctx = test_context();
            let mut binding = KeyBinding::None;
            let button = settle(&mut ctx, &mut binding);
            arm(&mut ctx, button, &mut binding);

            let mut changed = false;
            frame(&mut ctx, AWAY, true, |ui| {
                changed = ui.keybind("Jump", &mut binding)
            });
            assert!(changed);
            assert_eq!(binding, KeyBinding::Mouse(0));
            frame(&mut ctx, AWAY, false, |ui| {
                ui.keybind("Jump", &mut binding);
            });

            let button = settle(&mut ctx, &mut binding);
            arm(&mut ctx, button, &mut binding);
            let mut changed = false;
            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    mouse_pos: AWAY,
                    mouse_down: [false, true, false, false, false],
                    ..Default::default()
                },
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(changed);
            assert_eq!(binding, KeyBinding::Mouse(1));
        }

        #[test]
        fn a_mouse4_press_while_armed_binds_the_back_button() {
            let mut ctx = test_context();
            let mut binding = KeyBinding::None;
            let button = settle(&mut ctx, &mut binding);
            arm(&mut ctx, button, &mut binding);

            let mut changed = false;
            frame_with_input(
                &mut ctx,
                0.016,
                InputState {
                    mouse_pos: AWAY,
                    mouse_down: [false, false, false, true, false],
                    ..Default::default()
                },
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(changed);
            assert_eq!(binding, KeyBinding::Mouse(3));
            assert_eq!(alloc::string::ToString::to_string(&binding), "Mouse4");
        }

        #[test]
        fn a_key_pressed_with_modifiers_held_binds_a_chord() {
            let mut ctx = test_context();
            let mut binding = KeyBinding::None;
            let button = settle(&mut ctx, &mut binding);
            arm(&mut ctx, button, &mut binding);

            let mut changed = true;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(AWAY, &[KeyEvent::Press(Key::Shift)]),
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(!changed, "a modifier press alone is not a binding");
            assert_eq!(binding, KeyBinding::None);

            let mut changed = false;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(AWAY, &[KeyEvent::Press(Key::G)]),
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(changed);
            assert_eq!(
                binding,
                KeyBinding::Chord {
                    modifiers: Modifiers {
                        shift: true,
                        ctrl: false,
                        alt: false
                    },
                    key: Key::G,
                }
            );

            let button = settle(&mut ctx, &mut binding);
            arm(&mut ctx, button, &mut binding);
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(
                    AWAY,
                    &[KeyEvent::Press(Key::Shift), KeyEvent::Press(Key::Ctrl)],
                ),
                |ui| {
                    ui.keybind("Jump", &mut binding);
                },
            );
            let mut changed = false;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(AWAY, &[KeyEvent::Press(Key::K)]),
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(changed);
            assert_eq!(
                binding,
                KeyBinding::Chord {
                    modifiers: Modifiers {
                        shift: true,
                        ctrl: true,
                        alt: false
                    },
                    key: Key::K,
                }
            );

            let button = settle(&mut ctx, &mut binding);
            arm(&mut ctx, button, &mut binding);
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(AWAY, &[KeyEvent::Press(Key::Shift)]),
                |ui| {
                    ui.keybind("Jump", &mut binding);
                },
            );
            let mut changed = false;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(
                    AWAY,
                    &[
                        KeyEvent::Release(Key::Ctrl),
                        KeyEvent::Release(Key::Shift),
                        KeyEvent::Press(Key::H),
                    ],
                ),
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(changed);
            assert_eq!(binding, KeyBinding::Key(Key::H));
        }

        #[test]
        fn escape_while_armed_cancels_and_keeps_the_old_binding() {
            let mut ctx = test_context();
            let mut binding = KeyBinding::Key(Key::F);
            let button = settle(&mut ctx, &mut binding);
            arm(&mut ctx, button, &mut binding);

            let mut changed = true;
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(AWAY, &[KeyEvent::Press(Key::Escape)]),
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(!changed, "a cancel is not a capture");
            assert_eq!(
                binding,
                KeyBinding::Key(Key::F),
                "Escape keeps the old binding"
            );

            frame_with_input(
                &mut ctx,
                0.016,
                keyed(AWAY, &[KeyEvent::Press(Key::G)]),
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(!changed);
            assert_eq!(binding, KeyBinding::Key(Key::F));
        }

        #[test]
        fn clicking_the_button_again_while_armed_unarms_without_rebinding() {
            let mut ctx = test_context();
            let mut binding = KeyBinding::Key(Key::F);
            let button = settle(&mut ctx, &mut binding);
            arm(&mut ctx, button, &mut binding);
            let armed_button = armed_button_rect(&mut ctx, &mut binding);

            let at = center(armed_button);
            let mut changed = true;
            frame(&mut ctx, at, true, |ui| {
                changed = ui.keybind("Jump", &mut binding)
            });
            assert!(!changed, "unarming is not a capture");
            assert_eq!(binding, KeyBinding::Key(Key::F), "the old binding survives");
            frame(&mut ctx, at, false, |ui| {
                changed = ui.keybind("Jump", &mut binding)
            });

            frame_with_input(
                &mut ctx,
                0.016,
                keyed(AWAY, &[KeyEvent::Press(Key::G)]),
                |ui| changed = ui.keybind("Jump", &mut binding),
            );
            assert!(!changed);
            assert_eq!(binding, KeyBinding::Key(Key::F));
        }

        #[test]
        fn with_two_keybinds_only_the_armed_one_captures() {
            let mut ctx = test_context();
            let mut a = KeyBinding::None;
            let mut b = KeyBinding::None;
            let declare = |ui: &mut Ui, a: &mut KeyBinding, b: &mut KeyBinding| {
                ui.keybind("Jump##a", a);
                ui.keybind("Crouch##b", b);
            };
            let mut rect_a = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                rect_a = button_rect(ui, "None");
                declare(ui, &mut a, &mut b);
            });

            let at = center(rect_a);
            frame(&mut ctx, at, true, |ui| declare(ui, &mut a, &mut b));
            frame(&mut ctx, at, false, |ui| declare(ui, &mut a, &mut b));

            let (mut ca, mut cb) = (false, true);
            frame_with_input(
                &mut ctx,
                0.016,
                keyed(AWAY, &[KeyEvent::Press(Key::F)]),
                |ui| {
                    ca = ui.keybind("Jump##a", &mut a);
                    cb = ui.keybind("Crouch##b", &mut b);
                },
            );
            assert!(ca, "the armed keybind captures");
            assert!(!cb, "the unarmed keybind stays out of it");
            assert_eq!(a, KeyBinding::Key(Key::F));
            assert_eq!(b, KeyBinding::None);
        }

        #[test]
        fn the_armed_button_shows_the_prompt_in_the_active_color() {
            let mut ctx = test_context();
            let mut binding = KeyBinding::Key(Key::F);
            let button = settle(&mut ctx, &mut binding);
            assert_eq!(fill_color_at(&ctx, button), Some(ctx.style.widget_bg));
            assert!(
                !glyph_colors_in(&ctx, button).is_empty(),
                "the binding text must be emitted"
            );

            arm(&mut ctx, button, &mut binding);
            let armed_button = armed_button_rect(&mut ctx, &mut binding);
            assert_eq!(
                fill_color_at(&ctx, armed_button),
                Some(ctx.style.widget_bg_active),
                "the armed button must use the distinct active styling"
            );
            assert!(
                !glyph_colors_in(&ctx, armed_button).is_empty(),
                "the armed button must emit the prompt text"
            );
        }
    }

    mod tooltip {
        use crate::context::Context;
        use crate::input::InputState;
        use crate::ui::{Ui, TOOLTIP_DELAY, TOOLTIP_OFFSET};
        use crate::widgets::test_support::*;

        const AWAY: [f32; 2] = [-50.0, -50.0];

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn input_at(mouse_pos: [f32; 2]) -> InputState<'static> {
            InputState {
                mouse_pos,
                ..Default::default()
            }
        }

        fn declare(ui: &mut Ui) {
            ui.button("OK");
            ui.tooltip_text("tip");
        }

        fn settle(ctx: &mut Context) -> [f32; 4] {
            let mut rect = [0.0; 4];
            frame(ctx, AWAY, false, |ui| {
                rect = next_text_rect(ui, "OK");
                declare(ui);
            });
            rect
        }

        fn hover(ctx: &mut Context, dt: f32, at: [f32; 2]) {
            frame_with_input(ctx, dt, input_at(at), declare);
        }

        fn tooltip_rect(ctx: &mut Context, mouse: [f32; 2], text: &str) -> [f32; 4] {
            let style = ctx.style;
            let line_height = ctx.font.line_height();
            let mut widest = 0.0f32;
            let mut lines = 0usize;
            for line in text.split('\n') {
                widest = widest.max(ctx.font.measure(line)[0]);
                lines += 1;
            }
            let w = widest + 2.0 * style.window_padding[0];
            let h = lines as f32 * line_height + 2.0 * style.window_padding[1];
            let mut origin = [mouse[0] + TOOLTIP_OFFSET[0], mouse[1] + TOOLTIP_OFFSET[1]];
            let viewport = ctx.input.viewport_size;
            if viewport != [0.0, 0.0] {
                origin[0] = origin[0].clamp(0.0, (viewport[0] - w).max(0.0));
                origin[1] = origin[1].clamp(0.0, (viewport[1] - h).max(0.0));
            }
            [origin[0], origin[1], origin[0] + w, origin[1] + h]
        }

        fn overlay_is_empty(ctx: &Context) -> bool {
            ctx.overlay.as_draw_data().vertices.is_empty()
        }

        fn overlay_fill_color_at(ctx: &Context, rect: [f32; 4]) -> Option<[u8; 4]> {
            crate::widgets::test_support::chrome_color_in(ctx.overlay.as_draw_data(), rect)
        }

        fn overlay_has_glyphs_in(ctx: &Context, rect: [f32; 4]) -> bool {
            let data = ctx.overlay.as_draw_data();
            let mut offset = 0usize;
            for cmd in data.commands {
                let next = offset + cmd.elem_count as usize;
                if cmd.texture == crate::draw::TextureId::FONT {
                    for &i in &data.indices[offset..next] {
                        let v = data.vertices[i as usize];
                        if v.pos[0] >= rect[0]
                            && v.pos[0] <= rect[2]
                            && v.pos[1] >= rect[1]
                            && v.pos[1] <= rect[3]
                        {
                            return true;
                        }
                    }
                }
                offset = next;
            }
            false
        }

        #[test]
        fn no_tooltip_before_the_delay() {
            let mut ctx = test_context();
            let button = settle(&mut ctx);
            let at = center(button);
            hover(&mut ctx, TOOLTIP_DELAY - 0.2, at);
            assert!(
                overlay_is_empty(&ctx),
                "nothing may be emitted before the hover delay elapses"
            );
        }

        #[test]
        fn tooltip_appears_after_the_delay_near_the_mouse() {
            let mut ctx = test_context();
            let button = settle(&mut ctx);
            let at = center(button);
            hover(&mut ctx, TOOLTIP_DELAY - 0.2, at);
            assert!(overlay_is_empty(&ctx));
            hover(&mut ctx, 0.2, at);
            let rect = tooltip_rect(&mut ctx, at, "tip");
            assert_eq!(
                overlay_fill_color_at(&ctx, rect),
                Some(ctx.style.window_bg),
                "the tooltip background must sit at mouse + offset"
            );
            assert!(
                overlay_has_glyphs_in(&ctx, rect),
                "the tooltip text must be emitted inside it"
            );
            assert_eq!(ctx.open_popup, None);
            assert_eq!(ctx.popup_rect, None);
        }

        #[test]
        fn a_hover_break_resets_the_timer() {
            let mut ctx = test_context();
            let button = settle(&mut ctx);
            let at = center(button);
            hover(&mut ctx, TOOLTIP_DELAY - 0.2, at);
            hover(&mut ctx, 0.1, AWAY);
            hover(&mut ctx, TOOLTIP_DELAY - 0.2, at);
            assert!(
                overlay_is_empty(&ctx),
                "broken hover must not accumulate across the break"
            );
            hover(&mut ctx, 0.2, at);
            assert!(!overlay_is_empty(&ctx));
        }

        #[test]
        fn the_tooltip_hides_on_unhover() {
            let mut ctx = test_context();
            let button = settle(&mut ctx);
            let at = center(button);
            hover(&mut ctx, TOOLTIP_DELAY, at);
            assert!(!overlay_is_empty(&ctx), "the tooltip must be up");
            hover(&mut ctx, 0.1, AWAY);
            assert!(overlay_is_empty(&ctx), "unhover must hide the tooltip");
        }

        #[test]
        fn an_active_capture_hides_the_tooltip() {
            let mut ctx = test_context();
            let button = settle(&mut ctx);
            let at = center(button);
            hover(&mut ctx, TOOLTIP_DELAY, at);
            assert!(!overlay_is_empty(&ctx));
            frame(&mut ctx, at, true, declare);
            let tooltip = tooltip_rect(&mut ctx, at, "tip");
            assert!(
                !overlay_has_glyphs_in(&ctx, tooltip),
                "the tooltip must hide while a capture is active"
            );
        }

        #[test]
        fn multi_line_text_renders_per_line_sized_to_the_widest() {
            const TEXT: &str = "a\nbb";
            let mut ctx = test_context();
            let mut button = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                button = next_text_rect(ui, "OK");
                ui.button("OK");
                ui.tooltip_text(TEXT);
            });
            let at = center(button);
            frame_with_input(&mut ctx, TOOLTIP_DELAY, input_at(at), |ui| {
                ui.button("OK");
                ui.tooltip_text(TEXT);
            });
            let rect = tooltip_rect(&mut ctx, at, TEXT);
            assert_eq!(
                overlay_fill_color_at(&ctx, rect),
                Some(ctx.style.window_bg),
                "the popup must be sized to the widest line"
            );
            let line_height = ctx.font.line_height();
            let pad_y = ctx.style.window_padding[1];
            for i in 0..2 {
                let band = [
                    rect[0],
                    rect[1] + pad_y + i as f32 * line_height,
                    rect[2],
                    rect[1] + pad_y + (i + 1) as f32 * line_height,
                ];
                assert!(
                    overlay_has_glyphs_in(&ctx, band),
                    "line {i} of the tooltip must be emitted"
                );
            }
        }

        #[test]
        fn the_tooltip_is_clamped_into_a_known_viewport() {
            let mut ctx = test_context();
            let button = settle(&mut ctx);
            let at = center(button);
            let viewport = [at[0] + 30.0, at[1] + 40.0];
            let input = InputState {
                mouse_pos: at,
                viewport_size: viewport,
                ..Default::default()
            };
            frame_with_input(&mut ctx, TOOLTIP_DELAY, input, declare);
            let rect = tooltip_rect(&mut ctx, at, "tip");
            let unclamped = [at[0] + TOOLTIP_OFFSET[0], at[1] + TOOLTIP_OFFSET[1]];
            assert!(
                rect[0] < unclamped[0] && rect[1] < unclamped[1],
                "the origin must shift left/up to fit the viewport"
            );
            assert!(rect[2] <= viewport[0] && rect[3] <= viewport[1]);
            assert_eq!(
                overlay_fill_color_at(&ctx, rect),
                Some(ctx.style.window_bg),
                "the tooltip background must sit at the clamped origin"
            );
        }

        #[test]
        fn the_tooltip_neither_captures_nor_consumes_presses() {
            const TIP: &str = "a somewhat wider tooltip text";
            let mut ctx = test_context();
            let (mut a, mut b) = ([0.0; 4], [0.0; 4]);
            frame(&mut ctx, AWAY, false, |ui| {
                a = next_text_rect(ui, "OK");
                ui.button("OK");
                ui.tooltip_text(TIP);
                b = next_text_rect(ui, "Fire");
                ui.button("Fire");
            });
            let at = [a[0] + 5.0, a[3] - 1.0];
            let raise = |ui: &mut Ui, clicked: &mut bool| {
                ui.button("OK");
                ui.tooltip_text(TIP);
                *clicked = ui.button("Fire");
            };
            let mut clicked = false;
            frame_with_input(&mut ctx, TOOLTIP_DELAY, input_at(at), |ui| {
                raise(ui, &mut clicked)
            });
            let tip = tooltip_rect(&mut ctx, at, TIP);
            assert!(!overlay_is_empty(&ctx), "the tooltip must be up");
            let press = [tip[0] + 8.0, tip[1] + 8.0];
            assert!(
                press[0] >= b[0] && press[0] < b[2] && press[1] >= b[1] && press[1] < b[3],
                "the press point must be under the tooltip AND on button B"
            );
            frame(&mut ctx, press, true, |ui| raise(ui, &mut clicked));
            assert!(!clicked, "the press frame must not click");
            assert_eq!(
                ctx.active,
                Some(widget_id_in_w("Fire")),
                "the button under the tooltip must capture the press"
            );
            frame(&mut ctx, press, false, |ui| raise(ui, &mut clicked));
            assert!(clicked, "a button under the tooltip must still fire");
        }

        #[test]
        fn tooltip_text_without_a_preceding_widget_is_a_no_op() {
            let mut ctx = test_context();
            frame(&mut ctx, [100.0, 100.0], false, |ui| {
                ui.tooltip_text("tip");
            });
            frame(&mut ctx, [100.0, 100.0], false, |ui| {
                ui.tooltip_text("tip");
            });
            assert!(overlay_is_empty(&ctx));
        }

        #[test]
        fn a_non_interactive_label_does_not_become_the_tooltip_anchor() {
            let mut ctx = test_context();
            let mut button = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                button = next_text_rect(ui, "OK");
                ui.button("OK");
                ui.label("note");
                ui.tooltip_text("tip");
            });
            let at = center(button);
            frame_with_input(&mut ctx, TOOLTIP_DELAY, input_at(at), |ui| {
                ui.button("OK");
                ui.label("note");
                ui.tooltip_text("tip");
            });
            assert!(
                !overlay_is_empty(&ctx),
                "the tooltip must attach to the last INTERACTIVE widget"
            );
        }

        #[test]
        fn a_tooltip_on_a_stateful_widget_still_times() {
            let mut ctx = test_context();
            let mut header = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                let pos = ui.ctx.current_window_mut().cursor.next_pos();
                let w = ui.ctx.current_window_mut().cursor.available_width();
                let h = ui.ctx.font.line_height() + 2.0 * ui.ctx.style.frame_padding[1];
                header = [pos[0], pos[1], pos[0] + w, pos[1] + h];
                ui.collapsing_header("Head", |_| {});
                ui.tooltip_text("tip");
            });
            let at = center(header);
            frame_with_input(&mut ctx, TOOLTIP_DELAY, input_at(at), |ui| {
                ui.collapsing_header("Head", |_| {});
                ui.tooltip_text("tip");
            });
            assert!(
                !overlay_is_empty(&ctx),
                "the anchor's own widget state must not block the tooltip"
            );
        }

        fn overlay_has_exact_fill(ctx: &Context, rect: [f32; 4]) -> bool {
            let data = ctx.overlay.as_draw_data();
            data.vertices.windows(3).any(|anchor| {
                anchor[0].pos == [rect[0], rect[1]] && anchor[2].pos == [rect[2], rect[3]]
            })
        }

        #[test]
        fn a_tooltip_after_a_scroll_region_anchors_to_the_region() {
            const TIP: &str = "region tip";
            let mut ctx = test_context();
            let declare = |ui: &mut Ui| {
                ui.scroll_region("log", [200.0, 60.0], |ui| {
                    ui.label("line 0");
                    ui.label("line 1");
                });
                ui.tooltip_text(TIP);
            };
            let mut region = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                let pos = ui.ctx.current_window_mut().cursor.next_pos();
                region = [pos[0], pos[1], pos[0] + 200.0, pos[1] + 60.0];
                declare(ui);
            });

            let at = center(region);
            frame_with_input(&mut ctx, TOOLTIP_DELAY, input_at(at), declare);
            let rect = tooltip_rect(&mut ctx, at, TIP);
            assert_eq!(
                overlay_fill_color_at(&ctx, rect),
                Some(ctx.style.window_bg),
                "hovering the region must raise its tooltip at mouse + offset"
            );
            assert!(overlay_has_glyphs_in(&ctx, rect));

            frame_with_input(&mut ctx, 0.1, input_at(AWAY), declare);
            assert!(overlay_is_empty(&ctx), "unhover must hide the tooltip");
            let elsewhere = [region[0] + 10.0, region[3] + 30.0];
            frame_with_input(&mut ctx, TOOLTIP_DELAY, input_at(elsewhere), declare);
            assert!(
                overlay_is_empty(&ctx),
                "hovering outside the region must not raise its tooltip"
            );
        }

        #[test]
        fn tooltips_above_and_on_a_scroll_region_do_not_stack() {
            let mut ctx = test_context();
            ctx.style.corner_rounding = 0.0;
            let mut buf = alloc::string::String::new();
            let declare = |ui: &mut Ui, buf: &mut alloc::string::String, input: &mut [f32; 4]| {
                ui.text_input("filter", buf);
                *input = ui.ctx.last_item.expect("the text input registers").1;
                ui.tooltip_text("input tip");
                ui.scroll_region("log", [200.0, 60.0], |ui| {
                    ui.label("line 0");
                });
                ui.tooltip_text("region tip");
            };
            let mut input = [0.0; 4];
            frame(&mut ctx, AWAY, false, |ui| {
                declare(ui, &mut buf, &mut input)
            });

            let at = center(input);
            frame_with_input(&mut ctx, TOOLTIP_DELAY, input_at(at), |ui| {
                declare(ui, &mut buf, &mut input)
            });
            let tip = tooltip_rect(&mut ctx, at, "input tip");
            let stacked = tooltip_rect(&mut ctx, at, "region tip");
            assert!(
                tip[2] != stacked[2],
                "the test needs the two tooltips to differ in width"
            );
            assert!(
                overlay_has_exact_fill(&ctx, tip),
                "the input's own tooltip must fire over the input"
            );
            assert!(overlay_has_glyphs_in(&ctx, tip));
            assert!(
                !overlay_has_exact_fill(&ctx, stacked),
                "the region's tooltip must not anchor to the text input"
            );
        }
    }

    mod drag_fields {
        use crate::context::Context;
        use crate::ui::{Ui, DRAG_MARKER_CORE, SV_MARKER_RADIUS};
        use crate::widgets::test_support::*;

        const AWAY: [f32; 2] = [-50.0, -50.0];
        const SV_SIZE: f32 = 100.0;
        const STRIP_SIZE: [f32; 2] = [20.0, 100.0];
        const HUE: f32 = 240.0;

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn field_rect(ui: &mut Ui, size: [f32; 2]) -> [f32; 4] {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            [pos[0], pos[1], pos[0] + size[0], pos[1] + size[1]]
        }

        fn sv_marker_pos(rect: [f32; 4], sv: [f32; 2]) -> [f32; 2] {
            [
                rect[0] + sv[0].clamp(0.0, 1.0) * (rect[2] - rect[0]),
                rect[1] + (1.0 - sv[1].clamp(0.0, 1.0)) * (rect[3] - rect[1]),
            ]
        }

        fn hue_marker_y(rect: [f32; 4], h: f32) -> f32 {
            rect[1] + (h / 360.0).clamp(0.0, 1.0) * (rect[3] - rect[1])
        }

        fn settle_sv(ctx: &mut Context, sv: &mut [f32; 2]) -> [f32; 4] {
            let mut rect = [0.0; 4];
            frame(ctx, AWAY, false, |ui| {
                rect = field_rect(ui, [SV_SIZE, SV_SIZE]);
                ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, sv);
            });
            rect
        }

        fn settle_strip(ctx: &mut Context, h: &mut f32) -> [f32; 4] {
            let mut rect = [0.0; 4];
            frame(ctx, AWAY, false, |ui| {
                rect = field_rect(ui, STRIP_SIZE);
                ui.hue_strip("##hue", STRIP_SIZE, h);
            });
            rect
        }

        #[test]
        fn sv_press_maps_mouse_to_saturation_value() {
            let mut ctx = test_context();
            let mut sv = [0.0, 0.0];
            let rect = settle_sv(&mut ctx, &mut sv);
            let mid = center(rect);
            let mut changed = false;

            frame(&mut ctx, mid, true, |ui| {
                changed = ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv)
            });
            assert!(changed, "pressing a new position must report a change");
            assert!(
                (sv[0] - 0.5).abs() < 1e-3 && (sv[1] - 0.5).abs() < 1e-3,
                "center press must map to ≈ (0.5, 0.5), got {sv:?}"
            );
            assert_eq!(ctx.active, Some(widget_id_in_w("##sv")));

            frame(&mut ctx, mid, true, |ui| {
                changed = ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv)
            });
            assert!(!changed, "holding still must not report a change");

            frame(&mut ctx, mid, false, |ui| {
                changed = ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv)
            });
            assert!(!changed, "the release frame does not change the value");
            assert_eq!(ctx.active, None);
        }

        #[test]
        fn sv_corners_map_exactly() {
            let mut ctx = test_context();
            let mut sv = [0.5, 0.5];
            let rect = settle_sv(&mut ctx, &mut sv);
            let mid = center(rect);

            for (at, expected) in [
                ([rect[0], rect[1]], [0.0, 1.0]),
                ([rect[2], rect[1]], [1.0, 1.0]),
                ([rect[0], rect[3]], [0.0, 0.0]),
                ([rect[2], rect[3]], [1.0, 0.0]),
            ] {
                frame(&mut ctx, mid, true, |ui| {
                    ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
                });
                assert_eq!(ctx.active, Some(widget_id_in_w("##sv")));
                frame(&mut ctx, at, true, |ui| {
                    ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
                });
                assert_eq!(sv, expected, "drag to {at:?}");
                frame(&mut ctx, at, false, |ui| {
                    ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
                });
                assert_eq!(ctx.active, None);
            }
        }

        #[test]
        fn sv_capture_survives_off_rect_drag_and_clamps() {
            let mut ctx = test_context();
            let mut sv = [0.5, 0.5];
            let rect = settle_sv(&mut ctx, &mut sv);
            let mid = center(rect);

            frame(&mut ctx, mid, true, |ui| {
                ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
            });
            assert_eq!(ctx.active, Some(widget_id_in_w("##sv")));

            let mut changed = false;
            frame(&mut ctx, [rect[0] - 200.0, rect[3] + 200.0], true, |ui| {
                changed = ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv)
            });
            assert!(changed);
            assert_eq!(sv, [0.0, 0.0], "past the bottom-left must clamp");
            assert_eq!(
                ctx.active,
                Some(widget_id_in_w("##sv")),
                "capture must survive leaving the rect"
            );

            frame(&mut ctx, [rect[2] + 200.0, rect[1] - 200.0], true, |ui| {
                changed = ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv)
            });
            assert!(changed);
            assert_eq!(sv, [1.0, 1.0], "past the top-right must clamp");
            assert_eq!(ctx.active, Some(widget_id_in_w("##sv")));

            frame(&mut ctx, AWAY, false, |ui| {
                ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
            });
            assert_eq!(ctx.active, None);
            assert_eq!(sv, [1.0, 1.0], "release keeps the dragged value");
        }

        #[test]
        fn sv_changed_flag_is_true_exactly_on_value_change_frames() {
            let mut ctx = test_context();
            let mut sv = [0.5, 0.5];
            let rect = settle_sv(&mut ctx, &mut sv);
            let p = [
                rect[0] + 0.25 * (rect[2] - rect[0]),
                rect[1] + 0.25 * (rect[3] - rect[1]),
            ];
            let mut changed = true;

            frame(&mut ctx, p, false, |ui| {
                changed = ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv)
            });
            assert!(!changed, "hover without a press must not report a change");
            assert_eq!(ctx.hot, Some(widget_id_in_w("##sv")));

            frame(&mut ctx, p, true, |ui| {
                changed = ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv)
            });
            assert!(changed);
            assert_eq!(sv, [0.25, 0.75]);

            frame(&mut ctx, p, false, |ui| {
                ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
            });
            frame(&mut ctx, p, true, |ui| {
                changed = ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv)
            });
            assert!(
                !changed,
                "a press that moves nothing must not report a change"
            );
            assert_eq!(ctx.active, Some(widget_id_in_w("##sv")));

            frame(&mut ctx, p, true, |ui| {
                changed = ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv)
            });
            assert!(!changed, "holding still must not report a change");

            frame(&mut ctx, p, false, |ui| {
                changed = ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv)
            });
            assert!(!changed);
            assert_eq!(ctx.active, None);
        }

        #[test]
        fn sv_marker_ring_sits_at_the_current_value() {
            let mut ctx = test_context();
            ctx.style.anti_aliasing = false;
            let mut sv = [0.25, 0.75];
            let rect = settle_sv(&mut ctx, &mut sv);
            let expected = sv_marker_pos(rect, sv);

            let data = ctx.windows[0].draw_list.as_draw_data();
            let mut distances = alloc::vec::Vec::new();
            for v in data.vertices {
                if v.col == DRAG_MARKER_CORE {
                    let d = ((v.pos[0] - expected[0]).powi(2) + (v.pos[1] - expected[1]).powi(2))
                        .sqrt();
                    if d < 12.0 {
                        distances.push(d);
                    }
                }
            }
            assert!(
                distances.len() >= 8,
                "the marker ring must emit vertices around the value position"
            );
            for d in &distances {
                assert!(
                    (d - SV_MARKER_RADIUS).abs() <= 0.75,
                    "core ring vertex at radius {d}, expected ≈ {SV_MARKER_RADIUS}"
                );
            }
        }

        #[test]
        fn sv_field_emits_gradient_border_and_corner_colors() {
            let mut ctx = test_context();
            let mut sv = [0.5, 0.5];
            let rect = settle_sv(&mut ctx, &mut sv);

            let data = ctx.windows[0].draw_list.as_draw_data();
            assert!(
                data.vertices.len() >= 4 * 16 * 16,
                "the 16×16 SV gradient grid must be emitted"
            );
            let tr_x = rect[0] + (rect[2] - rect[0]) * 16.0 / 16.0;
            let b_y = rect[1] + (rect[3] - rect[1]) * 16.0 / 16.0;
            for (pos, col) in [
                ([rect[0], rect[1]], [255, 255, 255, 255]),
                ([tr_x, rect[1]], [0, 0, 255, 255]),
                ([rect[0], b_y], [0, 0, 0, 255]),
                ([tr_x, b_y], [0, 0, 0, 255]),
            ] {
                assert!(
                    data.vertices.iter().any(|v| v.pos == pos && v.col == col),
                    "SV corner {pos:?} must carry {col:?}"
                );
            }

            let border = ctx.style.border_size;
            assert_eq!(
                fill_color_at(&ctx, [rect[0], rect[1], rect[2], rect[1] + border]),
                Some(crate::widgets::lighten(
                    ctx.style.widget_bg,
                    crate::widgets::BORDER_LIGHTEN
                )),
                "the field must draw its frame border"
            );
        }

        #[test]
        fn sv_drag_over_a_button_does_not_hover_the_button() {
            let mut ctx = test_context();
            let mut sv = [0.0, 0.0];
            let (mut field, mut btn) = ([0.0; 4], [0.0; 4]);
            frame(&mut ctx, AWAY, false, |ui| {
                field = field_rect(ui, [SV_SIZE, SV_SIZE]);
                ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
                btn = next_text_rect(ui, "OK");
                ui.button("OK");
            });

            frame(&mut ctx, center(field), true, |ui| {
                ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
                ui.button("OK");
            });
            assert_eq!(ctx.active, Some(widget_id_in_w("##sv")));

            let mut clicked = true;
            frame(&mut ctx, center(btn), true, |ui| {
                ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
                clicked = ui.button("OK");
            });
            assert!(!clicked);
            assert_eq!(ctx.active, Some(widget_id_in_w("##sv")));
            assert_eq!(ctx.hot, None, "no widget is hovered mid-drag");

            frame(&mut ctx, center(btn), false, |ui| {
                ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
                clicked = ui.button("OK");
            });
            assert!(!clicked, "release over the button is not a click");
            assert_eq!(ctx.active, None);

            frame(&mut ctx, center(btn), false, |ui| {
                ui.sv_field("##sv", [SV_SIZE, SV_SIZE], HUE, &mut sv);
                ui.button("OK");
            });
            assert_eq!(ctx.hot, Some(widget_id_in_w("OK")));
        }

        #[test]
        fn hue_press_maps_mouse_y_to_hue_degrees() {
            let mut ctx = test_context();
            let mut h = 0.0;
            let rect = settle_strip(&mut ctx, &mut h);
            let mid = center(rect);
            let mut changed = false;

            frame(&mut ctx, mid, true, |ui| {
                changed = ui.hue_strip("##hue", STRIP_SIZE, &mut h)
            });
            assert!(changed, "pressing a new position must report a change");
            assert!(
                (h - 180.0).abs() < 0.5,
                "center press must map to ≈ 180°, got {h}"
            );
            assert_eq!(ctx.active, Some(widget_id_in_w("##hue")));

            frame(&mut ctx, [mid[0], rect[1]], true, |ui| {
                ui.hue_strip("##hue", STRIP_SIZE, &mut h);
            });
            assert_eq!(h, 0.0, "the top edge maps to 0°");
            frame(&mut ctx, [mid[0], rect[3]], true, |ui| {
                changed = ui.hue_strip("##hue", STRIP_SIZE, &mut h);
            });
            assert!(changed);
            assert_eq!(h, 360.0, "the bottom edge maps to 360°");

            frame(&mut ctx, mid, false, |ui| {
                ui.hue_strip("##hue", STRIP_SIZE, &mut h);
            });
            assert_eq!(ctx.active, None);
        }

        #[test]
        fn hue_capture_survives_off_rect_drag_and_clamps() {
            let mut ctx = test_context();
            let mut h = 180.0;
            let rect = settle_strip(&mut ctx, &mut h);
            let mid = center(rect);

            frame(&mut ctx, mid, true, |ui| {
                ui.hue_strip("##hue", STRIP_SIZE, &mut h);
            });
            assert_eq!(ctx.active, Some(widget_id_in_w("##hue")));

            let mut changed = false;
            frame(&mut ctx, [mid[0], rect[1] - 150.0], true, |ui| {
                changed = ui.hue_strip("##hue", STRIP_SIZE, &mut h)
            });
            assert!(changed);
            assert_eq!(h, 0.0, "above the strip must clamp to 0°");
            assert_eq!(
                ctx.active,
                Some(widget_id_in_w("##hue")),
                "capture must survive leaving the rect"
            );

            frame(&mut ctx, [mid[0], rect[3] + 150.0], true, |ui| {
                changed = ui.hue_strip("##hue", STRIP_SIZE, &mut h)
            });
            assert!(changed);
            assert_eq!(h, 360.0, "below the strip must clamp to 360°");
            assert_eq!(ctx.active, Some(widget_id_in_w("##hue")));

            frame(&mut ctx, AWAY, false, |ui| {
                ui.hue_strip("##hue", STRIP_SIZE, &mut h);
            });
            assert_eq!(ctx.active, None);
            assert_eq!(h, 360.0, "release keeps the dragged value");
        }

        #[test]
        fn hue_changed_flag_is_true_exactly_on_value_change_frames() {
            let mut ctx = test_context();
            let mut h = 180.0;
            let rect = settle_strip(&mut ctx, &mut h);
            let p = [center(rect)[0], rect[1] + 0.25 * (rect[3] - rect[1])];
            let mut changed = true;

            frame(&mut ctx, p, false, |ui| {
                changed = ui.hue_strip("##hue", STRIP_SIZE, &mut h)
            });
            assert!(!changed, "hover without a press must not report a change");
            assert_eq!(ctx.hot, Some(widget_id_in_w("##hue")));

            frame(&mut ctx, p, true, |ui| {
                changed = ui.hue_strip("##hue", STRIP_SIZE, &mut h)
            });
            assert!(changed);
            assert_eq!(h, 90.0);

            frame(&mut ctx, p, false, |ui| {
                ui.hue_strip("##hue", STRIP_SIZE, &mut h);
            });
            frame(&mut ctx, p, true, |ui| {
                changed = ui.hue_strip("##hue", STRIP_SIZE, &mut h)
            });
            assert!(
                !changed,
                "a press that moves nothing must not report a change"
            );
            assert_eq!(ctx.active, Some(widget_id_in_w("##hue")));

            frame(&mut ctx, p, false, |ui| {
                changed = ui.hue_strip("##hue", STRIP_SIZE, &mut h)
            });
            assert!(!changed);
            assert_eq!(ctx.active, None);
        }

        #[test]
        fn hue_marker_line_sits_at_the_current_value() {
            let mut ctx = test_context();
            ctx.style.anti_aliasing = false;
            let mut h = 90.0;
            let rect = settle_strip(&mut ctx, &mut h);
            let marker_y = hue_marker_y(rect, h);

            let data = ctx.windows[0].draw_list.as_draw_data();
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut count = 0usize;
            for v in data.vertices {
                if v.col == DRAG_MARKER_CORE {
                    assert!(
                        (v.pos[1] - marker_y).abs() <= 0.75,
                        "marker vertex y {} must sit at ≈ {marker_y}",
                        v.pos[1]
                    );
                    min_x = min_x.min(v.pos[0]);
                    max_x = max_x.max(v.pos[0]);
                    count += 1;
                }
            }
            assert!(count >= 4, "the marker line must emit vertices");
            assert!(
                (min_x - rect[0]).abs() <= 0.75,
                "the marker spans from the left edge"
            );
            assert!(
                (max_x - rect[2]).abs() <= 0.75,
                "the marker spans to the right edge"
            );
        }

        #[test]
        fn hue_strip_emits_gradient_border_and_boundary_colors() {
            let mut ctx = test_context();
            let mut h = 180.0;
            let rect = settle_strip(&mut ctx, &mut h);

            let data = ctx.windows[0].draw_list.as_draw_data();
            let b_y = rect[1] + (rect[3] - rect[1]) * 6.0 / 6.0;
            for pos in [[rect[0], rect[1]], [rect[0], b_y]] {
                assert!(
                    data.vertices
                        .iter()
                        .any(|v| v.pos == pos && v.col == [255, 0, 0, 255]),
                    "hue strip edge {pos:?} must be red"
                );
            }

            let border = ctx.style.border_size;
            assert_eq!(
                fill_color_at(&ctx, [rect[0], rect[1], rect[2], rect[1] + border]),
                Some(crate::widgets::lighten(
                    ctx.style.widget_bg,
                    crate::widgets::BORDER_LIGHTEN
                )),
                "the strip must draw its frame border"
            );
        }
    }

    mod z_order {
        use crate::context::Context;
        use crate::id::{Id, IdStack};
        use crate::input::InputState;
        use crate::scroll::ScrollFlags;

        const DEJAVU_SANS: &[u8] = include_bytes!("../tests/assets/DejaVuSans.ttf");
        const AWAY: [f32; 2] = [-50.0, -50.0];
        const POPUP_ID: Id = Id(0x50_4f_50);

        fn test_context() -> Context {
            Context::new(DEJAVU_SANS).expect("DejaVu Sans must load")
        }

        fn widx(ctx: &Context, title: &str) -> usize {
            ctx.windows
                .iter()
                .position(|w| w.title == title)
                .expect("window must exist")
        }

        fn widget_id(window: &str, label: &str) -> Id {
            let mut stack = IdStack::new();
            stack.push(window);
            stack.id(label)
        }

        fn chrome_id(window: &str) -> Id {
            let mut stack = IdStack::new();
            stack.push(window);
            stack.id(crate::ui::TITLE_BAR_TAG)
        }

        fn popup_button_id(window: &str, label: &str) -> Id {
            let mut stack = IdStack::new();
            stack.push(window);
            stack.push_id(POPUP_ID);
            stack.id(label)
        }

        fn title_bar_height(ctx: &Context) -> f32 {
            ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1]
        }

        fn first_button_rect(ctx: &mut Context, title: &str, label: &str) -> [f32; 4] {
            let style = ctx.style;
            let text = ctx.font.measure(label);
            let title_bar_height = ctx.font().line_height() + 2.0 * style.frame_padding[1];
            let window = &ctx.windows[widx(ctx, title)];
            let pos = [
                window.pos[0] + style.window_padding[0],
                window.pos[1] + title_bar_height + style.window_padding[1],
            ];
            let size = [
                text[0] + 2.0 * style.frame_padding[0],
                text[1] + 2.0 * style.frame_padding[1],
            ];
            [pos[0], pos[1], pos[0] + size[0], pos[1] + size[1]]
        }

        fn fill_color_in(ctx: &Context, title: &str, rect: [f32; 4]) -> Option<[u8; 4]> {
            crate::widgets::test_support::chrome_color_in(
                ctx.windows[widx(ctx, title)].draw_list.as_draw_data(),
                rect,
            )
        }

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn frame_ab(ctx: &mut Context, mouse: [f32; 2], down: bool, a: &mut bool, b: &mut bool) {
            ctx.set_input(InputState {
                mouse_pos: mouse,
                mouse_down: [down, false, false, false, false],
                ..Default::default()
            });
            ctx.new_frame(0.016);
            {
                let mut ui = ctx.ui();
                ui.window("A", ScrollFlags::NONE, |ui| *a = ui.button("A-btn"));
                ui.window("B", ScrollFlags::NONE, |ui| *b = ui.button("B-btn"));
            }
            ctx.render();
        }

        fn settle_stacked(ctx: &mut Context) -> ([f32; 4], [f32; 4], [f32; 2]) {
            let (mut a, mut b) = (false, false);
            frame_ab(ctx, AWAY, false, &mut a, &mut b);
            let ia = widx(ctx, "A");
            let ib = widx(ctx, "B");
            let pos_a = ctx.windows[ia].pos;
            ctx.windows[ib].pos = pos_a;
            let rect_a = first_button_rect(ctx, "A", "A-btn");
            let rect_b = first_button_rect(ctx, "B", "B-btn");
            let at = center(rect_a);
            assert!(
                crate::ui::point_in_rect(at, rect_b),
                "the scripted point must lie inside both stacked buttons"
            );
            (rect_a, rect_b, at)
        }

        #[test]
        fn front_window_wins_the_press_in_the_overlap() {
            let mut ctx = test_context();
            let (_, _, at) = settle_stacked(&mut ctx);
            let (mut a, mut b) = (false, true);
            frame_ab(&mut ctx, at, true, &mut a, &mut b);
            assert_eq!(
                ctx.active,
                Some(widget_id("B", "B-btn")),
                "the front window's widget must capture the press"
            );
            frame_ab(&mut ctx, at, false, &mut a, &mut b);
            assert!(b, "the front window's button must click");
            assert!(!a, "the back window's button must not see the press");
        }

        #[test]
        fn front_window_wins_even_when_declared_first() {
            let mut ctx = test_context();
            let (_, _, at) = settle_stacked(&mut ctx);
            let i = widx(&ctx, "A");
            ctx.bring_window_to_front(i);
            let (mut a, mut b) = (true, false);
            frame_ab(&mut ctx, at, true, &mut a, &mut b);
            assert_eq!(
                ctx.active,
                Some(widget_id("A", "A-btn")),
                "z-order, not declaration order, must decide the press"
            );
            frame_ab(&mut ctx, at, false, &mut a, &mut b);
            assert!(a, "the front-most window's button must click");
            assert!(!b, "the window behind must not see the press");
        }

        #[test]
        fn back_window_hover_is_suppressed_in_the_overlap() {
            let mut ctx = test_context();
            let (rect_a, rect_b, at) = settle_stacked(&mut ctx);
            let (mut a, mut b) = (false, false);
            frame_ab(&mut ctx, at, false, &mut a, &mut b);
            assert_eq!(ctx.hot, Some(widget_id("B", "B-btn")));
            assert_eq!(
                fill_color_in(&ctx, "B", rect_b),
                Some(ctx.style.widget_bg_hovered),
                "the front window's button hover-highlights"
            );
            assert_eq!(
                fill_color_in(&ctx, "A", rect_a),
                Some(ctx.style.widget_bg),
                "the back window's button must not hover-highlight"
            );
        }

        #[test]
        fn front_window_title_bar_captures_in_the_overlap_even_when_declared_first() {
            let mut ctx = test_context();
            settle_stacked(&mut ctx);
            let i = widx(&ctx, "A");
            ctx.bring_window_to_front(i);
            let pos = ctx.windows[widx(&ctx, "A")].pos;
            let grab = [pos[0] + 100.0, pos[1] + title_bar_height(&ctx) / 2.0];
            let (mut a, mut b) = (false, false);

            frame_ab(&mut ctx, grab, true, &mut a, &mut b);
            assert_eq!(
                ctx.active,
                Some(chrome_id("A")),
                "the front window's title bar must capture the drag"
            );
            let pos_b = ctx.windows[widx(&ctx, "B")].pos;
            let moved = [grab[0] + 30.0, grab[1] + 20.0];
            frame_ab(&mut ctx, moved, true, &mut a, &mut b);
            assert_eq!(
                ctx.windows[widx(&ctx, "A")].pos,
                [pos[0] + 30.0, pos[1] + 20.0],
                "the front window drags"
            );
            assert_eq!(
                ctx.windows[widx(&ctx, "B")].pos,
                pos_b,
                "the window behind must not drag"
            );
            frame_ab(&mut ctx, moved, false, &mut a, &mut b);
            assert_eq!(ctx.active, None);
        }

        #[test]
        fn press_in_back_window_body_raises_it_and_applies_to_its_widget() {
            let mut ctx = test_context();
            let (mut a, mut b) = (false, false);
            frame_ab(&mut ctx, AWAY, false, &mut a, &mut b);
            let ia = widx(&ctx, "A");
            let ib = widx(&ctx, "B");
            ctx.windows[ia].pos = [60.0, 60.0];
            ctx.windows[ib].pos = [300.0, 60.0];
            let rect_a = first_button_rect(&mut ctx, "A", "A-btn");
            let at = center(rect_a);
            let rect_b = ctx.windows[widx(&ctx, "B")].rect();
            assert!(
                at[0] < rect_b[0],
                "A's button must be clear of B for this script"
            );

            frame_ab(&mut ctx, at, true, &mut a, &mut b);
            assert_eq!(ctx.active, Some(widget_id("A", "A-btn")));
            assert_eq!(
                widx(&ctx, "A"),
                ctx.windows.len() - 1,
                "a press in the body must raise the back window"
            );
            frame_ab(&mut ctx, at, false, &mut a, &mut b);
            assert!(a, "the raised window's button must click");
            assert!(!b);
        }

        #[test]
        fn title_bar_drag_works_after_a_body_press_raise() {
            let mut ctx = test_context();
            let (mut a, mut b) = (false, false);
            frame_ab(&mut ctx, AWAY, false, &mut a, &mut b);
            let ia = widx(&ctx, "A");
            let ib = widx(&ctx, "B");
            ctx.windows[ia].pos = [60.0, 60.0];
            ctx.windows[ib].pos = [300.0, 60.0];
            let at = center(first_button_rect(&mut ctx, "A", "A-btn"));
            frame_ab(&mut ctx, at, true, &mut a, &mut b);
            frame_ab(&mut ctx, at, false, &mut a, &mut b);
            assert_eq!(widx(&ctx, "A"), ctx.windows.len() - 1);

            let grab = [320.0, 60.0 + title_bar_height(&ctx) / 2.0];
            frame_ab(&mut ctx, grab, true, &mut a, &mut b);
            assert_eq!(ctx.active, Some(chrome_id("A")));
            let pos_b = ctx.windows[widx(&ctx, "B")].pos;
            let moved = [grab[0] + 25.0, grab[1] + 15.0];
            frame_ab(&mut ctx, moved, true, &mut a, &mut b);
            assert_eq!(ctx.windows[widx(&ctx, "A")].pos, [85.0, 75.0]);
            assert_eq!(ctx.windows[widx(&ctx, "B")].pos, pos_b);
            frame_ab(&mut ctx, moved, false, &mut a, &mut b);
            assert_eq!(ctx.active, None);
        }

        fn frame_popup(
            ctx: &mut Context,
            mouse: [f32; 2],
            down: bool,
            popup_rect: [f32; 4],
            a: &mut bool,
            b: &mut bool,
            p: &mut bool,
        ) {
            ctx.set_input(InputState {
                mouse_pos: mouse,
                mouse_down: [down, false, false, false, false],
                ..Default::default()
            });
            ctx.new_frame(0.016);
            {
                let mut ui = ctx.ui();
                ui.window("A", ScrollFlags::NONE, |ui| {
                    *a = ui.button("A-btn");
                    if *a {
                        ui.open_popup(POPUP_ID);
                    }
                    ui.popup(POPUP_ID, popup_rect, |ui| *p = ui.button("P-btn"));
                });
                ui.window("B", ScrollFlags::NONE, |ui| *b = ui.button("B-btn"));
            }
            ctx.render();
        }

        fn popup_button_rect(ctx: &mut Context, popup_rect: [f32; 4]) -> [f32; 4] {
            let style = ctx.style;
            let text = ctx.font.measure("P-btn");
            let pos = [
                popup_rect[0] + style.window_padding[0],
                popup_rect[1] + style.window_padding[1],
            ];
            let size = [
                text[0] + 2.0 * style.frame_padding[0],
                text[1] + 2.0 * style.frame_padding[1],
            ];
            [pos[0], pos[1], pos[0] + size[0], pos[1] + size[1]]
        }

        fn settle_with_open_popup(
            ctx: &mut Context,
            popup_rect: [f32; 4],
            a: &mut bool,
            b: &mut bool,
            p: &mut bool,
        ) {
            frame_popup(ctx, AWAY, false, popup_rect, a, b, p);
            let ib = widx(ctx, "B");
            ctx.windows[ib].pos = [500.0, 300.0];
            let at = center(first_button_rect(ctx, "A", "A-btn"));
            frame_popup(ctx, at, true, popup_rect, a, b, p);
            frame_popup(ctx, at, false, popup_rect, a, b, p);
            assert_eq!(
                ctx.open_popup,
                Some(POPUP_ID),
                "the click must open the popup"
            );
            *a = false;
        }

        #[test]
        fn popup_layer_wins_over_all_windows() {
            let mut ctx = test_context();
            let popup_rect = [100.0, 100.0, 260.0, 180.0];
            let (mut a, mut b, mut p) = (false, false, false);
            settle_with_open_popup(&mut ctx, popup_rect, &mut a, &mut b, &mut p);
            let p_rect = popup_button_rect(&mut ctx, popup_rect);
            let at = center(p_rect);
            let style = ctx.style;
            let tb = title_bar_height(&ctx);
            let ib = widx(&ctx, "B");
            ctx.windows[ib].pos = [
                p_rect[0] - style.window_padding[0],
                p_rect[1] - tb - style.window_padding[1],
            ];
            assert!(
                crate::ui::point_in_rect(at, first_button_rect(&mut ctx, "B", "B-btn")),
                "the scripted point must lie inside B's button as well"
            );
            let order_before: alloc::vec::Vec<alloc::string::String> =
                ctx.windows.iter().map(|w| w.title.clone()).collect();

            frame_popup(&mut ctx, at, true, popup_rect, &mut a, &mut b, &mut p);
            assert_eq!(
                ctx.active,
                Some(popup_button_id("A", "P-btn")),
                "the popup's widget must capture the press"
            );
            frame_popup(&mut ctx, at, false, popup_rect, &mut a, &mut b, &mut p);
            assert!(p, "the popup's button must click");
            assert!(!b, "the window under the popup must not see the press");
            let order_after: alloc::vec::Vec<alloc::string::String> =
                ctx.windows.iter().map(|w| w.title.clone()).collect();
            assert_eq!(
                order_before, order_after,
                "a press inside the popup must not raise any window"
            );
        }

        #[test]
        fn outside_press_closes_the_popup_and_is_consumed_with_windows_underneath() {
            let mut ctx = test_context();
            let popup_rect = [100.0, 100.0, 260.0, 180.0];
            let (mut a, mut b, mut p) = (false, false, false);
            settle_with_open_popup(&mut ctx, popup_rect, &mut a, &mut b, &mut p);

            let at = center(first_button_rect(&mut ctx, "A", "A-btn"));
            let ib = widx(&ctx, "B");
            ctx.windows[ib].pos = [84.0, 84.0];
            let rect_b = ctx.windows[widx(&ctx, "B")].rect();
            assert!(crate::ui::point_in_rect(at, rect_b));
            assert!(!crate::ui::point_in_rect(at, popup_rect));
            let order_before: alloc::vec::Vec<alloc::string::String> =
                ctx.windows.iter().map(|w| w.title.clone()).collect();
            frame_popup(&mut ctx, at, true, popup_rect, &mut a, &mut b, &mut p);
            assert_eq!(
                ctx.open_popup, None,
                "the outside press must close the popup, and the consumed edge must not reopen it"
            );
            assert_eq!(ctx.active, None, "the consumed press captures nothing");
            frame_popup(&mut ctx, at, false, popup_rect, &mut a, &mut b, &mut p);
            assert!(!a, "the consumed press must not click A's button");
            assert!(!b);
            let order_after: alloc::vec::Vec<alloc::string::String> =
                ctx.windows.iter().map(|w| w.title.clone()).collect();
            assert_eq!(
                order_before, order_after,
                "the consumed press must not raise any window"
            );
        }

        #[test]
        fn window_widget_under_the_open_popup_does_not_hover_highlight() {
            let mut ctx = test_context();
            let popup_rect = [100.0, 100.0, 260.0, 180.0];
            let (mut a, mut b, mut p) = (false, false, false);
            settle_with_open_popup(&mut ctx, popup_rect, &mut a, &mut b, &mut p);
            let p_rect = popup_button_rect(&mut ctx, popup_rect);
            let at = center(p_rect);
            let style = ctx.style;
            let tb = title_bar_height(&ctx);
            let ib = widx(&ctx, "B");
            ctx.windows[ib].pos = [
                p_rect[0] - style.window_padding[0],
                p_rect[1] - tb - style.window_padding[1],
            ];
            let b_rect = first_button_rect(&mut ctx, "B", "B-btn");
            assert!(
                crate::ui::point_in_rect(at, b_rect),
                "the scripted point must lie inside B's button as well"
            );

            frame_popup(&mut ctx, at, false, popup_rect, &mut a, &mut b, &mut p);
            assert_eq!(ctx.hot, Some(popup_button_id("A", "P-btn")));
            assert_eq!(
                fill_color_in(&ctx, "B", b_rect),
                Some(ctx.style.widget_bg),
                "a window widget under the open popup must not hover-highlight"
            );
        }
    }

    mod close_button {
        use crate::context::Context;
        use crate::id::{Id, IdStack};

        const AWAY: [f32; 2] = [-50.0, -50.0];

        fn widx(ctx: &Context, title: &str) -> usize {
            ctx.windows
                .iter()
                .position(|w| w.title == title)
                .expect("window must exist")
        }

        fn close_id(title: &str) -> Id {
            let mut stack = IdStack::new();
            stack.push(title);
            stack.id(crate::ui::CLOSE_BUTTON_TAG)
        }

        fn close_zone(ctx: &Context, title: &str) -> [f32; 4] {
            let tb = super::title_bar_height(ctx);
            let window = &ctx.windows[widx(ctx, title)];
            [
                window.pos[0] + window.size[0] - tb,
                window.pos[1],
                window.pos[0] + window.size[0],
                window.pos[1] + tb,
            ]
        }

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn fill_color_in(ctx: &Context, title: &str, rect: [f32; 4]) -> Option<[u8; 4]> {
            crate::widgets::test_support::chrome_color_in(
                ctx.windows[widx(ctx, title)].draw_list.as_draw_data(),
                rect,
            )
        }

        #[test]
        fn close_glyph_renders_inside_a_square_zone_at_the_bars_right_end() {
            let mut ctx = super::test_context();
            ctx.style.anti_aliasing = false;
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            let zone = close_zone(&ctx, "Win");
            let tb = super::title_bar_height(&ctx);
            assert_eq!(
                zone[2] - zone[0],
                tb,
                "the zone must be a title-bar-height square"
            );
            let window = &ctx.windows[widx(&ctx, "Win")];
            assert_eq!(
                zone[2],
                window.pos[0] + window.size[0],
                "the zone must be flush with the title bar's right end"
            );
            assert_eq!(zone[3], window.pos[1] + tb);

            let data = window.draw_list.as_draw_data();
            let mut x_vertices = 0usize;
            let mut index_offset = 0usize;
            for cmd in data.commands {
                let next = index_offset + cmd.elem_count as usize;
                if cmd.texture != crate::draw::TextureId::FONT {
                    for &i in &data.indices[index_offset..next] {
                        let v = data.vertices[i as usize];
                        if v.col == ctx.style.title_text && v.pos[1] <= zone[3] {
                            assert!(
                                v.pos[0] >= zone[0]
                                    && v.pos[0] <= zone[2]
                                    && v.pos[1] >= zone[1]
                                    && v.pos[1] <= zone[3],
                                "× glyph vertex {:?} outside the close zone {zone:?}",
                                v.pos
                            );
                            x_vertices += 1;
                        }
                    }
                }
                index_offset = next;
            }
            assert!(
                x_vertices >= 8,
                "the × must emit two line quads; got {x_vertices} vertices"
            );
        }

        #[test]
        fn title_text_stays_left_of_the_close_zone() {
            let mut ctx = super::test_context();
            let title = "A Fairly Long Window Title That Would Reach Into The Close Zone";
            super::frame(&mut ctx, AWAY, false, &[title]);
            let zone = close_zone(&ctx, title);
            let window = &ctx.windows[widx(&ctx, title)];
            let text_x = window.pos[0] + ctx.style.window_padding[0];
            let full_width = ctx.font.measure(title)[0];
            assert!(
                text_x + full_width > zone[0],
                "the script's title must be long enough to overlap the zone untruncated"
            );
            let data = window.draw_list.as_draw_data();
            let mut index_offset = 0usize;
            let mut glyph_vertices = 0usize;
            for cmd in data.commands {
                let next = index_offset + cmd.elem_count as usize;
                if cmd.texture == crate::draw::TextureId::FONT {
                    for &i in &data.indices[index_offset..next] {
                        let v = data.vertices[i as usize];
                        if ctx.font.uv_samples_shape_mask(v.uv) {
                            continue;
                        }
                        assert!(
                            v.pos[0] <= zone[0] + 1e-3,
                            "title glyph vertex {:?} enters the close zone {zone:?}",
                            v.pos
                        );
                        glyph_vertices += 1;
                    }
                }
                index_offset = next;
            }
            assert!(
                glyph_vertices > 0,
                "the (truncated) title must still render"
            );
        }

        #[test]
        fn click_close_button_closes_the_window_other_windows_unaffected() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["A", "B"]);
            let zone = close_zone(&ctx, "A");
            let at = center(zone);
            let rect_b = ctx.windows[widx(&ctx, "B")].rect();
            assert!(
                !crate::ui::point_in_rect(at, rect_b),
                "A's close zone must be clear of B for this script"
            );
            let pos_a = ctx.windows[widx(&ctx, "A")].pos;

            super::frame(&mut ctx, at, true, &["A", "B"]);
            assert_eq!(ctx.active, Some(close_id("A")));
            assert!(
                ctx.windows[widx(&ctx, "A")].open,
                "the press alone must not close"
            );
            super::frame(&mut ctx, at, false, &["A", "B"]);
            assert_eq!(ctx.active, None, "the click must drop the capture");
            assert!(
                !ctx.windows[widx(&ctx, "A")].open,
                "the release inside the × must close the window"
            );

            super::frame(&mut ctx, AWAY, false, &["A", "B"]);
            let ia = widx(&ctx, "A");
            let ib = widx(&ctx, "B");
            assert!(
                ctx.windows[ia].draw_list.as_draw_data().indices.is_empty(),
                "a closed window must emit nothing"
            );
            assert_eq!(
                ctx.windows[ia].pos, pos_a,
                "closing keeps the window's state"
            );
            assert!(ctx.windows[ib].open);
            assert!(
                !ctx.windows[ib].draw_list.as_draw_data().indices.is_empty(),
                "the other window must keep emitting"
            );
        }

        #[test]
        fn set_window_open_true_reopens_a_closed_window() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["A"]);
            let at = center(close_zone(&ctx, "A"));
            super::frame(&mut ctx, at, true, &["A"]);
            super::frame(&mut ctx, at, false, &["A"]);
            assert!(!ctx.windows[widx(&ctx, "A")].open);
            super::frame(&mut ctx, AWAY, false, &["A"]);
            assert!(ctx.windows[widx(&ctx, "A")]
                .draw_list
                .as_draw_data()
                .indices
                .is_empty());

            ctx.set_window_open("A", true);
            assert!(ctx.windows[widx(&ctx, "A")].open);
            super::frame(&mut ctx, AWAY, false, &["A"]);
            assert!(
                !ctx.windows[widx(&ctx, "A")]
                    .draw_list
                    .as_draw_data()
                    .indices
                    .is_empty(),
                "a reopened window must emit again"
            );
        }

        #[test]
        fn set_window_open_with_unknown_title_is_a_noop() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["A"]);
            ctx.set_window_open("Nope", false);
            ctx.set_window_open("Nope", true);
            assert_eq!(
                ctx.windows.len(),
                1,
                "an unknown title must not create a window"
            );
            assert!(ctx.windows[0].open);
        }

        #[test]
        fn back_window_close_button_does_not_highlight_under_a_front_window() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["A", "B"]);
            let pos_a = ctx.windows[widx(&ctx, "A")].pos;
            let ib = widx(&ctx, "B");
            ctx.windows[ib].pos = pos_a;
            let zone = close_zone(&ctx, "A");
            assert_eq!(
                zone,
                close_zone(&ctx, "B"),
                "stacked windows share the zone"
            );
            let at = center(zone);

            super::frame(&mut ctx, at, false, &["A", "B"]);
            assert_eq!(
                fill_color_in(&ctx, "B", zone),
                Some(ctx.style.widget_bg_hovered),
                "the front window's × must hover-highlight"
            );
            assert_eq!(
                fill_color_in(&ctx, "A", zone),
                None,
                "the back window's × under the front window must not highlight"
            );
        }

        #[test]
        fn back_window_close_button_neither_captures_nor_closes_under_a_front_window() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["A", "B"]);
            let pos_a = ctx.windows[widx(&ctx, "A")].pos;
            let ib = widx(&ctx, "B");
            ctx.windows[ib].pos = pos_a;
            let at = center(close_zone(&ctx, "A"));

            super::frame(&mut ctx, at, true, &["A", "B"]);
            assert_eq!(ctx.active, Some(close_id("B")));
            super::frame(&mut ctx, at, false, &["A", "B"]);
            assert!(
                ctx.windows[widx(&ctx, "A")].open,
                "A's × must not fire under B"
            );
            assert!(!ctx.windows[widx(&ctx, "B")].open, "B's × must close B");
        }

        #[test]
        fn back_window_close_button_press_covered_by_a_front_window_body_is_inert() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["A", "B"]);
            let zone_a = close_zone(&ctx, "A");
            let at = center(zone_a);
            let tb = super::title_bar_height(&ctx);
            let ib = widx(&ctx, "B");
            ctx.windows[ib].pos = [at[0] - 50.0, at[1] - tb - 10.0];
            let rect_b = ctx.windows[ib].rect();
            assert!(crate::ui::point_in_rect(at, rect_b));
            assert!(
                at[1] > ctx.windows[ib].pos[1] + tb,
                "the point must be in B's body"
            );

            super::frame(&mut ctx, at, true, &["A", "B"]);
            assert_eq!(
                ctx.active, None,
                "a body press captures no chrome, and A's gated × must not capture"
            );
            super::frame(&mut ctx, at, false, &["A", "B"]);
            assert!(
                ctx.windows[widx(&ctx, "A")].open,
                "A's × must not close from a press attributed to B"
            );
            assert!(ctx.windows[widx(&ctx, "B")].open);
        }

        #[test]
        fn title_drag_released_on_the_close_button_does_not_close() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["A"]);
            let pos = ctx.windows[0].pos;
            let tb = super::title_bar_height(&ctx);
            let grab = [pos[0] + 40.0, pos[1] + tb / 2.0];
            assert!(grab[0] < close_zone(&ctx, "A")[0]);
            super::frame(&mut ctx, grab, true, &["A"]);
            assert_eq!(ctx.active, Some(super::chrome_id("A")));

            let moved = [grab[0] + 20.0, grab[1]];
            super::frame(&mut ctx, moved, true, &["A"]);
            assert_eq!(ctx.active, Some(super::chrome_id("A")));

            let at = center(close_zone(&ctx, "A"));
            super::frame(&mut ctx, at, false, &["A"]);
            assert_eq!(ctx.active, None);
            assert!(
                ctx.windows[0].open,
                "releasing a title drag over the × must not close the window"
            );
        }

        #[test]
        fn set_window_open_false_clears_the_windows_chrome_capture() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["A"]);
            let pos = ctx.windows[0].pos;
            let tb = super::title_bar_height(&ctx);
            let grab = [pos[0] + 40.0, pos[1] + tb / 2.0];
            super::frame(&mut ctx, grab, true, &["A"]);
            assert_eq!(ctx.active, Some(super::chrome_id("A")));

            ctx.set_window_open("A", false);
            assert!(!ctx.windows[0].open);
            assert_eq!(
                ctx.active, None,
                "closing must clear the window's title-bar capture"
            );
            super::frame(&mut ctx, grab, false, &["A"]);
            assert!(!ctx.windows[0].open);
            assert_eq!(ctx.active, None);
        }
    }

    mod resize_grip {
        use crate::context::{Context, MIN_WINDOW_SIZE};
        use crate::id::{Id, IdStack};
        use crate::input::InputState;
        use crate::scroll::ScrollFlags;
        use crate::ui::RESIZE_GRIP_SIZE;

        const AWAY: [f32; 2] = [-50.0, -50.0];

        fn widx(ctx: &Context, title: &str) -> usize {
            ctx.windows
                .iter()
                .position(|w| w.title == title)
                .expect("window must exist")
        }

        fn resize_id(title: &str) -> Id {
            let mut stack = IdStack::new();
            stack.push(title);
            stack.id(crate::ui::RESIZE_GRIP_TAG)
        }

        fn grip_zone(ctx: &Context, title: &str) -> [f32; 4] {
            let window = &ctx.windows[widx(ctx, title)];
            let rect = window.rect();
            [
                rect[2] - RESIZE_GRIP_SIZE,
                rect[3] - RESIZE_GRIP_SIZE,
                rect[2],
                rect[3],
            ]
        }

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn frame_win(
            ctx: &mut Context,
            mouse_pos: [f32; 2],
            left_down: bool,
            f: &mut impl FnMut(&mut crate::ui::Ui<'_>),
        ) {
            ctx.set_input(InputState {
                mouse_pos,
                mouse_down: [left_down, false, false, false, false],
                ..Default::default()
            });
            ctx.new_frame(0.016);
            ctx.ui().window("Win", ScrollFlags::NONE, f);
            ctx.render();
        }

        fn marker_vertices(ctx: &Context, title: &str) -> alloc::vec::Vec<[f32; 2]> {
            let zone = grip_zone(ctx, title);
            let window = &ctx.windows[widx(ctx, title)];
            window
                .draw_list
                .as_draw_data()
                .vertices
                .iter()
                .filter(|v| {
                    v.col == ctx.style.title_text
                        && v.pos[0] >= zone[0]
                        && v.pos[0] <= zone[2]
                        && v.pos[1] >= zone[1]
                        && v.pos[1] <= zone[3]
                })
                .map(|v| v.pos)
                .collect()
        }

        #[test]
        fn grip_marker_renders_a_triangle_in_the_bottom_right_corner() {
            let mut ctx = super::test_context();
            ctx.style.anti_aliasing = false;
            ctx.style.corner_rounding = 0.0;
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            let zone = grip_zone(&ctx, "Win");
            let window = &ctx.windows[widx(&ctx, "Win")];
            let rect = window.rect();
            assert_eq!(
                zone[2] - zone[0],
                RESIZE_GRIP_SIZE,
                "the zone must be a RESIZE_GRIP_SIZE square"
            );
            assert_eq!(
                [zone[2], zone[3]],
                [rect[2], rect[3]],
                "flush with the corner"
            );
            assert!(
                zone[1] >= rect[1] + super::title_bar_height(&ctx),
                "the grip must sit in the body, never in the title bar"
            );

            let expected = [
                [rect[2], rect[3] - RESIZE_GRIP_SIZE],
                [rect[2], rect[3]],
                [rect[2] - RESIZE_GRIP_SIZE, rect[3]],
            ];
            let marker = marker_vertices(&ctx, "Win");
            assert_eq!(
                marker.len(),
                3,
                "the marker must be one triangle; got vertices {marker:?}"
            );
            for corner in expected {
                assert!(
                    marker.contains(&corner),
                    "marker triangle must contain corner {corner:?}; got {marker:?}"
                );
            }
        }

        #[test]
        fn grip_marker_follows_the_corner_rounding() {
            let mut ctx = super::test_context();
            ctx.style.anti_aliasing = false;
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            let window = &ctx.windows[widx(&ctx, "Win")];
            let rect = window.rect();
            let r = ctx.style.corner_rounding;
            assert_eq!(r, 4.0, "the probe expects the default rounding");
            let inset = r * (1.0 - core::f32::consts::FRAC_1_SQRT_2);

            let expected = [
                [rect[2] - inset, rect[3] - RESIZE_GRIP_SIZE],
                [rect[2] - inset, rect[3] - inset],
                [rect[2] - RESIZE_GRIP_SIZE, rect[3] - inset],
            ];
            let marker = marker_vertices(&ctx, "Win");
            assert_eq!(
                marker.len(),
                3,
                "the marker must be one triangle; got vertices {marker:?}"
            );
            for corner in expected {
                assert!(
                    marker.contains(&corner),
                    "marker triangle must contain corner {corner:?}; got {marker:?}"
                );
            }
            let arc_center = [rect[2] - r, rect[3] - r];
            let tip = [rect[2] - inset, rect[3] - inset];
            let d = ((tip[0] - arc_center[0]).powi(2) + (tip[1] - arc_center[1]).powi(2)).sqrt();
            assert!(
                (d - r).abs() <= 1e-3,
                "the tip must lie ON the corner arc: distance {d} != radius {r}"
            );
            for v in &marker {
                if v[0] >= arc_center[0] && v[1] >= arc_center[1] {
                    let dv =
                        ((v[0] - arc_center[0]).powi(2) + (v[1] - arc_center[1]).powi(2)).sqrt();
                    assert!(
                        dv <= r + 1e-3,
                        "marker vertex {v:?} outside the rounded silhouette: {dv} > {r}"
                    );
                }
            }
        }

        #[test]
        fn grip_marker_is_emitted_after_the_windows_content() {
            let mut ctx = super::test_context();
            ctx.style.anti_aliasing = false;
            const CONTENT: [u8; 4] = [7, 7, 7, 255];
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            let zone = grip_zone(&ctx, "Win");
            let mut body = |ui: &mut crate::ui::Ui<'_>| {
                ui.draw_rect_filled([zone[0], zone[1]], [zone[2], zone[3]], CONTENT);
            };
            frame_win(&mut ctx, AWAY, false, &mut body);

            let window = &ctx.windows[widx(&ctx, "Win")];
            let data = window.draw_list.as_draw_data();
            let title_text = ctx.style.title_text;
            let marker_idx: alloc::vec::Vec<usize> = data
                .vertices
                .iter()
                .enumerate()
                .filter(|(_, v)| {
                    v.col == title_text
                        && v.pos[0] >= zone[0]
                        && v.pos[0] <= zone[2]
                        && v.pos[1] >= zone[1]
                        && v.pos[1] <= zone[3]
                })
                .map(|(i, _)| i)
                .collect();
            let last_content = data
                .vertices
                .iter()
                .rposition(|v| v.col == CONTENT)
                .expect("the content rect must emit vertices");
            assert_eq!(marker_idx.len(), 3, "the marker must be one triangle");
            assert!(
                marker_idx.iter().all(|&m| m > last_content),
                "every marker vertex must be emitted after the content \
                 (marker indices {marker_idx:?}, last content index {last_content})"
            );
            assert_eq!(
                *marker_idx.last().unwrap(),
                data.vertices.len() - 1,
                "the marker must be the window's last emitted geometry"
            );
        }

        #[test]
        fn grip_drag_resizes_the_window_following_the_mouse() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            let start_pos = ctx.windows[0].pos;
            let start_size = ctx.windows[0].size;
            let grip = center(grip_zone(&ctx, "Win"));

            super::frame(&mut ctx, grip, true, &["Win"]);
            assert_eq!(ctx.active, Some(resize_id("Win")));
            assert_eq!(
                ctx.windows[0].size, start_size,
                "the press alone must not resize"
            );

            let moved = [grip[0] + 40.0, grip[1] + 25.0];
            super::frame(&mut ctx, moved, true, &["Win"]);
            assert_eq!(ctx.active, Some(resize_id("Win")));
            assert_eq!(
                ctx.windows[0].size,
                [start_size[0] + 40.0, start_size[1] + 25.0]
            );
            assert_eq!(
                ctx.windows[0].pos, start_pos,
                "resizing must not move the window"
            );

            super::frame(&mut ctx, AWAY, false, &["Win"]);
            assert_eq!(ctx.active, None);
            assert_eq!(
                ctx.windows[0].size,
                [start_size[0] + 40.0, start_size[1] + 25.0]
            );
        }

        #[test]
        fn grip_drag_clamps_to_min_window_size() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            let pos = ctx.windows[0].pos;
            let grip = center(grip_zone(&ctx, "Win"));

            super::frame(&mut ctx, grip, true, &["Win"]);
            assert_eq!(ctx.active, Some(resize_id("Win")));
            super::frame(&mut ctx, [pos[0] + 1.0, pos[1] + 1.0], true, &["Win"]);
            assert_eq!(ctx.windows[0].size, MIN_WINDOW_SIZE);
            let zone = grip_zone(&ctx, "Win");
            assert!(
                zone[1] >= pos[1] + super::title_bar_height(&ctx),
                "even at MIN_WINDOW_SIZE the grip must stay in the body"
            );
            super::frame(&mut ctx, [pos[0] - 500.0, pos[1] - 500.0], true, &["Win"]);
            assert_eq!(ctx.windows[0].size, MIN_WINDOW_SIZE);
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            assert_eq!(ctx.active, None);
            assert_eq!(
                ctx.windows[0].size, MIN_WINDOW_SIZE,
                "the clamped size persists after release"
            );
        }

        #[test]
        fn grip_and_title_bar_captures_are_distinct_zones() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            let start_pos = ctx.windows[0].pos;
            let start_size = ctx.windows[0].size;
            let tb = super::title_bar_height(&ctx);

            let grab = [start_pos[0] + 40.0, start_pos[1] + tb / 2.0];
            assert!(
                grab[0] < start_pos[0] + start_size[0] - tb,
                "grab must miss the close zone"
            );
            super::frame(&mut ctx, grab, true, &["Win"]);
            assert_eq!(ctx.active, Some(super::chrome_id("Win")));
            super::frame(&mut ctx, [grab[0] + 30.0, grab[1] + 20.0], true, &["Win"]);
            assert_eq!(
                ctx.windows[0].size, start_size,
                "a title-bar drag must not resize"
            );
            super::frame(&mut ctx, AWAY, false, &["Win"]);

            let pos = ctx.windows[0].pos;
            let grip = center(grip_zone(&ctx, "Win"));
            super::frame(&mut ctx, grip, true, &["Win"]);
            assert_eq!(ctx.active, Some(resize_id("Win")));
            super::frame(&mut ctx, [grip[0] + 25.0, grip[1] + 15.0], true, &["Win"]);
            assert_eq!(
                ctx.windows[0].pos, pos,
                "a grip drag must not move the window"
            );
            assert_eq!(
                ctx.windows[0].size,
                [start_size[0] + 25.0, start_size[1] + 15.0]
            );
            super::frame(&mut ctx, AWAY, false, &["Win"]);
        }

        #[test]
        fn grip_press_is_hidden_from_body_widgets() {
            let mut ctx = super::test_context();
            let widget_rect = core::cell::Cell::new([0.0; 4]);
            let widget_saw_press = core::cell::Cell::new(false);
            let mut body = |ui: &mut crate::ui::Ui<'_>| {
                let rect = ui.allocate([400.0, 400.0]);
                widget_rect.set(rect);
                let id = ui.widget_id("##body");
                if ui.mouse_pressed(0) && crate::ui::point_in_rect(ui.mouse_pos(), rect) {
                    ui.capture(id);
                    widget_saw_press.set(true);
                }
            };
            frame_win(&mut ctx, AWAY, false, &mut body);
            let grip = center(grip_zone(&ctx, "Win"));
            assert!(
                crate::ui::point_in_rect(grip, widget_rect.get()),
                "the script's widget must cover the grip zone"
            );

            frame_win(&mut ctx, grip, true, &mut body);
            assert_eq!(
                ctx.active,
                Some(resize_id("Win")),
                "the grip must keep its press against body widgets"
            );
            assert!(
                !widget_saw_press.get(),
                "a press in the grip must not reach body widgets"
            );
            frame_win(&mut ctx, grip, false, &mut body);
            assert!(!widget_saw_press.get());
            assert_eq!(ctx.active, None);
        }

        #[test]
        fn layout_follows_the_new_size() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            let style = ctx.style;
            let tb = super::title_bar_height(&ctx);
            let grip = center(grip_zone(&ctx, "Win"));
            super::frame(&mut ctx, grip, true, &["Win"]);
            super::frame(&mut ctx, [grip[0] + 60.0, grip[1] + 30.0], true, &["Win"]);
            super::frame(&mut ctx, AWAY, false, &["Win"]);

            let window = &ctx.windows[0];
            let pos = window.pos;
            let size = window.size;
            assert_eq!(
                window.cursor.next_pos(),
                [
                    pos[0] + style.window_padding[0],
                    pos[1] + tb + style.window_padding[1]
                ]
            );
            assert_eq!(
                window.cursor.content_width(),
                size[0] - 2.0 * style.window_padding[0]
            );
        }

        #[test]
        fn small_window_near_viewport_edge_keeps_the_drag_clamp_sound() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            ctx.windows[0].size = MIN_WINDOW_SIZE;
            let pos = ctx.windows[0].pos;
            let tb = super::title_bar_height(&ctx);
            let grab = [pos[0] + 2.0, pos[1] + tb / 2.0];

            let viewport = [140.0, 70.0];
            super::frame_viewport(&mut ctx, grab, true, viewport, &["Win"]);
            assert_eq!(ctx.active, Some(super::chrome_id("Win")));
            super::frame_viewport(&mut ctx, [-500.0, -500.0], true, viewport, &["Win"]);
            assert_eq!(ctx.windows[0].pos, [32.0 - MIN_WINDOW_SIZE[0], 0.0]);

            let tiny = [100.0, 20.0];
            super::frame_viewport(&mut ctx, [500.0, 500.0], true, tiny, &["Win"]);
            assert_eq!(ctx.windows[0].pos, [tiny[0] - 32.0, 0.0]);
            super::frame_viewport(&mut ctx, AWAY, false, tiny, &["Win"]);
            assert_eq!(ctx.active, None);
        }

        #[test]
        fn closed_window_emits_no_grip_and_cannot_be_resized() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            let grip = center(grip_zone(&ctx, "Win"));
            let size = ctx.windows[0].size;

            ctx.set_window_open("Win", false);
            super::frame(&mut ctx, grip, true, &["Win"]);
            assert_eq!(ctx.active, None, "a closed window's grip must not capture");
            assert!(
                ctx.windows[0].draw_list.as_draw_data().indices.is_empty(),
                "a closed window emits no grip marker"
            );
            assert_eq!(ctx.windows[0].size, size);
            super::frame(&mut ctx, grip, false, &["Win"]);
            assert_eq!(
                ctx.windows[0].size, size,
                "a closed window cannot be resized"
            );
        }

        #[test]
        fn set_window_open_false_clears_the_resize_capture() {
            let mut ctx = super::test_context();
            super::frame(&mut ctx, AWAY, false, &["Win"]);
            let grip = center(grip_zone(&ctx, "Win"));
            super::frame(&mut ctx, grip, true, &["Win"]);
            assert_eq!(ctx.active, Some(resize_id("Win")));
            super::frame(&mut ctx, [grip[0] + 30.0, grip[1] + 30.0], true, &["Win"]);
            assert_eq!(ctx.active, Some(resize_id("Win")));

            ctx.set_window_open("Win", false);
            assert!(!ctx.windows[0].open);
            assert_eq!(ctx.active, None, "closing must clear the resize capture");
            super::frame(&mut ctx, grip, false, &["Win"]);
            assert!(!ctx.windows[0].open);
            assert_eq!(ctx.active, None);
        }
    }

    mod scroll_region {
        use crate::context::{Context, WidgetState};
        use crate::id::Id;
        use crate::input::InputState;
        use crate::ui::Ui;
        use crate::widgets::test_support::{fill_color_at, next_text_rect};

        const AWAY: [f32; 2] = [-50.0, -50.0];
        const REGION_SIZE: [f32; 2] = [200.0, 60.0];

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn region_id() -> Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            stack.id("region")
        }

        fn content_clip(ctx: &Context, region: [f32; 4]) -> [f32; 4] {
            let i = crate::scroll::content_inset(&ctx.style, region);
            [region[0] + i, region[1] + i, region[2] - i, region[3] - i]
        }

        fn button_id_in_region(label: &str) -> Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            stack.push_id(region_id());
            stack.id(label)
        }

        fn scroll_state(ctx: &Context) -> (f32, f32) {
            match ctx.widget_states.get(&region_id()) {
                Some(WidgetState::Scroll {
                    offset,
                    content_height,
                }) => (*offset, *content_height),
                _ => (0.0, 0.0),
            }
        }

        struct Probe {
            region: [f32; 4],
            target_rect: [f32; 4],
            target_clicked: bool,
        }

        fn declare(ui: &mut Ui, target: &str, probe: &mut Probe) {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            probe.region = [
                pos[0],
                pos[1],
                pos[0] + REGION_SIZE[0],
                pos[1] + REGION_SIZE[1],
            ];
            ui.scroll_region("region", REGION_SIZE, |ui| {
                for i in 0..10 {
                    let label = alloc::format!("B{i}");
                    if label == target {
                        probe.target_rect = next_text_rect(ui, &label);
                        probe.target_clicked = ui.button(&label);
                    } else {
                        ui.button(&label);
                    }
                }
            });
        }

        fn frame(ctx: &mut Context, mouse: [f32; 2], wheel: f32, f: impl FnOnce(&mut Ui)) {
            super::popup_frame(
                ctx,
                InputState {
                    mouse_pos: mouse,
                    mouse_wheel: wheel,
                    ..Default::default()
                },
                f,
            );
        }

        fn click_frame(
            ctx: &mut Context,
            mouse: [f32; 2],
            left_down: bool,
            f: impl FnOnce(&mut Ui),
        ) {
            super::popup_frame(
                ctx,
                InputState {
                    mouse_pos: mouse,
                    mouse_down: [left_down, false, false, false, false],
                    ..Default::default()
                },
                f,
            );
        }

        #[test]
        fn content_taller_than_the_region_emits_under_the_region_clip() {
            let mut ctx = super::test_context();
            let mut probe = Probe {
                region: [0.0; 4],
                target_rect: [0.0; 4],
                target_clicked: false,
            };
            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B9", &mut probe));
            let region = probe.region;
            let last = probe.target_rect;

            let window_rect = ctx.windows[0].rect();
            assert!(
                last[1] > region[3],
                "content must overflow the region: {last:?} vs {region:?}"
            );
            assert!(last[3] < window_rect[3]);

            let content_clip = content_clip(&ctx, region);
            let data = ctx.windows[0].draw_list.as_draw_data();
            let mut has_region_clip_cmd = false;
            let mut overflow_verts = 0usize;
            let mut offset = 0usize;
            for cmd in data.commands {
                let next = offset + cmd.elem_count as usize;
                if cmd.clip_rect == content_clip {
                    has_region_clip_cmd = true;
                }
                for &i in &data.indices[offset..next] {
                    let v = data.vertices[i as usize];
                    if v.pos[1] > region[3] && v.pos[1] < last[3] + 1.0 {
                        overflow_verts += 1;
                        assert_eq!(
                            cmd.clip_rect, content_clip,
                            "overflow vertex {:?} must carry the inset content clip, got {:?}",
                            v.pos, cmd.clip_rect
                        );
                    }
                }
                offset = next;
            }
            assert!(
                has_region_clip_cmd,
                "no command carries the inset content clip rect"
            );
            assert!(overflow_verts > 0, "no vertices found below the region");
        }

        #[test]
        fn the_region_draws_a_panel_and_leaves_the_clip_stack_balanced() {
            let mut ctx = super::test_context();
            let mut probe = Probe {
                region: [0.0; 4],
                target_rect: [0.0; 4],
                target_clicked: false,
            };
            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B0", &mut probe));
            assert_eq!(
                fill_color_at(&ctx, probe.region),
                Some(ctx.style.widget_bg),
                "the region draws a widget-style background panel"
            );
            assert_eq!(
                ctx.windows[0].draw_list.current_clip(),
                ctx.windows[0].rect(),
                "the region's clip push must be popped after its closure"
            );
        }

        #[test]
        fn the_content_inset_covers_the_border_and_corner_crescent() {
            let ctx = super::test_context();
            let style = ctx.style;
            let rect = [0.0, 0.0, 200.0, 60.0];
            let inset = crate::scroll::content_inset(&style, rect);
            let expected = style.border_size
                + style.corner_rounding * (1.0 - core::f32::consts::FRAC_1_SQRT_2);
            assert!((inset - expected).abs() < 1e-6, "{inset} vs {expected}");

            let mut crisp = style;
            crisp.corner_rounding = 0.0;
            assert_eq!(
                crate::scroll::content_inset(&crisp, rect),
                crisp.border_size
            );
            let mut bare = style;
            bare.corner_rounding = 0.0;
            bare.border_size = 0.0;
            assert_eq!(crate::scroll::content_inset(&bare, rect), 0.0);

            let tiny = [0.0, 0.0, 3.0, 3.0];
            let inset = crate::scroll::content_inset(&style, tiny);
            let expected = style.border_size + 1.5 * (1.0 - core::f32::consts::FRAC_1_SQRT_2);
            assert!((inset - expected).abs() < 1e-6, "{inset} vs {expected}");
            assert!(inset <= 1.5, "the inset never exceeds half the region");
        }

        #[test]
        fn content_emits_under_the_inset_clip_while_the_panel_keeps_the_window_clip() {
            let mut ctx = super::test_context();
            let mut probe = Probe {
                region: [0.0; 4],
                target_rect: [0.0; 4],
                target_clicked: false,
            };
            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B0", &mut probe));
            let region = probe.region;
            let clip = content_clip(&ctx, region);
            let window_rect = ctx.windows[0].rect();
            assert!(clip[0] > region[0] && clip[1] > region[1]);
            assert!(clip[2] < region[2] && clip[3] < region[3]);
            let data = ctx.windows[0].draw_list.as_draw_data();
            let mut content_cmds = 0usize;
            let mut panel_cmds = 0usize;
            for cmd in data.commands {
                if cmd.clip_rect == clip {
                    content_cmds += 1;
                }
                if cmd.clip_rect == window_rect {
                    panel_cmds += 1;
                }
            }
            assert!(content_cmds > 0, "region content rides the inset clip");
            assert!(
                panel_cmds > 0,
                "the region panel keeps the enclosing window clip"
            );
        }

        #[test]
        fn wheel_over_the_region_scrolls_the_content_and_persists() {
            let mut ctx = super::test_context();
            let mut probe = Probe {
                region: [0.0; 4],
                target_rect: [0.0; 4],
                target_clicked: false,
            };
            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B0", &mut probe));
            let region = probe.region;
            let before = probe.target_rect;
            assert_eq!(scroll_state(&ctx).0, 0.0);

            frame(&mut ctx, center(region), -1.0, |ui| {
                declare(ui, "B0", &mut probe)
            });
            let line = ctx.font.line_height();
            let (offset, _) = scroll_state(&ctx);
            let expected = super::super::WHEEL_LINES_PER_NOTCH * line;
            assert!(
                (offset - expected).abs() < 0.001,
                "offset {offset} should be one notch = {expected}"
            );
            frame(&mut ctx, center(region), 0.0, |ui| {
                declare(ui, "B0", &mut probe)
            });
            let shifted = probe.target_rect;
            assert!(
                (before[1] - shifted[1] - offset).abs() < 0.01,
                "content must shift up by the offset: before {before:?}, after {shifted:?}"
            );
            assert_eq!(before[0], shifted[0], "scrolling is vertical only");

            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B0", &mut probe));
            assert_eq!(scroll_state(&ctx).0, offset);
            assert_eq!(probe.target_rect, shifted);
            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B0", &mut probe));
            assert_eq!(scroll_state(&ctx).0, offset);
            assert_eq!(probe.target_rect, shifted);
        }

        #[test]
        fn wheel_outside_the_region_does_not_scroll() {
            let mut ctx = super::test_context();
            let mut probe = Probe {
                region: [0.0; 4],
                target_rect: [0.0; 4],
                target_clicked: false,
            };
            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B0", &mut probe));
            let region = probe.region;
            let before = probe.target_rect;

            let outside = [center(region)[0], region[3] + 30.0];
            frame(&mut ctx, outside, -1.0, |ui| declare(ui, "B0", &mut probe));
            assert_eq!(scroll_state(&ctx).0, 0.0);
            assert_eq!(probe.target_rect, before);
        }

        #[test]
        fn the_offset_clamps_at_both_ends() {
            let mut ctx = super::test_context();
            let mut probe = Probe {
                region: [0.0; 4],
                target_rect: [0.0; 4],
                target_clicked: false,
            };
            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B0", &mut probe));
            let region = probe.region;
            let region_h = region[3] - region[1];

            frame(&mut ctx, center(region), 1.0, |ui| {
                declare(ui, "B0", &mut probe)
            });
            assert_eq!(scroll_state(&ctx).0, 0.0);

            frame(&mut ctx, center(region), -100.0, |ui| {
                declare(ui, "B0", &mut probe)
            });
            let (offset, content_height) = scroll_state(&ctx);
            let visible_h = {
                let clip = content_clip(&ctx, region);
                clip[3] - clip[1]
            };
            assert!(
                content_height > region_h,
                "content must overflow: {content_height}"
            );
            assert!(offset > 0.0);
            assert_eq!(
                offset,
                (content_height - visible_h).max(0.0),
                "the offset must clamp to the bottom of the content"
            );

            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B0", &mut probe));
            assert_eq!(scroll_state(&ctx).0, offset);
        }

        #[test]
        fn a_button_below_the_fold_neither_hovers_nor_fires_until_scrolled_into_view() {
            let mut ctx = super::test_context();
            let mut probe = Probe {
                region: [0.0; 4],
                target_rect: [0.0; 4],
                target_clicked: false,
            };
            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B9", &mut probe));
            let region = probe.region;
            let hidden_rect = probe.target_rect;
            let b9 = button_id_in_region("B9");
            assert!(
                hidden_rect[1] > region[3],
                "B9 must start below the fold: {hidden_rect:?} vs {region:?}"
            );
            let hidden_point = center(hidden_rect);

            click_frame(&mut ctx, hidden_point, false, |ui| {
                declare(ui, "B9", &mut probe)
            });
            assert_eq!(ctx.hot, None, "a clipped-away button must not hover");
            assert!(!probe.target_clicked);
            click_frame(&mut ctx, hidden_point, true, |ui| {
                declare(ui, "B9", &mut probe)
            });
            assert_ne!(
                ctx.active,
                Some(b9),
                "a clipped-away button must not capture the press"
            );
            click_frame(&mut ctx, hidden_point, false, |ui| {
                declare(ui, "B9", &mut probe)
            });
            assert!(!probe.target_clicked, "a clipped-away button must not fire");

            frame(&mut ctx, center(region), -100.0, |ui| {
                declare(ui, "B9", &mut probe)
            });
            frame(&mut ctx, AWAY, 0.0, |ui| declare(ui, "B9", &mut probe));
            let visible_rect = probe.target_rect;
            assert!(
                visible_rect[1] >= region[1] && visible_rect[3] <= region[3],
                "B9 must be fully visible at max offset: {visible_rect:?} vs {region:?}"
            );
            let visible_point = center(visible_rect);
            click_frame(&mut ctx, visible_point, false, |ui| {
                declare(ui, "B9", &mut probe)
            });
            assert_eq!(ctx.hot, Some(b9), "the scrolled-in button must hover");
            click_frame(&mut ctx, visible_point, true, |ui| {
                declare(ui, "B9", &mut probe)
            });
            assert_eq!(ctx.active, Some(b9));
            click_frame(&mut ctx, visible_point, false, |ui| {
                declare(ui, "B9", &mut probe)
            });
            assert!(
                probe.target_clicked,
                "the scrolled-in button must fire on release"
            );
        }

        fn scroll_state_of(ctx: &Context, id: Id) -> Option<(f32, f32)> {
            match ctx.widget_states.get(&id) {
                Some(WidgetState::Scroll {
                    offset,
                    content_height,
                }) => Some((*offset, *content_height)),
                _ => None,
            }
        }

        fn labeled_region_id(label: &str) -> Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            stack.id(label)
        }

        fn nested_region_id(outer: Id, label: &str) -> Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            stack.push_id(outer);
            stack.id(label)
        }

        struct NestedProbe {
            outer: [f32; 4],
            inner: [f32; 4],
        }

        fn declare_nested(ui: &mut Ui, probe: &mut NestedProbe) {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            probe.outer = [pos[0], pos[1], pos[0] + 200.0, pos[1] + 120.0];
            ui.scroll_region("outer", [200.0, 120.0], |ui| {
                let pos = ui.ctx.current_window_mut().cursor.next_pos();
                probe.inner = [pos[0], pos[1], pos[0] + 180.0, pos[1] + 60.0];
                ui.scroll_region("inner", [180.0, 60.0], |ui| {
                    for i in 0..10 {
                        ui.button(&alloc::format!("I{i}"));
                    }
                });
                for i in 0..10 {
                    ui.button(&alloc::format!("O{i}"));
                }
            });
        }

        #[test]
        fn wheel_over_the_inner_of_nested_regions_scrolls_only_the_inner() {
            let mut ctx = super::test_context();
            let mut probe = NestedProbe {
                outer: [0.0; 4],
                inner: [0.0; 4],
            };
            frame(&mut ctx, AWAY, 0.0, |ui| declare_nested(ui, &mut probe));
            let outer_id = labeled_region_id("outer");
            let inner_id = nested_region_id(outer_id, "inner");
            let inner = probe.inner;
            assert!(
                inner[0] >= probe.outer[0]
                    && inner[1] >= probe.outer[1]
                    && inner[2] <= probe.outer[2]
                    && inner[3] <= probe.outer[3],
                "the inner region must nest inside the outer: {inner:?} vs {:?}",
                probe.outer
            );
            let outer_h = probe.outer[3] - probe.outer[1];
            let inner_h = inner[3] - inner[1];
            assert!(scroll_state_of(&ctx, outer_id).unwrap().1 > outer_h);
            assert!(scroll_state_of(&ctx, inner_id).unwrap().1 > inner_h);

            frame(&mut ctx, center(inner), -1.0, |ui| {
                declare_nested(ui, &mut probe)
            });
            let notch = super::super::WHEEL_LINES_PER_NOTCH * ctx.font.line_height();
            let inner_offset = scroll_state_of(&ctx, inner_id).unwrap().0;
            assert!(
                (inner_offset - notch).abs() < 0.001,
                "the inner region must scroll one notch: {inner_offset} != {notch}"
            );
            assert_eq!(
                scroll_state_of(&ctx, outer_id).unwrap().0,
                0.0,
                "the outer region must not co-consume the wheel"
            );
        }

        #[test]
        fn wheel_over_the_outer_only_area_of_nested_regions_scrolls_only_the_outer() {
            let mut ctx = super::test_context();
            let mut probe = NestedProbe {
                outer: [0.0; 4],
                inner: [0.0; 4],
            };
            frame(&mut ctx, AWAY, 0.0, |ui| declare_nested(ui, &mut probe));
            let outer_id = labeled_region_id("outer");
            let inner_id = nested_region_id(outer_id, "inner");

            let outer_only = [center(probe.outer)[0], probe.inner[3] + 20.0];
            assert!(super::super::point_in_rect(outer_only, probe.outer));
            assert!(!super::super::point_in_rect(outer_only, probe.inner));

            frame(&mut ctx, outer_only, -1.0, |ui| {
                declare_nested(ui, &mut probe)
            });
            let notch = super::super::WHEEL_LINES_PER_NOTCH * ctx.font.line_height();
            let outer_offset = scroll_state_of(&ctx, outer_id).unwrap().0;
            assert!(
                (outer_offset - notch).abs() < 0.001,
                "the outer region must scroll one notch: {outer_offset} != {notch}"
            );
            assert_eq!(
                scroll_state_of(&ctx, inner_id).unwrap().0,
                0.0,
                "the inner region must not scroll for a point outside it"
            );
        }

        #[test]
        fn wheel_with_no_region_under_the_mouse_changes_nothing() {
            let mut ctx = super::test_context();
            frame(&mut ctx, AWAY, 0.0, |ui| {
                ui.button("B0");
            });
            let body = center(ctx.windows[0].rect());
            frame(&mut ctx, body, -1.0, |ui| {
                ui.button("B0");
            });
            assert!(ctx.widget_states.is_empty());
        }

        const LONG: [&str; 6] = ["Red", "Green", "Blue", "Cyan", "Magenta", "Yellow"];

        struct PopupProbe {
            button: [f32; 4],
            region: [f32; 4],
        }

        fn declare_combo_over_region(ui: &mut Ui, selected: &mut usize, probe: &mut PopupProbe) {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            let style = ui.ctx.style;
            let line_height = ui.ctx.font.line_height();
            let text_w = ui.ctx.font.measure(LONG[*selected])[0];
            let h = line_height + 2.0 * style.frame_padding[1];
            let w = text_w + line_height * 0.5 + 3.0 * style.frame_padding[0];
            probe.button = [pos[0], pos[1], pos[0] + w, pos[1] + h];
            ui.combo("Color", &LONG, selected);
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            probe.region = [pos[0], pos[1], pos[0] + 200.0, pos[1] + 120.0];
            ui.scroll_region("region", [200.0, 120.0], |ui| {
                for i in 0..10 {
                    ui.button(&alloc::format!("B{i}"));
                }
            });
        }

        fn viewport_frame(
            ctx: &mut Context,
            mouse: [f32; 2],
            wheel: f32,
            left_down: bool,
            viewport: [f32; 2],
            selected: &mut usize,
            probe: &mut PopupProbe,
        ) {
            super::popup_frame(
                ctx,
                InputState {
                    mouse_pos: mouse,
                    mouse_wheel: wheel,
                    mouse_down: [left_down, false, false, false, false],
                    viewport_size: viewport,
                    ..Default::default()
                },
                |ui| declare_combo_over_region(ui, selected, probe),
            );
        }

        fn combo_list_region_id() -> Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            let popup_id = stack.id("Color");
            stack.push_id(popup_id);
            stack.id("##combo_list")
        }

        #[test]
        fn wheel_over_a_popup_region_beats_the_window_region_underneath() {
            let mut ctx = super::test_context();
            let mut selected = 0usize;
            let mut probe = PopupProbe {
                button: [0.0; 4],
                region: [0.0; 4],
            };
            viewport_frame(
                &mut ctx,
                AWAY,
                0.0,
                false,
                [0.0; 2],
                &mut selected,
                &mut probe,
            );
            let button = probe.button;
            let region = probe.region;
            let style = ctx.style;
            let row_h = ctx.font.line_height() + 2.0 * style.frame_padding[1];
            let popup_h = 2.0 * style.window_padding[1] + 2.0 * row_h + style.item_spacing[1];
            let viewport = [400.0, button[3] + popup_h];

            viewport_frame(
                &mut ctx,
                center(button),
                0.0,
                true,
                viewport,
                &mut selected,
                &mut probe,
            );
            viewport_frame(
                &mut ctx,
                center(button),
                0.0,
                false,
                viewport,
                &mut selected,
                &mut probe,
            );
            let popup = ctx.popup_rect.expect("the popup must be open");
            let pad = ctx.style.window_padding;
            let list = [
                popup[0] + pad[0],
                popup[1] + pad[1],
                popup[2] - pad[0],
                popup[3] - pad[1],
            ];
            let at = center(list);
            assert!(
                super::super::point_in_rect(at, region),
                "the popup list must sit over the window region: {at:?} vs {region:?}"
            );
            assert!(
                scroll_state_of(&ctx, region_id()).unwrap().1 > region[3] - region[1],
                "the window region must overflow"
            );
            assert!(
                scroll_state_of(&ctx, combo_list_region_id()).is_some(),
                "the clamped list must declare a scroll region"
            );

            viewport_frame(
                &mut ctx,
                at,
                -1.0,
                false,
                viewport,
                &mut selected,
                &mut probe,
            );
            let notch = super::super::WHEEL_LINES_PER_NOTCH * ctx.font.line_height();
            let list_offset = scroll_state_of(&ctx, combo_list_region_id()).unwrap().0;
            assert!(
                (list_offset - notch).abs() < 0.001,
                "the popup list must scroll one notch: {list_offset} != {notch}"
            );
            assert_eq!(
                scroll_state_of(&ctx, region_id()).unwrap().0,
                0.0,
                "the window region under the popup must not scroll"
            );
        }
    }

    mod child {
        use crate::context::{Context, WidgetState};
        use crate::id::Id;
        use crate::input::InputState;
        use crate::scroll::ScrollFlags;
        use crate::ui::Ui;
        use crate::widgets::test_support::next_text_rect;
        use alloc::vec::Vec;

        const AWAY: [f32; 2] = [-50.0, -50.0];
        const SIZE: [f32; 2] = [200.0, 60.0];

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn frame(ctx: &mut Context, mouse: [f32; 2], wheel: f32, f: impl FnOnce(&mut Ui)) {
            super::popup_frame(
                ctx,
                InputState {
                    mouse_pos: mouse,
                    mouse_wheel: wheel,
                    ..Default::default()
                },
                f,
            );
        }

        fn click_frame(
            ctx: &mut Context,
            mouse: [f32; 2],
            left_down: bool,
            f: impl FnOnce(&mut Ui),
        ) {
            super::popup_frame(
                ctx,
                InputState {
                    mouse_pos: mouse,
                    mouse_down: [left_down, false, false, false, false],
                    ..Default::default()
                },
                f,
            );
        }

        fn child_id(label: &str) -> Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            stack.id(label)
        }

        fn nested_id(parent: Id, label: &str) -> Id {
            let mut stack = crate::id::IdStack::new();
            stack.push("W");
            stack.push_id(parent);
            stack.id(label)
        }

        fn scroll_state_of(ctx: &Context, id: Id) -> Option<(f32, f32)> {
            match ctx.widget_states.get(&id) {
                Some(WidgetState::Scroll {
                    offset,
                    content_height,
                }) => Some((*offset, *content_height)),
                _ => None,
            }
        }

        fn declare_fitting(ui: &mut Ui, flags: ScrollFlags) {
            ui.window("W", ScrollFlags::NONE, |ui| {
                ui.child("c", SIZE, flags, |ui| {
                    ui.label("one line, fits easily");
                });
            });
        }

        #[test]
        fn none_and_wheel_are_byte_identical_when_content_fits() {
            let mut none_ctx = super::test_context();
            let none = super::scripted_frame(&mut none_ctx, AWAY, false, 0.0, |ui| {
                declare_fitting(ui, ScrollFlags::NONE)
            });
            let mut wheel_ctx = super::test_context();
            let wheel = super::scripted_frame(&mut wheel_ctx, AWAY, false, 0.0, |ui| {
                declare_fitting(ui, ScrollFlags::WHEEL)
            });
            assert_eq!(
                none, wheel,
                "content that fits must render identically regardless of scrollability"
            );
        }

        fn declare_child_wheel(ui: &mut Ui) {
            ui.window("W", ScrollFlags::NONE, |ui| {
                ui.child("Log", [200.0, 96.0], ScrollFlags::WHEEL, |ui| {
                    for _ in 0..20 {
                        ui.label("log line");
                    }
                });
            });
        }

        fn declare_scroll_region_sugar(ui: &mut Ui) {
            ui.window("W", ScrollFlags::NONE, |ui| {
                ui.scroll_region("Log", [200.0, 96.0], |ui| {
                    for _ in 0..20 {
                        ui.label("log line");
                    }
                });
            });
        }

        fn declare_child_pair(ui: &mut Ui) {
            assert!(ui.begin_window("W", ScrollFlags::NONE));
            let scope = ui.begin_child("Log", [200.0, 96.0], ScrollFlags::WHEEL);
            for _ in 0..20 {
                ui.label("log line");
            }
            ui.end_child(scope);
            ui.end_window();
        }

        fn wheel_script(
            declare: impl Fn(&mut Ui),
        ) -> Vec<(
            Vec<crate::draw::DrawVert>,
            Vec<u32>,
            Vec<crate::draw::DrawCmd>,
        )> {
            let mut ctx = super::test_context();
            let mut frames = Vec::new();
            frames.push(super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui)
            }));
            let origin = super::content_origin(&ctx);
            let over = [origin[0] + 10.0, origin[1] + 10.0];
            for _ in 0..3 {
                frames.push(super::scripted_frame(&mut ctx, over, false, -1.0, |ui| {
                    declare(ui)
                }));
            }
            frames
        }

        #[test]
        fn child_wheel_matches_scroll_region_sugar_on_overflowing_content() {
            let child_frames = wheel_script(declare_child_wheel);
            assert_ne!(
                child_frames[0], child_frames[2],
                "sanity: the wheel must have actually scrolled the content"
            );
            assert_eq!(
                child_frames,
                wheel_script(declare_scroll_region_sugar),
                "scroll_region must remain byte-identical sugar for child(.., ScrollFlags::WHEEL, ..)"
            );
        }

        #[test]
        fn child_closure_matches_begin_end_pair() {
            let closure = wheel_script(declare_child_wheel);
            assert_ne!(closure[0], closure[2]);
            assert_eq!(closure, wheel_script(declare_child_pair));
        }

        struct Probe {
            child: [f32; 4],
            target_rect: [f32; 4],
            target_clicked: bool,
        }

        fn declare_none_with_buttons(ui: &mut Ui, target: &str, probe: &mut Probe) {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            probe.child = [pos[0], pos[1], pos[0] + SIZE[0], pos[1] + SIZE[1]];
            ui.child("c", SIZE, ScrollFlags::NONE, |ui| {
                for i in 0..10 {
                    let label = alloc::format!("B{i}");
                    if label == target {
                        probe.target_rect = next_text_rect(ui, &label);
                        probe.target_clicked = ui.button(&label);
                    } else {
                        ui.button(&label);
                    }
                }
            });
        }

        #[test]
        fn none_offset_stays_zero_across_wheel_input_and_hides_content_below_the_fold() {
            let mut ctx = super::test_context();
            let id = child_id("c");
            let mut probe = Probe {
                child: [0.0; 4],
                target_rect: [0.0; 4],
                target_clicked: false,
            };
            frame(&mut ctx, AWAY, 0.0, |ui| {
                declare_none_with_buttons(ui, "B9", &mut probe)
            });
            let child_rect = probe.child;
            let hidden_rect = probe.target_rect;
            assert!(
                hidden_rect[1] > child_rect[3],
                "B9 must start below the fold: {hidden_rect:?} vs {child_rect:?}"
            );
            assert_eq!(scroll_state_of(&ctx, id).unwrap().0, 0.0);

            for _ in 0..3 {
                frame(&mut ctx, center(child_rect), -1.0, |ui| {
                    declare_none_with_buttons(ui, "B9", &mut probe)
                });
                assert_eq!(
                    scroll_state_of(&ctx, id).unwrap().0,
                    0.0,
                    "a NONE child must never scroll"
                );
                assert_eq!(probe.target_rect, hidden_rect, "layout must never shift");
            }

            let hidden_point = center(hidden_rect);
            let b9 = nested_id(id, "B9");
            click_frame(&mut ctx, hidden_point, false, |ui| {
                declare_none_with_buttons(ui, "B9", &mut probe)
            });
            assert_eq!(ctx.hot, None, "a clipped-away button must not hover");
            assert!(!probe.target_clicked);
            click_frame(&mut ctx, hidden_point, true, |ui| {
                declare_none_with_buttons(ui, "B9", &mut probe)
            });
            assert_ne!(
                ctx.active,
                Some(b9),
                "a clipped-away button must not capture the press"
            );
            click_frame(&mut ctx, hidden_point, false, |ui| {
                declare_none_with_buttons(ui, "B9", &mut probe)
            });
            assert!(!probe.target_clicked, "a clipped-away button must not fire");
        }

        struct NestedProbe {
            outer: [f32; 4],
            inner: [f32; 4],
        }

        fn declare_nested(ui: &mut Ui, inner_flags: ScrollFlags, probe: &mut NestedProbe) {
            let pos = ui.ctx.current_window_mut().cursor.next_pos();
            probe.outer = [pos[0], pos[1], pos[0] + 200.0, pos[1] + 120.0];
            ui.child("outer", [200.0, 120.0], ScrollFlags::WHEEL, |ui| {
                let pos = ui.ctx.current_window_mut().cursor.next_pos();
                probe.inner = [pos[0], pos[1], pos[0] + 180.0, pos[1] + 60.0];
                ui.child("inner", [180.0, 60.0], inner_flags, |ui| {
                    for i in 0..10 {
                        ui.button(&alloc::format!("I{i}"));
                    }
                });
                for i in 0..10 {
                    ui.button(&alloc::format!("O{i}"));
                }
            });
        }

        #[test]
        fn wheel_over_a_non_scrollable_inner_child_falls_through_to_the_scrollable_outer() {
            let mut ctx = super::test_context();
            let mut probe = NestedProbe {
                outer: [0.0; 4],
                inner: [0.0; 4],
            };
            frame(&mut ctx, AWAY, 0.0, |ui| {
                declare_nested(ui, ScrollFlags::NONE, &mut probe)
            });
            let outer_id = child_id("outer");
            let inner_id = nested_id(outer_id, "inner");

            let outer_h = probe.outer[3] - probe.outer[1];
            assert!(scroll_state_of(&ctx, outer_id).unwrap().1 > outer_h);

            frame(&mut ctx, center(probe.inner), -1.0, |ui| {
                declare_nested(ui, ScrollFlags::NONE, &mut probe)
            });
            let notch = super::super::WHEEL_LINES_PER_NOTCH * ctx.font.line_height();
            let outer_offset = scroll_state_of(&ctx, outer_id).unwrap().0;
            assert!(
                (outer_offset - notch).abs() < 0.001,
                "the wheel over a non-scrollable inner child must fall through to the outer: {outer_offset} != {notch}"
            );
            assert_eq!(
                scroll_state_of(&ctx, inner_id).unwrap().0,
                0.0,
                "a non-scrollable child never scrolls"
            );
        }

        #[test]
        fn wheel_over_a_scrollable_inner_child_scrolls_only_the_inner() {
            let mut ctx = super::test_context();
            let mut probe = NestedProbe {
                outer: [0.0; 4],
                inner: [0.0; 4],
            };
            frame(&mut ctx, AWAY, 0.0, |ui| {
                declare_nested(ui, ScrollFlags::WHEEL, &mut probe)
            });
            let outer_id = child_id("outer");
            let inner_id = nested_id(outer_id, "inner");

            let outer_h = probe.outer[3] - probe.outer[1];
            let inner_h = probe.inner[3] - probe.inner[1];
            assert!(scroll_state_of(&ctx, outer_id).unwrap().1 > outer_h);
            assert!(scroll_state_of(&ctx, inner_id).unwrap().1 > inner_h);

            frame(&mut ctx, center(probe.inner), -1.0, |ui| {
                declare_nested(ui, ScrollFlags::WHEEL, &mut probe)
            });
            let notch = super::super::WHEEL_LINES_PER_NOTCH * ctx.font.line_height();
            let inner_offset = scroll_state_of(&ctx, inner_id).unwrap().0;
            assert!(
                (inner_offset - notch).abs() < 0.001,
                "the inner child must scroll one notch: {inner_offset} != {notch}"
            );
            assert_eq!(
                scroll_state_of(&ctx, outer_id).unwrap().0,
                0.0,
                "the outer must not co-consume the wheel when the inner is scrollable (innermost-wins)"
            );
        }

        fn declare_flippable(ui: &mut Ui, flags: ScrollFlags) {
            ui.child("c", SIZE, flags, |ui| {
                for i in 0..10 {
                    ui.button(&alloc::format!("B{i}"));
                }
            });
        }

        #[test]
        fn a_runtime_flip_from_none_to_wheel_scrolls_on_the_first_wheel_frame() {
            let mut ctx = super::test_context();
            let id = child_id("c");

            frame(&mut ctx, AWAY, 0.0, |ui| {
                declare_flippable(ui, ScrollFlags::NONE)
            });
            frame(&mut ctx, AWAY, -1.0, |ui| {
                declare_flippable(ui, ScrollFlags::NONE)
            });
            let (offset, content_height) = scroll_state_of(&ctx, id).unwrap();
            assert_eq!(offset, 0.0, "NONE must never scroll, even with wheel input");
            assert!(
                content_height > SIZE[1],
                "content must already overflow the child by the time it flips"
            );

            let origin = super::content_origin(&ctx);
            let child_rect = [
                origin[0],
                origin[1],
                origin[0] + SIZE[0],
                origin[1] + SIZE[1],
            ];
            frame(&mut ctx, center(child_rect), -1.0, |ui| {
                declare_flippable(ui, ScrollFlags::WHEEL)
            });
            let notch = super::super::WHEEL_LINES_PER_NOTCH * ctx.font.line_height();
            let (offset_after, _) = scroll_state_of(&ctx, id).unwrap();
            assert!(
                (offset_after - notch).abs() < 0.001,
                "the first WHEEL frame's wheel input must scroll: {offset_after} != {notch}"
            );
        }

        fn declare_parkable(ui: &mut Ui, flags: ScrollFlags, first_row_y: &mut f32) {
            ui.child("c", SIZE, flags, |ui| {
                *first_row_y = ui.ctx.current_window_mut().cursor.next_pos()[1];
                for i in 0..10 {
                    ui.button(&alloc::format!("B{i}"));
                }
            });
        }

        #[test]
        fn a_none_frame_parks_the_childs_offset_at_the_top_without_destroying_it() {
            let mut ctx = super::test_context();
            let id = child_id("c");
            let mut first_row_y = 0.0;

            frame(&mut ctx, AWAY, 0.0, |ui| {
                declare_parkable(ui, ScrollFlags::WHEEL, &mut first_row_y)
            });
            let top_row_y = first_row_y;
            let origin = super::content_origin(&ctx);
            let child_rect = [
                origin[0],
                origin[1],
                origin[0] + SIZE[0],
                origin[1] + SIZE[1],
            ];
            frame(&mut ctx, center(child_rect), -1.0, |ui| {
                declare_parkable(ui, ScrollFlags::WHEEL, &mut first_row_y)
            });
            frame(&mut ctx, AWAY, 0.0, |ui| {
                declare_parkable(ui, ScrollFlags::WHEEL, &mut first_row_y)
            });
            let scrolled = scroll_state_of(&ctx, id).unwrap().0;
            assert!(scrolled > 0.0, "test premise: the child is scrolled");
            assert!(
                first_row_y < top_row_y,
                "test premise: the content is laid out scrolled"
            );

            frame(&mut ctx, AWAY, 0.0, |ui| {
                declare_parkable(ui, ScrollFlags::NONE, &mut first_row_y)
            });
            assert_eq!(
                first_row_y, top_row_y,
                "a NONE child lays its content out top-aligned whatever the stored offset"
            );
            let (parked, content_height) = scroll_state_of(&ctx, id).unwrap();
            assert_eq!(
                parked, scrolled,
                "a NONE frame must not overwrite the child's persisted offset"
            );
            assert!(
                content_height > SIZE[1],
                "a NONE frame must still measure and persist the content height"
            );

            frame(&mut ctx, AWAY, 0.0, |ui| {
                declare_parkable(ui, ScrollFlags::WHEEL, &mut first_row_y)
            });
            assert_eq!(
                scroll_state_of(&ctx, id).unwrap().0,
                scrolled,
                "flipping back to WHEEL must resume at the original offset"
            );
            assert!(
                first_row_y < top_row_y,
                "and lay the content back out scrolled"
            );
        }
    }

    mod window_scroll {
        use super::test_context;
        use crate::context::{Context, WidgetState};
        use crate::id::{Id, IdStack};
        use crate::input::InputState;
        use crate::scroll::ScrollFlags;
        use crate::ui::Ui;
        use alloc::vec::Vec;

        const AWAY: [f32; 2] = [-1000.0, -1000.0];
        const OVERFLOW_ROWS: usize = 60;
        const CHILD_SIZE: [f32; 2] = [200.0, 60.0];
        const LONG_ITEMS: [&str; 6] = ["red", "green", "blue", "cyan", "magenta", "yellow"];

        fn rows(ui: &mut Ui, n: usize) {
            for _ in 0..n {
                ui.label("row");
            }
        }

        fn declare(ui: &mut Ui, flags: ScrollFlags, n: usize) {
            ui.window("W", flags, |ui| rows(ui, n));
        }

        fn window_index(ctx: &Context, title: &str) -> usize {
            ctx.windows
                .iter()
                .position(|w| w.title == title)
                .expect("the window must be registered")
        }

        fn offset_of(ctx: &Context, title: &str) -> f32 {
            ctx.windows[window_index(ctx, title)].scroll_offset
        }

        fn content_height_of(ctx: &Context, title: &str) -> f32 {
            ctx.windows[window_index(ctx, title)].content_height
        }

        fn window_rect(ctx: &Context, title: &str) -> [f32; 4] {
            ctx.windows[window_index(ctx, title)].rect()
        }

        fn body_rect(ctx: &Context, title: &str) -> [f32; 4] {
            let r = window_rect(ctx, title);
            [r[0], r[1] + super::title_bar_height(ctx), r[2], r[3]]
        }

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn child_id(label: &str) -> Id {
            let mut stack = IdStack::new();
            stack.push("W");
            stack.id(label)
        }

        fn child_scroll_state(ctx: &Context, id: Id) -> Option<(f32, f32)> {
            match ctx.widget_states.get(&id) {
                Some(WidgetState::Scroll {
                    offset,
                    content_height,
                }) => Some((*offset, *content_height)),
                _ => None,
            }
        }

        #[test]
        fn window_none_draw_data_pin_matches_pre_scroll_snapshot() {
            fn declare_pin_fixture(ui: &mut Ui) {
                assert!(ui.begin_window("W", ScrollFlags::NONE));
                ui.label("x");
                let _ = ui.button("go");
                ui.end_window();
            }

            let mut ctx = test_context();
            let _settle = super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare_pin_fixture);
            let (verts, indices, cmds) =
                super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare_pin_fixture);

            assert_eq!(verts.len(), 175, "vertex count pin");
            assert_eq!(indices.len(), 261, "index count pin");
            assert_eq!(cmds.len(), 12, "command count pin");

            use crate::draw::{DrawCmd, DrawVert, TextureId};
            let expected_verts: Vec<DrawVert> = std::vec![
                DrawVert {
                    pos: [f32::from_bits(0x42700000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42700000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42700000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E60000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42700000), f32::from_bits(0x43E60000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B40000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B40000), f32::from_bits(0x43E60000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E60000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E60000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E60000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42700000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42700000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B40000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x43E40000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [16, 16, 20, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42700000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42700000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC0000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC0000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF40000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF40000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42700000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B20000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42800000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42700000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42800000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B40000), f32::from_bits(0x42AD4000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42700000), f32::from_bits(0x42AD4000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [27, 27, 34, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x4285B400)],
                    uv: [f32::from_bits(0x3DC00000), f32::from_bits(0x3E800000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42AC0000), f32::from_bits(0x4285B400)],
                    uv: [f32::from_bits(0x3E200000), f32::from_bits(0x3E800000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42AC0000), f32::from_bits(0x429DB400)],
                    uv: [f32::from_bits(0x3E200000), f32::from_bits(0x3E980000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x429DB400)],
                    uv: [f32::from_bits(0x3DC00000), f32::from_bits(0x3E980000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A902BF), f32::from_bits(0x4282B505)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B152BF), f32::from_bits(0x42A3F505)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B1AD41), f32::from_bits(0x42A28AFB)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A95D41), f32::from_bits(0x42814AFB)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A902BF), f32::from_bits(0x4282B505)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B152BF), f32::from_bits(0x42A3F505)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B0D5D8), f32::from_bits(0x42A5E8A2)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A885D8), f32::from_bits(0x4284A8A2)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B1AD41), f32::from_bits(0x42A28AFB)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A95D41), f32::from_bits(0x42814AFB)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A9DA28), f32::from_bits(0x427EAEBC)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B22A28), f32::from_bits(0x42A0975E)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A95D41), f32::from_bits(0x42A3F505)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B1AD41), f32::from_bits(0x4282B505)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B152BF), f32::from_bits(0x42814AFB)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A902BF), f32::from_bits(0x42A28AFB)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A95D41), f32::from_bits(0x42A3F505)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B1AD41), f32::from_bits(0x4282B505)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B22A28), f32::from_bits(0x4284A8A2)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A9DA28), f32::from_bits(0x42A5E8A2)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B152BF), f32::from_bits(0x42814AFB)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A902BF), f32::from_bits(0x42A28AFB)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43A885D8), f32::from_bits(0x42A0975E)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B0D5D8), f32::from_bits(0x427EAEBC)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42CCF400)],
                    uv: [f32::from_bits(0x3EAE0000), f32::from_bits(0x3EC60000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x429E0000), f32::from_bits(0x42CCF400)],
                    uv: [f32::from_bits(0x3EC00000), f32::from_bits(0x3EC60000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x429E0000), f32::from_bits(0x42DEF400)],
                    uv: [f32::from_bits(0x3EC00000), f32::from_bits(0x3ED80000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42DEF400)],
                    uv: [f32::from_bits(0x3EAE0000), f32::from_bits(0x3ED80000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC8000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC8000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF48000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF48000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x3DD07000), f32::from_bits(0x3EEC8000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x3DB07000), f32::from_bits(0x3EEC8000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x3DB07000), f32::from_bits(0x3EF48000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x3DD07000), f32::from_bits(0x3EF48000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EF44000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EF44000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x3DD00000), f32::from_bits(0x3EEC4000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x3DB00000), f32::from_bits(0x3EEC4000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x3DD07000), f32::from_bits(0x3EF44000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x3DB07000), f32::from_bits(0x3EF44000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x3DB07000), f32::from_bits(0x3EEC4000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x3DD07000), f32::from_bits(0x3EEC4000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [41, 74, 122, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EEC8000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EEC8000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EF48000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EF48000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x3E043800), f32::from_bits(0x3EEC8000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x3DE87000), f32::from_bits(0x3EEC8000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x3DE87000), f32::from_bits(0x3EF48000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x3E043800), f32::from_bits(0x3EF48000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EF44000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EF44000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x3E040000), f32::from_bits(0x3EEC4000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x3DE80000), f32::from_bits(0x3EEC4000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x3E043800), f32::from_bits(0x3EF44000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x3DE87000), f32::from_bits(0x3EF44000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x3DE87000), f32::from_bits(0x3EEC4000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x3E043800), f32::from_bits(0x3EEC4000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x42EE8000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x42F08000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x42F08000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x4310E000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x4310E000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C3E400), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42940000), f32::from_bits(0x4311E000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428E0000), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428E0000), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x428C0000), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C9E400), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x42F68000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42CBE400), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42C9E400), f32::from_bits(0x430DE000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [81, 114, 162, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42980000), f32::from_bits(0x43011A00)],
                    uv: [f32::from_bits(0x3E640000), f32::from_bits(0x3EA20000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42AA0000), f32::from_bits(0x43011A00)],
                    uv: [f32::from_bits(0x3E840000), f32::from_bits(0x3EA20000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42AA0000), f32::from_bits(0x430E1A00)],
                    uv: [f32::from_bits(0x3E840000), f32::from_bits(0x3EBC0000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42980000), f32::from_bits(0x430E1A00)],
                    uv: [f32::from_bits(0x3E640000), f32::from_bits(0x3EBC0000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42AC5000), f32::from_bits(0x43011A00)],
                    uv: [f32::from_bits(0x3B800000), f32::from_bits(0x3EC60000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42BE5000), f32::from_bits(0x43011A00)],
                    uv: [f32::from_bits(0x3D200000), f32::from_bits(0x3EC60000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42BE5000), f32::from_bits(0x430B1A00)],
                    uv: [f32::from_bits(0x3D200000), f32::from_bits(0x3EDA0000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x42AC5000), f32::from_bits(0x430B1A00)],
                    uv: [f32::from_bits(0x3B800000), f32::from_bits(0x3EDA0000)],
                    col: [240, 240, 240, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43DE0000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43E56A0A)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43AC0000), f32::from_bits(0x43E56A0A)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43E56A0A)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43DE0000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B3ADA3), f32::from_bits(0x43DD5CCE)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B3E6F1), f32::from_bits(0x43E5E6F1)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43AC0000), f32::from_bits(0x43E56A0A)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43E56A0A)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B3E6F1), f32::from_bits(0x43E5E6F1)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43AB5CCE), f32::from_bits(0x43E5ADA3)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B36A0A), f32::from_bits(0x43DE0000)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43AC0000), f32::from_bits(0x43E56A0A)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 255]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43AB5CCE), f32::from_bits(0x43E5ADA3)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
                DrawVert {
                    pos: [f32::from_bits(0x43B3ADA3), f32::from_bits(0x43DD5CCE)],
                    uv: [f32::from_bits(0x00000000), f32::from_bits(0x00000000)],
                    col: [224, 224, 224, 0]
                },
            ];
            let expected_indices: Vec<u32> = std::vec![
                0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15,
                16, 17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23, 24, 25, 26, 24, 26, 27, 28, 29, 30,
                28, 30, 31, 32, 33, 34, 32, 34, 35, 36, 37, 38, 36, 38, 39, 40, 41, 42, 40, 42, 43,
                44, 45, 46, 44, 46, 47, 48, 49, 50, 48, 50, 51, 52, 53, 54, 52, 54, 55, 56, 57, 58,
                56, 58, 59, 60, 61, 62, 60, 62, 63, 64, 65, 66, 64, 66, 67, 68, 69, 70, 68, 70, 71,
                72, 73, 74, 72, 74, 75, 76, 77, 78, 76, 78, 79, 80, 81, 82, 80, 82, 83, 84, 85, 86,
                84, 86, 87, 88, 89, 90, 88, 90, 91, 92, 93, 94, 92, 94, 95, 96, 97, 98, 96, 98, 99,
                100, 101, 102, 100, 102, 103, 104, 105, 106, 104, 106, 107, 108, 109, 110, 108,
                110, 111, 112, 113, 114, 112, 114, 115, 116, 117, 118, 116, 118, 119, 120, 121,
                122, 120, 122, 123, 124, 125, 126, 124, 126, 127, 128, 129, 130, 128, 130, 131,
                132, 133, 134, 132, 134, 135, 136, 137, 138, 136, 138, 139, 140, 141, 142, 140,
                142, 143, 144, 145, 146, 144, 146, 147, 148, 149, 150, 148, 150, 151, 152, 153,
                154, 152, 154, 155, 156, 157, 158, 156, 158, 159, 160, 161, 162, 163, 164, 165,
                163, 165, 166, 167, 168, 169, 167, 169, 170, 171, 172, 173, 171, 173, 174,
            ];
            let expected_cmds: Vec<DrawCmd> = std::vec![
                DrawCmd {
                    effects: 0,
                    elem_count: 24,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(0)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 30,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(1)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 12,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(0)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 12,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(1)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 6,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(0)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 36,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(1)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 30,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(0)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 30,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(1)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 24,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(0)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 24,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(1)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 12,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(0)
                },
                DrawCmd {
                    effects: 0,
                    elem_count: 21,
                    clip_rect: [
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x42700000),
                        f32::from_bits(0x43B40000),
                        f32::from_bits(0x43E60000)
                    ],
                    texture: TextureId(1)
                },
            ];
            assert_eq!(verts, expected_verts, "vertex data pin");
            assert_eq!(indices, expected_indices, "index data pin");
            assert_eq!(cmds, expected_cmds, "command data pin");
        }

        #[test]
        fn a_none_window_registers_nothing_and_does_not_move_under_the_wheel() {
            let mut ctx = test_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::NONE, OVERFLOW_ROWS)
            });
            let body = body_rect(&ctx, "W");
            let over = center(body);
            let mut frames = Vec::new();
            for _ in 0..3 {
                frames.push(super::scripted_frame(&mut ctx, over, false, -1.0, |ui| {
                    declare(ui, ScrollFlags::NONE, OVERFLOW_ROWS)
                }));
            }
            assert_eq!(offset_of(&ctx, "W"), 0.0, "a NONE window never scrolls");
            assert_eq!(
                frames[0], frames[2],
                "a NONE window's emission must not move under the wheel"
            );
            assert!(
                ctx.scroll_containers.is_empty(),
                "a NONE window must register no wheel-routing entry"
            );
            assert!(
                ctx.interaction_clip.is_none(),
                "a NONE window must leave the interaction clip untouched"
            );
        }

        #[test]
        fn wheel_over_the_body_scrolls_it_while_the_title_bar_stays_put() {
            let mut ctx = test_context();
            let settled = super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            let body = body_rect(&ctx, "W");
            assert_eq!(
                offset_of(&ctx, "W"),
                0.0,
                "the first declaration cannot scroll: the content height is unknown"
            );
            assert!(
                content_height_of(&ctx, "W") > body[3] - body[1],
                "test premise: the content must overflow the body"
            );

            let over = center(body);
            super::scripted_frame(&mut ctx, over, false, -1.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert!(
                offset_of(&ctx, "W") > 0.0,
                "the wheel must scroll the window body"
            );
            let scrolled = super::scripted_frame(&mut ctx, over, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });

            let content_col = ctx.style.text;
            let band = |frame: &(
                Vec<crate::draw::DrawVert>,
                Vec<u32>,
                Vec<crate::draw::DrawCmd>,
            )| {
                frame
                    .0
                    .iter()
                    .filter(|v| v.pos[1] <= body[1] && v.col != content_col)
                    .map(|v| v.pos)
                    .collect::<Vec<[f32; 2]>>()
            };
            assert!(
                !band(&settled).is_empty(),
                "test premise: the title bar emits"
            );
            assert_eq!(
                band(&settled),
                band(&scrolled),
                "the title bar, its text and the close glyph must not move when the body scrolls"
            );
        }

        #[test]
        fn a_scrollable_windows_content_carries_the_body_clip_not_the_window_clip() {
            let mut ctx = test_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            let body = body_rect(&ctx, "W");
            let over = center(body);
            super::scripted_frame(&mut ctx, over, false, -2.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert!(
                offset_of(&ctx, "W") > 0.0,
                "test premise: the body scrolled"
            );
            super::scripted_frame(&mut ctx, over, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });

            let rect = window_rect(&ctx, "W");
            let index = window_index(&ctx, "W");
            let data = ctx.windows[index].draw_list.as_draw_data();
            let mut saw_body_clip = false;
            for cmd in data.commands {
                if cmd.clip_rect == body {
                    saw_body_clip = true;
                } else {
                    assert_eq!(
                        cmd.clip_rect, rect,
                        "a scrollable window's commands carry either the body clip \
                         (content) or the full window rect (chrome and grip)"
                    );
                }
            }
            assert!(
                saw_body_clip,
                "the scrollable window must clip its content to the body rect"
            );

            assert!(
                data.vertices
                    .iter()
                    .any(|v| v.pos[1] < body[1] - 1.0 && v.col == ctx.style.text),
                "scrolled-away rows must still emit, above the body, under the body clip"
            );
        }

        #[test]
        fn a_widget_scrolled_above_the_body_is_not_hoverable() {
            fn declare_with_button(ui: &mut Ui) {
                ui.window("W", ScrollFlags::WHEEL, |ui| {
                    let _ = ui.button("first");
                    rows(ui, OVERFLOW_ROWS);
                });
            }
            let mut ctx = test_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare_with_button);
            let body = body_rect(&ctx, "W");
            let pad = ctx.style.window_padding;
            let row_h = ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1];

            let unscrolled_probe = [body[0] + pad[0] + 4.0, body[1] + pad[1] + row_h / 2.0];
            super::scripted_frame(&mut ctx, unscrolled_probe, false, 0.0, declare_with_button);
            assert!(
                ctx.hot.is_some(),
                "test premise: the first row's button hovers while it is visible"
            );

            let over = center(body);
            super::scripted_frame(&mut ctx, over, false, -3.0, declare_with_button);
            let offset = offset_of(&ctx, "W");
            assert!(
                offset > pad[1] + row_h,
                "test premise: the button must have scrolled out"
            );
            let probe = [
                body[0] + pad[0] + 4.0,
                body[1] - offset + pad[1] + row_h / 2.0,
            ];
            assert!(
                probe[1] < body[1],
                "test premise: the probe sits above the body"
            );
            super::scripted_frame(&mut ctx, probe, false, 0.0, declare_with_button);
            assert!(
                ctx.hot.is_none(),
                "a widget scrolled above the body must not hover"
            );
        }

        #[test]
        fn a_title_bar_drag_moves_the_body_clip_with_the_window() {
            const DRAG: f32 = 40.0;
            let declare_w = |ui: &mut Ui| declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS);

            let mut ctx = test_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare_w);
            let body = body_rect(&ctx, "W");
            super::scripted_frame(&mut ctx, center(body), false, -2.0, declare_w);
            assert!(
                offset_of(&ctx, "W") > 0.0,
                "test premise: the body scrolled"
            );

            let before = window_rect(&ctx, "W");
            let bar_h = super::title_bar_height(&ctx);
            let grab = [before[0] + 20.0, before[1] + bar_h / 2.0];
            super::scripted_frame(&mut ctx, grab, true, 0.0, declare_w);
            super::scripted_frame(&mut ctx, [grab[0], grab[1] + DRAG], true, 0.0, declare_w);

            let moved = window_rect(&ctx, "W");
            assert!(
                (moved[1] - before[1] - DRAG).abs() < 1e-3 && moved[0] == before[0],
                "test premise: the drag must move the window {DRAG} px down, \
                 {before:?} -> {moved:?}"
            );
            assert!(
                offset_of(&ctx, "W") > 0.0,
                "test premise: the window is still scrolled after the drag"
            );

            let moved_body = [moved[0], moved[1] + bar_h, moved[2], moved[3]];
            let stale_body = [before[0], before[1] + bar_h, before[2], before[3]];
            assert_ne!(moved_body, stale_body, "test premise: the body rect moved");

            let index = window_index(&ctx, "W");
            let data = ctx.windows[index].draw_list.as_draw_data();
            let clips: Vec<[f32; 4]> = data.commands.iter().map(|cmd| cmd.clip_rect).collect();
            assert!(
                clips.contains(&moved_body),
                "the content clip must be the POST-drag body rect {moved_body:?}, \
                 got {clips:?}"
            );
            assert!(
                !clips.contains(&stale_body),
                "no command may carry the PRE-drag body rect {stale_body:?}"
            );
            for clip in &clips {
                assert!(
                    *clip == moved_body || *clip == moved,
                    "every clip is the moved body or the moved window rect, got {clip:?}"
                );
            }
        }

        #[test]
        fn the_offset_clamps_to_zero_and_to_the_content_overflow() {
            let mut ctx = test_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            let body = body_rect(&ctx, "W");
            let over = center(body);

            super::scripted_frame(&mut ctx, over, false, 5.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert_eq!(
                offset_of(&ctx, "W"),
                0.0,
                "wheeling up at the top is a no-op"
            );

            super::scripted_frame(&mut ctx, over, false, -1000.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            let content_height = content_height_of(&ctx, "W");
            let max_offset = content_height - (body[3] - body[1]);
            assert!(max_offset > 0.0, "test premise: the content overflows");
            assert!(
                (offset_of(&ctx, "W") - max_offset).abs() < 1e-3,
                "the offset must clamp at {max_offset}, got {}",
                offset_of(&ctx, "W")
            );

            super::scripted_frame(&mut ctx, over, false, -1000.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert!(
                (offset_of(&ctx, "W") - max_offset).abs() < 1e-3,
                "wheeling down at the bottom is a no-op"
            );

            super::scripted_frame(&mut ctx, over, false, 1000.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert_eq!(offset_of(&ctx, "W"), 0.0, "the offset round-trips to zero");
        }

        #[test]
        fn content_shorter_than_the_body_never_scrolls() {
            let mut ctx = test_context();
            for _ in 0..2 {
                super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                    declare(ui, ScrollFlags::WHEEL, 2)
                });
            }
            let body = body_rect(&ctx, "W");
            assert!(
                content_height_of(&ctx, "W") < body[3] - body[1],
                "test premise: two rows must fit the body"
            );
            super::scripted_frame(&mut ctx, center(body), false, -10.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, 2)
            });
            assert_eq!(
                offset_of(&ctx, "W"),
                0.0,
                "a body the content fits inside has nothing to scroll"
            );
        }

        #[test]
        fn the_first_declaration_cannot_scroll_and_the_second_can() {
            let mut ctx = test_context();
            let inside = [200.0, 300.0];
            super::scripted_frame(&mut ctx, inside, false, -1.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert_eq!(
                offset_of(&ctx, "W"),
                0.0,
                "the very first declaration has no content height to clamp against, \
                 so its max_offset is 0 and the notch cannot move it"
            );
            assert!(
                content_height_of(&ctx, "W") > 0.0,
                "that same declaration measures the content height for the next frame"
            );
            super::scripted_frame(&mut ctx, inside, false, -1.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert!(
                offset_of(&ctx, "W") > 0.0,
                "from the second declaration the same notch scrolls"
            );
        }

        #[test]
        fn the_resize_grip_marker_is_still_last_and_unclipped_when_scrolled() {
            let mut ctx = test_context();
            ctx.style.anti_aliasing = false;
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            let body = body_rect(&ctx, "W");
            let over = center(body);
            super::scripted_frame(&mut ctx, over, false, -2.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert!(
                offset_of(&ctx, "W") > 0.0,
                "test premise: the body scrolled"
            );
            super::scripted_frame(&mut ctx, over, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });

            let rect = window_rect(&ctx, "W");
            let zone = [
                rect[2] - crate::ui::RESIZE_GRIP_SIZE,
                rect[3] - crate::ui::RESIZE_GRIP_SIZE,
                rect[2],
                rect[3],
            ];
            let index = window_index(&ctx, "W");
            let data = ctx.windows[index].draw_list.as_draw_data();
            let title_text = ctx.style.title_text;
            let marker: Vec<usize> = data
                .vertices
                .iter()
                .enumerate()
                .filter(|(_, v)| {
                    v.col == title_text
                        && v.pos[0] >= zone[0]
                        && v.pos[0] <= zone[2]
                        && v.pos[1] >= zone[1]
                        && v.pos[1] <= zone[3]
                })
                .map(|(i, _)| i)
                .collect();
            assert_eq!(marker.len(), 3, "the marker must be one triangle");
            assert_eq!(
                *marker.last().unwrap(),
                data.vertices.len() - 1,
                "the marker must remain the window's LAST geometry at a non-zero offset"
            );
            let last_cmd = data.commands.last().expect("the window emits commands");
            assert_eq!(
                last_cmd.clip_rect, rect,
                "the grip must be emitted AFTER the body clip is popped: a scrolled \
                 window must not clip away its own grip"
            );
        }

        #[test]
        fn a_window_closed_while_scrolled_reopens_at_the_same_offset() {
            let mut ctx = test_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            let body = body_rect(&ctx, "W");
            super::scripted_frame(&mut ctx, center(body), false, -2.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            let scrolled = offset_of(&ctx, "W");
            assert!(scrolled > 0.0, "test premise: the window is scrolled");

            let rect = window_rect(&ctx, "W");
            let bar_h = super::title_bar_height(&ctx);
            let close = center([rect[2] - bar_h, rect[1], rect[2], rect[1] + bar_h]);
            super::scripted_frame(&mut ctx, close, true, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            super::scripted_frame(&mut ctx, close, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert!(
                !ctx.windows[window_index(&ctx, "W")].open,
                "test premise: the close button must have closed the window"
            );
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });

            ctx.set_window_open("W", true);
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert_eq!(
                offset_of(&ctx, "W"),
                scrolled,
                "the offset lives on the Window, so a close/reopen round trip keeps it"
            );
        }

        #[test]
        fn a_none_frame_parks_the_offset_at_the_top_without_destroying_it() {
            let mut ctx = test_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            let body = body_rect(&ctx, "W");
            super::scripted_frame(&mut ctx, center(body), false, -2.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            let scrolled = offset_of(&ctx, "W");
            assert!(scrolled > 0.0, "test premise: the window is scrolled");

            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::NONE, OVERFLOW_ROWS)
            });
            assert_eq!(
                offset_of(&ctx, "W"),
                scrolled,
                "a NONE frame must not overwrite the window's persisted offset"
            );
            let index = window_index(&ctx, "W");
            let text = ctx.style.text;
            assert!(
                !ctx.windows[index]
                    .draw_list
                    .as_draw_data()
                    .vertices
                    .iter()
                    .any(|v| v.col == text && v.pos[1] < body[1]),
                "a NONE window lays its body out top-aligned whatever the stored offset"
            );

            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, OVERFLOW_ROWS)
            });
            assert_eq!(
                offset_of(&ctx, "W"),
                scrolled,
                "flipping back to WHEEL must resume at the original offset"
            );
        }

        fn declare_nested(ui: &mut Ui, window_flags: ScrollFlags, child_flags: ScrollFlags) {
            ui.window("W", window_flags, |ui| {
                ui.child("c", CHILD_SIZE, child_flags, |ui| rows(ui, 30));
                rows(ui, OVERFLOW_ROWS);
            });
        }

        fn route_notch(
            window_flags: ScrollFlags,
            child_flags: ScrollFlags,
            probe: impl Fn(&Context) -> [f32; 2],
        ) -> (f32, f32) {
            let mut ctx = test_context();
            for _ in 0..2 {
                super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                    declare_nested(ui, window_flags, child_flags)
                });
            }
            let at = probe(&ctx);
            super::scripted_frame(&mut ctx, at, false, -1.0, |ui| {
                declare_nested(ui, window_flags, child_flags)
            });
            let child = child_scroll_state(&ctx, child_id("c"))
                .expect("a child persists its state whatever its flags")
                .0;
            (offset_of(&ctx, "W"), child)
        }

        fn over_the_child(ctx: &Context) -> [f32; 2] {
            let origin = super::content_origin(ctx);
            [origin[0] + 10.0, origin[1] + 10.0]
        }

        fn over_the_bare_body(ctx: &Context) -> [f32; 2] {
            let origin = super::content_origin(ctx);
            let body = body_rect(ctx, "W");
            let y = origin[1] + CHILD_SIZE[1] + 20.0;
            assert!(y < body[3], "the probe must stay inside the body");
            [origin[0] + 10.0, y]
        }

        #[test]
        fn routing_a_scrollable_child_beats_the_scrollable_window_containing_it() {
            let (window, child) =
                route_notch(ScrollFlags::WHEEL, ScrollFlags::WHEEL, over_the_child);
            assert!(
                child > 0.0,
                "the innermost REGISTERED container under the mouse takes the notch"
            );
            assert_eq!(
                window, 0.0,
                "the window must NOT co-scroll when its child claimed the wheel"
            );
        }

        #[test]
        fn routing_the_bare_body_scrolls_the_window() {
            let (window, child) =
                route_notch(ScrollFlags::WHEEL, ScrollFlags::WHEEL, over_the_bare_body);
            assert!(
                window > 0.0,
                "with no child under the mouse the window itself takes the notch"
            );
            assert_eq!(
                child, 0.0,
                "the child must not scroll from outside its rect"
            );
        }

        #[test]
        fn routing_a_non_scrollable_child_falls_through_to_the_window() {
            let (window, child) =
                route_notch(ScrollFlags::WHEEL, ScrollFlags::NONE, over_the_child);
            assert!(
                window > 0.0,
                "a NONE child registers nothing, so the notch reaches the window"
            );
            assert_eq!(child, 0.0, "a NONE child's offset stays pinned at zero");
        }

        #[test]
        fn routing_a_non_scrollable_window_with_no_scrollable_child_moves_nothing() {
            let (window, child) = route_notch(ScrollFlags::NONE, ScrollFlags::NONE, over_the_child);
            assert_eq!(window, 0.0, "a NONE window never scrolls");
            assert_eq!(child, 0.0, "a NONE child never scrolls");
        }

        #[test]
        fn routing_only_the_z_top_of_two_overlapping_scrollable_windows_scrolls() {
            fn declare_two(ui: &mut Ui) {
                ui.window("A", ScrollFlags::WHEEL, |ui| rows(ui, OVERFLOW_ROWS));
                ui.window("B", ScrollFlags::WHEEL, |ui| rows(ui, OVERFLOW_ROWS));
            }
            let mut ctx = test_context();
            for _ in 0..2 {
                super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare_two);
            }
            let a = body_rect(&ctx, "A");
            let b = body_rect(&ctx, "B");
            let at = center(b);
            assert!(
                at[0] > a[0] && at[0] < a[2] && at[1] > a[1] && at[1] < a[3],
                "test premise: the probe must lie inside BOTH bodies"
            );
            assert_eq!(
                ctx.windows.last().map(|w| w.title.as_str()),
                Some("B"),
                "test premise: B is the z-top window"
            );
            super::scripted_frame(&mut ctx, at, false, -1.0, declare_two);
            assert!(
                offset_of(&ctx, "B") > 0.0,
                "the z-top window takes the notch"
            );
            assert_eq!(
                offset_of(&ctx, "A"),
                0.0,
                "the window BEHIND registered a false layer gate and must not scroll"
            );
        }

        #[test]
        fn routing_an_open_combo_popups_list_beats_the_scrollable_window_under_it() {
            fn declare_combo_window(ui: &mut Ui, selected: &mut usize) {
                ui.window("W", ScrollFlags::WHEEL, |ui| {
                    ui.combo("Color", &LONG_ITEMS, selected);
                    rows(ui, OVERFLOW_ROWS);
                });
            }
            fn combo_frame(
                ctx: &mut Context,
                mouse: [f32; 2],
                down: bool,
                wheel: f32,
                viewport: [f32; 2],
                selected: &mut usize,
            ) {
                ctx.set_input(InputState {
                    mouse_pos: mouse,
                    mouse_down: [down, false, false, false, false],
                    mouse_wheel: wheel,
                    keys: &[],
                    viewport_size: viewport,
                });
                ctx.new_frame(0.016);
                {
                    let mut ui = ctx.ui();
                    declare_combo_window(&mut ui, selected);
                }
                ctx.render();
            }
            fn list_region_id() -> Id {
                let mut stack = IdStack::new();
                stack.push("W");
                let popup = stack.id("Color");
                stack.push_id(popup);
                stack.id("##combo_list")
            }

            let mut ctx = test_context();
            let mut selected = 0usize;
            let row_h = ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1];
            let pad_y = ctx.style.window_padding[1];
            let spacing_y = ctx.style.item_spacing[1];
            combo_frame(&mut ctx, AWAY, false, 0.0, [0.0, 0.0], &mut selected);
            let origin = super::content_origin(&ctx);
            let button = [origin[0] + 4.0, origin[1] + row_h / 2.0];
            let viewport = [
                400.0,
                origin[1] + row_h + 2.0 * pad_y + 2.0 * row_h + spacing_y,
            ];

            combo_frame(&mut ctx, AWAY, false, 0.0, viewport, &mut selected);
            combo_frame(&mut ctx, button, true, 0.0, viewport, &mut selected);
            combo_frame(&mut ctx, button, false, 0.0, viewport, &mut selected);
            let popup = ctx
                .popup_rect
                .expect("the click must have opened the combo popup");
            assert!(
                child_scroll_state(&ctx, list_region_id()).is_some(),
                "test premise: the clamped popup must declare a scrolling list"
            );
            let window_before = offset_of(&ctx, "W");

            let at = center(popup);
            let body = body_rect(&ctx, "W");
            assert!(
                at[0] > body[0] && at[0] < body[2] && at[1] > body[1] && at[1] < body[3],
                "test premise: the probe must lie over the window's body too"
            );
            combo_frame(&mut ctx, at, false, -1.0, viewport, &mut selected);
            let list = child_scroll_state(&ctx, list_region_id())
                .expect("the list region persists its state")
                .0;
            assert!(list > 0.0, "the popup layer's list must take the notch");
            assert_eq!(
                offset_of(&ctx, "W"),
                window_before,
                "the scrollable window under an open popup must not scroll"
            );
        }

        #[test]
        fn a_container_at_its_clamp_limit_consumes_the_wheel() {
            fn declare_pair(ui: &mut Ui) {
                declare_nested(ui, ScrollFlags::WHEEL, ScrollFlags::WHEEL)
            }
            let mut ctx = test_context();
            for _ in 0..2 {
                super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare_pair);
            }
            let at = over_the_child(&ctx);
            super::scripted_frame(&mut ctx, at, false, -1000.0, declare_pair);
            let id = child_id("c");
            let (pinned, content_height) =
                child_scroll_state(&ctx, id).expect("the child persists its state");
            let child_rect = {
                let origin = super::content_origin(&ctx);
                [
                    origin[0],
                    origin[1],
                    origin[0] + CHILD_SIZE[0],
                    origin[1] + CHILD_SIZE[1],
                ]
            };
            let inset = crate::scroll::content_inset(&ctx.style, child_rect);
            let max_offset = content_height - (CHILD_SIZE[1] - 2.0 * inset);
            assert!(
                (pinned - max_offset).abs() < 1e-3,
                "test premise: the child must sit at its clamp limit ({pinned} vs {max_offset})"
            );
            assert_eq!(
                offset_of(&ctx, "W"),
                0.0,
                "test premise: the window has not scrolled"
            );

            super::scripted_frame(&mut ctx, at, false, -1.0, declare_pair);
            assert_eq!(
                child_scroll_state(&ctx, id).unwrap().0,
                pinned,
                "the exhausted child stays pinned"
            );
            assert_eq!(
                offset_of(&ctx, "W"),
                0.0,
                "and it CONSUMES the notch: the enclosing window must not move"
            );
        }
    }

    mod scrollbar {
        use crate::context::{Context, WidgetState};
        use crate::draw::{DrawVert, WHITE_PIXEL_UV};
        use crate::id::{Id, IdStack};
        use crate::scroll::{ScrollFlags, MIN_SCROLLBAR_THUMB, SCROLLBAR_TAG};
        use crate::ui::Ui;
        use alloc::vec::Vec;

        const AWAY: [f32; 2] = [-1000.0, -1000.0];
        const SIZE: [f32; 2] = [200.0, 96.0];
        const OVERFLOW_ROWS: usize = 20;
        const TRACK_BG: [u8; 4] = [10, 20, 30, 255];
        const THUMB_IDLE: [u8; 4] = [13, 23, 33, 255];
        const THUMB_HOVERED: [u8; 4] = [11, 21, 31, 255];
        const THUMB_ACTIVE: [u8; 4] = [12, 22, 32, 255];

        fn bar_context() -> Context {
            let mut ctx = super::test_context();
            ctx.style.corner_rounding = 0.0;
            ctx.style.scrollbar_bg = TRACK_BG;
            ctx.style.scrollbar_thumb = THUMB_IDLE;
            ctx.style.scrollbar_thumb_hovered = THUMB_HOVERED;
            ctx.style.scrollbar_thumb_active = THUMB_ACTIVE;
            ctx
        }

        fn rows(ui: &mut Ui, n: usize) {
            for _ in 0..n {
                ui.label("row");
            }
        }

        fn declare_child(ui: &mut Ui, flags: ScrollFlags) {
            ui.window("W", ScrollFlags::NONE, |ui| {
                ui.child("c", SIZE, flags, |ui| rows(ui, OVERFLOW_ROWS));
            });
        }

        fn child_id() -> Id {
            let mut stack = IdStack::new();
            stack.push("W");
            stack.id("c")
        }

        fn child_bar_id() -> Id {
            let mut stack = IdStack::new();
            stack.push("W");
            stack.push_id(child_id());
            stack.id(SCROLLBAR_TAG)
        }

        fn window_bar_id() -> Id {
            let mut stack = IdStack::new();
            stack.push("W");
            stack.id(SCROLLBAR_TAG)
        }

        fn scroll_state(ctx: &Context, id: Id) -> (f32, f32) {
            match ctx.widget_states.get(&id) {
                Some(WidgetState::Scroll {
                    offset,
                    content_height,
                }) => (*offset, *content_height),
                _ => panic!("the child must have persisted scroll state"),
            }
        }

        fn set_offset(ctx: &mut Context, id: Id, value: f32) {
            match ctx.widget_states.get_mut(&id) {
                Some(WidgetState::Scroll { offset, .. }) => *offset = value,
                _ => panic!("the child must have persisted scroll state"),
            }
        }

        fn child_rect(ctx: &Context) -> [f32; 4] {
            let o = super::content_origin(ctx);
            [o[0], o[1], o[0] + SIZE[0], o[1] + SIZE[1]]
        }

        fn child_track_rect(ctx: &Context) -> [f32; 4] {
            let r = child_rect(ctx);
            let b = ctx.style.border_size;
            let bar = ctx.style.scrollbar_size;
            [r[2] - b - bar, r[1] + b, r[2] - b, r[3] - b]
        }

        fn window_track_rect(ctx: &Context) -> [f32; 4] {
            let r = ctx.windows[0].rect();
            let bar = ctx.style.scrollbar_size;
            [r[2] - bar, r[1] + super::title_bar_height(ctx), r[2], r[3]]
        }

        fn center(rect: [f32; 4]) -> [f32; 2] {
            [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
        }

        fn bounds_of(verts: &[DrawVert], col: [u8; 4]) -> Option<[f32; 4]> {
            let mut found = false;
            let mut r = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
            for v in verts.iter().filter(|v| v.col == col) {
                found = true;
                r[0] = r[0].min(v.pos[0]);
                r[1] = r[1].min(v.pos[1]);
                r[2] = r[2].max(v.pos[0]);
                r[3] = r[3].max(v.pos[1]);
            }
            if found {
                Some(r)
            } else {
                None
            }
        }

        fn assert_close(got: f32, want: f32, what: &str) {
            assert!(
                (got - want).abs() < 1e-3,
                "{what}: expected {want}, got {got}"
            );
        }

        #[test]
        fn bar_narrows_the_content_rect_by_scrollbar_size_and_adds_track_and_thumb() {
            fn declare(ui: &mut Ui, flags: ScrollFlags, width: &mut f32) {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    ui.child("c", SIZE, flags, |ui| {
                        *width = crate::widgets::available_width(ui.ctx);
                        rows(ui, OVERFLOW_ROWS);
                    });
                });
            }

            let mut wheel_ctx = bar_context();
            let mut wheel_width = 0.0;
            super::scripted_frame(&mut wheel_ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::WHEEL, &mut wheel_width)
            });
            let (wheel_verts, _, _) =
                super::scripted_frame(&mut wheel_ctx, AWAY, false, 0.0, |ui| {
                    declare(ui, ScrollFlags::WHEEL, &mut wheel_width)
                });

            let mut bar_ctx = bar_context();
            let mut bar_width = 0.0;
            super::scripted_frame(&mut bar_ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::BAR, &mut bar_width)
            });
            let (bar_verts, _, _) = super::scripted_frame(&mut bar_ctx, AWAY, false, 0.0, |ui| {
                declare(ui, ScrollFlags::BAR, &mut bar_width)
            });

            assert_close(
                bar_width,
                wheel_width - bar_ctx.style.scrollbar_size,
                "the BAR content rect is narrower by exactly Style::scrollbar_size",
            );
            assert_eq!(
                bar_verts.len(),
                wheel_verts.len() + 8,
                "BAR adds exactly the track quad and the thumb quad"
            );
        }

        #[test]
        fn thumb_spans_the_track_ends_at_the_offset_limits_and_centres_mid_range() {
            let mut ctx = bar_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare_child(ui, ScrollFlags::BAR)
            });
            let id = child_id();
            let track = child_track_rect(&ctx);
            let track_h = track[3] - track[1];
            let (_, content_height) = scroll_state(&ctx, id);
            let max_offset = content_height - track_h;
            assert!(
                max_offset > 0.0,
                "test premise: the content must overflow ({content_height} vs {track_h})"
            );

            let at = center(track);
            let mut thumbs = Vec::new();
            for offset in [0.0, max_offset / 2.0, max_offset] {
                set_offset(&mut ctx, id, offset);
                let (verts, _, _) = super::scripted_frame(&mut ctx, at, false, 0.0, |ui| {
                    declare_child(ui, ScrollFlags::BAR)
                });
                thumbs
                    .push(bounds_of(&verts, THUMB_HOVERED).expect("the hovered thumb is emitted"));
            }

            for t in &thumbs {
                assert_close(t[0], track[0], "thumb left");
                assert_close(t[2], track[2], "thumb right");
            }
            let extent = thumbs[0][3] - thumbs[0][1];
            assert!(
                extent >= MIN_SCROLLBAR_THUMB && extent < track_h,
                "a proportional thumb: {extent} in [{MIN_SCROLLBAR_THUMB}, {track_h})"
            );
            for t in &thumbs {
                assert_close(t[3] - t[1], extent, "thumb extent is offset-independent");
            }

            assert_close(thumbs[0][1], track[1], "offset 0 puts the thumb at the top");
            assert_close(
                thumbs[2][3],
                track[3],
                "max_offset puts the thumb at the bottom",
            );
            assert_close(
                (thumbs[1][1] + thumbs[1][3]) / 2.0,
                (track[1] + track[3]) / 2.0,
                "half the offset centres the thumb on the track",
            );
            assert!(
                thumbs[0][1] < thumbs[1][1] && thumbs[1][1] < thumbs[2][1],
                "the thumb advances monotonically with the offset"
            );
        }

        #[test]
        fn dragging_the_thumb_advances_the_offset_proportionally_and_clamps_at_both_ends() {
            let mut ctx = bar_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare_child(ui, ScrollFlags::BAR)
            });
            let id = child_id();
            let track = child_track_rect(&ctx);
            let track_h = track[3] - track[1];
            let (_, content_height) = scroll_state(&ctx, id);
            let max_offset = content_height - track_h;
            assert!(max_offset > 0.0, "test premise: the content must overflow");

            let hover_at = center(track);
            let (verts, _, _) = super::scripted_frame(&mut ctx, hover_at, false, 0.0, |ui| {
                declare_child(ui, ScrollFlags::BAR)
            });
            let thumb = bounds_of(&verts, THUMB_HOVERED).expect("the hovered thumb is emitted");
            let extent = thumb[3] - thumb[1];
            let travel = track_h - extent;
            assert!(
                travel > 0.0,
                "test premise: the thumb must have room to move"
            );

            let grab_at = [track[0] + 2.0, thumb[1] + extent / 2.0];
            super::scripted_frame(&mut ctx, grab_at, true, 0.0, |ui| {
                declare_child(ui, ScrollFlags::BAR)
            });
            assert_eq!(
                ctx.active,
                Some(child_bar_id()),
                "the press on the thumb captures the bar"
            );
            assert_eq!(
                scroll_state(&ctx, id).0,
                0.0,
                "grabbing the thumb must not jump the content"
            );

            let mut dragged = 0.0;
            for step in [8.0f32, 8.0, 8.0] {
                dragged += step;
                super::scripted_frame(
                    &mut ctx,
                    [grab_at[0], grab_at[1] + dragged],
                    true,
                    0.0,
                    |ui| declare_child(ui, ScrollFlags::BAR),
                );
                assert_close(
                    scroll_state(&ctx, id).0,
                    dragged / travel * max_offset,
                    "the offset advances proportionally with the drag",
                );
            }
            assert!(
                scroll_state(&ctx, id).0 > 0.0,
                "sanity: the drag actually moved the content"
            );

            super::scripted_frame(
                &mut ctx,
                [track[0] - 400.0, track[3] + 500.0],
                true,
                0.0,
                |ui| declare_child(ui, ScrollFlags::BAR),
            );
            assert_close(
                scroll_state(&ctx, id).0,
                max_offset,
                "dragging past the bottom clamps at max_offset",
            );
            super::scripted_frame(
                &mut ctx,
                [track[2] + 400.0, track[1] - 500.0],
                true,
                0.0,
                |ui| declare_child(ui, ScrollFlags::BAR),
            );
            assert_close(
                scroll_state(&ctx, id).0,
                0.0,
                "dragging past the top clamps at 0",
            );

            super::scripted_frame(&mut ctx, hover_at, false, 0.0, |ui| {
                declare_child(ui, ScrollFlags::BAR)
            });
            assert_eq!(ctx.active, None, "the release drops the capture");
        }

        #[test]
        fn a_track_click_outside_the_thumb_centres_it_on_the_click() {
            let mut ctx = bar_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                declare_child(ui, ScrollFlags::BAR)
            });
            let id = child_id();
            let track = child_track_rect(&ctx);
            let track_h = track[3] - track[1];
            let (_, content_height) = scroll_state(&ctx, id);
            let max_offset = content_height - track_h;

            set_offset(&mut ctx, id, max_offset / 2.0);
            let (verts, _, _) = super::scripted_frame(&mut ctx, center(track), false, 0.0, |ui| {
                declare_child(ui, ScrollFlags::BAR)
            });
            let thumb = bounds_of(&verts, THUMB_HOVERED).expect("the hovered thumb is emitted");
            let extent = thumb[3] - thumb[1];

            for target_y in [track[1] + extent / 2.0 + 2.0, track[3] - extent / 2.0 - 2.0] {
                assert!(
                    target_y < thumb[1] || target_y > thumb[3],
                    "test premise: the click must land outside the thumb"
                );
                set_offset(&mut ctx, id, max_offset / 2.0);
                super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                    declare_child(ui, ScrollFlags::BAR)
                });
                set_offset(&mut ctx, id, max_offset / 2.0);
                let (verts, _, _) =
                    super::scripted_frame(&mut ctx, [track[0] + 2.0, target_y], true, 0.0, |ui| {
                        declare_child(ui, ScrollFlags::BAR)
                    });
                let jumped =
                    bounds_of(&verts, THUMB_ACTIVE).expect("the captured thumb is emitted");
                assert_close(
                    (jumped[1] + jumped[3]) / 2.0,
                    target_y,
                    "the thumb's centre lands on the click",
                );
                assert_close(jumped[3] - jumped[1], extent, "the extent is unchanged");
                super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| {
                    declare_child(ui, ScrollFlags::BAR)
                });
            }
        }

        #[test]
        fn a_child_whose_content_fits_emits_no_bar_and_releases_the_strip() {
            fn declare(ui: &mut Ui, width: &mut f32) {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    ui.child("c", SIZE, ScrollFlags::BAR, |ui| {
                        *width = crate::widgets::available_width(ui.ctx);
                        ui.label("one line");
                    });
                });
            }
            let mut ctx = bar_context();
            let mut width = 0.0;
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| declare(ui, &mut width));
            let id = child_id();
            let track = child_track_rect(&ctx);
            let (_, content_height) = scroll_state(&ctx, id);
            assert!(
                content_height <= track[3] - track[1],
                "test premise: the content must fit"
            );

            fn declare_wheel(ui: &mut Ui, width: &mut f32) {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    ui.child("c", SIZE, ScrollFlags::WHEEL, |ui| {
                        *width = crate::widgets::available_width(ui.ctx);
                        ui.label("one line");
                    });
                });
            }
            let mut wheel_ctx = bar_context();
            let mut wheel_width = 0.0;
            super::scripted_frame(&mut wheel_ctx, AWAY, false, 0.0, |ui| {
                declare_wheel(ui, &mut wheel_width)
            });
            super::scripted_frame(&mut wheel_ctx, AWAY, false, 0.0, |ui| {
                declare_wheel(ui, &mut wheel_width)
            });

            let at = center(track);
            let (verts, _, _) =
                super::scripted_frame(&mut ctx, at, false, 0.0, |ui| declare(ui, &mut width));
            assert!(
                bounds_of(&verts, TRACK_BG).is_none(),
                "content that fits emits NO track geometry"
            );
            for col in [THUMB_IDLE, THUMB_HOVERED, THUMB_ACTIVE] {
                assert!(
                    bounds_of(&verts, col).is_none(),
                    "content that fits emits NO thumb geometry ({col:?})"
                );
            }
            assert_ne!(ctx.hot, Some(child_bar_id()), "no bar, nothing to hover");
            assert_close(
                width,
                wheel_width,
                "the strip is released: the content is as wide as a WHEEL child's",
            );

            super::scripted_frame(&mut ctx, at, true, 0.0, |ui| declare(ui, &mut width));
            assert_eq!(ctx.active, None, "there is no bar to capture the press");
            super::scripted_frame(&mut ctx, [at[0], track[3] + 200.0], true, 0.0, |ui| {
                declare(ui, &mut width)
            });
            assert_eq!(ctx.active, None);
            assert_eq!(
                scroll_state(&ctx, id).0,
                0.0,
                "a culled bar never changes the offset"
            );
        }

        #[test]
        fn a_window_whose_content_fits_emits_no_bar() {
            fn declare(ui: &mut Ui) {
                ui.window("W", ScrollFlags::BAR, |ui| rows(ui, 1));
            }
            let mut ctx = bar_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare);
            let track = window_track_rect(&ctx);
            let (verts, _, _) = super::scripted_frame(&mut ctx, center(track), false, 0.0, declare);
            assert!(
                bounds_of(&verts, TRACK_BG).is_none(),
                "a fitting window body emits NO track"
            );
            for col in [THUMB_IDLE, THUMB_HOVERED, THUMB_ACTIVE] {
                assert!(
                    bounds_of(&verts, col).is_none(),
                    "a fitting window body emits NO thumb ({col:?})"
                );
            }
            assert_ne!(ctx.hot, Some(window_bar_id()));
        }

        #[test]
        fn the_same_child_gains_a_bar_only_once_its_rows_overflow() {
            fn declare(ui: &mut Ui, n: usize) {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    ui.child("c", SIZE, ScrollFlags::BAR, |ui| rows(ui, n));
                });
            }
            fn bar_emitted(rows_n: usize) -> bool {
                let mut ctx = bar_context();
                super::scripted_frame(&mut ctx, AWAY, false, 0.0, |ui| declare(ui, rows_n));
                let at = center(child_track_rect(&ctx));
                super::scripted_frame(&mut ctx, at, false, 0.0, |ui| declare(ui, rows_n));
                let (verts, _, _) =
                    super::scripted_frame(&mut ctx, at, false, 0.0, |ui| declare(ui, rows_n));
                let track = bounds_of(&verts, TRACK_BG).is_some();
                let thumb = [THUMB_IDLE, THUMB_HOVERED, THUMB_ACTIVE]
                    .iter()
                    .any(|c| bounds_of(&verts, *c).is_some());
                assert_eq!(track, thumb, "track and thumb are emitted together");
                track
            }

            let mut fitting = 0;
            for n in 1..=OVERFLOW_ROWS {
                if bar_emitted(n) {
                    break;
                }
                fitting = n;
            }
            assert!(fitting >= 1, "test premise: some row count must fit");
            assert!(!bar_emitted(fitting), "{fitting} rows fit: no bar");
            assert!(
                bar_emitted(fitting + 1),
                "{} rows overflow: the bar appears",
                fitting + 1
            );
            assert!(
                bar_emitted(OVERFLOW_ROWS),
                "and comfortably-overflowing content keeps it"
            );
        }

        #[test]
        fn short_track_with_overflowing_content_stays_inert_and_keeps_the_offset() {
            const TINY_SIZE: [f32; 2] = [200.0, 12.0];

            fn declare(ui: &mut Ui) {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    ui.child("c", TINY_SIZE, ScrollFlags::BAR, |ui| {
                        rows(ui, OVERFLOW_ROWS)
                    });
                });
            }
            let mut ctx = bar_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare);
            let id = child_id();

            let o = super::content_origin(&ctx);
            let rect = [o[0], o[1], o[0] + TINY_SIZE[0], o[1] + TINY_SIZE[1]];
            let b = ctx.style.border_size;
            let bar = ctx.style.scrollbar_size;
            let track = [rect[2] - b - bar, rect[1] + b, rect[2] - b, rect[3] - b];
            let track_h = track[3] - track[1];
            assert!(
                track_h < MIN_SCROLLBAR_THUMB,
                "test premise: the track is shorter than the minimum thumb ({track_h})"
            );

            let (_, content_height) = scroll_state(&ctx, id);
            let max_offset = content_height - track_h;
            assert!(
                max_offset > 0.0,
                "test premise: content overflows the tiny track"
            );

            set_offset(&mut ctx, id, max_offset / 2.0);

            let at = center(track);
            let (verts, _, _) = super::scripted_frame(&mut ctx, at, false, 0.0, declare);
            assert!(
                bounds_of(&verts, THUMB_HOVERED).is_none(),
                "a short-track bar never hover-highlights"
            );
            assert_ne!(
                ctx.hot,
                Some(child_bar_id()),
                "a short-track bar never sets hot"
            );

            super::scripted_frame(&mut ctx, at, true, 0.0, declare);
            assert_eq!(ctx.active, None, "a short-track bar never captures");
            assert_eq!(
                scroll_state(&ctx, id).0,
                max_offset / 2.0,
                "a click on a short-track bar must not disturb the held offset"
            );
        }

        #[test]
        fn a_window_bar_sits_in_the_body_and_the_resize_grip_still_renders_last() {
            fn declare(ui: &mut Ui) {
                ui.window("W", ScrollFlags::BAR, |ui| rows(ui, 60));
            }
            let mut ctx = bar_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare);
            let track = window_track_rect(&ctx);
            let at = center(track);
            let (verts, _, _) = super::scripted_frame(&mut ctx, at, false, 0.0, declare);

            let thumb = bounds_of(&verts, THUMB_HOVERED).expect("the hovered thumb is emitted");
            assert_close(thumb[0], track[0], "the thumb spans the strip");
            assert_close(thumb[2], track[2], "the thumb spans the strip");
            assert!(
                thumb[1] >= track[1] && thumb[3] <= track[3],
                "the thumb stays inside the body strip"
            );
            let bar_top = bounds_of(&verts, TRACK_BG).expect("the track is emitted")[1];
            assert_close(
                bar_top,
                track[1],
                "the bar starts at the body's top, below the title bar",
            );

            let grip_rgb = ctx.style.title_text;
            let last_bar = verts
                .iter()
                .rposition(|v| v.col == TRACK_BG || v.col == THUMB_HOVERED)
                .expect("the bar is emitted");
            let last_grip = verts
                .iter()
                .rposition(|v| v.col[..3] == grip_rgb[..3])
                .expect("the grip marker is emitted");
            assert!(
                last_grip > last_bar,
                "the grip must remain the window's last geometry ({last_grip} vs {last_bar})"
            );
            assert_eq!(
                last_grip,
                verts.len() - 1,
                "and nothing at all follows the grip"
            );
        }

        #[test]
        fn zero_corner_rounding_emits_one_crisp_quad_per_bar_part() {
            fn declare(ui: &mut Ui) {
                declare_child(ui, ScrollFlags::BAR)
            }
            let mut ctx = bar_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare);
            let track = child_track_rect(&ctx);
            let at = center(track);
            let (verts, _, _) = super::scripted_frame(&mut ctx, at, false, 0.0, declare);

            let thumb: Vec<&DrawVert> = verts.iter().filter(|v| v.col == THUMB_HOVERED).collect();
            assert_eq!(thumb.len(), 4, "one crisp quad, not a 9-slice composition");
            for v in &thumb {
                assert_eq!(v.uv, WHITE_PIXEL_UV, "crisp chrome samples the white pixel");
                assert!(
                    v.pos[0] == track[0] || v.pos[0] == track[2],
                    "the thumb spans the track's full width exactly"
                );
            }

            let mut rounded = bar_context();
            rounded.style.corner_rounding = 4.0;
            super::scripted_frame(&mut rounded, AWAY, false, 0.0, declare);
            let (rverts, _, _) = super::scripted_frame(&mut rounded, at, false, 0.0, declare);
            let rounded_thumb = rverts.iter().filter(|v| v.col == THUMB_HOVERED).count();
            assert!(
                rounded_thumb > 4,
                "rounded chrome is a 9-slice composition ({rounded_thumb} vertices)"
            );
        }

        #[test]
        fn wants_mouse_capture_holds_through_a_scrollbar_drag_off_the_window() {
            fn declare(ui: &mut Ui) {
                ui.window("W", ScrollFlags::BAR, |ui| rows(ui, 60));
            }
            let mut ctx = bar_context();
            super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare);
            assert!(
                !ctx.wants_mouse_capture(),
                "test premise: an idle frame with the mouse away wants nothing"
            );

            let track = window_track_rect(&ctx);
            let press_at = [center(track)[0], track[1] + MIN_SCROLLBAR_THUMB / 2.0];
            super::scripted_frame(&mut ctx, press_at, true, 0.0, declare);
            assert_eq!(ctx.active, Some(window_bar_id()), "the bar captured");
            assert!(ctx.wants_mouse_capture());

            super::scripted_frame(&mut ctx, AWAY, true, 0.0, declare);
            assert_eq!(ctx.active, Some(window_bar_id()), "the drag survives");
            assert!(
                ctx.wants_mouse_capture(),
                "a held scrollbar drag keeps the host swallowing mouse messages \
                 even off the window"
            );

            super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare);
            assert_eq!(ctx.active, None);
            assert!(!ctx.wants_mouse_capture());
        }

        #[test]
        fn bar_closure_forms_match_their_begin_end_pairs() {
            fn closure(ui: &mut Ui) {
                ui.window("W", ScrollFlags::BAR, |ui| {
                    ui.child("c", SIZE, ScrollFlags::BAR, |ui| rows(ui, OVERFLOW_ROWS));
                    rows(ui, 40);
                });
            }
            fn pair(ui: &mut Ui) {
                assert!(ui.begin_window("W", ScrollFlags::BAR));
                let scope = ui.begin_child("c", SIZE, ScrollFlags::BAR);
                rows(ui, OVERFLOW_ROWS);
                ui.end_child(scope);
                rows(ui, 40);
                ui.end_window();
            }

            fn script(
                declare: fn(&mut Ui),
            ) -> Vec<(Vec<DrawVert>, Vec<u32>, Vec<crate::draw::DrawCmd>)> {
                let mut ctx = bar_context();
                let mut frames = Vec::new();
                frames.push(super::scripted_frame(&mut ctx, AWAY, false, 0.0, declare));
                let track = child_track_rect(&ctx);
                let at = [track[0] + 2.0, track[1] + 4.0];
                frames.push(super::scripted_frame(&mut ctx, at, true, 0.0, declare));
                for step in [10.0f32, 20.0] {
                    frames.push(super::scripted_frame(
                        &mut ctx,
                        [at[0], at[1] + step],
                        true,
                        0.0,
                        declare,
                    ));
                }
                frames.push(super::scripted_frame(&mut ctx, at, false, 0.0, declare));
                frames
            }

            let closure_frames = script(closure);
            assert_ne!(
                closure_frames[1], closure_frames[3],
                "sanity: the drag must have moved something"
            );
            assert_eq!(
                closure_frames,
                script(pair),
                "the closure forms must stay byte-identical sugar over the pairs"
            );
        }
    }

    mod rounded_chrome {
        use crate::context::Context;
        use crate::draw::{DrawData, TextureId, WHITE_PIXEL_UV};
        use crate::ui::Ui;
        use crate::widgets::test_support::next_text_rect;

        fn frame(ctx: &mut Context, f: impl FnOnce(&mut Ui)) {
            super::popup_frame(ctx, crate::input::InputState::default(), f);
        }

        fn drag_frame(ctx: &mut Context, mouse: [f32; 2], left_down: bool) {
            super::popup_frame(
                ctx,
                crate::input::InputState {
                    mouse_pos: mouse,
                    mouse_down: [left_down, false, false, false, false],
                    ..Default::default()
                },
                |_| {},
            );
        }

        fn whole_quad_at(data: DrawData<'_>, rect: [f32; 4]) -> Option<(TextureId, [f32; 2])> {
            let mut offset = 0usize;
            for cmd in data.commands {
                let next = offset + cmd.elem_count as usize;
                let idx = &data.indices[offset..next];
                if !idx.is_empty() {
                    let lo = *idx.iter().min().expect("non-empty") as usize;
                    let hi = *idx.iter().max().expect("non-empty") as usize;
                    for anchor in data.vertices[lo..=hi].windows(3) {
                        if anchor[0].pos == [rect[0], rect[1]]
                            && anchor[2].pos == [rect[2], rect[3]]
                        {
                            return Some((cmd.texture, anchor[0].uv));
                        }
                    }
                }
                offset = next;
            }
            None
        }

        fn window_quad_at(ctx: &Context, rect: [f32; 4]) -> Option<(TextureId, [f32; 2])> {
            whole_quad_at(ctx.windows[0].draw_list.as_draw_data(), rect)
        }

        fn title_rect(ctx: &Context) -> [f32; 4] {
            let win = ctx.windows[0].rect();
            let h = ctx.font.line_height() + 2.0 * ctx.style.frame_padding[1];
            [win[0], win[1], win[2], win[1] + h]
        }

        #[test]
        fn rounding_on_composes_chrome_from_corner_patches() {
            let mut ctx = super::test_context();
            assert!(
                ctx.style.corner_rounding > 0.0,
                "the default style rounds its chrome"
            );
            let mut button = [0.0; 4];
            frame(&mut ctx, |ui| {
                button = next_text_rect(ui, "OK");
                ui.button("OK");
            });
            let r = ctx.style.corner_rounding;
            for rect in [ctx.windows[0].rect(), button] {
                let [x0, y0, x1, y1] = rect;
                assert!(
                    window_quad_at(&ctx, rect).is_none(),
                    "rounded chrome must not be one whole-rect quad at {rect:?}"
                );
                for corner in [
                    [x0, y0, x0 + r, y0 + r],
                    [x1 - r, y0, x1, y0 + r],
                    [x0, y1 - r, x0 + r, y1],
                    [x1 - r, y1 - r, x1, y1],
                ] {
                    let (tex, uv) = window_quad_at(&ctx, corner).expect("corner quad");
                    assert_eq!(
                        tex,
                        TextureId::FONT,
                        "corner {corner:?} must sample the patch"
                    );
                    assert_ne!(uv, WHITE_PIXEL_UV);
                }
                for solid in [
                    [x0 + r, y0 + r, x1 - r, y1 - r],
                    [x0 + r, y0, x1 - r, y0 + r],
                    [x0 + r, y1 - r, x1 - r, y1],
                    [x0, y0 + r, x0 + r, y1 - r],
                    [x1 - r, y0 + r, x1, y1 - r],
                ] {
                    let (tex, uv) = window_quad_at(&ctx, solid).expect("solid part");
                    assert_eq!(tex, TextureId::WHITE, "solid part {solid:?} must be crisp");
                    assert_eq!(uv, WHITE_PIXEL_UV);
                }
            }
        }

        #[test]
        fn zero_rounding_is_byte_identical_to_the_crisp_path() {
            fn frame_streams(
                rounding: f32,
            ) -> (alloc::vec::Vec<crate::draw::DrawVert>, alloc::vec::Vec<u32>) {
                let mut ctx = super::test_context();
                ctx.style.corner_rounding = rounding;
                frame(&mut ctx, |ui| {
                    ui.button("OK");
                });
                let data = ctx.windows[0].draw_list.as_draw_data();
                (data.vertices.to_vec(), data.indices.to_vec())
            }
            let zero = frame_streams(0.0);
            let negative = frame_streams(-3.0);
            assert_eq!(zero, negative, "any rounding <= 0 must emit the crisp path");

            let mut ctx = super::test_context();
            ctx.style.corner_rounding = 0.0;
            let mut button = [0.0; 4];
            frame(&mut ctx, |ui| {
                button = next_text_rect(ui, "OK");
                ui.button("OK");
            });
            let win = ctx.windows[0].rect();
            for rect in [win, title_rect(&ctx), button] {
                let (tex, uv) = window_quad_at(&ctx, rect).expect("chrome quad");
                assert_eq!(tex, TextureId::WHITE, "crisp chrome rect at {rect:?}");
                assert_eq!(uv, WHITE_PIXEL_UV, "crisp chrome samples the white pixel");
            }
        }

        #[test]
        fn masks_are_cached_across_frames_with_one_generation_bump() {
            let mut ctx = super::test_context();
            frame(&mut ctx, |ui| {
                ui.button("OK");
            });
            let gen = ctx.font().atlas_generation();
            frame(&mut ctx, |ui| {
                ui.button("OK");
            });
            assert_eq!(
                ctx.font().atlas_generation(),
                gen,
                "steady frames must hit the mask cache (no atlas mutation)"
            );
        }

        #[test]
        fn resize_drag_reuses_the_same_corner_patch_at_every_size() {
            let mut ctx = super::test_context();
            frame(&mut ctx, |_| {});
            let gen = ctx.font().atlas_generation();
            let initial_size = ctx.windows[0].size;
            let grip = {
                let r = ctx.windows[0].rect();
                [r[2] - 8.0, r[3] - 8.0]
            };
            drag_frame(&mut ctx, grip, true);
            assert!(ctx.active.is_some(), "the grip must capture the press");
            for i in 1..=20 {
                let at = [grip[0] + 5.0 * i as f32, grip[1] + 3.0 * i as f32];
                drag_frame(&mut ctx, at, true);
                assert_eq!(
                    ctx.font().atlas_generation(),
                    gen,
                    "size step {i} must not mint a new mask or re-upload the atlas"
                );
            }
            drag_frame(&mut ctx, grip, false);
            assert_ne!(
                ctx.windows[0].size, initial_size,
                "the drag must have resized"
            );
            let win = ctx.windows[0].rect();
            let r = ctx.style.corner_rounding;
            let (tex, _) = window_quad_at(&ctx, [win[0], win[1], win[0] + r, win[1] + r])
                .expect("bg corner quad");
            assert_eq!(
                tex,
                TextureId::FONT,
                "chrome must stay rounded through the drag"
            );
        }

        #[test]
        fn wide_chrome_rounds_via_radius_only_keys() {
            let mut ctx = super::test_context();
            let long = "W".repeat(80);
            let mut rect = [0.0; 4];
            frame(&mut ctx, |ui| {
                rect = next_text_rect(ui, &long);
                ui.button(&long);
            });
            let r = ctx.style.corner_rounding;
            assert!(
                rect[2] - rect[0] > 512.0,
                "the test button must be wide: {rect:?}"
            );
            let (tex, _) = window_quad_at(&ctx, [rect[0], rect[1], rect[0] + r, rect[1] + r])
                .expect("button corner quad");
            assert_eq!(
                tex,
                TextureId::FONT,
                "wide chrome must round like any other"
            );
            let (tex, uv) =
                window_quad_at(&ctx, [rect[0] + r, rect[1] + r, rect[2] - r, rect[3] - r])
                    .expect("button center");
            assert_eq!(tex, TextureId::WHITE);
            assert_eq!(uv, WHITE_PIXEL_UV);
        }

        #[test]
        fn title_bar_rounds_only_the_top_corners() {
            let mut ctx = super::test_context();
            frame(&mut ctx, |_| {});
            let r = ctx.style.corner_rounding;
            let title = title_rect(&ctx);
            let [x0, y0, x1, y1] = title;
            for corner in [[x0, y0, x0 + r, y0 + r], [x1 - r, y0, x1, y0 + r]] {
                let (tex, _) = window_quad_at(&ctx, corner).expect("title corner quad");
                assert_eq!(tex, TextureId::FONT, "title top corner {corner:?}");
            }
            let (tex, uv) = window_quad_at(&ctx, [x0, y0 + r, x1, y1]).expect("title body");
            assert_eq!(tex, TextureId::WHITE, "the title body is one solid rect");
            assert_eq!(uv, WHITE_PIXEL_UV);
            assert!(
                window_quad_at(&ctx, [x0, y1 - r, x0 + r, y1]).is_none(),
                "no bottom-left corner quad"
            );
            assert!(
                window_quad_at(&ctx, [x1 - r, y1 - r, x1, y1]).is_none(),
                "no bottom-right corner quad"
            );
        }

        #[test]
        fn small_widget_radius_clamps_to_half_the_smaller_dimension() {
            let mut ctx = super::test_context();
            let mut on = false;
            ctx.style.corner_rounding = 500.0;
            frame(&mut ctx, |ui| {
                ui.checkbox("On", &mut on);
            });
            let side = ctx.font.line_height();
            let clamped = crate::font::ShapeKey::rounded_corner(side / 2.0);
            let gen = ctx.font().atlas_generation();
            assert!(ctx.font.shape_mask(clamped).is_some());
            assert_eq!(
                ctx.font().atlas_generation(),
                gen,
                "the box mask used exactly side/2 — the probe must hit the cache"
            );
            assert!(ctx
                .font
                .shape_mask(crate::font::ShapeKey::rounded_corner(500.0))
                .is_some());
            assert!(
                ctx.font().atlas_generation() > gen,
                "the unclamped radius must be a cache miss (a new patch)"
            );
        }

        #[test]
        fn popup_background_rounds_like_the_window_chrome() {
            let popup = [400.0, 300.0, 560.0, 460.0];
            let mut ctx = super::test_context();
            frame(&mut ctx, |ui| {
                let id = ui.widget_id("##popup");
                ui.open_popup(id);
                ui.popup(id, popup, |_| {});
            });
            let r = ctx.style.corner_rounding;
            let (tex, _) = whole_quad_at(
                ctx.overlay.as_draw_data(),
                [popup[0], popup[1], popup[0] + r, popup[1] + r],
            )
            .expect("popup corner quad");
            assert_eq!(tex, TextureId::FONT, "the popup background must round");
            let mut ctx = super::test_context();
            ctx.style.corner_rounding = 0.0;
            frame(&mut ctx, |ui| {
                let id = ui.widget_id("##popup");
                ui.open_popup(id);
                ui.popup(id, popup, |_| {});
            });
            let (tex, uv) =
                whole_quad_at(ctx.overlay.as_draw_data(), popup).expect("popup bg quad");
            assert_eq!(
                tex,
                TextureId::WHITE,
                "zero rounding keeps the crisp popup bg"
            );
            assert_eq!(uv, WHITE_PIXEL_UV);
        }

        #[test]
        fn slider_and_text_input_and_keybind_and_combo_emit_corner_quads() {
            let mut ctx = super::test_context();
            let mut gain = 0.5_f32;
            let mut buf = alloc::string::String::new();
            let mut binding = crate::input::KeyBinding::Key(crate::input::Key::G);
            let mut selected = 0usize;
            let (mut track, mut field, mut keybind, mut combo) =
                ([0.0; 4], [0.0; 4], [0.0; 4], [0.0; 4]);
            frame(&mut ctx, |ui| {
                let style = ui.ctx.style;
                let pos = ui.ctx.current_window_mut().cursor.next_pos();
                let available = ui.ctx.current_window_mut().cursor.available_width();
                let h = ui.ctx.font.line_height() + 2.0 * style.frame_padding[1];
                field = [pos[0], pos[1], pos[0] + available, pos[1] + h];
                ui.text_input("##t", &mut buf);
                let pos = ui.ctx.current_window_mut().cursor.next_pos();
                track = [pos[0], pos[1], pos[0] + available, pos[1] + h];
                ui.slider_float("##s", &mut gain, 0.0..=1.0);
                let pos = ui.ctx.current_window_mut().cursor.next_pos();
                ui.keybind("##k", &mut binding);
                keybind = [pos[0], pos[1], pos[0] + 1.0, pos[1] + h];
                let pos = ui.ctx.current_window_mut().cursor.next_pos();
                ui.combo("##c", &["a", "b"], &mut selected);
                combo = [pos[0], pos[1], pos[0] + 1.0, pos[1] + h];
            });
            let r = ctx.style.corner_rounding;
            for (name, rect) in [("field", field), ("track", track)] {
                let (tex, _) =
                    window_quad_at(&ctx, [rect[0], rect[1], rect[0] + r, rect[1] + r]).expect(name);
                assert_eq!(tex, TextureId::FONT, "{name} must emit a corner patch quad");
            }
            let data = ctx.windows[0].draw_list.as_draw_data();
            for (name, p) in [
                ("keybind", [keybind[0], keybind[1]]),
                ("combo", [combo[0], combo[1]]),
            ] {
                let found = data
                    .vertices
                    .iter()
                    .any(|v| v.pos == p && ctx.font.uv_samples_shape_mask(v.uv));
                assert!(found, "{name} button must emit a corner patch quad");
            }
        }
    }

    mod capture_queries {
        use crate::scroll::ScrollFlags;
        use alloc::string::String;

        use super::test_context;
        use crate::context::Context;
        use crate::input::{InputState, Key, KeyBinding, KeyEvent};
        use crate::ui::{point_in_rect, Ui};

        const AWAY: [f32; 2] = [-1000.0, -1000.0];

        fn frame(ctx: &mut Context, input: InputState, declare: impl FnOnce(&mut Ui)) {
            ctx.set_input(input);
            ctx.new_frame(0.016);
            {
                let mut ui = ctx.ui();
                declare(&mut ui);
            }
            ctx.render();
        }

        fn input_at(mouse_pos: [f32; 2]) -> InputState<'static> {
            InputState {
                mouse_pos,
                ..Default::default()
            }
        }

        fn click_at(mouse_pos: [f32; 2], left_down: bool) -> InputState<'static> {
            InputState {
                mouse_pos,
                mouse_down: [left_down, false, false, false, false],
                ..Default::default()
            }
        }

        #[test]
        fn empty_desktop_wants_no_capture() {
            let mut ctx = test_context();
            frame(&mut ctx, InputState::default(), |_| {});
            assert!(!ctx.wants_mouse_capture());
            assert!(!ctx.wants_keyboard_capture());
        }

        #[test]
        fn cursor_inside_a_window_wants_the_mouse_but_not_the_keyboard() {
            let mut ctx = test_context();
            frame(&mut ctx, input_at(AWAY), |ui| {
                ui.window("W", ScrollFlags::NONE, |_| {});
            });
            let win = ctx.windows[0].rect();
            let inside = [(win[0] + win[2]) / 2.0, (win[1] + win[3]) / 2.0];
            frame(&mut ctx, input_at(inside), |ui| {
                ui.window("W", ScrollFlags::NONE, |_| {});
            });
            assert!(ctx.wants_mouse_capture());
            assert!(!ctx.wants_keyboard_capture());
        }

        #[test]
        fn cursor_outside_every_window_wants_no_capture() {
            let mut ctx = test_context();
            frame(&mut ctx, input_at([5000.0, 5000.0]), |ui| {
                ui.window("W", ScrollFlags::NONE, |_| {});
            });
            assert!(!ctx.wants_mouse_capture());
            assert!(!ctx.wants_keyboard_capture());
        }

        #[test]
        fn an_active_drag_wants_the_mouse_even_outside_the_window() {
            let mut ctx = test_context();
            let mut value = 0.5_f32;
            fn declare(ui: &mut Ui, value: &mut f32) {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    ui.slider_float("##s", value, 0.0..=1.0);
                });
            }
            let mut track = [0.0; 4];
            frame(&mut ctx, input_at(AWAY), |ui| {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    let pos = ui.ctx.current_window_mut().cursor.next_pos();
                    let width = ui.ctx.current_window_mut().cursor.available_width();
                    let h = ui.ctx.font.line_height() + 2.0 * ui.ctx.style.frame_padding[1];
                    track = [pos[0], pos[1], pos[0] + width, pos[1] + h];
                    ui.slider_float("##s", &mut value, 0.0..=1.0);
                });
            });
            let grab = [track[0] + 5.0, (track[1] + track[3]) / 2.0];
            frame(&mut ctx, click_at(grab, true), |ui| declare(ui, &mut value));
            assert!(ctx.wants_mouse_capture());
            let outside = [5000.0, 5000.0];
            frame(&mut ctx, click_at(outside, true), |ui| {
                declare(ui, &mut value)
            });
            assert!(ctx.active.is_some(), "the slider must still hold the drag");
            assert!(
                ctx.hover_top_window.is_none(),
                "the drag point is over no window"
            );
            assert!(ctx.wants_mouse_capture());
            frame(&mut ctx, click_at(outside, false), |ui| {
                declare(ui, &mut value)
            });
            assert!(!ctx.wants_mouse_capture());
        }

        #[test]
        fn an_open_combo_popup_wants_the_mouse_over_its_overhang() {
            let mut ctx = test_context();
            let mut selected = 0usize;
            const ITEMS: [&str; 4] = ["alpha", "beta", "gamma", "delta"];
            fn declare(ui: &mut Ui, selected: &mut usize) {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    for _ in 0..14 {
                        ui.label("filler line");
                    }
                    ui.combo("##c", &ITEMS, selected);
                });
            }
            let mut button = [0.0; 2];
            let mut row_h = 0.0;
            frame(&mut ctx, input_at(AWAY), |ui| {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    for _ in 0..14 {
                        ui.label("filler line");
                    }
                    let pos = ui.ctx.current_window_mut().cursor.next_pos();
                    row_h = ui.ctx.font.line_height() + 2.0 * ui.ctx.style.frame_padding[1];
                    button = pos;
                    ui.combo("##c", &ITEMS, &mut selected);
                });
            });
            let at = [button[0] + 2.0, button[1] + row_h / 2.0];
            frame(&mut ctx, click_at(at, true), |ui| {
                declare(ui, &mut selected)
            });
            frame(&mut ctx, click_at(at, false), |ui| {
                declare(ui, &mut selected)
            });
            assert!(
                ctx.open_popup.is_some(),
                "the click must open the combo popup"
            );
            frame(&mut ctx, input_at(AWAY), |ui| declare(ui, &mut selected));
            let popup = ctx.popup_rect.expect("popup rect recorded");
            let win = ctx.windows[0].rect();
            assert!(
                popup[3] > win[3],
                "test setup: the list must overhang the window ({popup:?} vs {win:?})"
            );
            let overhang = [popup[0] + 2.0, win[3] + 2.0];
            assert!(point_in_rect(overhang, popup));
            assert!(!point_in_rect(overhang, win));
            frame(&mut ctx, input_at(overhang), |ui| {
                declare(ui, &mut selected)
            });
            assert!(ctx.hover_top_window.is_none());
            assert!(ctx.wants_mouse_capture());
            assert!(!ctx.wants_keyboard_capture());
        }

        #[test]
        fn a_focused_text_input_wants_the_keyboard_until_defocused() {
            let mut ctx = test_context();
            let mut buf = String::new();
            fn declare(ui: &mut Ui, buf: &mut String) {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    ui.text_input("##t", buf);
                });
            }
            let mut field = [0.0; 4];
            frame(&mut ctx, input_at(AWAY), |ui| {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    let pos = ui.ctx.current_window_mut().cursor.next_pos();
                    let width = ui.ctx.current_window_mut().cursor.available_width();
                    let h = ui.ctx.font.line_height() + 2.0 * ui.ctx.style.frame_padding[1];
                    field = [pos[0], pos[1], pos[0] + width, pos[1] + h];
                    ui.text_input("##t", &mut buf);
                });
            });
            let at = [field[0] + 5.0, (field[1] + field[3]) / 2.0];
            frame(&mut ctx, click_at(at, true), |ui| declare(ui, &mut buf));
            frame(&mut ctx, click_at(at, false), |ui| declare(ui, &mut buf));
            frame(&mut ctx, input_at(AWAY), |ui| declare(ui, &mut buf));
            assert!(ctx.wants_keyboard_capture());
            let escape = InputState {
                keys: &[KeyEvent::Press(Key::Escape)],
                ..input_at(AWAY)
            };
            frame(&mut ctx, escape, |ui| declare(ui, &mut buf));
            assert!(!ctx.wants_keyboard_capture());
            assert!(!ctx.wants_mouse_capture());
        }

        #[test]
        fn an_armed_keybind_wants_the_keyboard() {
            let mut ctx = test_context();
            let mut binding = KeyBinding::None;
            fn declare(ui: &mut Ui, binding: &mut KeyBinding) {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    ui.keybind("##k", binding);
                });
            }
            let mut button = [0.0; 2];
            let mut row_h = 0.0;
            frame(&mut ctx, input_at(AWAY), |ui| {
                ui.window("W", ScrollFlags::NONE, |ui| {
                    let pos = ui.ctx.current_window_mut().cursor.next_pos();
                    row_h = ui.ctx.font.line_height() + 2.0 * ui.ctx.style.frame_padding[1];
                    button = pos;
                    ui.keybind("##k", &mut binding);
                });
            });
            let at = [button[0] + 2.0, button[1] + row_h / 2.0];
            frame(&mut ctx, click_at(at, true), |ui| declare(ui, &mut binding));
            frame(&mut ctx, click_at(at, false), |ui| {
                declare(ui, &mut binding)
            });
            frame(&mut ctx, input_at(AWAY), |ui| declare(ui, &mut binding));
            assert!(
                matches!(
                    ctx.widget_states.values().next(),
                    Some(crate::context::WidgetState::Keybind { armed: true })
                ),
                "the click must arm the keybind"
            );
            assert!(ctx.wants_keyboard_capture());
        }

        #[test]
        fn an_armed_keybind_without_the_capture_still_wants_the_keyboard() {
            let mut ctx = test_context();
            let mut binding = KeyBinding::None;
            fn declare(ui: &mut Ui, binding: &mut KeyBinding) {
                ui.window("W1", ScrollFlags::NONE, |ui| {
                    ui.keybind("##k", binding);
                });
                ui.window("W2", ScrollFlags::NONE, |_| {});
            }
            let mut keybind_at = [0.0; 2];
            frame(&mut ctx, input_at(AWAY), |ui| {
                ui.window("W1", ScrollFlags::NONE, |ui| {
                    let pos = ui.ctx.current_window_mut().cursor.next_pos();
                    let h = ui.ctx.font.line_height() + 2.0 * ui.ctx.style.frame_padding[1];
                    keybind_at = [pos[0] + 2.0, pos[1] + h / 2.0];
                    ui.keybind("##k", &mut binding);
                });
                ui.window("W2", ScrollFlags::NONE, |_| {});
            });
            let w1 = ctx.windows[0].rect();
            let w2 = ctx.windows[1].rect();
            assert!(
                !point_in_rect(keybind_at, w2),
                "test setup: arming click must miss W2"
            );
            frame(&mut ctx, click_at(keybind_at, true), |ui| {
                declare(ui, &mut binding)
            });
            frame(&mut ctx, click_at(keybind_at, false), |ui| {
                declare(ui, &mut binding)
            });
            assert!(ctx.wants_keyboard_capture());
            let title_h = ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1];
            let grab = [w1[2] + 12.0, w2[1] + title_h / 2.0];
            assert!(point_in_rect(grab, w2) && !point_in_rect(grab, w1));
            frame(&mut ctx, click_at(grab, true), |ui| {
                declare(ui, &mut binding)
            });
            frame(&mut ctx, click_at(grab, false), |ui| {
                declare(ui, &mut binding)
            });
            frame(&mut ctx, input_at(AWAY), |ui| declare(ui, &mut binding));
            assert!(
                ctx.active.is_none(),
                "the released drag must leave no capture"
            );
            assert!(
                ctx.widget_states
                    .values()
                    .any(|s| matches!(s, crate::context::WidgetState::Keybind { armed: true })),
                "the keybind must still be armed"
            );
            assert!(ctx.wants_keyboard_capture());
            assert!(!ctx.wants_mouse_capture());
        }
    }
}
