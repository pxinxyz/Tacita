use restricted_gui::context::Context as GuiContext;
use restricted_gui::draw::TextureId;
use restricted_gui::input::InputState;
use restricted_gui::ui::Ui;
use restricted_gui::ScrollFlags;
use restricted_gui_d3d12::{BackendError, D3d12Backend, OffscreenTarget, Queue};

const DEJAVU_SANS: &[u8] = include_bytes!("../../../core/tests/assets/DejaVuSans.ttf");

const W: u32 = 400;
const H: u32 = 500;
const TEXT_POS: [f32; 2] = [70.0, 100.0];

fn setup() -> (D3d12Backend, Queue, OffscreenTarget) {
    let (backend, _device, queue) = D3d12Backend::create_device().expect("device creation failed");
    let target =
        OffscreenTarget::new(backend.device(), W, H).expect("offscreen target creation failed");
    (backend, queue, target)
}

fn px(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * W as usize + x) * 4;
    pixels[i..i + 4].try_into().expect("pixel slice")
}

fn render_frame(
    ctx: &mut GuiContext,
    backend: &mut D3d12Backend,
    queue: &Queue,
    target: &OffscreenTarget,
    declare: impl FnOnce(&mut Ui),
) -> Vec<u8> {
    ctx.set_input(InputState {
        viewport_size: [W as f32, H as f32],
        ..Default::default()
    });
    ctx.new_frame(1.0 / 60.0);
    ctx.ui().window("test", ScrollFlags::NONE, declare);
    let (pixels, width, height) = ctx.font().atlas_pixels();
    backend
        .upload_atlas(pixels, width, height)
        .expect("atlas upload failed");
    let draw = ctx.render();
    target
        .clear(queue, backend.sync_mut(), [0.0, 0.0, 64.0 / 255.0, 1.0])
        .expect("clear failed");
    backend
        .render(&draw, target.resource(), target.rtv(), [W as f32, H as f32])
        .expect("render failed");
    target
        .readback(queue, backend.sync_mut())
        .expect("readback failed")
}

fn coverage_stats(
    frame: &[u8],
    blank: &[u8],
    x_range: (usize, usize),
    y_range: (usize, usize),
) -> (usize, usize) {
    let mut covered = 0;
    let mut partial = 0;
    for y in y_range.0..y_range.1 {
        for x in x_range.0..x_range.1 {
            let a = px(frame, x, y);
            if a != px(blank, x, y) {
                covered += 1;
                if a[0] > 8 && a[0] < 240 {
                    partial += 1;
                }
            }
        }
    }
    (covered, partial)
}

#[test]
fn atlas_upload_renders_real_glyph_coverage() {
    let (mut backend, queue, target) = setup();
    let mut ctx = GuiContext::new(DEJAVU_SANS).expect("test font loads");

    let text = render_frame(&mut ctx, &mut backend, &queue, &target, |ui| {
        ui.draw_text_at(TEXT_POS, "Wgy", [255, 255, 255, 255]);
    });
    let blank = render_frame(&mut ctx, &mut backend, &queue, &target, |_| {});

    let mut covered = 0;
    let mut partial = 0;
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (usize::MAX, 0, usize::MAX, 0);
    for i in 0..(W * H) as usize {
        let a = &text[i * 4..i * 4 + 4];
        if a != &blank[i * 4..i * 4 + 4] {
            covered += 1;
            if a[0] > 8 && a[0] < 240 {
                partial += 1;
            }
            min_x = min_x.min(i % W as usize);
            max_x = max_x.max(i % W as usize);
            min_y = min_y.min(i / W as usize);
            max_y = max_y.max(i / W as usize);
        }
    }
    assert!(
        covered > 40,
        "a three-letter word must light real pixels, got {covered}"
    );
    assert!(
        partial > 10,
        "atlas coverage must produce anti-aliased partial pixels, got {partial}"
    );
    assert!(
        min_x >= 65 && max_x <= 140 && min_y >= 95 && max_y <= 130,
        "diffs must be localized to the text band, got x {min_x}..{max_x}, y {min_y}..{max_y}"
    );
}

