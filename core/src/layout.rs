mod containers;
mod extensions;
#[cfg(test)]
#[path = "layout/extensions_tests.rs"]
mod extensions_tests;
pub use extensions::{
    resolve_layout_extended, resolve_layout_extended_with, ContentAlign, GridCell, LayoutOptions,
    GRID_AUTO,
};
mod solver;
pub use containers::{LayoutItemScope, LayoutScope};
pub use solver::{
    resolve_layout, resolve_layout_with, LayoutAlign, LayoutError, LayoutGeometry, LayoutItem,
    LayoutJustify, LayoutKind, LayoutSpec, MAX_LAYOUT_ITEMS,
};
#[cfg(test)]
#[path = "layout/tests.rs"]
mod container_tests;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Cursor {
    line_start_x: f32,
    content_right: f32,
    pos: [f32; 2],
    last_line_y: f32,
    line_height: f32,
    last_item_right: f32,
    has_last_item: bool,
    spacing: [f32; 2],
}

impl Cursor {
    pub(crate) fn new(
        window_pos: [f32; 2],
        window_width: f32,
        window_padding: [f32; 2],
        item_spacing: [f32; 2],
    ) -> Self {
        let origin = [
            window_pos[0] + window_padding[0],
            window_pos[1] + window_padding[1],
        ];
        Self {
            line_start_x: origin[0],
            content_right: window_pos[0] + window_width - window_padding[0],
            pos: origin,
            last_line_y: origin[1],
            line_height: 0.0,
            last_item_right: origin[0],
            has_last_item: false,
            spacing: item_spacing,
        }
    }

    #[cfg(test)]
    pub(crate) fn next_pos(&self) -> [f32; 2] {
        self.pos
    }

    #[cfg(test)]
    pub(crate) fn content_width(&self) -> f32 {
        self.content_right - self.line_start_x
    }

    pub(crate) fn pos_y(&self) -> f32 {
        self.pos[1]
    }

    pub(crate) fn position(&self) -> [f32; 2] {
        self.pos
    }

    pub(crate) fn available_width(&self) -> f32 {
        self.content_right - self.pos[0]
    }

    pub(crate) fn advance(&mut self, item_size: [f32; 2]) -> [f32; 4] {
        if self.pos[1] != self.last_line_y {
            self.line_height = 0.0;
        }
        let pos = self.pos;
        self.line_height = self.line_height.max(item_size[1]);
        self.last_item_right = pos[0] + item_size[0];
        self.last_line_y = pos[1];
        self.has_last_item = true;
        self.pos = [
            self.line_start_x,
            pos[1] + self.line_height + self.spacing[1],
        ];
        [pos[0], pos[1], self.last_item_right, pos[1] + item_size[1]]
    }

    pub(crate) fn skip_rows(&mut self, count: usize, item_size: [f32; 2]) {
        if count == 0 {
            return;
        }
        let stride = item_size[1] + self.spacing[1];
        self.last_line_y = self.pos[1] + (count - 1) as f32 * stride;
        self.pos = [self.line_start_x, self.pos[1] + count as f32 * stride];
        self.line_height = item_size[1];
        self.last_item_right = self.line_start_x + item_size[0];
        self.has_last_item = true;
    }

    pub(crate) fn same_line(&mut self) {
        if self.has_last_item {
            self.pos = [self.last_item_right + self.spacing[0], self.last_line_y];
        }
    }

    pub(crate) fn enter_region(&mut self, origin: [f32; 2], width: f32) {
        self.line_start_x = origin[0];
        self.content_right = origin[0] + width;
        self.pos = origin;
        self.last_line_y = origin[1];
        self.line_height = 0.0;
        self.last_item_right = origin[0];
        self.has_last_item = false;
    }

    pub(crate) fn indent(&mut self, amount: f32) {
        self.line_start_x += amount;
        self.pos[0] += amount;
    }

    pub(crate) fn unindent(&mut self, amount: f32) {
        self.line_start_x -= amount;
        self.pos[0] -= amount;
    }

