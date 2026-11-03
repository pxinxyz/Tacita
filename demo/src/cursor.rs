use std::f32::consts::{PI, TAU};

use tacita::draw::DrawVert;

const ARC_POINTS: usize = 15;
const DISC_POINTS: usize = 12;
const SPARK_POINTS: usize = 8;
const MAX_PARTICLES: usize = 24;
const DARK: [u8; 4] = [14, 16, 29, 255];
const CYAN: [u8; 3] = [125, 207, 255];
const BLUE: [u8; 3] = [122, 162, 247];

struct Particle {
    pos: [f32; 2],
    velocity: [f32; 2],
    age: f32,
    life: f32,
    radius: f32,
    rotation: f32,
    color: [u8; 3],
}

pub struct OrbitalCursor {
    pub vertices: Vec<DrawVert>,
    pub indices: Vec<u32>,
    particles: Vec<Particle>,
    last_pos: Option<[f32; 2]>,
    velocity: [f32; 2],
    heading: f32,
    orbit: f32,
    travel_remainder: f32,
    was_pressed: bool,
    click_age: Option<f32>,
    seed: u32,
}

impl OrbitalCursor {
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(1800),
            indices: Vec::with_capacity(6500),
            particles: Vec::with_capacity(MAX_PARTICLES),
            last_pos: None,
            velocity: [0.0; 2],
            heading: 0.0,
            orbit: 0.0,
            travel_remainder: 0.0,
            was_pressed: false,
            click_age: None,
            seed: 0,
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.particles.clear();
        self.last_pos = None;
        self.velocity = [0.0; 2];
        self.travel_remainder = 0.0;
        self.was_pressed = false;
        self.click_age = None;
    }

    pub fn rebuild(
        &mut self,
        center: [f32; 2],
        dt: f32,
        time: f32,
        pressed: bool,
        accent: [u8; 4],
    ) {
        self.vertices.clear();
        self.indices.clear();

        let dt = dt.clamp(0.0001, 0.05);
        let previous = self.last_pos.unwrap_or(center);
        let delta = [center[0] - previous[0], center[1] - previous[1]];
        let distance = delta[0].hypot(delta[1]);
        let instant = [delta[0] / dt, delta[1] / dt];
        let keep = (-14.0 * dt).exp();
        self.velocity = [
            self.velocity[0] * keep + instant[0] * (1.0 - keep),
            self.velocity[1] * keep + instant[1] * (1.0 - keep),
        ];
        let speed = self.velocity[0].hypot(self.velocity[1]);
        let motion = (speed / 850.0).clamp(0.0, 1.0);
        if speed > 8.0 {
            let target = self.velocity[1].atan2(self.velocity[0]);
            let angle_error = (target - self.heading + PI).rem_euclid(TAU) - PI;
            self.heading += angle_error * (1.0 - (-9.0 * dt).exp());
        } else {
            self.heading += dt * 0.24;
        }
        self.orbit += dt * (0.85 + 0.7 * motion);

        self.particles.retain_mut(|particle| {
            particle.age += dt;
            particle.pos[0] += particle.velocity[0] * dt;
            particle.pos[1] += particle.velocity[1] * dt;
            particle.velocity[0] *= (-3.0 * dt).exp();
            particle.velocity[1] *= (-3.0 * dt).exp();
            particle.age < particle.life
        });

        if distance > 1.0 {
            let count = ((self.travel_remainder + distance) / 10.0) as usize;
            self.travel_remainder = (self.travel_remainder + distance) % 10.0;
            let direction = [delta[0] / distance, delta[1] / distance];
            for i in 0..count.min(3) {
                let t = (i + 1) as f32 / (count.min(3) + 1) as f32;
                let origin = [previous[0] + delta[0] * t, previous[1] + delta[1] * t];
                self.spawn_trail(origin, direction, accent);
            }
        }
        if pressed && !self.was_pressed {
            self.click_age = Some(0.0);
            self.spawn_click(center, accent);
        }
        self.was_pressed = pressed;
        self.last_pos = Some(center);
        if let Some(age) = &mut self.click_age {
            *age += dt;
            if *age > 0.48 {
                self.click_age = None;
            }
        }

        for i in 0..self.particles.len() {
            let particle = &self.particles[i];
            let fade = (1.0 - particle.age / particle.life).clamp(0.0, 1.0);
            let (pos, radius, angle, color) = (
                particle.pos,
                particle.radius * (0.5 + 0.5 * fade),
                particle.rotation + particle.age * 2.0,
                [
                    particle.color[0],
                    particle.color[1],
                    particle.color[2],
                    (210.0 * fade) as u8,
                ],
            );
            self.push_spark(pos, radius, angle, color);
        }

        let direction = [self.heading.cos(), self.heading.sin()];
        let orbit_center = [
            center[0] - direction[0] * 2.0 * motion,
            center[1] - direction[1] * 2.0 * motion,
        ];
        let radius = 13.0 + 1.0 * motion + click_offset(self.click_age);
        let accent_rgb = [accent[0], accent[1], accent[2]];
        for i in 0..3 {
            let start = self.orbit + i as f32 * TAU / 3.0 + 0.06 * (time * 1.7 + i as f32).sin();
            let end = start + 0.88 + 0.12 * motion;
            let color = match i {
                0 => accent_rgb,
                1 => BLUE,
                _ => CYAN,
            };
            self.push_arc(orbit_center, radius, start, end, 4.2, DARK);
            self.push_arc(
                orbit_center,
                radius,
                start,
                end,
                2.35,
                [color[0], color[1], color[2], 255],
            );
        }

        let core_radius = 5.5 + 0.3 * motion - 0.15 * click_offset(self.click_age);
        self.push_diamond(center, core_radius, self.heading, DARK, 1.0);
        self.push_diamond(
            center,
            core_radius - 1.6,
            self.heading,
            [accent[0], accent[1], accent[2], 255],
            0.7,
        );
    }

    fn spawn_trail(&mut self, origin: [f32; 2], direction: [f32; 2], accent: [u8; 4]) {
        if self.particles.len() >= MAX_PARTICLES {
            return;
        }
        let jitter = self.next_unit() * 2.0 - 1.0;
        let normal = [-direction[1], direction[0]];
        let speed = 15.0 + self.next_unit() * 25.0;
        let color = if self.seed.is_multiple_of(3) {
            [accent[0], accent[1], accent[2]]
        } else if self.seed.is_multiple_of(2) {
            BLUE
        } else {
            CYAN
        };
        let life = 0.24 + self.next_unit() * 0.18;
        let radius = 1.5 + self.next_unit() * 1.4;
        let rotation = self.next_unit() * TAU;
        self.particles.push(Particle {
            pos: [
                origin[0] + normal[0] * jitter * 3.0,
                origin[1] + normal[1] * jitter * 3.0,
            ],
            velocity: [
                -direction[0] * speed + normal[0] * jitter * 17.0,
                -direction[1] * speed + normal[1] * jitter * 17.0,
            ],
            age: 0.0,
            life,
            radius,
            rotation,
            color,
        });
    }

    fn spawn_click(&mut self, center: [f32; 2], accent: [u8; 4]) {
        for i in 0..8 {
            if self.particles.len() >= MAX_PARTICLES {
                self.particles.remove(0);
            }
            let angle = i as f32 * TAU / 8.0 + self.orbit;
            let speed = 45.0 + self.next_unit() * 40.0;
            let color = match i % 3 {
                0 => [accent[0], accent[1], accent[2]],
                1 => CYAN,
                _ => BLUE,
            };
            let life = 0.32 + self.next_unit() * 0.18;
            let radius = 2.0 + self.next_unit() * 1.2;
            self.particles.push(Particle {
                pos: center,
                velocity: [angle.cos() * speed, angle.sin() * speed],
                age: 0.0,
                life,
                radius,
                rotation: angle,
                color,
            });
        }
    }

    fn next_unit(&mut self) -> f32 {
        self.seed = self.seed.wrapping_add(0x9e3779b9);
        let mut x = self.seed;
        x ^= x >> 16;
        x = x.wrapping_mul(0x7feb352d);
        x ^= x >> 15;
        x = x.wrapping_mul(0x846ca68b);
        x ^= x >> 16;
        x as f32 / u32::MAX as f32
    }

    fn push_arc(
        &mut self,
        center: [f32; 2],
        radius: f32,
        start: f32,
        end: f32,
        width: f32,
        col: [u8; 4],
    ) {
        let mut inner_fade = [[0.0; 2]; ARC_POINTS];
        let mut inner = [[0.0; 2]; ARC_POINTS];
        let mut outer = [[0.0; 2]; ARC_POINTS];
        let mut outer_fade = [[0.0; 2]; ARC_POINTS];
        for i in 0..ARC_POINTS {
            let angle = start + (end - start) * i as f32 / (ARC_POINTS - 1) as f32;
            let (sin, cos) = angle.sin_cos();
            let point = |r: f32| [center[0] + cos * r, center[1] + sin * r];
            inner_fade[i] = point(radius - width * 0.5 - 0.8);
            inner[i] = point(radius - width * 0.5);
            outer[i] = point(radius + width * 0.5);
            outer_fade[i] = point(radius + width * 0.5 + 0.8);
        }
        let clear = [col[0], col[1], col[2], 0];
        self.push_arc_band(&inner_fade, &inner, clear, col);
        self.push_arc_band(&inner, &outer, col, col);
        self.push_arc_band(&outer, &outer_fade, col, clear);
        let endpoint = |angle: f32| {
            [
                center[0] + radius * angle.cos(),
                center[1] + radius * angle.sin(),
            ]
        };
        self.push_disc(endpoint(start), width * 0.5, col, 0.8);
        self.push_disc(endpoint(end), width * 0.5, col, 0.8);
    }

    fn push_arc_band(
        &mut self,
        inner: &[[f32; 2]; ARC_POINTS],
        outer: &[[f32; 2]; ARC_POINTS],
        inner_col: [u8; 4],
        outer_col: [u8; 4],
    ) {
        let base = self.vertices.len() as u32;
        for i in 0..ARC_POINTS {
            self.vertices.push(vertex(inner[i], inner_col));
            self.vertices.push(vertex(outer[i], outer_col));
        }
        for i in 0..ARC_POINTS - 1 {
            let a = base + 2 * i as u32;
            let b = a + 2;
            self.indices
                .extend_from_slice(&[a, a + 1, b, b, a + 1, b + 1]);
        }
    }

    fn push_disc(&mut self, center: [f32; 2], radius: f32, col: [u8; 4], fringe: f32) {
        let base = self.vertices.len() as u32;
        self.vertices.push(vertex(center, col));
        for i in 0..DISC_POINTS {
            let angle = i as f32 * TAU / DISC_POINTS as f32;
            let (sin, cos) = angle.sin_cos();
            self.vertices.push(vertex(
                [center[0] + cos * radius, center[1] + sin * radius],
                col,
            ));
            self.vertices.push(vertex(
                [
                    center[0] + cos * (radius + fringe),
                    center[1] + sin * (radius + fringe),
                ],
                [col[0], col[1], col[2], 0],
            ));
        }
        for i in 0..DISC_POINTS {
            let a = base + 1 + 2 * i as u32;
            let b = base + 1 + 2 * ((i + 1) % DISC_POINTS) as u32;
            self.indices
                .extend_from_slice(&[base, a, b, a, a + 1, b, b, a + 1, b + 1]);
        }
    }

    fn push_diamond(
        &mut self,
        center: [f32; 2],
        radius: f32,
        angle: f32,
        col: [u8; 4],
        fringe: f32,
    ) {
        let base = self.vertices.len() as u32;
        self.vertices.push(vertex(center, col));
        for i in 0..4 {
            let a = angle + i as f32 * PI * 0.5;
            let (sin, cos) = a.sin_cos();
            self.vertices.push(vertex(
                [center[0] + cos * radius, center[1] + sin * radius],
                col,
            ));
            self.vertices.push(vertex(
                [
                    center[0] + cos * (radius + fringe),
                    center[1] + sin * (radius + fringe),
                ],
                [col[0], col[1], col[2], 0],
            ));
        }
        for i in 0..4 {
            let a = base + 1 + 2 * i as u32;
            let b = base + 1 + 2 * ((i + 1) % 4) as u32;
            self.indices
                .extend_from_slice(&[base, a, b, a, a + 1, b, b, a + 1, b + 1]);
        }
    }

    fn push_spark(&mut self, center: [f32; 2], radius: f32, rotation: f32, col: [u8; 4]) {
        if col[3] == 0 {
            return;
        }
        let base = self.vertices.len() as u32;
        self.vertices.push(vertex(center, col));
        for i in 0..SPARK_POINTS {
            let a = rotation + i as f32 * TAU / SPARK_POINTS as f32;
            let (sin, cos) = a.sin_cos();
            let r = if i % 2 == 0 { radius } else { radius * 0.28 };
            self.vertices
                .push(vertex([center[0] + cos * r, center[1] + sin * r], col));
            self.vertices.push(vertex(
                [center[0] + cos * (r + 0.7), center[1] + sin * (r + 0.7)],
                [col[0], col[1], col[2], 0],
            ));
        }
        for i in 0..SPARK_POINTS {
            let a = base + 1 + 2 * i as u32;
            let b = base + 1 + 2 * ((i + 1) % SPARK_POINTS) as u32;
            self.indices
                .extend_from_slice(&[base, a, b, a, a + 1, b, b, a + 1, b + 1]);
        }
    }
}

