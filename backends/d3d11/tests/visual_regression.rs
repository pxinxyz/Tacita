use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use tacita::context::{Context as GuiContext, DEFAULT_FONT_SIZE};
use tacita::draw::{DrawCmd, DrawData, DrawVert};
use tacita::font::Font;
use tacita::input::InputState;
use tacita::ScrollFlags;
use tacita_d3d11::{D3d11Backend, OffscreenTarget};

const FONT_BYTES: &[u8] = include_bytes!("../../../core/tests/assets/DejaVuSans.ttf");
const LOGICAL_SIZE: [u32; 2] = [720, 480];
const BACKDROP: [u8; 4] = [26, 27, 38, 255];

fn baseline_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("visual_baselines")
        .join(format!("{name}.png"))
}

fn write_png(path: &Path, width: u32, height: u32, pixels: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels)
        .unwrap();
}

fn read_png(path: &Path) -> (u32, u32, Vec<u8>) {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    pixels.truncate(info.buffer_size());
    (info.width, info.height, pixels)
}

fn declare_scene(gui: &mut GuiContext, selected: &mut usize) {
    let mut ui = gui.ui();
    ui.window("Visual regression", ScrollFlags::NONE, |ui| {
        ui.button("Rounded corners");
        ui.draw_rect_outline([83.0, 151.0], [154.0, 209.0], [122, 162, 247, 255], 3.0);
        ui.draw_rect_filled([174.0, 154.0], [213.0, 205.0], [158, 206, 106, 255]);
        ui.draw_arc([273.0, 180.0], 31.0, 0.35..=2.65, [187, 154, 247, 255], 7.0);
        ui.draw_circle_filled([124.0, 278.0], 28.0, [125, 207, 255, 255]);
        ui.draw_circle_outline([273.0, 278.0], 28.0, [247, 118, 142, 255], 5.0);
        ui.draw_text_at([84.0, 337.0], "Aa 012345  edges", [224, 228, 244, 255]);
    });
    ui.window("Widget regressions", ScrollFlags::NONE, |ui| {
        ui.label("Radio / progress / tree");
        ui.radio_button("Quiet", selected, 0);
        ui.same_line();
        ui.radio_button("Bright", selected, 1);
        ui.progress_bar("Transfer 63%", 0.63, [240.0, 20.0]);
        ui.tree_node("Root", |ui| {
            ui.child("Clipped branch", [260.0, 132.0], ScrollFlags::NONE, |ui| {
                ui.label("Inside the clip");
                ui.radio_button("Nested choice", selected, 2);
                ui.progress_bar("42%", 0.42, [180.0, 16.0]);
                ui.tree_node("Subtree", |ui| ui.label("Nested tree content"));
                ui.label("Partly clipped at bottom");
            });
        });
        ui.tree_node("Closed branch", |ui| ui.label("Hidden content"));
    });
}

