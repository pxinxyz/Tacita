use super::{DrawEffects, EffectError, SdfShape};
use crate::{draw::TextureId, shapes::RoundedRect, ui::Ui};

pub const SURFACE_NOISE: u32 = 1;
pub const SURFACE_GRAIN: u32 = 2;
pub const SURFACE_DITHER: u32 = 4;
pub const SURFACE_GRADE: u32 = 8;
pub const SURFACE_DESATURATE: u32 = 16;
pub const SURFACE_VIGNETTE: u32 = 32;
pub const SURFACE_CHROMATIC: u32 = 64;
pub const SURFACE_SCANLINES: u32 = 128;
pub const SURFACE_GRID: u32 = 256;
pub const SURFACE_DOTS: u32 = 512;
pub const SURFACE_SHIMMER: u32 = 1024;
pub const SURFACE_SWEEP: u32 = 2048;
pub const SURFACE_DISTORT: u32 = 4096;
pub const SURFACE_GLASS: u32 = 8192;
pub const SURFACE_ALL: u32 = 16383;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DitherKind {
    #[default]
    Bayer = 0,
    BlueNoise = 1,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceStyle {
    pub flags: u32,
    pub phase: f32,
    pub pixel_scale: [f32; 2],
    pub noise: f32,
    pub grain: f32,
    pub dither: f32,
    pub dither_kind: DitherKind,
    pub exposure: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub desaturation: f32,
    pub tint: [f32; 4],
    pub scanlines: f32,
    pub spacing: f32,
    pub grid_width: f32,
    pub dot_radius: f32,
    pub vignette: f32,
    pub shimmer: f32,
    pub sweep_width: f32,
    pub sweep_speed: f32,
    pub distortion: f32,
    pub frequency: f32,
    pub chromatic: f32,
    pub refraction: f32,
    pub rect: [f32; 4],
}
impl Default for SurfaceStyle {
    fn default() -> Self {
        Self {
            flags: 0,
            phase: 0.,
            pixel_scale: [1.; 2],
            noise: 0.12,
            grain: 0.06,
            dither: 1. / 255.,
            dither_kind: DitherKind::Bayer,
            exposure: 0.,
            contrast: 1.,
            saturation: 1.,
            desaturation: 1.,
            tint: [1.; 4],
            scanlines: 0.25,
            spacing: 24.,
            grid_width: 1.,
            dot_radius: 1.5,
            vignette: 0.65,
            shimmer: 0.3,
            sweep_width: 0.2,
            sweep_speed: 0.5,
            distortion: 4.,
            frequency: 4.,
            chromatic: 2.,
            refraction: 3.,
            rect: [0.; 4],
        }
    }
}
impl SurfaceStyle {
    pub fn is_valid(self) -> bool {
        fn range(v: f32, low: f32, high: f32) -> bool {
            v.is_finite() && (low..=high).contains(&v)
        }
        self.flags & !SURFACE_ALL == 0
            && range(self.phase, -1_000_000., 1_000_000.)
            && self.pixel_scale.iter().all(|v| range(*v, 0.01, 64.))
            && [
                self.noise,
                self.grain,
                self.dither,
                self.scanlines,
                self.vignette,
                self.desaturation,
            ]
            .iter()
            .all(|v| range(*v, 0., 1.))
            && range(self.exposure, -8., 8.)
            && [self.contrast, self.saturation]
                .iter()
                .all(|v| range(*v, 0., 4.))
            && self.tint[..3].iter().all(|v| range(*v, 0., 4.))
            && range(self.tint[3], 0., 1.)
            && range(self.spacing, 1., 1024.)
            && [self.grid_width, self.dot_radius]
                .iter()
                .all(|v| range(*v, 0., 32.))
            && range(self.shimmer, 0., 2.)
            && range(self.sweep_width, 0.001, 1.)
            && range(self.sweep_speed, -8., 8.)
            && [self.distortion, self.chromatic, self.refraction]
                .iter()
                .all(|v| range(*v, 0., 64.))
            && range(self.frequency, 0.01, 64.)
            && self.rect.iter().all(|v| range(*v, -1_000_000., 1_000_000.))
    }
}
impl Ui<'_> {
    pub fn draw_surface(
        &mut self,
        shape: RoundedRect,
        texture: Option<TextureId>,
        uv: [f32; 4],
        color: [u8; 4],
        mut style: SurfaceStyle,
    ) -> Result<(), EffectError> {
        let shape = shape.canonical().map_err(|_| EffectError::InvalidInput)?;
        if !style.is_valid() || !uv.iter().all(|v| v.is_finite() && v.abs() <= 4096.) {
            return Err(EffectError::InvalidInput);
        }
        if self.ctx.current_window.is_none() {
            return Ok(());
        }
        style.rect = shape.rect;
        let aa = if self.ctx.style.anti_aliasing { 1. } else { 0. };
        let draw = crate::widgets::draw_parts(self.ctx).1;
        let previous = draw.effects_state();
        draw.set_effects(DrawEffects {
            surface: style,
            sdf: SdfShape {
                rect: shape.rect,
                radii: shape.radii,
                parameters: [1., 0., 0., 0.],
                light: [0., 0., 0., aa],
            },
            ..previous
        });
        let (texture, uv) = texture.map_or(
            (
                TextureId::WHITE,
                [
                    crate::draw::WHITE_PIXEL_UV[0],
                    crate::draw::WHITE_PIXEL_UV[1],
                    crate::draw::WHITE_PIXEL_UV[0],
                    crate::draw::WHITE_PIXEL_UV[1],
                ],
            ),
            |id| (id, uv),
        );
        draw.add_textured_quad(
            texture,
            [shape.rect[0], shape.rect[1]],
            [shape.rect[2], shape.rect[3]],
            [uv[0], uv[1]],
            [uv[2], uv[3]],
            color,
        );
        draw.set_effects(previous);
        Ok(())
    }
}
