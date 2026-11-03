use chrono::Local;
use restricted_gui::draw::DrawVert;
use restricted_gui::style::Style;
use std::fmt::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use winit::window::ResizeDirection;

pub const BAR_HEIGHT: f32 = 40.0;
const BUTTON_WIDTH: f32 = 46.0;
const RESIZE_EDGE: f32 = 6.0;
const WHITE_UV: [f32; 2] = [0.0, 0.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Content,
    Drag,
    Minimize,
    Maximize,
    Close,
    Resize(ResizeDirection),
}

pub fn hit_test(pos: [f32; 2], size: [f32; 2], maximized: bool) -> Hit {
    let [x, y] = pos;
    let [w, h] = size;
    if w <= 0.0 || h <= 0.0 || x < 0.0 || y < 0.0 || x >= w || y >= h {
        return Hit::Content;
    }
    let left = !maximized && x < RESIZE_EDGE;
    let right = !maximized && x >= w - RESIZE_EDGE;
    let top = !maximized && y < RESIZE_EDGE;
    let bottom = !maximized && y >= h - RESIZE_EDGE;
    let edge = match (left, right, top, bottom) {
        (true, _, true, _) => Some(ResizeDirection::NorthWest),
        (_, true, true, _) => Some(ResizeDirection::NorthEast),
        (true, _, _, true) => Some(ResizeDirection::SouthWest),
        (_, true, _, true) => Some(ResizeDirection::SouthEast),
        (true, _, _, _) => Some(ResizeDirection::West),
        (_, true, _, _) => Some(ResizeDirection::East),
        (_, _, true, _) => Some(ResizeDirection::North),
        (_, _, _, true) => Some(ResizeDirection::South),
        _ => None,
    };
    if let Some(direction) = edge {
        return Hit::Resize(direction);
    }
    if y < BAR_HEIGHT {
        if x >= w - BUTTON_WIDTH {
            Hit::Close
        } else if x >= w - 2.0 * BUTTON_WIDTH {
            Hit::Maximize
        } else if x >= w - 3.0 * BUTTON_WIDTH {
            Hit::Minimize
        } else {
            Hit::Drag
        }
    } else {
        Hit::Content
    }
}

pub struct ChromeMesh {
    pub vertices: Vec<DrawVert>,
    pub indices: Vec<u32>,
    clock_text: String,
    clock_second: Option<u64>,
}

