use iced::Font;
use iced::font::Weight;

pub const TEXT: Font = Font::with_name("Inter");
pub const TEXT_MEDIUM: Font = Font {
    weight: Weight::Medium,
    ..TEXT
};
pub const TEXT_SEMIBOLD: Font = Font {
    weight: Weight::Semibold,
    ..TEXT
};
pub const NUMBERS: Font = Font::with_name("JetBrains Mono");

pub const FILES: [&[u8]; 4] = [
    include_bytes!("../fonts/Inter-Regular.ttf"),
    include_bytes!("../fonts/Inter-Medium.ttf"),
    include_bytes!("../fonts/Inter-SemiBold.ttf"),
    include_bytes!("../fonts/JetBrainsMono-Regular.ttf"),
];

#[cfg(test)]
mod tests {
    use iced::Font;
    use iced::font::{Family, Weight};

    use crate::fonts;

    fn faces() -> Vec<(String, u16)> {
        let mut database = fontdb::Database::new();
        for file in fonts::FILES {
            database.load_font_data(file.to_vec());
        }
        database
            .faces()
            .map(|face| (face.families[0].0.clone(), face.weight.0))
            .collect()
    }

    fn has_face(font: Font, weight: u16) -> bool {
        let Family::Name(family) = font.family else {
            return false;
        };
        faces().contains(&(family.to_owned(), weight))
    }

    #[test]
    fn the_text_typeface_is_inter_in_three_weights() {
        assert_eq!(fonts::TEXT.family, Family::Name("Inter"));
        assert!(has_face(fonts::TEXT, 400));
        assert!(has_face(fonts::TEXT_MEDIUM, 500));
        assert!(has_face(fonts::TEXT_SEMIBOLD, 600));
        assert_eq!(fonts::TEXT_MEDIUM.weight, Weight::Medium);
        assert_eq!(fonts::TEXT_SEMIBOLD.weight, Weight::Semibold);
    }

    #[test]
    fn the_number_typeface_is_jetbrains_mono() {
        assert_eq!(fonts::NUMBERS.family, Family::Name("JetBrains Mono"));
        assert!(has_face(fonts::NUMBERS, 400));
    }
}
