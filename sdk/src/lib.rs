#![no_std]

extern crate alloc;

mod host_services;
pub use host_services::*;

use alloc::string::String;
use alloc::vec::Vec;
use core::ops::RangeInclusive;

use tacita::context::Context;
use tacita::panels::{
    DockPanelScope, DockspaceScope, PanelCellScope, ResizableChildScope, SplitScope, TableScope,
};
pub use tacita::panels::{DockState, DockZone, SplitAxis};
pub use tacita::ui::ValidationState;
use tacita::ui::{ChildScope, CollapsingHeaderScope, DisabledScope, TreeNodeScope};

pub use tacita::animation::{lerp_color, pulse, Easing, MotionSettings, Spring, Tween};
pub use tacita::animation::{
    sample_keyframes, GradientOptions, GradientSpread, GradientStop, Keyframe, PlaybackMode,
    Timeline, TimelineOptions,
};
pub use tacita::draw::{DrawCmd, DrawData, DrawVert, TextureId};
pub use tacita::shapes::{BorderPlacement, RoundedRect, Shadow, ShapeError};
pub use tacita::svg::{rasterize_svg, SvgBitmap, SvgError, SvgIconId};
pub use tacita::text::{TextAlign, TextMetrics, TextOptions, TextOverflow, TextSpan};
pub use tacita::theme::{ThemeRole, ThemeTokens};
mod data;
mod editing;
mod paths;
mod shapes;
pub use tacita::data::{
    CurveEditor, CurveInterpolation, CurveKey, GradientEditor, PlotHover, PlotOptions,
    MAX_CURVE_KEYS, MAX_EDITOR_STOPS, MAX_PLOT_COLUMNS,
};
pub use tacita::data::{
    DataError, DataTableOptions, DataTableState, NumericEdit, Selection, SortSpec, VisibleRows,
    MAX_DATA_COLUMNS, MAX_DATA_ROWS,
};
pub use tacita::editing::{
    ClipboardAction, ClipboardRequest, EditError, TextCaret, TextEditOptions, TextEditResponse,
    TextEditor, TextEventKind, MAX_EDIT_BYTES,
};
pub use tacita::font::{
    CaretPosition, ShapedGlyph, ShapedOptions, ShapedText, TextDirection,
};
pub use tacita::layout::{
    resolve_layout_extended, ContentAlign, GridCell, LayoutOptions, GRID_AUTO,
};
pub use tacita::paths::{
    DashPattern, LineCap, LineJoin, PathError, StrokeStyle, MAX_DASH_LENGTHS, MAX_PATH_POINTS,
    MAX_POLYGON_POINTS,
};
mod images;
pub use tacita::diagnostics::{
    InputRecording, InspectionFlags, InspectionKind, InspectionNode, RecordedFrame, ReplayCursor,
    ReplayError,
};
pub use tacita::display::DisplayMetrics;
pub use tacita::images::{
    ImageError, ImageFit, ImageRegion, NineSlice, SpriteAtlas, SpriteId,
};
pub use tacita::rendering::{
    AlphaMode, ColorMode, CommandPlan, DrawCalls, DrawOptimization, GeometryRange, PlanError,
    PlannedDraw, QuadInstance, RenderStats, TextureEncoding, UploadStrategy,
};
mod effects;
mod input_state;
pub use tacita::drag_drop::DragPayload;
pub use tacita::effects::{
    AlphaMask, Blur, BlurMethod, DitherKind, DrawEffects, EffectError, FilterKind, FilterPacket,
    FilterPlan, FilterStep, GradientFill, GradientKind, GradientMask, GradientPaint, ImageFilter,
    MaskOp, MaskScope, MaskStack, RevealDirection, RevealMask, SdfKind, SdfShape, SdfStyle,
    StencilScope, SurfaceStyle, EFFECT_UNIFORM_SIZE, MAX_FILTER_PASSES, MAX_MASKS, MAX_PAINT_STOPS,
    SURFACE_ALL, SURFACE_CHROMATIC, SURFACE_DESATURATE, SURFACE_DISTORT, SURFACE_DITHER,
    SURFACE_DOTS, SURFACE_GLASS, SURFACE_GRADE, SURFACE_GRAIN, SURFACE_GRID, SURFACE_NOISE,
    SURFACE_SCANLINES, SURFACE_SHIMMER, SURFACE_SWEEP, SURFACE_VIGNETTE,
};
pub use tacita::font::{
    ColorFontError, Font, FontError, FontId, FontInfo, IconFont, TextRasterMode,
};
pub use tacita::icon;
pub use tacita::id::Id;
pub use tacita::input::{InputState, Key, KeyBinding, KeyEvent, Modifiers};
pub use tacita::interaction::{Navigation, Shortcut, ShortcutScope, MAX_POPUPS};
pub use tacita::layout::{
    resolve_layout, LayoutAlign, LayoutError, LayoutGeometry, LayoutItem, LayoutJustify,
    LayoutKind, LayoutSpec, MAX_LAYOUT_ITEMS,
};
pub use tacita::persistence::{
    decode_theme, encode_theme, Config, History, LayoutSnapshot, PanelLayout, PanelState,
    StateError, Value, WindowLayout,
};
pub use tacita::style::Style;
pub use tacita::ScrollFlags;
pub use tacita::{ScrollbarSkin, ScrollbarTrackClick};

