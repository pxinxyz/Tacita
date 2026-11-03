use crate::id::Id;
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::cell::RefCell;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum InspectionKind {
    Scope = 0,
    Item = 1,
}
pub struct InspectionFlags;
impl InspectionFlags {
    pub const INTERACTIVE: u32 = 1;
    pub const FOCUSABLE: u32 = 2;
    pub const DISABLED: u32 = 4;
    pub const HOVERED: u32 = 8;
    pub const CAPTURED: u32 = 16;
    pub const FOCUSED: u32 = 32;
    pub const EXPLICIT_BOUNDS: u32 = 64;
}
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct InspectionNode {
    pub id: u64,
    pub parent: u32,
    pub kind: u32,
    pub rect: [f32; 4],
    pub flags: u32,
    pub depth: u32,
}
#[derive(Default)]
struct Trace {
    nodes: Vec<InspectionNode>,
    scopes: Vec<u32>,
    names: BTreeMap<u64, String>,
    pending: Option<Id>,
    items: u64,
}
pub(crate) struct Inspector {
    enabled: bool,
    trace: RefCell<Trace>,
}
impl Inspector {
    pub fn new() -> Self {
        Self {
            enabled: false,
            trace: RefCell::new(Trace::default()),
        }
    }
    pub fn enable(&mut self, on: bool) {
        self.enabled = on;
        self.reset();
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn reset(&self) {
        if self.enabled {
            let mut t = self.trace.borrow_mut();
            t.nodes.clear();
            t.scopes.clear();
            t.pending = None;
            t.items = 0;
        }
    }
    pub fn label(&self, id: Id, label: &str) {
        if !self.enabled {
            return;
        }
        let mut t = self.trace.borrow_mut();
        let mut end = label.len().min(256);
        while !label.is_char_boundary(end) {
            end -= 1;
        }
        let label = &label[..end];
        if let Some(old) = t.names.get_mut(&id.0) {
            if old != label {
                old.clear();
                old.push_str(label);
            }
        } else if t.names.len() < 8192 {
            t.names.insert(id.0, String::from(label));
        }
        t.pending = Some(id);
    }
    pub fn scope(&self, id: Id, label: Option<&str>) {
        if !self.enabled {
            return;
        }
        if let Some(label) = label {
            self.label(id, label);
        }
        let mut t = self.trace.borrow_mut();
        let parent = t.scopes.last().copied().unwrap_or(u32::MAX);
        let depth = t.scopes.len() as u32;
        if t.nodes.len() >= 8192 {
            t.scopes.push(u32::MAX);
            return;
        }
        let index = t.nodes.len() as u32;
        t.nodes.push(InspectionNode {
            id: id.0,
            parent,
            kind: 0,
            rect: [0.; 4],
            flags: 0,
            depth,
        });
        t.scopes.push(index);
        t.pending = None;
    }
    pub fn pop(&self) {
        if !self.enabled {
            return;
        }
        let mut t = self.trace.borrow_mut();
        let Some(index) = t.scopes.pop() else {
            return;
        };
        if index == u32::MAX {
            return;
        }
        let rect = t.nodes[index as usize].rect;
        if rect[2] > rect[0] && rect[3] > rect[1] {
            Self::extend_scope(&mut t, rect);
        }
    }
    fn extend_scope(t: &mut Trace, rect: [f32; 4]) {
        if let Some(&index) = t.scopes.last() {
            if let Some(node) = t.nodes.get_mut(index as usize) {
                if node.flags & InspectionFlags::EXPLICIT_BOUNDS != 0 {
                    return;
                }
                if node.rect[2] <= node.rect[0] || node.rect[3] <= node.rect[1] {
                    node.rect = rect;
                } else {
                    node.rect = [
                        node.rect[0].min(rect[0]),
                        node.rect[1].min(rect[1]),
                        node.rect[2].max(rect[2]),
                        node.rect[3].max(rect[3]),
                    ];
                }
            }
        }
    }
    pub fn item(&self, rect: [f32; 4], disabled: bool, fallback: Id) {
        if !self.enabled {
            return;
        }
        let mut t = self.trace.borrow_mut();
        if t.nodes.len() >= 8192 {
            return;
        }
        t.items += 1;
        let id = t.pending.take().unwrap_or(fallback).0;
        let parent = t.scopes.last().copied().unwrap_or(u32::MAX);
        let depth = t.scopes.len() as u32;
        t.nodes.push(InspectionNode {
            id,
            parent,
            kind: 1,
            rect,
            flags: if disabled {
                InspectionFlags::DISABLED
            } else {
                0
            },
            depth,
        });
        Self::extend_scope(&mut t, rect);
    }
    pub fn interactive(&self, id: Id, rect: [f32; 4], flags: u32) {
        if !self.enabled {
            return;
        }
        let mut t = self.trace.borrow_mut();
        if t.nodes.len() >= 8192 {
            return;
        }
        if let Some(node) = t
            .nodes
            .iter_mut()
            .rev()
            .find(|n| n.kind == 1 && n.rect == rect)
        {
            node.id = id.0;
            node.flags |= flags;
        } else {
            drop(t);
            self.trace.borrow_mut().pending = Some(id);
            self.item(rect, false, id);
            self.interactive(id, rect, flags);
        }
    }
    pub fn scope_rect(&self, rect: [f32; 4]) {
        if !self.enabled {
            return;
        }
        let mut t = self.trace.borrow_mut();
        if let Some(&index) = t.scopes.last() {
            if let Some(node) = t.nodes.get_mut(index as usize) {
                node.rect = rect;
                node.flags |= InspectionFlags::EXPLICIT_BOUNDS;
            }
        }
    }
    pub fn count(&self) -> usize {
        if self.enabled {
            self.trace.borrow().nodes.len()
        } else {
            0
        }
    }
    pub fn node(
        &self,
        index: usize,
        hot: Option<Id>,
        active: Option<Id>,
        focus: Option<Id>,
    ) -> Option<InspectionNode> {
        if !self.enabled {
            return None;
        }
        let mut node = *self.trace.borrow().nodes.get(index)?;
        if node.id != 0 {
            for (current, flag) in [
                (hot, InspectionFlags::HOVERED),
                (active, InspectionFlags::CAPTURED),
                (focus, InspectionFlags::FOCUSED),
            ] {
                if current == Some(Id(node.id)) {
                    node.flags |= flag;
                }
            }
        }
        Some(node)
    }
    pub fn name(&self, id: u64, out: &mut String) {
        out.clear();
        if self.enabled {
            if let Some(name) = self.trace.borrow().names.get(&id) {
                out.push_str(name);
            }
        }
    }
}
impl crate::context::Context {
    pub fn set_inspection_enabled(&mut self, on: bool) {
        assert!(!self.frame_active, "inspection mode changes between frames");
        self.id_stack.inspector.enable(on);
    }
    pub fn inspection_enabled(&self) -> bool {
        self.id_stack.inspector.enabled()
    }
    pub fn inspection_count(&self) -> usize {
        self.id_stack.inspector.count()
    }
    pub fn inspection_node(&self, index: usize) -> Option<InspectionNode> {
        self.id_stack
            .inspector
            .node(index, self.hot, self.active, self.focused)
    }
    pub fn inspection_name(&self, id: u64, out: &mut String) {
        self.id_stack.inspector.name(id, out);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{context::Context, ScrollFlags};
    #[test]
    fn custom_item_without_layout_preserves_its_registered_name() {
        let mut inspector = Inspector::new();
        inspector.enable(true);
        inspector.label(Id(123), "custom item");
        inspector.interactive(Id(123), [1., 2., 10., 20.], InspectionFlags::INTERACTIVE);
        let mut name = String::new();
        inspector.name(123, &mut name);
        assert_eq!(name, "custom item");
        assert_eq!(inspector.count(), 1);
    }

    #[test]
    fn nested_trace_has_labels_bounds_and_flags_without_changing_ids() {
        let mut ctx = Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap();
        ctx.set_inspection_enabled(true);
        ctx.new_frame(0.016);
        ctx.ui().window("root", ScrollFlags::NONE, |ui| {
            ui.label("label");
            ui.button("button");
        });
        ctx.render();
        assert!(ctx.inspection_count() >= 3);
        let mut text = String::new();
        let button = (0..ctx.inspection_count())
            .filter_map(|i| ctx.inspection_node(i))
            .find(|n| n.flags & InspectionFlags::INTERACTIVE != 0)
            .unwrap();
        ctx.inspection_name(button.id, &mut text);
        assert_eq!(text, "button");
        assert_ne!(button.parent, u32::MAX);
        assert!(button.rect[2] > button.rect[0]);
        ctx.set_inspection_enabled(false);
        assert_eq!(ctx.inspection_count(), 0);
    }
}

#[cfg(test)]
mod bounds_tests {
    use crate::{context::Context, ScrollFlags};
    #[test]
    fn distinct_inert_items_and_window_bounds() {
        let mut ctx = Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap();
        ctx.set_window_rect("window", [50., 60.], [400., 300.]);
        ctx.set_inspection_enabled(true);
        ctx.new_frame(0.016);
        ctx.ui().window("window", ScrollFlags::NONE, |ui| {
            ui.allocate([20.; 2]);
            ui.allocate([20.; 2]);
        });
        ctx.render();
        let root = ctx.inspection_node(0).unwrap();
        assert_eq!(root.rect, [50., 60., 450., 360.]);
        let one = ctx.inspection_node(1).unwrap();
        let two = ctx.inspection_node(2).unwrap();
        assert_ne!(one.id, two.id);
        assert_eq!(one.parent, 0);
        assert_eq!(two.parent, 0);
    }
}
