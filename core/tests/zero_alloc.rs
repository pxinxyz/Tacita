use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use tacita::context::Context;
use tacita::draw::TextureId;
use tacita::font::FontId;
use tacita::input::{InputState, Key, KeyBinding, KeyEvent};
use tacita::ui::Ui;
use tacita::ScrollFlags;

struct CountingAllocator;

static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn allocation_count() -> usize {
    ALLOCATION_COUNT.load(Ordering::SeqCst)
}

const DEJAVU_SANS: &[u8] = include_bytes!("assets/DejaVuSans.ttf");

const VIEWPORT: [f32; 2] = [800.0, 600.0];

const AWAY: [f32; 2] = [-50.0, -50.0];

const WINDOW_POS: [f32; 2] = [60.0, 60.0];

const MAIN_RECT: [f32; 4] = [60.0, 60.0, 360.0, 460.0];

const ITEMS: [&str; 4] = ["Alpha", "Beta", "Gamma", "Delta"];

const LOG_LINES: [&str; 10] = [
    "log line 00",
    "log line 01",
    "log line 02",
    "log line 03",
    "log line 04",
    "log line 05",
    "log line 06",
    "log line 07",
    "log line 08",
    "log line 09",
];

const LONG_ITEMS: [&str; 20] = [
    "long item 00",
    "long item 01",
    "long item 02",
    "long item 03",
    "long item 04",
    "long item 05",
    "long item 06",
    "long item 07",
    "long item 08",
    "long item 09",
    "long item 10",
    "long item 11",
    "long item 12",
    "long item 13",
    "long item 14",
    "long item 15",
    "long item 16",
    "long item 17",
    "long item 18",
    "long item 19",
];

const PENTAGON: [[f32; 2]; 5] = [
    [90.0, 300.0],
    [140.0, 280.0],
    [190.0, 300.0],
    [170.0, 350.0],
    [110.0, 350.0],
];

const COMBO_ROW: usize = 5;

fn row_height(ctx: &Context) -> f32 {
    ctx.font().line_height() + 2.0 * ctx.style.frame_padding[1]
}

fn row_top(ctx: &Context, row: usize) -> f32 {
    let row_h = row_height(ctx);
    let content_y = WINDOW_POS[1] + row_h + ctx.style.window_padding[1];
    content_y + row as f32 * (row_h + ctx.style.item_spacing[1])
}

fn row_point(ctx: &Context, row: usize) -> [f32; 2] {
    [
        WINDOW_POS[0] + ctx.style.window_padding[0] + 4.0,
        row_top(ctx, row) + row_height(ctx) / 2.0,
    ]
}

struct CLayout {
    row_h: f32,
    lh: f32,
    s: f32,
    y0: f32,
}

impl CLayout {
    fn of(ctx: &Context) -> Self {
        let row_h = row_height(ctx);
        Self {
            row_h,
            lh: ctx.font().line_height(),
            s: ctx.style.item_spacing[1],
            y0: WINDOW_POS[1] + row_h + ctx.style.window_padding[1],
        }
    }

    fn top(&self, row: usize) -> f32 {
        self.y0 + row as f32 * (self.row_h + self.s)
    }

    fn point(&self, row: usize) -> [f32; 2] {
        [74.0, self.top(row) + self.row_h / 2.0]
    }

    fn region_rect(&self) -> [f32; 4] {
        let top = self.top(3);
        [70.0, top, 270.0, top + 48.0]
    }

    fn region_point(&self) -> [f32; 2] {
        [75.0, self.top(3) + 5.0]
    }

    fn header_point(&self) -> [f32; 2] {
        [74.0, self.top(3) + 48.0 + self.s + self.row_h / 2.0]
    }

    fn tab_bar_top(&self) -> f32 {
        self.top(3) + 48.0 + self.s + self.row_h + self.s
    }

    fn tab_two_point(&self) -> [f32; 2] {
        [92.0, self.tab_bar_top() + self.row_h / 2.0]
    }

    fn long_combo_top(&self) -> f32 {
        self.tab_bar_top() + (self.lh + self.s) + self.row_h + self.s + 2.0 * (self.lh + self.s)
    }

    fn long_combo_point(&self) -> [f32; 2] {
        [74.0, self.long_combo_top() + self.row_h / 2.0]
    }

