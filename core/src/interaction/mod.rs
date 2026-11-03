use crate::{
    context::Context,
    draw::DrawList,
    id::Id,
    input::{Key, KeyEvent, Modifiers},
    layout::Cursor,
};
use alloc::vec::Vec;

pub const MAX_POPUPS: usize = 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Navigation {
    Next,
    Previous,
    Left,
    Right,
    Up,
    Down,
    Activate,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutScope {
    Global,
    Window(Id),
    Popup(Id),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub key: Key,
    pub modifiers: Modifiers,
    pub scope: ShortcutScope,
}

pub(crate) struct PopupSlot {
    pub id: Option<Id>,
    pub modal: bool,
    pub rect: Option<[f32; 4]>,
    pub focus: Option<Id>,
    pub seen: u64,
    pub draw: DrawList,
    pub motion: crate::animation::PopupAnimation,
}
pub(crate) struct PopupScope {
    pub id: Id,
    pub index: usize,
    pub cursor: Cursor,
    pub prev: [bool; 5],
    pub mouse: [f32; 2],
    pub down: [bool; 5],
    pub wheel: f32,
    pub clip: Option<[f32; 4]>,
    pub keys: Vec<KeyEvent>,
}
pub(crate) struct Routing {
    pub pending_popup: Option<(Id, bool, usize)>,
    pub popups: Vec<PopupSlot>,
    pub count: usize,
    pub scopes: Vec<PopupScope>,
    pub depth: usize,
    pub navigation: Vec<Navigation>,
    pub focus_regions: Vec<(Id, [f32; 4])>,
    pub focus_regions_prev: Vec<(Id, [f32; 4])>,
    pub focus_windows: Vec<(Id, Id)>,
    pub focus_windows_prev: Vec<(Id, Id)>,
    pub shortcuts: Vec<(Id, Shortcut)>,
    pub shortcuts_prev: Vec<(Id, Shortcut)>,
    pub initial_modifiers: Modifiers,
    pub pressed_owners: [Option<Id>; 5],
    pub triggered: Vec<Id>,
    pub clock: f64,
    pub click_interval: f64,
    pub click_distance: f32,
    pub clicks: [Option<(f64, [f32; 2], Id)>; 5],
    pub doubles: [Option<Id>; 5],
}
impl Routing {
    pub fn new() -> Self {
        Self {
            popups: (0..MAX_POPUPS)
                .map(|_| PopupSlot {
                    id: None,
                    modal: false,
                    rect: None,
                    focus: None,
                    seen: 0,
                    draw: DrawList::new(),
                    motion: crate::animation::PopupAnimation::new(),
                })
                .collect(),
            scopes: (0..MAX_POPUPS)
                .map(|_| PopupScope {
                    id: Id(0),
                    index: 0,
                    cursor: Cursor::new([0.; 2], 0., [0.; 2], [0.; 2]),
                    prev: [false; 5],
                    mouse: [0.; 2],
                    down: [false; 5],
                    wheel: 0.,
                    clip: None,
                    keys: Vec::new(),
                })
                .collect(),
            pending_popup: None,
            count: 0,
            depth: 0,
            navigation: Vec::new(),
            focus_regions: Vec::new(),
            focus_regions_prev: Vec::new(),
            focus_windows: Vec::new(),
            focus_windows_prev: Vec::new(),
            shortcuts: Vec::new(),
            shortcuts_prev: Vec::new(),
            initial_modifiers: Modifiers::default(),
            pressed_owners: [None; 5],
            triggered: Vec::new(),
            clock: 0.,
            click_interval: 0.3,
            click_distance: 4.,
            clicks: [None; 5],
            doubles: [None; 5],
        }
    }
    pub fn scope_id(&self) -> Option<Id> {
        (self.depth > 0).then(|| self.scopes[self.depth - 1].id)
    }
}
impl Context {
    pub fn navigate(&mut self, event: Navigation) -> bool {
        assert!(!self.frame_active);
        if self.routing.navigation.len() >= 64 {
            return false;
        }
        self.routing.navigation.push(event);
        true
    }
    pub fn popup_depth(&self) -> usize {
        self.routing.count
    }
    pub fn modal_depth(&self) -> usize {
        self.routing.popups[..self.routing.count]
            .iter()
            .filter(|p| p.modal)
            .count()
    }
    pub fn set_double_click_settings(&mut self, seconds: f32, distance: f32) -> bool {
        if !seconds.is_finite() || seconds <= 0. || !distance.is_finite() || distance < 0. {
            return false;
        }
        self.routing.click_interval = seconds as f64;
        self.routing.click_distance = distance;
        self.routing.clicks = [None; 5];
        true
    }
    pub(crate) fn popup_input_allowed(&self) -> bool {
        if self.routing.depth > 0 {
            self.routing.scope_id() == self.open_popup
        } else {
            self.open_popup.is_none()
        }
    }
    pub(crate) fn sync_popup_top(&mut self) {
        self.open_popup = if self.routing.count > 0 {
            self.routing.popups[self.routing.count - 1].id
        } else {
            None
        };
        self.popup_rect = if self.routing.count > 0 {
            self.routing.popups[self.routing.count - 1].rect
        } else {
            None
        };
        self.popup_modal = self.modal_depth() > 0;
    }
    pub(crate) fn dismiss_popup(&mut self) {
        if self.routing.count == 0 {
            return;
        }
        self.release_popup_capture();
        self.routing.count -= 1;
        self.focused = self.routing.popups[self.routing.count].focus;
        self.focused_in_popup = self.routing.count > 0;
        self.sync_popup_top();
        self.focused_in_modal = self.popup_modal;
        self.keyboard_activate = None;
        self.focus_order.clear();
        self.focus_order_prev.clear();
        self.routing.focus_regions.clear();
        self.routing.focus_regions_prev.clear();
        if self.routing.depth > 0 && !self.popup_input_allowed() {
            self.mask_popup_input();
        }
    }
    pub(crate) fn push_popup(&mut self, id: Id, modal: bool) {
        let base = self.popup_open_level();
        if self.routing.popups[..self.routing.count]
            .iter()
            .any(|p| p.id == Some(id))
        {
            return;
        }
        assert!(base < MAX_POPUPS, "popup depth exceeds eight");
        if self.routing.scopes[..self.routing.depth]
            .iter()
            .any(|scope| scope.index == base)
        {
            self.routing.pending_popup = Some((id, modal, base));
            return;
        }
        while self.routing.count > base {
            self.dismiss_popup();
        }
        let focus = self.focused;
        self.release_popup_capture();
        self.clear_active();
        let slot = &mut self.routing.popups[base];
        slot.id = Some(id);
        slot.modal = modal;
        slot.rect = None;
        slot.focus = focus;
        slot.seen = self.motion_frame;
        slot.draw.clear();
        slot.motion.invalidate();
        self.routing.count = base + 1;
        self.sync_popup_top();
        self.focused = None;
        self.focused_in_popup = false;
        self.focused_in_modal = false;
        self.focus_rect = None;
        self.focus_clip = None;
        self.keyboard_activate = None;
        self.focus_order.clear();
        self.focus_order_prev.clear();
        self.focus_visuals.clear();
        self.focus_rounding.clear();
        self.focus_mask_visuals.clear();
        self.routing.focus_regions.clear();
        self.routing.focus_regions_prev.clear();
        if self.routing.depth > 0 {
            self.mask_popup_input();
        }
    }
    fn popup_open_level(&self) -> usize {
        self.routing.scopes[..self.routing.depth]
            .iter()
            .rev()
            .find_map(|scope| {
                self.routing.popups[..self.routing.count]
                    .iter()
                    .position(|slot| slot.id == Some(scope.id))
                    .map(|i| i + 1)
            })
            .unwrap_or(0)
    }
    pub fn can_open_popup(&self, id: Id) -> bool {
        self.routing.popups[..self.routing.count]
            .iter()
            .any(|slot| slot.id == Some(id))
            || self.popup_open_level() < MAX_POPUPS
    }
    fn mask_popup_input(&mut self) {
        self.input.mouse_pos = [-f32::MAX; 2];
        self.input.mouse_down = [false; 5];
        self.input.mouse_wheel = 0.;
        self.input.keys.clear();
        self.prev_mouse_down = [true; 5];
    }
    pub(crate) fn route_input(&mut self) {
        let r = &mut self.routing;
        if self.dt.is_finite() && self.dt > 0. {
            r.clock += self.dt as f64;
        }
        r.doubles = [None; 5];
        r.pressed_owners = [None; 5];
        core::mem::swap(&mut r.focus_regions, &mut r.focus_regions_prev);
        r.focus_regions.clear();
        core::mem::swap(&mut r.focus_windows, &mut r.focus_windows_prev);
        r.focus_windows.clear();
        core::mem::swap(&mut r.shortcuts, &mut r.shortcuts_prev);
        r.shortcuts.clear();
        r.triggered.clear();
        let editing = self.active.is_some_and(|id| {
            matches!(
                self.widget_states.get(&id),
                Some(
                    crate::context::WidgetState::TextInput(_)
                        | crate::context::WidgetState::TextEdit
                        | crate::context::WidgetState::Keybind { armed: true }
                )
            )
        });
        if !editing {
            let mut modifiers = r.initial_modifiers;
            let mut kept = 0;
            for i in 0..self.input.keys.len() {
                let event = self.input.keys[i];
                match event {
                    KeyEvent::Press(Key::Ctrl) => modifiers.ctrl = true,
                    KeyEvent::Release(Key::Ctrl) => modifiers.ctrl = false,
                    KeyEvent::Press(Key::Shift) => modifiers.shift = true,
                    KeyEvent::Release(Key::Shift) => modifiers.shift = false,
                    KeyEvent::Press(Key::Alt) => modifiers.alt = true,
                    KeyEvent::Release(Key::Alt) => modifiers.alt = false,
                    _ => {}
                }
                if let KeyEvent::Press(key) = event {
                    let window = self.focused.and_then(|id| {
                        r.focus_windows_prev
                            .iter()
                            .find(|(widget, _)| *widget == id)
                            .map(|(_, w)| *w)
                    });
                    let winner = r
                        .shortcuts_prev
                        .iter()
                        .filter(|(_, s)| s.key == key && s.modifiers == modifiers)
                        .filter_map(|(id, s)| {
                            let priority = match s.scope {
                                ShortcutScope::Popup(p) if Some(p) == self.open_popup => 3,
                                ShortcutScope::Window(w)
                                    if self.open_popup.is_none() && Some(w) == window =>
                                {
                                    2
                                }
                                ShortcutScope::Global if self.open_popup.is_none() => 1,
                                _ => 0,
                            };
                            (priority > 0).then_some((*id, priority))
                        })
                        .max_by_key(|(_, priority)| *priority);
                    if let Some((id, _)) = winner {
                        if !r.triggered.contains(&id) {
                            r.triggered.push(id);
                        }
                        continue;
                    }
                }
                self.input.keys[kept] = event;
                kept += 1;
            }
            self.input.keys.truncate(kept);
        }
    }
    pub(crate) fn route_navigation(&mut self) {
        for i in 0..self.routing.navigation.len() {
            let editing = self.active.is_some_and(|id| {
                matches!(
                    self.widget_states.get(&id),
                    Some(
                        crate::context::WidgetState::TextInput(_)
                            | crate::context::WidgetState::TextEdit
                            | crate::context::WidgetState::Keybind { armed: true }
                    )
                )
            });
            let event = self.routing.navigation[i];
            match event {
                Navigation::Cancel => {
                    if self.open_popup.is_some() {
                        self.dismiss_popup();
                    } else {
                        self.clear_active();
                        self.focused = None;
                    }
                }
                Navigation::Activate if !editing => self.keyboard_activate = self.focused,
                Navigation::Next
                | Navigation::Previous
                | Navigation::Left
                | Navigation::Right
                | Navigation::Up
                | Navigation::Down
                    if !editing =>
                {
                    let regions = &self.routing.focus_regions_prev;
                    let current = self
                        .focused
                        .and_then(|id| regions.iter().position(|r| r.0 == id));
                    let next = if matches!(event, Navigation::Next | Navigation::Previous) {
                        if regions.is_empty() {
                            None
                        } else {
                            Some(current.map_or(
                                if event == Navigation::Previous {
                                    regions.len() - 1
                                } else {
                                    0
                                },
                                |i| {
                                    if event == Navigation::Previous {
                                        (i + regions.len() - 1) % regions.len()
                                    } else {
                                        (i + 1) % regions.len()
                                    }
                                },
                            ))
                        }
                    } else if let Some(current) = current {
                        let a = regions[current].1;
                        let center = [(a[0] + a[2]) * 0.5, (a[1] + a[3]) * 0.5];
                        let axis = if matches!(event, Navigation::Left | Navigation::Right) {
                            0
                        } else {
                            1
                        };
                        let sign = if matches!(event, Navigation::Left | Navigation::Up) {
                            -1.
                        } else {
                            1.
                        };
                        regions
                            .iter()
                            .enumerate()
                            .filter_map(|(i, (_, b))| {
                                let delta = [
                                    (b[0] + b[2]) * 0.5 - center[0],
                                    (b[1] + b[3]) * 0.5 - center[1],
                                ];
                                (delta[axis] * sign > 0.)
                                    .then_some((i, delta[axis].abs() + delta[1 - axis].abs() * 2.))
                            })
                            .min_by(|a, b| a.1.total_cmp(&b.1))
                            .map(|v| v.0)
                    } else {
                        (!regions.is_empty()).then_some(0)
                    };
                    if let Some(next) = next {
                        self.clear_active();
                        self.focused = Some(self.routing.focus_regions_prev[next].0);
                        self.focused_in_popup = self.open_popup.is_some();
                        self.focused_in_modal = self.popup_modal;
                    }
                }
                _ => {}
            }
        }
        self.routing.navigation.clear();
    }
}
impl crate::ui::Ui<'_> {
    pub fn shortcut(&mut self, id: Id, shortcut: Shortcut) -> bool {
        if self.ctx.disabled_depth > 0 || !self.ctx.popup_input_allowed() {
            return false;
        }
        self.ctx.routing.shortcuts.push((id, shortcut));
        if let Some(i) = self.ctx.routing.triggered.iter().position(|&v| v == id) {
            self.ctx.routing.triggered.remove(i);
            true
        } else {
            false
        }
    }
    pub fn double_clicked(&mut self, id: Id, rect: [f32; 4], button: usize) -> bool {
        if button >= 5 || self.ctx.disabled_depth > 0 || !self.ctx.popup_input_allowed() {
            return false;
        }
        if self.ctx.routing.doubles[button] == Some(id) {
            return true;
        }
        if self.ctx.routing.pressed_owners[button] == Some(id) {
            return false;
        }
        if !self.ctx.input.mouse_down[button]
            || self.ctx.prev_mouse_down[button]
            || !crate::ui::point_in_rect(self.ctx.input.mouse_pos, rect)
            || !crate::ui::hover_layer_gate(self.ctx)
        {
            return false;
        }
        let r = &mut self.ctx.routing;
        let pos = self.ctx.input.mouse_pos;
        r.pressed_owners[button] = Some(id);
        let double = r.clicks[button].is_some_and(|(t, p, owner)| {
            owner == id && r.clock - t <= r.click_interval && {
                let dx = f64::from(p[0]) - f64::from(pos[0]);
                let dy = f64::from(p[1]) - f64::from(pos[1]);
                dx * dx + dy * dy <= f64::from(r.click_distance) * f64::from(r.click_distance)
            }
        });
        r.clicks[button] = if double {
            None
        } else {
            Some((r.clock, pos, id))
        };
        if double {
            r.doubles[button] = Some(id);
        }
        double
    }
}

#[cfg(test)]
mod tests;
