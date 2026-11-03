use crate::context::Context;
use crate::draw::intersect_rects;
use crate::layout::Cursor;
use crate::scroll::ScrollFlags;
use crate::ui::{ChildScope, Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

pub struct TableScope {
    child: ChildScope,
    rect: [f32; 4],
    columns: usize,
    rows: usize,
}

impl TableScope {
    pub fn columns(&self) -> usize {
        self.columns
    }
    pub fn rows(&self) -> usize {
        self.rows
    }
}

pub struct SplitScope {
    child: ChildScope,
    panes: [[f32; 4]; 2],
}

pub struct PanelCellScope {
    cursor: Cursor,
    clip: Option<[f32; 4]>,
    prev_left: bool,
    popup: bool,
}

pub struct ResizableChildScope {
    child: ChildScope,
    grip: [f32; 4],
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockZone {
    Left = 0,
    Right = 1,
    Floating = 2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockState {
    pub zone: DockZone,
    pub float_pos: [f32; 2],
}

impl Default for DockState {
    fn default() -> Self {
        Self {
            zone: DockZone::Left,
            float_pos: [24.0, 24.0],
        }
    }
}

pub struct DockspaceScope {
    child: ChildScope,
    rect: [f32; 4],
    left_taken: bool,
    right_taken: bool,
}

pub struct DockPanelScope {
    child: ChildScope,
    cursor: Cursor,
    popup: bool,
}

fn cursor(ctx: &mut Context) -> &mut Cursor {
    if ctx.popup_scope_active {
        &mut ctx.overlay_cursor
    } else {
        &mut ctx.current_window_mut().cursor
    }
}

fn point_inside(point: [f32; 2], rect: [f32; 4]) -> bool {
    point[0] >= rect[0] && point[0] < rect[2] && point[1] >= rect[1] && point[1] < rect[3]
}

fn dock_rect(workspace: [f32; 4], floating_size: [f32; 2], placement: &DockState) -> [f32; 4] {
    let width = (workspace[2] - workspace[0]).max(0.0);
    let height = (workspace[3] - workspace[1]).max(0.0);
    let float_w = floating_size[0].max(60.0).min(width);
    let float_h = floating_size[1].max(40.0).min(height);
    match placement.zone {
        DockZone::Left => [
            workspace[0],
            workspace[1],
            workspace[0] + width * 0.5,
            workspace[3],
        ],
        DockZone::Right => [
            workspace[0] + width * 0.5,
            workspace[1],
            workspace[2],
            workspace[3],
        ],
        DockZone::Floating => {
            let x = workspace[0] + placement.float_pos[0].clamp(0.0, (width - float_w).max(0.0));
            let y = workspace[1] + placement.float_pos[1].clamp(0.0, (height - float_h).max(0.0));
            [x, y, x + float_w, y + float_h]
        }
    }
}

pub(crate) fn begin_cell(
    ui: &mut Ui<'_>,
    rect: [f32; 4],
    group: usize,
    index: usize,
) -> PanelCellScope {
    let ctx = &mut *ui.ctx;
    let group_id = ctx.id_stack.id_index(group);
    ctx.id_stack.push_id(group_id);
    let cell_id = ctx.id_stack.id_index(index);
    ctx.id_stack.push_id(cell_id);
    let popup = ctx.popup_scope_active;
    let saved = *cursor(ctx);
    let clip = ctx.interaction_clip;
    let prev_left = ctx.prev_mouse_down[0];
    let visible = clip.map_or(rect, |outer| intersect_rects(outer, rect));
    ctx.interaction_clip = Some(visible);
    if !point_inside(ctx.input.mouse_pos, visible) {
        ctx.prev_mouse_down[0] = true;
    }
    crate::widgets::draw_parts(ctx).1.push_clip_rect(rect);
    cursor(ctx).enter_region([rect[0], rect[1]], (rect[2] - rect[0]).max(0.0));
    PanelCellScope {
        cursor: saved,
        clip,
        prev_left,
        popup,
    }
}

impl Ui<'_> {
    pub fn table(
        &mut self,
        label: &str,
        size: [f32; 2],
        columns: usize,
        rows: usize,
        f: impl FnOnce(&mut Ui, &TableScope),
    ) {
        let scope = self.begin_table(label, size, columns, rows);
        f(self, &scope);
        self.end_table(scope);
    }

    pub fn table_cell(
        &mut self,
        table: &TableScope,
        row: usize,
        column: usize,
        f: impl FnOnce(&mut Ui),
    ) {
        let scope = self.begin_table_cell(table, row, column);
        f(self);
        self.end_panel_cell(scope);
    }

    pub fn begin_table(
        &mut self,
        label: &str,
        size: [f32; 2],
        columns: usize,
        rows: usize,
    ) -> TableScope {
        assert!(columns > 0 && rows > 0, "table dimensions must be positive");
        let child = self.begin_child(label, size, ScrollFlags::NONE);
        let rect = child.content_rect;
        let line = self.ctx.style.widget_bg_hovered;
        for column in 1..columns {
            let x = rect[0] + (rect[2] - rect[0]) * column as f32 / columns as f32;
            self.draw_line([x, rect[1]], [x, rect[3]], line, 1.0);
        }
        for row in 1..rows {
            let y = rect[1] + (rect[3] - rect[1]) * row as f32 / rows as f32;
            self.draw_line([rect[0], y], [rect[2], y], line, 1.0);
        }
        TableScope {
            child,
            rect,
            columns,
            rows,
        }
    }

    pub fn begin_table_cell(
        &mut self,
        table: &TableScope,
        row: usize,
        column: usize,
    ) -> PanelCellScope {
        assert!(
            row < table.rows && column < table.columns,
            "table cell outside table bounds"
        );
        let width = (table.rect[2] - table.rect[0]) / table.columns as f32;
        let height = (table.rect[3] - table.rect[1]) / table.rows as f32;
        let x = table.rect[0] + column as f32 * width;
        let y = table.rect[1] + row as f32 * height;
        begin_cell(self, [x, y, x + width, y + height], row, column)
    }

    pub fn end_panel_cell(&mut self, scope: PanelCellScope) {
        let ctx = &mut *self.ctx;
        *cursor(ctx) = scope.cursor;
        crate::widgets::draw_parts(ctx).1.pop_clip_rect();
        ctx.interaction_clip = scope.clip;
        ctx.prev_mouse_down[0] = scope.prev_left;
        ctx.id_stack.pop();
        ctx.id_stack.pop();
        assert_eq!(
            ctx.popup_scope_active, scope.popup,
            "panel cell crossed popup scope"
        );
    }

    pub fn end_table(&mut self, scope: TableScope) {
        self.end_child(scope.child);
    }

    pub fn split(
        &mut self,
        label: &str,
        size: [f32; 2],
        axis: SplitAxis,
        ratio: &mut f32,
        f: impl FnOnce(&mut Ui, &SplitScope),
    ) {
        let scope = self.begin_split(label, size, axis, ratio);
        f(self, &scope);
        self.end_split(scope);
    }

    pub fn split_pane(&mut self, split: &SplitScope, index: usize, f: impl FnOnce(&mut Ui)) {
        let scope = self.begin_split_pane(split, index);
        f(self);
        self.end_panel_cell(scope);
    }

    pub fn begin_split(
        &mut self,
        label: &str,
        size: [f32; 2],
        axis: SplitAxis,
        ratio: &mut f32,
    ) -> SplitScope {
        let id = self.widget_id(label);
        let child = self.begin_child(label, size, ScrollFlags::NONE);
        let rect = child.content_rect;
        let dimension = if axis == SplitAxis::Horizontal { 0 } else { 1 };
        let span = (rect[dimension + 2] - rect[dimension]).max(0.0);
        let divider = 6.0_f32.min(span);
        let usable = (span - divider).max(0.0);
        if !ratio.is_finite() {
            *ratio = 0.5;
        }
        *ratio = ratio.clamp(0.0, 1.0);
        let drag_id = {
            self.ctx.id_stack.push_id(id);
            let drag_id = self.ctx.id_stack.id("##split_divider");
            self.ctx.id_stack.pop();
            drag_id
        };
        let mut center = rect[dimension] + usable * *ratio + divider * 0.5;
        let mut hit = rect;
        hit[dimension] = center - divider * 0.5;
        hit[dimension + 2] = center + divider * 0.5;
        if self.is_hovered(hit) && self.mouse_pressed(0) {
            self.capture(drag_id);
            self.ctx.active_drag_offset[dimension] = self.mouse_pos()[dimension] - center;
        }
        if self.is_captured(drag_id) {
            if self.mouse_down(0) && usable > 0.0 {
                center = self.mouse_pos()[dimension] - self.ctx.active_drag_offset[dimension];
                *ratio = ((center - rect[dimension] - divider * 0.5) / usable).clamp(0.0, 1.0);
            } else {
                self.release_capture(drag_id);
            }
        }
        center = rect[dimension] + usable * *ratio + divider * 0.5;
        hit[dimension] = center - divider * 0.5;
        hit[dimension + 2] = center + divider * 0.5;
        let mut first = rect;
        let mut second = rect;
        first[dimension + 2] = hit[dimension];
        second[dimension] = hit[dimension + 2];
        let divider_color = self.ctx.style.widget_bg_hovered;
        self.draw_rect_filled([hit[0], hit[1]], [hit[2], hit[3]], divider_color);
        SplitScope {
            child,
            panes: [first, second],
        }
    }

    pub fn begin_split_pane(&mut self, split: &SplitScope, index: usize) -> PanelCellScope {
        assert!(index < 2, "split pane index must be 0 or 1");
        begin_cell(self, split.panes[index], 0, index)
    }

    pub fn end_split(&mut self, scope: SplitScope) {
        self.end_child(scope.child);
    }

    pub fn begin_resizable_child(
        &mut self,
        label: &str,
        size: &mut [f32; 2],
        min_size: [f32; 2],
        flags: ScrollFlags,
    ) -> ResizableChildScope {
        size[0] = size[0].max(min_size[0].max(24.0));
        size[1] = size[1].max(min_size[1].max(24.0));
        let id = self.widget_id(label);
        let saved = *cursor(self.ctx);
        let old_rect = crate::widgets::item_rect(self.ctx, *size);
        *cursor(self.ctx) = saved;
        let grip = [
            old_rect[2] - 14.0,
            old_rect[3] - 14.0,
            old_rect[2],
            old_rect[3],
        ];
        let grip_id = {
            self.ctx.id_stack.push_id(id);
            let grip_id = self.ctx.id_stack.id("##child_resize");
            self.ctx.id_stack.pop();
            grip_id
        };
        if self.is_hovered(grip) && self.mouse_pressed(0) {
            self.capture(grip_id);
            let mouse = self.mouse_pos();
            self.ctx.active_drag_offset = [mouse[0] - old_rect[2], mouse[1] - old_rect[3]];
        }
        if self.is_captured(grip_id) {
            if self.mouse_down(0) {
                let mouse = self.mouse_pos();
                size[0] = (mouse[0] - self.ctx.active_drag_offset[0] - old_rect[0])
                    .max(min_size[0].max(24.0));
                size[1] = (mouse[1] - self.ctx.active_drag_offset[1] - old_rect[1])
                    .max(min_size[1].max(24.0));
            } else {
                self.release_capture(grip_id);
            }
        }
        let child = self.begin_child(label, *size, flags);
        if self.is_captured(grip_id) {
            self.ctx.prev_mouse_down[0] = true;
        }
        let rect = [
            old_rect[0],
            old_rect[1],
            old_rect[0] + size[0],
            old_rect[1] + size[1],
        ];
        ResizableChildScope {
            child,
            grip: [rect[2] - 14.0, rect[3] - 14.0, rect[2], rect[3]],
        }
    }

    pub fn end_resizable_child(&mut self, scope: ResizableChildScope) {
        self.end_child(scope.child);
        let col = self.ctx.style.text;
        let r = scope.grip;
        self.draw_line(
            [r[2] - 10.0, r[3] - 3.0],
            [r[2] - 3.0, r[3] - 10.0],
            col,
            1.0,
        );
        self.draw_line([r[2] - 6.0, r[3] - 3.0], [r[2] - 3.0, r[3] - 6.0], col, 1.0);
    }

    pub fn resizable_child(
        &mut self,
        label: &str,
        size: &mut [f32; 2],
        min_size: [f32; 2],
        flags: ScrollFlags,
        f: impl FnOnce(&mut Ui),
    ) {
        let scope = self.begin_resizable_child(label, size, min_size, flags);
        f(self);
        self.end_resizable_child(scope);
    }

    pub fn begin_dockspace(&mut self, label: &str, size: [f32; 2]) -> DockspaceScope {
        let child = self.begin_child(label, size, ScrollFlags::NONE);
        DockspaceScope {
            rect: child.content_rect,
            child,
            left_taken: false,
            right_taken: false,
        }
    }

    pub fn end_dockspace(&mut self, scope: DockspaceScope) {
        self.end_child(scope.child);
    }

    pub fn dockspace(
        &mut self,
        label: &str,
        size: [f32; 2],
        f: impl FnOnce(&mut Ui, &mut DockspaceScope),
    ) {
        let mut scope = self.begin_dockspace(label, size);
        f(self, &mut scope);
        self.end_dockspace(scope);
    }

    pub fn dock_panel(
        &mut self,
        dock: &mut DockspaceScope,
        label: &str,
        floating_size: [f32; 2],
        placement: &mut DockState,
        f: impl FnOnce(&mut Ui),
    ) {
        let scope = self.begin_dock_panel(dock, label, floating_size, placement);
        f(self);
        self.end_dock_panel(scope);
    }

    pub fn begin_dock_panel(
        &mut self,
        dock: &mut DockspaceScope,
        label: &str,
        floating_size: [f32; 2],
        placement: &mut DockState,
    ) -> DockPanelScope {
        let workspace = dock.rect;
        let width = (workspace[2] - workspace[0]).max(0.0);
        let height = (workspace[3] - workspace[1]).max(0.0);
        let float_w = floating_size[0].max(60.0).min(width);
        let float_h = floating_size[1].max(40.0).min(height);
        if (placement.zone == DockZone::Left && dock.left_taken)
            || (placement.zone == DockZone::Right && dock.right_taken)
        {
            placement.zone = DockZone::Floating;
        }
        let panel = dock_rect(workspace, floating_size, placement);
        let id = self.widget_id(label);
        let header_id = {
            self.ctx.id_stack.push_id(id);
            let result = self.ctx.id_stack.id("##dock_header");
            self.ctx.id_stack.pop();
            result
        };
        let header = [
            panel[0],
            panel[1],
            panel[2],
            (panel[1] + 24.0).min(panel[3]),
        ];
        if self.is_hovered(header) && self.mouse_pressed(0) {
            self.capture(header_id);
            let mouse = self.mouse_pos();
            self.ctx.active_drag_offset = [mouse[0] - panel[0], mouse[1] - panel[1]];
        }
        if self.is_captured(header_id) {
            let mouse = self.mouse_pos();
            if self.mouse_down(0) {
                placement.zone = DockZone::Floating;
                placement.float_pos = [
                    (mouse[0] - self.ctx.active_drag_offset[0] - workspace[0])
                        .clamp(0.0, (width - float_w).max(0.0)),
                    (mouse[1] - self.ctx.active_drag_offset[1] - workspace[1])
                        .clamp(0.0, (height - float_h).max(0.0)),
                ];
            } else {
                if point_inside(mouse, workspace) {
                    let relative_x = (mouse[0] - workspace[0]) / width.max(1.0);
                    if relative_x < 0.33 && !dock.left_taken {
                        placement.zone = DockZone::Left;
                    } else if relative_x > 0.67 && !dock.right_taken {
                        placement.zone = DockZone::Right;
                    }
                }
                self.release_capture(header_id);
            }
        }
        match placement.zone {
            DockZone::Left => dock.left_taken = true,
            DockZone::Right => dock.right_taken = true,
            DockZone::Floating => {}
        }
        let panel = dock_rect(workspace, floating_size, placement);
        let header = [
            panel[0],
            panel[1],
            panel[2],
            (panel[1] + 24.0).min(panel[3]),
        ];
        let popup = self.ctx.popup_scope_active;
        let saved = *cursor(self.ctx);
        cursor(self.ctx).enter_region([panel[0], panel[1]], panel[2] - panel[0]);
        let child = self.begin_child(
            label,
            [panel[2] - panel[0], panel[3] - panel[1]],
            ScrollFlags::NONE,
        );
        let header_color = self.ctx.style.widget_bg_hovered;
        let text_color = self.ctx.style.text;
        self.draw_rect_filled([header[0], header[1]], [header[2], header[3]], header_color);
        self.draw_text_at(
            [header[0] + 6.0, header[1] + 3.0],
            crate::id::split_label(label).0,
            text_color,
        );
        self.allocate([1.0, 24.0]);
        DockPanelScope {
            child,
            cursor: saved,
            popup,
        }
    }

    pub fn end_dock_panel(&mut self, scope: DockPanelScope) {
        self.end_child(scope.child);
        assert_eq!(
            self.ctx.popup_scope_active, scope.popup,
            "dock panel crossed popup scope"
        );
        *cursor(self.ctx) = scope.cursor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::input::InputState;
    use alloc::vec::Vec;

    fn context() -> Context {
        Context::new(include_bytes!("../tests/assets/DejaVuSans.ttf")).expect("font loads")
    }

    fn table_frame(
        closure: bool,
    ) -> (
        Vec<crate::draw::DrawVert>,
        Vec<u32>,
        Vec<crate::draw::DrawCmd>,
    ) {
        let mut ctx = context();
        ctx.new_frame(0.016);
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            if closure {
                ui.table("T", [200.0, 80.0], 2, 1, |ui, table| {
                    ui.table_cell(table, 0, 0, |ui| ui.label("one"));
                    ui.table_cell(table, 0, 1, |ui| ui.label("two"));
                });
            } else {
                let table = ui.begin_table("T", [200.0, 80.0], 2, 1);
                let first = ui.begin_table_cell(&table, 0, 0);
                ui.label("one");
                ui.end_panel_cell(first);
                let second = ui.begin_table_cell(&table, 0, 1);
                ui.label("two");
                ui.end_panel_cell(second);
                ui.end_table(table);
            }
        });
        let draw = ctx.render();
        (
            draw.vertices.to_vec(),
            draw.indices.to_vec(),
            draw.commands.to_vec(),
        )
    }

    #[test]
    fn table_closure_and_pair_emit_identical_geometry() {
        assert_eq!(table_frame(true), table_frame(false));
    }

    #[test]
    fn table_cells_have_distinct_widget_ids_and_clipped_layout() {
        let mut ctx = context();
        ctx.new_frame(0.016);
        let mut ids = [crate::id::Id(0); 2];
        let mut rects = [[0.0; 4]; 2];
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            let table = ui.begin_table("grid", [200.0, 80.0], 2, 1);
            for column in 0..2 {
                let cell = ui.begin_table_cell(&table, 0, column);
                ids[column] = ui.widget_id("repeated");
                rects[column] = ui.allocate([20.0, 10.0]);
                ui.end_panel_cell(cell);
            }
            ui.end_table(table);
        });
        assert_ne!(ids[0], ids[1]);
        assert!(rects[0][2] <= rects[1][0]);
        ctx.render();
    }

    #[test]
    fn splitter_drag_updates_host_ratio() {
        let mut ctx = context();
        ctx.set_window_rect("W", [20.0, 20.0], [500.0, 360.0]);
        let mut ratio = 0.5;
        let mut divider_x = 0.0;
        ctx.new_frame(0.016);
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            let split = ui.begin_split("s", [300.0, 100.0], SplitAxis::Horizontal, &mut ratio);
            divider_x = split.panes[0][2] + 3.0;
            ui.end_split(split);
        });
        ctx.render();
        let y = 85.0;
        ctx.set_input(InputState {
            mouse_pos: [divider_x, y],
            mouse_down: [true, false, false, false, false],
            ..InputState::default()
        });
        ctx.new_frame(0.016);
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            let split = ui.begin_split("s", [300.0, 100.0], SplitAxis::Horizontal, &mut ratio);
            ui.end_split(split);
        });
        ctx.render();
        ctx.set_input(InputState {
            mouse_pos: [divider_x + 40.0, y],
            mouse_down: [true, false, false, false, false],
            ..InputState::default()
        });
        ctx.new_frame(0.016);
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            let split = ui.begin_split("s", [300.0, 100.0], SplitAxis::Horizontal, &mut ratio);
            ui.end_split(split);
        });
        assert!(ratio > 0.5);
        ctx.render();
    }

    #[test]
    fn occupied_dock_lane_moves_later_panel_to_floating() {
        let mut ctx = context();
        ctx.new_frame(0.016);
        let mut first = DockState::default();
        let mut second = DockState::default();
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            let mut dock = ui.begin_dockspace("dock", [300.0, 160.0]);
            let a = ui.begin_dock_panel(&mut dock, "A", [100.0, 80.0], &mut first);
            ui.end_dock_panel(a);
            let b = ui.begin_dock_panel(&mut dock, "B", [100.0, 80.0], &mut second);
            ui.end_dock_panel(b);
            ui.end_dockspace(dock);
        });
        assert_eq!(first.zone, DockZone::Left);
        assert_eq!(second.zone, DockZone::Floating);
        ctx.render();
    }

    #[test]
    fn child_grip_drag_edits_host_size() {
        let mut ctx = context();
        ctx.set_window_rect("W", [20.0, 20.0], [500.0, 360.0]);
        let mut size = [180.0, 100.0];
        let mut grip = [0.0; 4];
        ctx.new_frame(0.016);
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            let child =
                ui.begin_resizable_child("panel", &mut size, [100.0, 80.0], ScrollFlags::NONE);
            grip = child.grip;
            ui.end_resizable_child(child);
        });
        ctx.render();
        let press = [grip[2] - 5.0, grip[3] - 5.0];
        ctx.set_input(InputState {
            mouse_pos: press,
            mouse_down: [true, false, false, false, false],
            ..InputState::default()
        });
        ctx.new_frame(0.016);
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            let child =
                ui.begin_resizable_child("panel", &mut size, [100.0, 80.0], ScrollFlags::NONE);
            ui.end_resizable_child(child);
        });
        ctx.render();
        ctx.set_input(InputState {
            mouse_pos: [press[0] + 30.0, press[1] + 20.0],
            mouse_down: [true, false, false, false, false],
            ..InputState::default()
        });
        ctx.new_frame(0.016);
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            let child =
                ui.begin_resizable_child("panel", &mut size, [100.0, 80.0], ScrollFlags::NONE);
            ui.end_resizable_child(child);
        });
        assert!(size[0] > 180.0 && size[1] > 100.0);
        ctx.render();
    }
}
