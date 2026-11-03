use crate::animation::{lerp_color, Easing};
use crate::{context::Context, style::Style};

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeRole {
    Background,
    Panel,
    Input,
    Hover,
    Accent,
    Text,
    TitleText,
    ScrollTrack,
    ScrollThumb,
    ScrollHover,
    ScrollActive,
    Secondary,
    Muted,
    Success,
    Warning,
    Danger,
}
pub const THEME_ROLES: [ThemeRole; 16] = [
    ThemeRole::Background,
    ThemeRole::Panel,
    ThemeRole::Input,
    ThemeRole::Hover,
    ThemeRole::Accent,
    ThemeRole::Text,
    ThemeRole::TitleText,
    ThemeRole::ScrollTrack,
    ThemeRole::ScrollThumb,
    ThemeRole::ScrollHover,
    ThemeRole::ScrollActive,
    ThemeRole::Secondary,
    ThemeRole::Muted,
    ThemeRole::Success,
    ThemeRole::Warning,
    ThemeRole::Danger,
];
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeTokens {
    pub colors: [[u8; 4]; 16],
    pub frame_padding: [f32; 2],
    pub item_spacing: [f32; 2],
    pub window_padding: [f32; 2],
    pub slider_grab_size: f32,
    pub scrollbar_size: f32,
    pub corner_rounding: f32,
    pub border_size: f32,
    pub anti_aliasing: bool,
}
impl Default for ThemeTokens {
    fn default() -> Self {
        Self::from_style(Style::default())
    }
}
impl ThemeTokens {
    pub fn high_contrast() -> Self {
        Self {
            colors: [
                [0, 0, 0, 255],
                [0, 0, 0, 255],
                [64, 64, 64, 255],
                [0, 72, 120, 255],
                [0, 92, 144, 255],
                [255, 255, 255, 255],
                [255, 255, 255, 255],
                [0, 0, 0, 255],
                [180, 180, 180, 255],
                [255, 255, 255, 255],
                [0, 192, 255, 255],
                [224, 224, 224, 255],
                [176, 176, 176, 255],
                [80, 255, 144, 255],
                [255, 224, 80, 255],
                [255, 112, 136, 255],
            ],
            corner_rounding: 2.,
            border_size: 1.,
            ..Self::default()
        }
    }
    pub fn from_style(s: Style) -> Self {
        let mut secondary = s.text;
        secondary[3] = (secondary[3] as u16 * 2 / 3) as u8;
        let mut muted = s.text;
        muted[3] /= 3;
        Self {
            colors: [
                s.title_bg,
                s.window_bg,
                s.widget_bg,
                s.widget_bg_hovered,
                s.widget_bg_active,
                s.text,
                s.title_text,
                s.scrollbar_bg,
                s.scrollbar_thumb,
                s.scrollbar_thumb_hovered,
                s.scrollbar_thumb_active,
                secondary,
                muted,
                [0, 229, 153, s.text[3]],
                [255, 183, 77, s.text[3]],
                [255, 51, 88, s.text[3]],
            ],
            frame_padding: s.frame_padding,
            item_spacing: s.item_spacing,
            window_padding: s.window_padding,
            slider_grab_size: s.slider_grab_size,
            scrollbar_size: s.scrollbar_size,
            corner_rounding: s.corner_rounding,
            border_size: s.border_size,
            anti_aliasing: s.anti_aliasing,
        }
    }
    pub fn color(&self, role: ThemeRole) -> [u8; 4] {
        self.colors[role as usize]
    }
    pub fn set_color(&mut self, role: ThemeRole, color: [u8; 4]) {
        self.colors[role as usize] = color;
    }
    pub fn is_valid(&self) -> bool {
        self.frame_padding
            .iter()
            .chain(self.item_spacing.iter())
            .chain(self.window_padding.iter())
            .chain(
                [
                    self.slider_grab_size,
                    self.scrollbar_size,
                    self.corner_rounding,
                    self.border_size,
                ]
                .iter(),
            )
            .all(|v| v.is_finite() && *v >= 0.0)
    }
    pub fn resolve(&self) -> Style {
        assert!(self.is_valid(), "invalid theme metrics");
        let c = &self.colors;
        Style {
            title_bg: c[0],
            window_bg: c[1],
            widget_bg: c[2],
            widget_bg_hovered: c[3],
            widget_bg_active: c[4],
            text: c[5],
            title_text: c[6],
            scrollbar_bg: c[7],
            scrollbar_thumb: c[8],
            scrollbar_thumb_hovered: c[9],
            scrollbar_thumb_active: c[10],
            frame_padding: self.frame_padding,
            item_spacing: self.item_spacing,
            window_padding: self.window_padding,
            slider_grab_size: self.slider_grab_size,
            scrollbar_size: self.scrollbar_size,
            corner_rounding: self.corner_rounding,
            border_size: self.border_size,
            anti_aliasing: self.anti_aliasing,
        }
    }
    pub fn interpolate(self, other: Self, amount: f32) -> Self {
        assert!(self.is_valid() && other.is_valid(), "invalid theme tokens");
        let t = Easing::Linear.sample(amount);
        let mix = |a: f32, b: f32| {
            (f64::from(a) * (1.0 - f64::from(t)) + f64::from(b) * f64::from(t)) as f32
        };
        Self {
            colors: core::array::from_fn(|i| lerp_color(self.colors[i], other.colors[i], t)),
            frame_padding: core::array::from_fn(|i| {
                mix(self.frame_padding[i], other.frame_padding[i])
            }),
            item_spacing: core::array::from_fn(|i| {
                mix(self.item_spacing[i], other.item_spacing[i])
            }),
            window_padding: core::array::from_fn(|i| {
                mix(self.window_padding[i], other.window_padding[i])
            }),
            slider_grab_size: mix(self.slider_grab_size, other.slider_grab_size),
            scrollbar_size: mix(self.scrollbar_size, other.scrollbar_size),
            corner_rounding: mix(self.corner_rounding, other.corner_rounding),
            border_size: mix(self.border_size, other.border_size),
            anti_aliasing: if t < 0.5 {
                self.anti_aliasing
            } else {
                other.anti_aliasing
            },
        }
    }
}
impl Context {
    pub fn set_theme_tokens(&mut self, tokens: ThemeTokens) {
        assert!(
            !self.frame_active && tokens.is_valid(),
            "theme changes require valid tokens between frames"
        );
        self.style = tokens.resolve();
        self.theme = Some(tokens);
    }
    pub fn theme_tokens(&self) -> ThemeTokens {
        let mut tokens = ThemeTokens::from_style(self.style);
        if let Some(theme) = self.theme {
            tokens.colors[11..].copy_from_slice(&theme.colors[11..]);
        }
        tokens
    }
    pub fn theme_color(&self, role: ThemeRole) -> [u8; 4] {
        let mut color = self.theme_tokens().color(role);
        if self.disabled_depth > 0 && (role as u32) >= 11 {
            if let Some(theme) = self.theme {
                let base = theme.color(ThemeRole::Text)[3];
                color[3] = if base > 0 {
                    (color[3] as f32 * (self.style.text[3] as f32 / base as f32).min(1.0)) as u8
                } else {
                    0
                };
            }
        }
        color
    }
}
impl crate::ui::Ui<'_> {
    pub fn theme_color(&self, role: ThemeRole) -> [u8; 4] {
        self.ctx.theme_color(role)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn high_contrast_is_opaque_with_readable_control_text() {
        let t = ThemeTokens::high_contrast();
        assert!(t.is_valid());
        assert!(t.colors.iter().all(|c| c[3] == 255));
        fn luminance(c: [u8; 4]) -> f64 {
            let channel = |v: u8| {
                let s = f64::from(v) / 255.;
                if s <= 0.04045 {
                    s / 12.92
                } else {
                    ((s + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(c[0]) + 0.7152 * channel(c[1]) + 0.0722 * channel(c[2])
        }
        let text = luminance(t.color(ThemeRole::Text));
        for role in [
            ThemeRole::Background,
            ThemeRole::Panel,
            ThemeRole::Input,
            ThemeRole::Hover,
            ThemeRole::Accent,
        ] {
            assert!((text + 0.05) / (luminance(t.color(role)) + 0.05) >= 4.5);
        }
    }
    #[test]
    fn semantic_roles_follow_disabled_opacity_and_restore() {
        let mut ctx = crate::widgets::test_support::test_context();
        let mut tokens = ThemeTokens::default();
        tokens.set_color(ThemeRole::Text, [255; 4]);
        tokens.set_color(ThemeRole::Danger, [1, 2, 3, 255]);
        ctx.set_theme_tokens(tokens);
        ctx.new_frame(0.0);
        ctx.ui().window("Theme", crate::ScrollFlags::NONE, |ui| {
            ui.disabled("Disabled", true, |ui| {
                assert_eq!(ui.theme_color(ThemeRole::Danger), [1, 2, 3, 110]);
                ui.disabled("Nested", true, |ui| {
                    assert_eq!(ui.theme_color(ThemeRole::Danger), [1, 2, 3, 110]);
                });
            });
            assert_eq!(ui.theme_color(ThemeRole::Danger), [1, 2, 3, 255]);
        });
        ctx.render();
    }
    #[test]
    fn style_roundtrip_and_semantic_roles() {
        let style = Style::default();
        let tokens = ThemeTokens::from_style(style);
        assert_eq!(tokens.resolve(), style);
        let mut ctx = crate::widgets::test_support::test_context();
        let mut tokens = tokens;
        tokens.set_color(ThemeRole::Danger, [1, 2, 3, 4]);
        ctx.set_theme_tokens(tokens);
        ctx.style.text = [8; 4];
        assert_eq!(ctx.theme_color(ThemeRole::Danger), [1, 2, 3, 4]);
        assert_eq!(ctx.theme_color(ThemeRole::Text), [8; 4]);
        assert!(!ThemeTokens {
            border_size: f32::NAN,
            ..tokens
        }
        .is_valid());
        assert_eq!(tokens.interpolate(tokens, 0.5), tokens);
    }
}
