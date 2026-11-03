use super::*;
use crate::{context::Context, input::InputState, ScrollFlags};

#[test]
fn atlas_growth_preserves_ids_pixels_and_extruded_edges() {
    let mut atlas = SpriteAtlas::new(TextureId(9), 8, 64, 1).unwrap();
    let first = atlas.insert(&[220, 20, 30, 128].repeat(9), [3, 3]).unwrap();
    let before = atlas.region(first).unwrap();
    for _ in 0..12 {
        atlas
            .insert(&[10, 30, 240, 255].repeat(16), [4, 4])
            .unwrap();
    }
    let region = atlas.region(first).unwrap();
    assert_eq!(region.size, before.size);
    assert!(atlas.size()[0] > 8 || atlas.size()[1] > 8);
    let pixels = atlas.pixels();
    let x = (region.uv_min[0] * atlas.size()[0] as f32) as usize;
    let y = (region.uv_min[1] * atlas.size()[1] as f32) as usize;
    let offset = ((y - 1) * atlas.size()[0] as usize + x - 1) * 4;
    assert_eq!(&pixels[offset..offset + 4], &[220, 20, 30, 128]);
    let generation = atlas.generation();
    assert!(atlas.insert(&[0; 4], [4, 4]).is_err());
    assert_eq!(atlas.generation(), generation);
    assert_eq!(atlas.len(), 13);
}

#[test]
fn fit_preserves_aspect_and_crops_source_for_cover() {
    let region = ImageRegion {
        texture: TextureId(9),
        uv_min: [0.2, 0.3],
        uv_max: [0.8, 0.9],
        size: [200., 100.],
    };
    let contain = fitted(region, [0., 0., 100., 100.], ImageFit::Contain);
    assert_eq!(contain.0, [0., 25., 100., 75.]);
    let cover = fitted(region, [0., 0., 100., 100.], ImageFit::Cover);
    assert!((cover.1.uv_min[0] - 0.35).abs() < 0.0001);
    assert!((cover.1.uv_max[0] - 0.65).abs() < 0.0001);
}

#[test]
fn narrow_nine_slice_stays_inside_destination_and_source() {
    let mut ctx = Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap();
    ctx.set_input(InputState {
        mouse_pos: [-100., -100.],
        ..Default::default()
    });
    ctx.new_frame(0.016);
    let region = ImageRegion {
        texture: TextureId(9),
        uv_min: [0.1, 0.2],
        uv_max: [0.9, 0.8],
        size: [32., 32.],
    };
    ctx.ui().window("images", ScrollFlags::NONE, |ui| {
        ui.draw_nine_slice(
            region,
            [100., 100., 106., 105.],
            NineSlice {
                source: [8.; 4],
                destination: [10.; 4],
            },
            [255; 4],
        )
        .unwrap();
    });
    let draw = ctx.render();
    let vertices: alloc::vec::Vec<_> = draw
        .vertices
        .iter()
        .filter(|v| v.pos[0] >= 100. && v.pos[0] <= 106. && v.pos[1] >= 100. && v.pos[1] <= 105.)
        .collect();
    assert!(!vertices.is_empty());
    for vertex in vertices {
        assert!(vertex.uv[0] >= 0.1 && vertex.uv[0] <= 0.9);
        assert!(vertex.uv[1] >= 0.2 && vertex.uv[1] <= 0.8);
    }
}

#[test]
fn image_button_captures_releases_and_respects_disabled_scopes() {
    let mut ctx = Context::new(include_bytes!("../../tests/assets/DejaVuSans.ttf")).unwrap();
    let region = ImageRegion {
        texture: TextureId(9),
        uv_min: [0.; 2],
        uv_max: [1.; 2],
        size: [80., 40.],
    };
    let mut rect = [0.; 4];
    ctx.new_frame(0.016);
    ctx.ui().window("image", ScrollFlags::NONE, |ui| {
        rect = ui
            .image(region, [80., 40.], ImageFit::Stretch, [255; 4])
            .unwrap();
    });
    ctx.render();
    let mouse = [(rect[0] + rect[2]) * 0.5, (rect[1] + rect[3]) * 0.5];
    for (position, down, disabled, expected) in [
        (mouse, true, false, false),
        ([-100.; 2], false, false, false),
        (mouse, true, false, false),
        (mouse, false, false, true),
        (mouse, true, true, false),
        (mouse, false, true, false),
    ] {
        ctx.set_input(InputState {
            mouse_pos: position,
            mouse_down: [down, false, false, false, false],
            ..Default::default()
        });
        ctx.new_frame(0.016);
        let mut clicked = false;
        ctx.ui().window("image", ScrollFlags::NONE, |ui| {
            ui.disabled("disabled", disabled, |ui| {
                clicked = ui
                    .image_button("sprite", region, [80., 40.], ImageFit::Contain, [255; 4])
                    .unwrap();
            });
        });
        ctx.render();
        assert_eq!(clicked, expected);
    }
}
#[test]
fn failed_atlas_growth_retains_pixels_regions_and_generation() {
    let mut atlas = SpriteAtlas::new(TextureId(9), 4, 4, 1).unwrap();
    let sprite = atlas.insert(&[255; 16], [2; 2]).unwrap();
    let pixels = atlas.pixels().to_vec();
    let generation = atlas.generation();
    let region = atlas.region(sprite);
    assert_eq!(atlas.insert(&[1; 16], [2; 2]), Err(ImageError::AtlasFull));
    assert_eq!(atlas.pixels(), pixels);
    assert_eq!(atlas.generation(), generation);
    assert_eq!(atlas.region(sprite), region);
}
