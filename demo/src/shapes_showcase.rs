use tacita::{
    draw::TextureId,
    shapes::{BorderPlacement, RoundedRect, Shadow},
    ui::Ui,
};
pub(super) struct State {
    pub surface: crate::surface_showcase::State,
    radii: [f32; 4],
    placement: usize,
    width: f32,
    phase: f32,
    blur: f32,
    shadow: bool,
    glow: bool,
    inset: bool,
    pill: bool,
    clicks: u32,
    paint_kind: usize,
    spread: usize,
    paint_phase: f32,
    animate: bool,
    last_time: f32,
    origin: [f32; 2],
    radii_scale: [f32; 2],
    gradient_border: bool,
    alpha_mask: bool,
    fade_mask: bool,
    mask_clicks: u32,
    sdf_reach: f32,
    sdf_glow: bool,
    sdf_edge: bool,
    sdf_highlight: bool,
    light_angle: f32,
    light_power: f32,
    rotate_light: bool,
    reveal_direction: usize,
    reveal_progress: f32,
    reveal_softness: f32,
    reveal_inverted: bool,
    animate_reveal: bool,
    reveal_clock: f32,
    reveal_clicks: u32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            surface: crate::surface_showcase::State::default(),
            radii: [28.0, 0.0, 14.0, 6.0],
            placement: 0,
            width: 1.0,
            phase: 0.0,
            blur: 12.0,
            shadow: true,
            glow: true,
            inset: false,
            pill: true,
            clicks: 0,
            paint_kind: 2,
            spread: 1,
            paint_phase: 0.0,
            animate: true,
            last_time: 0.0,
            origin: [0.5; 2],
            radii_scale: [0.65, 0.8],
            gradient_border: true,
            alpha_mask: true,
            fade_mask: true,
            mask_clicks: 0,
            sdf_reach: 8.0,
            sdf_glow: true,
            sdf_edge: false,
            sdf_highlight: true,
            light_angle: 3.9,
            light_power: 2.0,
            rotate_light: true,
            reveal_direction: 0,
            reveal_progress: 0.5,
            reveal_softness: 0.12,
            reveal_inverted: false,
            animate_reveal: true,
            reveal_clock: 0.5,
            reveal_clicks: 0,
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn show(
    ui: &mut Ui<'_>,
    state: &mut State,
    texture: TextureId,
    density: f32,
    style: tacita::style::Style,
    mask_texture: TextureId,
    time: f32,
    motion_active: bool,
) {
    crate::surface_showcase::show(
        ui,
        &mut state.surface,
        density,
        mask_texture,
        time,
        motion_active,
    );
    ui.label("Per-corner shapes / physical-pixel coverage / rounded content");
    ui.collapsing_header("Shape controls", |ui| {
        for (i, label) in ["Top left", "Top right", "Bottom right", "Bottom left"]
            .iter()
            .enumerate()
        {
            ui.filled_slider_float(label, &mut state.radii[i], 0.0..=48.0);
        }
        ui.combo(
            "Border placement",
            &["Inner", "Center", "Outer"],
            &mut state.placement,
        );
        ui.filled_slider_float("Border width (physical px)", &mut state.width, 0.125..=8.0);
        ui.filled_slider_float("Fractional physical offset", &mut state.phase, 0.0..=1.0);
        ui.filled_slider_float("Shadow/glow reach", &mut state.blur, 0.0..=28.0);
        ui.checkbox("Outer shadow", &mut state.shadow);
        ui.same_line();
        ui.checkbox("Soft glow", &mut state.glow);
        ui.checkbox("Inner shadow", &mut state.inset);
    });
    let color = style.text;
    let fill = style.widget_bg;
    let accent = style.widget_bg_active;
    let width = ui.available_size()[0].clamp(160.0, 520.0);
    ui.preview_canvas("Shapes preview##demo", [width, 280.0], |ui, rect| {
        let x = rect[0] + 28.0 + state.phase / density;
        let y = rect[1] + 30.0 + state.phase / density;
        let sample = RoundedRect::new([x, y, x + width - 56.0, y + 96.0], state.radii)
            .expect("bounded demo rectangle");
        if state.shadow {
            ui.draw_shadow(
                sample,
                Shadow {
                    offset: [3.0, 6.0],
                    spread: 0.0,
                    blur: state.blur,
                    color: [0, 0, 0, 130],
                    inset: false,
                },
            )
            .unwrap();
        }
        if state.glow {
            let mut glow = accent;
            glow[3] = 60;
            ui.draw_glow(sample, glow, state.blur).unwrap();
        }
        ui.draw_rounded_rect(sample, fill).unwrap();
        if state.inset {
            ui.draw_shadow(
                sample,
                Shadow {
                    offset: [2.0, 3.0],
                    spread: 0.0,
                    blur: state.blur,
                    color: [0, 0, 0, 170],
                    inset: true,
                },
            )
            .unwrap();
        }
        ui.draw_rounded_border(
            sample,
            accent,
            state.width / density,
            [
                BorderPlacement::Inner,
                BorderPlacement::Center,
                BorderPlacement::Outer,
            ][state.placement],
        )
        .unwrap();
        ui.draw_text_at([x + 24.0, y + 28.0], "Independent corners", color);
        ui.draw_text_at([x + 24.0, y + 52.0], "No atlas or renderer changes", color);
        let clipped = RoundedRect::new(
            [x, y + 138.0, x + width - 56.0, y + 230.0],
            [40.0, 12.0, 40.0, 0.0],
        )
        .unwrap();
        ui.rounded_clip("Rounded content##demo", clipped, |ui| {
            ui.draw_image(
                texture,
                [x - 20.0, y + 120.0],
                [x + width, y + 260.0],
                [0.0; 2],
                [1.0; 2],
                [255, 255, 255, 60],
            );
            ui.allocate([1.0, 8.0]);
            ui.pill_toggle("Clipped ordinary control", &mut state.pill);
            if ui.button("Click inside the silhouette") {
                state.clicks += 1;
                eprintln!("[demo] rounded clip clicks: {}", state.clicks);
            }
        })
        .unwrap();
        ui.draw_rounded_border(clipped, accent, 1.0 / density, BorderPlacement::Inner)
            .unwrap();
    });
    ui.label(&format!(
        "{} clipped button clicks. Corner cutouts reject hover and clicks.",
        state.clicks
    ));
    ui.label("Clip contours are geometric; shadows/glows use bounded distance bands.");
    effects_showcase(
        ui,
        state,
        texture,
        mask_texture,
        density,
        style,
        time,
        motion_active,
    );
}

#[allow(clippy::too_many_arguments)]
fn effects_showcase(
    ui: &mut Ui<'_>,
    state: &mut State,
    texture: TextureId,
    mask_texture: TextureId,
    density: f32,
    style: tacita::style::Style,
    time: f32,
    motion_active: bool,
) {
    use tacita::effects::{
        AlphaMask, GradientKind, GradientMask, GradientPaint, GradientSpread, GradientStop,
    };
    let elapsed = (time - state.last_time).clamp(0.0, 0.1);
    state.last_time = time;
    if state.animate && motion_active {
        state.paint_phase = (state.paint_phase + elapsed * 0.12).rem_euclid(1.0);
    }
    ui.separator();
    ui.label("Per-pixel gradients and masks");
    ui.collapsing_header("Paint controls", |ui| {
        ui.combo(
            "Gradient",
            &["Linear", "Radial", "Conic"],
            &mut state.paint_kind,
        );
        ui.combo("Spread", &["Clamp", "Repeat", "Mirror"], &mut state.spread);
        ui.checkbox("Animate phase", &mut state.animate);
        ui.filled_slider_float("Paint phase", &mut state.paint_phase, 0.0..=1.0);
        ui.filled_slider_float("Origin X", &mut state.origin[0], 0.0..=1.0);
        ui.filled_slider_float("Origin Y", &mut state.origin[1], 0.0..=1.0);
        if state.paint_kind == 1 {
            ui.filled_slider_float("Radial width", &mut state.radii_scale[0], 0.05..=1.5);
            ui.filled_slider_float("Radial height", &mut state.radii_scale[1], 0.05..=1.5);
        }
        ui.checkbox("Gradient border", &mut state.gradient_border);
        ui.checkbox("Texture alpha mask", &mut state.alpha_mask);
        ui.checkbox("Nested left-to-right fade", &mut state.fade_mask);
    });
    let stops = [
        GradientStop {
            position: 0.0,
            color: [99, 91, 255, 255],
        },
        GradientStop {
            position: 0.3,
            color: [0, 229, 255, 255],
        },
        GradientStop {
            position: 0.6,
            color: [255, 95, 148, 255],
        },
        GradientStop {
            position: 1.0,
            color: [99, 91, 255, 255],
        },
    ];
    let paint = GradientPaint {
        kind: [
            GradientKind::Linear,
            GradientKind::Radial,
            GradientKind::Conic,
        ][state.paint_kind],
        origin: state.origin,
        vector: if state.paint_kind == 1 {
            state.radii_scale
        } else {
            [1.0, 0.0]
        },
        spread: [
            GradientSpread::Clamp,
            GradientSpread::Repeat,
            GradientSpread::Mirror,
        ][state.spread],
        phase: state.paint_phase,
    };
    let width = ui.available_size()[0].clamp(160.0, 520.0);
    ui.preview_canvas("Gradient preview##demo", [width, 156.0], |ui, rect| {
        let sample = RoundedRect::new(
            [
                rect[0] + 18.0,
                rect[1] + 18.0,
                rect[2] - 18.0,
                rect[3] - 18.0,
            ],
            state.radii,
        )
        .unwrap();
        ui.draw_shape_gradient(sample, &stops, paint).unwrap();
        let badge = RoundedRect::new(
            [
                sample.rect[0] + 18.0,
                sample.rect[1] + 28.0,
                sample.rect[2] - 18.0,
                sample.rect[3] - 28.0,
            ],
            state.radii,
        )
        .unwrap();
        ui.draw_rounded_rect(badge, style.window_bg).unwrap();
        if state.gradient_border {
            ui.draw_gradient_border(
                badge,
                &stops,
                paint,
                state.width / density,
                [
                    BorderPlacement::Inner,
                    BorderPlacement::Center,
                    BorderPlacement::Outer,
                ][state.placement],
            )
            .unwrap();
        }
        ui.draw_text_at(
            [badge.rect[0] + 16.0, badge.rect[1] + 12.0],
            [
                "Linear paint",
                "Elliptical radial paint",
                "Clockwise conic paint",
            ][state.paint_kind],
            style.text,
        );
        ui.draw_text_at(
            [badge.rect[0] + 16.0, badge.rect[1] + 36.0],
            "Pixel shading / shared border paint",
            style.text,
        );
    });
    ui.label("Masks multiply alpha on images, text and ordinary controls; input remains live.");
    ui.preview_canvas("Mask preview##demo", [width, 158.0], |ui, rect| {
        let alpha = if state.alpha_mask {
            Some(
                ui.begin_alpha_mask(AlphaMask {
                    texture: mask_texture,
                    rect,
                    uv: [0.0, 0.0, 1.0, 1.0],
                })
                .unwrap(),
            )
        } else {
            None
        };
        let fade = if state.fade_mask {
            Some(
                ui.begin_gradient_mask(GradientMask {
                    start: [rect[0], rect[1]],
                    end: [rect[2], rect[1]],
                    opacity: [1.0, 0.05],
                })
                .unwrap(),
            )
        } else {
            None
        };
        ui.draw_image(
            texture,
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            [0.0; 2],
            [1.0; 2],
            [255, 255, 255, 130],
        );
        ui.allocate([1.0, 22.0]);
        ui.label("The same masks apply to all this content");
        ui.pill_toggle("Masked control", &mut state.pill);
        if ui.button("Masked button / live hit test") {
            state.mask_clicks += 1;
            eprintln!("[demo] masked button clicks: {}", state.mask_clicks);
        }
        if let Some(scope) = fade {
            ui.end_mask(scope);
        }
        if let Some(scope) = alpha {
            ui.end_mask(scope);
        }
    });
    ui.label(&format!(
        "{} masked button clicks. This row is outside both masks.",
        state.mask_clicks
    ));
    ui.label("Paint uses up to 16 stops; alpha scopes allow four nested masks.");
    sdf_showcase(
        ui,
        state,
        texture,
        density,
        style,
        elapsed,
        motion_active,
        &stops,
        paint,
    );
}

#[allow(clippy::too_many_arguments)]
fn sdf_showcase(
    ui: &mut Ui<'_>,
    state: &mut State,
    texture: TextureId,
    density: f32,
    style: tacita::style::Style,
    elapsed: f32,
    motion_active: bool,
    stops: &[tacita::effects::GradientStop],
    paint: tacita::effects::GradientPaint,
) {
    use tacita::effects::{RevealDirection, RevealMask, SdfKind, SdfStyle};
    if motion_active && state.rotate_light {
        state.light_angle = (state.light_angle + elapsed * 0.45).rem_euclid(std::f32::consts::TAU);
    }
    if motion_active && state.animate_reveal {
        state.reveal_clock = (state.reveal_clock + elapsed * 0.24).rem_euclid(2.0);
        let triangle = if state.reveal_clock <= 1.0 {
            state.reveal_clock
        } else {
            2.0 - state.reveal_clock
        };
        state.reveal_progress = triangle * triangle * (3.0 - 2.0 * triangle);
    }
    ui.separator();
    ui.label("SDF shapes and reveals");
    ui.collapsing_header("SDF controls", |ui| {
        ui.label("Shape controls above set corners, border placement, width and offset.");
        ui.checkbox("SDF halo", &mut state.sdf_glow);
        ui.same_line();
        ui.checkbox("Edge glow", &mut state.sdf_edge);
        ui.checkbox("Directional edge highlight", &mut state.sdf_highlight);
        ui.filled_slider_float("Shader falloff reach", &mut state.sdf_reach, 0.0..=16.0);
        ui.checkbox("Rotate light", &mut state.rotate_light);
        ui.filled_slider_float(
            "Light angle",
            &mut state.light_angle,
            0.0..=std::f32::consts::TAU,
        );
        ui.filled_slider_float("Light concentration", &mut state.light_power, 0.25..=12.0);
        ui.combo(
            "Reveal direction",
            &[
                "Left to right",
                "Right to left",
                "Top to bottom",
                "Bottom to top",
                "Radial",
            ],
            &mut state.reveal_direction,
        );
        ui.checkbox("Animate reveal", &mut state.animate_reveal);
        ui.filled_slider_float("Reveal progress", &mut state.reveal_progress, 0.0..=1.0);
        ui.filled_slider_float("Reveal softness", &mut state.reveal_softness, 0.0..=0.6);
        ui.checkbox("Invert reveal", &mut state.reveal_inverted);
    });
    let width = ui.available_size()[0].clamp(160.0, 520.0);
    let placement = [
        BorderPlacement::Inner,
        BorderPlacement::Center,
        BorderPlacement::Outer,
    ][state.placement];
    let sdf = SdfStyle {
        width: state.width / density,
        placement,
        reach: state.sdf_reach,
        light_direction: [state.light_angle.cos(), state.light_angle.sin()],
        light_power: state.light_power,
        ..Default::default()
    };
    ui.preview_canvas("CPU / SDF comparison##demo", [width, 158.0], |ui, rect| {
        let half = (rect[2] - rect[0]) * 0.5;
        for i in 0..2 {
            let x = rect[0] + i as f32 * half + 14.0 + state.phase / density;
            let y = rect[1] + 34.0 + state.phase / density;
            let sample = RoundedRect::new([x, y, x + half - 28.0, y + 92.0], state.radii).unwrap();
            if i == 0 {
                ui.draw_rounded_rect(sample, style.widget_bg).unwrap();
                ui.draw_rounded_border(sample, style.text, state.width / density, placement)
                    .unwrap();
            } else {
                ui.draw_sdf_shape(sample, style.widget_bg, sdf).unwrap();
                ui.draw_sdf_shape(
                    sample,
                    style.text,
                    SdfStyle {
                        kind: SdfKind::Border,
                        ..sdf
                    },
                )
                .unwrap();
            }
            ui.draw_text_at(
                [x, rect[1] + 10.0],
                if i == 0 { "CPU contour" } else { "SDF quad" },
                style.text,
            );
        }
    });
    ui.label("Match the 1px borders at fractional offsets. SDF uses four vertices per draw.");
    ui.preview_canvas("Shader surface##demo", [width, 260.0], |ui, rect| {
        let margin = 70.0_f32.min((rect[2] - rect[0] - 60.0) * 0.5);
        let shape = RoundedRect::new(
            [
                rect[0] + margin,
                rect[1] + 70.0,
                rect[2] - margin,
                rect[3] - 70.0,
            ],
            state.radii,
        )
        .unwrap();
        if state.sdf_glow {
            ui.draw_sdf_shape(
                shape,
                [99, 91, 255, 110],
                SdfStyle {
                    kind: SdfKind::Glow,
                    ..sdf
                },
            )
            .unwrap();
        }
        ui.draw_sdf_shape_gradient(shape, stops, paint, sdf)
            .unwrap();
        ui.draw_sdf_shape(
            shape,
            style.widget_bg_hovered,
            SdfStyle {
                kind: SdfKind::Border,
                ..sdf
            },
        )
        .unwrap();
        if state.sdf_edge {
            ui.draw_sdf_shape(
                shape,
                [0, 229, 255, 160],
                SdfStyle {
                    kind: SdfKind::EdgeGlow,
                    ..sdf
                },
            )
            .unwrap();
        }
        if state.sdf_highlight {
            ui.draw_sdf_shape(
                shape,
                [226, 229, 237, 240],
                SdfStyle {
                    kind: SdfKind::Highlight,
                    width: 3.0 / density,
                    ..sdf
                },
            )
            .unwrap();
        }
        ui.draw_text_at(
            [shape.rect[0] + 16.0, shape.rect[1] + 28.0],
            "Pixel-shaded surface",
            [255; 4],
        );
        ui.draw_text_at(
            [shape.rect[0] + 16.0, shape.rect[1] + 52.0],
            "Halo / edge glow / moving light",
            [255; 4],
        );
    });
    ui.preview_canvas("Reveal preview##demo", [width, 152.0], |ui, rect| {
        ui.reveal_mask(
            RevealMask {
                rect,
                direction: [
                    RevealDirection::LeftToRight,
                    RevealDirection::RightToLeft,
                    RevealDirection::TopToBottom,
                    RevealDirection::BottomToTop,
                    RevealDirection::Radial,
                ][state.reveal_direction],
                progress: state.reveal_progress,
                softness: state.reveal_softness,
                inverted: state.reveal_inverted,
            },
            |ui| {
                ui.draw_image(
                    texture,
                    [rect[0], rect[1]],
                    [rect[2], rect[3]],
                    [0.0; 2],
                    [1.0; 2],
                    [255, 255, 255, 90],
                );
                ui.allocate([1.0, 16.0]);
                ui.label("A shader wipe over ordinary content");
                ui.pill_toggle("Revealed control", &mut state.pill);
                if ui.button("Reveal button / live input") {
                    state.reveal_clicks += 1;
                    eprintln!("[demo] reveal button clicks: {}", state.reveal_clicks);
                }
            },
        )
        .unwrap();
    });
    ui.label(&format!(
        "{} reveal clicks. This row is outside the mask.",
        state.reveal_clicks
    ));
    ui.label("Disable Animate reveal to check exact 0/1 endpoints; masks only affect visuals.");
}
