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
    };
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

    #[test]
    fn text_on_the_accent_is_easy_to_read() {
        let contrast = PALETTE.on_accent.contrast_ratio(PALETTE.accent);
        assert!(contrast >= 4.5, "text on the accent: {contrast:.2}");
    }
}