fn click_offset(age: Option<f32>) -> f32 {
    let Some(age) = age else { return 0.0 };
    if age < 0.09 {
        -3.0 * age / 0.09
    } else if age < 0.25 {
        -3.0 + 4.5 * (age - 0.09) / 0.16
    } else {
        1.5 * (1.0 - (age - 0.25) / 0.23).max(0.0)
    }
}

fn vertex(pos: [f32; 2], col: [u8; 4]) -> DrawVert {
    DrawVert {
        pos,
        uv: [0.0, 0.0],
        col,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_and_clicking_keep_cursor_geometry_finite_and_bounded() {
        let mut cursor = OrbitalCursor::new();
        let vertex_capacity = cursor.vertices.capacity();
        let index_capacity = cursor.indices.capacity();
        for frame in 0..180 {
            let t = frame as f32 / 60.0;
            let pos = [180.0 + frame as f32 * 2.0, 130.0 + (t * 5.0).sin() * 20.0];
            cursor.rebuild(pos, 1.0 / 60.0, t, frame == 40, [187, 154, 247, 255]);
            assert!(!cursor.vertices.is_empty());
            assert!(cursor
                .indices
                .iter()
                .all(|&i| i < cursor.vertices.len() as u32));
            assert!(cursor
                .vertices
                .iter()
                .all(|v| v.pos.iter().all(|p| p.is_finite())));
            assert!(cursor.particles.len() <= MAX_PARTICLES);
            assert_eq!(cursor.vertices.capacity(), vertex_capacity);
            assert_eq!(cursor.indices.capacity(), index_capacity);
        }
        cursor.clear();
        assert!(cursor.vertices.is_empty());
        assert!(cursor.particles.is_empty());
    }
}
