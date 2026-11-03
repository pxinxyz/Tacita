use crate::{renderer::Renderer, shader_watch::ShaderWatcher};
use restricted_gui::{
    draw::TextureId,
    rendering::{AlphaMode, DrawOptimization},
    ui::Ui,
};
use std::{path::PathBuf, time::Duration};

const STRAIGHT: TextureId = TextureId(66);
const ASSOCIATED: TextureId = TextureId(67);
#[derive(Default)]
pub struct State {
    sort: bool,
    instances: bool,
    samples: usize,
    premultiplied: bool,
    applied: Option<(usize, bool, bool, bool)>,
    supported: [bool; 4],
    ready: bool,
    reload: Option<u8>,
    watching: bool,
    watcher: Option<ShaderWatcher>,
    message: String,
    watch_error: Option<String>,
    counts: (u32, usize),
}
impl State {
    pub fn smoke_settings(&mut self, case: crate::renderer_smoke::Case) {
        self.samples = case.samples;
        self.premultiplied = case.alpha == AlphaMode::Premultiplied;
        self.sort = case.optimization.sort_disjoint;
        self.instances = case.optimization.instance_quads;
    }
    pub fn prepare(&mut self, renderer: &mut Renderer) {
        self.counts = renderer.optimization_stats();
        self.supported = renderer.supported_samples();
        if !self.ready {
            let straight: Vec<u8> = (0..64 * 64)
                .flat_map(|p| {
                    let x = (p % 64) as f32 - 31.5;
                    let y = (p / 64) as f32 - 31.5;
                    let alpha = ((28. - (x * x + y * y).sqrt()).clamp(0., 1.) * 255.).round() as u8;
                    [125, 207, 255, alpha]
                })
                .collect();
            let associated: Vec<u8> = straight
                .chunks_exact(4)
                .flat_map(|p| {
                    [
                        ((p[0] as u16 * p[3] as u16 + 127) / 255) as u8,
                        ((p[1] as u16 * p[3] as u16 + 127) / 255) as u8,
                        p[3],
                        p[3],
                    ]
                })
                .collect();
            renderer.add_host_texture(STRAIGHT, &straight, 64, 64);
            renderer.add_host_texture(ASSOCIATED, &associated, 64, 64);
            if let Err(e) = renderer.set_texture_alpha_mode(ASSOCIATED, AlphaMode::Premultiplied) {
                self.message = e;
            }
            self.ready = true;
        }
        let desired = (self.samples, self.premultiplied, self.sort, self.instances);
        if self.applied != Some(desired) {
            eprintln!("[renderer] requested settings: {desired:?}");
            let result = pollster::block_on(renderer.configure_rendering(
                [1, 2, 4, 8][self.samples],
                if self.premultiplied {
                    AlphaMode::Premultiplied
                } else {
                    AlphaMode::Straight
                },
                DrawOptimization {
                    sort_disjoint: self.sort,
                    instance_quads: self.instances,
                },
            ));
            match result {
                Ok(()) => {
                    self.applied = Some(desired);
                    self.message = format!(
                        "{}x samples; renderer settings applied",
                        [1, 2, 4, 8][self.samples]
                    );
                }
                Err(e) => {
                    self.message = e;
                    let old = self.applied.unwrap_or((0, false, false, false));
                    (self.samples, self.premultiplied, self.sort, self.instances) = old;
                    self.applied = Some(old);
                }
            }
            eprintln!("[renderer] {}", self.message);
        }
        if let Some(action) = self.reload.take() {
            let source=match action {
                1=>Renderer::builtin_shader().replace("var color = surface_color(input.col*filtered(uv)*painting,uv,input.logical,input.col*painting);","var color = surface_color(input.col*filtered(uv)*painting,uv,input.logical,input.col*painting); color = vec4<f32>(color.rgb * vec3<f32>(1.0, 0.8, 1.0), color.a);"),
                2=>"this is deliberately invalid WGSL".into(),
                _=>Renderer::builtin_shader().into(),
            };
            self.message = match pollster::block_on(renderer.reload_shader(&source)) {
                Ok(()) => "shader replaced; textures and geometry retained".into(),
                Err(e) => format!("reload rejected; last good shader retained: {e}"),
            };
            eprintln!("[demo] {}", self.message);
        }
        if self.watching {
            if self.watcher.is_none() {
                let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../backends/shaders");
                match ShaderWatcher::new(
                    ["blue_noise.wgsl", "gui.wgsl", "surface.wgsl"]
                        .map(|p| base.join(p))
                        .to_vec(),
                    Duration::from_millis(500),
                ) {
                    Ok(watch) => self.watcher = Some(watch),
                    Err(e) => self.message = format!("watch: {e:?}"),
                }
            }
            if let Some(watch) = &mut self.watcher {
                match watch.poll() {
                    Ok(Some(source)) => {
                        self.watch_error = None;
                        self.message = match pollster::block_on(renderer.reload_shader(source)) {
                            Ok(()) => "changed shader files reloaded".into(),
                            Err(e) => {
                                format!("file reload rejected; last good shader retained: {e}")
                            }
                        };
                        eprintln!("[demo] {}", self.message);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        let error = format!("watch: {e:?}");
                        if self.watch_error.as_ref() != Some(&error) {
                            self.message = error.clone();
                            eprintln!("[demo] {error}");
                            self.watch_error = Some(error);
                        }
                    }
                }
            }
        } else {
            self.watcher = None;
            self.watch_error = None;
        }
    }
    pub fn draw(&mut self, ui: &mut Ui<'_>) {
        ui.label("Renderer completion / batching, samples and alpha");
        ui.checkbox("Sort disjoint commands", &mut self.sort);
        ui.same_line();
        ui.checkbox("Instance indexed quads", &mut self.instances);
        ui.combo(
            "Multisample count",
            &["1x", "2x", "4x", "8x"],
            &mut self.samples,
        );
        ui.checkbox("Premultiplied output", &mut self.premultiplied);
        ui.label(&format!(
            "Supported 1/2/4/8x: {:?} / {} moved commands / {} quad instances",
            self.supported, self.counts.0, self.counts.1
        ));
        ui.label("Alpha pairs: identical edges and tint over the same checkerboard");
        let rect = ui.allocate([320., 88.]);
        for panel in 0..2 {
            let x = rect[0] + panel as f32 * 160.;
            for row in 0..8 {
                for column in 0..14 {
                    let shade = if (row + column) % 2 == 0 {
                        [22, 25, 35, 255]
                    } else {
                        [50, 55, 72, 255]
                    };
                    ui.draw_rect_filled(
                        [x + column as f32 * 10., rect[1] + row as f32 * 10.],
                        [
                            x + (column + 1) as f32 * 10.,
                            rect[1] + (row + 1) as f32 * 10.,
                        ],
                        shade,
                    );
                }
            }
            ui.draw_image(
                if panel == 0 { STRAIGHT } else { ASSOCIATED },
                [x + 34., rect[1] + 8.],
                [x + 98., rect[1] + 72.],
                [0.; 2],
                [1.; 2],
                [255, 210, 255, 180],
            );
        }
        ui.label(
            "Left: straight input. Right: premultiplied input. Output toggle must preserve both.",
        );
        let rect = ui.allocate([320., 64.]);
        let points = [
            [rect[0] + 5., rect[1] + 56.],
            [rect[0] + 147.3, rect[1] + 3.2],
            [rect[0] + 307.7, rect[1] + 49.6],
        ];
        ui.draw_polygon_filled(&points, [125, 207, 255, 230]);
        ui.label("Triangle edge: compare 1x and 4x; disable Canvas AA to isolate MSAA.");
        if ui.button("Reload built-in shader") {
            self.reload = Some(0);
        }
        ui.same_line();
        if ui.button("Tint shader") {
            self.reload = Some(1);
        }
        ui.same_line();
        if ui.button("Reject bad shader") {
            self.reload = Some(2);
        }
        ui.checkbox("Watch shader files (500 ms)", &mut self.watching);
        ui.label("Watch reads backends/shaders/{blue_noise,gui,surface}.wgsl; errors keep the last shader.");
        if !self.message.is_empty() {
            ui.text_block(
                &self.message,
                ui.available_size()[0].max(160.),
                restricted_gui::text::TextOptions {
                    wrap: true,
                    max_lines: 6,
                    overflow: restricted_gui::text::TextOverflow::Ellipsis,
                    ..Default::default()
                },
            );
        }
        ui.separator();
    }
}
