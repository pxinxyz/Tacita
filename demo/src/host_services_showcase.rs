use tacita::{
    accessibility::{AccessibilityAction, AccessibilityActionKind, AccessibilityNode},
    context::Context,
    host_requests::CursorShape,
    ui::Ui,
};
#[derive(Default)]
pub struct State {
    pub large_targets: bool,
    pub native_cursor: bool,
    pub idle: bool,
    pub duplicates: bool,
    pub checked: bool,
    pub value: f32,
    pub text: String,
    pub clicks: u32,
    pub nodes: Vec<(AccessibilityNode, String, String)>,
    pub duplicates_count: usize,
    pub wake: Option<f32>,
    pub cursor: CursorShape,
    pub bridge_status: String,
    pub pending: Option<AccessibilityAction>,
    pub selected: usize,
    preview_root: Option<tacita::id::Id>,
}
impl State {
    pub fn collect(&mut self, ctx: &Context) {
        self.duplicates_count = ctx.duplicate_ids().len();
        self.wake = ctx.next_redraw_in();
        self.cursor = ctx.cursor_shape();
        let mut count = 0;
        for i in 0..ctx.accessibility_count() {
            let n = ctx.accessibility_node(i).unwrap();
            let mut ancestor = Some(n);
            let mut is_preview = false;
            while let Some(node) = ancestor {
                if Some(node.id) == self.preview_root {
                    is_preview = true;
                    break;
                }
                ancestor = ctx.accessibility_node(node.parent as usize);
            }
            if is_preview {
                continue;
            }
            if count == self.nodes.len() {
                self.nodes.push((n, String::new(), String::new()));
            }
            let row = &mut self.nodes[count];
            row.0 = n;
            ctx.accessibility_name(n.id, &mut row.1);
            ctx.accessibility_value(n.id, &mut row.2);
            count += 1;
            if count == 128 {
                break;
            }
        }
        self.nodes.truncate(count);
    }
    pub fn draw(&mut self, ui: &mut Ui<'_>) {
        ui.label("Host services / accessibility");
        ui.checkbox(
            "Larger hit targets (32 logical px)",
            &mut self.large_targets,
        );
        ui.checkbox("Native widget cursors", &mut self.native_cursor);
        ui.checkbox("Idle redraw mode", &mut self.idle);
        ui.tooltip_text("Limits redraws while this page stays open.");
        ui.checkbox("Deliberate duplicate IDs", &mut self.duplicates);
        ui.label("Select High contrast on the Appearance page.");
        ui.separator();
        ui.pill_toggle("Accessible toggle", &mut self.checked);
        if ui.button("Accessible action") {
            self.clicks += 1;
            eprintln!("[demo] accessible action: {}", self.clicks);
        }
        ui.slider_float("Accessible value", &mut self.value, 0.0..=100.0);
        ui.text_input("Accessible text", &mut self.text);
        let disabled = ui.begin_disabled("disabled accessibility", true);
        ui.button("Disabled action");
        ui.end_disabled(disabled);
        if self.duplicates {
            ui.button("Collision##same");
            ui.button("Collision##same");
        }
        ui.label(&format!(
            "clicks {} / duplicate IDs {} / cursor {:?}",
            self.clicks, self.duplicates_count, self.cursor
        ));
        ui.label(&format!(
            "core next wake {:?} / {}",
            self.wake, self.bridge_status
        ));
        ui.label("Idle mode follows core redraw requests and updates the status once per second.");
        ui.label("The semantic preview below is capped at 128 nodes, not the provider tree.");
        self.preview_root = Some(ui.widget_id("semantic tree"));
        let preview_width = ui.available_width().max(160.);
        ui.child(
            "semantic tree",
            [preview_width, 200.],
            tacita::ScrollFlags::BAR,
            |ui| {
                for (i, (node, name, value)) in self.nodes.iter().enumerate() {
                    if ui.radio_button(
                        &format!("{name} / {:?} / {value}##semantic-{i}", node.role),
                        &mut self.selected,
                        i,
                    ) {}
                }
            },
        );
        if ui.button("Focus selected semantic node") {
            if let Some((n, _, _)) = self.nodes.get(self.selected) {
                self.pending = Some(AccessibilityAction {
                    id: n.id,
                    kind: AccessibilityActionKind::Focus,
                    number: 0.,
                });
            }
        }
        if ui.button("Activate selected semantic node") {
            if let Some((n, _, _)) = self.nodes.get(self.selected) {
                self.pending = Some(AccessibilityAction {
                    id: n.id,
                    kind: AccessibilityActionKind::Activate,
                    number: 0.,
                });
            }
        }
    }
}
pub fn native_cursor(shape: CursorShape) -> winit::window::CursorIcon {
    use winit::window::CursorIcon;
    match shape {
        CursorShape::Default => CursorIcon::Default,
        CursorShape::Pointer => CursorIcon::Pointer,
        CursorShape::Text => CursorIcon::Text,
        CursorShape::ResizeHorizontal => CursorIcon::EwResize,
        CursorShape::ResizeVertical => CursorIcon::NsResize,
        CursorShape::ResizeDiagonal => CursorIcon::NwseResize,
        CursorShape::Grab => CursorIcon::Grab,
        CursorShape::Grabbing => CursorIcon::Grabbing,
        CursorShape::NotAllowed => CursorIcon::NotAllowed,
    }
}
