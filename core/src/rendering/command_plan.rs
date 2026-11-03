use super::DrawOptimization;
use crate::draw::{DrawCmd, DrawData, DrawVert};
use alloc::vec::Vec;

const MAX_COMMANDS: usize = 65_536;
const MAX_INDICES: usize = 16_777_216;
const LOOKAHEAD: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadInstance {
    pub vertices: [DrawVert; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryRange {
    Triangles {
        first_index: u32,
        index_count: u32,
    },
    Quads {
        first_instance: u32,
        instance_count: u32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedDraw {
    pub command: DrawCmd,
    pub geometry: GeometryRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    TooLarge,
    InvalidGeometry,
    InvalidCommands,
}

#[derive(Clone)]
struct Run {
    command: DrawCmd,
    first: u32,
    bounds: [f32; 4],
    barrier: bool,
}

#[derive(Default)]
pub struct CommandPlan {
    runs: Vec<Run>,
    draws: Vec<PlannedDraw>,
    instances: Vec<QuadInstance>,
    moved: u32,
}

impl CommandPlan {
    pub fn draws(&self) -> &[PlannedDraw] {
        &self.draws
    }
    pub fn instances(&self) -> &[QuadInstance] {
        &self.instances
    }
    pub fn reordered_commands(&self) -> u32 {
        self.moved
    }

    pub fn prepare(
        &mut self,
        data: &DrawData<'_>,
        options: DrawOptimization,
        scale: f32,
    ) -> Result<(), PlanError> {
        if data.commands.len() > MAX_COMMANDS || data.indices.len() > MAX_INDICES {
            return Err(PlanError::TooLarge);
        }
        if !scale.is_finite() || !(0.0..=8.0).contains(&scale) || scale == 0. {
            return Err(PlanError::InvalidGeometry);
        }
        let mut consumed = 0usize;
        for cmd in data.commands {
            consumed = consumed
                .checked_add(cmd.elem_count as usize)
                .ok_or(PlanError::TooLarge)?;
            if consumed > data.indices.len()
                || !cmd.elem_count.is_multiple_of(3)
                || cmd.clip_rect.iter().any(|v| !v.is_finite())
            {
                return Err(PlanError::InvalidCommands);
            }
        }
        if consumed != data.indices.len() {
            return Err(PlanError::InvalidCommands);
        }
        for &index in data.indices {
            let v = data
                .vertices
                .get(index as usize)
                .ok_or(PlanError::InvalidGeometry)?;
            if v.pos.iter().chain(v.uv.iter()).any(|v| !v.is_finite()) {
                return Err(PlanError::InvalidGeometry);
            }
        }
        let mut instance_count = 0;
        if options.instance_quads {
            let mut first = 0usize;
            for cmd in data.commands {
                let indices = &data.indices[first..first + cmd.elem_count as usize];
                let barrier = cmd.effects != 0
                    && data
                        .effects
                        .get(cmd.effects as usize - 1)
                        .is_none_or(|e| e.stencil != [0; 2] || e.filter != Default::default());
                if !barrier
                    && indices.len().is_multiple_of(6)
                    && indices.chunks_exact(6).all(is_quad)
                {
                    instance_count += indices.len() / 6;
                }
                first += cmd.elem_count as usize;
            }
            if instance_count > 64 * 1024 * 1024 / core::mem::size_of::<QuadInstance>() {
                return Err(PlanError::TooLarge);
            }
        }
        self.runs.clear();
        self.draws.clear();
        self.instances.clear();
        self.instances.reserve_exact(instance_count);
        self.moved = 0;
        let mut first = 0u32;
        for cmd in data.commands {
            let mut bounds = [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ];
            for &index in &data.indices[first as usize..(first + cmd.elem_count) as usize] {
                let p = data.vertices[index as usize].pos;
                bounds[0] = bounds[0].min(p[0]);
                bounds[1] = bounds[1].min(p[1]);
                bounds[2] = bounds[2].max(p[0]);
                bounds[3] = bounds[3].max(p[1]);
            }
            let margin = 0.5 / scale;
            bounds = [
                (bounds[0] - margin).max(floor_pixel(cmd.clip_rect[0] * scale) / scale),
                (bounds[1] - margin).max(floor_pixel(cmd.clip_rect[1] * scale) / scale),
                (bounds[2] + margin).min(-floor_pixel(-cmd.clip_rect[2] * scale) / scale),
                (bounds[3] + margin).min(-floor_pixel(-cmd.clip_rect[3] * scale) / scale),
            ];
            let barrier = cmd.effects != 0
                && data
                    .effects
                    .get(cmd.effects as usize - 1)
                    .is_none_or(|e| e.stencil != [0; 2] || e.filter != Default::default());
            self.runs.push(Run {
                command: cmd.clone(),
                first,
                bounds,
                barrier,
            });
            first += cmd.elem_count;
        }
        if options.sort_disjoint {
            for i in 1..self.runs.len() {
                if self.runs[i - 1].barrier {
                    continue;
                }
                let end = (i + LOOKAHEAD).min(self.runs.len());
                for j in i..end {
                    if self.runs[j].barrier {
                        break;
                    }
                    if same_state(&self.runs[i - 1].command, &self.runs[j].command)
                        && self.runs[i..j]
                            .iter()
                            .all(|r| !r.barrier && disjoint(r.bounds, self.runs[j].bounds))
                    {
                        if j != i {
                            self.runs[i..=j].rotate_right(1);
                            self.moved += 1;
                        }
                        break;
                    }
                }
            }
        }
        for run in &self.runs {
            let indices =
                &data.indices[run.first as usize..(run.first + run.command.elem_count) as usize];
            let quads = options.instance_quads
                && !run.barrier
                && !indices.is_empty()
                && indices.len().is_multiple_of(6)
                && indices.chunks_exact(6).all(is_quad);
            let geometry = if quads {
                let first_instance = self.instances.len() as u32;
                for q in indices.chunks_exact(6) {
                    self.instances.push(QuadInstance {
                        vertices: [0, 1, 2, 5].map(|i| data.vertices[q[i] as usize]),
                    });
                }
                GeometryRange::Quads {
                    first_instance,
                    instance_count: (indices.len() / 6) as u32,
                }
            } else {
                GeometryRange::Triangles {
                    first_index: run.first,
                    index_count: run.command.elem_count,
                }
            };
            if let (
                Some(previous),
                GeometryRange::Quads {
                    first_instance,
                    instance_count,
                },
            ) = (self.draws.last_mut(), geometry)
            {
                if same_state(&previous.command, &run.command) {
                    if let GeometryRange::Quads {
                        first_instance: start,
                        instance_count: count,
                    } = &mut previous.geometry
                    {
                        if *start + *count == first_instance {
                            *count += instance_count;
                            previous.command.elem_count += run.command.elem_count;
                            continue;
                        }
                    }
                }
            }
            self.draws.push(PlannedDraw {
                command: run.command.clone(),
                geometry,
            });
        }
        Ok(())
    }
}

fn floor_pixel(value: f32) -> f32 {
    if value.abs() >= 8_388_608. {
        return value;
    }
    let truncated = value as i32 as f32;
    if truncated > value {
        truncated - 1.
    } else {
        truncated
    }
}
fn is_quad(q: &[u32]) -> bool {
    q[0] == q[3]
        && q[2] == q[4]
        && q[0] != q[1]
        && q[0] != q[2]
        && q[0] != q[5]
        && q[1] != q[2]
        && q[1] != q[5]
        && q[2] != q[5]
}
fn same_state(a: &DrawCmd, b: &DrawCmd) -> bool {
    a.texture == b.texture && a.effects == b.effects && a.clip_rect == b.clip_rect
}
fn disjoint(a: [f32; 4], b: [f32; 4]) -> bool {
    a[2] <= b[0] || b[2] <= a[0] || a[3] <= b[1] || b[3] <= a[1]
}

pub struct DrawCalls<'a> {
    original: Option<core::slice::Iter<'a, DrawCmd>>,
    planned: core::slice::Iter<'a, PlannedDraw>,
    offset: u32,
}
impl<'a> DrawCalls<'a> {
    pub fn new(data: &'a DrawData<'_>, plan: Option<&'a CommandPlan>) -> Self {
        Self {
            original: plan.is_none().then(|| data.commands.iter()),
            planned: plan.map_or([].as_slice(), CommandPlan::draws).iter(),
            offset: 0,
        }
    }
}
impl<'a> Iterator for DrawCalls<'a> {
    type Item = (&'a DrawCmd, GeometryRange);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(original) = &mut self.original {
            let cmd = original.next()?;
            let range = GeometryRange::Triangles {
                first_index: self.offset,
                index_count: cmd.elem_count,
            };
            self.offset += cmd.elem_count;
            Some((cmd, range))
        } else {
            self.planned
                .next()
                .map(|draw| (&draw.command, draw.geometry))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::TextureId;
    fn fixture(overlap: bool) -> (Vec<DrawVert>, Vec<u32>, Vec<DrawCmd>) {
        let vertices = (0..3)
            .flat_map(|i| {
                let x = if overlap { 0. } else { i as f32 * 30. };
                [[x, 0.], [x + 10., 0.], [x + 10., 10.], [x, 10.]].map(|pos| DrawVert {
                    pos,
                    uv: [0., 0.],
                    col: [255; 4],
                })
            })
            .collect();
        let indices = (0..3)
            .flat_map(|i| [0, 1, 2, 0, 2, 3].map(|v| v + i * 4))
            .collect();
        let commands = [2, 3, 2]
            .map(|id| DrawCmd {
                effects: 0,
                elem_count: 6,
                clip_rect: [0., 0., 100., 100.],
                texture: TextureId(id),
            })
            .to_vec();
        (vertices, indices, commands)
    }
    #[test]
    fn disjoint_sort_batches_instances_but_overlap_preserves_order() {
        for overlap in [false, true] {
            let (vertices, indices, commands) = fixture(overlap);
            let data = DrawData {
                vertices: &vertices,
                indices: &indices,
                commands: &commands,
                effects: &[],
            };
            let mut plan = CommandPlan::default();
            plan.prepare(
                &data,
                DrawOptimization {
                    sort_disjoint: true,
                    instance_quads: true,
                },
                1.,
            )
            .unwrap();
            assert_eq!(plan.draws.len(), if overlap { 3 } else { 2 });
            assert_eq!(plan.moved, u32::from(!overlap));
            assert_eq!(plan.instances.len(), 3);
            assert_eq!(core::mem::size_of::<QuadInstance>(), 80);
        }
    }
    #[test]
    fn stencil_blocks_reorder_and_extraction_and_bad_input_preserves_plan() {
        let (vertices, mut indices, mut commands) = fixture(false);
        commands[1].effects = 1;
        let effect = crate::effects::DrawEffects {
            stencil: [0, 1],
            ..Default::default()
        };
        let effects = [effect];
        let mut plan = CommandPlan::default();
        let options = DrawOptimization {
            sort_disjoint: true,
            instance_quads: true,
        };
        plan.prepare(
            &DrawData {
                vertices: &vertices,
                indices: &indices,
                commands: &commands,
                effects: &effects,
            },
            options,
            1.,
        )
        .unwrap();
        assert_eq!(plan.draws.len(), 3);
        assert_eq!(plan.moved, 0);
        assert!(matches!(
            plan.draws[1].geometry,
            GeometryRange::Triangles { first_index: 6, .. }
        ));
        indices[0] = u32::MAX;
        assert_eq!(
            plan.prepare(
                &DrawData {
                    vertices: &vertices,
                    indices: &indices,
                    commands: &commands,
                    effects: &effects
                },
                options,
                1.
            ),
            Err(PlanError::InvalidGeometry)
        );
        assert_eq!(plan.draws.len(), 3);
    }
    #[test]
    fn extraction_preserves_vertex_attributes_and_mixed_triangle_offsets() {
        let (mut vertices, mut indices, mut commands) = fixture(false);
        for (i, v) in vertices.iter_mut().enumerate() {
            v.uv = [i as f32 / 12., 0.25];
            v.col = [i as u8, 127, 230, 100 + i as u8];
        }
        indices.truncate(15);
        commands[2].elem_count = 3;
        let mut plan = CommandPlan::default();
        plan.prepare(
            &DrawData {
                vertices: &vertices,
                indices: &indices,
                commands: &commands,
                effects: &[],
            },
            DrawOptimization {
                instance_quads: true,
                ..Default::default()
            },
            1.,
        )
        .unwrap();
        assert_eq!(plan.instances[0].vertices, vertices[..4]);
        assert_eq!(plan.instances[1].vertices, vertices[4..8]);
        assert_eq!(
            plan.draws[2].geometry,
            GeometryRange::Triangles {
                first_index: 12,
                index_count: 3
            }
        );
        let data = DrawData {
            vertices: &vertices,
            indices: &indices,
            commands: &commands,
            effects: &[],
        };
        let originals: Vec<_> = DrawCalls::new(&data, None)
            .map(|(_, range)| range)
            .collect();
        assert_eq!(
            originals,
            [
                GeometryRange::Triangles {
                    first_index: 0,
                    index_count: 6
                },
                GeometryRange::Triangles {
                    first_index: 6,
                    index_count: 6
                },
                GeometryRange::Triangles {
                    first_index: 12,
                    index_count: 3
                }
            ]
        );
    }
    #[test]
    fn conservative_pixel_bounds_prevent_near_edge_reordering_at_each_density() {
        for scale in [1., 1.25, 1.5, 2., 4.] {
            let (mut vertices, indices, commands) = fixture(false);
            for vertex in &mut vertices[8..] {
                vertex.pos[0] -= 20. - 0.25 / scale;
            }
            let mut plan = CommandPlan::default();
            plan.prepare(
                &DrawData {
                    vertices: &vertices,
                    indices: &indices,
                    commands: &commands,
                    effects: &[],
                },
                DrawOptimization {
                    sort_disjoint: true,
                    ..Default::default()
                },
                scale,
            )
            .unwrap();
            assert_eq!(plan.reordered_commands(), 0);
            assert_eq!(plan.draws[1].command.texture, TextureId(3));
        }
    }
    #[test]
    fn invalid_runs_reject_before_replacing_output_and_filters_are_barriers() {
        let (vertices, indices, mut commands) = fixture(false);
        let effects = [crate::effects::DrawEffects {
            filter: crate::effects::ImageFilter {
                overwrite: 1,
                ..Default::default()
            },
            ..Default::default()
        }];
        commands[1].effects = 1;
        let mut plan = CommandPlan::default();
        let options = DrawOptimization {
            sort_disjoint: true,
            instance_quads: true,
        };
        plan.prepare(
            &DrawData {
                vertices: &vertices,
                indices: &indices,
                commands: &commands,
                effects: &effects,
            },
            options,
            1.,
        )
        .unwrap();
        assert_eq!(plan.moved, 0);
        assert_eq!(plan.instances.len(), 2);
        let old = plan.draws.clone();
        commands[0].elem_count = 5;
        assert_eq!(
            plan.prepare(
                &DrawData {
                    vertices: &vertices,
                    indices: &indices,
                    commands: &commands,
                    effects: &effects
                },
                options,
                1.
            ),
            Err(PlanError::InvalidCommands)
        );
        assert_eq!(plan.draws, old);
        commands[0].elem_count = 6;
        assert_eq!(
            plan.prepare(
                &DrawData {
                    vertices: &vertices,
                    indices: &indices,
                    commands: &commands,
                    effects: &effects
                },
                options,
                0.
            ),
            Err(PlanError::InvalidGeometry)
        );
        assert_eq!(plan.draws, old);
    }
}
