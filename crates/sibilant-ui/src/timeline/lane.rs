use sibilant_core::project::{Project, Track};

use super::Viewport;

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
    use sibilant_core::project::{AudioSource, Clip, Ids, Project, Region, Track};
    use sibilant_core::time::{Duration, Musical, Position, Tempo, TempoMap};

    use crate::timeline::Viewport;

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
}
