use restricted_gui::context::Context as GuiContext;
use restricted_gui::input::{InputState, KeyEvent};
use restricted_gui::style::Style;
use restricted_gui::ui::Ui;
use restricted_gui::ScrollFlags;
use restricted_gui_d3d11::{Context as D3dContext, D3d11Backend, Device, OffscreenTarget};

const DEJAVU_SANS: &[u8] = include_bytes!("../../../core/tests/assets/DejaVuSans.ttf");

const W: u32 = 400;
const H: u32 = 500;
const CLEAR: [u8; 4] = [0, 0, 64, 255];
const WINDOW_POS: [f32; 2] = [60.0, 60.0];
const AWAY: [f32; 2] = [-50.0, -50.0];

fn setup() -> (D3d11Backend, Device, D3dContext, OffscreenTarget) {
    let (backend, device, context) = D3d11Backend::create_device().expect("device creation failed");
    let target = OffscreenTarget::new(&device, W, H).expect("offscreen target creation failed");
    (backend, device, context, target)
}

fn px(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * W as usize + x) * 4;
    pixels[i..i + 4].try_into().expect("pixel slice")
}

fn render_frame(
    ctx: &mut GuiContext,
    backend: &mut D3d11Backend,
    d3dctx: &D3dContext,
    target: &OffscreenTarget,
    input: InputState,
    declare: impl FnOnce(&mut Ui),
) -> Vec<u8> {
    ctx.set_input(input);
    ctx.new_frame(1.0 / 60.0);
    ctx.ui().window("e2e", ScrollFlags::NONE, declare);
    let (pixels, width, height) = ctx.font().atlas_pixels();
    backend
        .upload_atlas(pixels, width, height)
        .expect("atlas upload failed");
    backend
        .upload_color_font_atlas(ctx.font())
        .expect("color atlas upload failed");
    let draw = ctx.render();
    target.clear(d3dctx, [0.0, 0.0, 64.0 / 255.0, 1.0]);
    backend
        .render(&draw, target.rtv(), [W as f32, H as f32])
        .expect("render failed");
    target.readback(d3dctx).expect("readback failed")
}

fn input_at(mouse_pos: [f32; 2], left_down: bool) -> InputState<'static> {
    InputState {
        mouse_pos,
        mouse_down: [left_down, false, false, false, false],
        viewport_size: [W as f32, H as f32],
        ..Default::default()
    }
}

struct WidgetLayout {
    title_bar: [f32; 4],
    button: [f32; 4],
    checkbox_box: [f32; 4],
    text_field: [f32; 4],
}

fn widget_layout(style: &Style, line_height: f32, button_text_size: [f32; 2]) -> WidgetLayout {
    let title_bar_height = line_height + 2.0 * style.frame_padding[1];
    let content_x = WINDOW_POS[0] + style.window_padding[0];
    let content_y = WINDOW_POS[1] + title_bar_height + style.window_padding[1];
    let button = [
        content_x,
        content_y,
        content_x + button_text_size[0] + 2.0 * style.frame_padding[0],
        content_y + button_text_size[1] + 2.0 * style.frame_padding[1],
    ];
    let box_y = button[3] + style.item_spacing[1];
    let checkbox_box = [
        content_x,
        box_y,
        content_x + line_height,
        box_y + line_height,
    ];
    let field_y = checkbox_box[3] + style.item_spacing[1];
    let text_field = [
        content_x,
        field_y,
        content_x + 200.0,
        field_y + line_height + 2.0 * style.frame_padding[1],
    ];
    WidgetLayout {
        title_bar: [
            WINDOW_POS[0],
            WINDOW_POS[1],
            360.0,
            WINDOW_POS[1] + title_bar_height,
        ],
        button,
        checkbox_box,
        text_field,
    }
}

fn partial_between(p: [u8; 4], lo: [u8; 4], hi: [u8; 4], pure: &[[u8; 4]]) -> bool {
    if p[3] != 255 || pure.contains(&p) {
        return false;
    }
    (0..3).all(|c| {
        let (min, max) = (lo[c].min(hi[c]), lo[c].max(hi[c]));
        p[c] > min && p[c] < max
    })
}

