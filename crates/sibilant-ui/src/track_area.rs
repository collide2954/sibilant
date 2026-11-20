use iced::widget::{button, column, container, row, scrollable, space, text};
use iced::{Border, Element, Length};
use sibilant_core::project::{Clip, Project, Track};

use crate::command::Command;
use crate::style::{filled, quiet};
use crate::theme::{Color, Palette};
use crate::timeline::{RULER_HEIGHT, View, lane, ruler};

const ROW_HEIGHT: f32 = 64.0;
const HEADER_WIDTH: f32 = 200.0;
const SLOT_WIDTH: f32 = 80.0;
const COLOR_STRIP_WIDTH: f32 = 4.0;
const DIVIDER: f32 = 1.0;
const PADDING: f32 = 8.0;
const TEXT_SIZE: f32 = 13.0;
const SMALL_TEXT_SIZE: f32 = 11.0;
const TOGGLE_SIZE: f32 = 20.0;
const TOGGLE_SPACING: f32 = 4.0;
const RADIUS: f32 = 4.0;

pub fn view<'a>(
    project: &'a Project,
    timeline: View<'a>,
    palette: Palette,
) -> Element<'a, Command> {
    let rows = column(
        project
            .tracks()
            .map(|track| track_row(project, track, timeline, palette)),
    )
    .spacing(DIVIDER);
    let add_track = button(text("Add track").size(TEXT_SIZE))
        .on_press(Command::AddTrack)
        .width(HEADER_WIDTH)
        .padding(PADDING)
        .style(move |_, status| quiet(palette, status));
    let content = column![rows, scene_row(project, palette), add_track].spacing(DIVIDER);
    let ruler = ruler(timeline, project.time_signature_map(), palette);
    let ruler_row = row![space().width(HEADER_WIDTH), slot_spacers(project), ruler]
        .spacing(DIVIDER)
        .height(RULER_HEIGHT);
    column![ruler_row, scrollable(content)]
        .spacing(DIVIDER)
        .into()
}

fn track_row<'a>(
    project: &'a Project,
    track: &'a Track,
    timeline: View<'a>,
    palette: Palette,
) -> Element<'a, Command> {
    let color = palette.track_color(track.color());
    let slots = project
        .scenes()
        .map(|scene| slot(track.slot(scene.id()), color, palette));
    row![
        header(track, palette),
        row(slots).spacing(DIVIDER),
        lane(timeline, project, track, palette)
    ]
    .spacing(DIVIDER)
    .height(ROW_HEIGHT)
    .into()
}

fn slot_spacers(project: &Project) -> Element<'_, Command> {
    row(project.scenes().map(|_| space().width(SLOT_WIDTH).into()))
        .spacing(DIVIDER)
        .into()
}

fn header(track: &Track, palette: Palette) -> Element<'_, Command> {
    let color = palette.track_color(track.color());
    let mix = track.mix();
    let toggles = row![
        toggle(
            "M",
            mix.mute,
            palette.accent,
            Command::ToggleMute(track.id()),
            palette
        ),
        toggle(
            "S",
            mix.solo,
            palette.solo,
            Command::ToggleSolo(track.id()),
            palette
        ),
    ]
    .spacing(TOGGLE_SPACING);
    let name = text(track.name()).size(TEXT_SIZE).color(palette.text);
    let body = row![name, space::horizontal(), toggles].padding(PADDING);
    let strip = container(space())
        .width(COLOR_STRIP_WIDTH)
        .height(Length::Fill)
        .style(move |_| filled(color));
    container(row![strip, body])
        .width(HEADER_WIDTH)
        .height(Length::Fill)
        .style(move |_| filled(palette.raised))
        .into()
}

fn toggle(
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
        .size(SMALL_TEXT_SIZE)
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

fn slot(clip: Option<&Clip>, color: Color, palette: Palette) -> Element<'_, Command> {
    let (background, content): (Color, Element<'_, Command>) = match clip {
        Some(clip) => (
            color,
            text(clip.name())
                .size(SMALL_TEXT_SIZE)
                .color(palette.on_track_color)
                .into(),
        ),
        None => (palette.surface, space().into()),
    };
    container(content)
        .width(SLOT_WIDTH)
        .height(Length::Fill)
        .padding(PADDING)
        .style(move |_| filled(background))
        .into()
}

