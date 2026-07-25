#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub window_bg: [u8; 4],
    pub title_bg: [u8; 4],
    pub title_text: [u8; 4],
    pub widget_bg: [u8; 4],
    pub widget_bg_hovered: [u8; 4],
    pub widget_bg_active: [u8; 4],
    pub text: [u8; 4],
    pub frame_padding: [f32; 2],
    pub item_spacing: [f32; 2],
    pub window_padding: [f32; 2],
    pub slider_grab_size: f32,
    pub corner_rounding: f32,
    pub border_size: f32,
    pub anti_aliasing: bool,
}

impl Default for Style {
    // The M1 default palette, eyeballed once and enshrined forever. rgb(66,150,250) has seniority here.
    fn default() -> Self {
        Self {
            window_bg: [16, 16, 20, 255],
            title_bg: [27, 27, 34, 255],
            title_text: [224, 224, 224, 255],
            widget_bg: [41, 74, 122, 255],
            widget_bg_hovered: [66, 150, 250, 255],
            widget_bg_active: [15, 96, 214, 255],
            text: [240, 240, 240, 255],
            frame_padding: [6.0, 4.0],
            item_spacing: [8.0, 4.0],
            window_padding: [10.0, 10.0],
            slider_grab_size: 14.0,
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

        let colors: [[u8; 4]; 7] = [
            style.window_bg,
            style.title_bg,
            style.title_text,
            style.widget_bg,
            style.widget_bg_hovered,
            style.widget_bg_active,
            style.text,
        ];
        for col in colors {
            assert_eq!(col[3], 255, "default colors must be opaque");
        }

        assert!(style.frame_padding[0] > 0.0 && style.frame_padding[1] > 0.0);
        assert!(style.item_spacing[0] > 0.0 && style.item_spacing[1] > 0.0);
        assert!(style.window_padding[0] > 0.0 && style.window_padding[1] > 0.0);
        assert!(style.slider_grab_size > 0.0);
        assert!(style.corner_rounding > 0.0);
        assert!(style.border_size > 0.0);
    }
}
