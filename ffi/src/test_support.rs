use std::ptr;

use restricted_gui::context::Context;

use crate::{gui_context_new, gui_new_frame, gui_render, GuiContext, GuiInput, GuiKeyEvent};

pub(crate) const DEJAVU_SANS: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

pub(crate) const AWAY: [f32; 2] = [-1000.0, -1000.0];

pub(crate) const WINDOW_POS: [f32; 2] = [60.0, 60.0];
pub(crate) const WINDOW_SIZE: [f32; 2] = [300.0, 400.0];

// Predicts widget rects from public Style/Font metrics so tests can assert exact pixels. A test that knows where every pixel lives is a test that fears nothing.
pub(crate) fn new_context() -> *mut GuiContext {
    let ctx = unsafe { gui_context_new(DEJAVU_SANS.as_ptr(), DEJAVU_SANS.len()) };
    assert!(!ctx.is_null(), "valid font must produce a context");
    ctx
}

pub(crate) fn input_at(mouse_pos: [f32; 2], left_down: bool) -> GuiInput {
    GuiInput {
        mouse_pos,
        mouse_down: [left_down as u8, 0, 0],
        mouse_wheel: 0.0,
        viewport_size: [800.0, 600.0],
        keys: ptr::null(),
        key_count: 0,
    }
}

pub(crate) fn frame(ctx: *mut GuiContext, input: &GuiInput, declare: impl FnOnce(*mut GuiContext)) {
    unsafe { gui_new_frame(ctx, input, 0.016) };
    declare(ctx);
    unsafe { gui_render(ctx) };
}

pub(crate) fn window_frame(
    ctx: *mut GuiContext,
    mouse_pos: [f32; 2],
    left_down: bool,
    content: impl FnOnce(*mut GuiContext),
) {
    let input = input_at(mouse_pos, left_down);
    frame(ctx, &input, |ctx| {
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) });
        content(ctx);
        unsafe { crate::gui_end_window(ctx) };
    });
}

pub(crate) fn window_frame_keys(
    ctx: *mut GuiContext,
    mouse_pos: [f32; 2],
    keys: &[GuiKeyEvent],
    content: impl FnOnce(*mut GuiContext),
) {
    let mut input = input_at(mouse_pos, false);
    input.keys = keys.as_ptr();
    input.key_count = keys.len();
    frame(ctx, &input, |ctx| {
        assert!(unsafe { crate::gui_begin_window(ctx, b"W".as_ptr(), 1) });
        content(ctx);
        unsafe { crate::gui_end_window(ctx) };
    });
}

pub(crate) fn center(rect: [f32; 4]) -> [f32; 2] {
    [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
}

pub(crate) struct Metrics {
    font: restricted_gui::font::Font,
    pub line_height: f32,
    pub frame_padding: [f32; 2],
    pub window_padding: [f32; 2],
    pub item_spacing: [f32; 2],
}

impl Metrics {
    pub fn new() -> Self {
        let ctx = Context::new(DEJAVU_SANS).expect("reference context");
        let font = restricted_gui::font::Font::new(
            DEJAVU_SANS,
            restricted_gui::context::DEFAULT_FONT_SIZE,
        )
        .expect("DejaVu Sans must load");
        Self {
            line_height: ctx.font().line_height(),
            frame_padding: ctx.style.frame_padding,
            window_padding: ctx.style.window_padding,
            item_spacing: ctx.style.item_spacing,
            font,
        }
    }

    pub fn measure(&mut self, text: &str) -> [f32; 2] {
        self.font.measure(text)
    }

    pub fn row_h(&self) -> f32 {
        self.line_height + 2.0 * self.frame_padding[1]
    }

    pub fn title_bar_h(&self) -> f32 {
        self.row_h()
    }

    pub fn content_origin(&self) -> [f32; 2] {
        [
            WINDOW_POS[0] + self.window_padding[0],
            WINDOW_POS[1] + self.title_bar_h() + self.window_padding[1],
        ]
    }

    pub fn content_right(&self) -> f32 {
        WINDOW_POS[0] + WINDOW_SIZE[0] - self.window_padding[0]
    }

    pub fn text_widget_rect(&mut self, display: &str) -> [f32; 4] {
        let pos = self.content_origin();
        let text = self.measure(display);
        [
            pos[0],
            pos[1],
            pos[0] + text[0] + 2.0 * self.frame_padding[0],
            pos[1] + text[1] + 2.0 * self.frame_padding[1],
        ]
    }

    pub fn checkbox_box_rect(&self) -> [f32; 4] {
        let pos = self.content_origin();
        [
            pos[0],
            pos[1],
            pos[0] + self.line_height,
            pos[1] + self.line_height,
        ]
    }

    pub fn full_width_row_rect(&self) -> [f32; 4] {
        let pos = self.content_origin();
        [pos[0], pos[1], self.content_right(), pos[1] + self.row_h()]
    }

    pub fn slider_track_rect(&mut self, display: &str) -> [f32; 4] {
        let pos = self.content_origin();
        let available = self.content_right() - pos[0];
        let label_w = self.measure(display)[0];
        let track_w = (available - label_w - self.item_spacing[0]).max(0.0);
        [pos[0], pos[1], pos[0] + track_w, pos[1] + self.row_h()]
    }

    pub fn text_field_rect(&mut self, display: &str) -> [f32; 4] {
        self.slider_track_rect(display)
    }

    pub fn combo_button_rect(&mut self, current: &str) -> [f32; 4] {
        let pos = self.content_origin();
        let w = self.measure(current)[0] + self.line_height * 0.5 + 3.0 * self.frame_padding[0];
        [pos[0], pos[1], pos[0] + w, pos[1] + self.row_h()]
    }

    pub fn combo_row_rect(&mut self, button: [f32; 4], items: &[&str], i: usize) -> [f32; 4] {
        let button_w = button[2] - button[0];
        let widest = items
            .iter()
            .map(|item| self.measure(item)[0])
            .fold(0.0f32, f32::max);
        let popup_w =
            button_w.max(widest + 2.0 * self.frame_padding[0] + 2.0 * self.window_padding[0]);
        let row_h = self.row_h();
        let y = button[3] + self.window_padding[1] + i as f32 * (row_h + self.item_spacing[1]);
        [
            button[0] + self.window_padding[0],
            y,
            button[0] + popup_w - self.window_padding[0],
            y + row_h,
        ]
    }
}
