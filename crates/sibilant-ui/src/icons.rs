use iced::Font;

pub const FONT: Font = Font::with_name("lucide");
pub const FILE: &[u8] = include_bytes!("../fonts/lucide.ttf");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Loop,
    Metronome,
    Pause,
    Play,
    Record,
    ReturnToStart,
    Stop,
}

impl Icon {
    pub const ALL: [Self; 7] = [
        Self::Loop,
        Self::Metronome,
        Self::Pause,
        Self::Play,
        Self::Record,
        Self::ReturnToStart,
        Self::Stop,
    ];

    pub const fn codepoint(self) -> char {
        match self {
            Self::Loop => '\u{e146}',
            Self::Metronome => '\u{e6bc}',
            Self::Pause => '\u{e12e}',
            Self::Play => '\u{e13c}',
            Self::Record => '\u{e076}',
            Self::ReturnToStart => '\u{e15f}',
            Self::Stop => '\u{e167}',
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::font::Family;
    use skrifa::{FontRef, MetadataProvider};

    use crate::icons::{self, Icon};

    const LUCIDE_NAMES: [(Icon, &str); 7] = [
        (Icon::Loop, "repeat"),
        (Icon::Metronome, "metronome"),
        (Icon::Pause, "pause"),
        (Icon::Play, "play"),
        (Icon::Record, "circle"),
        (Icon::ReturnToStart, "skip-back"),
        (Icon::Stop, "square"),
    ];

    #[test]
    fn the_icon_font_provides_the_family_that_the_icons_use() {
        let mut database = fontdb::Database::new();
        database.load_font_data(icons::FILE.to_vec());
        let Family::Name(family) = icons::FONT.family else {
            panic!("the icon font has no family name");
        };
        assert!(database.faces().any(|face| face.families[0].0 == family));
    }

    #[test]
    fn each_icon_shows_the_lucide_glyph_with_its_name() {
        let font = FontRef::new(icons::FILE).unwrap();
        let names = font.glyph_names();
        for (icon, lucide_name) in LUCIDE_NAMES {
            let glyph = font.charmap().map(icon.codepoint());
            let name = glyph.and_then(|glyph| names.get(glyph));
            assert_eq!(
                name.as_ref().map(skrifa::GlyphName::as_str),
                Some(lucide_name),
                "{icon:?}"
            );
        }
    }

    #[test]
    fn the_list_of_icons_is_complete() {
        assert_eq!(Icon::ALL.len(), LUCIDE_NAMES.len());
        for (icon, _) in LUCIDE_NAMES {
            assert!(Icon::ALL.contains(&icon), "{icon:?}");
        }
    }
}