    fn long_combo_button_bottom(&self) -> f32 {
        self.long_combo_top() + self.row_h
    }
}

struct Rig {
    ctx: Context,
    big_font: FontId,
    buf: String,
    gain: f32,
    enabled: bool,
    binding: KeyBinding,
    selected: usize,
    popup_visible: bool,
    vertex_count: usize,
    clips: [[f32; 4]; 128],
    clip_count: usize,
    region_min_y: Option<f32>,
    popup_scroll_min_y: Option<f32>,
    slider_changed: bool,
    keybind_changed: bool,
    selected_long: usize,
}

impl Rig {
    fn new() -> Self {
        let mut ctx = Context::new(DEJAVU_SANS).expect("DejaVu Sans must load");
        let big_font = ctx
            .add_font(DEJAVU_SANS, 24.0)
            .expect("a second face must register");
        Self {
            ctx,
            big_font,
            buf: String::from("hello"),
            gain: 0.5,
            enabled: true,
            binding: KeyBinding::Key(Key::G),
            selected: 1,
            popup_visible: false,
            vertex_count: 0,
            clips: [[0.0; 4]; 128],
            clip_count: 0,
            region_min_y: None,
            popup_scroll_min_y: None,
            slider_changed: false,
            keybind_changed: false,
            selected_long: 0,
        }
    }

    fn frame(&mut self, input: InputState<'_>) {
        self.ctx.set_input(input);
        self.ctx.new_frame(0.016);
        {
            let mut ui = self.ctx.ui();
            ui.window("Main", ScrollFlags::NONE, |ui| {
                ui.text_input("##t", &mut self.buf);
                ui.slider_float("Gain", &mut self.gain, 0.0..=1.0);
                ui.checkbox("Enabled", &mut self.enabled);
                ui.button("Apply");
                ui.keybind("Shortcut", &mut self.binding);
                ui.combo("Combo", &ITEMS, &mut self.selected);
                ui.scroll_region("Log", [200.0, 48.0], |ui| {
                    ui.label("line one");
                    ui.label("line two");
                    ui.label("line three");
                });
                ui.draw_polygon_filled(&PENTAGON, [200, 200, 60, 255]);
            });
        }
        self.observe();
    }

    fn frame_c(&mut self, input: InputState<'_>, dt: f32) {
        self.ctx.set_input(input);
        self.ctx.new_frame(dt);
        {
            let mut ui = self.ctx.ui();
            ui.window("Main", ScrollFlags::NONE, |ui| {
                ui.button("Apply");
                ui.tooltip_text("Apply the settings");
                self.keybind_changed |= ui.keybind("Shortcut", &mut self.binding);
                self.slider_changed |= ui.slider_float("Gain", &mut self.gain, 0.0..=1.0);
                ui.scroll_region("Log", [200.0, 48.0], |ui| {
                    for line in LOG_LINES {
                        ui.label(line);
                    }
                    ui.scroll_region("Inner", [180.0, 32.0], |ui| {
                        for line in LOG_LINES {
                            ui.label(line);
                        }
                    });
                });
                ui.collapsing_header("Settings", |ui| {
                    ui.label("Header body");
                });
                ui.tab_bar("Tabs", |bar| {
                    bar.tab("##1", |ui| {
                        ui.label("First tab body");
                    });
                    bar.tab("Two", |ui| {
                        ui.label("Second tab body");
                        ui.label("Extra line");
                    });
                });
                ui.combo("Long", &LONG_ITEMS, &mut self.selected_long);
            });
            ui.window("Aux", ScrollFlags::NONE, |ui| {
                ui.label("Aux window");
                ui.button("Aux button");
            });
        }
        self.observe();
    }