#[test]
fn atlas_reupload_after_growth_keeps_rendering() {
    let (mut backend, queue, target) = setup();
    let mut ctx = GuiContext::new(DEJAVU_SANS).expect("test font loads");

    let generation0 = ctx.font().atlas_generation();
    let (initial_w, initial_h) = {
        let (pixels, width, height) = ctx.font().atlas_pixels();
        backend
            .upload_atlas(pixels, width, height)
            .expect("initial atlas upload");
        (width, height)
    };

    let many: String = ('\u{100}'..='\u{2FF}').collect();
    let declare = |ui: &mut Ui| {
        ui.measure_text(&many);
        ui.draw_text_at(TEXT_POS, "ĀĂĄĆĈĊČĎĐ", [255, 255, 255, 255]);
    };
    let growth_frame = render_frame(&mut ctx, &mut backend, &queue, &target, declare);

    let (grown_w, grown_h) = {
        let (_, width, height) = ctx.font().atlas_pixels();
        (width, height)
    };
    assert!(
        grown_w > initial_w || grown_h > initial_h,
        "caching 512 extra glyphs must grow the atlas past {initial_w}x{initial_h}, \
         still {grown_w}x{grown_h}"
    );
    assert!(
        ctx.font().atlas_generation() > generation0,
        "glyph caching must bump the atlas generation"
    );

    let settled_a = render_frame(&mut ctx, &mut backend, &queue, &target, declare);
    let settled_b = render_frame(&mut ctx, &mut backend, &queue, &target, declare);
    assert_eq!(
        settled_a, settled_b,
        "settled frames after the grown re-upload must render identically"
    );

    let blank = render_frame(&mut ctx, &mut backend, &queue, &target, |_| {});
    let (covered, partial) = coverage_stats(&settled_a, &blank, (65, 200), (95, 130));
    assert!(
        covered > 40,
        "non-ASCII glyphs must render after growth, got {covered}"
    );
    assert!(
        partial > 10,
        "glyph edges must be coverage-modulated, got {partial}"
    );
    let (growth_covered, _) = coverage_stats(&growth_frame, &blank, (65, 200), (95, 130));
    assert!(
        growth_covered > 40,
        "the growth frame's own text must already sample the recreated texture, got {growth_covered}"
    );
}

#[test]
fn host_texture_and_atlas_coexist_in_one_frame() {
    let (mut backend, queue, target) = setup();
    let mut ctx = GuiContext::new(DEJAVU_SANS).expect("test font loads");

    let mut rgba = [0u8; 4 * 4 * 4];
    for y in 0..4usize {
        for x in 0..4usize {
            let c = match (x >= 2, y >= 2) {
                (false, false) => [255, 0, 0, 255],
                (true, false) => [0, 255, 0, 255],
                (false, true) => [0, 0, 255, 255],
                (true, true) => [255, 255, 255, 255],
            };
            rgba[(y * 4 + x) * 4..(y * 4 + x) * 4 + 4].copy_from_slice(&c);
        }
    }
    backend
        .add_host_texture(TextureId(2), &rgba, 4, 4)
        .expect("host texture registration failed");

    let frame = render_frame(&mut ctx, &mut backend, &queue, &target, |ui| {
        ui.draw_text_at(TEXT_POS, "Hi", [255, 255, 255, 255]);
        ui.draw_image(
            TextureId(2),
            [80.0, 130.0],
            [112.0, 162.0],
            [0.0, 0.0],
            [1.0, 1.0],
            [255, 255, 255, 255],
        );
        ui.draw_image(
            TextureId(9),
            [160.0, 130.0],
            [192.0, 162.0],
            [0.0, 0.0],
            [1.0, 1.0],
            [255, 255, 255, 255],
        );
    });
    let blank = render_frame(&mut ctx, &mut backend, &queue, &target, |_| {});

    assert_eq!(px(&frame, 88, 138), [255, 0, 0, 255], "pattern top-left");
    assert_eq!(px(&frame, 104, 138), [0, 255, 0, 255], "pattern top-right");
    assert_eq!(px(&frame, 88, 154), [0, 0, 255, 255], "pattern bottom-left");
    assert_eq!(
        px(&frame, 104, 154),
        [255, 255, 255, 255],
        "pattern bottom-right"
    );

    for (x, y) in [(165, 135), (176, 146), (188, 158)] {
        assert_eq!(
            px(&frame, x, y),
            px(&blank, x, y),
            "unregistered id must skip at ({x}, {y})"
        );
    }

    let (_covered, partial) = coverage_stats(&frame, &blank, (65, 120), (95, 130));
    assert!(
        partial > 5,
        "atlas text must render alongside the host texture"
    );
}

