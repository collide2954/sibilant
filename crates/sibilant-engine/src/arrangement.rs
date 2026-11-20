use std::sync::Arc;

use sibilant_core::project::{Project, SourceId, Track};

use crate::timing::Timing;

const FADE_MILLISECONDS: i64 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    channels: Vec<Box<[f32]>>,
}

impl AudioBuffer {
    pub fn new(channels: Vec<Vec<f32>>) -> Self {
        Self {
            channels: channels.into_iter().map(Vec::into_boxed_slice).collect(),
        }
    }

    pub fn frames(&self) -> usize {
        self.channels.first().map_or(0, |channel| channel.len())
    }

    fn frame(&self, index: usize) -> Option<[f32; 2]> {
        let left = *self.channels.first()?.get(index)?;
        let right = self
            .channels
            .get(1)
            .and_then(|channel| channel.get(index))
            .copied()
            .unwrap_or(left);
        Some([left, right])
    }
}

impl ClipPlayback {
    pub(crate) fn sample(&self, frame: i64) -> Option<[f32; 2]> {
        let index = usize::try_from(self.offset + frame - self.start).ok()?;
        let [left, right] = self.audio.frame(index)?;
        let gain = self.fade_gain(frame);
        Some([left * gain, right * gain])
    }

    fn fade_gain(&self, frame: i64) -> f32 {
        let edge = (frame - self.start).min(self.end - 1 - frame);
        if edge >= self.fade {
            1.0
        } else {
            ratio(edge, self.fade)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClipPlayback {
    pub start: i64,
    pub end: i64,
    pub offset: i64,
    pub fade: i64,
    pub audio: Arc<AudioBuffer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackPlayback {
    pub gains: [f32; 2],
    pub clips: Vec<ClipPlayback>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arrangement {
    timing: Timing,
    tracks: Vec<TrackPlayback>,
}

impl Arrangement {
    pub fn new(timing: Timing, tracks: Vec<TrackPlayback>) -> Self {
        Self { timing, tracks }
    }

    pub fn of_project(
        project: &Project,
        sample_rate: u32,
        audio: impl Fn(SourceId) -> Option<Arc<AudioBuffer>>,
    ) -> Self {
        let timing = Timing::new(
            sample_rate,
            project.tempo_map().clone(),
            project.time_signature_map().clone(),
        );
        let master = project.master_gain().amplitude();
        let tracks = project
            .heard_tracks()
            .map(|track| {
                let mix = track.mix();
                let gain = mix.gain.amplitude() * master;
                let pan = mix.pan.channel_gains();
                TrackPlayback {
                    gains: [to_f32(gain * pan.left), to_f32(gain * pan.right)],
                    clips: clips(&timing, track, &audio),
                }
            })
            .collect();
        Self { timing, tracks }
    }

    pub fn timing(&self) -> &Timing {
        &self.timing
    }

    pub fn tracks(&self) -> &[TrackPlayback] {
        &self.tracks
    }
}

fn clips(
    timing: &Timing,
    track: &Track,
    audio: &impl Fn(SourceId) -> Option<Arc<AudioBuffer>>,
) -> Vec<ClipPlayback> {
    let timeline: Vec<_> = track.timeline().collect();
    let fade = i64::from(timing.sample_rate()) * FADE_MILLISECONDS / 1_000;
    timeline
        .iter()
        .enumerate()
        .filter_map(|(index, &(start, clip))| {
            let audio = audio(clip.source())?;
            let first = timing.frame_of(start);
            let audio_end =
                timing.first_frame_at_or_after(timing.clock_of(start) + clip.region().length);
            let end = timeline
                .get(index + 1)
                .map_or(audio_end, |&(next, _)| audio_end.min(timing.frame_of(next)));
            (end > first).then(|| ClipPlayback {
                start: first,
                end,
                offset: clip.region().start.to_frames(timing.sample_rate()),
                fade: fade.min((end - first) / 2).max(1),
                audio,
            })
        })
        .collect()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "track gains stay far inside the range of f32"
)]
fn to_f32(value: f64) -> f32 {
    value as f32
}

#[expect(
    clippy::cast_precision_loss,
    reason = "fade lengths stay far below 2^24 frames"
)]
fn ratio(part: i64, whole: i64) -> f32 {
    part as f32 / whole as f32
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sibilant_core::mix::{Gain, Mix, Pan};
    use sibilant_core::project::{AudioSource, Clip, Ids, Project, Region, SourceId, TrackId};
    use sibilant_core::time::{
        Clock, Duration, Musical, Position, SUPERCLOCKS_PER_SECOND, Tempo, TempoMap,
    };

    use crate::arrangement::{Arrangement, AudioBuffer, ClipPlayback};

    const RATE: u32 = 48_000;

    struct Session {
        project: Project,
        source: SourceId,
        track: TrackId,
        audio: Arc<AudioBuffer>,
    }

    impl Session {
        fn new(source_length: Duration<Clock>) -> Self {
            let mut ids = Ids::default();
            let project = Project::new(&mut ids);
            let source = ids.source();
            let track = project.tracks().next().unwrap().id();
            let project = project
                .add_source(AudioSource::new(source, "loop.wav", source_length))
                .unwrap();
            let audio = Arc::new(AudioBuffer::new(vec![vec![0.0; 10]]));
            Self {
                project,
                source,
                track,
                audio,
            }
        }

        fn place(mut self, start: Position<Musical>, region: Region) -> Self {
            let clip = Clip::new("Loop", self.source, region);
            self.project = self.project.place_clip(self.track, start, clip).unwrap();
            self
        }

        fn arrangement(&self) -> Arrangement {
            let audio = Arc::clone(&self.audio);
            let source = self.source;
            Arrangement::of_project(&self.project, RATE, move |id| {
                (id == source).then(|| Arc::clone(&audio))
            })
        }

        fn clips(&self) -> Vec<(i64, i64, i64)> {
            self.arrangement().tracks()[0]
                .clips
                .iter()
                .map(|clip| (clip.start, clip.end, clip.offset))
                .collect()
        }
    }

    fn seconds(seconds: i64) -> Duration<Clock> {
        Duration::from_seconds(seconds)
    }

    fn milliseconds(milliseconds: i64) -> Duration<Clock> {
        Duration::from_superclocks(SUPERCLOCKS_PER_SECOND / 1_000 * milliseconds)
    }

    fn whole(length: Duration<Clock>) -> Region {
        Region {
            start: Duration::ZERO,
            length,
        }
    }

    fn quarter_notes(quarter_notes: i64) -> Position<Musical> {
        Position::from_quarter_notes(quarter_notes)
    }

    #[test]
    fn a_clip_plays_from_the_frame_of_its_position_to_the_end_of_its_audio() {
        let session = Session::new(seconds(2)).place(quarter_notes(4), whole(seconds(2)));
        assert_eq!(session.clips(), [(96_000, 192_000, 0)]);
    }

    #[test]
    fn the_start_of_the_region_is_the_first_frame_of_the_audio_that_plays() {
        let region = Region {
            start: milliseconds(500),
            length: seconds(1),
        };
        let session = Session::new(seconds(2)).place(quarter_notes(0), region);
        assert_eq!(session.clips(), [(0, 48_000, 24_000)]);
    }

    #[test]
    fn the_next_clip_cuts_off_a_clip_at_its_start() {
        let at_60_bpm = TempoMap::new(Tempo::from_bpm(60.0).unwrap());
        let mut session = Session::new(seconds(4));
        session.project = session.project.with_tempo_map(at_60_bpm);
        let mut session = session
            .place(quarter_notes(0), whole(seconds(4)))
            .place(quarter_notes(4), whole(seconds(4)));
        session.project = session.project.with_tempo_map(TempoMap::default());
        assert_eq!(session.clips(), [(0, 96_000, 0), (96_000, 288_000, 0)]);
    }

    #[test]
    fn clips_without_loaded_audio_are_left_out() {
        let session = Session::new(seconds(2)).place(quarter_notes(0), whole(seconds(2)));
        let arrangement = Arrangement::of_project(&session.project, RATE, |_| None);
        assert_eq!(arrangement.tracks()[0].clips, []);
    }

    #[test]
    fn each_clip_fades_for_4_ms_or_for_half_of_a_shorter_clip() {
        let session = Session::new(seconds(2))
            .place(quarter_notes(0), whole(seconds(1)))
            .place(quarter_notes(4), whole(milliseconds(6)));
        let fades: Vec<i64> = session.arrangement().tracks()[0]
            .clips
            .iter()
            .map(|clip: &ClipPlayback| clip.fade)
            .collect();
        assert_eq!(fades, [192, 144]);
    }

    #[test]
    fn each_track_gets_its_gain_its_pan_and_the_master_gain() {
        let mut session = Session::new(seconds(2));
        let mix = Mix {
            gain: Gain::from_db(-20.0).unwrap(),
            pan: Pan::new(0.5).unwrap(),
            ..Mix::default()
        };
        session.project = session
            .project
            .set_mix(session.track, mix)
            .unwrap()
            .with_master_gain(Gain::from_db(6.0).unwrap());
        let [left, right] = session.arrangement().tracks()[0].gains;
        let master = 10_f32.powf(0.3);
        assert!((left - 0.05 * master).abs() < 1e-6, "left {left}");
        assert!((right - 0.1 * master).abs() < 1e-6, "right {right}");
    }

    #[test]
    fn tracks_that_are_not_heard_are_left_out() {
        let mut session = Session::new(seconds(2));
        let muted = Mix {
            mute: true,
            ..Mix::default()
        };
        session.project = session.project.set_mix(session.track, muted).unwrap();
        assert_eq!(session.arrangement().tracks().len(), 1);
    }

    #[test]
    fn the_arrangement_keeps_the_timing_of_the_project() {
        let session = Session::new(seconds(2));
        let timing = session.arrangement().timing().clone();
        assert_eq!(timing.sample_rate(), RATE);
        assert_eq!(timing.frame_of(quarter_notes(1)), 24_000);
    }
}
