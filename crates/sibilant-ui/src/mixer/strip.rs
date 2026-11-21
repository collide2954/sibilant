use iced::widget::{column, container, row, scrollable, space, text};
use iced::{Alignment, Element, Length};
use sibilant_core::project::{Project, Track};

use super::{Channel, Meter, Meters, fader, gain_text, meter, pan, pan_text};
use crate::command::Command;
use crate::style::{filled, toggle};
use crate::theme::Palette;

const STRIP_WIDTH: f32 = 72.0;
const PADDING: f32 = 6.0;
const SPACING: f32 = 4.0;
const DIVIDER: f32 = 1.0;
const NAME_SIZE: f32 = 11.0;
const VALUE_SIZE: f32 = 10.0;
const PAN_HEIGHT: f32 = 14.0;
const METER_WIDTH: f32 = 6.0;
const COLOR_HEIGHT: f32 = 3.0;

pub fn view<'a>(
    project: &'a Project,
    meters: &'a Meters,
    palette: Palette,
) -> Element<'a, Command> {
    let strips = project.tracks().enumerate().map(|(index, track)| {
        let level = meters.tracks.get(index).copied().unwrap_or_default();
        track_strip(track, level, palette)
    });
    let strips = scrollable(row(strips).spacing(DIVIDER))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default(),
        ))
        .width(Length::Fill);
    row![strips, master_strip(project, meters.master, palette)]
        .spacing(DIVIDER)
        .height(Length::Fill)
        .into()
}

fn track_strip(track: &Track, level: Meter, palette: Palette) -> Element<'_, Command> {
    let (id, mix) = (track.id(), track.mix());
    let color = palette.track_color(track.color());
    let band = container(space())
        .width(Length::Fill)
        .height(COLOR_HEIGHT)
        .style(move |_| filled(color));
    let name = text(track.name()).size(NAME_SIZE).color(palette.text);
    let pan_control = container(pan(id, mix.pan.position(), palette)).height(PAN_HEIGHT);
    let pan_value = text(pan_text(mix.pan.position()))
        .size(VALUE_SIZE)
        .color(palette.text_muted);
    let toggles = row![
        toggle(
            "M",
            mix.mute,
            palette.accent,
            Command::ToggleMute(id),
            palette
        ),
        toggle(
            "S",
            mix.solo,
            palette.solo,
            Command::ToggleSolo(id),
            palette
        ),
    ]
    .spacing(SPACING);
    let body = column![
        band,
        name,
        pan_control,
        pan_value,
        level_row(Channel::Track(id), mix.gain.db(), level, palette),
        gain_value(mix.gain.db(), palette),
        toggles,
    ]
    .spacing(SPACING)
    .align_x(Alignment::Center);
    strip(body, palette)
}

fn master_strip(project: &Project, level: Meter, palette: Palette) -> Element<'_, Command> {
    let db = project.master_gain().db();
    let body = column![
        text("Master").size(NAME_SIZE).color(palette.text),
        level_row(Channel::Master, db, level, palette),
        gain_value(db, palette),
    ]
    .spacing(SPACING)
    .align_x(Alignment::Center);
    strip(body, palette)
}

fn level_row(
    channel: Channel,
    db: f64,
    level: Meter,
    palette: Palette,
) -> Element<'static, Command> {
    let meter = container(meter(level.level, level.clipped, palette)).width(METER_WIDTH);
    row![fader(channel, db, palette), meter]
        .spacing(SPACING)
        .height(Length::Fill)
        .into()
}

fn gain_value(db: f64, palette: Palette) -> Element<'static, Command> {
    text(gain_text(db))
        .size(VALUE_SIZE)
        .color(palette.text_muted)
        .into()
}

fn strip<'a>(body: impl Into<Element<'a, Command>>, palette: Palette) -> Element<'a, Command> {
    container(body)
        .width(STRIP_WIDTH)
        .height(Length::Fill)
        .padding(PADDING)
        .style(move |_| filled(palette.raised))
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Size;
    use iced_test::simulator;
    use sibilant_core::mix::{Gain, Mix, Pan};
    use sibilant_core::project::{Ids, Project, Track};

    use crate::command::Command;
    use crate::mixer::{self, Meter, Meters};
    use crate::pixels::Pixels;
    use crate::theme::Palette;

    fn project() -> Project {
        Project::new(&mut Ids::default())
    }

    fn mixer_of(project: &Project) -> iced::Element<'_, Command> {
        static METERS: std::sync::LazyLock<Meters> = std::sync::LazyLock::new(Meters::default);
        mixer::view(project, &METERS, Palette::GRAPHITE)
    }

    #[test]
    fn the_mixer_has_a_strip_for_each_track_in_order_and_the_master_at_the_end() {
        let project = project();
        let mut ui = simulator(mixer_of(&project));
        let mut x = |label: &str| ui.find(label).unwrap().bounds().x;
        let (first, second, master) = (x("Audio 1"), x("Audio 2"), x("Master"));
        assert!(first < second && second < master);
    }

    #[test]
    fn a_strip_shows_the_gain_and_the_pan_of_its_track() {
        let project = project();
        let track = project.tracks().next().map(Track::id).unwrap();
        let mix = Mix {
            gain: Gain::from_db(-6.0).unwrap(),
            pan: Pan::new(-0.5).unwrap(),
            ..Mix::default()
        };
        let project = project.set_mix(track, mix).unwrap();
        let mut ui = simulator(mixer_of(&project));
        assert!(ui.find("-6.0 dB").is_ok());
        assert!(ui.find("L50").is_ok());
    }

    #[test]
    fn the_master_strip_shows_the_master_gain() {
        let project = project().with_master_gain(Gain::from_db(2.5).unwrap());
        let mut ui = simulator(mixer_of(&project));
        assert!(ui.find("+2.5 dB").is_ok());
    }

    fn shows_a_level_in_column(pixels: &Pixels, x: usize) -> bool {
        (0..240).any(|y| pixels.color_at(x, y) == Palette::GRAPHITE.accent)
    }

    #[test]
    fn each_strip_shows_the_meter_of_its_channel() {
        let project = project();
        let full = Meter {
            level: 1.0,
            clipped: false,
        };
        let meters = Meters {
            tracks: vec![full, Meter::default()],
            master: full,
        };
        let size = Size::new(400.0, 240.0);
        let pixels = Pixels::render(mixer::view(&project, &meters, Palette::GRAPHITE), size);
        let quiet = Pixels::render(mixer_of(&project), size);
        assert!(shows_a_level_in_column(&pixels, 63));
        assert!(!shows_a_level_in_column(&pixels, 63 + 73));
        assert!(shows_a_level_in_column(&pixels, 400 - 72 + 63));
        assert!(!shows_a_level_in_column(&quiet, 63));
        assert!(!shows_a_level_in_column(&quiet, 400 - 72 + 63));
    }

    #[test]
    fn the_mute_and_solo_buttons_of_a_strip_send_their_commands() {
        let project = project();
        let track = project.tracks().next().map(Track::id).unwrap();
        let mut ui = simulator(mixer_of(&project));
        ui.click("M").unwrap();
        ui.click("S").unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            [Command::ToggleMute(track), Command::ToggleSolo(track)]
        );
    }
}
