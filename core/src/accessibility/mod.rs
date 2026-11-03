use crate::{context::Context, id::Id};
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::cell::RefCell;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityRole {
    Group,
    Window,
    Text,
    Button,
    CheckBox,
    RadioButton,
    Slider,
    TextInput,
    ComboBox,
    Tab,
    TreeItem,
    ProgressBar,
    Image,
    List,
    ListItem,
    MenuItem,
}
impl AccessibilityRole {
    pub fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::Group,
            1 => Self::Window,
            2 => Self::Text,
            3 => Self::Button,
            4 => Self::CheckBox,
            5 => Self::RadioButton,
            6 => Self::Slider,
            7 => Self::TextInput,
            8 => Self::ComboBox,
            9 => Self::Tab,
            10 => Self::TreeItem,
            11 => Self::ProgressBar,
            12 => Self::Image,
            13 => Self::List,
            14 => Self::ListItem,
            15 => Self::MenuItem,
            _ => return None,
        })
    }
}
pub struct AccessibilityState;
impl AccessibilityState {
    pub const DISABLED: u32 = 1;
    pub const FOCUSED: u32 = 2;
    pub const CHECKED: u32 = 4;
    pub const SELECTED: u32 = 8;
    pub const EXPANDED: u32 = 16;
    pub const READ_ONLY: u32 = 32;
    pub const HIDDEN: u32 = 64;
    pub const BLOCKED: u32 = 128;
}
pub struct AccessibilityActions;
impl AccessibilityActions {
    pub const FOCUS: u32 = 1;
    pub const ACTIVATE: u32 = 2;
    pub const SET_NUMBER: u32 = 4;
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityActionKind {
    Focus = 1,
    Activate = 2,
    SetNumber = 4,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccessibilityAction {
    pub id: Id,
    pub kind: AccessibilityActionKind,
    pub number: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccessibilityNode {
    pub id: Id,
    pub parent: u32,
    pub role: AccessibilityRole,
    pub rect: [f32; 4],
    pub states: u32,
    pub actions: u32,
}
#[derive(Default)]
struct Frame {
    nodes: Vec<AccessibilityNode>,
    scopes: Vec<u32>,
    names: BTreeMap<Id, String>,
    values: BTreeMap<Id, String>,
    pending: Option<Id>,
    overflow: bool,
}
pub(crate) struct SemanticTree {
    enabled: bool,
    frame: RefCell<Frame>,
}
impl Default for SemanticTree {
    fn default() -> Self {
        Self {
            enabled: false,
            frame: RefCell::new(Frame::default()),
        }
    }
}
fn store(map: &mut BTreeMap<Id, String>, id: Id, text: &str) -> bool {
    let mut n = text.len().min(256);
    while !text.is_char_boundary(n) {
        n -= 1;
    }
    let text = &text[..n];
    if let Some(old) = map.get_mut(&id) {
        if old != text {
            old.clear();
            old.push_str(text);
        }
    } else if map.len() < 8192 {
        map.insert(id, String::from(text));
    } else {
        return false;
    }
    true
}
impl SemanticTree {
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }
    fn enable(&mut self, on: bool) {
        self.enabled = on;
        self.frame.borrow_mut().nodes.clear();
        self.frame.borrow_mut().scopes.clear();
    }
    pub(crate) fn reset(&self) {
        if self.enabled {
            let mut f = self.frame.borrow_mut();
            f.nodes.clear();
            f.scopes.clear();
            f.pending = None;
            f.overflow = false;
        }
    }
    pub(crate) fn label(&self, id: Id, label: &str) {
        if self.enabled {
            let mut f = self.frame.borrow_mut();
            if !store(&mut f.names, id, crate::id::split_label(label).0) {
                f.overflow = true;
            }
            f.pending = Some(id);
        }
    }
    pub(crate) fn scope(&self, id: Id, label: Option<&str>) {
        if !self.enabled {
            return;
        }
        if let Some(label) = label {
            self.label(id, label);
        }
        let mut f = self.frame.borrow_mut();
        let parent = f.scopes.last().copied().unwrap_or(u32::MAX);
        if let Some(index) = f.nodes.iter().rposition(|n| n.id == id) {
            if f.nodes[index].actions == 0 {
                f.nodes[index].role = AccessibilityRole::Group;
            }
            f.scopes.push(index as u32);
            f.pending = None;
            return;
        }
        let index = f.nodes.len() as u32;
        if index >= 8192 {
            f.overflow = true;
            f.scopes.push(u32::MAX);
            return;
        }
        f.nodes.push(AccessibilityNode {
            id,
            parent,
            role: if parent == u32::MAX {
                AccessibilityRole::Window
            } else {
                AccessibilityRole::Group
            },
            rect: [0.; 4],
            states: 0,
            actions: 0,
        });
        f.scopes.push(index);
        f.pending = None;
    }
    pub(crate) fn pop(&self) {
        if self.enabled {
            self.frame.borrow_mut().scopes.pop();
        }
    }
    pub(crate) fn scope_rect(&self, rect: [f32; 4]) {
        if self.enabled {
            let mut f = self.frame.borrow_mut();
            if let Some(index) = f.scopes.last().copied() {
                if let Some(n) = f.nodes.get_mut(index as usize) {
                    n.rect = rect;
                }
            }
        }
    }
    pub(crate) fn item(&self, fallback: Id, rect: [f32; 4], states: u32) {
        if !self.enabled {
            return;
        }
        let mut f = self.frame.borrow_mut();
        if f.nodes.len() >= 8192 {
            f.overflow = true;
            return;
        }
        let id = f.pending.take().unwrap_or(fallback);
        let parent = f.scopes.last().copied().unwrap_or(u32::MAX);
        f.nodes.push(AccessibilityNode {
            id,
            parent,
            role: AccessibilityRole::Text,
            rect,
            states,
            actions: 0,
        });
    }
    pub(crate) fn interactive(&self, id: Id, rect: [f32; 4], states: u32, focus: bool) {
        if !self.enabled {
            return;
        }
        let mut f = self.frame.borrow_mut();
        let index = f.nodes.iter().rposition(|n| {
            n.role != AccessibilityRole::Group
                && n.role != AccessibilityRole::Window
                && (n.id == id || n.rect == rect)
        });
        let index = if let Some(i) = index {
            i
        } else {
            let parent = f.scopes.last().copied().unwrap_or(u32::MAX);
            if f.nodes.len() >= 8192 {
                f.overflow = true;
                return;
            }
            let i = f.nodes.len();
            f.nodes.push(AccessibilityNode {
                id,
                parent,
                role: AccessibilityRole::Button,
                rect,
                states,
                actions: 0,
            });
            i
        };
        let n = &mut f.nodes[index];
        n.id = id;
        n.rect = rect;
        n.states = states;
        n.role = AccessibilityRole::Button;
        n.actions = if focus {
            AccessibilityActions::ACTIVATE | AccessibilityActions::FOCUS
        } else {
            0
        };
    }
    pub(crate) fn describe(
        &self,
        id: Id,
        role: AccessibilityRole,
        value: &str,
        states: u32,
        actions: u32,
    ) {
        if !self.enabled {
            return;
        }
        let mut f = self.frame.borrow_mut();
        if !store(&mut f.values, id, value) {
            f.overflow = true;
        }
        if let Some(n) = f.nodes.iter_mut().rev().find(|n| n.id == id) {
            n.role = role;
            n.states |= states;
            n.actions = actions;
        }
    }
    fn node(&self, index: usize, focused: Option<Id>) -> Option<AccessibilityNode> {
        if !self.enabled {
            return None;
        }
        let mut n = *self.frame.borrow().nodes.get(index)?;
        if focused == Some(n.id) {
            n.states |= AccessibilityState::FOCUSED;
        }
        Some(n)
    }
    fn text(&self, id: Id, value: bool, out: &mut String) {
        out.clear();
        if !self.enabled {
            return;
        }
        let f = self.frame.borrow();
        if let Some(s) = if value {
            f.values.get(&id)
        } else {
            f.names.get(&id)
        } {
            out.push_str(s);
        }
    }
    pub(crate) fn name(&self, id: Id, name: &str) {
        if self.enabled {
            let mut f = self.frame.borrow_mut();
            if !store(&mut f.names, id, name) {
                f.overflow = true;
            }
        }
    }
}
#[derive(Default)]
pub(crate) struct ActionQueue {
    pub pending: Vec<AccessibilityAction>,
    pub current: Vec<AccessibilityAction>,
}
impl Context {
    pub fn set_accessibility_enabled(&mut self, on: bool) {
        assert!(!self.frame_active, "accessibility changes between frames");
        self.id_stack.semantics.enable(on);
        self.accessibility_actions.pending.clear();
        self.accessibility_actions.current.clear();
    }
    pub fn accessibility_enabled(&self) -> bool {
        self.id_stack.semantics.enabled()
    }
    pub fn accessibility_count(&self) -> usize {
        if self.accessibility_enabled() {
            self.id_stack.semantics.frame.borrow().nodes.len()
        } else {
            0
        }
    }
    pub fn accessibility_overflowed(&self) -> bool {
        self.accessibility_enabled() && self.id_stack.semantics.frame.borrow().overflow
    }
    pub fn accessibility_node(&self, index: usize) -> Option<AccessibilityNode> {
        self.id_stack.semantics.node(index, self.focused)
    }
    pub fn accessibility_name(&self, id: Id, out: &mut String) {
        self.id_stack.semantics.text(id, false, out);
    }
    pub fn accessibility_value(&self, id: Id, out: &mut String) {
        self.id_stack.semantics.text(id, true, out);
    }
    pub fn submit_accessibility_action(&mut self, action: AccessibilityAction) -> bool {
        assert!(
            !self.frame_active,
            "accessibility actions require an idle context"
        );
        if !action.number.is_finite() {
            return false;
        }
        let valid = (0..self.accessibility_count())
            .filter_map(|i| self.accessibility_node(i))
            .any(|n| {
                n.id == action.id
                    && n.actions & action.kind as u32 != 0
                    && n.states
                        & (AccessibilityState::DISABLED
                            | AccessibilityState::HIDDEN
                            | AccessibilityState::BLOCKED)
                        == 0
            });
        if !valid {
            return false;
        }
        if action.kind == AccessibilityActionKind::Focus {
            self.accessibility_actions
                .pending
                .retain(|a| a.kind != AccessibilityActionKind::Focus);
        }
        if let Some(old) = self
            .accessibility_actions
            .pending
            .iter_mut()
            .find(|a| a.id == action.id && a.kind == action.kind)
        {
            *old = action;
        } else {
            if self.accessibility_actions.pending.len() >= 64 {
                return false;
            }
            self.accessibility_actions.pending.push(action);
        }
        self.invalidate();
        true
    }
    pub(crate) fn semantic_states(&self, rect: [f32; 4]) -> u32 {
        let mut s = if self.disabled_depth > 0 {
            AccessibilityState::DISABLED
        } else {
            0
        };
        if !self.popup_input_allowed() || (self.open_popup.is_some() && !self.popup_scope_active) {
            s |= AccessibilityState::BLOCKED;
        }
        let r = self.semantic_clip(rect);
        if r[2] <= r[0]
            || r[3] <= r[1]
            || self.rounded_clips.iter().any(|clip| {
                clip.popup == self.popup_scope_active
                    && clip.popup_id == self.routing.scope_id()
                    && clip.window == self.current_window
                    && !clip.shape.intersects(r)
            })
        {
            s |= AccessibilityState::HIDDEN;
        }
        s
    }
    pub(crate) fn semantic_clip(&self, r: [f32; 4]) -> [f32; 4] {
        let mut c = if self.popup_scope_active {
            self.overlay.current_clip()
        } else if let Some(id) = self.current_window {
            self.windows[self.window_index(id)].draw_list.current_clip()
        } else {
            r
        };
        if let Some(input) = self.interaction_clip {
            c = [
                c[0].max(input[0]),
                c[1].max(input[1]),
                c[2].min(input[2]),
                c[3].min(input[3]),
            ];
        }
        [
            r[0].max(c[0]),
            r[1].max(c[1]),
            r[2].min(c[2]),
            r[3].min(c[3]),
        ]
    }
    pub(crate) fn semantic_register(&mut self, id: Id, rect: [f32; 4], focus: bool) {
        let states = self.semantic_states(rect);
        self.id_stack.semantics.interactive(
            id,
            self.semantic_clip(self.target_rect(rect)),
            states,
            focus,
        );
        if states
            & (AccessibilityState::DISABLED
                | AccessibilityState::HIDDEN
                | AccessibilityState::BLOCKED)
            != 0
        {
            return;
        }
        for action in &self.accessibility_actions.current {
            if action.id == id {
                match action.kind {
                    AccessibilityActionKind::Focus => {
                        if focus {
                            self.focused = Some(id);
                            self.focused_in_popup = self.popup_scope_active;
                            self.focused_in_modal = self.popup_modal;
                        }
                    }
                    AccessibilityActionKind::Activate => self.keyboard_activate = Some(id),
                    AccessibilityActionKind::SetNumber => {}
                }
            }
        }
    }
    pub(crate) fn semantic_number(&self, id: Id) -> Option<f64> {
        if self.disabled_depth > 0 || !self.popup_input_allowed() {
            return None;
        }
        if !self
            .id_stack
            .semantics
            .frame
            .borrow()
            .nodes
            .iter()
            .any(|n| {
                n.id == id
                    && n.states
                        & (AccessibilityState::DISABLED
                            | AccessibilityState::HIDDEN
                            | AccessibilityState::BLOCKED)
                        == 0
            })
        {
            return None;
        }
        self.accessibility_actions
            .current
            .iter()
            .rev()
            .find(|a| a.id == id && a.kind == AccessibilityActionKind::SetNumber)
            .map(|a| a.number)
    }
}
impl crate::ui::Ui<'_> {
    pub fn accessibility_info(
        &mut self,
        id: Id,
        role: AccessibilityRole,
        value: &str,
        states: u32,
        actions: u32,
    ) {
        self.ctx
            .id_stack
            .semantics
            .describe(id, role, value, states, actions);
    }
    pub fn accessibility_name(&mut self, id: Id, name: &str) {
        self.ctx.id_stack.semantics.name(id, name);
    }
}

#[cfg(test)]
mod tests;
