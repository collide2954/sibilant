use std::time::Duration;

use iced::widget::{Row, button, center, column, container, opaque, row, space, stack, text};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreChoice {
    Restore,
    DontRestore,
}

pub fn unsaved_changes<'a>(name: &str, palette: Palette) -> Element<'a, Choice> {
    let save = choice_button("Save", Choice::Save, primary(palette));
    let dont_save = choice_button("Don't Save", Choice::DontSave, quiet(palette));
    let cancel = choice_button("Cancel", Choice::Cancel, quiet(palette));
    let buttons = if cfg!(target_os = "windows") {
        row![space::horizontal(), save, dont_save, cancel]
    } else {
        row![dont_save, space::horizontal(), cancel, save]
    };
    card(
        format!("Do you want to save the changes to “{name}”?"),
        "Your changes are lost if you do not save them.".to_owned(),
        buttons,
        palette,
    )
}

pub fn restore<'a>(name: &str, age: Duration, palette: Palette) -> Element<'a, RestoreChoice> {
    let restore = choice_button("Restore", RestoreChoice::Restore, primary(palette));
    let dont_restore = choice_button("Don't Restore", RestoreChoice::DontRestore, quiet(palette));
    let buttons = if cfg!(target_os = "windows") {
        row![space::horizontal(), restore, dont_restore]
    } else {
        row![dont_restore, space::horizontal(), restore]
    };
    card(
        format!("Restore the unsaved changes to “{name}”?"),
        format!(
            "Sibilant closed before you saved them. The newest copy is from {}.",
            ago(age)
        ),
        buttons,
        palette,
    )
}

pub fn ago(age: Duration) -> String {
    let minutes = age.as_secs() / 60;
    let (count, unit) = match minutes {
        0 => return "less than a minute ago".to_owned(),
        1..60 => (minutes, "minute"),
        60..1_440 => (minutes / 60, "hour"),
        _ => (minutes / 1_440, "day"),
    };
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {unit}{plural} ago")
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

fn card<'a, Message: 'a>(
    question: String,
    detail: String,
    buttons: Row<'a, Message>,
    palette: Palette,
) -> Element<'a, Message> {
    let question = text(question).size(QUESTION_SIZE).color(palette.text);
    let detail = text(detail).size(TEXT_SIZE).color(palette.text_muted);
    let content = column![question, detail, buttons.spacing(BUTTON_SPACING)].spacing(SPACING);
    container(content)
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

fn primary(palette: Palette) -> (Color, Color) {
    (palette.accent, palette.on_accent)
}

fn quiet(palette: Palette) -> (Color, Color) {
    (palette.surface, palette.text)
}

fn choice_button<'a, Message: Clone + 'a>(
    label: &'static str,
    choice: Message,
    (background, foreground): (Color, Color),
) -> Element<'a, Message> {
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
    use std::time::Duration;

    use iced::widget::{button, text};
    use iced_test::simulator;

    use crate::dialog::{self, Choice, RestoreChoice};
    use crate::theme::Palette;

    fn ask() -> iced::Element<'static, Choice> {
        dialog::unsaved_changes("MySong", Palette::GRAPHITE)
    }

    fn offer() -> iced::Element<'static, RestoreChoice> {
        dialog::restore("MySong", Duration::from_mins(4), Palette::GRAPHITE)
    }

    #[test]
    fn the_restore_dialog_names_the_project_and_the_age_of_its_copy() {
        let mut ui = simulator(offer());
        assert!(ui.find("Restore the unsaved changes to “MySong”?").is_ok());
        let detail =
            "Sibilant closed before you saved them. The newest copy is from 4 minutes ago.";
        assert!(ui.find(detail).is_ok());
    }

    #[test]
    fn each_restore_button_gives_its_answer() {
        let answers = [
            ("Restore", RestoreChoice::Restore),
            ("Don't Restore", RestoreChoice::DontRestore),
        ];
        for (label, choice) in answers {
            let mut ui = simulator(offer());
            ui.click(label).unwrap();
            let messages: Vec<_> = ui.into_messages().collect();
            assert_eq!(messages, [choice], "{label}");
        }
    }

    fn restore_left_edges() -> [f32; 2] {
        let mut ui = simulator(offer());
        ["Restore", "Don't Restore"].map(|label| ui.find(label).unwrap().bounds().x)
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn on_windows_restore_comes_before_dont_restore() {
        let [restore, dont_restore] = restore_left_edges();
        assert!(restore < dont_restore);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn on_macos_and_linux_dont_restore_is_at_the_left_and_restore_at_the_right() {
        let [restore, dont_restore] = restore_left_edges();
        assert!(dont_restore < restore);
    }

    #[test]
    fn the_age_reads_in_minutes_hours_and_days() {
        let ages = [
            (0, "less than a minute ago"),
            (59, "less than a minute ago"),
            (60, "1 minute ago"),
            (4 * 60 + 30, "4 minutes ago"),
            (60 * 60, "1 hour ago"),
            (5 * 60 * 60, "5 hours ago"),
            (24 * 60 * 60, "1 day ago"),
            (3 * 24 * 60 * 60, "3 days ago"),
        ];
        for (seconds, age) in ages {
            assert_eq!(dialog::ago(Duration::from_secs(seconds)), age, "{seconds}");
        }
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
