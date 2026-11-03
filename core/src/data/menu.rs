use super::{click, DataError};
use crate::{context::WidgetState, id::Id, ui::Ui};

pub struct MenuBarScope {
    rect: [f32; 4],
    next_x: f32,
    id: Id,
    base: usize,
}

fn popup_rect(ui: &Ui<'_>, anchor: [f32; 2], size: [f32; 2]) -> [f32; 4] {
    let viewport = ui.ctx.input.viewport_size;
    let mut point = anchor;
    for axis in 0..2 {
        if viewport[axis] > 0. {
            point[axis] = point[axis]
                .min((viewport[axis] - size[axis]).max(0.))
                .max(0.);
        }
    }
    [point[0], point[1], point[0] + size[0], point[1] + size[1]]
}
fn valid_size(size: [f32; 2]) -> bool {
    size.iter()
        .all(|v| v.is_finite() && *v >= 32. && *v <= 10_000.)
}
impl Ui<'_> {
    pub fn begin_menu_bar(&mut self, label: &str) -> MenuBarScope {
        let height = self.ctx.font.line_height_in(self.ctx.active_font())
            + 2. * self.ctx.style.frame_padding[1];
        let width = crate::widgets::available_width(self.ctx);
        let rect = self.allocate([width, height]);
        let id = self.widget_id(label);
        self.ctx.id_stack.push_id(id);
        let base = if self.ctx.routing.depth == 0 {
            0
        } else {
            self.ctx.routing.scopes[self.ctx.routing.depth - 1].index + 1
        };
        let color = self.ctx.style.widget_bg;
        crate::widgets::draw_parts(self.ctx).1.add_filled_rect(
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            color,
        );
        MenuBarScope {
            rect,
            next_x: rect[0],
            id,
            base,
        }
    }
    pub fn begin_menu(
        &mut self,
        bar: &mut MenuBarScope,
        label: &str,
        size: [f32; 2],
    ) -> Result<bool, DataError> {
        if !valid_size(size) {
            return Err(DataError::InvalidInput);
        }
        let style = self.ctx.style;
        let text = crate::id::split_label(label).0;
        let width =
            self.measure_text(text)[0] + 2. * style.frame_padding[0] + style.item_spacing[0];
        let rect = [
            bar.next_x.min(bar.rect[2]),
            bar.rect[1],
            (bar.next_x + width).min(bar.rect[2]),
            bar.rect[3],
        ];
        bar.next_x += width;
        let id = self.widget_id(label);
        let previous = match self.ctx.widget_states.get(&bar.id) {
            Some(WidgetState::MenuBar { open }) => *open,
            _ => None,
        };
        let own_open = previous.is_some_and(|root| {
            self.ctx.routing.popups[..self.ctx.routing.count]
                .iter()
                .any(|p| p.id == Some(root))
        });
        let hover = self.is_hovered(rect);
        let activated = click(self, id, rect);
        if activated || own_open && hover && previous != Some(id) {
            if activated && self.is_popup_open(id) {
                while self.ctx.routing.count > bar.base {
                    self.close_popup();
                }
            } else {
                self.open_popup(id);
                self.ctx
                    .widget_states
                    .insert(bar.id, WidgetState::MenuBar { open: Some(id) });
            }
        }
        let color = if self.is_popup_open(id) {
            style.widget_bg_active
        } else if hover {
            style.widget_bg_hovered
        } else {
            style.widget_bg
        };
        let color = self.ctx.widget_color(id, color);
        crate::widgets::draw_parts(self.ctx).1.add_filled_rect(
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            color,
        );
        crate::widgets::draw_parts(self.ctx).1.push_clip_rect(rect);
        self.draw_text_at(
            [
                rect[0] + style.frame_padding[0],
                rect[1] + style.frame_padding[1],
            ],
            text,
            style.text,
        );
        crate::widgets::draw_parts(self.ctx).1.pop_clip_rect();
        Ok(self.begin_popup(id, popup_rect(self, [rect[0], rect[3]], size)))
    }
    pub fn begin_submenu(&mut self, label: &str, size: [f32; 2]) -> Result<bool, DataError> {
        if !valid_size(size) || !self.ctx.popup_scope_active {
            return Err(DataError::InvalidInput);
        }
        let id = self.widget_id(label);
        let height = self.ctx.font.line_height_in(self.ctx.active_font())
            + 2. * self.ctx.style.frame_padding[1];
        let width = crate::widgets::available_width(self.ctx);
        let rect = self.allocate([width, height]);
        let style = self.ctx.style;
        if click(self, id, rect) {
            self.open_popup(id);
        }
        let color = if self.is_popup_open(id) {
            style.widget_bg_active
        } else if self.ctx.hot == Some(id) {
            style.widget_bg_hovered
        } else {
            style.window_bg
        };
        let color = self.ctx.widget_color(id, color);
        crate::widgets::draw_parts(self.ctx).1.add_filled_rect(
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            color,
        );
        self.draw_text_at(
            [
                rect[0] + style.frame_padding[0],
                rect[1] + style.frame_padding[1],
            ],
            crate::id::split_label(label).0,
            style.text,
        );
        self.draw_text_at(
            [rect[2] - 18., rect[1] + style.frame_padding[1]],
            ">",
            style.text,
        );
        Ok(self.begin_popup(id, popup_rect(self, [rect[2], rect[1]], size)))
    }
    pub fn menu_item(
        &mut self,
        bar: &MenuBarScope,
        label: &str,
        hint: &str,
        enabled: bool,
    ) -> bool {
        assert!(self.ctx.popup_scope_active, "menu item requires a popup");
        let disabled = self.begin_disabled("##menu_disabled", !enabled);
        let height = self.ctx.font.line_height_in(self.ctx.active_font())
            + 2. * self.ctx.style.frame_padding[1];
        let width = crate::widgets::available_width(self.ctx);
        let rect = self.allocate([width, height]);
        let id = self.widget_id(label);
        let style = self.ctx.style;
        let clicked = click(self, id, rect);
        let color = if self.is_captured(id) {
            style.widget_bg_active
        } else if self.ctx.hot == Some(id) {
            style.widget_bg_hovered
        } else {
            style.window_bg
        };
        let color = self.ctx.widget_color(id, color);
        crate::widgets::draw_parts(self.ctx).1.add_filled_rect(
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            color,
        );
        let hint_width = self.measure_text(hint)[0];
        self.draw_text_at(
            [
                rect[0] + style.frame_padding[0],
                rect[1] + style.frame_padding[1],
            ],
            crate::id::split_label(label).0,
            style.text,
        );
        self.draw_text_at(
            [
                rect[2] - hint_width - style.frame_padding[0],
                rect[1] + style.frame_padding[1],
            ],
            hint,
            style.text,
        );
        self.end_disabled(disabled);
        if clicked {
            while self.ctx.routing.count > bar.base {
                self.close_popup();
            }
        }
        clicked
    }
    pub fn end_menu(&mut self) {
        self.end_popup();
    }
    pub fn end_menu_bar(&mut self, _bar: MenuBarScope) {
        self.ctx.id_stack.pop();
    }
    pub fn menu_bar(&mut self, label: &str, f: impl FnOnce(&mut Ui, &mut MenuBarScope)) {
        let mut bar = self.begin_menu_bar(label);
        f(self, &mut bar);
        self.end_menu_bar(bar);
    }
    pub fn menu(
        &mut self,
        bar: &mut MenuBarScope,
        label: &str,
        size: [f32; 2],
        f: impl FnOnce(&mut Ui, &MenuBarScope),
    ) -> Result<(), DataError> {
        if self.begin_menu(bar, label, size)? {
            f(self, bar);
            self.end_menu();
        }
        Ok(())
    }
    pub fn submenu(
        &mut self,
        label: &str,
        size: [f32; 2],
        f: impl FnOnce(&mut Ui),
    ) -> Result<(), DataError> {
        if self.begin_submenu(label, size)? {
            f(self);
            self.end_menu();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{context::Context, input::InputState, ScrollFlags};
    const FONT: &[u8] = include_bytes!("../../tests/assets/DejaVuSans.ttf");

    #[test]
    fn root_menu_click_hover_switch_and_leaf_dismissal_balance_scopes() {
        let mut ctx = Context::new(FONT).unwrap();
        ctx.set_window_rect("W", [0., 0.], [500., 400.]);
        let mut header = [0.; 4];
        let mut leaf = [0.; 4];
        let mut actions = 0;
        let mut frame = |ctx: &mut Context, point: [f32; 2], down: bool| {
            ctx.set_input(InputState {
                mouse_pos: point,
                mouse_down: [down, false, false, false, false],
                ..InputState::default()
            });
            ctx.new_frame(0.016);
            ctx.ui().window("W", ScrollFlags::NONE, |ui| {
                ui.menu_bar("bar", |ui, bar| {
                    header = bar.rect;
                    ui.menu(bar, "File", [180., 100.], |ui, bar| {
                        if ui.menu_item(bar, "Action", "", true) {
                            actions += 1;
                        }
                        leaf = ui.ctx.last_item.unwrap().1;
                    })
                    .unwrap();
                    ui.menu(bar, "About", [180., 100.], |ui, bar| {
                        if ui.menu_item(bar, "Action", "", true) {
                            actions += 1;
                        }
                        leaf = ui.ctx.last_item.unwrap().1;
                    })
                    .unwrap();
                });
            });
            ctx.render();
            assert_eq!(ctx.routing.depth, 0);
            (header, leaf)
        };
        let (bar, _) = frame(&mut ctx, [0., 0.], false);
        let file = [bar[0] + 10., (bar[1] + bar[3]) * 0.5];
        frame(&mut ctx, file, true);
        frame(&mut ctx, file, false);
        assert_eq!(ctx.routing.count, 1);
        let file_id = ctx.open_popup;
        let about = [bar[0] + 80., file[1]];
        let (_, item) = frame(&mut ctx, about, false);
        assert_eq!(ctx.routing.count, 1);
        assert_ne!(ctx.open_popup, file_id);
        let point = [(item[0] + item[2]) * 0.5, (item[1] + item[3]) * 0.5];
        frame(&mut ctx, point, true);
        frame(&mut ctx, point, false);
        assert_eq!(ctx.routing.count, 0);
        assert_eq!(actions, 1);
    }
}
