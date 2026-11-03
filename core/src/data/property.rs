use super::{DataCellScope, DataError};
use crate::ui::Ui;

pub struct PropertyGridScope {
    width: f32,
    label_width: f32,
    row_height: f32,
}
impl Ui<'_> {
    pub fn begin_property_grid(
        &mut self,
        label: &str,
        label_width: f32,
        row_height: f32,
    ) -> Result<PropertyGridScope, DataError> {
        let width = crate::widgets::available_width(self.ctx);
        if !label_width.is_finite()
            || label_width <= 0.
            || !row_height.is_finite()
            || !(1. ..=10_000.).contains(&row_height)
        {
            return Err(DataError::InvalidInput);
        }
        let id = self.widget_id(label);
        self.ctx.id_stack.push_id(id);
        Ok(PropertyGridScope {
            width,
            label_width: label_width.min(width * 0.7),
            row_height,
        })
    }
    pub fn begin_property_row(&mut self, grid: &PropertyGridScope, label: &str) -> DataCellScope {
        let previous_left = self.ctx.prev_mouse_down[0];
        let parent_clip = self.ctx.interaction_clip;
        let rect = self.allocate([grid.width, grid.row_height]);
        let style = self.ctx.style;
        let id = self.widget_id(label);
        self.ctx.id_stack.push_id(id);
        let label_clip = [rect[0], rect[1], rect[0] + grid.label_width, rect[3]];
        crate::widgets::draw_parts(self.ctx)
            .1
            .push_clip_rect(label_clip);
        self.draw_text_at(
            [rect[0], rect[1] + style.frame_padding[1]],
            crate::id::split_label(label).0,
            style.text,
        );
        crate::widgets::draw_parts(self.ctx).1.pop_clip_rect();
        let cell = self.data_cell(
            [
                (rect[0] + grid.label_width + style.item_spacing[0]).min(rect[2]),
                rect[1],
                rect[2],
                rect[3],
            ],
            rect,
            0,
            0,
        );
        if !crate::ui::point_in_rect(self.mouse_pos(), rect)
            && parent_clip.is_none_or(|clip| crate::ui::point_in_rect(self.mouse_pos(), clip))
        {
            self.ctx.prev_mouse_down[0] = previous_left;
        }
        cell
    }
    pub fn end_property_row(&mut self, scope: DataCellScope) {
        self.end_data_cell(scope);
        self.ctx.id_stack.pop();
    }
    pub fn end_property_grid(&mut self, _grid: PropertyGridScope) {
        self.ctx.id_stack.pop();
    }
    pub fn property_grid(
        &mut self,
        label: &str,
        label_width: f32,
        row_height: f32,
        f: impl FnOnce(&mut Ui, &PropertyGridScope),
    ) -> Result<(), DataError> {
        let grid = self.begin_property_grid(label, label_width, row_height)?;
        f(self, &grid);
        self.end_property_grid(grid);
        Ok(())
    }
    pub fn property_row(&mut self, grid: &PropertyGridScope, label: &str, f: impl FnOnce(&mut Ui)) {
        let row = self.begin_property_row(grid, label);
        f(self);
        self.end_property_row(row);
    }
}
