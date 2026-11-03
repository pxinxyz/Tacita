use crate::postprocess::{BLOOM, BLURRED, SCENE};
use tacita::{draw::TextureId, effects::*, shapes::RoundedRect, ui::Ui};
const OPTIONS: [(&str, u32); 14] = [
    ("Noise", SURFACE_NOISE),
    ("Film grain", SURFACE_GRAIN),
    ("Dithering", SURFACE_DITHER),
    ("Color grade", SURFACE_GRADE),
    ("Desaturation", SURFACE_DESATURATE),
    ("Vignette", SURFACE_VIGNETTE),
    ("Chromatic shift", SURFACE_CHROMATIC),
    ("Scanlines", SURFACE_SCANLINES),
    ("Grid", SURFACE_GRID),
    ("Dot grid", SURFACE_DOTS),
    ("Shimmer", SURFACE_SHIMMER),
    ("Specular sweep", SURFACE_SWEEP),
    ("UV distortion", SURFACE_DISTORT),
    ("Glass refraction", SURFACE_GLASS),
];
pub(super) struct State {
    flags: [bool; 14],
    style: SurfaceStyle,
    dither: usize,
    animate: bool,
    pub phase: f32,
    last_time: Option<f32>,
    blur_method: usize,
    levels: i32,
    pub radius: f32,
    pub threshold: f32,
    pub intensity: f32,
    alpha_mask: bool,
    fade_mask: bool,
    clicks: u32,
}
impl Default for State {
    fn default() -> Self {
        let mut flags = [false; 14];
        flags[5] = true;
        flags[11] = true;
        Self {
            flags,
            style: SurfaceStyle::default(),
            dither: 1,
            animate: true,
            phase: 0.,
            last_time: None,
            blur_method: 1,
            levels: 3,
            radius: 2.,
            threshold: 0.7,
            intensity: 1.3,
            alpha_mask: false,
            fade_mask: false,
            clicks: 0,
        }
    }
}
impl State {
    pub fn blur(&self) -> Blur {
        Blur {
            method: if self.blur_method == 0 {
                BlurMethod::Separable
            } else {
                BlurMethod::DualKawase
            },
            radius: self.radius,
            levels: self.levels as u32,
        }
    }
}
pub(super) fn show(
    ui: &mut Ui<'_>,
    state: &mut State,
    density: f32,
    mask_texture: TextureId,
    time: f32,
    motion_active: bool,
) {
    let elapsed = state
        .last_time
        .map_or(0., |last| (time - last).clamp(0., 0.1));
    state.last_time = Some(time);
    if motion_active && state.animate {
        state.phase = (state.phase + elapsed).rem_euclid(100.);
    }
    ui.label("GPU surfaces / captured backdrop / blur / bloom / stencil");
    ui.collapsing_header("Surface effect controls", |ui| {
        for (i, (label, _)) in OPTIONS.iter().enumerate() {
            if i % 2 == 1 {
                ui.same_line();
            }
            ui.checkbox(label, &mut state.flags[i]);
        }
        ui.checkbox("Animate surface phase", &mut state.animate);
        ui.filled_slider_float("Manual surface phase", &mut state.phase, 0.0..=20.0);
        ui.combo(
            "Dither pattern",
            &["Bayer 4 x 4", "Blue noise 8 x 8"],
            &mut state.dither,
        );
        ui.filled_slider_float("Noise strength", &mut state.style.noise, 0.0..=0.5);
        ui.filled_slider_float("Grain strength", &mut state.style.grain, 0.0..=0.5);
        ui.filled_slider_float("Dither strength", &mut state.style.dither, 0.0..=0.1);
        ui.filled_slider_float("Exposure (stops)", &mut state.style.exposure, -2.0..=2.0);
        ui.filled_slider_float("Contrast", &mut state.style.contrast, 0.0..=2.0);
        ui.filled_slider_float("Saturation", &mut state.style.saturation, 0.0..=2.0);
        ui.filled_slider_float(
            "Desaturation amount",
            &mut state.style.desaturation,
            0.0..=1.0,
        );
        ui.filled_slider_float("Pattern spacing", &mut state.style.spacing, 2.0..=48.0);
        ui.filled_slider_float("Scanline amount", &mut state.style.scanlines, 0.0..=1.0);
        ui.filled_slider_float("Grid width", &mut state.style.grid_width, 0.0..=4.0);
        ui.filled_slider_float("Dot radius", &mut state.style.dot_radius, 0.0..=4.0);
        ui.filled_slider_float("Vignette amount", &mut state.style.vignette, 0.0..=1.0);
        ui.filled_slider_float("Sweep brightness", &mut state.style.shimmer, 0.0..=1.0);
        ui.filled_slider_float("Sweep width", &mut state.style.sweep_width, 0.01..=0.6);
        ui.filled_slider_float("Sweep speed", &mut state.style.sweep_speed, -2.0..=2.0);
        ui.filled_slider_float(
            "UV displacement (texture px)",
            &mut state.style.distortion,
            0.0..=24.0,
        );
        ui.filled_slider_float(
            "Distortion frequency",
            &mut state.style.frequency,
            0.1..=12.0,
        );
        ui.filled_slider_float(
            "Chromatic offset (texture px)",
            &mut state.style.chromatic,
            0.0..=16.0,
        );
        ui.filled_slider_float(
            "Refraction (texture px)",
            &mut state.style.refraction,
            0.0..=24.0,
        );
    });
    state.style.flags = OPTIONS
        .iter()
        .enumerate()
        .filter(|(i, _)| state.flags[*i])
        .fold(0, |flags, (_, (_, flag))| flags | flag);
    state.style.phase = state.phase;
    state.style.pixel_scale = [density; 2];
    state.style.dither_kind = if state.dither == 0 {
        DitherKind::Bayer
    } else {
        DitherKind::BlueNoise
    };
    let width = ui.available_size()[0].clamp(160., 520.);
    ui.label("Source (left) / selected surface effects (right)");
    ui.preview_canvas("Surface comparison", [width, 175.], |ui, rect| {
        let middle = (rect[0] + rect[2]) * 0.5;
        ui.draw_surface(
            RoundedRect::new([rect[0], rect[1], middle - 4., rect[3]], [6.; 4]).unwrap(),
            Some(SCENE),
            [0., 0., 1., 1.],
            [255; 4],
            SurfaceStyle::default(),
        )
        .unwrap();
        ui.draw_surface(
            RoundedRect::new([middle + 4., rect[1], rect[2], rect[3]], [6.; 4]).unwrap(),
            Some(SCENE),
            [0., 0., 1., 1.],
            [255; 4],
            state.style,
        )
        .unwrap();
    });
    ui.collapsing_header("Blur and bloom controls", |ui| {
        ui.combo(
            "Blur method",
            &["Separable Gaussian", "Dual-Kawase"],
            &mut state.blur_method,
        );
        ui.filled_slider_float("Blur radius (source px)", &mut state.radius, 0.0..=12.0);
        ui.drag_int("Downsample levels", &mut state.levels, 1.0, 1..=6);
        ui.filled_slider_float("Bloom threshold", &mut state.threshold, 0.0..=1.0);
        ui.filled_slider_float("Bloom intensity", &mut state.intensity, 0.0..=3.0);
        ui.checkbox("Texture alpha blur mask", &mut state.alpha_mask);
        ui.checkbox("Feathered blur mask", &mut state.fade_mask);
    });
    ui.label("Frosted glass: the blurred capture follows the same background UVs");
    ui.preview_canvas("Captured backdrop", [width, 210.], |ui, rect| {
        ui.draw_surface(
            RoundedRect::new(rect, [6.; 4]).unwrap(),
            Some(SCENE),
            [0., 0., 1., 1.],
            [255; 4],
            SurfaceStyle::default(),
        )
        .unwrap();
        let [w, h] = [rect[2] - rect[0], rect[3] - rect[1]];
        let glass = [
            rect[0] + w * 0.2,
            rect[1] + h * 0.12,
            rect[0] + w * 0.82,
            rect[1] + h * 0.88,
        ];
        let alpha = state.alpha_mask.then(|| {
            ui.begin_alpha_mask(AlphaMask {
                texture: mask_texture,
                rect: glass,
                uv: [0., 0., 1., 1.],
            })
            .unwrap()
        });
        let fade = state.fade_mask.then(|| {
            ui.begin_gradient_mask(GradientMask {
                start: [glass[0], glass[1]],
                end: [glass[2], glass[1]],
                opacity: [1., 0.],
            })
            .unwrap()
        });
        ui.draw_surface(
            RoundedRect::new(glass, [18.; 4]).unwrap(),
            Some(BLURRED),
            [0.2, 0.12, 0.82, 0.88],
            [255; 4],
            SurfaceStyle {
                flags: SURFACE_GLASS | SURFACE_NOISE | SURFACE_DITHER,
                noise: 0.035,
                dither_kind: DitherKind::BlueNoise,
                pixel_scale: [density; 2],
                tint: [0.88, 0.92, 1., 0.96],
                ..Default::default()
            },
        )
        .unwrap();
        if let Some(scope) = fade {
            ui.end_mask(scope);
        }
        if let Some(scope) = alpha {
            ui.end_mask(scope);
        }
        ui.draw_text_at(
            [glass[0] + 16., glass[1] + 20.],
            "frosted glass",
            [240, 242, 250, 255],
        );
        ui.draw_text_at(
            [glass[0] + 16., glass[1] + 44.],
            "GPU snapshot + blur + refraction",
            [230, 232, 240, 255],
        );
    });
    ui.label("Captured scene (left) / threshold bloom (right)");
    ui.preview_canvas("Bloom comparison", [width, 170.], |ui, rect| {
        let middle = (rect[0] + rect[2]) * 0.5;
        for (texture, region) in [
            (SCENE, [rect[0], rect[1], middle - 4., rect[3]]),
            (BLOOM, [middle + 4., rect[1], rect[2], rect[3]]),
        ] {
            ui.draw_surface(
                RoundedRect::new(region, [6.; 4]).unwrap(),
                Some(texture),
                [0., 0., 1., 1.],
                [255; 4],
                SurfaceStyle::default(),
            )
            .unwrap();
        }
    });
    ui.label("Nested GPU stencil: hard contours, rounded hit tests, popup escape");
    ui.preview_canvas("Stencil clip", [width, 200.], |ui, rect| {
        let outer = RoundedRect::new(rect, [40., 4., 30., 14.]).unwrap();
        ui.stencil_clip("Outer stencil", outer, |ui| {
            ui.draw_surface(
                RoundedRect::new(rect, [0.; 4]).unwrap(),
                Some(SCENE),
                [0., 0., 1., 1.],
                [255; 4],
                SurfaceStyle {
                    flags: SURFACE_GRID,
                    ..Default::default()
                },
            )
            .unwrap();
            ui.draw_text_at([rect[0] + 40., rect[1] + 10.], "Outer stencil", [255; 4]);
            let inner = [rect[0] + 28., rect[1] + 40., rect[2] - 28., rect[3] - 20.];
            ui.stencil_clip(
                "Inner stencil",
                RoundedRect::new(inner, [26.; 4]).unwrap(),
                |ui| {
                    ui.draw_surface(
                        RoundedRect::new(inner, [0.; 4]).unwrap(),
                        Some(BLURRED),
                        [0., 0., 1., 1.],
                        [255; 4],
                        SurfaceStyle::default(),
                    )
                    .unwrap();
                    ui.rounded_clip(
                        "Stencil content inset",
                        RoundedRect::new(
                            [
                                inner[0] + 18.,
                                inner[1] + 18.,
                                inner[2] - 18.,
                                inner[3] - 18.,
                            ],
                            [0.; 4],
                        )
                        .unwrap(),
                        |ui| {
                            let popup = ui.widget_id("Stencil popup");
                            if ui.button("Open popup outside clip") {
                                ui.open_popup(popup);
                                state.clicks += 1;
                                eprintln!("[demo] stencil click {}", state.clicks);
                            }
                            ui.popup(
                                popup,
                                [rect[2] - 90., rect[1] - 30., rect[2] + 120., rect[1] + 85.],
                                |ui| {
                                    ui.label("Popup escapes both contours");
                                    if ui.button("Close stencil popup") {
                                        ui.close_popup();
                                    }
                                },
                            );
                        },
                    )
                    .unwrap();
                },
            )
            .unwrap();
        })
        .unwrap();
    });
    ui.separator();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tacita::{context::Context, input::InputState, ScrollFlags};

    #[test]
    fn gallery_canvases_emit_valid_geometry_at_default_and_narrow_window_sizes() {
        let mut ctx =
            Context::new(include_bytes!("../../core/tests/assets/DejaVuSans.ttf")).unwrap();
        let mut state = State::default();
        for width in [300., 120., 160., 760.] {
            ctx.set_window_rect("Effects", [60., 60.], [width, 400.]);
            ctx.set_input(InputState {
                viewport_size: [1350., 800.],
                mouse_pos: [-1000.; 2],
                ..Default::default()
            });
            ctx.new_frame(0.016);
            ctx.ui().window("Effects", ScrollFlags::BAR, |ui| {
                show(ui, &mut state, 1., TextureId(3), 0., false);
            });
            let draw = ctx.render();
            for texture in [SCENE, BLURRED, BLOOM] {
                assert!(draw.commands.iter().any(|cmd| cmd.texture == texture));
            }
            assert!(draw
                .vertices
                .iter()
                .all(|v| v.pos.iter().all(|p| p.is_finite())));
        }
    }
}
