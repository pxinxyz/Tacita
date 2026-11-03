use crate::{
    DashPattern, GridCell, Gui, LayoutError, LayoutItem, LayoutOptions, LayoutSpec, PathError,
    Scope, StrokeStyle,
};

impl Gui {
    pub fn draw_polyline(
        &mut self,
        points: &[[f32; 2]],
        closed: bool,
        style: StrokeStyle,
        color: [u8; 4],
    ) -> Result<(), PathError> {
        if self.emitting() {
            self.ctx.ui().draw_polyline(points, closed, style, color)
        } else {
            Ok(())
        }
    }
    pub fn draw_dashed_polyline(
        &mut self,
        points: &[[f32; 2]],
        closed: bool,
        style: StrokeStyle,
        dash: DashPattern<'_>,
        color: [u8; 4],
    ) -> Result<(), PathError> {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_dashed_polyline(points, closed, style, dash, color)
        } else {
            Ok(())
        }
    }
    pub fn draw_quadratic_bezier(
        &mut self,
        points: [[f32; 2]; 3],
        style: StrokeStyle,
        color: [u8; 4],
    ) -> Result<(), PathError> {
        if self.emitting() {
            self.ctx.ui().draw_quadratic_bezier(points, style, color)
        } else {
            Ok(())
        }
    }
    pub fn draw_cubic_bezier(
        &mut self,
        points: [[f32; 2]; 4],
        style: StrokeStyle,
        color: [u8; 4],
    ) -> Result<(), PathError> {
        if self.emitting() {
            self.ctx.ui().draw_cubic_bezier(points, style, color)
        } else {
            Ok(())
        }
    }
    pub fn draw_concave_polygon(
        &mut self,
        points: &[[f32; 2]],
        color: [u8; 4],
    ) -> Result<(), PathError> {
        if self.emitting() {
            self.ctx.ui().draw_concave_polygon(points, color)
        } else {
            Ok(())
        }
    }
    pub fn begin_layout_extended(
        &mut self,
        label: &str,
        spec: LayoutSpec,
        items: &[LayoutItem],
        options: LayoutOptions,
        cells: &[GridCell],
    ) -> Result<[f32; 4], LayoutError> {
        let scope = if self.emitting() {
            Some(
                self.ctx
                    .ui()
                    .begin_layout_extended(label, spec, items, options, cells)?,
            )
        } else {
            None
        };
        let rect = scope.as_ref().map_or([0.0; 4], |s| s.rect());
        self.scopes.push(Scope::Layout(scope));
        Ok(rect)
    }
}
