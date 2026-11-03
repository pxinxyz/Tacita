use restricted_gui::context::Context as GuiContext;
use restricted_gui::draw::{DrawCmd, DrawData, DrawVert, TextureId};
use restricted_gui::input::InputState;
use restricted_gui::ui::Ui;
use restricted_gui::ScrollFlags;
use restricted_gui_d3d12::{D3d12Backend, OffscreenTarget, Queue};

const DEJAVU_SANS: &[u8] = include_bytes!("../../../core/tests/assets/DejaVuSans.ttf");

const W: u32 = 400;
const H: u32 = 500;
const TEXT_POS: [f32; 2] = [70.0, 100.0];
const BAND: ((usize, usize), (usize, usize)) = ((65, 200), (95, 130));
const QUAD_MIN: [f32; 2] = [80.0, 130.0];
const QUAD_MAX: [f32; 2] = [112.0, 162.0];
const HAMMER_FRAMES: usize = 30;

const RETIREMENT_BOUND: usize = 16;

const REG_COLORS: [[u8; 4]; 5] = [
    [255, 0, 0, 255],
    [0, 255, 0, 255],
    [0, 0, 255, 255],
    [255, 255, 0, 255],
    [0, 255, 255, 255],
];

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

fn band_pixels(pixels: &[u8]) -> Vec<[u8; 4]> {
    let ((x0, x1), (y0, y1)) = BAND;
    let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0));
    for y in y0..y1 {
        for x in x0..x1 {
            out.push(px(pixels, x, y));
        }
    }
    out
}

fn frame(
    ctx: &mut GuiContext,
    backend: &mut D3d12Backend,
    queue: &Queue,
    target: &OffscreenTarget,
    declare: impl FnOnce(&mut Ui),
    readback: bool,
) -> Option<Vec<u8>> {
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
    if readback {
        Some(
            target
                .readback(queue, backend.sync_mut())
                .expect("readback failed"),
        )
    } else {
        None
    }
}

#[test]
fn hammer_alternating_uploads_and_reregistrations_stay_coherent() {
    let (mut backend, queue, target) = setup();
    let mut ctx = GuiContext::new(DEJAVU_SANS).expect("test font loads");
    let idle_baseline = backend.full_idle_wait_count();

    let blank = frame(&mut ctx, &mut backend, &queue, &target, |_| {}, true).expect("readback");
    let (initial_w, initial_h) = {
        let (_, w, h) = ctx.font().atlas_pixels();
        (w, h)
    };

    let mut current_color = [128u8, 128, 128, 255];
    backend
        .add_host_texture(TextureId(2), &current_color.repeat(4), 2, 2)
        .expect("initial registration");

    let mut reference_band: Option<Vec<[u8; 4]>> = None;
    let mut registrations = 0usize;
    for f in 0..HAMMER_FRAMES {
        let want_readback = f % 5 == 4 || f == HAMMER_FRAMES - 1;
        if f % 2 == 1 {
            current_color = REG_COLORS[registrations % REG_COLORS.len()];
            registrations += 1;
            backend
                .add_host_texture(TextureId(2), &current_color.repeat(4), 2, 2)
                .expect("re-registration failed");
        }
        let shot = frame(
            &mut ctx,
            &mut backend,
            &queue,
            &target,
            |ui| {
                if f % 2 == 0 {
                    let start = 0x100u32 + (f as u32 / 2) * 96;
                    let batch: String = (start..start + 96).filter_map(char::from_u32).collect();
                    ui.measure_text(&batch);
                }
                ui.draw_text_at(TEXT_POS, "Hammer", [255, 255, 255, 255]);
                ui.draw_image(
                    TextureId(2),
                    QUAD_MIN,
                    QUAD_MAX,
                    [0.0, 0.0],
                    [1.0, 1.0],
                    [255; 4],
                );
            },
            want_readback,
        );

        assert!(
            backend.retirement_queue_len() <= RETIREMENT_BOUND,
            "frame {f}: retirement queue grew past {} — the sweep is not keeping up",
            RETIREMENT_BOUND
        );
        assert_eq!(
            backend.full_idle_wait_count(),
            idle_baseline,
            "frame {f}: steady-state work must never block on full GPU idle"
        );

        let Some(shot) = shot else { continue };
        for (x, y) in [(88, 138), (104, 138), (88, 154), (104, 154), (96, 146)] {
            assert_eq!(
                px(&shot, x, y),
                current_color,
                "frame {f}: quad pixel ({x}, {y}) must be the current registration's color"
            );
        }
        let band = band_pixels(&shot);
        match &reference_band {
            Some(reference) => assert_eq!(
                &band, reference,
                "frame {f}: the text band changed across the hammer — tearing or atlas corruption"
            ),
            None => {
                let mut covered = 0;
                let mut partial = 0;
                for (i, b) in band.iter().enumerate() {
                    let blank_px = px(&blank, 65 + i % 135, 95 + i / 135);
                    if *b != blank_px {
                        covered += 1;
                        if b[0] > 8 && b[0] < 240 {
                            partial += 1;
                        }
                    }
                }
                assert!(
                    covered > 40,
                    "the text must light real pixels, got {covered}"
                );
                assert!(
                    partial > 10,
                    "glyph edges must be coverage-modulated, got {partial}"
                );
                reference_band = Some(band);
            }
        }
    }

    let (grown_w, grown_h) = {
        let (_, w, h) = ctx.font().atlas_pixels();
        (w, h)
    };
    assert!(
        grown_w > initial_w || grown_h > initial_h,
        "15 glyph batches must have forced atlas growth past {initial_w}x{initial_h}, \
         still {grown_w}x{grown_h}"
    );
}

