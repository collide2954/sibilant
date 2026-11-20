use iced::{Element, Never, Theme};
use sibilant_ui::shell;
use sibilant_ui::theme::Palette;

#[derive(Debug, Default)]
pub(crate) struct App {
    palette: Palette,
}

impl App {
    pub(crate) fn view(&self) -> Element<'_, Never> {
        shell::view(self.palette)
    }

    pub(crate) fn theme(&self) -> Theme {
        Theme::from(self.palette)
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use sibilant_ui::theme::Palette;

    use crate::app::App;

    #[test]
    fn the_app_uses_the_graphite_theme() {
        let theme = App::default().theme();
        assert_eq!(
            theme.palette(),
            iced::Theme::from(Palette::GRAPHITE).palette()
        );
    }

    #[test]
    fn the_app_shows_the_zones_of_the_main_window() {
        let app = App::default();
        let mut ui = simulator(app.view());
        for zone in ["Transport", "Browser", "Tracks", "Inspector", "Mixer"] {
            assert!(ui.find(zone).is_ok(), "{zone}");
        }
    }
}
