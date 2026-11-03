use restricted_gui::context::Context;
use restricted_gui::id::Id;
use restricted_gui::ui::Ui;
use restricted_gui::ScrollFlags;

pub const WINDOW_TITLE: &str = "inspector##demo";

pub struct WindowStats {
    pub title: String,
    pub vertices: usize,
    pub indices: usize,
    pub commands: usize,
}

pub struct InspectionSnapshot {
    pub hovered: Option<Id>,
    pub captured: Option<Id>,
    pub windows: Vec<WindowStats>,
    pub clips: Vec<[f32; 4]>,
    pub nodes: Vec<(restricted_gui::diagnostics::InspectionNode, String)>,
}

impl InspectionSnapshot {
    pub fn capture(ctx: &Context, inspector_title: &str) -> Self {
        let mut windows = Vec::new();
        let mut clips = Vec::new();
        for window in ctx.inspect_windows() {
            if window.title == inspector_title {
                continue;
            }
            windows.push(WindowStats {
                title: window
                    .title
                    .split("##")
                    .next()
                    .unwrap_or(window.title)
                    .to_string(),
                vertices: window.vertices,
                indices: window.indices,
                commands: window.commands.len(),
            });
            push_unique_clip(&mut clips, window.rect);
            for command in window.commands {
                push_unique_clip(&mut clips, command.clip_rect);
            }
        }
        for command in ctx.inspect_overlay_commands() {
            push_unique_clip(&mut clips, command.clip_rect);
        }
        Self {
            hovered: ctx.hovered_widget_id(),
            captured: ctx.captured_widget_id(),
            windows,
            clips,
            nodes: (0..ctx.inspection_count())
                .filter_map(|i| ctx.inspection_node(i))
                .map(|node| {
                    let mut name = String::new();
                    ctx.inspection_name(node.id, &mut name);
                    (node, name)
                })
                .collect(),
        }
    }
}

fn push_unique_clip(clips: &mut Vec<[f32; 4]>, rect: [f32; 4]) {
    if rect.iter().all(|value| value.is_finite()) && !clips.contains(&rect) {
        clips.push(rect);
    }
}

pub fn id_label(id: Option<Id>) -> String {
    id.map_or_else(|| "none".to_string(), |id| format!("{:016x}", id.0))
}

pub fn show(ui: &mut Ui<'_>, snapshot: &InspectionSnapshot, selected: &mut Option<u64>) {
    ui.window(WINDOW_TITLE, ScrollFlags::BAR, |ui| {
        ui.label(&format!("hovered: {}", id_label(snapshot.hovered)));
        ui.label(&format!("captured: {}", id_label(snapshot.captured)));
        ui.label(&format!(
            "clip bounds: {} (cyan outlines)",
            snapshot.clips.len()
        ));
        for window in &snapshot.windows {
            ui.label(&format!(
                "{}: {}v {}i {}c",
                window.title, window.vertices, window.indices, window.commands
            ));
        }
        ui.separator();
        ui.label("Declared widget tree (select to highlight)");
        let mut chosen = snapshot
            .nodes
            .iter()
            .position(|(node, _)| Some(node.id) == *selected)
            .unwrap_or(usize::MAX);
        for (index, (node, name)) in snapshot.nodes.iter().enumerate() {
            let prefix = if node.kind == 0 {
                "scope"
            } else if node.flags & 1 != 0 {
                "control"
            } else {
                "item"
            };
            let name = if name.is_empty() { "unnamed" } else { name };
            let label = format!(
                "{}{}: {}##trace{}",
                "  ".repeat(node.depth.min(24) as usize),
                prefix,
                name,
                index
            );
            if ui.nav_row(&label, &mut chosen, index) {
                *selected = Some(node.id);
            }
        }
        if let Some((node, name)) = snapshot
            .nodes
            .iter()
            .find(|(node, _)| Some(node.id) == *selected)
        {
            ui.label(&format!(
                "{name}: {:016x} / flags {:02x}",
                node.id, node.flags
            ));
            ui.label(&format!("bounds: {:?}", node.rect));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../../core/tests/assets/DejaVuSans.ttf");

    #[test]
    fn snapshot_counts_only_demo_windows_and_deduplicates_clips() {
        let mut ctx = Context::new(FONT).unwrap();
        ctx.new_frame(1.0 / 60.0);
        {
            let mut ui = ctx.ui();
            ui.window("demo##one", ScrollFlags::NONE, |ui| {
                ui.label("hello");
            });
            ui.window("inspector##demo", ScrollFlags::NONE, |ui| {
                ui.label("self");
            });
        }
        let snapshot = InspectionSnapshot::capture(&ctx, "inspector##demo");
        assert_eq!(snapshot.windows.len(), 1);
        assert_eq!(snapshot.windows[0].title, "demo");
        assert!(snapshot.windows[0].vertices > 0);
        assert!(snapshot.windows[0].indices > 0);
        assert!(snapshot.windows[0].commands > 0);
        assert!(!snapshot.clips.is_empty());
        assert!(snapshot.clips.len() <= snapshot.windows[0].commands + 1);
        let _ = ctx.render();
    }

    #[test]
    fn repeated_identical_clips_are_recorded_once() {
        let mut clips = Vec::new();
        push_unique_clip(&mut clips, [1.0, 2.0, 3.0, 4.0]);
        push_unique_clip(&mut clips, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(clips.len(), 1);
    }
}
