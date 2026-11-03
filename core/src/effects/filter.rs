use super::{DrawEffects, EffectError};
use crate::{
    draw::{DrawCmd, DrawData, DrawVert, TextureId},
    ui::Ui,
};

pub const MAX_FILTER_PASSES: usize = 15;
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilterKind {
    #[default]
    Copy = 0,
    GaussianX = 1,
    GaussianY = 2,
    KawaseDown = 3,
    KawaseUp = 4,
    Threshold = 5,
    BloomCompose = 6,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImageFilter {
    pub kind: FilterKind,
    pub overwrite: u32,
    pub radius: f32,
    pub threshold: f32,
    pub intensity: f32,
    pub texel: [f32; 2],
    pub reserved: f32,
    pub secondary: u64,
    pub params: [f32; 6],
}
impl ImageFilter {
    pub fn is_valid(self) -> bool {
        self.overwrite <= 1
            && self.radius.is_finite()
            && (0.0..=64.0).contains(&self.radius)
            && self.threshold.is_finite()
            && (0.0..=1.0).contains(&self.threshold)
            && self.intensity.is_finite()
            && (0.0..=8.0).contains(&self.intensity)
            && self
                .texel
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            && self.reserved == 0.
            && self.params == [0.; 6]
    }
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlurMethod {
    Separable = 0,
    DualKawase = 1,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blur {
    pub method: BlurMethod,
    pub radius: f32,
    pub levels: u32,
}
impl Default for Blur {
    fn default() -> Self {
        Self {
            method: BlurMethod::DualKawase,
            radius: 4.,
            levels: 3,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FilterStep {
    pub source: usize,
    pub size: [u32; 2],
    pub filter: ImageFilter,
}
pub struct FilterPlan {
    steps: [FilterStep; MAX_FILTER_PASSES],
    len: usize,
    input_size: [u32; 2],
}
impl FilterPlan {
    pub fn blur(size: [u32; 2], blur: Blur) -> Result<Self, EffectError> {
        Self::build(size, blur, None)
    }
    pub fn bloom(
        size: [u32; 2],
        blur: Blur,
        threshold: f32,
        intensity: f32,
    ) -> Result<Self, EffectError> {
        if !threshold.is_finite()
            || !(0.0..=1.0).contains(&threshold)
            || !intensity.is_finite()
            || !(0.0..=8.0).contains(&intensity)
        {
            return Err(EffectError::InvalidInput);
        }
        Self::build(size, blur, Some((threshold, intensity)))
    }
    fn build(size: [u32; 2], blur: Blur, bloom: Option<(f32, f32)>) -> Result<Self, EffectError> {
        if size.iter().any(|v| *v == 0 || *v > 16384)
            || !blur.radius.is_finite()
            || !(0.0..=64.0).contains(&blur.radius)
            || !(1..=6).contains(&blur.levels)
        {
            return Err(EffectError::InvalidInput);
        }
        let mut p = Self {
            steps: [FilterStep::default(); MAX_FILTER_PASSES],
            len: 0,
            input_size: size,
        };
        if let Some((threshold, _)) = bloom {
            p.push(size, FilterKind::Threshold, 0., threshold, 0.);
        }
        if blur.radius == 0. {
            p.push(size, FilterKind::Copy, 0., 0., 0.);
        } else if blur.method == BlurMethod::Separable {
            p.push(size, FilterKind::GaussianX, blur.radius, 0., 0.);
            p.push(size, FilterKind::GaussianY, blur.radius, 0., 0.);
        } else {
            let mut levels = [[0; 2]; 7];
            levels[0] = size;
            let mut n = 0;
            for i in 1..=blur.levels as usize {
                let next = [levels[i - 1][0].div_ceil(2), levels[i - 1][1].div_ceil(2)];
                if next == levels[i - 1] {
                    break;
                }
                levels[i] = next;
                n = i;
                p.push(next, FilterKind::KawaseDown, blur.radius, 0., 0.);
            }
            for i in (0..n).rev() {
                p.push(levels[i], FilterKind::KawaseUp, blur.radius, 0., 0.);
            }
            if n == 0 {
                p.push(size, FilterKind::Copy, 0., 0., 0.);
            }
        }
        if let Some((_, intensity)) = bloom {
            p.push(size, FilterKind::BloomCompose, 0., 0., intensity);
        }
        Ok(p)
    }
    fn push(
        &mut self,
        size: [u32; 2],
        kind: FilterKind,
        radius: f32,
        threshold: f32,
        intensity: f32,
    ) {
        let source = self.len;
        let source_size = if source == 0 {
            self.input_size
        } else {
            self.steps[source - 1].size
        };
        self.steps[self.len] = FilterStep {
            source,
            size,
            filter: ImageFilter {
                kind,
                overwrite: 1,
                radius,
                threshold,
                intensity,
                texel: [1. / source_size[0] as f32, 1. / source_size[1] as f32],
                ..Default::default()
            },
        };
        self.len += 1;
    }
    pub fn input_size(&self) -> [u32; 2] {
        self.input_size
    }
    pub fn steps(&self) -> &[FilterStep] {
        &self.steps[..self.len]
    }
    pub fn packet(
        &self,
        index: usize,
        input: TextureId,
        targets: &[TextureId],
    ) -> Result<FilterPacket, EffectError> {
        let step = *self.steps().get(index).ok_or(EffectError::InvalidInput)?;
        let source = if step.source == 0 {
            input
        } else {
            *targets
                .get(step.source - 1)
                .ok_or(EffectError::InvalidInput)?
        };
        let mut filter = step.filter;
        if filter.kind == FilterKind::BloomCompose {
            filter.secondary = input.0;
        }
        FilterPacket::new(step.size, source, filter)
    }
}
pub struct FilterPacket {
    vertices: [DrawVert; 4],
    indices: [u32; 6],
    commands: [DrawCmd; 1],
    effects: [DrawEffects; 1],
}
impl FilterPacket {
    pub fn new(
        size: [u32; 2],
        source: TextureId,
        filter: ImageFilter,
    ) -> Result<Self, EffectError> {
        if size.iter().any(|v| *v == 0 || *v > 16384) || !filter.is_valid() {
            return Err(EffectError::InvalidInput);
        }
        let [w, h] = size.map(|v| v as f32);
        Ok(Self {
            vertices: [
                DrawVert {
                    pos: [0., 0.],
                    uv: [0., 0.],
                    col: [255; 4],
                },
                DrawVert {
                    pos: [w, 0.],
                    uv: [1., 0.],
                    col: [255; 4],
                },
                DrawVert {
                    pos: [w, h],
                    uv: [1., 1.],
                    col: [255; 4],
                },
                DrawVert {
                    pos: [0., h],
                    uv: [0., 1.],
                    col: [255; 4],
                },
            ],
            indices: [0, 1, 2, 0, 2, 3],
            commands: [DrawCmd {
                elem_count: 6,
                clip_rect: [0., 0., w, h],
                texture: source,
                effects: 1,
            }],
            effects: [DrawEffects {
                filter,
                ..Default::default()
            }],
        })
    }
    pub fn draw_data(&self) -> DrawData<'_> {
        DrawData {
            vertices: &self.vertices,
            indices: &self.indices,
            commands: &self.commands,
            effects: &self.effects,
        }
    }
}
impl Ui<'_> {
    pub fn draw_filtered_image(
        &mut self,
        texture: TextureId,
        rect: [f32; 4],
        uv: [f32; 4],
        color: [u8; 4],
        filter: ImageFilter,
    ) -> Result<(), EffectError> {
        if !filter.is_valid()
            || crate::shapes::RoundedRect::new(rect, [0.; 4]).is_err()
            || !uv.iter().all(|v| v.is_finite() && v.abs() <= 4096.)
        {
            return Err(EffectError::InvalidInput);
        }
        if self.ctx.current_window.is_none() {
            return Ok(());
        }
        let draw = crate::widgets::draw_parts(self.ctx).1;
        let previous = draw.effects_state();
        if filter.kind == FilterKind::BloomCompose && previous.masks.count == 4 {
            return Err(EffectError::ScopeLimit);
        }
        draw.set_effects(DrawEffects { filter, ..previous });
        draw.add_textured_quad(
            texture,
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            [uv[0], uv[1]],
            [uv[2], uv[3]],
            color,
        );
        draw.set_effects(previous);
        Ok(())
    }
}