struct BlendRange {
    lo: [u8; 4],
    hi: [u8; 4],
    pure: [[u8; 4]; 3],
}

fn count_partials(
    pixels: &[u8],
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    range: &BlendRange,
) -> usize {
    let mut count = 0;
    for y in y0..y1.min(H as usize) {
        for x in x0..x1.min(W as usize) {
            if partial_between(px(pixels, x, y), range.lo, range.hi, &range.pure) {
                count += 1;
            }
        }
    }
    count
}

fn count_exact_in(pixels: &[u8], rect: [f32; 4], inset: f32, color: [u8; 4]) -> usize {
    let x0 = (rect[0] + inset).ceil() as usize;
    let y0 = (rect[1] + inset).ceil() as usize;
    let x1 = (rect[2] - inset).floor() as usize;
    let y1 = (rect[3] - inset).floor() as usize;
    let mut count = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            if px(pixels, x, y) == color {
                count += 1;
            }
        }
    }
    count
}

fn center(rect: [f32; 4]) -> [f32; 2] {
    [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]
}

fn border_color(bg: [u8; 4]) -> [u8; 4] {
    [
        bg[0].saturating_add(40),
        bg[1].saturating_add(40),
        bg[2].saturating_add(40),
        bg[3],
    ]
}

fn declare_widgets<'a>(
    checked: &'a mut bool,
    text: &'a mut String,
    button_text_size: &'a mut [f32; 2],
) -> impl FnOnce(&mut Ui) + 'a {
    move |ui: &mut Ui| {
        *button_text_size = ui.measure_text("Click me");
        ui.button("Click me");
        ui.checkbox("Check me", checked);
        ui.text_input("Name", text);
    }
}