#[allow(clippy::large_enum_variant)]
enum Scope {
    Rows(Option<tacita::data::RowClipperScope>),
    DataTable(Option<tacita::data::DataTableScope>),
    DataCell(Option<tacita::data::DataCellScope>),
    MenuBar(Option<tacita::data::MenuBarScope>),
    DataMenu,
    PropertyGrid(Option<tacita::data::PropertyGridScope>),
    PropertyRow(Option<tacita::data::DataCellScope>),
    Stencil(Option<tacita::effects::StencilScope>),
    Mask(Option<tacita::effects::MaskScope>),
    RoundedClip(Option<tacita::shapes::RoundedClipScope>),
    Layout(Option<tacita::layout::LayoutScope>),
    LayoutItem(Option<tacita::layout::LayoutItemScope>),
    AnimatedLayout(Option<tacita::animation::AnimatedLayoutScope>),
    CollapsingHeader(Option<CollapsingHeaderScope>),
    TreeNode(Option<TreeNodeScope>),
    ScrollRegion(Option<ChildScope>),
    Child(Option<ChildScope>),
    GroupBox(Option<ChildScope>),
    PreviewCanvas(Option<ChildScope>),
    Disabled(Option<DisabledScope>),
    TabBar(Option<()>),
    Popup(Option<()>),
    Table(Option<TableScope>),
    Split(Option<SplitScope>),
    PanelCell(Option<PanelCellScope>),
    ResizableChild(Option<ResizableChildScope>),
    Dockspace(Option<DockspaceScope>),
    DockPanel(Option<DockPanelScope>),
}

impl Scope {
    fn name(&self) -> &'static str {
        match self {
            Scope::Rows(_) => "rows",
            Scope::DataTable(_) => "data table",
            Scope::DataCell(_) => "data cell",
            Scope::MenuBar(_) => "menu bar",
            Scope::DataMenu => "menu",
            Scope::PropertyGrid(_) => "property grid",
            Scope::PropertyRow(_) => "property row",
            Scope::Stencil(_) => "stencil",
            Scope::Mask(_) => "mask",
            Scope::RoundedClip(_) => "rounded clip",
            Scope::Layout(_) => "layout",
            Scope::LayoutItem(_) => "layout item",
            Scope::AnimatedLayout(_) => "animated layout",
            Scope::CollapsingHeader(_) => "collapsing header",
            Scope::TreeNode(_) => "tree node",
            Scope::ScrollRegion(_) => "scroll region",
            Scope::Child(_) => "child",
            Scope::GroupBox(_) => "group box",
            Scope::PreviewCanvas(_) => "preview canvas",
            Scope::Disabled(_) => "disabled section",
            Scope::TabBar(_) => "tab bar",
            Scope::Popup(_) => "popup",
            Scope::Table(_) => "table",
            Scope::Split(_) => "split",
            Scope::PanelCell(_) => "panel cell",
            Scope::ResizableChild(_) => "resizable child",
            Scope::Dockspace(_) => "dockspace",
            Scope::DockPanel(_) => "dock panel",
        }
    }
}

pub struct Gui {
    ctx: Context,
    window_open: bool,
    closed_window_pending: bool,
    scopes: Vec<Scope>,
}

impl Gui {
    pub fn set_text_raster_mode(&mut self, mode: TextRasterMode) {
        self.ctx.set_text_raster_mode(mode);
    }
    pub fn text_raster_mode(&self) -> TextRasterMode {
        self.ctx.text_raster_mode()
    }
    pub fn add_svg_icon(&mut self, source: &str, size: [f32; 2]) -> Result<SvgIconId, SvgError> {
        self.ctx.add_svg_icon(source, size)
    }
    pub fn draw_svg_icon(
        &mut self,
        icon: SvgIconId,
        rect: [f32; 4],
        tint: [u8; 4],
    ) -> Result<(), SvgError> {
        if self.emitting() {
            self.ctx.ui().draw_svg_icon(icon, rect, tint)
        } else {
            Ok(())
        }
    }
    pub fn svg_icon(
        &mut self,
        icon: SvgIconId,
        size: [f32; 2],
        tint: [u8; 4],
    ) -> Result<[f32; 4], SvgError> {
        if self.emitting() {
            self.ctx.ui().svg_icon(icon, size, tint)
        } else {
            Ok([0.0; 4])
        }
    }
    pub fn available_size(&mut self) -> [f32; 2] {
        if self.emitting() {
            self.ctx.ui().available_size()
        } else {
            [0.0; 2]
        }
    }
    pub fn begin_layout(
        &mut self,
        label: &str,
        spec: LayoutSpec,
        items: &[LayoutItem],
    ) -> Result<[f32; 4], LayoutError> {
        let scope = if self.emitting() {
            Some(self.ctx.ui().begin_layout(label, spec, items)?)
        } else {
            None
        };
        let rect = scope.as_ref().map_or([0.0; 4], |s| s.rect());
        self.scopes.push(Scope::Layout(scope));
        Ok(rect)
    }
    pub fn layout_item_rect(&self, index: usize) -> Option<[f32; 4]> {
        match self.scopes.last() {
            Some(Scope::Layout(Some(s))) => s.item_rect(index),
            Some(Scope::Layout(None)) => None,
            _ => panic!("layout_item_rect requires layout"),
        }
    }
    pub fn begin_layout_item(&mut self, index: usize) -> Result<[f32; 4], LayoutError> {
        let (scope, rect) = if self.emitting() {
            let Some(Scope::Layout(Some(parent))) = self.scopes.last_mut() else {
                panic!("begin_layout_item requires layout");
            };
            let scope = self.ctx.ui().begin_layout_item(parent, index)?;
            (Some(scope), parent.item_rect(index).unwrap_or([0.0; 4]))
        } else {
            (None, [0.0; 4])
        };
        self.scopes.push(Scope::LayoutItem(scope));
        Ok(rect)
    }
    pub fn end_layout_item(&mut self) {
        match self.scopes.pop() {
            Some(Scope::LayoutItem(Some(scope))) => {
                let Some(Scope::Layout(Some(parent))) = self.scopes.last_mut() else {
                    panic!("layout item lost parent");
                };
                self.ctx.ui().end_layout_item(parent, scope);
            }
            Some(Scope::LayoutItem(None)) => {}
            _ => panic!("end_layout_item requires layout item"),
        }
    }
    pub fn end_layout(&mut self) {
        match self.scopes.pop() {
            Some(Scope::Layout(Some(scope))) => self.ctx.ui().end_layout(scope),
            Some(Scope::Layout(None)) => {}
            _ => panic!("end_layout requires layout"),
        }
    }