fn scene_row(project: &Project, palette: Palette) -> Element<'_, Command> {
    let names = project.scenes().map(|scene| {
        container(
            text(scene.name())
                .size(SMALL_TEXT_SIZE)
                .color(palette.text_muted),
        )
        .width(SLOT_WIDTH)
        .padding(PADDING)
        .into()
    });
    row![space().width(HEADER_WIDTH), row(names).spacing(DIVIDER)]
        .spacing(DIVIDER)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::keyboard::{self, Modifiers};
    use iced::mouse::{self, ScrollDelta};
    use iced::{Event, Point};
    use iced_test::simulator;
    use sibilant_core::project::{AudioSource, Clip, Ids, Project, Region};
    use sibilant_core::time::Duration;

    use crate::command::Command;
    use crate::theme::Palette;
    use crate::timeline::{RULER_HEIGHT, View};
    use crate::track_area;

    fn new_project() -> Project {
        Project::new(&mut Ids::default())
    }

    #[test]
    fn command_and_the_wheel_over_a_lane_zoom_around_the_mouse() {
        let project = new_project();
        let mut ui = simulator(track_area::view(
            &project,
            View::default(),
            Palette::GRAPHITE,
        ));
        ui.point_at(Point::new(700.0, 60.0));
        ui.simulate([
            Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::COMMAND)),
            Event::Mouse(mouse::Event::WheelScrolled {
                delta: ScrollDelta::Lines { x: 0.0, y: 1.0 },
            }),
        ]);
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            [Command::ZoomTimeline {
                steps: 1.0,
                anchor: 175.0
            }]
        );
    }

    #[test]
    fn the_tracks_start_below_the_ruler() {
        let project = new_project();
        let mut ui = simulator(track_area::view(
            &project,
            View::default(),
            Palette::GRAPHITE,
        ));
        assert!(ui.find("Audio 1").unwrap().bounds().y >= RULER_HEIGHT);
    }

    #[test]
    fn shows_a_row_for_each_track_in_order() {
        let project = new_project();
        let mut ui = simulator(track_area::view(
            &project,
            View::default(),
            Palette::GRAPHITE,
        ));
        let first = ui.find("Audio 1").unwrap().bounds();
        let second = ui.find("Audio 2").unwrap().bounds();
        assert!(first.y < second.y);
    }

    #[test]
    fn shows_the_scenes_below_the_tracks() {
        let project = new_project();
        let mut ui = simulator(track_area::view(
            &project,
            View::default(),
            Palette::GRAPHITE,
        ));
        let track = ui.find("Audio 2").unwrap().bounds();
        let scene = ui.find("Scene 1").unwrap().bounds();
        assert!(track.y < scene.y);
    }

    #[test]
    fn the_add_track_button_sends_its_command() {
        let project = new_project();
        let mut ui = simulator(track_area::view(
            &project,
            View::default(),
            Palette::GRAPHITE,
        ));
        ui.click("Add track").unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, [Command::AddTrack]);
    }

    #[test]
    fn the_mute_and_solo_buttons_send_their_commands_for_their_track() {
        let project = new_project();
        let first = project.tracks().next().unwrap().id();
        let mut ui = simulator(track_area::view(
            &project,
            View::default(),
            Palette::GRAPHITE,
        ));
        ui.click("M").unwrap();
        ui.click("S").unwrap();
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            [Command::ToggleMute(first), Command::ToggleSolo(first)]
        );
    }

    #[test]
    fn a_full_slot_shows_the_name_of_its_clip() {
        let mut ids = Ids::default();
        let project = Project::new(&mut ids);
        let (source, length) = (ids.source(), Duration::from_seconds(1));
        let track = project.tracks().next().unwrap().id();
        let scene = project.scenes().next().unwrap().id();
        let region = Region {
            start: Duration::ZERO,
            length,
        };
        let project = project
            .add_source(AudioSource::new(source, "kick.wav", length))
            .and_then(|project| {
                project.put_clip_in_slot(track, scene, Clip::new("Kick", source, region))
            })
            .unwrap();
        let mut ui = simulator(track_area::view(
            &project,
            View::default(),
            Palette::GRAPHITE,
        ));
        assert!(ui.find("Kick").is_ok());
    }
}