#[test]
fn window_widgets_render_style_colors_and_corner_masks() {
    let (mut backend, _device, d3dctx, target) = setup();
    let mut ctx = GuiContext::new(DEJAVU_SANS).expect("test font loads");
    let style = ctx.style;
    assert!(
        style.corner_rounding > 0.0,
        "this test verifies the rounded-chrome path"
    );
    assert_ne!(style.window_bg, style.title_bg);
    assert_ne!(style.window_bg, style.widget_bg);
    assert_ne!(style.title_bg, style.widget_bg);

    let mut checked = false;
    let mut text = String::new();
    let mut button_text_size = [0.0; 2];
    let pixels = render_frame(
        &mut ctx,
        &mut backend,
        &d3dctx,
        &target,
        input_at(AWAY, false),
        declare_widgets(&mut checked, &mut text, &mut button_text_size),
    );
    let layout = widget_layout(&style, ctx.font().line_height(), button_text_size);

    assert_eq!(px(&pixels, 10, 10), CLEAR, "outside the window stays clear");
    assert_eq!(px(&pixels, 390, 490), CLEAR, "below the window stays clear");
    assert_eq!(px(&pixels, 200, 400), style.window_bg, "window body fill");

    let band_y1 = (layout.title_bar[3] - 3.0).floor() as usize;
    let mut band = 0;
    let mut band_total = 0;
    for y in 64..band_y1 {
        for x in 120..320 {
            band_total += 1;
            if px(&pixels, x, y) == style.title_bg {
                band += 1;
            }
        }
    }
    assert!(band_total > 0, "the title band region must be non-empty");
    assert!(
        band * 100 >= band_total * 95,
        "title bar band must be the title_bg color: {band}/{band_total}"
    );

    let btn = layout.button;
    let cy = ((btn[1] + btn[3]) / 2.0) as usize;
    let left_interior = (btn[0] + 4.0) as usize;
    let right_interior = (btn[2] - 4.0) as usize;
    assert_eq!(
        px(&pixels, left_interior, cy),
        style.widget_bg,
        "button left interior is the exact idle fill"
    );
    assert_eq!(
        px(&pixels, right_interior, cy),
        style.widget_bg,
        "button right interior is the exact idle fill"
    );
    let area = ((btn[2] - btn[0] - 4.0) * (btn[3] - btn[1] - 4.0)) as usize;
    let fill = count_exact_in(&pixels, btn, 2.0, style.widget_bg);
    assert!(
        fill * 2 >= area,
        "most of the button interior must be the exact fill: {fill}/{area}"
    );
    let center_px = px(&pixels, ((btn[0] + btn[2]) / 2.0) as usize, cy);
    assert_eq!(center_px[3], 255, "button center is fully covered");
    assert_ne!(center_px, style.window_bg, "button center is covered");

    let fill_range = BlendRange {
        lo: style.window_bg,
        hi: border_color(style.widget_bg),
        pure: [
            style.window_bg,
            style.widget_bg,
            border_color(style.widget_bg),
        ],
    };
    let bx0 = btn[0].floor() as usize;
    let by0 = btn[1].floor() as usize;
    let bx1 = btn[2].ceil() as usize;
    let by1 = btn[3].ceil() as usize;
    for (name, x0, y0, x1, y1) in [
        ("top-left", bx0 - 2, by0 - 2, bx0 + 7, by0 + 7),
        ("top-right", bx1 - 6, by0 - 2, bx1 + 3, by0 + 7),
        ("bottom-left", bx0 - 2, by1 - 6, bx0 + 7, by1 + 3),
        ("bottom-right", bx1 - 6, by1 - 6, bx1 + 3, by1 + 3),
    ] {
        let partials = count_partials(&pixels, x0, y0, x1, y1, &fill_range);
        assert!(
            partials >= 3,
            "button {name} corner must show partial mask coverage, got {partials}"
        );
    }

    let cb = layout.checkbox_box;
    assert_eq!(
        px(
            &pixels,
            (cb[0] + 4.0) as usize,
            ((cb[1] + cb[3]) / 2.0) as usize
        ),
        style.widget_bg,
        "checkbox box interior is the idle fill"
    );
    let field = layout.text_field;
    assert_eq!(
        px(
            &pixels,
            (field[0] + 8.0) as usize,
            ((field[1] + field[3]) / 2.0) as usize
        ),
        style.widget_bg,
        "text field interior is the idle fill"
    );
}

#[test]
fn masked_circle_renders_partial_edge_coverage() {
    let (mut backend, _device, d3dctx, target) = setup();
    let mut ctx = GuiContext::new(DEJAVU_SANS).expect("test font loads");
    let style = ctx.style;
    assert!(
        style.anti_aliasing,
        "this test verifies the analytic-mask AA path"
    );

    const CENTER: [f32; 2] = [210.0, 260.0];
    const RADIUS: f32 = 40.0;
    const RED: [u8; 4] = [255, 0, 0, 255];
    let pixels = render_frame(
        &mut ctx,
        &mut backend,
        &d3dctx,
        &target,
        input_at(AWAY, false),
        |ui| ui.draw_circle_filled(CENTER, RADIUS, RED),
    );

    assert_eq!(px(&pixels, 210, 260), RED, "circle center");
    assert_eq!(px(&pixels, 200, 250), RED, "circle interior");

    let mut solid = 0;
    for y in 225..295usize {
        for x in 175..245usize {
            let dx = x as f32 + 0.5 - CENTER[0];
            let dy = y as f32 + 0.5 - CENTER[1];
            if dx * dx + dy * dy < 35.0 * 35.0 && px(&pixels, x, y) == RED {
                solid += 1;
            }
        }
    }
    assert!(
        solid >= 2000,
        "the disk interior must be solid, got {solid}"
    );

    let pure = [style.window_bg, RED];
    let mut partials = 0;
    let mut inside_partials = 0;
    for y in 215..305usize {
        for x in 165..255usize {
            let dx = x as f32 + 0.5 - CENTER[0];
            let dy = y as f32 + 0.5 - CENTER[1];
            let dist = (dx * dx + dy * dy).sqrt();
            if !partial_between(px(&pixels, x, y), style.window_bg, RED, &pure) {
                continue;
            }
            if (RADIUS - 2.0..=RADIUS + 2.0).contains(&dist) {
                partials += 1;
            }
            if (RADIUS - 1.0..RADIUS - 0.25).contains(&dist) {
                inside_partials += 1;
            }
        }
    }
    assert!(
        partials >= 30,
        "the circle edge must be anti-aliased with partial coverage, got {partials}"
    );
    assert!(
        inside_partials >= 20,
        "the mask path must fade INSIDE the boundary too (fan+fringe stays crisp there), \
         got {inside_partials}"
    );

    for (x, y) in [(258, 260), (162, 260), (210, 302), (210, 218), (242, 292)] {
        assert_eq!(
            px(&pixels, x, y),
            style.window_bg,
            "outside the disk at ({x}, {y})"
        );
    }
}

