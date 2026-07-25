#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::ops::RangeInclusive;

use restricted_gui::context::Context;
use restricted_gui::ui::{CollapsingHeaderScope, ScrollRegionScope};

pub use restricted_gui::draw::{DrawCmd, DrawData, DrawVert, TextureId};
pub use restricted_gui::font::{Font, FontError, IconFont};
pub use restricted_gui::icon;
pub use restricted_gui::id::Id;
pub use restricted_gui::input::{InputState, Key, KeyBinding, KeyEvent, Modifiers};
pub use restricted_gui::style::Style;

enum Scope {
    CollapsingHeader(Option<CollapsingHeaderScope>),
    ScrollRegion(Option<ScrollRegionScope>),
    TabBar(Option<()>),
}

impl Scope {
    fn name(&self) -> &'static str {
        match self {
            Scope::CollapsingHeader(_) => "collapsing header",
            Scope::ScrollRegion(_) => "scroll region",
            Scope::TabBar(_) => "tab bar",
        }
    }
}

pub struct Gui {
    ctx: Context,
    window_open: bool,
    closed_window_pending: bool,
    scopes: Vec<Scope>,
}

impl Gui {
    pub fn new(font_bytes: &[u8]) -> Result<Gui, FontError> {
        Ok(Self::with_context(Context::new(font_bytes)?))
    }

    pub fn new_with_icons(font_bytes: &[u8], icons: &[IconFont<'_>]) -> Result<Gui, FontError> {
        Ok(Self::with_context(Context::new_with_icons(
            font_bytes, icons,
        )?))
    }

    pub fn with_font(font: Font) -> Gui {
        Self::with_context(Context::with_font(font))
    }

    fn with_context(ctx: Context) -> Gui {
        Gui {
            ctx,
            window_open: false,
            closed_window_pending: false,
            scopes: Vec::new(),
        }
    }

    fn emitting(&self) -> bool {
        if self.closed_window_pending {
            return false;
        }
        !matches!(
            self.scopes.last(),
            Some(Scope::CollapsingHeader(None))
                | Some(Scope::ScrollRegion(None))
                | Some(Scope::TabBar(None))
        )
    }

    pub fn style(&mut self) -> &mut Style {
        &mut self.ctx.style
    }

    pub fn set_window_open(&mut self, title: &str, open: bool) {
        self.ctx.set_window_open(title, open);
    }

    // Every method here forwards verbatim to the core. This crate is a receptionist with a CS degree.
    pub fn new_frame(&mut self, input: InputState, dt: f32) {
        self.ctx.set_input(input);
        self.ctx.new_frame(dt);
    }

    pub fn render(&mut self) -> DrawData<'_> {
        assert!(
            !self.window_open && !self.closed_window_pending && self.scopes.is_empty(),
            "Gui::render called with open scopes: every begin_* must be \
             closed by its end_* before render"
        );
        self.ctx.render()
    }

    pub fn atlas_pixels(&self) -> (&[u8], u32, u32) {
        self.ctx.font().atlas_pixels()
    }

    pub fn atlas_generation(&self) -> u64 {
        self.ctx.font().atlas_generation()
    }

    pub fn begin_window(&mut self, title: &str) {
        assert!(
            self.scopes.is_empty(),
            "Gui::begin_window called while a container scope is open: \
             close every container before beginning a window"
        );
        assert!(
            !self.window_open && !self.closed_window_pending,
            "Gui::begin_window called while a window scope is open or \
             unended: nested windows are not supported"
        );
        if self.ctx.ui().begin_window(title) {
            self.window_open = true;
        } else {
            self.closed_window_pending = true;
        }
    }

    pub fn end_window(&mut self) {
        assert!(
            self.scopes.is_empty(),
            "Gui::end_window called while a container scope is open: close \
             every container before ending the window"
        );
        if self.window_open {
            self.ctx.ui().end_window();
            self.window_open = false;
        } else if self.closed_window_pending {
            self.closed_window_pending = false;
        } else {
            panic!(
                "Gui::end_window called with no window open: every \
                 end_window must follow a begin_window"
            );
        }
    }

