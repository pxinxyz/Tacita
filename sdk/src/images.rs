use crate::Gui;
use restricted_gui::images::{ImageError, ImageFit, ImageRegion, NineSlice};
impl Gui {
    pub fn image(
        &mut self,
        region: ImageRegion,
        size: [f32; 2],
        fit: ImageFit,
        tint: [u8; 4],
    ) -> Result<[f32; 4], ImageError> {
        if !self.emitting() {
            return Ok([0.; 4]);
        }
        self.ctx.ui().image(region, size, fit, tint)
    }
    pub fn image_button(
        &mut self,
        label: &str,
        region: ImageRegion,
        size: [f32; 2],
        fit: ImageFit,
        tint: [u8; 4],
    ) -> Result<bool, ImageError> {
        if !self.emitting() {
            return Ok(false);
        }
        self.ctx.ui().image_button(label, region, size, fit, tint)
    }
    pub fn draw_nine_slice(
        &mut self,
        region: ImageRegion,
        rect: [f32; 4],
        slices: NineSlice,
        tint: [u8; 4],
    ) -> Result<(), ImageError> {
        if !self.emitting() {
            return Ok(());
        }
        self.ctx.ui().draw_nine_slice(region, rect, slices, tint)
    }
}

impl crate::Gui {
    pub fn update_display_metrics(
        &mut self,
        metrics: restricted_gui::display::DisplayMetrics,
    ) -> Result<bool, crate::FontError> {
        self.ctx.update_display_metrics(metrics)
    }
}

impl crate::Gui {
    pub fn set_inspection_enabled(&mut self, on: bool) {
        self.ctx.set_inspection_enabled(on);
    }
    pub fn inspection_count(&self) -> usize {
        self.ctx.inspection_count()
    }
    pub fn inspection_node(&self, index: usize) -> Option<crate::InspectionNode> {
        self.ctx.inspection_node(index)
    }
    pub fn inspection_name(&self, id: u64, out: &mut alloc::string::String) {
        self.ctx.inspection_name(id, out);
    }
    pub fn new_frame_recorded(&mut self, frame: &crate::RecordedFrame) {
        for &event in frame.navigation() {
            self.ctx.navigate(event);
        }
        self.new_frame(frame.input(), frame.dt);
    }
}
