use iced::widget::{button, container};

use crate::theme::{Color, Palette};

pub(crate) fn filled(color: Color) -> container::Style {
    container::Style {
        background: Some(iced::Color::from(color).into()),
        ..container::Style::default()
    }
}

pub(crate) fn quiet(palette: Palette, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            Some(iced::Color::from(palette.raised).into())
        }
        button::Status::Active | button::Status::Disabled => None,
    };
    button::Style {
        background,
        text_color: palette.text_muted.into(),
        ..button::Style::default()
    }
}
