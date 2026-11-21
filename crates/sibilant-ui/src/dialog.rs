use iced::widget::{button, center, column, container, opaque, row, space, stack, text};
use iced::{Border, Element};

use crate::theme::{Color, Palette};

const WIDTH: f32 = 420.0;
const PADDING: f32 = 20.0;
const SPACING: f32 = 12.0;
const BUTTON_SPACING: f32 = 8.0;
const BUTTON_PADDING: [f32; 2] = [6.0, 14.0];
const QUESTION_SIZE: f32 = 15.0;
const TEXT_SIZE: f32 = 13.0;
const RADIUS: f32 = 8.0;
const BUTTON_RADIUS: f32 = 4.0;
const SCRIM_ALPHA: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Save,
    DontSave,
    Cancel,
}

pub fn unsaved_changes<'a>(name: &str, palette: Palette) -> Element<'a, Choice> {
    let question = text(format!("Do you want to save the changes to “{name}”?"))
        .size(QUESTION_SIZE)
        .color(palette.text);
    let detail = text("Your changes are lost if you do not save them.")
        .size(TEXT_SIZE)
        .color(palette.text_muted);
    container(column![question, detail, buttons(palette)].spacing(SPACING))
        .width(WIDTH)
        .padding(PADDING)
        .style(move |_| container::Style {
            background: Some(iced::Color::from(palette.raised).into()),
            border: Border {
                color: palette.border.into(),
                width: 1.0,
                radius: RADIUS.into(),
            },
            ..container::Style::default()
        })
        .into()
}

pub fn modal<'a, Message: 'a>(
    base: impl Into<Element<'a, Message>>,
    dialog: impl Into<Element<'a, Message>>,
    palette: Palette,
) -> Element<'a, Message> {
    let scrim = iced::Color {
        a: SCRIM_ALPHA,
        ..palette.background.into()
    };
    let layer = center(dialog).style(move |_| container::Style {
        background: Some(scrim.into()),
        ..container::Style::default()
    });
    stack![base.into(), opaque(layer)].into()
}

fn buttons<'a>(palette: Palette) -> Element<'a, Choice> {
    let save = choice_button("Save", Choice::Save, (palette.accent, palette.on_accent));
    let quiet = (palette.surface, palette.text);
    let dont_save = choice_button("Don't Save", Choice::DontSave, quiet);
    let cancel = choice_button("Cancel", Choice::Cancel, quiet);
    let buttons = if cfg!(target_os = "windows") {
        row![space::horizontal(), save, dont_save, cancel]
    } else {
        row![dont_save, space::horizontal(), cancel, save]
    };
    buttons.spacing(BUTTON_SPACING).into()
}

fn choice_button<'a>(
    label: &'static str,
    choice: Choice,
    (background, foreground): (Color, Color),
) -> Element<'a, Choice> {
    button(text(label).size(TEXT_SIZE))
        .on_press(choice)
        .padding(BUTTON_PADDING)
        .style(move |_, _| button::Style {
            background: Some(iced::Color::from(background).into()),
            text_color: foreground.into(),
            border: Border::default().rounded(BUTTON_RADIUS),
            ..button::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::{button, text};
    use iced_test::simulator;

    use crate::dialog::{self, Choice};
    use crate::theme::Palette;

    fn ask() -> iced::Element<'static, Choice> {
        dialog::unsaved_changes("MySong", Palette::GRAPHITE)
    }

    #[test]
    fn the_dialog_asks_to_save_the_changes_to_the_project() {
        let mut ui = simulator(ask());
        assert!(
            ui.find("Do you want to save the changes to “MySong”?")
                .is_ok()
        );
        assert!(
            ui.find("Your changes are lost if you do not save them.")
                .is_ok()
        );
    }

    #[test]
    fn each_button_gives_its_answer() {
        let answers = [
            ("Save", Choice::Save),
            ("Don't Save", Choice::DontSave),
            ("Cancel", Choice::Cancel),
        ];
        for (label, choice) in answers {
            let mut ui = simulator(ask());
            ui.click(label).unwrap();
            let messages: Vec<_> = ui.into_messages().collect();
            assert_eq!(messages, [choice], "{label}");
        }
    }

    fn left_edges() -> [f32; 3] {
        let mut ui = simulator(ask());
        ["Save", "Don't Save", "Cancel"].map(|label| ui.find(label).unwrap().bounds().x)
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn on_windows_the_buttons_read_save_then_dont_save_then_cancel() {
        let [save, dont_save, cancel] = left_edges();
        assert!(save < dont_save && dont_save < cancel);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn on_macos_and_linux_dont_save_is_at_the_left_and_save_at_the_right() {
        let [save, dont_save, cancel] = left_edges();
        assert!(dont_save < cancel && cancel < save);
    }

    #[test]
    fn the_window_below_a_dialog_takes_no_clicks() {
        let base = button(text("Add")).on_press("add");
        let mut ui = simulator(dialog::modal(base, text("Question"), Palette::GRAPHITE));
        assert!(ui.find("Question").is_ok());
        ui.click("Add").unwrap();
        assert_eq!(ui.into_messages().count(), 0);
    }
}
