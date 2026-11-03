pub const MAX_LAYOUT_ITEMS: usize = 64;
pub(super) const LIMIT: f32 = 1_000_000.0;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutKind {
    Row = 0,
    #[default]
    Column = 1,
    Grid = 2,
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutAlign {
    #[default]
    Start = 0,
    Center = 1,
    End = 2,
    Stretch = 3,
    Baseline = 4,
}
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutJustify {
    #[default]
    Start = 0,
    Center = 1,
    End = 2,
    SpaceBetween = 3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutSpec {
    pub kind: LayoutKind,
    pub size: [f32; 2],
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub padding: [f32; 4],
    pub gap: [f32; 2],
    pub align: LayoutAlign,
    pub justify: LayoutJustify,
    pub columns: usize,
}
impl Default for LayoutSpec {
    fn default() -> Self {
        Self {
            kind: LayoutKind::Column,
            size: [0.0; 2],
            min: [0.0; 2],
            max: [LIMIT; 2],
            padding: [0.0; 4],
            gap: [6.0; 2],
            align: LayoutAlign::Start,
            justify: LayoutJustify::Start,
            columns: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutItem {
    pub preferred: [f32; 2],
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub grow: f32,
    pub shrink: f32,
    pub margin: [f32; 4],
    pub baseline: f32,
    pub absolute: bool,
    pub anchor: [f32; 2],
    pub offset: [f32; 2],
    pub layer: i32,
    pub hit_test: bool,
}
impl Default for LayoutItem {
    fn default() -> Self {
        Self {
            preferred: [0.0; 2],
            min: [0.0; 2],
            max: [LIMIT; 2],
            grow: 0.0,
            shrink: 1.0,
            margin: [0.0; 4],
            baseline: 0.0,
            absolute: false,
            anchor: [0.0; 2],
            offset: [0.0; 2],
            layer: 0,
            hit_test: true,
        }
    }
}
impl LayoutItem {
    pub fn fixed(size: [f32; 2]) -> Self {
        Self {
            preferred: size,
            min: size,
            max: size,
            ..Self::default()
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    TooManyItems,
    InvalidGeometry,
    InvalidIndex,
    DuplicateItem,
}

#[derive(Clone, Copy, Debug)]
pub struct LayoutGeometry {
    pub size: [f32; 2],
    pub(crate) count: usize,
    pub(crate) rects: [[f32; 4]; MAX_LAYOUT_ITEMS],
    pub(crate) layers: [i32; MAX_LAYOUT_ITEMS],
    pub(crate) hit_test: [bool; MAX_LAYOUT_ITEMS],
}
impl LayoutGeometry {
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn rect(&self, index: usize) -> Option<[f32; 4]> {
        (index < self.count).then(|| self.rects[index])
    }
}
fn lengths(values: &[f32]) -> bool {
    values
        .iter()
        .all(|v| v.is_finite() && *v >= 0.0 && *v <= LIMIT)
}
fn constraints(min: [f32; 2], max: [f32; 2]) -> bool {
    lengths(&min) && lengths(&max) && min[0] <= max[0] && min[1] <= max[1]
}
impl LayoutSpec {
    pub fn is_valid(&self) -> bool {
        lengths(&self.size)
            && constraints(self.min, self.max)
            && lengths(&self.padding)
            && lengths(&self.gap)
            && self.columns > 0
            && self.columns <= MAX_LAYOUT_ITEMS
            && !(self.kind == LayoutKind::Column && self.align == LayoutAlign::Baseline)
    }
}
impl LayoutItem {
    pub fn is_valid(&self) -> bool {
        lengths(&self.preferred)
            && constraints(self.min, self.max)
            && lengths(&self.margin)
            && lengths(&[self.grow, self.shrink, self.baseline])
            && self
                .anchor
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0 && *v <= 1.0)
            && self
                .offset
                .iter()
                .all(|v| v.is_finite() && v.abs() <= LIMIT)
    }
}
pub(super) fn distribution(justify: LayoutJustify, free: f32, count: usize) -> (f32, f32) {
    let free = free.max(0.0);
    match justify {
        LayoutJustify::Start => (0.0, 0.0),
        LayoutJustify::Center => (free * 0.5, 0.0),
        LayoutJustify::End => (free, 0.0),
        LayoutJustify::SpaceBetween if count > 1 => (0.0, free / (count - 1) as f32),
        LayoutJustify::SpaceBetween => (0.0, 0.0),
    }
}
pub(super) fn flex(
    values: &mut [f32],
    minima: &[f32],
    maxima: &[f32],
    weights: &[f32],
    target: f32,
) {
    for _ in 0..=values.len() {
        let delta = target - values.iter().sum::<f32>();
        if delta.abs() < 0.001 {
            break;
        }
        let mut total = 0.0;
        for (i, &value) in values.iter().enumerate() {
            if (delta > 0.0 && value < maxima[i]) || (delta < 0.0 && value > minima[i]) {
                total += weights[i];
            }
        }
        if total <= 0.0 {
            break;
        }
        for (i, value) in values.iter_mut().enumerate() {
            if (delta > 0.0 && *value < maxima[i]) || (delta < 0.0 && *value > minima[i]) {
                *value = (*value + delta * weights[i] / total).clamp(minima[i], maxima[i]);
            }
        }
    }
}
pub(super) fn cross(align: LayoutAlign, room: f32, size: f32) -> f32 {
    match align {
        LayoutAlign::Center => (room - size) * 0.5,
        LayoutAlign::End => room - size,
        _ => 0.0,
    }
}
pub fn resolve_layout(
    spec: LayoutSpec,
    items: &[LayoutItem],
) -> Result<LayoutGeometry, LayoutError> {
    resolve_layout_with(spec, items.len(), |i| items[i])
}
pub fn resolve_layout_with(
    spec: LayoutSpec,
    count: usize,
    get: impl Fn(usize) -> LayoutItem,
) -> Result<LayoutGeometry, LayoutError> {
    if count > MAX_LAYOUT_ITEMS {
        return Err(LayoutError::TooManyItems);
    }
    if !spec.is_valid() || (0..count).any(|i| !get(i).is_valid()) {
        return Err(LayoutError::InvalidGeometry);
    }
    let mut result = LayoutGeometry {
        size: [0.0; 2],
        count,
        rects: [[0.0; 4]; MAX_LAYOUT_ITEMS],
        layers: [0; MAX_LAYOUT_ITEMS],
        hit_test: [false; MAX_LAYOUT_ITEMS],
    };
    let mut flow = [0; MAX_LAYOUT_ITEMS];
    let mut n = 0;
    let mut dims = [[0.0; 2]; MAX_LAYOUT_ITEMS];
    for (i, dimensions) in dims[..count].iter_mut().enumerate() {
        let item = get(i);
        *dimensions = [
            item.preferred[0].clamp(item.min[0], item.max[0]),
            item.preferred[1].clamp(item.min[1], item.max[1]),
        ];
        result.layers[i] = item.layer;
        result.hit_test[i] = item.hit_test;
        if !item.absolute {
            flow[n] = i;
            n += 1;
        }
    }
    let mut natural = [0.0_f32; 2];
    let mut cols = [0.0_f32; MAX_LAYOUT_ITEMS];
    let mut rows = [0.0_f32; MAX_LAYOUT_ITEMS];
    let mut col_min = [0.0_f32; MAX_LAYOUT_ITEMS];
    let mut row_min = [0.0_f32; MAX_LAYOUT_ITEMS];
    let mut baselines = [0.0_f32; MAX_LAYOUT_ITEMS];
    let mut descents = [0.0_f32; MAX_LAYOUT_ITEMS];
    let nc = spec.columns.min(n.max(1));
    let nr = n.div_ceil(nc);
    let main = if spec.kind == LayoutKind::Column {
        1
    } else {
        0
    };
    let other = 1 - main;
    for (k, &i) in flow[..n].iter().enumerate() {
        let item = get(i);
        let outer = [
            dims[i][0] + item.margin[0] + item.margin[2],
            dims[i][1] + item.margin[1] + item.margin[3],
        ];
        if spec.kind == LayoutKind::Grid {
            cols[k % nc] = cols[k % nc].max(outer[0]);
            rows[k / nc] = rows[k / nc].max(outer[1]);
            col_min[k % nc] = col_min[k % nc].max(item.min[0] + item.margin[0] + item.margin[2]);
            row_min[k / nc] = row_min[k / nc].max(item.min[1] + item.margin[1] + item.margin[3]);
        } else {
            natural[main] += outer[main];
            natural[other] = natural[other].max(outer[other]);
        }
        let row = if spec.kind == LayoutKind::Grid {
            k / nc
        } else {
            0
        };
        baselines[row] = baselines[row].max(item.margin[1] + item.baseline.min(dims[i][1]));
        descents[row] =
            descents[row].max(item.margin[3] + dims[i][1] - item.baseline.min(dims[i][1]));
    }
    if spec.kind == LayoutKind::Grid {
        if spec.align == LayoutAlign::Baseline {
            for (r, track) in rows[..nr].iter_mut().enumerate() {
                *track = (*track).max(baselines[r] + descents[r]);
            }
        }
        natural = [
            cols[..nc].iter().sum::<f32>() + spec.gap[0] * nc.saturating_sub(1) as f32,
            rows[..nr].iter().sum::<f32>() + spec.gap[1] * nr.saturating_sub(1) as f32,
        ];
    } else {
        natural[main] += spec.gap[main] * n.saturating_sub(1) as f32;
        if spec.align == LayoutAlign::Baseline {
            natural[1] = baselines[0] + descents[0];
        }
    }
    for (a, size) in result.size.iter_mut().enumerate() {
        let intrinsic = natural[a] + spec.padding[a] + spec.padding[a + 2];
        *size = if spec.size[a] == 0.0 {
            intrinsic
        } else {
            spec.size[a]
        }
        .clamp(spec.min[a], spec.max[a]);
    }
    let room = [
        (result.size[0] - spec.padding[0] - spec.padding[2]).max(0.0),
        (result.size[1] - spec.padding[1] - spec.padding[3]).max(0.0),
    ];
    if spec.kind == LayoutKind::Grid {
        let unit = [1.0; MAX_LAYOUT_ITEMS];
        let max = [LIMIT * 4.0; MAX_LAYOUT_ITEMS];
        if spec.size[0] > 0.0 {
            flex(
                &mut cols[..nc],
                &col_min[..nc],
                &max[..nc],
                &unit[..nc],
                (room[0] - spec.gap[0] * nc.saturating_sub(1) as f32).max(0.0),
            );
        }
        if spec.size[1] > 0.0 {
            flex(
                &mut rows[..nr],
                &row_min[..nr],
                &max[..nr],
                &unit[..nr],
                (room[1] - spec.gap[1] * nr.saturating_sub(1) as f32).max(0.0),
            );
        }
        for (k, &i) in flow[..n].iter().enumerate() {
            let item = get(i);
            let col = k % nc;
            let row = k / nc;
            let avail = [
                (cols[col] - item.margin[0] - item.margin[2]).max(0.0),
                (rows[row] - item.margin[1] - item.margin[3]).max(0.0),
            ];
            if spec.align == LayoutAlign::Stretch {
                for (a, dimension) in dims[i].iter_mut().enumerate() {
                    *dimension = avail[a].clamp(item.min[a], item.max[a]);
                }
            } else if item.shrink > 0.0 {
                for (a, dimension) in dims[i].iter_mut().enumerate() {
                    *dimension = (*dimension).min(avail[a]).clamp(item.min[a], item.max[a]);
                }
            }
        }
        if spec.align == LayoutAlign::Baseline {
            baselines.fill(0.0);
            descents.fill(0.0);
            for (k, &i) in flow[..n].iter().enumerate() {
                let item = get(i);
                let row = k / nc;
                let baseline = item.baseline.min(dims[i][1]);
                baselines[row] = baselines[row].max(item.margin[1] + baseline);
                descents[row] = descents[row].max(item.margin[3] + dims[i][1] - baseline);
            }
            for (r, track) in rows[..nr].iter_mut().enumerate() {
                *track = (*track).max(baselines[r] + descents[r]);
            }
        }
        for (k, &i) in flow[..n].iter().enumerate() {
            let item = get(i);
            let col = k % nc;
            let row = k / nc;
            let cell = [
                spec.padding[0] + cols[..col].iter().sum::<f32>() + spec.gap[0] * col as f32,
                spec.padding[1] + rows[..row].iter().sum::<f32>() + spec.gap[1] * row as f32,
            ];
            let avail = [
                (cols[col] - item.margin[0] - item.margin[2]).max(0.0),
                (rows[row] - item.margin[1] - item.margin[3]).max(0.0),
            ];
            let x = cell[0] + item.margin[0] + cross(spec.align, avail[0], dims[i][0]);
            let y = if spec.align == LayoutAlign::Baseline {
                cell[1] + baselines[row] - item.baseline.min(dims[i][1])
            } else {
                cell[1] + item.margin[1] + cross(spec.align, avail[1], dims[i][1])
            };
            result.rects[i] = [x, y, x + dims[i][0], y + dims[i][1]];
        }
    } else {
        let mut values = [0.0; MAX_LAYOUT_ITEMS];
        let mut mins = values;
        let mut maxs = values;
        let mut weights = values;
        let mut margins = 0.0;
        for (k, &i) in flow[..n].iter().enumerate() {
            let item = get(i);
            values[k] = dims[i][main];
            mins[k] = item.min[main];
            maxs[k] = item.max[main];
            margins += item.margin[main] + item.margin[main + 2];
        }
        let target = (room[main] - margins - spec.gap[main] * n.saturating_sub(1) as f32).max(0.0);
        let growing = target > values[..n].iter().sum::<f32>();
        for (k, &i) in flow[..n].iter().enumerate() {
            let item = get(i);
            weights[k] = if growing {
                item.grow
            } else {
                item.shrink * values[k]
            };
        }
        flex(
            &mut values[..n],
            &mins[..n],
            &maxs[..n],
            &weights[..n],
            target,
        );
        let used =
            values[..n].iter().sum::<f32>() + margins + spec.gap[main] * n.saturating_sub(1) as f32;
        let (offset, extra_gap) = distribution(spec.justify, room[main] - used, n);
        let mut pen = spec.padding[main] + offset;
        for (k, &i) in flow[..n].iter().enumerate() {
            let item = get(i);
            dims[i][main] = values[k];
            let avail = (room[other] - item.margin[other] - item.margin[other + 2]).max(0.0);
            if spec.align == LayoutAlign::Stretch {
                dims[i][other] = avail.clamp(item.min[other], item.max[other]);
            }
            let mut pos = [0.0; 2];
            pos[main] = pen + item.margin[main];
            pos[other] =
                spec.padding[other] + item.margin[other] + cross(spec.align, avail, dims[i][other]);
            if spec.align == LayoutAlign::Baseline {
                pos[1] = spec.padding[1] + baselines[0] - item.baseline.min(dims[i][1]);
            }
            result.rects[i] = [pos[0], pos[1], pos[0] + dims[i][0], pos[1] + dims[i][1]];
            pen +=
                item.margin[main] + values[k] + item.margin[main + 2] + spec.gap[main] + extra_gap;
        }
    }
    for (i, &dimensions) in dims[..count].iter().enumerate() {
        let item = get(i);
        if item.absolute {
            let mut pos = [0.0; 2];
            for (a, coordinate) in pos.iter_mut().enumerate() {
                let avail = (room[a] - item.margin[a] - item.margin[a + 2]).max(0.0);
                *coordinate = spec.padding[a]
                    + item.margin[a]
                    + (avail - dimensions[a]) * item.anchor[a]
                    + item.offset[a];
            }
            result.rects[i] = [
                pos[0],
                pos[1],
                pos[0] + dimensions[0],
                pos[1] + dimensions[1],
            ];
        }
    }
    if !lengths(&result.size)
        || result.rects[..count]
            .iter()
            .flatten()
            .any(|v| !v.is_finite())
    {
        return Err(LayoutError::InvalidGeometry);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flex_freezes_constraints_and_redistributes() {
        let items = [
            LayoutItem {
                preferred: [20.0, 10.0],
                max: [30.0, 100.0],
                grow: 1.0,
                ..LayoutItem::default()
            },
            LayoutItem {
                preferred: [20.0, 10.0],
                grow: 1.0,
                ..LayoutItem::default()
            },
        ];
        let g = resolve_layout(
            LayoutSpec {
                kind: LayoutKind::Row,
                size: [100.0, 20.0],
                gap: [0.0; 2],
                ..LayoutSpec::default()
            },
            &items,
        )
        .unwrap();
        assert_eq!(g.rect(0), Some([0.0, 0.0, 30.0, 10.0]));
        assert_eq!(g.rect(1), Some([30.0, 0.0, 100.0, 10.0]));
        let small = resolve_layout(
            LayoutSpec {
                kind: LayoutKind::Row,
                size: [10.0, 20.0],
                gap: [0.0; 2],
                ..LayoutSpec::default()
            },
            &items,
        )
        .unwrap();
        assert_eq!(small.rect(1).unwrap()[2], 10.0);
    }
    #[test]
    fn intrinsic_baseline_margins_and_absolute_anchor() {
        let items = [
            LayoutItem {
                preferred: [20.0, 10.0],
                baseline: 8.0,
                margin: [1.0, 2.0, 3.0, 4.0],
                ..LayoutItem::default()
            },
            LayoutItem {
                preferred: [30.0, 20.0],
                baseline: 15.0,
                ..LayoutItem::default()
            },
            LayoutItem {
                absolute: true,
                preferred: [8.0, 8.0],
                anchor: [1.0, 1.0],
                ..LayoutItem::default()
            },
        ];
        let g = resolve_layout(
            LayoutSpec {
                kind: LayoutKind::Row,
                align: LayoutAlign::Baseline,
                padding: [2.0; 4],
                ..LayoutSpec::default()
            },
            &items,
        )
        .unwrap();
        assert_eq!(g.size, [64.0, 25.0]);
        assert_eq!(g.rect(0).unwrap()[1] + 8.0, g.rect(1).unwrap()[1] + 15.0);
        assert_eq!(g.rect(2), Some([54.0, 15.0, 62.0, 23.0]));
    }
    #[test]
    fn grid_uses_uneven_intrinsic_tracks_and_nested_constraints() {
        let items = [
            LayoutItem::fixed([10.0, 5.0]),
            LayoutItem::fixed([30.0, 10.0]),
            LayoutItem::fixed([20.0, 15.0]),
        ];
        let g = resolve_layout(
            LayoutSpec {
                kind: LayoutKind::Grid,
                columns: 2,
                gap: [2.0, 3.0],
                ..LayoutSpec::default()
            },
            &items,
        )
        .unwrap();
        assert_eq!(g.size, [52.0, 28.0]);
        assert_eq!(g.rect(2), Some([0.0, 13.0, 20.0, 28.0]));
        assert!(resolve_layout(
            LayoutSpec {
                padding: [f32::NAN; 4],
                ..LayoutSpec::default()
            },
            &items
        )
        .is_err());
        assert!(resolve_layout_with(LayoutSpec::default(), 65, |_| LayoutItem::default()).is_err());
    }
    #[test]
    fn narrow_grid_preserves_track_minimums_without_overlapping_cells() {
        let items = [LayoutItem {
            preferred: [100.0, 30.0],
            min: [40.0, 20.0],
            ..LayoutItem::default()
        }; 2];
        let g = resolve_layout(
            LayoutSpec {
                kind: LayoutKind::Grid,
                columns: 2,
                size: [50.0, 30.0],
                gap: [5.0; 2],
                ..LayoutSpec::default()
            },
            &items,
        )
        .unwrap();
        let first = g.rect(0).unwrap();
        let second = g.rect(1).unwrap();
        assert_eq!(first[2] - first[0], 40.0);
        assert_eq!(second[0], 45.0);
        assert_eq!(second[2], 85.0);
        assert_eq!(g.size[0], 50.0);
    }

    #[test]
    fn constrained_grid_recomputes_baselines_before_placing_the_next_row() {
        let items = [
            LayoutItem {
                preferred: [20.0, 40.0],
                baseline: 35.0,
                ..LayoutItem::default()
            },
            LayoutItem {
                preferred: [20.0, 20.0],
                baseline: 5.0,
                ..LayoutItem::default()
            },
            LayoutItem::fixed([20.0, 10.0]),
        ];
        let g = resolve_layout(
            LayoutSpec {
                kind: LayoutKind::Grid,
                columns: 2,
                align: LayoutAlign::Baseline,
                size: [50.0, 35.0],
                gap: [5.0; 2],
                ..LayoutSpec::default()
            },
            &items,
        )
        .unwrap();
        let a = g.rect(0).unwrap();
        let b = g.rect(1).unwrap();
        let c = g.rect(2).unwrap();
        assert_eq!(
            a[1] + (a[3] - a[1]).min(35.0),
            b[1] + (b[3] - b[1]).min(5.0)
        );
        assert!(c[1] >= a[3].max(b[3]) + 5.0);
    }

    #[test]
    fn grid_track_minimum_includes_large_margins_without_invalid_clamping() {
        let g = resolve_layout(
            LayoutSpec {
                kind: LayoutKind::Grid,
                size: [100.0; 2],
                ..LayoutSpec::default()
            },
            &[LayoutItem {
                min: [LIMIT; 2],
                margin: [LIMIT; 4],
                ..LayoutItem::default()
            }],
        )
        .unwrap();
        assert_eq!(g.rect(0), Some([LIMIT, LIMIT, LIMIT * 2.0, LIMIT * 2.0]));
    }
}
