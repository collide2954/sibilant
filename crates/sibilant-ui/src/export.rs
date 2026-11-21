use iced::widget::{button, row, space, text};
use iced::{Alignment, Border, Element};
use sibilant_media::Depth;

use crate::dialog::{self, card, choice_button};
use crate::theme::Palette;

const SAMPLE_RATES: [(u32, &str); 3] =
    [(44_100, "44.1 kHz"), (48_000, "48 kHz"), (96_000, "96 kHz")];
const DEPTHS: [(Depth, &str); 3] = [
    (Depth::Int16, "16-bit"),
    (Depth::Int24, "24-bit"),
    (Depth::Float32, "32-bit float"),
];
const TEXT_SIZE: f32 = 13.0;
const LABEL_WIDTH: f32 = 96.0;
const OPTION_SPACING: f32 = 4.0;
const OPTION_PADDING: [f32; 2] = [4.0, 10.0];
const OPTION_RADIUS: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    pub sample_rate: u32,
    pub depth: Depth,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            depth: Depth::Int24,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    SampleRate(u32),
    Depth(Depth),
    Export,
    Cancel,
}

pub fn dialog<'a>(name: &str, format: Format, palette: Palette) -> Element<'a, Event> {
    let rates = SAMPLE_RATES.map(|(rate, label)| {
        let chosen = rate == format.sample_rate;
        option(label, Event::SampleRate(rate), chosen, palette)
    });
    let depths = DEPTHS
        .map(|(depth, label)| option(label, Event::Depth(depth), depth == format.depth, palette));
    let export = choice_button("Export", Event::Export, dialog::primary(palette));
    let cancel = choice_button("Cancel", Event::Cancel, dialog::quiet(palette));
    let buttons = if cfg!(target_os = "windows") {
        row![space::horizontal(), export, cancel]
    } else {
        row![space::horizontal(), cancel, export]
    };
    card(
        format!("Export “{name}”"),
        "Sibilant makes a stereo WAV file from bar 1 to the end of the last clip.".to_owned(),
        vec![
            setting("Sample rate", rates, palette),
            setting("Bit depth", depths, palette),
        ],
        buttons,
        palette,
    )
}

fn setting<'a>(
    label: &'static str,
    options: [Element<'a, Event>; 3],
    palette: Palette,
) -> Element<'a, Event> {
    row![
        text(label)
            .size(TEXT_SIZE)
            .color(palette.text_muted)
            .width(LABEL_WIDTH),
        row(options).spacing(OPTION_SPACING)
    ]
    .align_y(Alignment::Center)
    .into()
}

fn option<'a>(
    label: &'static str,
    event: Event,
    chosen: bool,
    palette: Palette,
) -> Element<'a, Event> {
    let (background, foreground, edge) = if chosen {
        (palette.border, palette.text, palette.accent)
    } else {
        (palette.surface, palette.text_muted, palette.surface)
    };
    button(text(label).size(TEXT_SIZE))
        .on_press(event)
        .padding(OPTION_PADDING)
        .style(move |_, _| button::Style {
            background: Some(iced::Color::from(background).into()),
            text_color: foreground.into(),
            border: Border {
                color: edge.into(),
                width: 1.0,
                radius: OPTION_RADIUS.into(),
            },
            ..button::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use iced::{Point, Settings, Size};
    use iced_test::{Simulator, simulator};
    use sibilant_media::Depth;

    use crate::export::{self, Event, Format};
    use crate::pixels::Pixels;
    use crate::theme::Palette;

    fn ask(format: Format) -> iced::Element<'static, Event> {
        export::dialog("MySong", format, Palette::GRAPHITE)
    }

    #[test]
    fn the_first_choices_are_48_khz_and_24_bit() {
        let first = Format {
            sample_rate: 48_000,
            depth: Depth::Int24,
        };
        assert_eq!(Format::default(), first);
    }

    #[test]
    fn the_dialog_names_the_project_and_tells_what_the_file_contains() {
        let mut ui = simulator(ask(Format::default()));
        assert!(ui.find("Export “MySong”").is_ok());
        let contents = "Sibilant makes a stereo WAV file from bar 1 to the end of the last clip.";
        assert!(ui.find(contents).is_ok());
        assert!(ui.find("Sample rate").is_ok());
        assert!(ui.find("Bit depth").is_ok());
    }

    #[test]
    fn each_option_and_button_gives_its_event() {
        let events = [
            ("44.1 kHz", Event::SampleRate(44_100)),
            ("48 kHz", Event::SampleRate(48_000)),
            ("96 kHz", Event::SampleRate(96_000)),
            ("16-bit", Event::Depth(Depth::Int16)),
            ("24-bit", Event::Depth(Depth::Int24)),
            ("32-bit float", Event::Depth(Depth::Float32)),
            ("Export", Event::Export),
            ("Cancel", Event::Cancel),
        ];
        for (label, event) in events {
            let mut ui = simulator(ask(Format::default()));
            ui.click(label).unwrap();
            let messages: Vec<_> = ui.into_messages().collect();
            assert_eq!(messages, [event], "{label}");
        }
    }

    #[test]
    fn the_options_read_from_low_to_high_with_the_rates_above_the_depths() {
        let mut ui = simulator(ask(Format::default()));
        let mut bounds = |label: &str| ui.find(label).unwrap().bounds();
        let rates = ["44.1 kHz", "48 kHz", "96 kHz"].map(&mut bounds);
        let depths = ["16-bit", "24-bit", "32-bit float"].map(&mut bounds);
        assert!(rates.map(|rate| rate.x).is_sorted());
        assert!(depths.map(|depth| depth.x).is_sorted());
        assert!(rates[0].y < depths[0].y);
    }

    fn button_left_edges() -> [f32; 2] {
        let mut ui = simulator(ask(Format::default()));
        ["Export", "Cancel"].map(|label| ui.find(label).unwrap().bounds().x)
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn on_windows_export_comes_before_cancel() {
        let [export, cancel] = button_left_edges();
        assert!(export < cancel);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn on_macos_and_linux_cancel_comes_before_export() {
        let [export, cancel] = button_left_edges();
        assert!(cancel < export);
    }

    #[test]
    fn the_chosen_options_stand_out() {
        let format = Format {
            sample_rate: 44_100,
            depth: Depth::Float32,
        };
        let size = Size::new(480.0, 320.0);
        let mut ui = Simulator::with_size(Settings::default(), size, ask(format));
        let mut beside = |label: &str| {
            let bounds = ui.find(label).unwrap().bounds();
            Point::new(bounds.x - 4.0, bounds.center_y())
        };
        let points = ["44.1 kHz", "48 kHz", "24-bit", "32-bit float"].map(&mut beside);
        let pixels = Pixels::render(ask(format), size);
        let (chosen, other) = (Palette::GRAPHITE.border, Palette::GRAPHITE.surface);
        assert_eq!(
            points.map(|point| pixels.color_near(point)),
            [chosen, other, other, chosen]
        );
    }
}
