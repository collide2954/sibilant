use iced::widget::{button, container, text};
use iced::{Border, Element, Length};

use crate::command::Command;
use crate::theme::{Color, Palette};

const TOGGLE_SIZE: f32 = 20.0;
const TOGGLE_TEXT_SIZE: f32 = 11.0;
const RADIUS: f32 = 4.0;

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

pub(crate) fn toggle(
    label: &'static str,
    on: bool,
    on_color: Color,
    command: Command,
    palette: Palette,
) -> Element<'static, Command> {
    let (background, foreground) = if on {
        (on_color, palette.on_accent)
    } else {
        (palette.surface, palette.text_muted)
    };
    let label = text(label)
        .size(TOGGLE_TEXT_SIZE)
        .width(Length::Fill)
        .height(Length::Fill)
        .center();
    button(label)
        .on_press(command)
        .width(TOGGLE_SIZE)
        .height(TOGGLE_SIZE)
        .padding(0)
        .style(move |_, _| button::Style {
            background: Some(iced::Color::from(background).into()),
            text_color: foreground.into(),
            border: Border::default().rounded(RADIUS),
            ..button::Style::default()
        })
        .into()
}
