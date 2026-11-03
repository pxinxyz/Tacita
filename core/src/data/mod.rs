use crate::{id::Id, panels::PanelCellScope, ui::Ui};
use core::ops::Range;

mod curve;
mod editor_canvas;
mod gradient;
mod menu;
mod numeric;
mod plots;
mod property;
mod selection;
mod table;
pub use curve::{CurveEditor, CurveInterpolation, CurveKey, MAX_CURVE_KEYS};
pub use gradient::{GradientEditor, MAX_EDITOR_STOPS};
pub use menu::MenuBarScope;
pub use numeric::NumericEdit;
pub use plots::{PlotHover, PlotOptions, MAX_PLOT_COLUMNS};
pub use property::PropertyGridScope;
pub use selection::Selection;
pub use table::{DataTableOptions, DataTableScope, DataTableState, SortSpec};

pub const MAX_DATA_ROWS: usize = 1_000_000;
pub const MAX_DATA_COLUMNS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataError {
    InvalidInput,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisibleRows {
    pub start: usize,
    pub end: usize,
}
impl VisibleRows {
    pub fn new(
        count: usize,
        height: f32,
        gap: f32,
        origin: f32,
        clip: [f32; 2],
    ) -> Result<Self, DataError> {
        if ![height, gap, origin, clip[0], clip[1]]
            .iter()
            .all(|v| v.is_finite())
            || height <= 0.
            || gap < 0.
            || clip[1] < clip[0]
        {
            return Err(DataError::InvalidInput);
        }
        let stride = height as f64 + gap as f64;
        let origin = origin as f64;
        let mut start = (((clip[0] as f64 - origin) / stride).max(0.) as usize).min(count);
        let mut end = ((((clip[1] as f64 - origin) / stride).max(0.) as usize).saturating_add(1))
            .min(count)
            .max(start);
        if start < end && origin + start as f64 * stride + height as f64 <= clip[0] as f64 {
            start += 1;
        }
        if start < end && origin + (end - 1) as f64 * stride >= clip[1] as f64 {
            end -= 1;
        }
        if clip[0] == clip[1] {
            end = start;
        }
        Ok(Self { start, end })
    }
    pub(crate) fn in_rect(
        count: usize,
        height: f32,
        gap: f32,
        origin: f32,
        clip: [f32; 4],
    ) -> Result<Self, DataError> {
        if clip[2] <= clip[0] || clip[3] <= clip[1] {
            return Ok(Self::default());
        }
        Self::new(count, height, gap, origin, [clip[1], clip[3]])
    }

    pub fn range(self) -> Range<usize> {
        self.start..self.end
    }
}

pub struct RowClipperScope {
    rect: [f32; 4],
    height: f32,
    stride: f32,
    visible: VisibleRows,
}
impl RowClipperScope {
    pub fn visible_rows(&self) -> VisibleRows {
        self.visible
    }
}

pub struct DataCellScope {
    cell: PanelCellScope,
}

impl Ui<'_> {
    pub fn begin_rows(
        &mut self,
        label: &str,
        count: usize,
        height: f32,
    ) -> Result<RowClipperScope, DataError> {
        let gap = self.ctx.style.item_spacing[1];
        if count > MAX_DATA_ROWS
            || !height.is_finite()
            || height <= 0.
            || !gap.is_finite()
            || gap < 0.
        {
            return Err(DataError::InvalidInput);
        }
        let total = if count == 0 {
            0.
        } else {
            count as f32 * (height + gap) - gap
        };
        let width = crate::widgets::available_width(self.ctx);
        let rect = self.allocate([width, total]);
        let clip = self
            .ctx
            .interaction_clip
            .unwrap_or_else(|| crate::widgets::draw_parts(self.ctx).1.current_clip());
        let visible = VisibleRows::in_rect(count, height, gap, rect[1], clip)?;
        let id = self.widget_id(label);
        self.ctx.id_stack.push_id(id);
        Ok(RowClipperScope {
            rect,
            height,
            stride: height + gap,
            visible,
        })
    }
    pub fn begin_row(&mut self, scope: &RowClipperScope, index: usize) -> Option<DataCellScope> {
        if !scope.visible.range().contains(&index) {
            return None;
        }
        let y = scope.rect[1] + index as f32 * scope.stride;
        Some(self.data_cell(
            [scope.rect[0], y, scope.rect[2], y + scope.height],
            scope.rect,
            index,
            0,
        ))
    }
    pub fn end_rows(&mut self, _scope: RowClipperScope) {
        self.ctx.id_stack.pop();
    }
    pub fn rows(
        &mut self,
        label: &str,
        count: usize,
        height: f32,
        f: impl FnOnce(&mut Ui, &RowClipperScope),
    ) -> Result<(), DataError> {
        let scope = self.begin_rows(label, count, height)?;
        f(self, &scope);
        self.end_rows(scope);
        Ok(())
    }
    pub fn row(&mut self, scope: &RowClipperScope, index: usize, f: impl FnOnce(&mut Ui)) -> bool {
        let Some(cell) = self.begin_row(scope, index) else {
            return false;
        };
        f(self);
        self.end_data_cell(cell);
        true
    }
    pub(crate) fn data_cell(
        &mut self,
        rect: [f32; 4],
        clip: [f32; 4],
        group: usize,
        index: usize,
    ) -> DataCellScope {
        let cell = crate::panels::begin_cell(self, rect, group, index);
        crate::widgets::draw_parts(self.ctx).1.push_clip_rect(clip);
        self.ctx.interaction_clip = Some(crate::draw::intersect_rects(
            self.ctx.interaction_clip.unwrap_or(rect),
            clip,
        ));
        if !crate::ui::point_in_rect(
            self.ctx.input.mouse_pos,
            self.ctx.interaction_clip.unwrap_or(rect),
        ) {
            self.ctx.prev_mouse_down[0] = true;
        }
        DataCellScope { cell }
    }
    pub fn end_data_cell(&mut self, scope: DataCellScope) {
        crate::widgets::draw_parts(self.ctx).1.pop_clip_rect();
        self.end_panel_cell(scope.cell);
    }
}

pub(crate) fn click(ui: &mut Ui<'_>, id: Id, rect: [f32; 4]) -> bool {
    crate::widgets::register_focusable_item(ui.ctx, id, rect);
    let hovered = ui.is_hovered(rect);
    if hovered {
        ui.ctx.hot = Some(id);
    }
    if ui.ctx.disabled_depth == 0
        && crate::ui::point_in_rect(ui.mouse_pos(), rect)
        && crate::ui::hover_layer_gate(ui.ctx)
        && ui.mouse_pressed(0)
    {
        ui.capture(id);
        ui.focus(id);
    }
    let result = ui.is_captured(id)
        && ui.mouse_released(0)
        && crate::ui::point_in_rect(ui.mouse_pos(), rect)
        && ui
            .ctx
            .interaction_clip
            .is_none_or(|clip| crate::ui::point_in_rect(ui.mouse_pos(), clip));
    if ui.is_captured(id) && !ui.mouse_down(0) {
        ui.release_capture(id);
    }
    result || ui.keyboard_activated(id)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod editor_tests;
