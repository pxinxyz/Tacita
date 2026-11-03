use tacita::rendering::{AlphaMode, DrawOptimization};

pub const FRAMES_PER_CASE: u64 = 120;
const CASES: usize = 8;
#[derive(Clone, Copy)]
pub struct Case {
    pub aa: bool,
    pub samples: usize,
    pub alpha: AlphaMode,
    pub optimization: DrawOptimization,
}
pub fn case(frame: u64, supported: [bool; 4]) -> Option<Case> {
    let index = (frame / FRAMES_PER_CASE) as usize;
    let high = supported
        .iter()
        .rposition(|supported| *supported)
        .unwrap_or(0);
    let (aa, samples, premultiplied, sort, instances) = match index {
        0 => (true, 0, false, false, false),
        1 => (true, high, true, true, true),
        2 => (false, high, true, true, true),
        3 => (true, high, true, true, true),
        4 => (false, high, true, true, false),
        5 => (false, high, false, false, false),
        6 => (false, 0, true, true, true),
        7 => (true, 0, false, false, false),
        _ => return None,
    };
    Some(Case {
        aa,
        samples,
        alpha: if premultiplied {
            AlphaMode::Premultiplied
        } else {
            AlphaMode::Straight
        },
        optimization: DrawOptimization {
            sort_disjoint: sort,
            instance_quads: instances,
        },
    })
}
pub fn complete(frame: u64) -> bool {
    frame >= FRAMES_PER_CASE * CASES as u64
}