    pub fn new(font_bytes: &[u8]) -> Result<Gui, FontError> {
        Ok(Self::with_context(Context::new(font_bytes)?))
    }

    pub fn new_with_icons(font_bytes: &[u8], icons: &[IconFont<'_>]) -> Result<Gui, FontError> {
        Ok(Self::with_context(Context::new_with_icons(
            font_bytes, icons,
        )?))
    }

    pub fn with_font(font: Font) -> Gui {
        Self::with_context(Context::with_font(font))
    }

    fn with_context(ctx: Context) -> Gui {
        Gui {
            ctx,
            window_open: false,
            closed_window_pending: false,
            scopes: Vec::new(),
        }
    }

    fn emitting(&self) -> bool {
        if self.closed_window_pending {
            return false;
        }
        !matches!(
            self.scopes.last(),
            Some(Scope::Layout(None))
                | Some(Scope::Stencil(None))
                | Some(Scope::Mask(None))
                | Some(Scope::RoundedClip(None))
                | Some(Scope::LayoutItem(None))
                | Some(Scope::AnimatedLayout(None))
                | Some(Scope::CollapsingHeader(None))
                | Some(Scope::TreeNode(None))
                | Some(Scope::ScrollRegion(None))
                | Some(Scope::Child(None))
                | Some(Scope::GroupBox(None))
                | Some(Scope::PreviewCanvas(None))
                | Some(Scope::Disabled(None))
                | Some(Scope::TabBar(None))
                | Some(Scope::Popup(None))
                | Some(Scope::Rows(None))
                | Some(Scope::DataTable(None))
                | Some(Scope::DataCell(None))
                | Some(Scope::MenuBar(None))
                | Some(Scope::PropertyGrid(None))
                | Some(Scope::PropertyRow(None))
                | Some(Scope::Table(None))
                | Some(Scope::Split(None))
                | Some(Scope::PanelCell(None))
                | Some(Scope::ResizableChild(None))
                | Some(Scope::Dockspace(None))
                | Some(Scope::DockPanel(None))
        )
    }

    pub fn style(&mut self) -> &mut Style {
        &mut self.ctx.style
    }

    pub fn set_window_open(&mut self, title: &str, open: bool) {
        self.ctx.set_window_open(title, open);
    }

    pub fn set_window_rect(&mut self, title: &str, pos: [f32; 2], size: [f32; 2]) {
        self.ctx.set_window_rect(title, pos, size);
    }

    pub fn set_scrollbar_skin(&mut self, skin: ScrollbarSkin) {
        self.ctx.set_scrollbar_skin(skin);
    }

    pub fn set_motion_settings(&mut self, settings: MotionSettings) {
        self.ctx.set_motion_settings(settings);
    }
    pub fn set_theme_tokens(&mut self, tokens: ThemeTokens) {
        self.ctx.set_theme_tokens(tokens);
    }
    pub fn theme_tokens(&self) -> ThemeTokens {
        self.ctx.theme_tokens()
    }
    pub fn theme_color(&self, role: ThemeRole) -> [u8; 4] {
        self.ctx.theme_color(role)
    }
    pub fn animate_timeline(&mut self, id: Id, keys: &[Keyframe], options: TimelineOptions) -> f32 {
        if self.emitting() {
            self.ctx.animate_timeline(id, keys, options)
        } else {
            keys.last().map_or(0.0, |key| key.value)
        }
    }
    pub fn seek_timeline(&mut self, id: Id, time: f32) {
        if self.emitting() {
            self.ctx.seek_timeline(id, time);
        }
    }
    pub fn animate_rect(
        &mut self,
        id: Id,
        initial: [f32; 4],
        target: [f32; 4],
        duration: f32,
        easing: Easing,
    ) -> [f32; 4] {
        if self.emitting() {
            self.ctx.animate_rect(id, initial, target, duration, easing)
        } else {
            target
        }
    }
    pub fn begin_animated_layout(
        &mut self,
        label: &str,
        target: [f32; 4],
        duration: f32,
        easing: Easing,
    ) -> [f32; 4] {
        if self.emitting() {
            let (scope, rect) = self
                .ctx
                .ui()
                .begin_animated_layout(label, target, duration, easing);
            self.scopes.push(Scope::AnimatedLayout(Some(scope)));
            rect
        } else {
            self.scopes.push(Scope::AnimatedLayout(None));
            [0.0; 4]
        }
    }
    pub fn end_animated_layout(&mut self) {
        match self.scopes.pop() {
            Some(Scope::AnimatedLayout(Some(scope))) => self.ctx.ui().end_animated_layout(scope),
            Some(Scope::AnimatedLayout(None)) => {}
            Some(other) => panic!("end_animated_layout cannot close {}", other.name()),
            None => panic!("end_animated_layout without begin"),
        }
    }
    pub fn draw_gradient(
        &mut self,
        rect: [f32; 4],
        stops: &[GradientStop],
        options: GradientOptions,
    ) {
        if self.emitting() {
            self.ctx.ui().draw_gradient(rect, stops, options);
        }
    }
    pub fn draw_animated_gradient(
        &mut self,
        rect: [f32; 4],
        stops: &[GradientStop],
        options: GradientOptions,
        time: f32,
        period: f32,
    ) {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_animated_gradient(rect, stops, options, time, period);
        }
    }
    pub fn measure_text_block(
        &mut self,
        text: &str,
        width: f32,
        options: TextOptions,
    ) -> TextMetrics {
        self.ctx.ui().measure_text_block(text, width, options)
    }
    pub fn measure_rich_text(
        &mut self,
        spans: &[TextSpan<'_>],
        width: f32,
        options: TextOptions,
    ) -> TextMetrics {
        self.ctx.ui().measure_rich_text(spans, width, options)
    }
    pub fn text_block(&mut self, text: &str, width: f32, options: TextOptions) -> TextMetrics {
        if self.emitting() {
            self.ctx.ui().text_block(text, width, options)
        } else {
            TextMetrics::default()
        }
    }
    pub fn rich_text(
        &mut self,
        spans: &[TextSpan<'_>],
        width: f32,
        options: TextOptions,
    ) -> TextMetrics {
        if self.emitting() {
            self.ctx.ui().rich_text(spans, width, options)
        } else {
            TextMetrics::default()
        }
    }
    pub fn draw_text_box(
        &mut self,
        rect: [f32; 4],
        text: &str,
        color: [u8; 4],
        options: TextOptions,
    ) -> TextMetrics {
        if self.emitting() {
            self.ctx.ui().draw_text_box(rect, text, color, options)
        } else {
            TextMetrics::default()
        }
    }
    pub fn draw_rich_text_box(
        &mut self,
        rect: [f32; 4],
        spans: &[TextSpan<'_>],
        options: TextOptions,
    ) -> TextMetrics {
        if self.emitting() {
            self.ctx.ui().draw_rich_text_box(rect, spans, options)
        } else {
            TextMetrics::default()
        }
    }
    pub fn motion_settings(&self) -> MotionSettings {
        self.ctx.motion_settings()
    }
    pub fn animate_float(
        &mut self,
        id: Id,
        initial: f32,
        target: f32,
        duration: f32,
        easing: Easing,
    ) -> f32 {
        if self.emitting() {
            self.ctx
                .animate_float(id, initial, target, duration, easing)
        } else {
            target
        }
    }
    pub fn animate_spring(&mut self, id: Id, initial: f32, target: f32, response: f32) -> f32 {
        if self.emitting() {
            self.ctx.animate_spring(id, initial, target, response)
        } else {
            target
        }
    }
    pub fn animate_color(
        &mut self,
        id: Id,
        initial: [u8; 4],
        target: [u8; 4],
        duration: f32,
        easing: Easing,
    ) -> [u8; 4] {
        if self.emitting() {
            self.ctx
                .animate_color(id, initial, target, duration, easing)
        } else {
            target
        }
    }

    pub fn add_font(&mut self, bytes: &[u8], px_size: f32) -> Result<FontId, FontError> {
        self.ctx.add_font(bytes, px_size)
    }

    pub fn add_font_with_icons(
        &mut self,
        bytes: &[u8],
        px_size: f32,
        icons: &[IconFont<'_>],
    ) -> Result<FontId, FontError> {
        self.ctx.add_font_with_icons(bytes, px_size, icons)
    }

    pub fn new_frame(&mut self, input: InputState, dt: f32) {
        self.ctx.set_input(input);
        self.ctx.new_frame(dt);
    }

    pub fn render(&mut self) -> DrawData<'_> {
        assert!(
            !self.window_open && !self.closed_window_pending && self.scopes.is_empty(),
            "Gui::render called with open scopes: every begin_* must be \
             closed by its end_* before render"
        );
        self.ctx.render()
    }

    pub fn atlas_pixels(&self) -> (&[u8], u32, u32) {
        self.ctx.font().atlas_pixels()
    }

    pub fn atlas_generation(&self) -> u64 {
        self.ctx.font().atlas_generation()
    }

    pub fn font_count(&self) -> usize {
        self.ctx.font().face_count()
    }

    pub fn font_info(&self, id: FontId) -> Option<FontInfo> {
        self.ctx.font().face_info(id)
    }

    pub fn active_font(&self) -> FontId {
        self.ctx.active_font()
    }

    pub fn begin_window(&mut self, title: &str, flags: ScrollFlags) {
        assert!(
            self.scopes.is_empty(),
            "Gui::begin_window called while a container scope is open: \
             close every container before beginning a window"
        );
        assert!(
            !self.window_open && !self.closed_window_pending,
            "Gui::begin_window called while a window scope is open or \
             unended: nested windows are not supported"
        );
        if self.ctx.ui().begin_window(title, flags) {
            self.window_open = true;
        } else {
            self.closed_window_pending = true;
        }
    }

    pub fn end_window(&mut self) {
        assert!(
            self.scopes.is_empty(),
            "Gui::end_window called while a container scope is open: close \
             every container before ending the window"
        );
        if self.window_open {
            self.ctx.ui().end_window();
            self.window_open = false;
        } else if self.closed_window_pending {
            self.closed_window_pending = false;
        } else {
            panic!(
                "Gui::end_window called with no window open: every \
                 end_window must follow a begin_window"
            );
        }
    }

    pub fn push_font(&mut self, id: FontId) {
        if self.emitting() {
            self.ctx.ui().push_font(id);
        }
    }

    pub fn pop_font(&mut self) {
        if self.emitting() {
            self.ctx.ui().pop_font();
        }
    }

    pub fn label(&mut self, text: &str) {
        if self.emitting() {
            self.ctx.ui().label(text);
        }
    }

    pub fn button(&mut self, label: &str) -> bool {
        self.emitting() && self.ctx.ui().button(label)
    }

    pub fn icon_button(&mut self, label: &str, side: f32) -> bool {
        self.emitting() && self.ctx.ui().icon_button(label, side)
    }

    pub fn separator(&mut self) {
        if self.emitting() {
            self.ctx.ui().separator();
        }
    }

    pub fn color_swatch(&mut self, label: &str, color: [u8; 4], size: [f32; 2]) {
        if self.emitting() {
            self.ctx.ui().color_swatch(label, color, size);
        }
    }

    pub fn status_badge(&mut self, label: &str, color: [u8; 4]) {
        if self.emitting() {
            self.ctx.ui().status_badge(label, color);
        }
    }

    pub fn hotkey_hint(&mut self, label: &str) {
        if self.emitting() {
            self.ctx.ui().hotkey_hint(label);
        }
    }

    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> bool {
        self.emitting() && self.ctx.ui().checkbox(label, value)
    }

    pub fn pill_toggle(&mut self, label: &str, value: &mut bool) -> bool {
        self.emitting() && self.ctx.ui().pill_toggle(label, value)
    }

    pub fn segmented_control(&mut self, label: &str, items: &[&str], selected: &mut usize) -> bool {
        self.emitting() && self.ctx.ui().segmented_control(label, items, selected)
    }

    pub fn radio_button(&mut self, label: &str, selected: &mut usize, index: usize) -> bool {
        self.emitting() && self.ctx.ui().radio_button(label, selected, index)
    }

    pub fn validation_message(&mut self, state: ValidationState, message: &str) {
        if self.emitting() {
            self.ctx.ui().validation_message(state, message);
        }
    }

    pub fn search_box(&mut self, label: &str, query: &mut String) -> bool {
        self.emitting() && self.ctx.ui().search_box(label, query)
    }

    pub fn nav_row(&mut self, label: &str, selected: &mut usize, index: usize) -> bool {
        self.emitting() && self.ctx.ui().nav_row(label, selected, index)
    }

    pub fn breadcrumbs(&mut self, label: &str, items: &[&str], selected: &mut usize) -> bool {
        self.emitting() && self.ctx.ui().breadcrumbs(label, items, selected)
    }

    pub fn progress_bar(&mut self, label: &str, fraction: f32, size: [f32; 2]) {
        if self.emitting() {
            self.ctx.ui().progress_bar(label, fraction, size);
        }
    }

    pub fn spinner(&mut self, label: &str, diameter: f32, phase: f32) {
        if self.emitting() {
            self.ctx.ui().spinner(label, diameter, phase);
        }
    }

    pub fn slider_float(
        &mut self,
        label: &str,
        value: &mut f32,
        range: RangeInclusive<f32>,
    ) -> bool {
        self.emitting() && self.ctx.ui().slider_float(label, value, range)
    }

    pub fn filled_slider_float(
        &mut self,
        label: &str,
        value: &mut f32,
        range: RangeInclusive<f32>,
    ) -> bool {
        self.emitting() && self.ctx.ui().filled_slider_float(label, value, range)
    }

    pub fn slider_int(&mut self, label: &str, value: &mut i32, range: RangeInclusive<i32>) -> bool {
        self.emitting() && self.ctx.ui().slider_int(label, value, range)
    }

    pub fn filled_slider_int(
        &mut self,
        label: &str,
        value: &mut i32,
        range: RangeInclusive<i32>,
    ) -> bool {
        self.emitting() && self.ctx.ui().filled_slider_int(label, value, range)
    }

    pub fn range_slider_float(
        &mut self,
        label: &str,
        lower: &mut f32,
        upper: &mut f32,
        range: RangeInclusive<f32>,
    ) -> bool {
        self.emitting() && self.ctx.ui().range_slider_float(label, lower, upper, range)
    }

    pub fn range_slider_int(
        &mut self,
        label: &str,
        lower: &mut i32,
        upper: &mut i32,
        range: RangeInclusive<i32>,
    ) -> bool {
        self.emitting() && self.ctx.ui().range_slider_int(label, lower, upper, range)
    }

    pub fn drag_float(
        &mut self,
        label: &str,
        value: &mut f32,
        speed: f32,
        range: RangeInclusive<f32>,
    ) -> bool {
        self.emitting() && self.ctx.ui().drag_float(label, value, speed, range)
    }

    pub fn drag_int(
        &mut self,
        label: &str,
        value: &mut i32,
        speed: f32,
        range: RangeInclusive<i32>,
    ) -> bool {
        self.emitting() && self.ctx.ui().drag_int(label, value, speed, range)
    }

    pub fn text_input(&mut self, label: &str, buf: &mut String) -> bool {
        self.emitting() && self.ctx.ui().text_input(label, buf)
    }

    pub fn same_line(&mut self) {
        if self.emitting() {
            self.ctx.ui().same_line();
        }
    }

    pub fn combo(&mut self, label: &str, items: &[&str], selected: &mut usize) -> bool {
        self.emitting() && self.ctx.ui().combo(label, items, selected)
    }

    pub fn searchable_combo(
        &mut self,
        label: &str,
        items: &[&str],
        selected: &mut usize,
        query: &mut String,
    ) -> bool {
        self.emitting()
            && self
                .ctx
                .ui()
                .searchable_combo(label, items, selected, query)
    }

    pub fn list_box(
        &mut self,
        label: &str,
        items: &[&str],
        selected: &mut usize,
        size: [f32; 2],
    ) -> bool {
        self.emitting() && self.ctx.ui().list_box(label, items, selected, size)
    }

    pub fn color_edit_rgba(&mut self, label: &str, value: &mut [u8; 4]) -> bool {
        self.emitting() && self.ctx.ui().color_edit_rgba(label, value)
    }

    pub fn color_picker_rgba(&mut self, label: &str, value: &mut [u8; 4]) -> bool {
        self.emitting() && self.ctx.ui().color_picker_rgba(label, value)
    }

    pub fn keybind(&mut self, label: &str, binding: &mut KeyBinding) -> bool {
        self.emitting() && self.ctx.ui().keybind(label, binding)
    }

    pub fn tooltip_text(&mut self, text: &str) {
        if self.emitting() {
            self.ctx.ui().tooltip_text(text);
        }
    }

    pub fn open_popup(&mut self, id: Id) {
        if self.emitting() {
            self.ctx.ui().open_popup(id);
        }
    }

    pub fn open_modal(&mut self, id: Id) {
        if self.emitting() {
            self.ctx.ui().open_modal(id);
        }
    }

    pub fn close_popup(&mut self) {
        self.ctx.ui().close_popup();
    }

    pub fn is_popup_open(&mut self, id: Id) -> bool {
        self.ctx.ui().is_popup_open(id)
    }

    pub fn begin_popup(&mut self, id: Id, rect: [f32; 4]) -> bool {
        let open = self.emitting() && self.ctx.ui().begin_popup(id, rect);
        self.scopes.push(Scope::Popup(open.then_some(())));
        open
    }

    pub fn begin_modal(&mut self, id: Id, rect: [f32; 4]) -> bool {
        let open = self.emitting() && self.ctx.ui().begin_modal(id, rect);
        self.scopes.push(Scope::Popup(open.then_some(())));
        open
    }

    pub fn begin_context_menu(&mut self, id: Id, target: [f32; 4], size: [f32; 2]) -> bool {
        let open = self.emitting() && self.ctx.ui().begin_context_menu(id, target, size);
        self.scopes.push(Scope::Popup(open.then_some(())));
        open
    }

    pub fn end_popup(&mut self) {
        match self.scopes.pop() {
            Some(Scope::Popup(Some(()))) => self.ctx.ui().end_popup(),
            Some(Scope::Popup(None)) => {}
            Some(other) => panic!("Gui::end_popup called while {} is open", other.name()),
            None => panic!("Gui::end_popup without begin_popup"),
        }
    }

    pub fn toast(&mut self, text: &str, seconds: f32) {
        if self.emitting() {
            self.ctx.ui().toast(text, seconds);
        }
    }

    pub fn begin_collapsing_header(&mut self, title: &str) {
        let scope = if self.emitting() {
            self.ctx.ui().begin_collapsing_header(title)
        } else {
            None
        };
        self.scopes.push(Scope::CollapsingHeader(scope));
    }

    pub fn end_collapsing_header(&mut self) {
        match self.scopes.pop() {
            Some(Scope::CollapsingHeader(scope)) => {
                if let Some(scope) = scope {
                    self.ctx.ui().end_collapsing_header(scope);
                }
            }
            Some(other) => panic!(
                "Gui::end_collapsing_header called while the innermost open \
                 scope is a {}",
                other.name()
            ),
            None => panic!(
                "Gui::end_collapsing_header called with no collapsing \
                 header open: every end must follow its begin"
            ),
        }
    }

    pub fn begin_tree_node(&mut self, label: &str) {
        let scope = if self.emitting() {
            self.ctx.ui().begin_tree_node(label)
        } else {
            None
        };
        self.scopes.push(Scope::TreeNode(scope));
    }

    pub fn end_tree_node(&mut self) {
        match self.scopes.pop() {
            Some(Scope::TreeNode(scope)) => {
                if let Some(scope) = scope {
                    self.ctx.ui().end_tree_node(scope);
                }
            }
            Some(other) => panic!(
                "Gui::end_tree_node called while the innermost open scope is a {}",
                other.name()
            ),
            None => panic!("Gui::end_tree_node called with no tree node open"),
        }
    }

    pub fn begin_scroll_region(&mut self, label: &str, size: [f32; 2]) {
        let scope = if self.emitting() {
            Some(self.ctx.ui().begin_scroll_region(label, size))
        } else {
            None
        };
        self.scopes.push(Scope::ScrollRegion(scope));
    }

    pub fn end_scroll_region(&mut self) {
        match self.scopes.pop() {
            Some(Scope::ScrollRegion(scope)) => {
                if let Some(scope) = scope {
                    self.ctx.ui().end_scroll_region(scope);
                }
            }
            Some(other) => panic!(
                "Gui::end_scroll_region called while the innermost open \
                 scope is a {}",
                other.name()
            ),
            None => panic!(
                "Gui::end_scroll_region called with no scroll region open: \
                 every end must follow its begin"
            ),
        }
    }

    pub fn begin_child(&mut self, label: &str, size: [f32; 2], flags: ScrollFlags) {
        let scope = if self.emitting() {
            Some(self.ctx.ui().begin_child(label, size, flags))
        } else {
            None
        };
        self.scopes.push(Scope::Child(scope));
    }

    pub fn end_child(&mut self) {
        match self.scopes.pop() {
            Some(Scope::Child(scope)) => {
                if let Some(scope) = scope {
                    self.ctx.ui().end_child(scope);
                }
            }
            Some(other) => panic!(
                "Gui::end_child called while the innermost open scope is a {}",
                other.name()
            ),
            None => panic!(
                "Gui::end_child called with no child open: every end must \
                 follow its begin"
            ),
        }
    }

    pub fn begin_table(&mut self, label: &str, size: [f32; 2], columns: usize, rows: usize) {
        let scope = self
            .emitting()
            .then(|| self.ctx.ui().begin_table(label, size, columns, rows));
        self.scopes.push(Scope::Table(scope));
    }

    pub fn begin_table_cell(&mut self, row: usize, column: usize) {
        let scope = if self.emitting() {
            let Some(Scope::Table(Some(table))) = self.scopes.last() else {
                panic!("begin_table_cell requires an open table");
            };
            Some(self.ctx.ui().begin_table_cell(table, row, column))
        } else {
            None
        };
        self.scopes.push(Scope::PanelCell(scope));
    }

    pub fn end_panel_cell(&mut self) {
        match self.scopes.pop() {
            Some(Scope::PanelCell(Some(scope))) => self.ctx.ui().end_panel_cell(scope),
            Some(Scope::PanelCell(None)) => {}
            Some(other) => panic!("end_panel_cell found {}", other.name()),
            None => panic!("end_panel_cell with no cell open"),
        }
    }

    pub fn end_table(&mut self) {
        match self.scopes.pop() {
            Some(Scope::Table(Some(scope))) => self.ctx.ui().end_table(scope),
            Some(Scope::Table(None)) => {}
            Some(other) => panic!("end_table found {}", other.name()),
            None => panic!("end_table with no table open"),
        }
    }

    pub fn begin_split(&mut self, label: &str, size: [f32; 2], axis: SplitAxis, ratio: &mut f32) {
        let scope = self
            .emitting()
            .then(|| self.ctx.ui().begin_split(label, size, axis, ratio));
        self.scopes.push(Scope::Split(scope));
    }

    pub fn begin_split_pane(&mut self, index: usize) {
        let scope = if self.emitting() {
            let Some(Scope::Split(Some(split))) = self.scopes.last() else {
                panic!("begin_split_pane requires an open split");
            };
            Some(self.ctx.ui().begin_split_pane(split, index))
        } else {
            None
        };
        self.scopes.push(Scope::PanelCell(scope));
    }

    pub fn end_split(&mut self) {
        match self.scopes.pop() {
            Some(Scope::Split(Some(scope))) => self.ctx.ui().end_split(scope),
            Some(Scope::Split(None)) => {}
            Some(other) => panic!("end_split found {}", other.name()),
            None => panic!("end_split with no split open"),
        }
    }

    pub fn begin_resizable_child(
        &mut self,
        label: &str,
        size: &mut [f32; 2],
        min_size: [f32; 2],
        flags: ScrollFlags,
    ) {
        let scope = self.emitting().then(|| {
            self.ctx
                .ui()
                .begin_resizable_child(label, size, min_size, flags)
        });
        self.scopes.push(Scope::ResizableChild(scope));
    }

    pub fn end_resizable_child(&mut self) {
        match self.scopes.pop() {
            Some(Scope::ResizableChild(Some(scope))) => self.ctx.ui().end_resizable_child(scope),
            Some(Scope::ResizableChild(None)) => {}
            Some(other) => panic!("end_resizable_child found {}", other.name()),
            None => panic!("end_resizable_child with no child open"),
        }
    }

    pub fn begin_dockspace(&mut self, label: &str, size: [f32; 2]) {
        let scope = self
            .emitting()
            .then(|| self.ctx.ui().begin_dockspace(label, size));
        self.scopes.push(Scope::Dockspace(scope));
    }

    pub fn begin_dock_panel(
        &mut self,
        label: &str,
        floating_size: [f32; 2],
        placement: &mut DockState,
    ) {
        let scope = if self.emitting() {
            let Some(Scope::Dockspace(Some(dock))) = self.scopes.last_mut() else {
                panic!("begin_dock_panel requires an open dockspace");
            };
            Some(
                self.ctx
                    .ui()
                    .begin_dock_panel(dock, label, floating_size, placement),
            )
        } else {
            None
        };
        self.scopes.push(Scope::DockPanel(scope));
    }

    pub fn end_dock_panel(&mut self) {
        match self.scopes.pop() {
            Some(Scope::DockPanel(Some(scope))) => self.ctx.ui().end_dock_panel(scope),
            Some(Scope::DockPanel(None)) => {}
            Some(other) => panic!("end_dock_panel found {}", other.name()),
            None => panic!("end_dock_panel with no panel open"),
        }
    }

    pub fn end_dockspace(&mut self) {
        match self.scopes.pop() {
            Some(Scope::Dockspace(Some(scope))) => self.ctx.ui().end_dockspace(scope),
            Some(Scope::Dockspace(None)) => {}
            Some(other) => panic!("end_dockspace found {}", other.name()),
            None => panic!("end_dockspace with no dockspace open"),
        }
    }

    pub fn begin_group_box(&mut self, label: &str, size: [f32; 2]) {
        let token = if self.emitting() {
            Some(self.ctx.ui().begin_group_box(label, size))
        } else {
            None
        };
        self.scopes.push(Scope::GroupBox(token));
    }

    pub fn end_group_box(&mut self) {
        match self.scopes.pop() {
            Some(Scope::GroupBox(Some(token))) => self.ctx.ui().end_group_box(token),
            Some(Scope::GroupBox(None)) => {}
            Some(other) => panic!(
                "Gui::end_group_box called while the innermost scope is a {}",
                other.name()
            ),
            None => panic!("Gui::end_group_box called with no group box open"),
        }
    }

    pub fn begin_preview_canvas(&mut self, label: &str, size: [f32; 2]) -> [f32; 4] {
        let (token, rect) = if self.emitting() {
            let (token, rect) = self.ctx.ui().begin_preview_canvas(label, size);
            (Some(token), rect)
        } else {
            (None, [0.0; 4])
        };
        self.scopes.push(Scope::PreviewCanvas(token));
        rect
    }

    pub fn end_preview_canvas(&mut self) {
        match self.scopes.pop() {
            Some(Scope::PreviewCanvas(Some(token))) => self.ctx.ui().end_preview_canvas(token),
            Some(Scope::PreviewCanvas(None)) => {}
            Some(other) => panic!(
                "Gui::end_preview_canvas called while the innermost scope is a {}",
                other.name()
            ),
            None => panic!("Gui::end_preview_canvas called with no preview canvas open"),
        }
    }

    pub fn begin_disabled(&mut self, label: &str, disabled: bool) {
        let token = if self.emitting() {
            Some(self.ctx.ui().begin_disabled(label, disabled))
        } else {
            None
        };
        self.scopes.push(Scope::Disabled(token));
    }

    pub fn end_disabled(&mut self) {
        match self.scopes.pop() {
            Some(Scope::Disabled(Some(token))) => self.ctx.ui().end_disabled(token),
            Some(Scope::Disabled(None)) => {}
            Some(other) => panic!(
                "Gui::end_disabled called while the innermost scope is a {}",
                other.name()
            ),
            None => panic!("Gui::end_disabled called with no disabled section open"),
        }
    }

    pub fn begin_tab_bar(&mut self, label: &str) {
        if self.emitting() {
            self.ctx.ui().begin_tab_bar(label);
            self.scopes.push(Scope::TabBar(Some(())));
        } else {
            self.scopes.push(Scope::TabBar(None));
        }
    }

    pub fn tab(&mut self, title: &str) -> bool {
        self.emitting() && self.ctx.ui().tab(title)
    }

    pub fn end_tab_bar(&mut self) {
        match self.scopes.pop() {
            Some(Scope::TabBar(scope)) => {
                if scope.is_some() {
                    self.ctx.ui().end_tab_bar();
                }
            }
            Some(other) => panic!(
                "Gui::end_tab_bar called while the innermost open scope is \
                 a {}",
                other.name()
            ),
            None => panic!(
                "Gui::end_tab_bar called with no tab bar open: every end \
                 must follow its begin"
            ),
        }
    }

    pub fn draw_line(&mut self, p0: [f32; 2], p1: [f32; 2], col: [u8; 4], thickness: f32) {
        if self.emitting() {
            self.ctx.ui().draw_line(p0, p1, col, thickness);
        }
    }

    pub fn draw_rect_filled(&mut self, min: [f32; 2], max: [f32; 2], col: [u8; 4]) {
        if self.emitting() {
            self.ctx.ui().draw_rect_filled(min, max, col);
        }
    }

    pub fn draw_rect_outline(
        &mut self,
        min: [f32; 2],
        max: [f32; 2],
        col: [u8; 4],
        thickness: f32,
    ) {
        if self.emitting() {
            self.ctx.ui().draw_rect_outline(min, max, col, thickness);
        }
    }

    pub fn draw_circle_filled(&mut self, center: [f32; 2], radius: f32, col: [u8; 4]) {
        if self.emitting() {
            self.ctx.ui().draw_circle_filled(center, radius, col);
        }
    }

    pub fn draw_circle_outline(
        &mut self,
        center: [f32; 2],
        radius: f32,
        col: [u8; 4],
        thickness: f32,
    ) {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_circle_outline(center, radius, col, thickness);
        }
    }

    pub fn draw_arc(
        &mut self,
        center: [f32; 2],
        radius: f32,
        angle_range: RangeInclusive<f32>,
        col: [u8; 4],
        thickness: f32,
    ) {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_arc(center, radius, angle_range, col, thickness);
        }
    }

    pub fn draw_polygon_filled(&mut self, points: &[[f32; 2]], col: [u8; 4]) {
        if self.emitting() {
            self.ctx.ui().draw_polygon_filled(points, col);
        }
    }

    pub fn draw_image(
        &mut self,
        texture: TextureId,
        min: [f32; 2],
        max: [f32; 2],
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        tint: [u8; 4],
    ) {
        if self.emitting() {
            self.ctx
                .ui()
                .draw_image(texture, min, max, uv_min, uv_max, tint);
        }
    }

    pub fn draw_text_at(&mut self, pos: [f32; 2], text: &str, col: [u8; 4]) {
        if self.emitting() {
            self.ctx.ui().draw_text_at(pos, text, col);
        }
    }

    pub fn measure_text(&mut self, text: &str) -> [f32; 2] {
        self.ctx.ui().measure_text(text)
    }

    pub fn widget_id(&mut self, label: &str) -> Id {
        self.ctx.ui().widget_id(label)
    }

    pub fn allocate(&mut self, size: [f32; 2]) -> [f32; 4] {
        if self.emitting() {
            self.ctx.ui().allocate(size)
        } else {
            [0.0; 4]
        }
    }

    pub fn is_hovered(&mut self, rect: [f32; 4]) -> bool {
        self.ctx.ui().is_hovered(rect)
    }

    pub fn capture(&mut self, id: Id) {
        if self.emitting() {
            self.ctx.ui().capture(id);
        }
    }

    pub fn release_capture(&mut self, id: Id) {
        if self.emitting() {
            self.ctx.ui().release_capture(id);
        }
    }

    pub fn is_captured(&mut self, id: Id) -> bool {
        self.ctx.ui().is_captured(id)
    }

    pub fn focusable(&mut self, id: Id, rect: [f32; 4]) {
        if self.emitting() {
            self.ctx.ui().focusable(id, rect);
        }
    }

    pub fn is_focused(&mut self, id: Id) -> bool {
        self.emitting() && self.ctx.ui().is_focused(id)
    }

    pub fn focus(&mut self, id: Id) {
        if self.emitting() {
            self.ctx.ui().focus(id);
        }
    }

    pub fn keyboard_activated(&mut self, id: Id) -> bool {
        self.emitting() && self.ctx.ui().keyboard_activated(id)
    }

    pub fn drag_source(&mut self, id: Id, rect: [f32; 4], payload: DragPayload) -> bool {
        self.emitting() && self.ctx.ui().drag_source(id, rect, payload)
    }

    pub fn drag_target(&mut self, rect: [f32; 4], type_id: u64) -> Option<DragPayload> {
        if self.emitting() {
            self.ctx.ui().drag_target(rect, type_id)
        } else {
            None
        }
    }

    pub fn mouse_pos(&mut self) -> [f32; 2] {
        self.ctx.ui().mouse_pos()
    }

    pub fn mouse_pressed(&mut self, button: usize) -> bool {
        self.ctx.ui().mouse_pressed(button)
    }

    pub fn mouse_down(&mut self, button: usize) -> bool {
        self.ctx.ui().mouse_down(button)
    }

    pub fn mouse_released(&mut self, button: usize) -> bool {
        self.ctx.ui().mouse_released(button)
    }
}
