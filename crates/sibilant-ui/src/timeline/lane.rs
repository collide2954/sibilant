use iced::keyboard::Modifiers;
use iced::widget::canvas::{self, Action, Canvas, Event, Frame, Geometry};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};
use sibilant_core::project::{Clip, ClipKey, Project, Track};
use sibilant_core::time::{Musical, Position};

use super::drag::{drag_interaction, draw_drop_line, drop_action};
use super::{Grip, Marquee, Pointer, View, Viewport, WaveColumn, draw_playhead, wheel};
use crate::command::Command;
use crate::theme::{Color, Palette};

const CLIP_INSET: f32 = 2.0;
const CLIP_GAP: f32 = 1.0;
const NAME_INSET: f32 = 4.0;
const NAME_SIZE: f32 = 11.0;
const WAVE_MARGIN: f32 = 2.0;
const WAVE_ALPHA: f32 = 0.5;
const EDGE: f32 = 6.0;
const OUTLINE: f32 = 2.0;
const MARQUEE_ALPHA: f32 = 0.2;

struct Lane<'a> {
    view: View<'a>,
    project: &'a Project,
    track: &'a Track,
    palette: Palette,
}

impl canvas::Program<Command> for Lane<'_> {
    type State = Modifiers;

    fn update(
        &self,
        modifiers: &mut Modifiers,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Command>> {
        let time_signatures = self.project.time_signature_map();
        let track = Some(self.track.id());
        drop_action(self.view, time_signatures, track, event, bounds, cursor)
            .or_else(|| self.pointer(*modifiers, event, bounds, cursor))
            .or_else(|| wheel(modifiers, event, bounds, cursor))
    }

    fn mouse_interaction(
        &self,
        _modifiers: &Modifiers,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let on_edge = cursor.position_in(bounds).is_some_and(|position| {
            matches!(
                self.grip_at(position.x, bounds.width),
                Some((_, Grip::Start | Grip::End))
            )
        });
        if on_edge && !self.view.dragging {
            mouse::Interaction::ResizingHorizontally
        } else {
            drag_interaction(self.view, bounds, cursor)
        }
    }

    fn draw(
        &self,
        _modifiers: &Modifiers,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = self.palette;
        let height = bounds.height;
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            iced::Color::from(palette.background),
        );
        let grid = self
            .view
            .viewport
            .grid(self.project.time_signature_map(), bounds.width);
        for x in grid.beats {
            let line = Size::new(1.0, height);
            frame.fill_rectangle(Point::new(x, 0.0), line, iced::Color::from(palette.surface));
        }
        for bar in grid.bars {
            let line = Size::new(1.0, height);
            frame.fill_rectangle(
                Point::new(bar.x, 0.0),
                line,
                iced::Color::from(palette.border),
            );
        }
        let color = palette.track_color(self.track.color());
        for clip_box in self
            .view
            .viewport
            .clip_boxes(self.project, self.track, bounds.width)
        {
            let area = Rectangle::new(
                Point::new(clip_box.x, CLIP_INSET),
                Size::new(
                    (clip_box.width - CLIP_GAP).max(1.0),
                    height - 2.0 * CLIP_INSET,
                ),
            );
            frame.fill_rectangle(area.position(), area.size(), iced::Color::from(color));
            if let Some(peaks) = self.view.waveforms.get(&clip_box.clip.source()) {
                let columns =
                    self.view
                        .viewport
                        .waveform(self.project, &clip_box, peaks, bounds.width);
                draw_waveform(&mut frame, area, &columns, palette);
            }
            frame.with_clip(area, |frame| {
                frame.fill_text(canvas::Text {
                    content: clip_box.clip.name().to_owned(),
                    position: Point::new(area.x + NAME_INSET, area.y + NAME_INSET),
                    color: palette.on_track_color.into(),
                    size: NAME_SIZE.into(),
                    ..canvas::Text::default()
                });
            });
            if self.view.selection.contains(&self.key(clip_box.start)) {
                draw_outline(&mut frame, area, palette.text);
            }
        }
        if let Some(marquee) = self.view.marquee.filter(|marquee| self.is_inside(marquee)) {
            draw_marquee(&mut frame, self.view.viewport, marquee, palette);
        }
        let time_signatures = self.project.time_signature_map();
        draw_drop_line(
            &mut frame,
            self.view,
            time_signatures,
            bounds,
            cursor,
            palette,
        );
        draw_playhead(&mut frame, self.view, palette);
        vec![frame.into_geometry()]
    }
}