    pub fn label(&mut self, text: &str) {
        if self.emitting() {
            self.ctx.ui().label(text);
        }
    }

    pub fn button(&mut self, label: &str) -> bool {
        self.emitting() && self.ctx.ui().button(label)
    }

    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> bool {
        self.emitting() && self.ctx.ui().checkbox(label, value)
    }

    pub fn slider_float(
        &mut self,
        label: &str,
        value: &mut f32,
        range: RangeInclusive<f32>,
    ) -> bool {
        self.emitting() && self.ctx.ui().slider_float(label, value, range)
    }

    pub fn slider_int(&mut self, label: &str, value: &mut i32, range: RangeInclusive<i32>) -> bool {
        self.emitting() && self.ctx.ui().slider_int(label, value, range)
    }

    pub fn text_input(&mut self, label: &str, buf: &mut String) -> bool {
        self.emitting() && self.ctx.ui().text_input(label, buf)
    }

    pub fn same_line(&mut self) {
        if self.emitting() {
            self.ctx.ui().same_line();
        }
    }

    pub fn combo(&mut self, label: &str, items: &[&str], selected: &mut usize) -> bool {
        self.emitting() && self.ctx.ui().combo(label, items, selected)
    }

    pub fn color_edit_rgba(&mut self, label: &str, value: &mut [u8; 4]) -> bool {
        self.emitting() && self.ctx.ui().color_edit_rgba(label, value)
    }

    pub fn color_picker_rgba(&mut self, label: &str, value: &mut [u8; 4]) -> bool {
        self.emitting() && self.ctx.ui().color_picker_rgba(label, value)
    }

    pub fn keybind(&mut self, label: &str, binding: &mut KeyBinding) -> bool {
        self.emitting() && self.ctx.ui().keybind(label, binding)
    }

    pub fn tooltip_text(&mut self, text: &str) {
        if self.emitting() {
            self.ctx.ui().tooltip_text(text);
        }
    }

    pub fn begin_collapsing_header(&mut self, title: &str) {
        let scope = if self.emitting() {
            self.ctx.ui().begin_collapsing_header(title)
        } else {
            None
        };
        self.scopes.push(Scope::CollapsingHeader(scope));
    }

    pub fn end_collapsing_header(&mut self) {
        match self.scopes.pop() {
            Some(Scope::CollapsingHeader(scope)) => {
                if let Some(scope) = scope {
                    self.ctx.ui().end_collapsing_header(scope);
                }
            }
            Some(other) => panic!(
                "Gui::end_collapsing_header called while the innermost open \
                 scope is a {}",
                other.name()
            ),
            None => panic!(
                "Gui::end_collapsing_header called with no collapsing \
                 header open: every end must follow its begin"
            ),
        }
    }

    pub fn begin_scroll_region(&mut self, label: &str, size: [f32; 2]) {
        let scope = if self.emitting() {
            Some(self.ctx.ui().begin_scroll_region(label, size))
        } else {
            None
        };
        self.scopes.push(Scope::ScrollRegion(scope));
    }

    pub fn end_scroll_region(&mut self) {
        match self.scopes.pop() {
            Some(Scope::ScrollRegion(scope)) => {
                if let Some(scope) = scope {
                    self.ctx.ui().end_scroll_region(scope);
                }
            }
            Some(other) => panic!(
                "Gui::end_scroll_region called while the innermost open \
                 scope is a {}",
                other.name()
            ),
            None => panic!(
                "Gui::end_scroll_region called with no scroll region open: \
                 every end must follow its begin"
            ),
        }
    }

    pub fn begin_tab_bar(&mut self, label: &str) {
        if self.emitting() {
            self.ctx.ui().begin_tab_bar(label);
            self.scopes.push(Scope::TabBar(Some(())));
        } else {
            self.scopes.push(Scope::TabBar(None));
        }
    }

    pub fn tab(&mut self, title: &str) -> bool {
        self.emitting() && self.ctx.ui().tab(title)
    }

