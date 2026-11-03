mod command_plan;
pub use command_plan::{
    CommandPlan, DrawCalls, GeometryRange, PlanError, PlannedDraw, QuadInstance,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AlphaMode {
    #[default]
    Straight,
    Premultiplied,
}
impl AlphaMode {
    pub fn clear_color(self, mut color: [f32; 4]) -> [f32; 4] {
        if self == Self::Premultiplied {
            let alpha = color[3];
            for channel in &mut color[..3] {
                *channel *= alpha;
            }
        }
        color
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DrawOptimization {
    pub sort_disjoint: bool,
    pub instance_quads: bool,
}

impl DrawOptimization {
    pub fn enabled(self) -> bool {
        self.sort_disjoint || self.instance_quads
    }
}

pub fn alpha_flags(input: AlphaMode, output: AlphaMode, secondary: AlphaMode) -> u32 {
    u32::from(input == AlphaMode::Premultiplied)
        | (u32::from(output == AlphaMode::Premultiplied) << 1)
        | (u32::from(secondary == AlphaMode::Premultiplied) << 2)
}

pub fn valid_sample_count(samples: u32) -> bool {
    matches!(samples, 1 | 2 | 4 | 8)
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ColorMode {
    #[default]
    Legacy,
    Srgb,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextureEncoding {
    #[default]
    Linear,
    Srgb,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadStrategy {
    DiscardMap,
    PersistentMapped,
    QueueWrite,
}
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RenderStats {
    pub draw_calls: u64,
    pub uploaded_bytes: u64,
    pub vertex_capacity: u64,
    pub index_capacity: u64,
    pub upload_capacity: u64,
    pub pool_bytes: u64,
    pub pool_entries: u64,
    pub allocations: u64,
    pub reuses: u64,
    pub gpu_ms: Option<f64>,
}