impl Lane<'_> {
    fn pointer(
        &self,
        modifiers: Modifiers,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Command>> {
        let position = cursor.position_in(bounds)?;
        let (track, free) = (self.track.id(), modifiers.alt());
        let at = self.view.viewport.position_at(position.x);
        let pointer = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if !self.view.dragging =>
            {
                Pointer::Pressed {
                    track,
                    at,
                    grip: self.grip_at(position.x, bounds.width),
                    toggle: modifiers.command(),
                    free,
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if self.view.pointing => {
                Pointer::Moved { track, at, free }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.view.pointing =>
            {
                Pointer::Released { track, at, free }
            }
            _ => return None,
        };
        Some(Action::publish(Command::Pointer(pointer)).and_capture())
    }

    fn grip_at(&self, x: f32, width: f32) -> Option<(ClipKey, Grip)> {
        let clip_box = self
            .view
            .viewport
            .clip_boxes(self.project, self.track, width)
            .into_iter()
            .find(|clip_box| clip_box.x <= x && x < clip_box.x + clip_box.width)?;
        let edge = EDGE.min(clip_box.width / 3.0);
        let grip = if x < clip_box.x + edge {
            Grip::Start
        } else if x >= clip_box.x + clip_box.width - edge {
            Grip::End
        } else {
            Grip::Body
        };
        Some((self.key(clip_box.start), grip))
    }

    fn key(&self, start: Position<Musical>) -> ClipKey {
        ClipKey {
            track: self.track.id(),
            start,
        }
    }

    fn is_inside(&self, marquee: &Marquee) -> bool {
        let index = |track| self.project.tracks().position(|other| other.id() == track);
        let (Some(first), Some(last), Some(this)) = (
            index(marquee.tracks.0),
            index(marquee.tracks.1),
            index(self.track.id()),
        ) else {
            return false;
        };
        (first.min(last)..=first.max(last)).contains(&this)
    }
}

fn draw_outline(frame: &mut Frame, area: Rectangle, color: Color) {
    let color = iced::Color::from(color);
    let (left, top) = (area.x, area.y);
    let (right, bottom) = (
        area.x + area.width - OUTLINE,
        area.y + area.height - OUTLINE,
    );
    frame.fill_rectangle(Point::new(left, top), Size::new(area.width, OUTLINE), color);
    frame.fill_rectangle(
        Point::new(left, bottom),
        Size::new(area.width, OUTLINE),
        color,
    );
    frame.fill_rectangle(
        Point::new(left, top),
        Size::new(OUTLINE, area.height),
        color,
    );
    frame.fill_rectangle(
        Point::new(right, top),
        Size::new(OUTLINE, area.height),
        color,
    );
}

fn draw_marquee(frame: &mut Frame, viewport: Viewport, marquee: Marquee, palette: Palette) {
    let height = frame.size().height;
    let left = viewport.x_of(marquee.from.min(marquee.to));
    let right = viewport.x_of(marquee.from.max(marquee.to));
    let fill = iced::Color {
        a: MARQUEE_ALPHA,
        ..iced::Color::from(palette.accent)
    };
    let border = iced::Color::from(palette.accent);
    frame.fill_rectangle(Point::new(left, 0.0), Size::new(right - left, height), fill);
    frame.fill_rectangle(Point::new(left, 0.0), Size::new(1.0, height), border);
    frame.fill_rectangle(Point::new(right, 0.0), Size::new(1.0, height), border);
}

fn draw_waveform(frame: &mut Frame, area: Rectangle, columns: &[WaveColumn], palette: Palette) {
    let middle = area.center_y();
    let half = area.height / 2.0 - WAVE_MARGIN;
    let color = iced::Color {
        a: WAVE_ALPHA,
        ..iced::Color::from(palette.on_track_color)
    };
    for column in columns
        .iter()
        .filter(|column| column.x < area.x + area.width)
    {
        let top = (middle - column.peak.max * half).max(area.y);
        let bottom = (middle - column.peak.min * half).min(area.y + area.height);
        frame.fill_rectangle(
            Point::new(column.x, top),
            Size::new(1.0, (bottom - top).max(1.0)),
            color,
        );
    }
}

pub fn lane<'a>(
    view: View<'a>,
    project: &'a Project,
    track: &'a Track,
    palette: Palette,
) -> Element<'a, Command> {
    let lane = Lane {
        view,
        project,
        track,
        palette,
    };
    Canvas::new(lane)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipBox<'a> {
    pub x: f32,
    pub width: f32,
    pub start: Position<Musical>,
    pub clip: &'a Clip,
}

