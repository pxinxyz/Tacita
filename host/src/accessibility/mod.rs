use restricted_gui::{
    accessibility::{AccessibilityAction, AccessibilityNode},
    context::Context,
    id::Id,
};
#[derive(Clone, Debug)]
pub struct Node {
    pub semantic: AccessibilityNode,
    pub name: String,
    pub value: String,
}
#[derive(Default)]
pub struct Snapshot {
    pub nodes: Vec<Node>,
    pub overflow: bool,
}
impl Snapshot {
    pub fn update(&mut self, ctx: &Context, origin: [f32; 2], scale: f32) {
        assert!(scale.is_finite() && scale > 0. && origin.iter().all(|n| n.is_finite()));
        let count = ctx.accessibility_count();
        self.nodes.truncate(count);
        for i in 0..count {
            let mut semantic = ctx.accessibility_node(i).expect("stable semantic snapshot");
            semantic.rect = [
                origin[0] + semantic.rect[0] * scale,
                origin[1] + semantic.rect[1] * scale,
                origin[0] + semantic.rect[2] * scale,
                origin[1] + semantic.rect[3] * scale,
            ];
            if i == self.nodes.len() {
                self.nodes.push(Node {
                    semantic,
                    name: String::new(),
                    value: String::new(),
                });
            }
            let n = &mut self.nodes[i];
            n.semantic = semantic;
            ctx.accessibility_name(semantic.id, &mut n.name);
            ctx.accessibility_value(semantic.id, &mut n.value);
        }
        self.overflow = ctx.accessibility_overflowed();
    }
    pub fn node(&self, id: Id) -> Option<&Node> {
        self.nodes.iter().find(|n| n.semantic.id == id)
    }
    pub fn parent(&self, id: Id) -> Option<Id> {
        let n = self.node(id)?;
        self.nodes
            .get(n.semantic.parent as usize)
            .map(|p| p.semantic.id)
    }
    pub fn children(&self, parent: Option<Id>) -> impl Iterator<Item = &Node> {
        let index = parent.and_then(|id| self.nodes.iter().position(|n| n.semantic.id == id));
        self.nodes.iter().filter(move |n| match parent {
            Some(_) => index == Some(n.semantic.parent as usize),
            None => n.semantic.parent == u32::MAX,
        })
    }
    pub fn contains(&self, ancestor: Option<Id>, id: Id) -> bool {
        let Some(ancestor) = ancestor else {
            return true;
        };
        let Some(mut index) = self.nodes.iter().position(|n| n.semantic.id == id) else {
            return false;
        };
        for _ in 0..self.nodes.len() {
            let n = &self.nodes[index];
            if n.semantic.id == ancestor {
                return true;
            }
            index = n.semantic.parent as usize;
            if index >= self.nodes.len() {
                return false;
            }
        }
        false
    }
}
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::ScreenReaderBridge;
pub fn apply_actions(
    ctx: &mut Context,
    actions: impl IntoIterator<Item = AccessibilityAction>,
) -> usize {
    actions
        .into_iter()
        .filter(|a| ctx.submit_accessibility_action(*a))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use restricted_gui::ScrollFlags;
    #[test]
    fn snapshot_hides_id_suffix_and_maps_screen_coordinates() {
        let mut ctx =
            Context::new(include_bytes!("../../../core/tests/assets/DejaVuSans.ttf")).unwrap();
        ctx.set_accessibility_enabled(true);
        ctx.new_frame(0.016);
        ctx.ui().window("root##id", ScrollFlags::NONE, |ui| {
            ui.button("press##id");
        });
        ctx.render();
        let mut s = Snapshot::default();
        s.update(&ctx, [100., 200.], 2.);
        assert!(s.nodes.iter().any(|n| n.name == "press"));
        assert_eq!(s.nodes[0].name, "root");
        assert_eq!(s.nodes[0].semantic.rect[0], 220.);
    }
    #[test]
    fn native_snapshot_keeps_nested_children_and_defers_actions() {
        use restricted_gui::accessibility::{AccessibilityActionKind, AccessibilityRole};
        let mut ctx =
            Context::new(include_bytes!("../../../core/tests/assets/DejaVuSans.ttf")).unwrap();
        ctx.set_accessibility_enabled(true);
        let mut checked = false;
        ctx.new_frame(0.016);
        ctx.ui().window("root", ScrollFlags::NONE, |ui| {
            ui.child("panel", [180., 100.], ScrollFlags::NONE, |ui| {
                ui.checkbox("toggle", &mut checked);
            });
        });
        ctx.render();
        let mut snapshot = Snapshot::default();
        snapshot.update(&ctx, [0.; 2], 1.);
        let window = snapshot.children(None).next().unwrap().semantic.id;
        let panel = snapshot.children(Some(window)).next().unwrap();
        assert_eq!(panel.semantic.role, AccessibilityRole::Group);
        let leaf = snapshot
            .children(Some(panel.semantic.id))
            .next()
            .unwrap()
            .semantic
            .id;
        assert_eq!(
            apply_actions(
                &mut ctx,
                [AccessibilityAction {
                    id: leaf,
                    kind: AccessibilityActionKind::Activate,
                    number: 0.,
                }]
            ),
            1
        );
        assert!(!checked);
        ctx.new_frame(0.016);
        ctx.ui().window("root", ScrollFlags::NONE, |ui| {
            ui.child("panel", [180., 100.], ScrollFlags::NONE, |ui| {
                assert!(ui.checkbox("toggle", &mut checked));
            });
        });
        ctx.render();
        assert!(checked);
    }
}