#[test]
fn uploads_and_renders_never_wait_for_full_gpu_idle() {
    let (mut backend, queue, target) = setup();
    let baseline = backend.full_idle_wait_count();

    let vertices = [
        DrawVert {
            pos: [0.0, 0.0],
            uv: [0.0, 0.0],
            col: [255, 255, 255, 255],
        },
        DrawVert {
            pos: [8.0, 0.0],
            uv: [1.0, 0.0],
            col: [255, 255, 255, 255],
        },
        DrawVert {
            pos: [8.0, 8.0],
            uv: [1.0, 1.0],
            col: [255, 255, 255, 255],
        },
        DrawVert {
            pos: [0.0, 8.0],
            uv: [0.0, 1.0],
            col: [255, 255, 255, 255],
        },
    ];
    let indices = [0u32, 1, 2, 0, 2, 3];
    let commands = [DrawCmd {
        effects: 0,
        elem_count: 6,
        clip_rect: [0.0, 0.0, W as f32, H as f32],
        texture: TextureId(2),
    }];
    let draw = DrawData {
        effects: &[],
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };

    for k in 0..12u32 {
        let size = if k % 2 == 0 { 8 } else { 16 };
        let pixels = vec![200u8; (size * size) as usize];
        backend
            .upload_atlas(&pixels, size, size)
            .expect("atlas upload failed");
        let color = [k as u8, 255 - k as u8, 64, 255];
        backend
            .add_host_texture(TextureId(2), &color.repeat(4), 2, 2)
            .expect("host registration failed");
        backend
            .render(&draw, target.resource(), target.rtv(), [W as f32, H as f32])
            .expect("render failed");
        assert_eq!(
            backend.full_idle_wait_count(),
            baseline,
            "iteration {k}: an upload or render blocked on full GPU idle"
        );
    }
    let _ = target
        .readback(&queue, backend.sync_mut())
        .expect("final readback failed");
    assert_eq!(
        backend.full_idle_wait_count(),
        baseline,
        "the harness drain must not count as a pipeline full-idle wait"
    );
}

#[test]
fn retirement_queue_drains_once_fences_pass() {
    let (mut backend, queue, target) = setup();

    let vertices = [
        DrawVert {
            pos: [0.0, 0.0],
            uv: [0.0, 0.0],
            col: [255, 255, 255, 255],
        },
        DrawVert {
            pos: [8.0, 0.0],
            uv: [1.0, 0.0],
            col: [255, 255, 255, 255],
        },
        DrawVert {
            pos: [8.0, 8.0],
            uv: [1.0, 1.0],
            col: [255, 255, 255, 255],
        },
        DrawVert {
            pos: [0.0, 8.0],
            uv: [0.0, 1.0],
            col: [255, 255, 255, 255],
        },
    ];
    let indices = [0u32, 1, 2, 0, 2, 3];
    let commands = [DrawCmd {
        effects: 0,
        elem_count: 6,
        clip_rect: [0.0, 0.0, W as f32, H as f32],
        texture: TextureId::WHITE,
    }];
    let draw = DrawData {
        effects: &[],
        vertices: &vertices,
        indices: &indices,
        commands: &commands,
    };

    for k in 0..4u32 {
        let size = if k % 2 == 0 { 8 } else { 16 };
        let pixels = vec![200u8; (size * size) as usize];
        backend
            .upload_atlas(&pixels, size, size)
            .expect("atlas upload failed");
        let color = [255, k as u8, 0, 255];
        backend
            .add_host_texture(TextureId(2), &color.repeat(4), 2, 2)
            .expect("host registration failed");
        backend
            .render(&draw, target.resource(), target.rtv(), [W as f32, H as f32])
            .expect("render failed");
        assert!(
            backend.retirement_queue_len() <= RETIREMENT_BOUND,
            "iteration {k}: retirement queue must stay bounded"
        );
    }

    let _ = target
        .readback(&queue, backend.sync_mut())
        .expect("readback failed");
    backend
        .render(&draw, target.resource(), target.rtv(), [W as f32, H as f32])
        .expect("sweep render failed");
    assert_eq!(
        backend.retirement_queue_len(),
        0,
        "every retired resource's fence has passed — the queue must drain to 0"
    );
    let _ = target
        .readback(&queue, backend.sync_mut())
        .expect("final readback failed");
}
