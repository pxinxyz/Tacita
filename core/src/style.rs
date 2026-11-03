#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub window_bg: [u8; 4],
    pub title_bg: [u8; 4],
    pub title_text: [u8; 4],
    pub widget_bg: [u8; 4],
    pub widget_bg_hovered: [u8; 4],
    pub widget_bg_active: [u8; 4],
    pub text: [u8; 4],
    pub scrollbar_bg: [u8; 4],
    pub scrollbar_thumb: [u8; 4],
    pub scrollbar_thumb_hovered: [u8; 4],
    pub scrollbar_thumb_active: [u8; 4],
    pub frame_padding: [f32; 2],
    pub item_spacing: [f32; 2],
    pub window_padding: [f32; 2],
    pub slider_grab_size: f32,
    pub scrollbar_size: f32,
    pub corner_rounding: f32,
    pub border_size: f32,
    pub anti_aliasing: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            window_bg: [16, 16, 20, 255],
            title_bg: [27, 27, 34, 255],
            title_text: [224, 224, 224, 255],
            widget_bg: [41, 74, 122, 255],
            widget_bg_hovered: [66, 150, 250, 255],
            widget_bg_active: [15, 96, 214, 255],
            text: [240, 240, 240, 255],
            scrollbar_bg: [10, 10, 13, 255],
            scrollbar_thumb: [78, 78, 94, 255],
            scrollbar_thumb_hovered: [110, 110, 132, 255],
            scrollbar_thumb_active: [142, 142, 170, 255],
            frame_padding: [6.0, 4.0],
            item_spacing: [8.0, 4.0],
            window_padding: [10.0, 10.0],
            slider_grab_size: 14.0,
            scrollbar_size: 12.0,
            corner_rounding: 4.0,
            border_size: 1.0,
            anti_aliasing: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anti_aliasing_defaults_on() {
        assert!(Style::default().anti_aliasing);
    }

    #[test]
    fn default_style_is_opaque_and_sized() {
        let style = Style::default();

        let colors: [[u8; 4]; 11] = [
            style.window_bg,
            style.title_bg,
            style.title_text,
            style.widget_bg,
            style.widget_bg_hovered,
            style.widget_bg_active,
            style.text,
            style.scrollbar_bg,
            style.scrollbar_thumb,
            style.scrollbar_thumb_hovered,
            style.scrollbar_thumb_active,
        ];
        for col in colors {
            assert_eq!(col[3], 255, "default colors must be opaque");
        }

        assert!(style.frame_padding[0] > 0.0 && style.frame_padding[1] > 0.0);
        assert!(style.item_spacing[0] > 0.0 && style.item_spacing[1] > 0.0);
        assert!(style.window_padding[0] > 0.0 && style.window_padding[1] > 0.0);
        assert!(style.slider_grab_size > 0.0);
        assert!(style.scrollbar_size > 0.0);
        assert!(style.corner_rounding > 0.0);
        assert!(style.border_size > 0.0);
    }

    #[test]
    fn default_scrollbar_track_and_thumb_are_visually_distinct() {
        let s = Style::default();

        assert_ne!(
            s.scrollbar_bg, s.scrollbar_thumb,
            "the track and the idle thumb must not share a color"
        );
        assert_ne!(
            s.scrollbar_bg, s.widget_bg,
            "the track is its own color, not the widget fill"
        );

        fn luma(c: [u8; 4]) -> u32 {
            c[0] as u32 + c[1] as u32 + c[2] as u32
        }
        assert!(
            luma(s.scrollbar_bg) < luma(s.window_bg),
            "the track reads as a recess: darker than the window background"
        );
        assert!(
            luma(s.scrollbar_thumb) > luma(s.scrollbar_bg),
            "the idle thumb is clearly lighter than the track"
        );
        assert!(
            luma(s.scrollbar_thumb_hovered) > luma(s.scrollbar_thumb),
            "hover brightens the thumb"
        );
        assert!(
            luma(s.scrollbar_thumb_active) > luma(s.scrollbar_thumb_hovered),
            "dragging brightens it further"
        );
    }
}