fn render_scene(scale: f32) -> (u32, u32, Vec<u8>) {
    let width = (LOGICAL_SIZE[0] as f32 * scale).round() as u32;
    let height = (LOGICAL_SIZE[1] as f32 * scale).round() as u32;
    let (mut backend, device, d3dctx) = D3d11Backend::create_device().unwrap();
    let target = OffscreenTarget::new(&device, width, height).unwrap();
    let font = Font::new_with_icons_at_scale(FONT_BYTES, DEFAULT_FONT_SIZE, &[], scale).unwrap();
    let mut gui = GuiContext::with_font(font);
    gui.set_window_rect("Visual regression", [60.0, 60.0], [300.0, 400.0]);
    gui.set_window_rect("Widget regressions", [390.0, 60.0], [300.0, 400.0]);
    let mut selected = 1;
    let mut closed_vertices = 0;
    let mut parent_only_vertices = 0;
    for (frame, (mouse_pos, down)) in [
        ([430.0, 175.0], true),
        ([430.0, 175.0], false),
        ([463.0, 278.0], true),
        ([463.0, 278.0], false),
        ([-100.0, -100.0], false),
    ]
    .into_iter()
    .enumerate()
    {
        gui.set_input(InputState {
            mouse_pos,
            mouse_down: [down, false, false, false, false],
            viewport_size: [LOGICAL_SIZE[0] as f32, LOGICAL_SIZE[1] as f32],
            ..Default::default()
        });
        gui.new_frame(1.0 / 60.0);
        declare_scene(&mut gui, &mut selected);
        if frame < 4 {
            let draw = gui.render();
            if frame == 0 {
                closed_vertices = draw.vertices.len();
            } else if frame == 2 {
                parent_only_vertices = draw.vertices.len();
            }
        }
    }
    let (atlas, atlas_w, atlas_h) = gui.font().atlas_pixels();
    backend.upload_atlas(atlas, atlas_w, atlas_h).unwrap();
    let draw = gui.render();
    assert!(
        draw.vertices.len() > closed_vertices,
        "the click sequence must open the root tree and declare its clipped content"
    );
    assert!(
        draw.vertices.len() > parent_only_vertices,
        "the click sequence must open the nested tree"
    );

    let vertices: Vec<DrawVert> = draw
        .vertices
        .iter()
        .map(|vertex| DrawVert {
            pos: [vertex.pos[0] * scale, vertex.pos[1] * scale],
            ..*vertex
        })
        .collect();
    let commands: Vec<DrawCmd> = draw
        .commands
        .iter()
        .map(|command| DrawCmd {
            clip_rect: command.clip_rect.map(|value| value * scale),
            ..command.clone()
        })
        .collect();
    let effects: Vec<_> = draw
        .effects
        .iter()
        .copied()
        .map(|mut effect| {
            effect.gradient.rect = effect.gradient.rect.map(|value| value * scale);
            effect.sdf.rect = effect.sdf.rect.map(|value| value * scale);
            effect.sdf.radii = effect.sdf.radii.map(|value| value * scale);
            effect.sdf.parameters[1] *= scale;
            effect.sdf.parameters[3] *= scale;
            for mask in &mut effect.masks.ops[..effect.masks.count as usize] {
                mask.rect = mask.rect.map(|value| value * scale);
            }
            effect
        })
        .collect();
    let physical = DrawData {
        effects: &effects,
        vertices: &vertices,
        indices: draw.indices,
        commands: &commands,
    };
    target.clear(
        &d3dctx,
        [
            BACKDROP[0] as f32 / 255.0,
            BACKDROP[1] as f32 / 255.0,
            BACKDROP[2] as f32 / 255.0,
            1.0,
        ],
    );
    backend
        .render(&physical, target.rtv(), [width as f32, height as f32])
        .unwrap();
    (width, height, target.readback(&d3dctx).unwrap())
}

#[test]
fn primitives_widgets_and_dpi_match_captures() {
    for (name, scale) in [
        ("dpi100", 1.0),
        ("dpi125", 1.25),
        ("dpi150", 1.5),
        ("dpi200", 2.0),
    ] {
        let (width, height, actual) = render_scene(scale);
        let baseline = baseline_path(name);
        if std::env::var("TACITA_UPDATE_VISUALS").as_deref() == Ok("1") {
            write_png(&baseline, width, height, &actual);
            eprintln!("updated {}", baseline.display());
            continue;
        }
        let (expected_width, expected_height, expected) = read_png(&baseline);
        assert_eq!(
            (width, height),
            (expected_width, expected_height),
            "{name} size"
        );
        let changed = actual
            .chunks_exact(4)
            .zip(expected.chunks_exact(4))
            .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 2))
            .count();
        let limit = (actual.len() / 4 / 2000).max(8);
        if changed > limit {
            let output = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/visual-regressions")
                .join(format!("{name}-actual.png"));
            write_png(&output, width, height, &actual);
            panic!(
                "{name}: {changed} pixels differ (limit {limit}); actual capture: {}",
                output.display()
            );
        }
    }
}
