use crate::style_watch::{ReloadStatus, StyleWatcher};
use restricted_gui::{
    context::Context,
    id::Id,
    input::{Key, Modifiers},
    interaction::{Navigation, Shortcut, ShortcutScope},
    persistence::{decode_theme, encode_theme, Config, History, LayoutSnapshot, PanelState, Value},
    theme::ThemeTokens,
    ui::Ui,
};
use std::{path::PathBuf, time::Duration};
#[derive(Clone, Copy)]
enum Action {
    SaveConfig,
    LoadConfig,
    SaveTheme,
    LoadTheme,
    SaveLayout,
    LoadLayout,
    Watch,
    StopWatch,
    Undo,
    Redo,
}
pub struct Outcome {
    pub theme: Option<ThemeTokens>,
    pub layout: Option<LayoutSnapshot>,
}
pub struct State {
    gain: i32,
    enabled: bool,
    name: String,
    history: History<i32>,
    config: Config,
    action: Option<Action>,
    navigation: Vec<Navigation>,
    nav_focus: Option<Id>,
    status: String,
    watch: Option<StyleWatcher>,
    directory: PathBuf,
    activation: usize,
    double_clicks: usize,
}
impl Default for State {
    fn default() -> Self {
        let mut config = Config::new();
        config.register("gain", Value::Integer(10)).unwrap();
        config.register("enabled", Value::Bool(true)).unwrap();
        config
            .register("name", Value::Text(String::from("preview")))
            .unwrap();
        let directory = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("preview-state")))
            .unwrap_or_else(|| PathBuf::from("preview-state"));
        Self {
            gain: 10,
            enabled: true,
            name: String::from("preview"),
            history: History::new(10, 64).unwrap(),
            config,
            action: None,
            navigation: Vec::new(),
            nav_focus: None,
            status: String::from("State files are created only when saved or watched."),
            watch: None,
            directory,
            activation: 0,
            double_clicks: 0,
        }
    }
}
impl State {
    pub fn process(&mut self, ctx: &mut Context, panels: &[(&str, PanelState)]) -> Outcome {
        let mut result = Outcome {
            theme: None,
            layout: None,
        };
        for event in self.navigation.drain(..) {
            ctx.navigate(event);
        }
        if let Some(action) = self.action.take() {
            let action_result = (|| -> Result<(), String> {
                match action {
                    Action::Undo => {
                        if let Some(v) = self.history.undo() {
                            self.gain = *v;
                        }
                    }
                    Action::Redo => {
                        if let Some(v) = self.history.redo() {
                            self.gain = *v;
                        }
                    }
                    Action::StopWatch => {
                        self.watch = None;
                    }
                    Action::SaveConfig => {
                        self.config
                            .set("gain", Value::Integer(self.gain as i64))
                            .unwrap();
                        self.config
                            .set("enabled", Value::Bool(self.enabled))
                            .unwrap();
                        self.config
                            .set("name", Value::Text(self.name.clone()))
                            .map_err(|e| format!("{e:?}"))?;
                        std::fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
                        std::fs::write(self.directory.join("demo.config"), self.config.encode())
                            .map_err(|e| e.to_string())?;
                    }
                    Action::LoadConfig => {
                        let bytes =
                            read_bounded(&self.directory.join("demo.config"), 2 * 1024 * 1024)?;
                        let mut config = self.config.clone();
                        config.load(&bytes).map_err(|e| format!("{e:?}"))?;
                        let Some(Value::Integer(gain)) = config.get("gain") else {
                            unreachable!()
                        };
                        if !(0..=100).contains(gain) {
                            return Err(String::from("gain must be 0..100"));
                        }
                        self.gain = *gain as i32;
                        if let Some(Value::Bool(enabled)) = config.get("enabled") {
                            self.enabled = *enabled;
                        }
                        if let Some(Value::Text(name)) = config.get("name") {
                            self.name.clone_from(name);
                        }
                        self.config = config;
                        self.history.reset(self.gain);
                    }
                    Action::SaveTheme | Action::Watch => {
                        std::fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
                        let path = self.directory.join("preview.theme");
                        if matches!(action, Action::SaveTheme) || !path.exists() {
                            std::fs::write(
                                &path,
                                encode_theme(ctx.theme_tokens()).map_err(|e| format!("{e:?}"))?,
                            )
                            .map_err(|e| e.to_string())?;
                        }
                        if matches!(action, Action::Watch) {
                            self.watch = Some(StyleWatcher::new(path, Duration::from_millis(500)));
                        }
                    }
                    Action::LoadTheme => {
                        let bytes = read_bounded(&self.directory.join("preview.theme"), 16384)?;
                        result.theme = Some(decode_theme(&bytes).map_err(|e| format!("{e:?}"))?);
                    }
                    Action::SaveLayout => {
                        let mut layout = ctx.save_layout();
                        for (name, state) in panels {
                            layout
                                .set_panel(name, *state)
                                .map_err(|e| format!("{e:?}"))?;
                        }
                        std::fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
                        std::fs::write(
                            self.directory.join("preview.layout"),
                            layout.encode().map_err(|e| format!("{e:?}"))?,
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    Action::LoadLayout => {
                        let bytes =
                            read_bounded(&self.directory.join("preview.layout"), 2 * 1024 * 1024)?;
                        let layout =
                            LayoutSnapshot::decode(&bytes).map_err(|e| format!("{e:?}"))?;
                        ctx.load_layout(&layout).map_err(|e| format!("{e:?}"))?;
                        result.layout = Some(layout);
                    }
                }
                Ok(())
            })();
            self.status = match action_result {
                Ok(()) => String::from("Action applied."),
                Err(error) => format!("Kept previous state: {error}"),
            };
        }
        if let Some(watch) = &mut self.watch {
            if let Some(theme) = watch.poll() {
                result.theme = Some(theme);
                self.status = String::from("Theme reloaded from disk.");
            } else if matches!(
                watch.status(),
                ReloadStatus::Invalid(_) | ReloadStatus::Io(_)
            ) {
                self.status = format!("Kept last good theme: {:?}", watch.status());
            }
        }
        result
    }
    pub fn draw(&mut self, ui: &mut Ui) {
        ui.label("Input routing, nested overlays and saved state");
        ui.label("Gamepad API: host-fed directions / activate / cancel (simulated below).");
        ui.separator();
        ui.checkbox("Persisted enabled", &mut self.enabled);
        ui.text_input("Persisted name", &mut self.name);
        let changed = ui.drag_int("History gain", &mut self.gain, 0.5, 0..=100);
        if changed {
            if !self.history.is_grouping() {
                self.history.begin_group();
            }
            self.history.record(self.gain);
        }
        if !ui.mouse_down(0) && self.history.is_grouping() {
            self.history.end_group();
        }
        if ui.button("-1##history") {
            self.gain = (self.gain - 1).max(0);
            self.history.record(self.gain);
        }
        ui.same_line();
        if ui.button("+1##history") {
            self.gain = (self.gain + 1).min(100);
            self.history.record(self.gain);
        }
        ui.same_line();
        if ui.button("Undo / Ctrl+Z") {
            self.action = Some(Action::Undo);
        }
        ui.same_line();
        if ui.button("Redo / Ctrl+Y") {
            self.action = Some(Action::Redo);
        }
        let modifiers = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        if ui.shortcut(
            ui.widget_id("undo action"),
            Shortcut {
                key: Key::Z,
                modifiers,
                scope: ShortcutScope::Global,
            },
        ) {
            self.action = Some(Action::Undo);
        }
        if ui.shortcut(
            ui.widget_id("redo action"),
            Shortcut {
                key: Key::Y,
                modifiers,
                scope: ShortcutScope::Global,
            },
        ) {
            self.action = Some(Action::Redo);
        }
        ui.label("Drag edits form one undo step. Text/keybind capture owns shortcuts first.");
        let id = ui.widget_id("double click tile");
        let rect = ui.allocate([300., 28.]);
        ui.focusable(id, rect);
        if ui.double_clicked(id, rect, 0) {
            self.double_clicks += 1;
            self.gain = 10;
            self.history.record(self.gain);
        }
        ui.draw_rect_filled(
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            ui.theme_color(restricted_gui::theme::ThemeRole::Input),
        );
        ui.draw_text_at(
            [rect[0] + 8., rect[1] + 4.],
            "Double-click here to reset gain",
            ui.theme_color(restricted_gui::theme::ThemeRole::Text),
        );
        ui.label(&format!(
            "Double clicks: {} / target activations: {}",
            self.double_clicks, self.activation
        ));
        ui.separator();
        let first = ui.widget_id("Target A");
        for label in ["Target A", "Target B", "Target C"] {
            let id = ui.widget_id(label);
            if ui.button(label) {
                self.activation += 1;
                self.nav_focus = Some(id);
            }
            if ui.is_focused(id) {
                self.nav_focus = Some(id);
            }
            ui.same_line();
        }
        ui.label("navigation targets");
        for (index, (label, event)) in [
            ("Previous", Navigation::Previous),
            ("Next", Navigation::Next),
            ("Up", Navigation::Up),
            ("Down", Navigation::Down),
            ("Left", Navigation::Left),
            ("Right", Navigation::Right),
            ("Activate", Navigation::Activate),
            ("Cancel", Navigation::Cancel),
        ]
        .into_iter()
        .enumerate()
        {
            if ui.button(label) {
                self.navigation.push(event);
                ui.focus(self.nav_focus.unwrap_or(first));
            }
            if index % 4 != 3 {
                ui.same_line();
            }
        }
        ui.label("queued for next frame");
        ui.separator();
        let root = ui.widget_id("stack root");
        let child = ui.widget_id("stack child");
        let modal = ui.widget_id("stack modal");
        let modal2 = ui.widget_id("stack modal two");
        if ui.button("Nested popup") {
            ui.open_popup(root);
        }
        ui.same_line();
        if ui.button("Nested modal") {
            ui.open_modal(modal);
        }
        ui.label(&format!(
            "Open levels: {} / modals: {}. Escape closes one level.",
            ui.popup_depth(),
            ui.modal_depth()
        ));
        ui.popup(root, [580., 210., 900., 460.], |ui| {
            ui.label("Parent popup remains visible.");
            if ui.button("Open child popup") {
                ui.open_popup(child);
            }
            ui.popup(child, [820., 330., 1140., 570.], |ui| {
                ui.label("Child escapes its parent's clip.");
                if ui.button("Open child modal") {
                    ui.open_modal(modal2);
                }
                ui.modal(modal2, [700., 280., 1060., 540.], |ui| {
                    ui.label("Top modal owns input.");
                    if ui.button("Back##modal2") {
                        ui.close_popup();
                    }
                });
                if ui.button("Back##child") {
                    ui.close_popup();
                }
            });
            if ui.button("Close parent") {
                ui.close_popup();
            }
        });
        ui.modal(modal, [490., 200., 900., 470.], |ui| {
            ui.label("Modal parent blocks the background.");
            if ui.button("Second modal") {
                ui.open_modal(modal2);
            }
            ui.modal(modal2, [610., 280., 1020., 550.], |ui| {
                ui.label("Escape restores the first modal.");
                if ui.button("Back##nested-modal") {
                    ui.close_popup();
                }
            });
            if ui.button("Dismiss stack root") {
                ui.close_popup();
            }
        });
        ui.separator();
        for (index, (label, action)) in [
            ("Save config", Action::SaveConfig),
            ("Load config", Action::LoadConfig),
            ("Save theme", Action::SaveTheme),
            ("Load theme", Action::LoadTheme),
            ("Save layout", Action::SaveLayout),
            ("Load layout", Action::LoadLayout),
        ]
        .into_iter()
        .enumerate()
        {
            if ui.button(label) {
                self.action = Some(action);
            }
            if index % 3 != 2 {
                ui.same_line();
            }
        }
        ui.label("explicit snapshots");
        if ui.button("Watch theme file") {
            self.action = Some(Action::Watch);
        }
        ui.tooltip_text(&self.directory.join("preview.theme").to_string_lossy());
        ui.same_line();
        if ui.button("Stop watching") {
            self.action = Some(Action::StopWatch);
        }
        if let Some(watch) = &self.watch {
            ui.label(&format!("Watching {}", watch.path().display()));
        }
        ui.label(&self.status);
        ui.label("Config saves the three fields above; layout saves windows + Panels state.");
    }
}
fn read_bounded(path: &std::path::Path, max: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut data = Vec::new();
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    file.take(max as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() > max {
        Err(String::from("file exceeds snapshot size limit"))
    } else {
        Ok(data)
    }
}
