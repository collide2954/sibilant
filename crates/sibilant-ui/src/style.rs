use iced::widget::container;

use crate::theme::Color;

pub(crate) fn filled(color: Color) -> container::Style {
    container::Style {
        background: Some(iced::Color::from(color).into()),
        ..container::Style::default()
    }
}
