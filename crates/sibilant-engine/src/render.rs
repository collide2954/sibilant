use sibilant_core::project::{Project, Track};

use crate::arrangement::Arrangement;
use crate::engine::{self, Command};
use crate::timing::Timing;

const CHANNELS: usize = 2;

pub fn length(project: &Project, sample_rate: u32) -> usize {
    let timing = Timing::new(
        sample_rate,
        project.tempo_map().clone(),
        project.time_signature_map().clone(),
    );
    let end = project
        .tracks()
        .flat_map(Track::timeline)
        .map(|(start, clip)| {
            timing.first_frame_at_or_after(timing.clock_of(start) + clip.region().length)
        })
        .max()
        .unwrap_or(0);
    usize::try_from(end).unwrap_or(0)
}

pub fn render(
    arrangement: Arrangement,
    frames: usize,
    block: usize,
    mut take: impl FnMut(&[f32]) -> bool,
) -> bool {
    let (mut controller, mut engine) = engine::new(arrangement);
    controller.send(Command::TogglePlay);
    let mut buffer = vec![0.0; block * CHANNELS];
    let mut done = 0;
    while done < frames {
        let samples = &mut buffer[..block.min(frames - done) * CHANNELS];
        engine.process(samples, CHANNELS);
        if !take(samples) {
            return false;
        }
        done += samples.len() / CHANNELS;
    }
    true
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sibilant_core::mix::Mix;
    use sibilant_core::project::{AudioSource, Clip, Ids, Project, Region, Track, TrackId};
    use sibilant_core::time::{Duration, Position, TempoMap, TimeSignatureMap};

    use crate::arrangement::{Arrangement, AudioBuffer, ClipPlayback, TrackPlayback};
    use crate::render::{length, render};
    use crate::timing::Timing;

    fn steady(frames: usize, value: f32) -> Arrangement {
        let timing = Timing::new(48_000, TempoMap::default(), TimeSignatureMap::default());
        let clip = ClipPlayback {
            start: 0,
            end: i64::try_from(frames).unwrap(),
            offset: 0,
            fade: 1,
            audio: Arc::new(AudioBuffer::new(vec![vec![value; frames]])),
        };
        let track = TrackPlayback {
            gains: [1.0, 1.0],
            meter: 0,
            clips: vec![clip],
        };
        Arrangement::new(timing, vec![track])
    }

    #[test]
    fn render_plays_the_arrangement_from_the_start_in_blocks() {
        let (mut blocks, mut left) = (Vec::new(), Vec::<f32>::new());
        let finished = render(steady(100, 0.5), 150, 64, |samples| {
            blocks.push(samples.len() / 2);
            left.extend(samples.iter().step_by(2).copied());
            true
        });
        assert!(finished);
        assert_eq!(blocks, [64, 64, 22]);
        assert_eq!(left[50], 0.5);
        assert_eq!(left[120], 0.0);
    }

    fn with_clips(clips: &[(usize, i64, i64)]) -> Project {
        let mut ids = Ids::default();
        let source = ids.source();
        let mut project = Project::new(&mut ids)
            .add_source(AudioSource::new(
                source,
                "loop.wav",
                Duration::from_seconds(8),
            ))
            .unwrap();
        let tracks: Vec<TrackId> = project.tracks().map(Track::id).collect();
        for &(track, quarter_notes, seconds) in clips {
            let region = Region {
                start: Duration::ZERO,
                length: Duration::from_seconds(seconds),
            };
            let start = Position::from_quarter_notes(quarter_notes);
            let clip = Clip::new("Loop", source, region);
            project = project.place_clip(tracks[track], start, clip).unwrap();
        }
        project
    }

    #[test]
    fn a_project_without_clips_has_no_length() {
        assert_eq!(length(&with_clips(&[]), 48_000), 0);
    }

    #[test]
    fn the_length_reaches_from_the_start_to_the_end_of_the_last_clip_of_all_tracks() {
        let project = with_clips(&[(0, 0, 1), (1, 4, 1)]);
        assert_eq!(length(&project, 48_000), 144_000);
        assert_eq!(length(&project, 44_100), 132_300);
    }

    #[test]
    fn a_clip_that_starts_first_but_ends_last_sets_the_length() {
        let project = with_clips(&[(0, 0, 4), (1, 2, 1)]);
        assert_eq!(length(&project, 48_000), 192_000);
    }

    #[test]
    fn the_clips_of_a_muted_track_count_for_the_length() {
        let project = with_clips(&[(0, 0, 1), (1, 4, 1)]);
        let last = project.tracks().nth(1).map(Track::id).unwrap();
        let muted = Mix {
            mute: true,
            ..Mix::default()
        };
        let project = project.set_mix(last, muted).unwrap();
        assert_eq!(length(&project, 48_000), 144_000);
    }

    #[test]
    fn render_stops_when_the_receiver_asks() {
        let mut blocks = 0;
        let finished = render(steady(100, 0.5), 1_000, 64, |_| {
            blocks += 1;
            blocks < 2
        });
        assert!(!finished);
        assert_eq!(blocks, 2);
    }
}
