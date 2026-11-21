use iced::widget::{Container, column, container, mouse_area, opaque, pin, row, stack, text};
use iced::{Alignment, Element, Length};

use crate::style::filled;
use crate::theme::{Color, Palette};

const TRANSPORT_HEIGHT: f32 = 48.0;
const TRANSPORT_PADDING: [f32; 2] = [8.0, 12.0];
const MENU_SPACING: f32 = 12.0;
const MENU_GAP: f32 = 4.0;
const BROWSER_WIDTH: f32 = 240.0;
const INSPECTOR_WIDTH: f32 = 280.0;
const BOTTOM_HEIGHT: f32 = 240.0;
const DIVIDER: f32 = 1.0;
const PADDING: f32 = 12.0;
const TAB_SPACING: f32 = 16.0;
const ZONE_SPACING: f32 = 8.0;
const TEXT_SIZE: f32 = 13.0;

pub fn view<'a, Message: 'a>(
    palette: Palette,
    menu: impl Into<Element<'a, Message>>,
    transport: impl Into<Element<'a, Message>>,
    browser: impl Into<Element<'a, Message>>,
    tracks: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let top = row![menu.into(), transport.into()]
        .spacing(MENU_SPACING)
        .align_y(Alignment::Center);
    let transport = zone(palette.raised, top)
        .height(TRANSPORT_HEIGHT)
        .padding(TRANSPORT_PADDING);
    let browser = column![label(palette, "Browser"), browser.into()].spacing(ZONE_SPACING);
    let browser = zone(palette.surface, browser).width(BROWSER_WIDTH);
    let tracks = container(tracks)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| filled(palette.background));
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

pub fn with_menu<'a, Message: Clone + 'a>(
    window: impl Into<Element<'a, Message>>,
    menu: impl Into<Element<'a, Message>>,
    close: Message,
) -> Element<'a, Message> {
    let menu = pin(opaque(menu))
        .x(TRANSPORT_PADDING[1])
        .y(TRANSPORT_HEIGHT - TRANSPORT_PADDING[0] + MENU_GAP);
    stack![window.into(), mouse_area(menu).on_press(close)].into()
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

#[cfg(test)]
mod tests {
    use iced::widget::text;
    use iced_test::simulator;

    use crate::shell;
    use crate::theme::Palette;

    fn shell<'a, Message: 'a>() -> iced::Element<'a, Message> {
        shell::view(
            Palette::GRAPHITE,
            text("File"),
            text("Transport"),
            text("Places"),
            text("Tracks"),
        )
    }

    #[test]
    fn the_file_menu_button_is_at_the_left_end_of_the_top_bar() {
        let mut ui = simulator(shell::<()>());
        let file = ui.find("File").unwrap().bounds();
        let transport = ui.find("Transport").unwrap().bounds();
        let browser = ui.find("Browser").unwrap().bounds();
        assert!(file.x < transport.x && file.y < browser.y);
        assert!((file.x - browser.x).abs() < 0.5);
    }

    #[test]
    fn the_menu_opens_below_the_file_menu_button() {
        let mut ui = simulator(shell::with_menu(shell(), text("New"), ()));
        let file = ui.find("File").unwrap().bounds();
        let new = ui.find("New").unwrap().bounds();
        assert!((new.x - file.x).abs() < 0.5);
        assert!(new.y >= file.y + file.height);
    }

    #[test]
    fn a_click_beside_an_open_menu_closes_it() {
        let mut ui = simulator(shell::with_menu(shell(), text("New"), "close"));
        ui.click("Tracks").unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, ["close"]);
    }

    #[test]
    fn a_click_on_the_open_menu_does_not_close_it() {
        let mut ui = simulator(shell::with_menu(shell(), text("New"), "close"));
        ui.click("New").unwrap();
        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn the_window_has_the_five_zones_in_their_places() {
        let mut ui = simulator(shell::<()>());
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
    fn the_browser_zone_shows_the_browser_below_its_title() {
        let mut ui = simulator(shell::<()>());
        let title = ui.find("Browser").unwrap().bounds();
        let places = ui.find("Places").unwrap().bounds();
        let tracks = ui.find("Tracks").unwrap().bounds();
        assert!(title.y < places.y && places.x < tracks.x);
    }

    #[test]
    fn the_bottom_zone_has_tabs_for_the_mixer_and_the_clip_editor() {
        let mut ui = simulator(shell::<()>());
        let mixer = ui.find("Mixer").unwrap().bounds();
        let editor = ui.find("Clip editor").unwrap().bounds();
        assert!(mixer.x < editor.x);
    }
}