#[test]
fn reregistering_an_id_replaces_the_texture() {
    let (mut backend, queue, target) = setup();
    let mut ctx = GuiContext::new(DEJAVU_SANS).expect("test font loads");

    let red = [255u8, 0, 0, 255].repeat(4);
    let green = [0u8, 255, 0, 255].repeat(4);
    let draw = |ui: &mut Ui| {
        ui.draw_image(
            TextureId(2),
            [80.0, 130.0],
            [96.0, 146.0],
            [0.0, 0.0],
            [1.0, 1.0],
            [255, 255, 255, 255],
        );
    };

    backend
        .add_host_texture(TextureId(2), &red, 2, 2)
        .expect("first registration");
    let first = render_frame(&mut ctx, &mut backend, &queue, &target, draw);
    assert_eq!(
        px(&first, 88, 138),
        [255, 0, 0, 255],
        "first registration draws"
    );

    backend
        .add_host_texture(TextureId(2), &green, 2, 2)
        .expect("re-registration");
    let second = render_frame(&mut ctx, &mut backend, &queue, &target, draw);
    assert_eq!(
        px(&second, 88, 138),
        [0, 255, 0, 255],
        "re-registration replaces"
    );
}

#[test]
fn descriptor_heap_exhaustion_rejects_ids_past_capacity() {
    let (mut backend, queue, target) = setup();
    let texel = [255u8, 0, 0, 255];

    for id in 2..=64u64 {
        backend
            .add_host_texture(TextureId(id), &texel, 1, 1)
            .unwrap_or_else(|e| {
                panic!(
                    "registration of id {id} (slot {}) must succeed: {e}",
                    id - 1
                )
            });
    }

    let err = backend
        .add_host_texture(TextureId(65), &texel, 1, 1)
        .expect_err("a 64th new id must exhaust the 64-slot heap");
    assert!(
        matches!(err, BackendError::DescriptorHeapExhausted(64)),
        "got {err:?}"
    );

    let err = backend
        .add_host_texture(TextureId(2), &texel, 1, 1)
        .expect_err("re-registration also needs a slot under a full heap");
    assert!(
        matches!(err, BackendError::DescriptorHeapExhausted(64)),
        "got {err:?}"
    );

    target
        .readback(&queue, backend.sync_mut())
        .expect("final drain");
}

#[test]
fn reserved_ids_and_bad_sizes_are_rejected() {
    let (mut backend, _queue, _target) = setup();
    let rgba = [255u8; 4 * 4 * 4];

    let err = backend
        .add_host_texture(TextureId::FONT, &rgba, 4, 4)
        .expect_err("FONT is reserved for the atlas");
    assert!(
        matches!(err, BackendError::ReservedTextureId(0)),
        "got {err:?}"
    );

    let err = backend
        .add_host_texture(TextureId::WHITE, &rgba, 4, 4)
        .expect_err("WHITE is reserved for the atlas");
    assert!(
        matches!(err, BackendError::ReservedTextureId(1)),
        "got {err:?}"
    );

    let err = backend
        .add_host_texture(TextureId(2), &rgba[..8], 4, 4)
        .expect_err("short RGBA data must be rejected");
    assert!(
        matches!(err, BackendError::InvalidTextureData(_)),
        "got {err:?}"
    );

    let err = backend
        .add_host_texture(TextureId(2), &[], 0, 4)
        .expect_err("a zero dimension must be rejected");
    assert!(
        matches!(err, BackendError::InvalidTextureData(_)),
        "got {err:?}"
    );

    let err = backend
        .upload_atlas(&[0u8; 15], 4, 4)
        .expect_err("atlas data must be exactly width * height");
    assert!(
        matches!(err, BackendError::InvalidTextureData(_)),
        "got {err:?}"
    );

    let err = backend
        .upload_atlas(&[], 4, 0)
        .expect_err("a zero dimension must be rejected");
    assert!(
        matches!(err, BackendError::InvalidTextureData(_)),
        "got {err:?}"
    );
}