impl Viewport {
    pub fn clip_boxes<'a>(
        &self,
        project: &Project,
        track: &'a Track,
        width: f32,
    ) -> Vec<ClipBox<'a>> {
        let next_starts = track
            .timeline()
            .skip(1)
            .map(|(start, _)| Some(start))
            .chain([None]);
        track
            .timeline()
            .zip(next_starts)
            .filter_map(|((start, clip), next_start)| {
                let end = project.clip_end(start, clip);
                let stop = next_start.map_or(end, |next_start| end.min(next_start));
                let x = self.x_of(start);
                let clip_box = ClipBox {
                    x,
                    width: self.x_of(stop) - x,
                    start,
                    clip,
                };
                (x < width && x + clip_box.width > 0.0).then_some(clip_box)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use iced::keyboard::{self, Modifiers};
    use iced::widget::canvas;
    use iced::{Event, Point, Rectangle, Size, mouse};
    use iced_test::simulator;
    use sibilant_core::project::{AudioSource, Clip, ClipKey, Ids, Project, Region, Track};
    use sibilant_core::time::{Duration, Musical, Position, Tempo, TempoMap};
    use sibilant_media::{Audio, Peaks};

    use crate::command::Command;
    use crate::pixels::Pixels;
    use crate::theme::Palette;
    use crate::timeline::lane::Lane;
    use crate::timeline::{Grip, Marquee, Pointer, View, Viewport, Waveforms, lane};

    fn quarter_notes(quarter_notes: i64) -> Position<Musical> {
        Position::from_quarter_notes(quarter_notes)
    }

    fn project_with_clips(starts: &[i64], seconds: i64, tempo_map: TempoMap) -> Project {
        let mut ids = Ids::default();
        let project = Project::new(&mut ids).with_tempo_map(tempo_map);
        let (source, track) = (ids.source(), project.tracks().next().unwrap().id());
        let length = Duration::from_seconds(seconds);
        let region = Region {
            start: Duration::ZERO,
            length,
        };
        let project = project
            .add_source(AudioSource::new(source, "loop.wav", length))
            .unwrap();
        starts.iter().fold(project, |project, &start| {
            let clip = Clip::new("Loop", source, region);
            project
                .place_clip(track, quarter_notes(start), clip)
                .unwrap()
        })
    }

    fn first_track(project: &Project) -> &Track {
        project.tracks().next().unwrap()
    }

    fn boxes(project: &Project, width: f32) -> Vec<(f32, f32)> {
        Viewport::default()
            .clip_boxes(project, first_track(project), width)
            .iter()
            .map(|clip_box| (clip_box.x, clip_box.width))
            .collect()
    }

    #[test]
    fn a_clip_box_spans_the_clip_on_the_timeline() {
        let project = project_with_clips(&[4], 2, TempoMap::default());
        assert_eq!(boxes(&project, 1000.0), [(128.0, 128.0)]);
    }

    #[test]
    fn a_clip_box_shows_the_name_of_its_clip() {
        let project = project_with_clips(&[4], 2, TempoMap::default());
        let viewport = Viewport::default();
        let clip_boxes = viewport.clip_boxes(&project, first_track(&project), 1000.0);
        assert_eq!(clip_boxes[0].clip.name(), "Loop");
        assert_eq!(clip_boxes[0].start, quarter_notes(4));
    }

    #[test]
    fn a_clip_box_stops_where_the_next_clip_starts() {
        let at_60_bpm = TempoMap::new(Tempo::from_bpm(60.0).unwrap());
        let project = project_with_clips(&[0, 4], 4, at_60_bpm).with_tempo_map(TempoMap::default());
        assert_eq!(boxes(&project, 1000.0), [(0.0, 128.0), (128.0, 256.0)]);
    }

    #[test]
    fn clips_outside_the_view_have_no_box() {
        let project = project_with_clips(&[100], 2, TempoMap::default());
        assert_eq!(boxes(&project, 300.0), []);
    }

    #[test]
    fn the_playhead_is_a_line_in_the_accent_color() {
        let (viewport, palette) = (Viewport::default(), Palette::GRAPHITE);
        let project = Project::new(&mut Ids::default());
        let view = View {
            viewport,
            playhead: viewport.position_at(144.0),
            ..View::default()
        };
        let lane = lane(view, &project, first_track(&project), palette);
        let pixels = Pixels::render(lane, Size::new(400.0, 64.0));
        assert_eq!(pixels.color_at(144, 32), palette.accent);
        assert_eq!(pixels.color_at(146, 32), palette.background);
    }

    fn dragging() -> View<'static> {
        View {
            dragging: true,
            ..View::default()
        }
    }

    #[test]
    fn a_dragged_file_shows_its_drop_position_as_a_wide_line() {
        let palette = Palette::GRAPHITE;
        let project = Project::new(&mut Ids::default());
        let lane = lane(dragging(), &project, first_track(&project), palette);
        let pixels = Pixels::render_pointing(lane, Size::new(400.0, 64.0), Point::new(40.0, 32.0));
        assert_eq!(pixels.color_at(32, 10), palette.accent);
        assert_eq!(pixels.color_at(33, 10), palette.accent);
        assert_eq!(pixels.color_at(36, 10), palette.background);
    }

    #[test]
    fn without_a_drag_the_lane_shows_no_drop_line() {
        let palette = Palette::GRAPHITE;
        let project = Project::new(&mut Ids::default());
        let lane = lane(View::default(), &project, first_track(&project), palette);
        let pixels = Pixels::render_pointing(lane, Size::new(400.0, 64.0), Point::new(40.0, 32.0));
        assert_eq!(pixels.color_at(33, 10), palette.background);
    }

    fn one_clip() -> (Project, ClipKey) {
        let project = project_with_clips(&[4], 2, TempoMap::default());
        let key = ClipKey {
            track: first_track(&project).id(),
            start: quarter_notes(4),
        };
        (project, key)
    }

    fn pointer_events(view: View<'_>, project: &Project, events: Vec<Event>) -> Vec<Command> {
        let lane = lane(view, project, first_track(project), Palette::GRAPHITE);
        let mut ui = simulator(lane);
        ui.simulate(events);
        ui.into_messages().collect()
    }

    fn press_at(project: &Project, x: f32, modifiers: Modifiers) -> Vec<Command> {
        let lane = lane(
            View::default(),
            project,
            first_track(project),
            Palette::GRAPHITE,
        );
        let mut ui = simulator(lane);
        ui.point_at(Point::new(x, 32.0));
        ui.simulate([
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        ]);
        ui.into_messages().collect()
    }

    fn grip_at(project: &Project, x: f32) -> Option<(ClipKey, Grip)> {
        match press_at(project, x, Modifiers::empty())[..] {
            [Command::Pointer(Pointer::Pressed { grip, .. })] => grip,
            _ => panic!("the press gave no pointer event"),
        }
    }

    #[test]
    fn a_press_on_a_clip_names_the_clip_and_the_position() {
        let (project, key) = one_clip();
        assert_eq!(
            press_at(&project, 190.0, Modifiers::empty()),
            [Command::Pointer(Pointer::Pressed {
                track: key.track,
                at: Viewport::default().position_at(190.0),
                grip: Some((key, Grip::Body)),
                toggle: false,
                free: false,
            })]
        );
    }

    #[test]
    fn a_press_near_an_edge_of_a_clip_names_the_edge() {
        let (project, key) = one_clip();
        assert_eq!(grip_at(&project, 130.0), Some((key, Grip::Start)));
        assert_eq!(grip_at(&project, 253.0), Some((key, Grip::End)));
    }

    #[test]
    fn a_press_on_empty_space_names_no_clip() {
        let (project, _) = one_clip();
        assert_eq!(grip_at(&project, 50.0), None);
    }

    #[test]
    fn a_press_tells_if_ctrl_and_alt_are_down() {
        let (project, _) = one_clip();
        let pressed = press_at(&project, 50.0, Modifiers::COMMAND | Modifiers::ALT);
        assert!(matches!(
            pressed[..],
            [Command::Pointer(Pointer::Pressed {
                toggle: true,
                free: true,
                ..
            })]
        ));
    }

    fn move_and_release(x: f32) -> Vec<Event> {
        vec![
            Event::Mouse(mouse::Event::CursorMoved {
                position: Point::new(x, 32.0),
            }),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ]
    }

    #[test]
    fn while_pointing_the_lane_reports_each_move_and_the_release() {
        let (project, key) = one_clip();
        let view = View {
            pointing: true,
            ..View::default()
        };
        let at = Viewport::default().position_at(64.0);
        let lane = lane(view, &project, first_track(&project), Palette::GRAPHITE);
        let mut ui = simulator(lane);
        ui.point_at(Point::new(64.0, 32.0));
        ui.simulate(move_and_release(64.0));
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            [
                Command::Pointer(Pointer::Moved {
                    track: key.track,
                    at,
                    free: false
                }),
                Command::Pointer(Pointer::Released {
                    track: key.track,
                    at,
                    free: false
                })
            ]
        );
    }

    #[test]
    fn without_pointing_the_lane_reports_no_moves_and_no_release() {
        let (project, _) = one_clip();
        assert_eq!(
            pointer_events(View::default(), &project, move_and_release(64.0)),
            []
        );
    }

    fn cursor_at(view: View<'_>, project: &Project, x: f32) -> mouse::Interaction {
        let lane = Lane {
            view,
            project,
            track: first_track(project),
            palette: Palette::GRAPHITE,
        };
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(400.0, 64.0));
        let cursor = mouse::Cursor::Available(Point::new(x, 32.0));
        canvas::Program::mouse_interaction(&lane, &Modifiers::empty(), bounds, cursor)
    }

    #[test]
    fn the_edges_of_a_clip_show_a_resize_cursor() {
        let (project, _) = one_clip();
        let resize = mouse::Interaction::ResizingHorizontally;
        assert_eq!(cursor_at(View::default(), &project, 130.0), resize);
        assert_eq!(cursor_at(View::default(), &project, 253.0), resize);
        assert_eq!(
            cursor_at(View::default(), &project, 190.0),
            mouse::Interaction::default()
        );
        let dragging = View {
            dragging: true,
            ..View::default()
        };
        assert_eq!(
            cursor_at(dragging, &project, 130.0),
            mouse::Interaction::Grabbing
        );
    }

    #[test]
    fn a_selected_clip_has_an_outline() {
        let (project, key) = one_clip();
        let palette = Palette::GRAPHITE;
        let clip = palette.track_color(first_track(&project).color());
        let selection = BTreeSet::from([key]);
        let selected = View {
            selection: &selection,
            ..View::default()
        };
        let size = Size::new(400.0, 64.0);
        let outlined = Pixels::render(
            lane(selected, &project, first_track(&project), palette),
            size,
        );
        let plain = Pixels::render(
            lane(View::default(), &project, first_track(&project), palette),
            size,
        );
        assert_eq!(outlined.color_at(190, 3), palette.text);
        assert_eq!(outlined.color_at(190, 10), clip);
        assert_eq!(plain.color_at(190, 3), clip);
    }

    #[test]
    fn the_box_covers_its_span_on_each_track_inside_it() {
        let palette = Palette::GRAPHITE;
        let project = Project::new(&mut Ids::default());
        let tracks: Vec<_> = project.tracks().collect();
        let marquee = Marquee {
            from: quarter_notes(3),
            to: quarter_notes(1),
            tracks: (tracks[0].id(), tracks[0].id()),
        };
        let view = View {
            marquee: Some(marquee),
            ..View::default()
        };
        let size = Size::new(400.0, 64.0);
        let inside = Pixels::render(lane(view, &project, tracks[0], palette), size);
        let outside = Pixels::render(lane(view, &project, tracks[1], palette), size);
        assert_eq!(inside.color_at(32, 20), palette.accent);
        assert_ne!(inside.color_at(48, 20), palette.background);
        assert_eq!(inside.color_at(150, 20), palette.background);
        assert_eq!(outside.color_at(48, 20), palette.background);
    }

    #[test]
    fn a_clip_shows_the_waveform_of_its_audio() {
        let palette = Palette::GRAPHITE;
        let project = project_with_clips(&[0], 2, TempoMap::default());
        let source = first_track(&project).timeline().next().unwrap().1.source();
        let mut samples = [1.0, -1.0].repeat(24_000);
        samples.resize(96_000, 0.0);
        let peaks = Arc::new(Peaks::of(&Audio::new(48_000, vec![samples])));
        let waveforms = Waveforms::from([(source, peaks)]);
        let view = View {
            waveforms: &waveforms,
            ..View::default()
        };
        let lane = lane(view, &project, first_track(&project), palette);
        let pixels = Pixels::render(lane, Size::new(400.0, 64.0));
        let clip = palette.track_color(first_track(&project).color());
        assert_ne!(pixels.color_at(32, 20), clip);
        assert_eq!(pixels.color_at(96, 20), clip);
    }
}
