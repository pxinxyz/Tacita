use super::{DrawEffects, EFFECT_UNIFORM_SIZE};

impl DrawEffects {
    pub fn uniform_bytes_with_alpha(self, flags: u32) -> [u8; EFFECT_UNIFORM_SIZE] {
        let mut bytes = self.uniform_bytes();
        bytes[52..56].copy_from_slice(&(flags as f32).to_le_bytes());
        bytes
    }
    pub fn uniform_bytes(self) -> [u8; EFFECT_UNIFORM_SIZE] {
        let mut values = [0.0_f32; EFFECT_UNIFORM_SIZE / 4];
        values[..4].copy_from_slice(&self.gradient.rect);
        values[4..6].copy_from_slice(&self.gradient.origin);
        values[6..8].copy_from_slice(&self.gradient.vector);
        values[8..12].copy_from_slice(&[
            self.gradient.kind as f32,
            self.gradient.spread as f32,
            self.gradient.phase,
            self.gradient.stop_count as f32,
        ]);
        values[12] = self.masks.count as f32;
        values[14] = self.stencil[1] as f32;
        values[15] = self.stencil[0] as f32;
        for (i, stop) in self.gradient.stops.iter().enumerate() {
            let base = 16 + i * 8;
            values[base] = stop.position;
            for (j, c) in stop.color.iter().enumerate() {
                values[base + 4 + j] = *c as f32 / 255.0;
            }
        }
        for (i, op) in self.masks.ops.iter().enumerate() {
            let base = 144 + i * 12;
            values[base..base + 4].copy_from_slice(&op.rect);
            values[base + 4..base + 8].copy_from_slice(&op.params);
            values[base + 8] = op.kind as f32;
        }
        values[192..196].copy_from_slice(&self.sdf.rect);
        values[196..200].copy_from_slice(&self.sdf.radii);
        values[200..204].copy_from_slice(&self.sdf.parameters);
        values[204..208].copy_from_slice(&self.sdf.light);
        let s = self.surface;
        if s.flags != 0 {
            values[208..212].copy_from_slice(&s.rect);
            values[212..216].copy_from_slice(&[
                s.flags as f32,
                s.phase,
                s.pixel_scale[0],
                s.pixel_scale[1],
            ]);
            values[216..220].copy_from_slice(&[
                s.noise,
                s.grain,
                s.dither,
                s.dither_kind as u32 as f32,
            ]);
            values[220..224].copy_from_slice(&[
                s.exposure,
                s.contrast,
                s.saturation,
                s.desaturation,
            ]);
            values[224..228].copy_from_slice(&s.tint);
            values[228..232].copy_from_slice(&[s.scanlines, s.spacing, s.grid_width, s.dot_radius]);
            values[232..236].copy_from_slice(&[
                s.vignette,
                s.shimmer,
                s.sweep_width,
                s.sweep_speed,
            ]);
            values[236..240].copy_from_slice(&[
                s.distortion,
                s.frequency,
                s.chromatic,
                s.refraction,
            ]);
        }
        let f = self.filter;
        if f != super::ImageFilter::default() {
            values[240..244].copy_from_slice(&[
                f.kind as u32 as f32,
                f.overwrite as f32,
                f.radius,
                f.threshold,
            ]);
            values[244..248].copy_from_slice(&[f.intensity, f.texel[0], f.texel[1], f.reserved]);
            values[248..254].copy_from_slice(&f.params);
        }
        let mut bytes = [0; EFFECT_UNIFORM_SIZE];
        for (slot, value) in bytes.chunks_exact_mut(4).zip(values) {
            slot.copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}
