use sibilant_core::project::TrackColor;

use super::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub name: &'static str,
    pub background: Color,
    pub surface: Color,
    pub raised: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub record: Color,
    pub solo: Color,
    pub track_colors: [Color; 16],
    pub on_track_color: Color,
}

impl Palette {
    pub const GRAPHITE: Self = Self {
        name: "Graphite",
        background: Color::from_hex(0x17_18_1B),
        surface: Color::from_hex(0x1F_20_24),
        raised: Color::from_hex(0x2A_2B_30),
        border: Color::from_hex(0x34_36_3C),
        text: Color::from_hex(0xE8_E9_EC),
        text_muted: Color::from_hex(0xA3_A6_AE),
        accent: Color::from_hex(0x8B_7C_F6),
        on_accent: Color::from_hex(0x17_18_1B),
        record: Color::from_hex(0xE5_48_4D),
        solo: Color::from_hex(0xF5_D9_0A),
        track_colors: [
            Color::from_hex(0xF8_71_71),
            Color::from_hex(0x2D_D4_BF),
            Color::from_hex(0xE8_79_F9),
            Color::from_hex(0x4A_DE_80),
            Color::from_hex(0xA7_8B_FA),
            Color::from_hex(0xFA_CC_15),
            Color::from_hex(0x60_A5_FA),
            Color::from_hex(0xFB_92_3C),
            Color::from_hex(0x22_D3_EE),
            Color::from_hex(0xF4_72_B6),
            Color::from_hex(0x34_D3_99),
            Color::from_hex(0xC0_84_FC),
            Color::from_hex(0xA3_E6_35),
            Color::from_hex(0x81_8C_F8),
            Color::from_hex(0xFB_BF_24),
            Color::from_hex(0x38_BD_F8),
        ],
        on_track_color: Color::from_hex(0x17_18_1B),
    };

    pub fn track_color(&self, color: TrackColor) -> Color {
        self.track_colors[usize::from(color.index())]
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::GRAPHITE
    }
}

impl From<Palette> for iced::Theme {
    fn from(palette: Palette) -> Self {
        Self::custom(
            palette.name,
            iced::theme::Palette {
                background: palette.background.into(),
                text: palette.text.into(),
                primary: palette.accent.into(),
                success: palette.accent.into(),
                warning: palette.solo.into(),
                danger: palette.record.into(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use sibilant_core::project::TrackColor;

    use crate::theme::{Color, Palette};

    const PALETTE: Palette = Palette::GRAPHITE;

    fn surfaces() -> [(&'static str, Color); 3] {
        [
            ("background", PALETTE.background),
            ("surface", PALETTE.surface),
            ("raised", PALETTE.raised),
        ]
    }

    #[test]
    fn the_default_palette_is_graphite() {
        assert_eq!(Palette::default(), Palette::GRAPHITE);
    }

    #[test]
    fn the_accent_is_violet() {
        assert_eq!(PALETTE.accent, Color::from_hex(0x8B_7C_F6));
    }

    #[test]
    fn text_is_easy_to_read_on_each_surface() {
        for (name, surface) in surfaces() {
            let text = PALETTE.text.contrast_ratio(surface);
            let muted = PALETTE.text_muted.contrast_ratio(surface);
            assert!(text >= 7.0, "text on {name}: {text:.2}");
            assert!(muted >= 4.5, "muted text on {name}: {muted:.2}");
        }
    }

    #[test]
    fn the_accent_and_the_status_colors_stand_out_on_each_surface() {
        let marks = [
            ("accent", PALETTE.accent),
            ("record", PALETTE.record),
            ("solo", PALETTE.solo),
        ];
        for (surface_name, surface) in surfaces() {
            for (name, color) in marks {
                let contrast = color.contrast_ratio(surface);
                assert!(contrast >= 3.0, "{name} on {surface_name}: {contrast:.2}");
            }
        }
    }

    #[test]
    fn the_iced_theme_takes_its_colors_from_the_palette() {
        let iced = iced::Theme::from(PALETTE).palette();
        assert_eq!(iced.background, iced::Color::from(PALETTE.background));
        assert_eq!(iced.text, iced::Color::from(PALETTE.text));
        assert_eq!(iced.primary, iced::Color::from(PALETTE.accent));
        assert_eq!(iced.warning, iced::Color::from(PALETTE.solo));
        assert_eq!(iced.danger, iced::Color::from(PALETTE.record));
    }

    fn hue(color: Color) -> f64 {
        let (red, green, blue) = (color.red(), color.green(), color.blue());
        let (max, min) = (red.max(green).max(blue), red.min(green).min(blue));
        let chroma = f64::from(max - min);
        let [r, g, b] = [red, green, blue].map(f64::from);
        let sector = if max == min {
            0.0
        } else if max == red {
            ((g - b) / chroma).rem_euclid(6.0)
        } else if max == green {
            (b - r) / chroma + 2.0
        } else {
            (r - g) / chroma + 4.0
        };
        sector * 60.0
    }

    #[test]
    fn the_palette_has_a_value_for_each_track_color() {
        assert_eq!(PALETTE.track_colors.len(), usize::from(TrackColor::COUNT));
        let fourth = TrackColor::new(3).unwrap();
        assert_eq!(PALETTE.track_color(fourth), PALETTE.track_colors[3]);
    }

    #[test]
    fn each_track_color_stands_out_on_the_lanes() {
        for (index, color) in PALETTE.track_colors.into_iter().enumerate() {
            for (surface_name, surface) in surfaces() {
                let contrast = color.contrast_ratio(surface);
                assert!(
                    contrast >= 3.0,
                    "color {index} on {surface_name}: {contrast:.2}"
                );
            }
        }
    }

    #[test]
    fn text_on_each_track_color_is_easy_to_read() {
        for (index, color) in PALETTE.track_colors.into_iter().enumerate() {
            let contrast = PALETTE.on_track_color.contrast_ratio(color);
            assert!(contrast >= 4.5, "text on color {index}: {contrast:.2}");
        }
    }

    #[test]
    fn neighbouring_track_colors_have_clearly_different_hues() {
        let colors = PALETTE.track_colors;
        for (index, color) in colors.into_iter().enumerate() {
            let next = colors[(index + 1) % colors.len()];
            let difference = (hue(color) - hue(next)).abs();
            let angle = difference.min(360.0 - difference);
            assert!(angle >= 60.0, "colors {index} and next: {angle:.0} degrees");
        }
    }

    #[test]
    fn text_on_the_accent_is_easy_to_read() {
        let contrast = PALETTE.on_accent.contrast_ratio(PALETTE.accent);
        assert!(contrast >= 4.5, "text on the accent: {contrast:.2}");
    }
}