#[test]
fn button_click_progression_is_visible_in_pixels() {
    let (mut backend, _device, d3dctx, target) = setup();
    let mut ctx = GuiContext::new(DEJAVU_SANS).expect("test font loads");
    let style = ctx.style;
    assert_ne!(style.widget_bg, style.widget_bg_hovered);
    assert_ne!(style.widget_bg_hovered, style.widget_bg_active);
    assert_ne!(style.widget_bg, style.widget_bg_active);

    let mut checked = false;
    let mut text = String::new();
    let mut button_text_size = [0.0; 2];
    render_frame(
        &mut ctx,
        &mut backend,
        &d3dctx,
        &target,
        input_at(AWAY, false),
        declare_widgets(&mut checked, &mut text, &mut button_text_size),
    );
    let layout = widget_layout(&style, ctx.font().line_height(), button_text_size);
    let btn = layout.button;
    let at = center(btn);
    let probe = ((btn[0] + 4.0) as usize, ((btn[1] + btn[3]) / 2.0) as usize);

    let mut clicked = false;
    macro_rules! ui_frame {
        ($input:expr) => {
            render_frame(&mut ctx, &mut backend, &d3dctx, &target, $input, |ui| {
                clicked = ui.button("Click me");
                ui.checkbox("Check me", &mut checked);
                ui.text_input("Name", &mut text);
            })
        };
    }

    let hover = ui_frame!(input_at(at, false));
    assert!(!clicked, "hover alone is not a click");
    assert_eq!(
        px(&hover, probe.0, probe.1),
        style.widget_bg_hovered,
        "hovered button shows the hover color"
    );
    let hovered_fill = count_exact_in(&hover, btn, 2.0, style.widget_bg_hovered);
    let area = ((btn[2] - btn[0] - 4.0) * (btn[3] - btn[1] - 4.0)) as usize;
    assert!(
        hovered_fill * 2 >= area,
        "hovered button interior is mostly the hover color: {hovered_fill}/{area}"
    );

    let press = ui_frame!(input_at(at, true));
    assert!(!clicked, "the press frame must not report the click");
    assert_eq!(
        px(&press, probe.0, probe.1),
        style.widget_bg_active,
        "pressed button shows the active color"
    );

    let release = ui_frame!(input_at(at, false));
    assert!(clicked, "release inside the button must click");
    assert_eq!(
        px(&release, probe.0, probe.1),
        style.widget_bg_hovered,
        "released-but-hovered button shows the hover color again"
    );

    ui_frame!(input_at(at, false));
    assert!(!clicked, "the click must not repeat");
}