impl ChromeMesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(6000),
            indices: Vec::with_capacity(9000),
            clock_text: String::with_capacity(32),
            clock_second: None,
        }
    }

    pub fn rebuild(
        &mut self,
        size: [f32; 2],
        hover: Option<[f32; 2]>,
        maximized: bool,
        style: &Style,
    ) {
        self.vertices.clear();
        self.indices.clear();
        let [w, h] = size;
        if w < 1.0 || h < 1.0 {
            return;
        }
        let ink = style.title_text;
        let hover_hit = hover.map(|p| hit_test(p, size, maximized));

        self.rect([0.0, 0.0], [w, BAR_HEIGHT], style.title_bg);
        let compact = w < 800.0;
        let detail_scale = if compact { 1.25 } else { 1.5 };
        self.text(
            [20.0, (BAR_HEIGHT - 7.0 * detail_scale) * 0.5],
            "restricted gui / preview",
            detail_scale,
            ink,
        );

        self.update_clock();
        let clock_text = std::mem::take(&mut self.clock_text);
        let clock_width = clock_text.len() as f32 * 6.0 * detail_scale;
        let clock_x = w - 3.0 * BUTTON_WIDTH - 24.0 - clock_width;
        self.text(
            [clock_x, (BAR_HEIGHT - 7.0 * detail_scale) * 0.5],
            &clock_text,
            detail_scale,
            ink,
        );
        self.clock_text = clock_text;

        for (hit, slot) in [
            (Hit::Minimize, 2.0),
            (Hit::Maximize, 1.0),
            (Hit::Close, 0.0),
        ] {
            let left = w - (slot + 1.0) * BUTTON_WIDTH;
            if hover_hit == Some(hit) {
                self.rect(
                    [left, 0.0],
                    [left + BUTTON_WIDTH, BAR_HEIGHT],
                    style.widget_bg,
                );
            }
            let cx = left + BUTTON_WIDTH * 0.5;
            let cy = BAR_HEIGHT * 0.5;
            match hit {
                Hit::Minimize => self.line([cx - 6.0, cy + 4.0], [cx + 6.0, cy + 4.0], 1.6, ink),
                Hit::Maximize => {
                    if maximized {
                        self.outline([cx - 4.0, cy - 3.0], [cx + 5.0, cy + 5.0], 1.3, ink);
                        self.line([cx - 2.0, cy - 5.0], [cx + 6.0, cy - 5.0], 1.3, ink);
                        self.line([cx + 6.0, cy - 5.0], [cx + 6.0, cy + 2.0], 1.3, ink);
                    } else {
                        self.outline([cx - 5.0, cy - 5.0], [cx + 5.0, cy + 5.0], 1.3, ink);
                    }
                }
                Hit::Close => {
                    self.line([cx - 5.0, cy - 5.0], [cx + 5.0, cy + 5.0], 1.6, ink);
                    self.line([cx + 5.0, cy - 5.0], [cx - 5.0, cy + 5.0], 1.6, ink);
                }
                _ => {}
            }
        }
    }

    pub fn append_inspection_clips(&mut self, clips: &[[f32; 4]], size: [f32; 2]) {
        for clip in clips {
            let min = [clip[0].clamp(0.0, size[0]), clip[1].clamp(0.0, size[1])];
            let max = [clip[2].clamp(0.0, size[0]), clip[3].clamp(0.0, size[1])];
            if max[0] - min[0] >= 2.0 && max[1] - min[1] >= 2.0 {
                self.outline(min, max, 1.0, [125, 207, 255, 120]);
            }
        }
    }

    pub fn append_inspection_selection(&mut self, rect: [f32; 4], size: [f32; 2]) {
        let min = [rect[0].clamp(0., size[0]), rect[1].clamp(0., size[1])];
        let max = [rect[2].clamp(0., size[0]), rect[3].clamp(0., size[1])];
        if max[0] > min[0] && max[1] > min[1] {
            self.outline(min, max, 2., [255, 187, 102, 255]);
        }
    }

    fn update_clock(&mut self) {
        let second = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        if self.clock_second == Some(second) {
            return;
        }
        self.clock_second = Some(second);
        self.clock_text.clear();
        let now = Local::now();
        write!(
            self.clock_text,
            "{} : {}",
            now.format("%H:%M:%S"),
            now.format("%d.%m.%Y")
        )
        .expect("writing to a String cannot fail");
    }

    fn text(&mut self, origin: [f32; 2], value: &str, scale: f32, col: [u8; 4]) {
        let mut x = origin[0];
        for ch in value.chars() {
            if let Some(rows) = glyph(ch) {
                for (row, bits) in rows.iter().enumerate() {
                    for column in 0..5 {
                        if bits & (1 << (4 - column)) != 0 {
                            let min = [x + column as f32 * scale, origin[1] + row as f32 * scale];
                            self.rect(min, [min[0] + scale, min[1] + scale], col);
                        }
                    }
                }
            }
            x += 6.0 * scale;
        }
    }

    fn outline(&mut self, min: [f32; 2], max: [f32; 2], width: f32, col: [u8; 4]) {
        self.rect(min, [max[0], min[1] + width], col);
        self.rect([min[0], max[1] - width], max, col);
        self.rect(
            [min[0], min[1] + width],
            [min[0] + width, max[1] - width],
            col,
        );
        self.rect(
            [max[0] - width, min[1] + width],
            [max[0], max[1] - width],
            col,
        );
    }

    fn line(&mut self, a: [f32; 2], b: [f32; 2], width: f32, col: [u8; 4]) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy).max(0.001);
        let n = [-dy / len * width * 0.5, dx / len * width * 0.5];
        let base = self.vertices.len() as u32;
        for p in [
            [a[0] + n[0], a[1] + n[1]],
            [b[0] + n[0], b[1] + n[1]],
            [b[0] - n[0], b[1] - n[1]],
            [a[0] - n[0], a[1] - n[1]],
        ] {
            self.vertices.push(vertex(p, col));
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn rect(&mut self, min: [f32; 2], max: [f32; 2], col: [u8; 4]) {
        let base = self.vertices.len() as u32;
        for p in [min, [max[0], min[1]], max, [min[0], max[1]]] {
            self.vertices.push(vertex(p, col));
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

fn vertex(pos: [f32; 2], col: [u8; 4]) -> DrawVert {
    DrawVert {
        pos,
        uv: WHITE_UV,
        col,
    }
}

fn glyph(ch: char) -> Option<[u8; 7]> {
    Some(match ch {
        'c' => [0, 0, 0b01110, 0b10000, 0b10000, 0b10000, 0b01110],
        'd' => [
            0b00001, 0b00001, 0b01101, 0b10011, 0b10001, 0b10011, 0b01101,
        ],
        'e' => [0, 0, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110],
        'g' => [0, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110],
        'i' => [0b00100, 0, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110],
        'p' => [0, 0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000],
        'r' => [0, 0, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000],
        's' => [0, 0, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110],
        't' => [
            0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00100, 0b00011,
        ],
        'u' => [0, 0, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101],
        'v' => [0, 0, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'w' => [0, 0, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        ':' => [0, 0b00100, 0b00100, 0, 0b00100, 0b00100, 0],
        '.' => [0, 0, 0, 0, 0, 0b00100, 0b00100],
        '/' => [
            0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000,
        ],
        ' ' => [0; 7],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_uses_the_selected_style_colors() {
        let mut mesh = ChromeMesh::new();
        let style = restricted_gui::style::Style {
            title_bg: [8, 9, 12, 255],
            title_text: [226, 229, 237, 255],
            widget_bg: [21, 24, 32, 255],
            ..Default::default()
        };
        mesh.rebuild([960.0, 720.0], Some([940.0, 20.0]), false, &style);
        assert_eq!(mesh.vertices[0].col, style.title_bg);
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.col == style.title_text));
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.col == style.widget_bg));
    }

    #[test]
    fn chrome_controls_and_resize_edges_have_distinct_hits() {
        let size = [960.0, 720.0];
        assert_eq!(hit_test([100.0, 20.0], size, false), Hit::Drag);
        assert_eq!(hit_test([830.0, 20.0], size, false), Hit::Minimize);
        assert_eq!(hit_test([890.0, 20.0], size, false), Hit::Maximize);
        assert_eq!(hit_test([940.0, 20.0], size, false), Hit::Close);
        assert_eq!(
            hit_test([959.0, 719.0], size, false),
            Hit::Resize(ResizeDirection::SouthEast)
        );
        assert_eq!(hit_test([200.0, 100.0], size, false), Hit::Content);
        assert_eq!(hit_test([959.0, 719.0], size, true), Hit::Content);
    }

    #[test]
    fn chrome_mesh_has_valid_indices_at_small_and_large_sizes() {
        let mut mesh = ChromeMesh::new();
        let vertex_capacity = mesh.vertices.capacity();
        let index_capacity = mesh.indices.capacity();
        for size in [[640.0, 480.0], [960.0, 720.0], [1600.0, 1000.0]] {
            mesh.rebuild(size, Some([size[0] - 25.0, 20.0]), false, &Style::default());
            assert!(mesh.indices.iter().all(|&i| i < mesh.vertices.len() as u32));
            assert!(mesh
                .vertices
                .iter()
                .all(|v| v.pos.iter().all(|p| p.is_finite())));
            assert_eq!(mesh.vertices.capacity(), vertex_capacity);
            assert_eq!(mesh.indices.capacity(), index_capacity);
        }
    }

    #[test]
    fn inspection_clips_are_bounded_to_viewport() {
        let mut mesh = ChromeMesh::new();
        mesh.rebuild([200.0, 100.0], None, false, &Style::default());
        let before = mesh.indices.len();
        let before_vertices = mesh.vertices.len();
        mesh.append_inspection_clips(&[[-20.0, -10.0, 80.0, 70.0]], [200.0, 100.0]);
        assert!(mesh.indices.len() > before);
        assert!(mesh.vertices[before_vertices..].iter().all(|vertex| {
            (0.0..=200.0).contains(&vertex.pos[0]) && (0.0..=100.0).contains(&vertex.pos[1])
        }));
    }

    #[test]
    fn title_font_covers_every_displayed_character() {
        for ch in "restricted gui / preview 00:00:00 : 00.00.0000".chars() {
            assert!(glyph(ch).is_some(), "missing title glyph: {ch}");
        }
    }
}
