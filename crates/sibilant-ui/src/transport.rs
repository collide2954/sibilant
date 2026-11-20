use iced::widget::{button, row, text};
use iced::{Alignment, Border, Element, Length};
use sibilant_core::time::{
    Clock, Musical, Position, SUPERCLOCKS_PER_SECOND, TICKS_PER_QUARTER_NOTE, Tempo,
    TimeSignatureMap,
};

use crate::command::Command;
use crate::fonts;
use crate::icons::{self, Icon};
use crate::theme::Palette;

const SIXTEENTH: i64 = TICKS_PER_QUARTER_NOTE / 4;
const SUPERCLOCKS_PER_MILLISECOND: i64 = SUPERCLOCKS_PER_SECOND / 1_000;
const BUTTON_SIZE: f32 = 32.0;
const ICON_SIZE: f32 = 16.0;
const NUMBER_SIZE: f32 = 15.0;
const SPACING: f32 = 12.0;
const RADIUS: f32 = 4.0;

#[derive(Debug, Clone, Copy)]
pub struct Display<'a> {
    pub playing: bool,
    pub metronome: bool,
    pub position: Position<Musical>,
    pub clock: Position<Clock>,
    pub tempo: Tempo,
    pub time_signatures: &'a TimeSignatureMap,
}

pub fn view(display: Display<'_>, palette: Palette) -> Element<'_, Command> {
    let play_icon = if display.playing {
        Icon::Stop
    } else {
        Icon::Play
    };
    let play = icon_button(play_icon, display.playing, Command::PlayOrStop, palette);
    let metronome = icon_button(
        Icon::Metronome,
        display.metronome,
        Command::ToggleMetronome,
        palette,
    );
    let number = |content: String, color| {
        text(content)
            .font(fonts::NUMBERS)
            .size(NUMBER_SIZE)
            .color(color)
    };
    let position = bars_beats_sixteenths(display.time_signatures, display.position);
    row![
        play,
        metronome,
        number(position, palette.text),
        number(clock_time(display.clock), palette.text_muted),
        number(format!("{} BPM", display.tempo.bpm()), palette.text_muted),
    ]
    .spacing(SPACING)
    .align_y(Alignment::Center)
    .into()
}

fn icon_button(
    icon: Icon,
    active: bool,
    command: Command,
    palette: Palette,
) -> Element<'static, Command> {
    let (background, foreground) = if active {
        (palette.accent, palette.on_accent)
    } else {
        (palette.surface, palette.text)
    };
    let glyph = text(icon.codepoint().to_string())
        .font(icons::FONT)
        .size(ICON_SIZE)
        .width(Length::Fill)
        .height(Length::Fill)
        .center();
    button(glyph)
        .on_press(command)
        .width(BUTTON_SIZE)
        .height(BUTTON_SIZE)
        .padding(0)
        .style(move |_, _| button::Style {
            background: Some(iced::Color::from(background).into()),
            text_color: foreground.into(),
            border: Border::default().rounded(RADIUS),
            ..button::Style::default()
        })
        .into()
}

pub fn bars_beats_sixteenths(
    time_signatures: &TimeSignatureMap,
    position: Position<Musical>,
) -> String {
    let bar_beat = time_signatures.to_bar_beat(position);
    let sixteenth = bar_beat.offset.ticks() / SIXTEENTH + 1;
    format!("{}.{}.{sixteenth}", bar_beat.bar, bar_beat.beat)
}