    pub fn end_tab_bar(&mut self) {
        match self.scopes.pop() {
            Some(Scope::TabBar(scope)) => {
                if scope.is_some() {
                    self.ctx.ui().end_tab_bar();
                }
            }
            Some(other) => panic!(
                "Gui::end_tab_bar called while the innermost open scope is \
                 a {}",
                other.name()
            ),
            None => panic!(
                "Gui::end_tab_bar called with no tab bar open: every end \
                 must follow its begin"
            ),
        }
    }

    pub fn draw_line(&mut self, p0: [f32; 2], p1: [f32; 2], col: [u8; 4], thickness: f32) {
        if self.emitting() {
            self.ctx.ui().draw_line(p0, p1, col, thickness);
        }
    }

    pub fn draw_rect_filled(&mut self, min: [f32; 2], max: [f32; 2], col: [u8; 4]) {
        if self.emitting() {
            self.ctx.ui().draw_rect_filled(min, max, col);
        }
    }

    pub fn draw_rect_outline(
        &mut self,
        min: [f32; 2],
        max: [f32; 2],
        col: [u8; 4],
        thickness: f32,
    ) {
        if self.emitting() {
            self.ctx.ui().draw_rect_outline(min, max, col, thickness);
        }
    }

    pub fn draw_circle_filled(&mut self, center: [f32; 2], radius: f32, col: [u8; 4]) {
        if self.emitting() {
            self.ctx.ui().draw_circle_filled(center, radius, col);
        }
    }

    pub fn draw_circle_outline(
        &mut self,
        center: [f32; 2],
        radius: f32,
        col: [u8; 4],
        thickness: f32,
    ) {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_circle_outline(center, radius, col, thickness);
        }
    }

    pub fn draw_arc(
        &mut self,
        center: [f32; 2],
        radius: f32,
        angle_range: RangeInclusive<f32>,
        col: [u8; 4],
        thickness: f32,
    ) {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_arc(center, radius, angle_range, col, thickness);
        }
    }

    pub fn draw_polygon_filled(&mut self, points: &[[f32; 2]], col: [u8; 4]) {
        if self.emitting() {
            self.ctx.ui().draw_polygon_filled(points, col);
        }
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
        if self.emitting() {
            self.ctx
                .ui()
                .draw_image(texture, min, max, uv_min, uv_max, tint);
        }
    }

    pub fn draw_text_at(&mut self, pos: [f32; 2], text: &str, col: [u8; 4]) {
        if self.emitting() {
            self.ctx.ui().draw_text_at(pos, text, col);
        }
    }

    pub fn measure_text(&mut self, text: &str) -> [f32; 2] {
        self.ctx.ui().measure_text(text)
    }

    pub fn widget_id(&mut self, label: &str) -> Id {
        self.ctx.ui().widget_id(label)
    }

    pub fn allocate(&mut self, size: [f32; 2]) -> [f32; 4] {
        if self.emitting() {
            self.ctx.ui().allocate(size)
        } else {
            [0.0; 4]
        }
    }

    pub fn is_hovered(&mut self, rect: [f32; 4]) -> bool {
        self.ctx.ui().is_hovered(rect)
    }

    pub fn capture(&mut self, id: Id) {
        if self.emitting() {
            self.ctx.ui().capture(id);
        }
    }

    pub fn release_capture(&mut self, id: Id) {
        if self.emitting() {
            self.ctx.ui().release_capture(id);
        }
    }

    pub fn is_captured(&mut self, id: Id) -> bool {
        self.ctx.ui().is_captured(id)
    }

    pub fn mouse_pos(&mut self) -> [f32; 2] {
        self.ctx.ui().mouse_pos()
    }

    pub fn mouse_pressed(&mut self, button: usize) -> bool {
        self.ctx.ui().mouse_pressed(button)
    }

    pub fn mouse_down(&mut self, button: usize) -> bool {
        self.ctx.ui().mouse_down(button)
    }

    pub fn mouse_released(&mut self, button: usize) -> bool {
        self.ctx.ui().mouse_released(button)
    }
}
