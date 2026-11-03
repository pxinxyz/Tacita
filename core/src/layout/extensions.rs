use super::solver::{cross, distribution, flex, LIMIT};
use super::{
    resolve_layout_with, LayoutAlign, LayoutError, LayoutGeometry, LayoutItem, LayoutKind,
    LayoutSpec, MAX_LAYOUT_ITEMS,
};

pub const GRID_AUTO: u32 = u32::MAX;
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContentAlign {
    #[default]
    Start = 0,
    Center = 1,
    End = 2,
    Stretch = 3,
    SpaceBetween = 4,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayoutOptions {
    pub wrap: bool,
    pub align_content: ContentAlign,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridCell {
    pub row: u32,
    pub column: u32,
    pub row_span: u32,
    pub column_span: u32,
}
impl Default for GridCell {
    fn default() -> Self {
        Self {
            row: GRID_AUTO,
            column: GRID_AUTO,
            row_span: 1,
            column_span: 1,
        }
    }
}
impl GridCell {
    pub fn at(row: u32, column: u32, row_span: u32, column_span: u32) -> Self {
        Self {
            row,
            column,
            row_span,
            column_span,
        }
    }
    pub fn spanning(row_span: u32, column_span: u32) -> Self {
        Self {
            row_span,
            column_span,
            ..Self::default()
        }
    }
    fn is_valid(self) -> bool {
        (1..=64).contains(&self.row_span)
            && (1..=64).contains(&self.column_span)
            && ((self.row == GRID_AUTO && self.column == GRID_AUTO)
                || (self.row < 64 && self.column < 64))
    }
}
pub fn resolve_layout_extended(
    spec: LayoutSpec,
    items: &[LayoutItem],
    options: LayoutOptions,
    cells: &[GridCell],
) -> Result<LayoutGeometry, LayoutError> {
    if !cells.is_empty() && cells.len() != items.len() {
        return Err(LayoutError::InvalidGeometry);
    }
    resolve_layout_extended_with(spec, options, items.len(), |i| {
        (items[i], cells.get(i).copied().unwrap_or_default())
    })
}
pub fn resolve_layout_extended_with(
    spec: LayoutSpec,
    options: LayoutOptions,
    count: usize,
    get: impl Fn(usize) -> (LayoutItem, GridCell),
) -> Result<LayoutGeometry, LayoutError> {
    if count > MAX_LAYOUT_ITEMS {
        return Err(LayoutError::TooManyItems);
    }
    if !spec.is_valid()
        || (options.wrap && spec.kind == LayoutKind::Grid)
        || (0..count).any(|i| {
            let (item, cell) = get(i);
            !item.is_valid()
                || !cell.is_valid()
                || ((spec.kind != LayoutKind::Grid || item.absolute) && cell != GridCell::default())
        })
    {
        return Err(LayoutError::InvalidGeometry);
    }
    if !options.wrap && (0..count).all(|i| get(i).1 == GridCell::default()) {
        return resolve_layout_with(spec, count, |i| get(i).0);
    }
    let mut g = LayoutGeometry {
        size: [0.; 2],
        count,
        rects: [[0.; 4]; MAX_LAYOUT_ITEMS],
        layers: [0; MAX_LAYOUT_ITEMS],
        hit_test: [false; MAX_LAYOUT_ITEMS],
    };
    let mut dims = [[0.; 2]; MAX_LAYOUT_ITEMS];
    let mut flow = [0; MAX_LAYOUT_ITEMS];
    let mut n = 0;
    for (i, dim) in dims.iter_mut().enumerate().take(count) {
        let item = get(i).0;
        *dim = core::array::from_fn(|a| item.preferred[a].clamp(item.min[a], item.max[a]));
        g.layers[i] = item.layer;
        g.hit_test[i] = item.hit_test;
        if !item.absolute {
            flow[n] = i;
            n += 1;
        }
    }
    if spec.kind == LayoutKind::Grid {
        grid(spec, &flow[..n], &get, &mut dims, &mut g)?;
    } else {
        wrap(spec, options, &flow[..n], &get, &mut dims, &mut g);
    }
    let room = room(spec, g.size);
    for (i, dim) in dims.iter().enumerate().take(count) {
        let item = get(i).0;
        if item.absolute {
            let pos = core::array::from_fn::<_, 2, _>(|a| {
                spec.padding[a]
                    + item.margin[a]
                    + ((room[a] - item.margin[a] - item.margin[a + 2]).max(0.) - dim[a])
                        * item.anchor[a]
                    + item.offset[a]
            });
            g.rects[i] = [pos[0], pos[1], pos[0] + dim[0], pos[1] + dim[1]];
        }
    }
    if g.size
        .iter()
        .any(|v| !v.is_finite() || !(0.0..=LIMIT).contains(v))
        || g.rects[..count].iter().flatten().any(|v| !v.is_finite())
    {
        return Err(LayoutError::InvalidGeometry);
    }
    Ok(g)
}
fn room(spec: LayoutSpec, size: [f32; 2]) -> [f32; 2] {
    core::array::from_fn(|a| (size[a] - spec.padding[a] - spec.padding[a + 2]).max(0.))
}
fn container_size(spec: LayoutSpec, natural: [f32; 2]) -> [f32; 2] {
    core::array::from_fn(|a| {
        if spec.size[a] > 0. {
            spec.size[a]
        } else {
            natural[a] + spec.padding[a] + spec.padding[a + 2]
        }
        .clamp(spec.min[a], spec.max[a])
    })
}
fn expand(tracks: &mut [f32], need: f32) {
    let deficit = (need - tracks.iter().sum::<f32>()).max(0.) / tracks.len() as f32;
    for t in tracks {
        *t += deficit;
    }
}
fn grid(
    spec: LayoutSpec,
    flow: &[usize],
    get: &impl Fn(usize) -> (LayoutItem, GridCell),
    dims: &mut [[f32; 2]; MAX_LAYOUT_ITEMS],
    g: &mut LayoutGeometry,
) -> Result<(), LayoutError> {
    let mut occupied = [0u64; 64];
    let mut cells = [GridCell::default(); MAX_LAYOUT_ITEMS];
    let mut rows = 0;
    let columns = if flow.is_empty() { 0 } else { spec.columns };
    let mask = |column: u32, span: u32| {
        if span == 64 {
            u64::MAX
        } else {
            ((1u64 << span) - 1) << column
        }
    };
    for &i in flow {
        let c = get(i).1;
        if c.column_span as usize > columns {
            return Err(LayoutError::InvalidGeometry);
        }
        if c.row == GRID_AUTO {
            continue;
        }
        if c.row + c.row_span > 64 || c.column + c.column_span > columns as u32 {
            return Err(LayoutError::InvalidGeometry);
        }
        let bits = mask(c.column, c.column_span);
        if occupied[c.row as usize..(c.row + c.row_span) as usize]
            .iter()
            .any(|v| *v & bits != 0)
        {
            return Err(LayoutError::InvalidGeometry);
        }
        for row in &mut occupied[c.row as usize..(c.row + c.row_span) as usize] {
            *row |= bits;
        }
        cells[i] = c;
        rows = rows.max((c.row + c.row_span) as usize);
    }
    for &i in flow {
        let mut c = get(i).1;
        if c.row != GRID_AUTO {
            continue;
        }
        let mut found = None;
        'search: for r in 0..=64 - c.row_span {
            for col in 0..=columns as u32 - c.column_span {
                let bits = mask(col, c.column_span);
                if occupied[r as usize..(r + c.row_span) as usize]
                    .iter()
                    .all(|v| *v & bits == 0)
                {
                    found = Some((r, col, bits));
                    break 'search;
                }
            }
        }
        let (r, col, bits) = found.ok_or(LayoutError::InvalidGeometry)?;
        c.row = r;
        c.column = col;
        for row in &mut occupied[r as usize..(r + c.row_span) as usize] {
            *row |= bits;
        }
        cells[i] = c;
        rows = rows.max((r + c.row_span) as usize);
    }
    let mut tracks = [[0.; MAX_LAYOUT_ITEMS]; 2];
    let mut minima = tracks;
    for axis in 0..2 {
        for span in 1..=64 {
            for &i in flow {
                let c = cells[i];
                let (start, length) = if axis == 0 {
                    (c.column, c.column_span)
                } else {
                    (c.row, c.row_span)
                };
                if length != span {
                    continue;
                }
                let item = get(i).0;
                let a = start as usize;
                let b = (start + length) as usize;
                let spacing = spec.gap[axis] * (length - 1) as f32;
                let margins = item.margin[axis] + item.margin[axis + 2];
                expand(
                    &mut tracks[axis][a..b],
                    (dims[i][axis] + margins - spacing).max(0.),
                );
                expand(
                    &mut minima[axis][a..b],
                    (item.min[axis] + margins - spacing).max(0.),
                );
            }
        }
    }
    let mut baseline = [0f32; 64];
    let mut descent = baseline;
    if spec.align == LayoutAlign::Baseline {
        for &i in flow {
            let c = cells[i];
            let item = get(i).0;
            let r = c.row as usize;
            let b = item.baseline.min(dims[i][1]);
            baseline[r] = baseline[r].max(item.margin[1] + b);
            if c.row_span == 1 {
                descent[r] = descent[r].max(item.margin[3] + dims[i][1] - b);
            }
        }
        for r in 0..rows {
            tracks[1][r] = tracks[1][r].max(baseline[r] + descent[r]);
        }
    }
    let count = [columns, rows];
    let natural = core::array::from_fn(|a| {
        tracks[a][..count[a]].iter().sum::<f32>() + spec.gap[a] * count[a].saturating_sub(1) as f32
    });
    g.size = container_size(spec, natural);
    let available = room(spec, g.size);
    let unit = [1.; 64];
    let max = [LIMIT * 4.; 64];
    for a in 0..2 {
        flex(
            &mut tracks[a][..count[a]],
            &minima[a][..count[a]],
            &max[..count[a]],
            &unit[..count[a]],
            (available[a] - spec.gap[a] * count[a].saturating_sub(1) as f32).max(0.),
        );
    }
    for &i in flow {
        let c = cells[i];
        let item = get(i).0;
        let spans = [
            (c.column as usize, c.column_span as usize),
            (c.row as usize, c.row_span as usize),
        ];
        for a in 0..2 {
            let (start, len) = spans[a];
            let avail = (tracks[a][start..start + len].iter().sum::<f32>()
                + spec.gap[a] * (len - 1) as f32
                - item.margin[a]
                - item.margin[a + 2])
                .max(0.);
            if spec.align == LayoutAlign::Stretch {
                dims[i][a] = avail.clamp(item.min[a], item.max[a]);
            } else if item.shrink > 0. {
                dims[i][a] = dims[i][a].min(avail).clamp(item.min[a], item.max[a]);
            }
        }
    }
    if spec.align == LayoutAlign::Baseline {
        baseline.fill(0.);
        descent.fill(0.);
        for &i in flow {
            let c = cells[i];
            let item = get(i).0;
            let r = c.row as usize;
            let b = item.baseline.min(dims[i][1]);
            baseline[r] = baseline[r].max(item.margin[1] + b);
            if c.row_span == 1 {
                descent[r] = descent[r].max(item.margin[3] + dims[i][1] - b);
            }
        }
        for r in 0..rows {
            tracks[1][r] = tracks[1][r].max(baseline[r] + descent[r]);
        }
    }
    for &i in flow {
        let c = cells[i];
        let item = get(i).0;
        let spans = [
            (c.column as usize, c.column_span as usize),
            (c.row as usize, c.row_span as usize),
        ];
        let mut pos = [0.; 2];
        for a in 0..2 {
            let (start, len) = spans[a];
            let avail = (tracks[a][start..start + len].iter().sum::<f32>()
                + spec.gap[a] * (len - 1) as f32
                - item.margin[a]
                - item.margin[a + 2])
                .max(0.);
            pos[a] = spec.padding[a]
                + tracks[a][..start].iter().sum::<f32>()
                + spec.gap[a] * start as f32
                + item.margin[a]
                + cross(spec.align, avail, dims[i][a]);
        }
        if spec.align == LayoutAlign::Baseline {
            pos[1] = spec.padding[1]
                + tracks[1][..c.row as usize].iter().sum::<f32>()
                + spec.gap[1] * c.row as f32
                + baseline[c.row as usize]
                - item.baseline.min(dims[i][1]);
        }
        g.rects[i] = [pos[0], pos[1], pos[0] + dims[i][0], pos[1] + dims[i][1]];
    }
    Ok(())
}
fn wrap(
    spec: LayoutSpec,
    options: LayoutOptions,
    flow: &[usize],
    get: &impl Fn(usize) -> (LayoutItem, GridCell),
    dims: &mut [[f32; 2]; MAX_LAYOUT_ITEMS],
    g: &mut LayoutGeometry,
) {
    let main = usize::from(spec.kind == LayoutKind::Column);
    let other = 1 - main;
    let natural_main = flow
        .iter()
        .map(|&i| {
            let item = get(i).0;
            dims[i][main] + item.margin[main] + item.margin[main + 2]
        })
        .sum::<f32>()
        + spec.gap[main] * flow.len().saturating_sub(1) as f32;
    let mut natural = [0.; 2];
    natural[main] = natural_main;
    let main_size = container_size(spec, natural)[main];
    let limit = (main_size - spec.padding[main] - spec.padding[main + 2]).max(0.);
    let mut lines = [[0usize; 2]; 64];
    let mut line_count = 0;
    let mut begin = 0;
    let mut used = 0.;
    for (k, &i) in flow.iter().enumerate() {
        let item = get(i).0;
        let extent = dims[i][main] + item.margin[main] + item.margin[main + 2];
        if k > begin && used + spec.gap[main] + extent > limit + 0.0001 {
            lines[line_count] = [begin, k];
            line_count += 1;
            begin = k;
            used = 0.;
        }
        if k > begin {
            used += spec.gap[main];
        }
        used += extent;
    }
    if !flow.is_empty() {
        lines[line_count] = [begin, flow.len()];
        line_count += 1;
    }
    let mut heights = [0f32; 64];
    let mut baselines = heights;
    for line in 0..line_count {
        let range = lines[line][0]..lines[line][1];
        let indices = &flow[range];
        let mut values = [0.; 64];
        let mut mins = values;
        let mut maxs = values;
        let mut weights = values;
        let mut margins = 0.;
        for (k, &i) in indices.iter().enumerate() {
            let item = get(i).0;
            values[k] = dims[i][main];
            mins[k] = item.min[main];
            maxs[k] = item.max[main];
            margins += item.margin[main] + item.margin[main + 2];
        }
        let target =
            (limit - margins - spec.gap[main] * indices.len().saturating_sub(1) as f32).max(0.);
        let growing = target > values[..indices.len()].iter().sum::<f32>();
        for (k, &i) in indices.iter().enumerate() {
            let item = get(i).0;
            weights[k] = if growing {
                item.grow
            } else {
                item.shrink * values[k]
            };
        }
        flex(
            &mut values[..indices.len()],
            &mins[..indices.len()],
            &maxs[..indices.len()],
            &weights[..indices.len()],
            target,
        );
        let mut descent = 0f32;
        for (k, &i) in indices.iter().enumerate() {
            let item = get(i).0;
            dims[i][main] = values[k];
            heights[line] =
                heights[line].max(dims[i][other] + item.margin[other] + item.margin[other + 2]);
            if spec.align == LayoutAlign::Baseline {
                let b = item.baseline.min(dims[i][1]);
                baselines[line] = baselines[line].max(item.margin[1] + b);
                descent = descent.max(item.margin[3] + dims[i][1] - b);
            }
        }
        if spec.align == LayoutAlign::Baseline {
            heights[line] = heights[line].max(baselines[line] + descent);
        }
    }
    natural[other] = heights[..line_count].iter().sum::<f32>()
        + spec.gap[other] * line_count.saturating_sub(1) as f32;
    g.size = container_size(spec, natural);
    let available = room(spec, g.size);
    let free = (available[other] - natural[other]).max(0.);
    let (offset, extra) = match options.align_content {
        ContentAlign::Start => (0., 0.),
        ContentAlign::Center => (free * 0.5, 0.),
        ContentAlign::End => (free, 0.),
        ContentAlign::SpaceBetween if line_count > 1 => (0., free / (line_count - 1) as f32),
        ContentAlign::Stretch if line_count > 0 => {
            for h in &mut heights[..line_count] {
                *h += free / line_count as f32;
            }
            (0., 0.)
        }
        _ => (0., 0.),
    };
    let mut cross_pen = spec.padding[other] + offset;
    for line in 0..line_count {
        let indices = &flow[lines[line][0]..lines[line][1]];
        let occupied = indices
            .iter()
            .map(|&i| {
                let item = get(i).0;
                dims[i][main] + item.margin[main] + item.margin[main + 2]
            })
            .sum::<f32>()
            + spec.gap[main] * indices.len().saturating_sub(1) as f32;
        let (offset, gap) = distribution(spec.justify, available[main] - occupied, indices.len());
        let mut pen = spec.padding[main] + offset;
        for &i in indices {
            let item = get(i).0;
            let avail = (heights[line] - item.margin[other] - item.margin[other + 2]).max(0.);
            if spec.align == LayoutAlign::Stretch {
                dims[i][other] = avail.clamp(item.min[other], item.max[other]);
            }
            let mut pos = [0.; 2];
            pos[main] = pen + item.margin[main];
            pos[other] = cross_pen + item.margin[other] + cross(spec.align, avail, dims[i][other]);
            if spec.align == LayoutAlign::Baseline {
                pos[1] = cross_pen + baselines[line] - item.baseline.min(dims[i][1]);
            }
            g.rects[i] = [pos[0], pos[1], pos[0] + dims[i][0], pos[1] + dims[i][1]];
            pen += dims[i][main] + item.margin[main] + item.margin[main + 2] + spec.gap[main] + gap;
        }
        cross_pen += heights[line] + spec.gap[other] + extra;
    }
}
