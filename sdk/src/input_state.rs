use crate::{Gui, Id, Navigation, Shortcut};
use tacita::persistence::{LayoutSnapshot, StateError};
impl Gui {
    pub fn navigate(&mut self, event: Navigation) -> bool {
        self.ctx.navigate(event)
    }
    pub fn popup_depth(&self) -> usize {
        self.ctx.popup_depth()
    }
    pub fn modal_depth(&self) -> usize {
        self.ctx.modal_depth()
    }
    pub fn set_double_click_settings(&mut self, seconds: f32, distance: f32) -> bool {
        self.ctx.set_double_click_settings(seconds, distance)
    }
    pub fn double_clicked(&mut self, id: Id, rect: [f32; 4], button: usize) -> bool {
        self.emitting() && self.ctx.ui().double_clicked(id, rect, button)
    }
    pub fn shortcut(&mut self, id: Id, chord: Shortcut) -> bool {
        self.emitting() && self.ctx.ui().shortcut(id, chord)
    }
    pub fn save_layout(&self) -> LayoutSnapshot {
        self.ctx.save_layout()
    }
    pub fn load_layout(&mut self, layout: &LayoutSnapshot) -> Result<(), StateError> {
        self.ctx.load_layout(layout)
    }
    pub fn load_theme(&mut self, data: &[u8]) -> Result<(), StateError> {
        self.ctx.load_theme(data)
    }
    pub fn save_theme(&self) -> Result<alloc::string::String, StateError> {
        self.ctx.save_theme()
    }
}
