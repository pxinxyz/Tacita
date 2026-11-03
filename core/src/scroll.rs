use crate::context::Context;
use crate::id::Id;
use crate::style::Style;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollFlags {
    pub scrollable: bool,
    pub scrollbar: bool,
}

impl ScrollFlags {
    pub const NONE: Self = Self {
        scrollable: false,
        scrollbar: false,
    };

    pub const WHEEL: Self = Self {
        scrollable: true,
        scrollbar: false,
    };

    pub const BAR: Self = Self {
        scrollable: true,
        scrollbar: true,
    };
}

impl Default for ScrollFlags {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrollTarget {
    Widget,
    Window,
}

pub(crate) struct ScrollContainerEntry {
    pub(crate) id: Id,
    pub(crate) target: ScrollTarget,
    pub(crate) rect: [f32; 4],
    pub(crate) layer_owns_point: bool,
    pub(crate) max_offset: f32,
    pub(crate) line_height: f32,
}

pub(crate) const MIN_SCROLLBAR_THUMB: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarSkin {
    pub track_inset: f32,
    pub thumb_inset: f32,
    pub track_rounding: f32,
    pub thumb_rounding: f32,
    pub min_thumb: f32,
    pub track_click: ScrollbarTrackClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarTrackClick {
    Jump,
    Page,
}

impl Default for ScrollbarSkin {
    fn default() -> Self {
        Self {
            track_inset: 0.0,
            thumb_inset: 0.0,
            track_rounding: 0.0,
            thumb_rounding: -1.0,
            min_thumb: MIN_SCROLLBAR_THUMB,
            track_click: ScrollbarTrackClick::Jump,
        }
    }
}

pub(crate) const SCROLLBAR_TAG: &str = "##scrollbar";

pub(crate) struct ScrollGeometry {
    pub(crate) content_rect: [f32; 4],
    pub(crate) scrollbar_rect: Option<[f32; 4]>,
    pub(crate) max_offset: f32,
}

pub(crate) fn content_inset(style: &Style, rect: [f32; 4]) -> f32 {
    let r = if style.corner_rounding.is_finite() {
        style
            .corner_rounding
            .min((rect[2] - rect[0]).min(rect[3] - rect[1]) / 2.0)
    } else {
        0.0
    };
    let crescent = if r > 0.0 {
        r * (1.0 - core::f32::consts::FRAC_1_SQRT_2)
    } else {
        0.0
    };
    (style.border_size.max(0.0) + crescent).min((rect[2] - rect[0]).min(rect[3] - rect[1]) / 2.0)
}

pub(crate) fn geometry(
    style: &Style,
    rect: [f32; 4],
    inset: f32,
    flags: ScrollFlags,
    content_height: f32,
) -> ScrollGeometry {
    geometry_axis(style, rect, inset, flags, content_height, 1)
}

pub(crate) fn geometry_axis(
    style: &Style,
    rect: [f32; 4],
    inset: f32,
    flags: ScrollFlags,
    content_extent: f32,
    axis: usize,
) -> ScrollGeometry {
    let cross = axis ^ 1;
    let half_w = (rect[2] - rect[0]).max(0.0) / 2.0;
    let half_h = (rect[3] - rect[1]).max(0.0) / 2.0;
    let inset = inset.max(0.0).min(half_w).min(half_h);
    let inset_rect = [
        rect[0] + inset,
        rect[1] + inset,
        rect[2] - inset,
        rect[3] - inset,
    ];

    let visible_extent = (inset_rect[axis + 2] - inset_rect[axis]).max(0.0);
    let want_bar = flags.scrollable && flags.scrollbar && content_extent > visible_extent;
    let bar_size = style.scrollbar_size.max(0.0);
    let cross_extent = inset_rect[cross + 2] - inset_rect[cross];

    let (content_rect, scrollbar_rect) = if want_bar && cross_extent > bar_size {
        let split = inset_rect[cross + 2] - bar_size;
        let mut content_rect = inset_rect;
        content_rect[cross + 2] = split;
        let mut scrollbar_rect = inset_rect;
        scrollbar_rect[cross] = split;
        (content_rect, Some(scrollbar_rect))
    } else {
        (inset_rect, None)
    };

    let max_offset = if flags.scrollable {
        (content_extent - visible_extent).max(0.0)
    } else {
        0.0
    };

    ScrollGeometry {
        content_rect,
        scrollbar_rect,
        max_offset,
    }
}
fn thumb_extent(
    track_height: f32,
    visible_height: f32,
    content_height: f32,
    min_thumb: f32,
) -> f32 {
    let track_height = track_height.max(0.0);
    if content_height <= 0.0 || content_height <= visible_height {
        return track_height;
    }
    let ratio = visible_height.max(0.0) / content_height;
    (ratio * track_height).max(min_thumb).min(track_height)
}

pub(crate) fn thumb_position(
    track_height: f32,
    thumb_extent: f32,
    offset: f32,
    max_offset: f32,
) -> f32 {
    if max_offset <= 0.0 {
        return 0.0;
    }
    let travel = (track_height - thumb_extent).max(0.0);
    (offset / max_offset) * travel
}

pub(crate) fn offset_from_thumb_position(
    track_height: f32,
    thumb_extent: f32,
    thumb_position: f32,
    max_offset: f32,
) -> f32 {
    let travel = (track_height - thumb_extent).max(0.0);
    if travel <= 0.0 || max_offset <= 0.0 {
        return 0.0;
    }
    (thumb_position / travel * max_offset).clamp(0.0, max_offset)
}

pub(crate) fn scrollbar(
    ctx: &mut Context,
    scope_id: Id,
    track_rect: [f32; 4],
    offset: f32,
    max_offset: f32,
    visible_height: f32,
    content_height: f32,
) -> f32 {
    scrollbar_axis(
        ctx,
        scope_id,
        track_rect,
        offset,
        max_offset,
        visible_height,
        content_height,
        1,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn scrollbar_axis(
    ctx: &mut Context,
    scope_id: Id,
    track_rect: [f32; 4],
    offset: f32,
    max_offset: f32,
    visible_height: f32,
    content_height: f32,
    axis: usize,
) -> f32 {
    let cross = axis ^ 1;
    let style = ctx.style;
    let skin = ctx.scrollbar_skin;
    let track_top = track_rect[axis];
    let track_height = (track_rect[axis + 2] - track_rect[axis]).max(0.0);
    let thumb_h = thumb_extent(track_height, visible_height, content_height, skin.min_thumb);
    let live = max_offset > 0.0 && track_height > thumb_h;

    let mouse = ctx.input.mouse_pos;
    let left_down = ctx.input.mouse_down[0];
    let left_pressed = left_down && !ctx.prev_mouse_down[0];

    let mut new_offset = offset;
    let mut drawn_offset = if live {
        offset.clamp(0.0, max_offset)
    } else {
        0.0
    };

    if live {
        let thumb_top = track_top + thumb_position(track_height, thumb_h, drawn_offset, max_offset);
        if left_pressed && crate::ui::hovered(ctx, track_rect) {
            let mut thumb_rect = track_rect;
            thumb_rect[axis] = thumb_top;
            thumb_rect[axis + 2] = thumb_top + thumb_h;
            let on_thumb = crate::ui::point_in_rect(mouse, thumb_rect);
            let grab = if on_thumb {
                mouse[axis] - thumb_top
            } else {
                thumb_h / 2.0
            };
            ctx.set_active(scope_id);
            ctx.active_drag_offset = [0.0; 2];
            ctx.active_drag_offset[axis] = grab;
            if !on_thumb && skin.track_click == ScrollbarTrackClick::Page {
                new_offset = (offset
                    + if mouse[axis] < thumb_top {
                        -visible_height
                    } else {
                        visible_height
                    })
                .clamp(0.0, max_offset);
                drawn_offset = new_offset;
                ctx.clear_active();
            }
        }
        let held = ctx.active == Some(scope_id);
        if held && left_down {
            let grab = ctx.active_drag_offset[axis];
            new_offset = offset_from_thumb_position(
                track_height,
                thumb_h,
                mouse[axis] - track_top - grab,
                max_offset,
            );
            drawn_offset = new_offset;
        }
        if held && !left_down {
            ctx.clear_active();
        }
    }

    let hovered = live && crate::ui::hovered(ctx, track_rect);
    if hovered {
        ctx.hot = Some(scope_id);
    }
    let active = ctx.active == Some(scope_id);
    let thumb_bg = if active {
        style.scrollbar_thumb_active
    } else if hovered {
        style.scrollbar_thumb_hovered
    } else {
        style.scrollbar_thumb
    };

    let thumb_bg = ctx.widget_color(scope_id, thumb_bg);
    let thumb_top = track_top + thumb_position(track_height, thumb_h, drawn_offset, max_offset);
    let (font, draw_list) = crate::widgets::draw_parts(ctx);
    let mut track = track_rect;
    track[cross] += skin.track_inset;
    track[cross + 2] -= skin.track_inset;
    let mut thumb = track_rect;
    thumb[cross] += skin.thumb_inset;
    thumb[cross + 2] -= skin.thumb_inset;
    thumb[axis] = thumb_top;
    thumb[axis + 2] = thumb_top + thumb_h;
    if track[cross + 2] > track[cross] {
        crate::widgets::chrome_fill(
            font,
            draw_list,
            skin.track_rounding,
            track,
            style.scrollbar_bg,
        );
    }
    if thumb[cross + 2] > thumb[cross] {
        crate::widgets::chrome_fill(
            font,
            draw_list,
            if skin.thumb_rounding < 0.0 {
                style.corner_rounding
            } else {
                skin.thumb_rounding
            },
            thumb,
            thumb_bg,
        );
    }
    new_offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_geometry_is_the_transpose_of_vertical_geometry() {
        let style = Style::default();
        let rect = [10., 20., 110., 220.];
        let transpose = |r: [f32; 4]| [r[1], r[0], r[3], r[2]];
        let vertical = geometry(&style, transpose(rect), 0., ScrollFlags::BAR, 400.);
        let horizontal = geometry_axis(&style, rect, 0., ScrollFlags::BAR, 400., 0);
        assert_eq!(horizontal.content_rect, transpose(vertical.content_rect));
        assert_eq!(
            horizontal.scrollbar_rect,
            vertical.scrollbar_rect.map(transpose)
        );
        assert_eq!(horizontal.max_offset, vertical.max_offset);
    }

    #[test]
    fn skin_minimum_thumb_changes_geometry_without_changing_default() {
        assert_eq!(thumb_extent(100.0, 10.0, 100.0, MIN_SCROLLBAR_THUMB), 16.0);
        assert_eq!(thumb_extent(100.0, 10.0, 100.0, 28.0), 28.0);
        assert_eq!(thumb_extent(12.0, 10.0, 100.0, 28.0), 12.0);
    }

    fn style_with(scrollbar_size: f32, corner_rounding: f32, border_size: f32) -> Style {
        Style {
            scrollbar_size,
            corner_rounding,
            border_size,
            ..Style::default()
        }
    }

    #[test]
    fn scroll_flags_constants() {
        assert_eq!(
            ScrollFlags::NONE,
            ScrollFlags {
                scrollable: false,
                scrollbar: false
            }
        );
        assert_eq!(
            ScrollFlags::WHEEL,
            ScrollFlags {
                scrollable: true,
                scrollbar: false
            }
        );
        assert_eq!(
            ScrollFlags::BAR,
            ScrollFlags {
                scrollable: true,
                scrollbar: true
            }
        );
        assert_eq!(ScrollFlags::default(), ScrollFlags::NONE);
    }

    #[test]
    fn content_inset_parity_default_style() {
        let style = Style::default();
        let rect = [0.0, 0.0, 200.0, 100.0];
        let inset = content_inset(&style, rect);
        assert!((inset - 2.171_573).abs() < 1e-5, "got {inset}");
    }

    #[test]
    fn content_inset_parity_zero_rounding() {
        let style = style_with(12.0, 0.0, 3.0);
        let inset = content_inset(&style, [0.0, 0.0, 400.0, 300.0]);
        assert_eq!(inset, 3.0);
    }

    #[test]
    fn content_inset_parity_negative_border_clamped_to_zero() {
        let style = style_with(12.0, 0.0, -5.0);
        let inset = content_inset(&style, [0.0, 0.0, 400.0, 300.0]);
        assert_eq!(inset, 0.0);
    }

    #[test]
    fn content_inset_parity_non_finite_rounding_treated_as_zero() {
        let style = style_with(12.0, f32::NAN, 2.0);
        let inset = content_inset(&style, [0.0, 0.0, 400.0, 300.0]);
        assert_eq!(inset, 2.0);

        let style = style_with(12.0, f32::INFINITY, 2.0);
        let inset = content_inset(&style, [0.0, 0.0, 400.0, 300.0]);
        assert_eq!(inset, 2.0);
    }

    #[test]
    fn content_inset_parity_clamped_to_half_smaller_dimension() {
        let style = Style::default();
        let inset = content_inset(&style, [0.0, 0.0, 10.0, 6.0]);
        assert!((inset - 1.878_68).abs() < 1e-5, "got {inset}");

        let style = style_with(12.0, 100.0, 100.0);
        let inset = content_inset(&style, [0.0, 0.0, 10.0, 6.0]);
        assert_eq!(inset, 3.0, "clamped to half the smaller dimension");
    }

    #[test]
    fn content_inset_parity_matrix() {
        let cases: &[(f32, f32, [f32; 4])] = &[
            (0.0, 0.0, [0.0, 0.0, 50.0, 50.0]),
            (2.0, 1.0, [0.0, 0.0, 50.0, 50.0]),
            (8.0, 0.0, [0.0, 0.0, 20.0, 20.0]),
            (4.0, 2.0, [10.0, 10.0, 110.0, 60.0]),
            (0.0, 5.0, [0.0, 0.0, 8.0, 8.0]),
        ];
        for &(rounding, border, rect) in cases {
            let style = style_with(12.0, rounding, border);
            let w = rect[2] - rect[0];
            let h = rect[3] - rect[1];
            let half_min = w.min(h) / 2.0;
            let r = rounding.min(half_min);
            let crescent = if r > 0.0 {
                r * (1.0 - core::f32::consts::FRAC_1_SQRT_2)
            } else {
                0.0
            };
            let expected = (border.max(0.0) + crescent).min(half_min);
            let got = content_inset(&style, rect);
            assert!(
                (got - expected).abs() < 1e-6,
                "rounding={rounding} border={border} rect={rect:?}: expected {expected}, got {got}"
            );
        }
    }

    #[test]
    fn geometry_none_has_no_scrollbar_and_zero_max_offset() {
        let style = style_with(12.0, 0.0, 0.0);
        let rect = [0.0, 0.0, 100.0, 50.0];
        let g = geometry(&style, rect, 0.0, ScrollFlags::NONE, 500.0);
        assert!(g.scrollbar_rect.is_none());
        assert_eq!(
            g.max_offset, 0.0,
            "NONE never scrolls, regardless of overflow"
        );
        assert_eq!(g.content_rect, rect);
    }

    #[test]
    fn geometry_wheel_has_no_scrollbar_but_scrolls() {
        let style = style_with(12.0, 0.0, 0.0);
        let rect = [0.0, 0.0, 100.0, 50.0];
        let g = geometry(&style, rect, 0.0, ScrollFlags::WHEEL, 500.0);
        assert!(g.scrollbar_rect.is_none());
        assert_eq!(g.content_rect, rect);
        assert_eq!(g.max_offset, 450.0, "500 content - 50 visible");
    }

    #[test]
    fn geometry_bar_reserves_scrollbar_strip_on_the_right() {
        let style = style_with(12.0, 0.0, 0.0);
        let rect = [0.0, 0.0, 100.0, 50.0];
        let g = geometry(&style, rect, 0.0, ScrollFlags::BAR, 500.0);
        let scrollbar_rect = g.scrollbar_rect.expect("BAR reserves a strip");
        assert_eq!(g.content_rect, [0.0, 0.0, 88.0, 50.0]);
        assert_eq!(scrollbar_rect, [88.0, 0.0, 100.0, 50.0]);
        assert_eq!(scrollbar_rect[1], g.content_rect[1]);
        assert_eq!(scrollbar_rect[3], g.content_rect[3]);
        assert_eq!(g.max_offset, 450.0);
    }

    #[test]
    fn geometry_bar_on_narrow_rect_yields_no_scrollbar_and_no_inversion() {
        let style = style_with(12.0, 0.0, 0.0);
        let rect = [0.0, 0.0, 10.0, 50.0];
        let g = geometry(&style, rect, 0.0, ScrollFlags::BAR, 500.0);
        assert!(g.scrollbar_rect.is_none());
        assert_eq!(
            g.content_rect, rect,
            "unshrunk when too narrow to host the strip"
        );
        assert!(g.content_rect[2] >= g.content_rect[0], "never inverted");
    }

    #[test]
    fn geometry_bar_exactly_scrollbar_size_wide_yields_no_scrollbar() {
        let style = style_with(12.0, 0.0, 0.0);
        let rect = [0.0, 0.0, 12.0, 50.0];
        let g = geometry(&style, rect, 0.0, ScrollFlags::BAR, 500.0);
        assert!(g.scrollbar_rect.is_none());
        assert_eq!(g.content_rect, rect);
    }

    #[test]
    fn geometry_applies_inset_before_the_scrollbar_split() {
        let style = style_with(10.0, 0.0, 0.0);
        let rect = [0.0, 0.0, 100.0, 100.0];
        let g = geometry(&style, rect, 5.0, ScrollFlags::BAR, 500.0);
        assert_eq!(g.content_rect, [5.0, 5.0, 85.0, 95.0]);
        assert_eq!(g.scrollbar_rect, Some([85.0, 5.0, 95.0, 95.0]));
    }

    #[test]
    fn geometry_bar_with_fitting_content_reserves_nothing_and_releases_the_strip() {
        let style = style_with(12.0, 0.0, 0.0);
        let rect = [0.0, 0.0, 100.0, 50.0];
        let g = geometry(&style, rect, 0.0, ScrollFlags::BAR, 30.0);
        assert!(
            g.scrollbar_rect.is_none(),
            "content that fits draws no track and no thumb"
        );
        assert_eq!(
            g.content_rect, rect,
            "the reserved strip is released back to the content (full inset width)"
        );
        assert_eq!(g.max_offset, 0.0);

        let g = geometry(&style, rect, 0.0, ScrollFlags::BAR, 50.0);
        assert!(g.scrollbar_rect.is_none(), "content == visible still fits");
        assert_eq!(g.content_rect, rect);

        let g = geometry(
            &style,
            [0.0, 0.0, 100.0, 100.0],
            5.0,
            ScrollFlags::BAR,
            10.0,
        );
        assert!(g.scrollbar_rect.is_none());
        assert_eq!(g.content_rect, [5.0, 5.0, 95.0, 95.0]);
    }

    #[test]
    fn geometry_bar_with_overflowing_content_narrows_by_exactly_scrollbar_size() {
        let style = style_with(12.0, 0.0, 0.0);
        let rect = [0.0, 0.0, 100.0, 50.0];
        let fits = geometry(&style, rect, 0.0, ScrollFlags::BAR, 30.0);
        let overflows = geometry(&style, rect, 0.0, ScrollFlags::BAR, 50.1);
        let track = overflows
            .scrollbar_rect
            .expect("overflowing content reserves a strip");
        assert_eq!(
            (fits.content_rect[2] - fits.content_rect[0])
                - (overflows.content_rect[2] - overflows.content_rect[0]),
            style.scrollbar_size,
            "overflow narrows the content by exactly Style::scrollbar_size"
        );
        assert_eq!(track, [88.0, 0.0, 100.0, 50.0]);
    }

    #[test]
    fn geometry_bar_threshold_is_content_height_strictly_over_visible_height() {
        let style = style_with(12.0, 0.0, 0.0);
        let rect = [0.0, 0.0, 100.0, 50.0];
        for (content, want) in [
            (0.0f32, false),
            (49.9, false),
            (50.0, false),
            (50.1, true),
            (500.0, true),
        ] {
            let g = geometry(&style, rect, 0.0, ScrollFlags::BAR, content);
            assert_eq!(
                g.scrollbar_rect.is_some(),
                want,
                "content_height {content}: expected bar = {want}"
            );
        }
    }

    #[test]
    fn geometry_degenerate_rect_never_inverts() {
        let style = style_with(12.0, 4.0, 1.0);
        let rect = [0.0, 0.0, 4.0, 4.0];
        let g = geometry(&style, rect, 50.0, ScrollFlags::BAR, 10.0);
        assert!(g.content_rect[2] >= g.content_rect[0]);
        assert!(g.content_rect[3] >= g.content_rect[1]);
        if let Some(sb) = g.scrollbar_rect {
            assert!(sb[2] >= sb[0]);
            assert!(sb[3] >= sb[1]);
        }
    }

    #[test]
    fn geometry_zero_size_rect_never_inverts_or_nans() {
        let style = style_with(12.0, 0.0, 0.0);
        let rect = [10.0, 10.0, 10.0, 10.0];
        let g = geometry(&style, rect, 0.0, ScrollFlags::BAR, 100.0);
        assert!(g.content_rect[2] >= g.content_rect[0]);
        assert!(g.content_rect[3] >= g.content_rect[1]);
        assert!(g.max_offset.is_finite());
        assert!(!g.max_offset.is_nan());
    }

    #[test]
    fn thumb_extent_half_visible_content_is_half_track() {
        let extent = thumb_extent(200.0, 50.0, 100.0, MIN_SCROLLBAR_THUMB);
        assert_eq!(extent, 100.0);
    }

    #[test]
    fn thumb_extent_tiny_ratio_floors_at_minimum() {
        let extent = thumb_extent(200.0, 1.0, 10_000.0, MIN_SCROLLBAR_THUMB);
        assert_eq!(extent, MIN_SCROLLBAR_THUMB);
    }

    #[test]
    fn thumb_extent_content_fits_is_full_track() {
        let extent = thumb_extent(200.0, 100.0, 80.0, MIN_SCROLLBAR_THUMB);
        assert_eq!(extent, 200.0);
        let extent = thumb_extent(200.0, 100.0, 0.0, MIN_SCROLLBAR_THUMB);
        assert_eq!(extent, 200.0);
        let extent = thumb_extent(200.0, 100.0, -5.0, MIN_SCROLLBAR_THUMB);
        assert_eq!(extent, 200.0);
    }

    #[test]
    fn thumb_extent_never_exceeds_track_even_with_minimum_floor() {
        let extent = thumb_extent(10.0, 1.0, 10_000.0, MIN_SCROLLBAR_THUMB);
        assert_eq!(extent, 10.0);
    }

    #[test]
    fn thumb_extent_zero_track_is_zero_and_finite() {
        let extent = thumb_extent(0.0, 50.0, 100.0, MIN_SCROLLBAR_THUMB);
        assert_eq!(extent, 0.0);
        assert!(extent.is_finite());
    }

    #[test]
    fn thumb_position_zero_offset_is_zero() {
        let pos = thumb_position(200.0, 50.0, 0.0, 450.0);
        assert_eq!(pos, 0.0);
    }

    #[test]
    fn thumb_position_max_offset_reaches_track_minus_thumb() {
        let pos = thumb_position(200.0, 50.0, 450.0, 450.0);
        assert_eq!(pos, 150.0);
    }

    #[test]
    fn thumb_position_zero_max_offset_is_zero_no_div_by_zero() {
        let pos = thumb_position(200.0, 50.0, 0.0, 0.0);
        assert_eq!(pos, 0.0);
        assert!(pos.is_finite());

        let pos = thumb_position(200.0, 200.0, 0.0, 0.0);
        assert_eq!(pos, 0.0);
    }

    #[test]
    fn offset_from_thumb_position_round_trips_across_a_sweep() {
        let track = 200.0;
        let thumb = 50.0;
        let max_offset = 450.0;
        for i in 0..=20 {
            let offset = max_offset * (i as f32) / 20.0;
            let pos = thumb_position(track, thumb, offset, max_offset);
            let back = offset_from_thumb_position(track, thumb, pos, max_offset);
            assert!(
                (back - offset).abs() < 1e-3,
                "offset {offset} -> pos {pos} -> back {back}"
            );
        }
    }

    #[test]
    fn offset_from_thumb_position_clamps_outside_the_valid_range() {
        let track = 200.0;
        let thumb = 50.0;
        let max_offset = 450.0;
        let over = offset_from_thumb_position(track, thumb, 1000.0, max_offset);
        assert_eq!(over, max_offset);
        let under = offset_from_thumb_position(track, thumb, -100.0, max_offset);
        assert_eq!(under, 0.0);
    }

    #[test]
    fn offset_from_thumb_position_thumb_fills_track_returns_zero() {
        let offset = offset_from_thumb_position(200.0, 200.0, 50.0, 450.0);
        assert_eq!(offset, 0.0);
        let offset = offset_from_thumb_position(200.0, 250.0, 50.0, 450.0);
        assert_eq!(offset, 0.0);
    }

    #[test]
    fn offset_from_thumb_position_zero_max_offset_is_zero() {
        let offset = offset_from_thumb_position(200.0, 50.0, 100.0, 0.0);
        assert_eq!(offset, 0.0);
    }

    #[test]
    fn scrollbar_tag_is_id_only_and_unique_per_container_scope() {
        use crate::id::{split_label, IdStack};

        assert_eq!(split_label(SCROLLBAR_TAG).0, "");

        let mut a = IdStack::new();
        a.push("Window A");
        let mut a_again = IdStack::new();
        a_again.push("Window A");
        let mut b = IdStack::new();
        b.push("Window B");
        assert_eq!(a.id(SCROLLBAR_TAG), a_again.id(SCROLLBAR_TAG));
        assert_ne!(a.id(SCROLLBAR_TAG), b.id(SCROLLBAR_TAG));

        let child = a.id("c");
        let mut nested = IdStack::new();
        nested.push("Window A");
        nested.push_id(child);
        assert_ne!(nested.id(SCROLLBAR_TAG), a.id(SCROLLBAR_TAG));
    }

    #[test]
    fn geometry_clamps_negative_inset_on_degenerate_rect() {
        let style = Style::default();
        let rect = [0.0, 0.0, -100.0, 50.0];
        let inset = content_inset(&style, rect);
        assert!(
            inset < 0.0,
            "test premise: inset must be negative, got {inset}"
        );

        let g = geometry(&style, rect, inset, ScrollFlags::WHEEL, 1000.0);

        assert_eq!(g.content_rect[0], 0.0);
        assert_eq!(g.content_rect[2], -100.0);
        assert!(
            g.content_rect[2] < g.content_rect[0],
            "horizontal extent remains inverted, inherited from the input rect"
        );

        assert!(
            g.content_rect[3] >= g.content_rect[1],
            "bottom >= top (height is non-inverted)"
        );

        assert_eq!(
            g.max_offset, 950.0,
            "max_offset reflects clamped inset (new: 950, old non-clamped: 850)"
        );
    }
}