    fn observe(&mut self) {
        let layout = CLayout::of(&self.ctx);
        let region = layout.region_rect();
        let inset = 1.0 + 4.0 * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
        let region_clip = [
            region[0] + inset,
            region[1] + inset,
            region[2] - inset,
            region[3] - inset,
        ];
        let popup_top = layout.long_combo_button_bottom() - 1.0;
        let data = self.ctx.render();
        self.popup_visible = data
            .commands
            .iter()
            .any(|cmd| cmd.clip_rect == [0.0, 0.0, VIEWPORT[0], VIEWPORT[1]]);
        self.vertex_count = data.vertices.len();
        assert!(
            data.commands.len() <= self.clips.len(),
            "frame produced more commands than the observation array holds"
        );
        self.clip_count = data.commands.len();
        let mut region_min_y: Option<f32> = None;
        let mut popup_min_y: Option<f32> = None;
        let mut offset = 0usize;
        for (i, cmd) in data.commands.iter().enumerate() {
            self.clips[i] = cmd.clip_rect;
            let next = offset + cmd.elem_count as usize;
            if cmd.texture == TextureId::FONT {
                if cmd.clip_rect == region_clip {
                    for &idx in &data.indices[offset..next] {
                        let y = data.vertices[idx as usize].pos[1];
                        region_min_y = Some(match region_min_y {
                            Some(m) => m.min(y),
                            None => y,
                        });
                    }
                }
                if cmd.clip_rect[1] > popup_top {
                    for &idx in &data.indices[offset..next] {
                        let y = data.vertices[idx as usize].pos[1];
                        popup_min_y = Some(match popup_min_y {
                            Some(m) => m.min(y),
                            None => y,
                        });
                    }
                }
            }
            offset = next;
        }
        self.region_min_y = region_min_y;
        self.popup_scroll_min_y = popup_min_y;
    }

    fn idle_frame(&mut self, keys: &[KeyEvent]) {
        self.frame(InputState {
            mouse_pos: AWAY,
            keys,
            viewport_size: VIEWPORT,
            ..Default::default()
        });
    }

    fn click_frame(&mut self, point: [f32; 2], left_down: bool) {
        self.frame(InputState {
            mouse_pos: point,
            mouse_down: [left_down, false, false, false, false],
            viewport_size: VIEWPORT,
            ..Default::default()
        });
    }

    fn c_frame(
        &mut self,
        mouse: [f32; 2],
        left_down: bool,
        wheel: f32,
        keys: &[KeyEvent],
        dt: f32,
    ) {
        self.frame_c(
            InputState {
                mouse_pos: mouse,
                mouse_down: [left_down, false, false, false, false],
                mouse_wheel: wheel,
                keys,
                viewport_size: VIEWPORT,
            },
            dt,
        );
    }

    fn clips_snapshot(&self) -> ([[f32; 4]; 128], usize) {
        (self.clips, self.clip_count)
    }

    fn font_switch_frame(&mut self, scoped: bool) {
        self.ctx.set_input(InputState {
            mouse_pos: AWAY,
            viewport_size: VIEWPORT,
            ..Default::default()
        });
        self.ctx.new_frame(0.016);
        {
            let mut ui = self.ctx.ui();
            ui.window("Fonts", ScrollFlags::NONE, |ui| {
                ui.label("plain");
                if scoped {
                    ui.with_font(self.big_font, |ui| {
                        ui.label("big face");
                        let _ = ui.button("Big");
                    });
                }
            });
        }
        let data = self.ctx.render();
        self.vertex_count = data.vertices.len();
    }
}

