use iced::widget::{Container, column, container, row, text};
use iced::{Element, Length};

use crate::theme::{Color, Palette};

const TRANSPORT_HEIGHT: f32 = 48.0;
const BROWSER_WIDTH: f32 = 240.0;
const INSPECTOR_WIDTH: f32 = 280.0;
const BOTTOM_HEIGHT: f32 = 240.0;
const DIVIDER: f32 = 1.0;
const PADDING: f32 = 12.0;
const TAB_SPACING: f32 = 16.0;
const TEXT_SIZE: f32 = 13.0;

pub fn view<'a, Message: 'a>(palette: Palette) -> Element<'a, Message> {
    let transport = zone(palette.raised, label(palette, "Transport")).height(TRANSPORT_HEIGHT);
    let browser = zone(palette.surface, label(palette, "Browser")).width(BROWSER_WIDTH);
    let tracks = zone(palette.background, label(palette, "Tracks"));
    let inspector = zone(palette.surface, label(palette, "Inspector")).width(INSPECTOR_WIDTH);
    let tabs = row![label(palette, "Mixer"), label(palette, "Clip editor")].spacing(TAB_SPACING);
    let bottom = zone(palette.surface, tabs).height(BOTTOM_HEIGHT);
    let middle = row![browser, tracks, inspector]
        .spacing(DIVIDER)
        .height(Length::Fill);
    let layout = column![transport, middle, bottom].spacing(DIVIDER);
    container(layout)
        .style(move |_| filled(palette.border))
        .into()
}

fn zone<'a, Message: 'a>(
    background: Color,
    content: impl Into<Element<'a, Message>>,
) -> Container<'a, Message> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(PADDING)
        .style(move |_| filled(background))
}

fn label<'a, Message: 'a>(palette: Palette, content: &'a str) -> Element<'a, Message> {
    text(content)
        .size(TEXT_SIZE)
        .color(palette.text_muted)
        .into()
}

fn filled(color: Color) -> container::Style {
    container::Style {
        background: Some(iced::Color::from(color).into()),
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use crate::shell;
    use crate::theme::Palette;

    #[test]
    fn the_window_has_the_five_zones_in_their_places() {
        let mut ui = simulator(shell::view::<()>(Palette::GRAPHITE));
        let mut bounds = |label: &str| ui.find(label).unwrap().bounds();
        let transport = bounds("Transport");
        let browser = bounds("Browser");
        let tracks = bounds("Tracks");
        let inspector = bounds("Inspector");
        let mixer = bounds("Mixer");
        assert!(transport.y < browser.y && transport.y < tracks.y);
        assert!(browser.x < tracks.x && tracks.x < inspector.x);
        assert!(inspector.y < mixer.y && tracks.y < mixer.y);
    }

    #[test]
    fn the_bottom_zone_has_tabs_for_the_mixer_and_the_clip_editor() {
        let mut ui = simulator(shell::view::<()>(Palette::GRAPHITE));
        let mixer = ui.find("Mixer").unwrap().bounds();
        let editor = ui.find("Clip editor").unwrap().bounds();
        assert!(mixer.x < editor.x);
    }
}
