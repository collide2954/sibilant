use sibilant_core::project::Project;
use sibilant_media::{Peak, Peaks};

use super::{ClipBox, Viewport};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveColumn {
    pub x: f32,
    pub peak: Peak,
}

impl Viewport {
    pub fn waveform(
        &self,
        project: &Project,
        clip_box: &ClipBox<'_>,
        peaks: &Peaks,
        width: f32,
    ) -> Vec<WaveColumn> {
        let tempo = project.tempo_map();
        let region = clip_box.clip.region();
        let rate = peaks.sample_rate();
        let first = region.start.to_frames(rate);
        let last = (region.start + region.length).to_frames(rate);
        let clip_start = tempo.to_clock(clip_box.start);
        let frame_at = |x: f32| {
            let into_clip = tempo.to_clock(self.position_at(x)) - clip_start;
            (into_clip + region.start)
                .to_frames(rate)
                .clamp(first, last)
        };
        let right = (clip_box.x + clip_box.width).min(width);
        std::iter::successors(Some(clip_box.x.max(0.0).floor()), |x| Some(x + 1.0))
            .take_while(|&x| x < right)
            .filter_map(|x| {
                let frames = index(frame_at(x))..index(frame_at(x + 1.0));
                peaks.range(frames).map(|peak| WaveColumn { x, peak })
            })
            .collect()
    }
}

fn index(frame: i64) -> usize {
    usize::try_from(frame).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use sibilant_core::project::{AudioSource, Clip, Ids, Project, Region};
    use sibilant_core::time::{Duration, Musical, Position};
    use sibilant_media::{Audio, Peak, Peaks};

    use crate::timeline::{Viewport, WaveColumn};

    const RATE: u32 = 48_000;

    fn half_up_half_down() -> Peaks {
        let mut samples = vec![0.5; 48_000];
        samples.resize(96_000, -0.5);
        Peaks::of(&Audio::new(RATE, vec![samples]))
    }

    fn project_with_clip(start: Position<Musical>, region: Region) -> Project {
        let mut ids = Ids::default();
        let project = Project::new(&mut ids);
        let (source, track) = (ids.source(), project.tracks().next().unwrap().id());
        project
            .add_source(AudioSource::new(
                source,
                "loop.wav",
                Duration::from_seconds(2),
            ))
            .and_then(|project| project.place_clip(track, start, Clip::new("Loop", source, region)))
            .unwrap()
    }

    fn columns(project: &Project, width: f32) -> Vec<WaveColumn> {
        let viewport = Viewport::default();
        let track = project.tracks().next().unwrap();
        let clip_box = viewport.clip_boxes(project, track, width)[0];
        viewport.waveform(project, &clip_box, &half_up_half_down(), width)
    }

    fn whole() -> Region {
        Region {
            start: Duration::ZERO,
            length: Duration::from_seconds(2),
        }
    }

    const UP: Peak = Peak { min: 0.5, max: 0.5 };
    const DOWN: Peak = Peak {
        min: -0.5,
        max: -0.5,
    };

    #[test]
    fn each_pixel_of_a_clip_shows_the_peak_of_its_part_of_the_audio() {
        let columns = columns(&project_with_clip(Position::START, whole()), 1_000.0);
        assert_eq!(columns.len(), 128);
        assert_eq!(columns[10], WaveColumn { x: 10.0, peak: UP });
        assert_eq!(
            columns[100],
            WaveColumn {
                x: 100.0,
                peak: DOWN
            }
        );
    }

    #[test]
    fn the_region_start_moves_the_audio_into_the_clip() {
        let region = Region {
            start: Duration::from_seconds(1),
            length: Duration::from_seconds(1),
        };
        let columns = columns(&project_with_clip(Position::START, region), 1_000.0);
        assert_eq!(columns.len(), 64);
        assert_eq!(columns[10].peak, DOWN);
    }

    #[test]
    fn a_clip_later_on_the_timeline_starts_its_waveform_at_its_start() {
        let columns = columns(
            &project_with_clip(Position::from_quarter_notes(4), whole()),
            1_000.0,
        );
        assert_eq!(columns[0].x, 128.0);
        assert_eq!(columns[10].peak, UP);
    }

    #[test]
    fn pixels_outside_the_view_have_no_column() {
        let columns = columns(&project_with_clip(Position::START, whole()), 20.0);
        assert_eq!(columns.len(), 20);
    }
}