#[test]
fn scripted_input_drives_checkbox_toggle_and_text_entry() {
    let (mut backend, _device, d3dctx, target) = setup();
    let mut ctx = GuiContext::new(DEJAVU_SANS).expect("test font loads");
    let style = ctx.style;

    let mut checked = false;
    let mut text = String::new();
    let mut button_text_size = [0.0; 2];
    let idle = render_frame(
        &mut ctx,
        &mut backend,
        &d3dctx,
        &target,
        input_at(AWAY, false),
        declare_widgets(&mut checked, &mut text, &mut button_text_size),
    );
    let layout = widget_layout(&style, ctx.font().line_height(), button_text_size);

    macro_rules! ui_frame {
        ($input:expr) => {
            render_frame(&mut ctx, &mut backend, &d3dctx, &target, $input, |ui| {
                ui.button("Click me");
                ui.checkbox("Check me", &mut checked);
                ui.text_input("Name", &mut text);
            })
        };
    }

    let box_at = center(layout.checkbox_box);
    ui_frame!(input_at(box_at, true));
    assert!(!checked, "the press frame must not toggle yet");
    let toggled = ui_frame!(input_at(box_at, false));
    assert!(checked, "release inside the checkbox must toggle it on");

    let cb = layout.checkbox_box;
    let inset = (cb[2] - cb[0]) / 4.0;
    let marks = count_exact_in(&toggled, cb, inset + 1.0, style.text);
    assert!(
        marks >= 10,
        "the check mark must render, got {marks} text pixels"
    );

    let field = layout.text_field;
    let field_at = [field[0] + 8.0, (field[1] + field[3]) / 2.0];
    ui_frame!(input_at(field_at, true));
    ui_frame!(input_at(field_at, false));

    let keys = [KeyEvent::Char('h'), KeyEvent::Char('i')];
    let typing_input = InputState {
        mouse_pos: AWAY,
        keys: &keys,
        viewport_size: [W as f32, H as f32],
        ..Default::default()
    };
    let typed = ui_frame!(typing_input);
    assert_eq!(
        text, "hi",
        "scripted Char events must reach the focused field"
    );

    let y0 = field[1].ceil() as usize;
    let y1 = field[3].floor() as usize;
    let mut diffs = 0;
    for y in y0..y1 {
        for x in (field[0].ceil() as usize)..((field[0] + 60.0) as usize) {
            if px(&typed, x, y) != px(&idle, x, y) {
                diffs += 1;
            }
        }
    }
    assert!(
        diffs >= 8,
        "typed text must render in the field, got {diffs} diff pixels"
    );
}

#[test]
fn color_emoji_atlas_renders_palette_colors_and_disabled_opacity() {
    let (mut backend, _device, context, target) = setup();
    let mut ctx = GuiContext::new(DEJAVU_SANS).unwrap();
    let emoji = ctx
        .add_font(
            include_bytes!("../../../core/tests/assets/Noto-COLRv1.ttf"),
            32.,
        )
        .unwrap();
    ctx.set_font_fallback_chain(restricted_gui::font::FontId::DEFAULT, &[emoji])
        .unwrap();
    ctx.set_color_atlas_texture(Some(restricted_gui::draw::TextureId(90)))
        .unwrap();
    let mut shaped = restricted_gui::font::ShapedText::default();
    let mut rect = [0.; 4];
    let mut peak = [0u8; 2];
    for (n, disabled) in [false, true].into_iter().enumerate() {
        let image = render_frame(
            &mut ctx,
            &mut backend,
            &context,
            &target,
            input_at(AWAY, false),
            |ui| {
                let scope = ui.begin_disabled("color opacity", disabled);
                rect = ui
                    .shaped_text(&mut shaped, "😀 👩‍💻 🇬🇧 👍🏽", Default::default())
                    .unwrap();
                ui.end_disabled(scope);
            },
        );
        let mut colored = 0;
        for y in rect[1].max(0.) as usize..(rect[3] as usize).min(H as usize) {
            for x in rect[0].max(0.) as usize..(rect[2] as usize).min(W as usize) {
                let pixel = px(&image, x, y);
                if pixel[0] > pixel[2].saturating_add(30) {
                    colored += 1;
                    peak[n] = peak[n].max(pixel[0]);
                }
            }
        }
        assert!(colored > 20, "colored glyph pixels missing: {colored}");
    }
    assert!(
        peak[0] > 220 && peak[1] < peak[0],
        "palette/opacity mismatch: {peak:?}"
    );
}
