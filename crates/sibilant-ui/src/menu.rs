use iced::widget::{button, column, container, row, space, text};
use iced::{Border, Element, Length};

use crate::command::Command;
use crate::style::filled;
use crate::theme::{Color, Palette};

const WIDTH: f32 = 220.0;
const PADDING: f32 = 4.0;
const ITEM_PADDING: [f32; 2] = [6.0, 10.0];
const DIVIDER_PADDING: [f32; 2] = [4.0, 6.0];
const TEXT_SIZE: f32 = 13.0;
const RADIUS: f32 = 6.0;
const ITEM_RADIUS: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Toggle,
    Close,
    Choose(Command),
}

pub fn file_button<'a>(open: bool, palette: Palette) -> Element<'a, Event> {
    button(text("File").size(TEXT_SIZE))
        .on_press(Event::Toggle)
        .padding(ITEM_PADDING)
        .style(move |_, status| {
            let lit = open || matches!(status, button::Status::Hovered | button::Status::Pressed);
            highlight(lit.then_some(palette.raised), palette.text)
        })
        .into()
}

pub fn file_menu<'a>(palette: Palette) -> Element<'a, Event> {
    let items = column![
        item("New", shortcut("N", false), Command::New, palette),
        item("Open…", shortcut("O", false), Command::Open, palette),
        divider(palette),
        item("Save", shortcut("S", false), Command::Save, palette),
        item("Save As…", shortcut("S", true), Command::SaveAs, palette),
        divider(palette),
        item(
            "Export Audio…",
            shortcut("E", true),
            Command::Export,
            palette
        ),
    ];
    container(items)
        .width(WIDTH)
        .padding(PADDING)
        .style(move |_| container::Style {
            border: Border {
                color: palette.border.into(),
                width: 1.0,
                radius: RADIUS.into(),
            },
            ..filled(palette.raised)
        })
        .into()
}

fn item<'a>(
    label: &'static str,
    keys: String,
    command: Command,
    palette: Palette,
) -> Element<'a, Event> {
    let content = row![
        text(label).size(TEXT_SIZE).color(palette.text),
        space::horizontal(),
        text(keys).size(TEXT_SIZE).color(palette.text_muted),
    ];
    button(content)
        .on_press(Event::Choose(command))
        .width(Length::Fill)
        .padding(ITEM_PADDING)
        .style(move |_, status| {
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            highlight(lit.then_some(palette.border), palette.text)
        })
        .into()
}

fn divider<'a>(palette: Palette) -> Element<'a, Event> {
    let line = container(space())
        .width(Length::Fill)
        .height(1.0)
        .style(move |_| filled(palette.border));
    container(line).padding(DIVIDER_PADDING).into()
}

fn highlight(background: Option<Color>, text: Color) -> button::Style {
    button::Style {
        background: background.map(|color| iced::Color::from(color).into()),
        text_color: text.into(),
        border: Border::default().rounded(ITEM_RADIUS),
        ..button::Style::default()
    }
}

fn shortcut(key: &str, shift: bool) -> String {
    match (cfg!(target_os = "macos"), shift) {
        (true, true) => format!("⇧⌘{key}"),
        (true, false) => format!("⌘{key}"),
        (false, true) => format!("Ctrl+Shift+{key}"),
        (false, false) => format!("Ctrl+{key}"),
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use crate::command::Command;
    use crate::menu::{self, Event};
    use crate::theme::Palette;

    #[test]
    fn the_file_button_opens_and_closes_the_file_menu() {
        let mut ui = simulator(menu::file_button(false, Palette::GRAPHITE));
        ui.click("File").unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, [Event::Toggle]);
    }

    #[test]
    fn each_item_of_the_file_menu_runs_its_command() {
        let items = [
            ("New", Command::New),
            ("Open…", Command::Open),
            ("Save", Command::Save),
            ("Save As…", Command::SaveAs),
            ("Export Audio…", Command::Export),
        ];
        for (label, command) in items {
            let mut ui = simulator(menu::file_menu(Palette::GRAPHITE));
            ui.click(label).unwrap();
            let messages: Vec<_> = ui.into_messages().collect();
            assert_eq!(messages, [Event::Choose(command)], "{label}");
        }
    }

    #[test]
    fn the_items_read_new_open_save_save_as_and_export_from_the_top() {
        let mut ui = simulator(menu::file_menu(Palette::GRAPHITE));
        let tops = ["New", "Open…", "Save", "Save As…", "Export Audio…"]
            .map(|label| ui.find(label).unwrap().bounds().y);
        assert!(tops.is_sorted(), "{tops:?}");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_items_show_their_ctrl_shortcuts() {
        let mut ui = simulator(menu::file_menu(Palette::GRAPHITE));
        for keys in ["Ctrl+N", "Ctrl+O", "Ctrl+S", "Ctrl+Shift+S", "Ctrl+Shift+E"] {
            assert!(ui.find(keys).is_ok(), "{keys}");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_items_show_their_cmd_shortcuts() {
        let mut ui = simulator(menu::file_menu(Palette::GRAPHITE));
        for keys in ["⌘N", "⌘O", "⌘S", "⇧⌘S", "⇧⌘E"] {
            assert!(ui.find(keys).is_ok(), "{keys}");
        }
    }
}