    pub(crate) fn collapse_from(&mut self, saved: Self, height: f32) {
        *self = saved;
        if height > 0.0 {
            self.last_line_y = self.pos[1];
            self.line_height = (height - self.spacing[1]).max(0.0);
            self.pos[1] += height;
            self.last_item_right = self.line_start_x;
            self.has_last_item = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POS: [f32; 2] = [100.0, 50.0];
    const WIDTH: f32 = 300.0;
    const PADDING: [f32; 2] = [10.0, 10.0];
    const SPACING: [f32; 2] = [8.0, 4.0];

    fn cursor() -> Cursor {
        Cursor::new(POS, WIDTH, PADDING, SPACING)
    }

    #[test]
    fn sequential_advances_stack_vertically_with_spacing() {
        let mut c = cursor();

        let r0 = c.advance([50.0, 20.0]);
        assert_eq!(r0, [110.0, 60.0, 160.0, 80.0]);

        let r1 = c.advance([30.0, 10.0]);
        assert_eq!(r1, [110.0, 84.0, 140.0, 94.0]);

        let r2 = c.advance([70.0, 16.0]);
        assert_eq!(r2, [110.0, 98.0, 180.0, 114.0]);
    }

    #[test]
    fn skipped_rows_match_repeated_vertical_advances() {
        let mut expected = cursor();
        let mut actual = cursor();
        for _ in 0..100 {
            expected.advance([50.0, 20.0]);
        }
        actual.skip_rows(100, [50.0, 20.0]);
        assert_eq!(actual, expected);
        assert_eq!(actual.advance([10.0, 5.0]), expected.advance([10.0, 5.0]));
    }

    #[test]
    fn same_line_places_next_item_after_previous_right_edge() {
        let mut c = cursor();

        let r0 = c.advance([50.0, 20.0]);
        c.same_line();
        let r1 = c.advance([30.0, 10.0]);

        assert_eq!(
            r1,
            [
                r0[2] + SPACING[0],
                r0[1],
                r0[2] + SPACING[0] + 30.0,
                r0[1] + 10.0
            ]
        );

        let r2 = c.advance([10.0, 5.0]);
        assert_eq!(r2[1], r0[1] + 20.0 + SPACING[1]);
    }

    #[test]
    fn cursor_respects_window_padding_on_both_axes() {
        let mut c = cursor();

        let r = c.advance([10.0, 10.0]);
        assert_eq!([r[0], r[1]], [POS[0] + PADDING[0], POS[1] + PADDING[1]]);

        assert_eq!(c.content_width(), WIDTH - 2.0 * PADDING[0]);

        c.same_line();
        assert_eq!(
            c.available_width(),
            WIDTH - 2.0 * PADDING[0] - 10.0 - SPACING[0]
        );
    }

    #[test]
    fn same_line_before_any_item_is_a_no_op() {
        let mut c = cursor();
        c.same_line();
        let r = c.advance([10.0, 10.0]);
        assert_eq!([r[0], r[1]], [POS[0] + PADDING[0], POS[1] + PADDING[1]]);
    }

    #[test]
    fn enter_region_lays_out_independently_of_the_outer_flow() {
        let mut c = cursor();

        c.advance([50.0, 20.0]);
        let outer = c;

        c.enter_region([400.0, 60.0], 120.0);
        assert_eq!(c.available_width(), 120.0);
        let a = c.advance([40.0, 10.0]);
        assert_eq!(a, [400.0, 60.0, 440.0, 70.0]);

        let b = c.advance([40.0, 10.0]);
        assert_eq!(b, [400.0, 74.0, 440.0, 84.0]);

        c.same_line();
        let d = c.advance([30.0, 10.0]);
        assert_eq!(d, [448.0, 74.0, 478.0, 84.0]);

        c = outer;
        let r1 = c.advance([30.0, 10.0]);
        assert_eq!(r1, [110.0, 84.0, 140.0, 94.0]);
    }

    #[test]
    fn same_line_right_after_enter_region_is_a_no_op() {
        let mut c = cursor();
        c.advance([50.0, 20.0]);
        c.enter_region([400.0, 60.0], 120.0);
        c.same_line();
        let r = c.advance([10.0, 10.0]);
        assert_eq!(r, [400.0, 60.0, 410.0, 70.0]);
    }

    #[test]
    fn indent_shifts_new_lines_and_unindent_restores() {
        let mut c = cursor();
        let base_x = POS[0] + PADDING[0];
        c.indent(20.0);

        let r0 = c.advance([30.0, 10.0]);
        assert_eq!(r0[0], base_x + 20.0);
        assert_eq!(c.available_width(), WIDTH - 2.0 * PADDING[0] - 20.0);

        c.indent(5.0);
        let r1 = c.advance([10.0, 8.0]);
        assert_eq!(r1[0], base_x + 25.0);
        c.unindent(5.0);
        let r2 = c.advance([10.0, 8.0]);
        assert_eq!(r2[0], base_x + 20.0);
        c.unindent(20.0);
        let r3 = c.advance([10.0, 8.0]);
        assert_eq!(r3[0], base_x);
    }
}
