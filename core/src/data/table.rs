use super::{click, DataCellScope, DataError, VisibleRows, MAX_DATA_COLUMNS, MAX_DATA_ROWS};
use crate::{
    id::Id,
    ui::{ChildScope, Ui},
    ScrollFlags,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortSpec {
    pub column: usize,
    pub descending: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{context::Context, input::InputState};
    const FONT: &[u8] = include_bytes!("../../tests/assets/DejaVuSans.ttf");

    #[test]
    fn resizing_survives_leaving_table_and_does_not_trigger_sort() {
        let mut ctx = Context::new(FONT).unwrap();
        ctx.set_window_rect("W", [0., 0.], [600., 500.]);
        let mut state = DataTableState::default();
        let frame = |ctx: &mut Context, state: &mut DataTableState, point: [f32; 2], down: bool| {
            ctx.set_input(InputState {
                mouse_pos: point,
                mouse_down: [down, false, false, false, false],
                ..InputState::default()
            });
            ctx.new_frame(0.016);
            let mut rect = [0.; 4];
            ctx.ui().window("W", ScrollFlags::NONE, |ui| {
                let table = ui
                    .begin_data_table(
                        "T",
                        &["First", "Second", "Third"],
                        30,
                        state,
                        DataTableOptions {
                            size: [300., 140.],
                            row_height: 28.,
                            frozen_columns: 1,
                            frozen_rows: 0,
                        },
                    )
                    .unwrap();
                rect = table.rect;
                ui.end_data_table(table);
            });
            ctx.render();
            rect
        };
        let rect = frame(&mut ctx, &mut state, [0., 0.], false);
        let edge = [rect[0] + 140., rect[1] + 10.];
        frame(&mut ctx, &mut state, edge, true);
        assert!(ctx.active.is_some());
        frame(&mut ctx, &mut state, [edge[0] + 40., rect[3] + 50.], true);
        assert_eq!(state.widths[0], 180.);
        assert_eq!(state.sort, None);
        frame(&mut ctx, &mut state, [edge[0] + 40., rect[3] + 50.], false);
        assert_eq!(state.widths[0], 180.);
        assert_eq!(ctx.active, None);
    }

    #[test]
    fn horizontal_bar_returns_offset_and_retains_drag_outside_viewport() {
        let mut ctx = Context::new(FONT).unwrap();
        ctx.set_window_rect("W", [0., 0.], [600., 500.]);
        let mut state = DataTableState::default();
        let frame = |ctx: &mut Context, state: &mut DataTableState, point: [f32; 2], down: bool| {
            ctx.set_input(InputState {
                mouse_pos: point,
                mouse_down: [down, false, false, false, false],
                ..InputState::default()
            });
            ctx.new_frame(0.016);
            let mut track = [0.; 4];
            ctx.ui().window("W", ScrollFlags::NONE, |ui| {
                let table = ui
                    .begin_data_table(
                        "T",
                        &["ID", "Name", "Kind"],
                        30,
                        state,
                        DataTableOptions {
                            size: [300., 140.],
                            ..DataTableOptions::default()
                        },
                    )
                    .unwrap();
                track = table.horizontal_track.unwrap();
                state.scroll_x = ui.end_data_table(table);
            });
            ctx.render();
            track
        };
        let track = frame(&mut ctx, &mut state, [0., 0.], false);
        let thumb = [track[0] + 4., (track[1] + track[3]) * 0.5];
        frame(&mut ctx, &mut state, thumb, true);
        assert!(ctx.active.is_some());
        frame(&mut ctx, &mut state, [thumb[0] + 40., track[3] + 40.], true);
        assert!(state.scroll_x > 0.);
        let offset = state.scroll_x;
        frame(
            &mut ctx,
            &mut state,
            [thumb[0] + 40., track[3] + 40.],
            false,
        );
        assert_eq!(state.scroll_x, offset);
        assert_eq!(ctx.active, None);
        assert_eq!(state.sort, None);
    }

    #[test]
    fn frozen_columns_clip_scrolling_cells_and_invalid_begins_leave_state_intact() {
        let mut ctx = Context::new(FONT).unwrap();
        ctx.set_window_rect("W", [0., 0.], [600., 500.]);
        ctx.set_input(InputState::default());
        ctx.new_frame(0.016);
        let mut state = DataTableState {
            scroll_x: 70.,
            ..DataTableState::default()
        };
        ctx.ui().window("W", ScrollFlags::NONE, |ui| {
            let options = DataTableOptions {
                size: [300., 140.],
                row_height: 30.,
                frozen_columns: 1,
                frozen_rows: 1,
            };
            let table = ui
                .begin_data_table("table", &["ID", "Value", "Kind"], 100, &mut state, options)
                .unwrap();
            assert_eq!(table.column_x(0), table.rect[0]);
            assert!(table.column_x(1) < table.rect[0] + table.frozen_width);
            let cell = ui.begin_data_table_cell(&table, 1, 1).unwrap();
            let clip = ui.ctx.interaction_clip.unwrap();
            assert_eq!(clip[0], table.rect[0] + table.frozen_width);
            assert!(clip[1] >= table.rect[1] + table.header_height + 30.);
            ui.end_data_cell(cell);
            assert!(ui.begin_data_table_cell(&table, 99, 1).is_none());
            ui.end_data_table(table);
            let saved = state;
            let bad = DataTableOptions {
                frozen_columns: 4,
                ..options
            };
            assert!(ui
                .begin_data_table("bad", &["ID"], 100, &mut state, bad)
                .is_err());
            assert_eq!(state.widths, saved.widths);
            assert_eq!(state.scroll_x, saved.scroll_x);
        });
        ctx.render();
    }
}
#[derive(Clone, Copy, Debug)]
pub struct DataTableState {
    pub widths: [f32; MAX_DATA_COLUMNS],
    pub scroll_x: f32,
    pub sort: Option<SortSpec>,
}
impl Default for DataTableState {
    fn default() -> Self {
        Self {
            widths: [140.; MAX_DATA_COLUMNS],
            scroll_x: 0.,
            sort: None,
        }
    }
}
impl DataTableState {
    pub fn cycle_sort(&mut self, column: usize) {
        if column >= MAX_DATA_COLUMNS {
            return;
        }
        self.sort = match self.sort {
            Some(s) if s.column == column && s.descending => None,
            Some(s) if s.column == column => Some(SortSpec {
                column,
                descending: true,
            }),
            _ => Some(SortSpec {
                column,
                descending: false,
            }),
        };
    }
}
#[derive(Clone, Copy, Debug)]
pub struct DataTableOptions {
    pub size: [f32; 2],
    pub row_height: f32,
    pub frozen_columns: usize,
    pub frozen_rows: usize,
}
impl Default for DataTableOptions {
    fn default() -> Self {
        Self {
            size: [500., 240.],
            row_height: 28.,
            frozen_columns: 1,
            frozen_rows: 0,
        }
    }
}
impl DataTableOptions {
    pub fn is_valid(&self, columns: usize, rows: usize) -> bool {
        columns > 0
            && columns <= MAX_DATA_COLUMNS
            && rows <= MAX_DATA_ROWS
            && self.frozen_columns <= columns
            && self.frozen_rows <= rows
            && self
                .size
                .iter()
                .all(|v| v.is_finite() && *v >= 40. && *v <= 1_000_000.)
            && self.row_height.is_finite()
            && self.row_height >= 1.
            && self.row_height <= 10_000.
    }
}
pub struct DataTableScope {
    child: ChildScope,
    rect: [f32; 4],
    widths: [f32; MAX_DATA_COLUMNS],
    columns: usize,
    rows: usize,
    options: DataTableOptions,
    scroll: [f32; 2],
    frozen_width: f32,
    header_height: f32,
    visible: VisibleRows,
    horizontal_track: Option<[f32; 4]>,
    horizontal_id: Id,
    horizontal_range: f32,
    content_width: f32,
    visible_width: f32,
}
impl DataTableScope {
    pub fn visible_rows(&self) -> VisibleRows {
        self.visible
    }
    pub fn columns(&self) -> usize {
        self.columns
    }
    pub fn rows(&self) -> usize {
        self.rows
    }
    pub fn frozen_rows(&self) -> usize {
        self.options.frozen_rows
    }
    fn column_x(&self, column: usize) -> f32 {
        self.rect[0] + self.widths[..column].iter().sum::<f32>()
            - if column < self.options.frozen_columns {
                0.
            } else {
                self.scroll[0]
            }
    }
    fn column_clip(&self, column: usize) -> [f32; 4] {
        let boundary = (self.rect[0] + self.frozen_width).min(self.rect[2]);
        [
            if column < self.options.frozen_columns {
                self.rect[0]
            } else {
                boundary
            },
            self.rect[1],
            if column < self.options.frozen_columns {
                boundary
            } else {
                self.rect[2]
            },
            self.rect[3],
        ]
    }
}
impl Ui<'_> {
    pub fn begin_data_table(
        &mut self,
        label: &str,
        headers: &[&str],
        rows: usize,
        state: &mut DataTableState,
        options: DataTableOptions,
    ) -> Result<DataTableScope, DataError> {
        if !options.is_valid(headers.len(), rows)
            || !state.scroll_x.is_finite()
            || state.sort.is_some_and(|s| s.column >= headers.len())
            || !state.widths[..headers.len()]
                .iter()
                .all(|w| w.is_finite() && *w >= 32. && *w <= 10_000.)
        {
            return Err(DataError::InvalidInput);
        }
        let initial_state = *state;
        let child = self.begin_child(label, options.size, ScrollFlags::BAR);
        let mut rect = child.content_rect;
        let header_height = options.row_height.max(
            self.ctx.font.line_height_in(self.ctx.active_font())
                + 2. * self.ctx.style.frame_padding[1],
        );
        let mut frozen_width = state.widths[..options.frozen_columns].iter().sum::<f32>();
        for column in 0..headers.len() {
            let edge = rect[0] + state.widths[..=column].iter().sum::<f32>()
                - if column < options.frozen_columns {
                    0.
                } else {
                    state.scroll_x
                };
            let hit = [
                edge - 3.,
                rect[1],
                edge + 3.,
                (rect[1] + header_height).min(rect[3]),
            ];
            self.ctx.id_stack.push_id(Id(column as u64));
            let id = self.widget_id("##column_resize");
            self.ctx.id_stack.pop();
            let clip = if column < options.frozen_columns {
                [
                    rect[0],
                    rect[1],
                    (rect[0] + frozen_width + 3.).min(rect[2]),
                    rect[3],
                ]
            } else {
                [
                    (rect[0] + frozen_width).min(rect[2]),
                    rect[1],
                    rect[2],
                    rect[3],
                ]
            };
            crate::widgets::register_item(self.ctx, id, crate::draw::intersect_rects(hit, clip));
            if self.is_hovered(hit)
                && crate::ui::point_in_rect(self.mouse_pos(), clip)
                && (column < options.frozen_columns || edge > clip[0])
            {
                self.ctx.hot = Some(id);
            }
            if self.ctx.disabled_depth == 0
                && (column < options.frozen_columns || edge > clip[0])
                && crate::ui::hover_layer_gate(self.ctx)
                && crate::ui::point_in_rect(self.mouse_pos(), hit)
                && crate::ui::point_in_rect(self.mouse_pos(), clip)
                && self.mouse_pressed(0)
            {
                self.capture(id);
                self.ctx.active_drag_offset = [self.mouse_pos()[0], state.widths[column]];
            }
            if self.is_captured(id) {
                if self.mouse_down(0) {
                    state.widths[column] = (self.ctx.active_drag_offset[1] + self.mouse_pos()[0]
                        - self.ctx.active_drag_offset[0])
                        .clamp(32., 10_000.);
                } else {
                    self.release_capture(id);
                }
            }
        }
        frozen_width = state.widths[..options.frozen_columns].iter().sum::<f32>();
        let total_width = state.widths[..headers.len()].iter().sum::<f32>();
        let width = rect[2] - rect[0];
        let content_width = (total_width - frozen_width).max(0.);
        let enabled = options.frozen_columns < headers.len() && frozen_width < width;
        let scroll_rect = [
            (rect[0] + frozen_width).min(rect[2]),
            rect[1],
            rect[2],
            rect[3],
        ];
        let horizontal = crate::scroll::geometry_axis(
            &self.ctx.style,
            scroll_rect,
            0.,
            if enabled {
                ScrollFlags::BAR
            } else {
                ScrollFlags::NONE
            },
            content_width,
            0,
        );
        let horizontal_track = horizontal.scrollbar_rect;
        let horizontal_id = self.widget_id("##horizontal_scroll");
        let visible_width = (scroll_rect[2] - scroll_rect[0]).max(0.);
        let bar_height = rect[3] - horizontal.content_rect[3];
        rect[3] = horizontal.content_rect[3];
        state.scroll_x = state.scroll_x.clamp(0., horizontal.max_offset);
        let scroll = [state.scroll_x, child.scroll_offset()];
        self.allocate([
            rect[2] - rect[0],
            header_height + rows as f32 * options.row_height + bar_height,
        ]);
        let body_start =
            (rect[1] + header_height + options.frozen_rows as f32 * options.row_height)
                .min(rect[3]);
        let clip = crate::draw::intersect_rects(rect, self.ctx.interaction_clip.unwrap_or(rect));
        let visible = VisibleRows::new(
            rows,
            options.row_height,
            0.,
            rect[1] + header_height - scroll[1],
            [
                body_start.max(clip[1]),
                clip[3].max(body_start.max(clip[1])),
            ],
        );
        let mut visible = match visible {
            Ok(visible) => visible,
            Err(error) => {
                self.end_child(child);
                *state = initial_state;
                return Err(error);
            }
        };
        visible.start = visible.start.max(options.frozen_rows);
        visible.end = visible.end.max(visible.start);
        let table = DataTableScope {
            child,
            rect,
            widths: state.widths,
            columns: headers.len(),
            rows,
            options,
            scroll,
            frozen_width,
            header_height,
            visible,
            horizontal_track,
            horizontal_id,
            horizontal_range: horizontal.max_offset,
            content_width,
            visible_width,
        };
        for (column, header) in headers.iter().enumerate() {
            let x = table.column_x(column);
            let cell = [
                x,
                rect[1],
                x + table.widths[column],
                (rect[1] + header_height).min(rect[3]),
            ];
            let clip = table.column_clip(column);
            let shown = crate::draw::intersect_rects(cell, clip);
            if shown[2] <= shown[0] || shown[3] <= shown[1] {
                continue;
            }
            self.ctx.id_stack.push_id(Id(column as u64));
            let resize_id = self.widget_id("##column_resize");
            self.ctx.id_stack.pop();
            let scope = self.data_cell(cell, clip, usize::MAX, column);
            let id = self.widget_id("##sort");
            let hit = [cell[0] + 4., cell[1], cell[2] - 4., cell[3]];
            if click(self, id, hit) {
                state.cycle_sort(column);
            }
            let style = self.ctx.style;
            let bg = if state.sort.is_some_and(|s| s.column == column) {
                style.widget_bg_active
            } else if self.ctx.hot == Some(id) {
                style.widget_bg_hovered
            } else {
                style.widget_bg
            };
            let bg = self.ctx.widget_color(id, bg);
            crate::widgets::draw_parts(self.ctx).1.add_filled_rect(
                [cell[0], cell[1]],
                [cell[2], cell[3]],
                bg,
            );
            self.draw_text_at(
                [
                    cell[0] + style.frame_padding[0],
                    cell[1] + style.frame_padding[1],
                ],
                crate::id::split_label(header).0,
                style.text,
            );
            if let Some(sort) = state.sort.filter(|s| s.column == column) {
                self.draw_text_at(
                    [cell[2] - 20., cell[1] + style.frame_padding[1]],
                    if sort.descending { "v" } else { "^" },
                    style.text,
                );
            }
            let edge_color = if self.ctx.active == Some(resize_id) {
                style.widget_bg_active
            } else if self.ctx.hot == Some(resize_id) {
                style.widget_bg_hovered
            } else {
                style.scrollbar_thumb
            };
            crate::widgets::draw_parts(self.ctx).1.add_filled_rect(
                [cell[2] - 1., cell[1]],
                [cell[2], cell[3]],
                edge_color,
            );
            self.end_data_cell(scope);
        }
        Ok(table)
    }
    pub fn begin_data_table_cell(
        &mut self,
        table: &DataTableScope,
        row: usize,
        column: usize,
    ) -> Option<DataCellScope> {
        if row >= table.rows
            || column >= table.columns
            || row >= table.options.frozen_rows && !table.visible.range().contains(&row)
        {
            return None;
        }
        let x = table.column_x(column);
        let y = table.rect[1] + table.header_height + row as f32 * table.options.row_height
            - if row < table.options.frozen_rows {
                0.
            } else {
                table.scroll[1]
            };
        let rect = [x, y, x + table.widths[column], y + table.options.row_height];
        let mut clip = table.column_clip(column);
        let boundary = (table.rect[1]
            + table.header_height
            + table.options.frozen_rows as f32 * table.options.row_height)
            .min(table.rect[3]);
        clip[1] = if row < table.options.frozen_rows {
            (table.rect[1] + table.header_height).min(table.rect[3])
        } else {
            boundary
        };
        if row < table.options.frozen_rows {
            clip[3] = boundary;
        }
        let shown = crate::draw::intersect_rects(rect, clip);
        if shown[2] <= shown[0] || shown[3] <= shown[1] {
            return None;
        }
        let style = self.ctx.style;
        let scope = self.data_cell(rect, clip, row, column);
        let padding = style.frame_padding[0].min((rect[2] - rect[0]) * 0.25);
        let origin = [rect[0] + padding, rect[1] + 2.];
        let width = (rect[2] - rect[0] - padding * 2.).max(0.);
        if self.ctx.popup_scope_active {
            self.ctx.overlay_cursor.enter_region(origin, width);
        } else {
            self.ctx
                .current_window_mut()
                .cursor
                .enter_region(origin, width);
        }
        let bg = if row.is_multiple_of(2) {
            style.window_bg
        } else {
            style.widget_bg
        };
        crate::widgets::draw_parts(self.ctx).1.add_filled_rect(
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            bg,
        );
        Some(scope)
    }
    pub fn end_data_table_cell(&mut self, scope: DataCellScope) {
        self.end_data_cell(scope);
    }
    pub fn end_data_table(&mut self, table: DataTableScope) -> f32 {
        self.end_child(table.child);
        match table.horizontal_track {
            Some(track) => crate::scroll::scrollbar_axis(
                self.ctx,
                table.horizontal_id,
                track,
                table.scroll[0],
                table.horizontal_range,
                table.visible_width,
                table.content_width,
                0,
            ),
            None => table.scroll[0],
        }
    }
    pub fn data_table(
        &mut self,
        label: &str,
        headers: &[&str],
        rows: usize,
        state: &mut DataTableState,
        options: DataTableOptions,
        f: impl FnOnce(&mut Ui, &DataTableScope),
    ) -> Result<(), DataError> {
        let table = self.begin_data_table(label, headers, rows, state, options)?;
        f(self, &table);
        state.scroll_x = self.end_data_table(table);
        Ok(())
    }
    pub fn data_table_cell(
        &mut self,
        table: &DataTableScope,
        row: usize,
        column: usize,
        f: impl FnOnce(&mut Ui),
    ) -> bool {
        let Some(cell) = self.begin_data_table_cell(table, row, column) else {
            return false;
        };
        f(self);
        self.end_data_table_cell(cell);
        true
    }
}
