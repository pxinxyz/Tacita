use super::{
    resolve_layout_with, LayoutError, LayoutGeometry, LayoutItem, LayoutSpec, MAX_LAYOUT_ITEMS,
};
use crate::draw::PaintRange;
use crate::panels::PanelCellScope;
use crate::ui::Ui;

pub struct LayoutScope {
    geometry: LayoutGeometry,
    rect: [f32; 4],
    region: PanelCellScope,
    visited: u64,
    open_item: bool,
    ranges: [PaintRange; MAX_LAYOUT_ITEMS],
    range_count: usize,
    paint_start: [usize; 2],
}
impl LayoutScope {
    pub fn rect(&self) -> [f32; 4] {
        self.rect
    }
    pub fn len(&self) -> usize {
        self.geometry.len()
    }
    pub fn is_empty(&self) -> bool {
        self.geometry.is_empty()
    }
    pub fn item_rect(&self, index: usize) -> Option<[f32; 4]> {
        self.geometry.rect(index)
    }
}
pub struct LayoutItemScope {
    region: PanelCellScope,
    index: usize,
    start: [usize; 2],
}

impl Ui<'_> {
    pub fn available_size(&self) -> [f32; 2] {
        let cursor = if self.ctx.popup_scope_active {
            &self.ctx.overlay_cursor
        } else {
            &self.ctx.windows[self.ctx.window_index(
                self.ctx
                    .current_window
                    .expect("layout query needs a window"),
            )]
            .cursor
        };
        let height = self
            .ctx
            .interaction_clip
            .map_or(0.0, |r| (r[3] - cursor.position()[1]).max(0.0));
        [cursor.available_width().max(0.0), height]
    }
    pub fn layout(
        &mut self,
        label: &str,
        spec: LayoutSpec,
        items: &[LayoutItem],
        body: impl FnOnce(&mut Ui, &mut LayoutScope),
    ) -> Result<(), LayoutError> {
        let mut scope = self.begin_layout(label, spec, items)?;
        body(self, &mut scope);
        self.end_layout(scope);
        Ok(())
    }
    pub fn begin_layout(
        &mut self,
        label: &str,
        spec: LayoutSpec,
        items: &[LayoutItem],
    ) -> Result<LayoutScope, LayoutError> {
        self.begin_layout_with(label, spec, items.len(), |i| items[i])
    }
    pub fn begin_layout_with(
        &mut self,
        label: &str,
        spec: LayoutSpec,
        count: usize,
        get: impl Fn(usize) -> LayoutItem,
    ) -> Result<LayoutScope, LayoutError> {
        self.begin_resolved_layout(label, resolve_layout_with(spec, count, get)?)
    }
    fn begin_resolved_layout(
        &mut self,
        label: &str,
        mut geometry: LayoutGeometry,
    ) -> Result<LayoutScope, LayoutError> {
        let count = geometry.len();
        let rect = self.allocate(geometry.size);
        for item in &mut geometry.rects[..count] {
            item[0] += rect[0];
            item[2] += rect[0];
            item[1] += rect[1];
            item[3] += rect[1];
        }
        let id = self.widget_id(label);
        self.ctx.id_stack.push_id(id);
        let region = crate::panels::begin_cell(self, rect, 0x4c41594f, 0);
        let mark = crate::widgets::draw_parts(self.ctx).1.paint_mark();
        Ok(LayoutScope {
            geometry,
            rect,
            region,
            visited: 0,
            open_item: false,
            ranges: [PaintRange::default(); MAX_LAYOUT_ITEMS],
            range_count: 0,
            paint_start: mark,
        })
    }
    pub fn begin_layout_extended(
        &mut self,
        label: &str,
        spec: LayoutSpec,
        items: &[LayoutItem],
        options: super::LayoutOptions,
        cells: &[super::GridCell],
    ) -> Result<LayoutScope, LayoutError> {
        self.begin_resolved_layout(
            label,
            super::resolve_layout_extended(spec, items, options, cells)?,
        )
    }
    pub fn begin_layout_extended_with(
        &mut self,
        label: &str,
        spec: LayoutSpec,
        options: super::LayoutOptions,
        count: usize,
        get: impl Fn(usize) -> (LayoutItem, super::GridCell),
    ) -> Result<LayoutScope, LayoutError> {
        self.begin_resolved_layout(
            label,
            super::resolve_layout_extended_with(spec, options, count, get)?,
        )
    }
    pub fn layout_extended(
        &mut self,
        label: &str,
        spec: LayoutSpec,
        items: &[LayoutItem],
        options: super::LayoutOptions,
        cells: &[super::GridCell],
        body: impl FnOnce(&mut Ui, &mut LayoutScope),
    ) -> Result<(), LayoutError> {
        let mut scope = self.begin_layout_extended(label, spec, items, options, cells)?;
        body(self, &mut scope);
        self.end_layout(scope);
        Ok(())
    }
    pub fn layout_item(
        &mut self,
        layout: &mut LayoutScope,
        index: usize,
        body: impl FnOnce(&mut Ui, [f32; 4]),
    ) -> Result<(), LayoutError> {
        let scope = self.begin_layout_item(layout, index)?;
        body(self, layout.geometry.rects[index]);
        self.end_layout_item(layout, scope);
        Ok(())
    }
    pub fn begin_layout_item(
        &mut self,
        layout: &mut LayoutScope,
        index: usize,
    ) -> Result<LayoutItemScope, LayoutError> {
        if index >= layout.len() {
            return Err(LayoutError::InvalidIndex);
        }
        if layout.visited & (1 << index) != 0 {
            return Err(LayoutError::DuplicateItem);
        }
        assert!(!layout.open_item, "close the previous layout item first");
        if layout.visited != 0 && index <= 63 - layout.visited.leading_zeros() as usize {
            return Err(LayoutError::InvalidIndex);
        }
        let start = crate::widgets::draw_parts(self.ctx).1.paint_mark();
        let rect = layout.geometry.rects[index];
        let region = crate::panels::begin_cell(self, rect, 0x4954454d, index);
        let mouse = self.ctx.input.mouse_pos;
        let inside = |r: [f32; 4]| {
            mouse[0] >= r[0] && mouse[0] < r[2] && mouse[1] >= r[1] && mouse[1] < r[3]
        };
        let blocked = !layout.geometry.hit_test[index]
            || (0..layout.len()).any(|other| {
                layout.geometry.hit_test[other]
                    && (layout.geometry.layers[other], other)
                        > (layout.geometry.layers[index], index)
                    && inside(layout.geometry.rects[other])
            });
        if blocked {
            self.ctx.interaction_clip = Some([0.0; 4]);
            self.ctx.prev_mouse_down[0] = true;
        }
        layout.open_item = true;
        layout.visited |= 1 << index;
        Ok(LayoutItemScope {
            region,
            index,
            start,
        })
    }
    pub fn end_layout_item(&mut self, layout: &mut LayoutScope, scope: LayoutItemScope) {
        assert!(layout.open_item, "no layout item open");
        self.end_panel_cell(scope.region);
        let end = crate::widgets::draw_parts(self.ctx).1.paint_mark();
        layout.ranges[layout.range_count] = PaintRange {
            start: scope.start,
            end,
            layer: layout.geometry.layers[scope.index],
        };
        layout.range_count += 1;
        layout.open_item = false;
    }
    pub fn end_layout(&mut self, scope: LayoutScope) {
        assert!(!scope.open_item, "close layout item before layout");
        let draw = crate::widgets::draw_parts(self.ctx).1;
        let end = draw.paint_mark();
        draw.order_paint(&scope.ranges[..scope.range_count], scope.paint_start, end);
        self.end_panel_cell(scope.region);
        self.ctx.id_stack.pop();
    }
}
