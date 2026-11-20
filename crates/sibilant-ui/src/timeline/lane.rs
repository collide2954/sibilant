use iced::keyboard::Modifiers;
use iced::widget::canvas::{self, Action, Canvas, Event, Frame, Geometry};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};
use sibilant_core::project::{Project, Track};
use sibilant_core::time::{Musical, Position};

use super::{Viewport, draw_playhead, wheel};
use crate::command::Command;
use crate::theme::Palette;

const CLIP_INSET: f32 = 2.0;
const CLIP_GAP: f32 = 1.0;
const NAME_INSET: f32 = 4.0;
const NAME_SIZE: f32 = 11.0;

struct Lane<'a> {
    viewport: Viewport,
    playhead: Position<Musical>,
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
        wheel(modifiers, event, bounds, cursor)
    }

    fn draw(
        &self,
        _modifiers: &Modifiers,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
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
        for clip in self
            .viewport
            .clip_boxes(self.project, self.track, bounds.width)
        {
            let area = Rectangle::new(
                Point::new(clip.x, CLIP_INSET),
                Size::new((clip.width - CLIP_GAP).max(1.0), height - 2.0 * CLIP_INSET),
            );
            frame.fill_rectangle(area.position(), area.size(), iced::Color::from(color));
            frame.with_clip(area, |frame| {
                frame.fill_text(canvas::Text {
                    content: clip.name.to_owned(),
                    position: Point::new(area.x + NAME_INSET, area.y + NAME_INSET),
                    color: palette.on_track_color.into(),
                    size: NAME_SIZE.into(),
                    ..canvas::Text::default()
                });
            });
        }
        draw_playhead(&mut frame, self.viewport, self.playhead, palette);
        vec![frame.into_geometry()]
    }
}

pub fn lane<'a>(
    viewport: Viewport,
    playhead: Position<Musical>,
    project: &'a Project,
    track: &'a Track,
    palette: Palette,
) -> Element<'a, Command> {
    let lane = Lane {
        viewport,
        playhead,
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
    pub name: &'a str,
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
                    name: clip.name(),
                };
                (x < width && x + clip_box.width > 0.0).then_some(clip_box)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use iced::Size;
    use sibilant_core::project::{AudioSource, Clip, Ids, Project, Region, Track};
    use sibilant_core::time::{Duration, Musical, Position, Tempo, TempoMap};

    use crate::pixels::Pixels;
    use crate::theme::Palette;
    use crate::timeline::{Viewport, lane};

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
        assert_eq!(clip_boxes[0].name, "Loop");
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
        let playhead = viewport.position_at(144.0);
        let lane = lane(viewport, playhead, &project, first_track(&project), palette);
        let pixels = Pixels::render(lane, Size::new(400.0, 64.0));
        assert_eq!(pixels.color_at(144, 32), palette.accent);
        assert_eq!(pixels.color_at(146, 32), palette.background);
    }
}