pub fn clock_time(clock: Position<Clock>) -> String {
    let milliseconds = clock.superclocks().max(0) / SUPERCLOCKS_PER_MILLISECOND;
    let (minutes, rest) = (milliseconds / 60_000, milliseconds % 60_000);
    format!("{minutes}:{:02}.{:03}", rest / 1_000, rest % 1_000)
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use sibilant_core::time::{
        Clock, Duration, Musical, Position, SUPERCLOCKS_PER_SECOND, TICKS_PER_QUARTER_NOTE, Tempo,
        TimeSignature, TimeSignatureMap,
    };

    use crate::command::Command;
    use crate::icons::Icon;
    use crate::theme::Palette;
    use crate::transport::{self, Display, bars_beats_sixteenths, clock_time};

    const SIXTEENTH: i64 = TICKS_PER_QUARTER_NOTE / 4;

    fn at(ticks: i64) -> Position<Musical> {
        Position::from_ticks(ticks)
    }

    #[test]
    fn the_project_start_reads_1_1_1() {
        let map = TimeSignatureMap::default();
        assert_eq!(bars_beats_sixteenths(&map, Position::START), "1.1.1");
    }

    #[test]
    fn the_position_counts_bars_beats_and_sixteenths_from_1() {
        let map = TimeSignatureMap::default();
        let quarter_note = TICKS_PER_QUARTER_NOTE;
        assert_eq!(bars_beats_sixteenths(&map, at(quarter_note)), "1.2.1");
        assert_eq!(
            bars_beats_sixteenths(&map, at(quarter_note + SIXTEENTH)),
            "1.2.2"
        );
        assert_eq!(bars_beats_sixteenths(&map, at(4 * quarter_note)), "2.1.1");
    }

    #[test]
    fn an_eighth_note_beat_has_two_sixteenths() {
        let map = TimeSignatureMap::new(TimeSignature::new(6, 8).unwrap());
        assert_eq!(bars_beats_sixteenths(&map, at(SIXTEENTH)), "1.1.2");
        assert_eq!(bars_beats_sixteenths(&map, at(2 * SIXTEENTH)), "1.2.1");
    }

    fn milliseconds(milliseconds: i64) -> Position<Clock> {
        Position::START + Duration::from_superclocks(milliseconds * SUPERCLOCKS_PER_SECOND / 1_000)
    }

    #[test]
    fn the_clock_time_shows_minutes_seconds_and_milliseconds() {
        assert_eq!(clock_time(Position::START), "0:00.000");
        assert_eq!(clock_time(milliseconds(61_500)), "1:01.500");
        assert_eq!(clock_time(milliseconds(600_007)), "10:00.007");
    }

    fn display(map: &TimeSignatureMap, playing: bool) -> Display<'_> {
        Display {
            playing,
            metronome: false,
            position: Position::START,
            clock: Position::START,
            tempo: Tempo::default(),
            time_signatures: map,
        }
    }

    fn glyph(icon: Icon) -> String {
        icon.codepoint().to_string()
    }

    #[test]
    fn the_transport_shows_the_position_the_time_and_the_tempo() {
        let map = TimeSignatureMap::default();
        let mut ui = simulator(transport::view(display(&map, false), Palette::GRAPHITE));
        for label in ["1.1.1", "0:00.000", "120 BPM"] {
            assert!(ui.find(label).is_ok(), "{label}");
        }
    }

    #[test]
    fn the_play_button_plays_and_stops() {
        let map = TimeSignatureMap::default();
        let mut ui = simulator(transport::view(display(&map, false), Palette::GRAPHITE));
        ui.click(glyph(Icon::Play).as_str()).unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, [Command::PlayOrStop]);
    }

    #[test]
    fn the_play_button_shows_stop_during_playback() {
        let map = TimeSignatureMap::default();
        let mut ui = simulator(transport::view(display(&map, true), Palette::GRAPHITE));
        assert!(ui.find(glyph(Icon::Stop).as_str()).is_ok());
    }

    #[test]
    fn the_metronome_button_switches_the_metronome() {
        let map = TimeSignatureMap::default();
        let mut ui = simulator(transport::view(display(&map, false), Palette::GRAPHITE));
        ui.click(glyph(Icon::Metronome).as_str()).unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, [Command::ToggleMetronome]);
    }

    #[test]
    fn the_clock_time_drops_parts_of_a_millisecond() {
        let almost = milliseconds(1) - Duration::from_superclocks(1);
        assert_eq!(clock_time(almost), "0:00.000");
    }
}
