use tacita::draw::DrawVert;

pub const BAR_HEIGHT: f32 = 0.;

pub struct ChromeMesh {
    pub vertices: Vec<DrawVert>,
    pub indices: Vec<u32>,
}

impl ChromeMesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(256),
            indices: Vec::with_capacity(384),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn append_inspection_clips(&mut self, clips: &[[f32; 4]], size: [f32; 2]) {
        for &rect in clips {
            self.outline(rect, size, 1., [151, 196, 210, 140]);
        }
    }

    pub fn append_inspection_selection(&mut self, rect: [f32; 4], size: [f32; 2]) {
        self.outline(rect, size, 2., [222, 182, 125, 255]);
    }

    fn outline(&mut self, rect: [f32; 4], size: [f32; 2], width: f32, color: [u8; 4]) {
        let min = [rect[0].clamp(0., size[0]), rect[1].clamp(0., size[1])];
        let max = [rect[2].clamp(0., size[0]), rect[3].clamp(0., size[1])];
        if max[0] <= min[0] || max[1] <= min[1] {
            return;
        }
        self.rect(min, [max[0], min[1] + width], color);
        self.rect([min[0], max[1] - width], max, color);
        self.rect(
            [min[0], min[1] + width],
            [min[0] + width, max[1] - width],
            color,
        );
        self.rect(
            [max[0] - width, min[1] + width],
            [max[0], max[1] - width],
            color,
        );
    }

    fn rect(&mut self, min: [f32; 2], max: [f32; 2], color: [u8; 4]) {
        let base = self.vertices.len() as u32;
        for pos in [min, [max[0], min[1]], max, [min[0], max[1]]] {
            self.vertices.push(DrawVert {
                pos,
                uv: [0.; 2],
                col: color,
            });
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}
