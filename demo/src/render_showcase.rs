use crate::renderer::Renderer;
use tacita::{
    context::Context,
    diagnostics::{InputRecording, ReplayCursor},
    draw::{DrawCmd, DrawData, DrawVert, TextureId},
    images::{ImageFit, ImageRegion, NineSlice, SpriteAtlas, SpriteId},
    input::InputState,
    rendering::{ColorMode, RenderStats},
    style::Style,
    ui::Ui,
    ScrollFlags,
};
const ATLAS: TextureId = TextureId(60);
const LEGACY: TextureId = TextureId(61);
const MANAGED: TextureId = TextureId(62);
const PAD: TextureId = TextureId(63);
const PAD_FONT: TextureId = TextureId(64);
const MANAGED_PREVIEW: TextureId = TextureId(65);
#[derive(Clone, Copy)]
enum Action {
    Record,
    Stop,
    Replay,
    Reset,
}
pub struct State {
    rendering: crate::rendering_showcase::State,
    atlas: SpriteAtlas,
    sprites: [SpriteId; 2],
    atlas_generation: u64,
    preview_ready: bool,
    fit: usize,
    image_size: f32,
    nine_width: f32,
    clicks: u32,
    reserve: bool,
    compact: bool,
    trim: bool,
    pad: Option<[f32; 4]>,
    visible: bool,
    context: Context,
    recording: InputRecording,
    cursor: ReplayCursor,
    action: Option<Action>,
    recording_on: bool,
    playing: bool,
    sample_clicks: u32,
    sample_value: f32,
    expected: Option<(u32, f32)>,
    status: String,
    font_generation: u64,
}
fn sample_context() -> Context {
    let mut ctx = Context::new(crate::DEJAVU_SANS).expect("bundled font");
    ctx.set_window_rect("replay pad", [0.; 2], [320., 160.]);
    ctx
}
impl Default for State {
    fn default() -> Self {
        let mut atlas = SpriteAtlas::new(ATLAS, 128, 1024, 2).expect("bounded sprite atlas");
        let checker: Vec<u8> = (0..64 * 64)
            .flat_map(|p| {
                let x = p % 64;
                let y = p / 64;
                if (x / 8 + y / 8) % 2 == 0 {
                    [125, 207, 255, 255]
                } else {
                    [99, 91, 255, 255]
                }
            })
            .collect();
        let border: Vec<u8> = (0..32 * 32)
            .flat_map(|p| {
                let x = p % 32;
                let y = p / 32;
                if !(6..26).contains(&x) || !(6..26).contains(&y) {
                    [125, 207, 255, 255]
                } else {
                    [21, 24, 32, 255]
                }
            })
            .collect();
        let sprites = [
            atlas.insert(&checker, [64; 2]).unwrap(),
            atlas.insert(&border, [32; 2]).unwrap(),
        ];
        Self {
            rendering: Default::default(),
            atlas,
            sprites,
            atlas_generation: 0,
            preview_ready: false,
            fit: 1,
            image_size: 160.,
            nine_width: 260.,
            clicks: 0,
            reserve: false,
            compact: false,
            trim: false,
            pad: None,
            visible: false,
            context: sample_context(),
            recording: InputRecording::new(),
            cursor: ReplayCursor::default(),
            action: None,
            recording_on: false,
            playing: false,
            sample_clicks: 0,
            sample_value: 0.25,
            expected: None,
            status: String::new(),
            font_generation: u64::MAX,
        }
    }
}
impl State {
    pub fn smoke_settings(&mut self, case: crate::renderer_smoke::Case) {
        self.rendering.smoke_settings(case);
    }
    pub fn prepare_rendering(&mut self, renderer: &mut Renderer) {
        self.rendering.prepare(renderer);
    }
    fn reset_sample(&mut self) {
        self.context = sample_context();
        self.sample_clicks = 0;
        self.sample_value = 0.25;
        self.font_generation = u64::MAX;
    }
    pub fn prepare(
        &mut self,
        renderer: &mut Renderer,
        input: InputState<'_>,
        dt: f32,
        style: Style,
        scale: f32,
    ) {
        if self.reserve {
            renderer.set_geometry_capacity(4 * 1024 * 1024, 1024 * 1024);
            self.reserve = false;
        }
        if self.compact {
            renderer.set_geometry_capacity(128 * 1024, 64 * 1024);
            self.compact = false;
        }
        if self.trim {
            renderer.trim_geometry_pool();
            self.trim = false;
        }
        if self.atlas.generation() != self.atlas_generation {
            let size = self.atlas.size();
            renderer.add_host_texture(ATLAS, self.atlas.pixels(), size[0], size[1]);
            self.atlas_generation = self.atlas.generation();
        }
        if !self.visible {
            return;
        }
        self.visible = false;
        match self.action.take() {
            Some(Action::Record) => {
                self.reset_sample();
                self.recording.clear();
                self.recording_on = true;
                self.playing = false;
                self.expected = None;
                self.status = "recording the pad".into();
            }
            Some(Action::Stop) => {
                self.recording_on = false;
                self.playing = false;
                self.expected = Some((self.sample_clicks, self.sample_value));
                self.status = "recording stopped".into();
            }
            Some(Action::Replay) => {
                self.reset_sample();
                self.cursor.restart();
                self.playing = !self.recording.is_empty();
                self.recording_on = false;
                self.status = "replaying recorded input + dt".into();
            }
            Some(Action::Reset) => {
                self.reset_sample();
                self.recording_on = false;
                self.playing = false;
                self.status = "pad reset; recording retained".into();
            }
            None => {}
        }
        let _ = self
            .context
            .update_display_metrics(tacita::display::DisplayMetrics {
                physical_size: [(320. * scale) as u32, (160. * scale) as u32],
                scale,
            });
        self.context.style = style;
        let mouse = self.pad.map_or([-100.; 2], |rect| {
            [input.mouse_pos[0] - rect[0], input.mouse_pos[1] - rect[1]]
        });
        let local = InputState {
            mouse_pos: mouse,
            viewport_size: [320., 160.],
            ..input
        };
        if self.playing {
            if let Some(frame) = self.cursor.next(&self.recording) {
                self.context.replay_frame(frame);
            } else {
                self.playing = false;
                self.status = format!(
                    "replay complete: {} clicks, {:.3}",
                    self.sample_clicks, self.sample_value
                );
                self.context.set_input(InputState {
                    viewport_size: [320., 160.],
                    ..Default::default()
                });
                self.context.new_frame(dt);
            }
        } else {
            if self.recording_on && self.recording.push(local, dt, &[]).is_err() {
                self.recording_on = false;
                self.status = "recording reached its bound".into();
            }
            self.context.set_input(local);
            self.context.new_frame(dt);
        }
        let clicks = &mut self.sample_clicks;
        let value = &mut self.sample_value;
        self.context
            .ui()
            .window("replay pad", ScrollFlags::NONE, |ui| {
                if ui.button("Click me") {
                    *clicks += 1;
                }
                ui.same_line();
                ui.label(&format!("{} clicks", *clicks));
                ui.slider_float("Value", value, 0.0..=1.0);
            });
        let font = self.context.font();
        if font.atlas_generation() != self.font_generation {
            let (pixels, w, h) = font.atlas_pixels();
            let rgba: Vec<u8> = pixels
                .iter()
                .flat_map(|&alpha| [255, 255, 255, alpha])
                .collect();
            renderer.add_host_texture(PAD_FONT, &rgba, w, h);
            self.font_generation = font.atlas_generation();
        }
        let draw = self.context.render();
        let commands: Vec<_> = draw
            .commands
            .iter()
            .map(|c| DrawCmd {
                texture: if c.texture.0 < 2 { PAD_FONT } else { c.texture },
                ..*c
            })
            .collect();
        let draw = DrawData {
            commands: &commands,
            ..draw
        };
        if let Err(error) = renderer.render_to_texture(
            PAD,
            [(320. * scale) as u32, (160. * scale) as u32],
            ColorMode::Legacy,
            scale,
            &draw,
            [0.03, 0.035, 0.05, 1.],
        ) {
            self.status = error;
        }
        let vertices = [
            DrawVert {
                pos: [8., 8.],
                uv: [0.; 2],
                col: [128, 128, 128, 128],
            },
            DrawVert {
                pos: [152., 8.],
                uv: [0.; 2],
                col: [128, 128, 128, 128],
            },
            DrawVert {
                pos: [152., 88.],
                uv: [0.; 2],
                col: [128, 128, 128, 128],
            },
            DrawVert {
                pos: [8., 88.],
                uv: [0.; 2],
                col: [128, 128, 128, 128],
            },
        ];
        let indices = [0, 1, 2, 0, 2, 3];
        let commands = [DrawCmd {
            elem_count: 6,
            clip_rect: [0., 0., 160., 96.],
            texture: TextureId::WHITE,
            effects: 0,
        }];
        let draw = DrawData {
            vertices: &vertices,
            indices: &indices,
            commands: &commands,
            effects: &[],
        };
        for (id, mode) in [(LEGACY, ColorMode::Legacy), (MANAGED, ColorMode::Srgb)] {
            if let Err(error) =
                renderer.render_to_texture(id, [160, 96], mode, 1., &draw, [0., 0., 0., 1.])
            {
                self.status = error;
            }
        }
        if !self.preview_ready
            && renderer
                .register_encoded_preview(MANAGED, MANAGED_PREVIEW)
                .is_ok()
        {
            self.preview_ready = true;
        }
    }
    pub fn draw(&mut self, ui: &mut Ui<'_>, stats: RenderStats, timing_supported: bool) {
        self.visible = true;
        self.rendering.draw(ui);
        ui.label("Renderer / storage");
        ui.label(&format!(
            "{} draws / {:.1} KiB uploaded / {} spare buffers ({:.1} KiB)",
            stats.draw_calls,
            stats.uploaded_bytes as f64 / 1024.,
            stats.pool_entries,
            stats.pool_bytes as f64 / 1024.
        ));
        ui.label(&format!(
            "{} allocations / {} reuses / GPU {}",
            stats.allocations,
            stats.reuses,
            stats.gpu_ms.map_or_else(
                || if timing_supported {
                    "waiting for timestamp".into()
                } else {
                    "timestamps unavailable".into()
                },
                |v| format!("{v:.3} ms")
            )
        ));
        if ui.button("Reserve large buffers") {
            self.reserve = true;
        }
        ui.same_line();
        if ui.button("Compact buffers") {
            self.compact = true;
        }
        ui.same_line();
        if ui.button("Trim spares") {
            self.trim = true;
        }
        ui.separator();
        ui.label("Stable atlas / image button");
        ui.combo("Fit", &["stretch", "contain", "cover"], &mut self.fit);
        ui.slider_float("Image width", &mut self.image_size, 80.0..=260.0);
        let region = self.atlas.region(self.sprites[0]).unwrap();
        let fit = [ImageFit::Stretch, ImageFit::Contain, ImageFit::Cover][self.fit];
        if ui
            .image_button(
                "sprite button",
                region,
                [self.image_size, 80.],
                fit,
                [255; 4],
            )
            .unwrap()
        {
            self.clicks += 1;
        }
        ui.label(&format!(
            "Image releases: {} / {} sprites / atlas {:?}",
            self.clicks,
            self.atlas.len(),
            self.atlas.size()
        ));
        if ui.button("Grow atlas (IDs stay stable)") {
            let rgba = vec![100; 64 * 64 * 4];
            for _ in 0..8 {
                if self.atlas.insert(&rgba, [64; 2]).is_err() {
                    break;
                }
            }
        }
        ui.label("Nine-slice: fixed borders while the center stretches");
        ui.slider_float("Nine-slice width", &mut self.nine_width, 4.0..=340.0);
        let rect = ui.allocate([self.nine_width, 64.]);
        ui.draw_nine_slice(
            self.atlas.region(self.sprites[1]).unwrap(),
            rect,
            NineSlice {
                source: [6.; 4],
                destination: [6.; 4],
            },
            [255; 4],
        )
        .unwrap();
        ui.separator();
        ui.label("Offscreen: encoded-space legacy / linear-light sRGB");
        let region = |id, size| ImageRegion {
            texture: id,
            uv_min: [0.; 2],
            uv_max: [1.; 2],
            size,
        };
        ui.image(
            region(LEGACY, [160., 96.]),
            [160., 96.],
            ImageFit::Stretch,
            [255; 4],
        )
        .unwrap();
        ui.same_line();
        ui.image(
            region(MANAGED_PREVIEW, [160., 96.]),
            [160., 96.],
            ImageFit::Stretch,
            [255; 4],
        )
        .unwrap();
        ui.label(
            "Same translucent gray; right blends in linear light and displays the encoded result.",
        );
        ui.separator();
        ui.label("Record / replay: interact inside the pad below");
        if ui.button("Record pad") {
            self.action = Some(Action::Record);
        }
        ui.same_line();
        if ui.button("Stop recording") {
            self.action = Some(Action::Stop);
        }
        ui.same_line();
        if ui.button("Replay") {
            self.action = Some(Action::Replay);
        }
        ui.same_line();
        if ui.button("Reset pad") {
            self.action = Some(Action::Reset);
        }
        self.pad = Some(
            ui.image(
                region(PAD, [320., 160.]),
                [320., 160.],
                ImageFit::Stretch,
                [255; 4],
            )
            .unwrap(),
        );
        ui.label(&format!(
            "{} frames / playback {} / {}",
            self.recording.len(),
            self.cursor.position(),
            self.status
        ));
        if let Some((clicks, value)) = self.expected {
            ui.label(&format!(
                "Recorded final: {clicks} clicks / {value:.3}; current: {} / {:.3}",
                self.sample_clicks, self.sample_value
            ));
        }
    }
}