#[test]
fn steady_frames_allocate_zero_after_warmup() {
    let warmup_start = allocation_count();
    let mut rig = Rig::new();

    let field = row_point(&rig.ctx, 0);
    rig.click_frame(field, true);
    rig.click_frame(field, false);
    rig.idle_frame(&[KeyEvent::Press(Key::Right)]);
    rig.idle_frame(&[KeyEvent::Press(Key::Left)]);

    let warmup_allocs = allocation_count() - warmup_start;
    assert!(
        warmup_allocs > 0,
        "sanity: warmup must allocate (context, window, states, atlas, buffers)"
    );

    rig.idle_frame(&[KeyEvent::Char('!')]);
    assert_ne!(
        rig.buf, "hello",
        "the text input must be focused after the warmup click"
    );
    rig.buf = String::from("hello");

    let measured = allocation_count();
    for i in 0..16 {
        let key = if i % 2 == 0 { Key::Left } else { Key::Right };
        rig.idle_frame(&[KeyEvent::Press(key)]);
    }
    let scenario_a_allocs = allocation_count() - measured;
    assert_eq!(
        scenario_a_allocs, 0,
        "steady frames with a focused text input must allocate zero bytes"
    );

    rig.click_frame(AWAY, true);
    rig.click_frame(AWAY, false);
    let combo_button = row_point(&rig.ctx, COMBO_ROW);
    rig.click_frame(combo_button, true);
    rig.click_frame(combo_button, false);
    rig.idle_frame(&[]);
    rig.idle_frame(&[]);
    assert!(
        rig.popup_visible,
        "the combo popup must be open for scenario B (overlay clip = viewport)"
    );

    let measured = allocation_count();
    for _ in 0..16 {
        rig.idle_frame(&[]);
    }
    let scenario_b_allocs = allocation_count() - measured;
    assert!(
        rig.popup_visible,
        "the combo popup must still be open at the end of scenario B"
    );
    assert_eq!(
        scenario_b_allocs, 0,
        "steady frames with an open popup must allocate zero bytes"
    );

    rig.click_frame(AWAY, true);
    rig.click_frame(AWAY, false);
    assert!(
        !rig.popup_visible,
        "the combo popup must be closed before scenario C"
    );

    let layout = CLayout::of(&rig.ctx);

    rig.c_frame(AWAY, false, 0.0, &[], 0.016);
    rig.c_frame([100.0, 90.0], true, 0.0, &[], 0.016);
    rig.c_frame([420.0, 110.0], true, 0.0, &[], 0.016);
    rig.c_frame([420.0, 110.0], false, 0.0, &[], 0.016);

    let vertices_tab_one = rig.vertex_count;
    rig.c_frame(layout.tab_two_point(), true, 0.0, &[], 0.016);
    rig.c_frame(layout.tab_two_point(), false, 0.0, &[], 0.016);
    rig.c_frame(AWAY, false, 0.0, &[], 0.016);
    let vertices_tab_two = rig.vertex_count;
    assert_ne!(
        vertices_tab_one, vertices_tab_two,
        "clicking tab Two must switch the visible tab content"
    );
    rig.c_frame(layout.header_point(), true, 0.0, &[], 0.016);
    rig.c_frame(layout.header_point(), false, 0.0, &[], 0.016);
    rig.c_frame(AWAY, false, 0.0, &[], 0.016);
    assert_ne!(
        vertices_tab_two, rig.vertex_count,
        "clicking the collapsing header must open its content"
    );

    for _ in 0..6 {
        rig.c_frame(layout.point(0), false, 0.0, &[], 0.1);
    }
    assert!(
        rig.popup_visible,
        "the tooltip must be visible before scenario C1's measured frames"
    );

    let measured = allocation_count();
    for _ in 0..16 {
        rig.c_frame(layout.point(0), false, 0.0, &[], 0.016);
    }
    let scenario_c1_allocs = allocation_count() - measured;
    assert!(
        rig.popup_visible,
        "the tooltip must still be visible at the end of scenario C1"
    );
    let (clips, clip_count) = rig.clips_snapshot();
    assert!(
        clips[..clip_count].contains(&MAIN_RECT),
        "window Main's geometry must be present (clip = window rect)"
    );
    assert!(
        clips[..clip_count].contains(&[404.0, 104.0, 704.0, 504.0]),
        "window Aux's geometry must be present at its dragged position"
    );
    assert_eq!(
        scenario_c1_allocs, 0,
        "steady multi-window frames with tab bar, open header, and active tooltip must allocate zero"
    );

    rig.c_frame([420.0, 110.0], true, 0.0, &[], 0.016);
    let measured = allocation_count();
    let (mut prev_clips, mut prev_count) = rig.clips_snapshot();
    let mut window_moved = false;
    for i in 0..16 {
        rig.c_frame(
            [424.0 + 4.0 * i as f32, 111.0 + i as f32],
            true,
            0.0,
            &[],
            0.016,
        );
        let (clips, count) = rig.clips_snapshot();
        if clips[..count] != prev_clips[..prev_count] {
            window_moved = true;
        }
        prev_clips = clips;
        prev_count = count;
    }
    let scenario_c2_allocs = allocation_count() - measured;
    rig.c_frame(AWAY, false, 0.0, &[], 0.016);
    assert!(window_moved, "the dragged window's geometry must move");
    assert_eq!(
        scenario_c2_allocs, 0,
        "frames with a title-bar drag in progress must allocate zero"
    );

    rig.c_frame(layout.point(2), true, 0.0, &[], 0.016);
    rig.slider_changed = false;
    let gain_before = rig.gain;
    let measured = allocation_count();
    for i in 0..16 {
        let mut point = layout.point(2);
        point[0] += 8.0 * i as f32;
        rig.c_frame(point, true, 0.0, &[], 0.016);
    }
    let scenario_c3_allocs = allocation_count() - measured;
    rig.c_frame(layout.point(2), false, 0.0, &[], 0.016);
    assert!(
        rig.slider_changed,
        "the slider must report value changes during the drag"
    );
    assert_ne!(
        gain_before, rig.gain,
        "the slider value must move during the drag"
    );
    assert_eq!(
        scenario_c3_allocs, 0,
        "frames with a slider mid-drag must allocate zero"
    );

    rig.c_frame(layout.region_point(), false, 0.0, &[], 0.016);
    assert!(
        rig.region_min_y.is_some(),
        "the scroll region must emit glyph vertices to observe"
    );
    let measured = allocation_count();
    let mut y_min_seen = f32::MAX;
    let mut y_max_seen = f32::MIN;
    for i in 0..16 {
        let wheel = if i < 8 { -1.0 } else { 1.0 };
        rig.c_frame(layout.region_point(), false, wheel, &[], 0.016);
        let y = rig
            .region_min_y
            .expect("the scroll region must keep emitting glyphs");
        y_min_seen = y_min_seen.min(y);
        y_max_seen = y_max_seen.max(y);
    }
    let scenario_c4_allocs = allocation_count() - measured;
    assert_ne!(
        y_min_seen, y_max_seen,
        "the scroll offset must change under the wheel (content y must move)"
    );
    assert_eq!(
        scenario_c4_allocs, 0,
        "frames with wheel scrolling must allocate zero"
    );

    rig.c_frame(layout.point(1), true, 0.0, &[], 0.016);
    rig.c_frame(layout.point(1), false, 0.0, &[], 0.016);
    rig.c_frame(AWAY, false, 0.0, &[], 0.016);
    let measured = allocation_count();
    for _ in 0..16 {
        rig.c_frame(AWAY, false, 0.0, &[], 0.016);
    }
    let scenario_c5_allocs = allocation_count() - measured;
    assert_eq!(
        scenario_c5_allocs, 0,
        "frames with an armed keybind must allocate zero"
    );
    rig.keybind_changed = false;
    rig.c_frame(AWAY, false, 0.0, &[KeyEvent::Press(Key::K)], 0.016);
    assert!(
        rig.keybind_changed,
        "the keybind must report a capture after the measured frames"
    );
    assert_eq!(
        rig.binding,
        KeyBinding::Key(Key::K),
        "the keybind must have been armed to bind the pressed key"
    );

    rig.c_frame(layout.long_combo_point(), true, 0.0, &[], 0.016);
    rig.c_frame(layout.long_combo_point(), false, 0.0, &[], 0.016);
    for _ in 0..8 {
        rig.c_frame([100.0, 500.0], false, -1.0, &[], 0.016);
    }
    for _ in 0..8 {
        rig.c_frame([100.0, 500.0], false, 1.0, &[], 0.016);
    }
    assert!(
        rig.popup_visible,
        "the long combo's popup must be open for scenario C6"
    );
    assert!(
        rig.popup_scroll_min_y.is_some(),
        "the open popup must emit scrollable glyph rows to observe"
    );

    let measured = allocation_count();
    let mut y_min_seen = f32::MAX;
    let mut y_max_seen = f32::MIN;
    for i in 0..16 {
        let wheel = if i < 8 { -1.0 } else { 1.0 };
        rig.c_frame([100.0, 500.0], false, wheel, &[], 0.016);
        let y = rig
            .popup_scroll_min_y
            .expect("the popup must keep emitting glyph rows");
        y_min_seen = y_min_seen.min(y);
        y_max_seen = y_max_seen.max(y);
    }
    let scenario_c6_allocs = allocation_count() - measured;
    assert!(
        rig.popup_visible,
        "the long combo's popup must still be open at the end of scenario C6"
    );
    assert_ne!(
        y_min_seen, y_max_seen,
        "the popup's scroll offset must change under the wheel (row y must move)"
    );
    assert_eq!(
        scenario_c6_allocs, 0,
        "frames wheel-scrolling a popup must allocate zero"
    );

    rig.font_switch_frame(false);
    let vertices_unscoped = rig.vertex_count;
    rig.font_switch_frame(true);
    rig.font_switch_frame(true);
    assert!(
        rig.vertex_count > vertices_unscoped,
        "the scoped frame must emit more geometry (big-face label + button)"
    );

    let measured = allocation_count();
    for _ in 0..16 {
        rig.font_switch_frame(true);
    }
    let scenario_d_allocs = allocation_count() - measured;
    assert_eq!(
        scenario_d_allocs, 0,
        "steady frames switching font faces via with_font must allocate zero"
    );

    type EVert = ([f32; 2], [u8; 4]);
    const SC_MARK: [u8; 4] = [255, 0, 0, 255];
    const BODY_MARK: [u8; 4] = [0, 255, 0, 255];
    const E_MARK_SIZE: f32 = 16.0;

    fn e_marker(ui: &mut Ui<'_>, color: [u8; 4]) {
        let rect = ui.allocate([E_MARK_SIZE, E_MARK_SIZE]);
        ui.draw_rect_filled([rect[0], rect[1]], [rect[2], rect[3]], color);
    }

    fn e_find_marker(verts: &[EVert], color: [u8; 4]) -> [f32; 4] {
        verts
            .windows(4)
            .find_map(|quad| {
                let (min, max) = (quad[0].0, quad[2].0);
                let matches = quad.iter().all(|v| v.1 == color)
                    && (max[0] - min[0] - E_MARK_SIZE).abs() < 1e-3
                    && (max[1] - min[1] - E_MARK_SIZE).abs() < 1e-3;
                if matches {
                    Some([min[0], min[1], max[0], max[1]])
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("scenario E marker {color:?} not found"))
    }

    fn e_declare(ui: &mut Ui<'_>) {
        ui.window("Scroll", ScrollFlags::BAR, |ui| {
            ui.child("sc", [220.0, 80.0], ScrollFlags::BAR, |ui| {
                e_marker(ui, SC_MARK);
                for line in LOG_LINES {
                    ui.label(line);
                }
            });
            ui.child("nc", [220.0, 80.0], ScrollFlags::NONE, |ui| {
                for line in LOG_LINES {
                    ui.label(line);
                }
            });
            e_marker(ui, BODY_MARK);
            for line in LOG_LINES {
                ui.label(line);
            }
            for line in LOG_LINES {
                ui.label(line);
            }
        });
    }

    fn e_frame(ctx: &mut Context, mouse: [f32; 2], left_down: bool, wheel: f32) {
        ctx.set_input(InputState {
            mouse_pos: mouse,
            mouse_down: [left_down, false, false, false, false],
            mouse_wheel: wheel,
            keys: &[],
            viewport_size: VIEWPORT,
        });
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            e_declare(&mut ui);
        }
        ctx.render();
    }

    fn e_observe(ctx: &mut Context, mouse: [f32; 2], left_down: bool, wheel: f32) -> Vec<EVert> {
        ctx.set_input(InputState {
            mouse_pos: mouse,
            mouse_down: [left_down, false, false, false, false],
            mouse_wheel: wheel,
            keys: &[],
            viewport_size: VIEWPORT,
        });
        ctx.new_frame(0.016);
        {
            let mut ui = ctx.ui();
            e_declare(&mut ui);
        }
        let data = ctx.render();
        data.vertices.iter().map(|v| (v.pos, v.col)).collect()
    }

    let mut ctx_e = Context::new(DEJAVU_SANS).expect("DejaVu Sans must load");
    let title_bar_h = row_height(&ctx_e);
    let content_origin = [
        WINDOW_POS[0] + ctx_e.style.window_padding[0],
        WINDOW_POS[1] + title_bar_h + ctx_e.style.window_padding[1],
    ];
    let spacing = ctx_e.style.item_spacing[1];
    let child_h = 80.0;
    let over_child = [content_origin[0] + 10.0, content_origin[1] + 10.0];
    let over_body = [
        content_origin[0] + 10.0,
        content_origin[1] + 2.0 * child_h + 2.0 * spacing + 10.0,
    ];
    let body_right = WINDOW_POS[0] + 300.0;
    let body_top = WINDOW_POS[1] + title_bar_h;
    let track_left = body_right - ctx_e.style.scrollbar_size;
    let drag_press = [
        track_left + ctx_e.style.scrollbar_size / 2.0,
        body_top + 30.0,
    ];
    let reset_probe = [content_origin[0] + 240.0, content_origin[1] + 10.0];

    e_frame(&mut ctx_e, AWAY, false, 0.0);
    e_frame(&mut ctx_e, AWAY, false, 0.0);
    e_frame(&mut ctx_e, over_child, false, -1.0);
    e_frame(&mut ctx_e, over_child, false, 1.0);
    e_frame(&mut ctx_e, over_body, false, -1.0);
    e_frame(&mut ctx_e, over_body, false, 1.0);
    e_frame(&mut ctx_e, drag_press, true, 0.0);
    e_frame(&mut ctx_e, [drag_press[0], drag_press[1] + 60.0], true, 0.0);
    e_frame(&mut ctx_e, AWAY, false, 0.0);
    e_frame(&mut ctx_e, reset_probe, false, 10_000.0);
    e_frame(&mut ctx_e, AWAY, false, 0.0);

    let measured = allocation_count();
    for _ in 0..16 {
        e_frame(&mut ctx_e, AWAY, false, 0.0);
    }
    let scenario_e1_allocs = allocation_count() - measured;
    assert_eq!(
        scenario_e1_allocs, 0,
        "idle frames of a BAR window holding a BAR child and a NONE child must allocate zero"
    );

    let before = e_observe(&mut ctx_e, over_child, false, 0.0);
    let sc_before = e_find_marker(&before, SC_MARK);
    let body_before = e_find_marker(&before, BODY_MARK);
    let measured = allocation_count();
    for _ in 0..16 {
        e_frame(&mut ctx_e, over_child, false, -1.0);
    }
    let scenario_e2_allocs = allocation_count() - measured;
    let after = e_observe(&mut ctx_e, over_child, false, 0.0);
    let sc_after = e_find_marker(&after, SC_MARK);
    let body_after = e_find_marker(&after, BODY_MARK);
    assert_ne!(
        sc_before[1], sc_after[1],
        "the child's scroll offset must move under the wheel"
    );
    assert_eq!(
        body_before[1], body_after[1],
        "the window must not co-scroll when its child claims the wheel"
    );
    assert_eq!(
        scenario_e2_allocs, 0,
        "frames wheel-scrolling the child must allocate zero"
    );

    let before = e_observe(&mut ctx_e, over_body, false, 0.0);
    let body_before = e_find_marker(&before, BODY_MARK);
    let measured = allocation_count();
    for _ in 0..16 {
        e_frame(&mut ctx_e, over_body, false, -1.0);
    }
    let scenario_e3_allocs = allocation_count() - measured;
    let after = e_observe(&mut ctx_e, over_body, false, 0.0);
    let body_after = e_find_marker(&after, BODY_MARK);
    assert_ne!(
        body_before[1], body_after[1],
        "the window's own scroll offset must move under the wheel"
    );
    assert_eq!(
        scenario_e3_allocs, 0,
        "frames wheel-scrolling the window's bare body must allocate zero"
    );

    let before = e_observe(&mut ctx_e, AWAY, false, 0.0);
    let body_before = e_find_marker(&before, BODY_MARK);
    e_frame(&mut ctx_e, drag_press, true, 0.0);
    let measured = allocation_count();
    for i in 0..16 {
        let y = drag_press[1] + i as f32 * 4.0;
        e_frame(&mut ctx_e, [drag_press[0], y], true, 0.0);
    }
    let scenario_e4_allocs = allocation_count() - measured;
    let after = e_observe(&mut ctx_e, [drag_press[0], drag_press[1] + 60.0], true, 0.0);
    e_frame(&mut ctx_e, AWAY, false, 0.0);
    let body_after = e_find_marker(&after, BODY_MARK);
    assert_ne!(
        body_before[1], body_after[1],
        "the window's content must move while its scrollbar is being dragged"
    );
    assert_eq!(
        scenario_e4_allocs, 0,
        "frames with an active scrollbar drag must allocate zero"
    );
}
