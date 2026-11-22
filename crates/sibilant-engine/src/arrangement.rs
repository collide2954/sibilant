use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use sibilant_core::project::{Project, SceneId, SourceId, Track, TrackId};
use sibilant_core::time::Quantize;

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

    pub(crate) fn frame(&self, index: usize) -> Option<[f32; 2]> {
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
pub struct SlotPlayback {
    pub scene: SceneId,
    pub offset: i64,
    pub length: i64,
    pub fade: i64,
    pub audio: Arc<AudioBuffer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackPlayback {
    pub id: TrackId,
    pub heard: bool,
    pub gains: [f32; 2],
    pub meter: usize,
    pub clips: Vec<ClipPlayback>,
    pub slots: Vec<SlotPlayback>,
}

#[derive(Debug, Clone)]
pub struct Arrangement {
    timing: Timing,
    tracks: Vec<TrackPlayback>,
    master: f32,
    quantize: Quantize,
    meters: Arc<[AtomicU32]>,
}

impl Arrangement {
    pub fn new(timing: Timing, tracks: Vec<TrackPlayback>) -> Self {
        let meters = tracks
            .iter()
            .map(|track| track.meter + 1)
            .max()
            .unwrap_or(0);
        Self {
            timing,
            tracks,
            master: 1.0,
            quantize: Quantize::default(),
            meters: silent(meters),
        }
    }

    #[must_use]
    pub fn with_master(self, master: f32) -> Self {
        Self { master, ..self }
    }

    #[must_use]
    pub fn with_quantize(self, quantize: Quantize) -> Self {
        Self { quantize, ..self }
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
        let heard: Vec<TrackId> = project.heard_tracks().map(Track::id).collect();
        let tracks = project
            .tracks()
            .enumerate()
            .map(|(meter, track)| {
                let mix = track.mix();
                let gain = mix.gain.amplitude();
                let pan = mix.pan.channel_gains();
                TrackPlayback {
                    id: track.id(),
                    heard: heard.contains(&track.id()),
                    gains: [to_f32(gain * pan.left), to_f32(gain * pan.right)],
                    meter,
                    clips: clips(&timing, track, &audio),
                    slots: slots(&timing, track, &audio),
                }
            })
            .collect();
        Self {
            timing,
            tracks,
            master: to_f32(project.master_gain().amplitude()),
            quantize: project.launch_quantize(),
            meters: silent(project.tracks().len()),
        }
    }

    pub fn timing(&self) -> &Timing {
        &self.timing
    }

    pub fn quantize(&self) -> Quantize {
        self.quantize
    }

    pub fn tracks(&self) -> &[TrackPlayback] {
        &self.tracks
    }

    pub fn master(&self) -> f32 {
        self.master
    }

    pub fn meter_count(&self) -> usize {
        self.meters.len()
    }

    pub(crate) fn meters(&self) -> &Arc<[AtomicU32]> {
        &self.meters
    }
}

fn silent(meters: usize) -> Arc<[AtomicU32]> {
    (0..meters).map(|_| AtomicU32::new(0)).collect()
}

fn clips(
    timing: &Timing,
    track: &Track,
    audio: &impl Fn(SourceId) -> Option<Arc<AudioBuffer>>,
) -> Vec<ClipPlayback> {
    let timeline: Vec<_> = track.timeline().collect();
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
                fade: fade(timing, end - first),
                audio,
            })
        })
        .collect()
}

fn slots(
    timing: &Timing,
    track: &Track,
    audio: &impl Fn(SourceId) -> Option<Arc<AudioBuffer>>,
) -> Vec<SlotPlayback> {
    let rate = timing.sample_rate();
    track
        .slots()
        .filter_map(|(scene, clip)| {
            let length = clip.region().length.to_frames(rate);
            if length <= 0 {
                return None;
            }
            Some(SlotPlayback {
                scene,
                offset: clip.region().start.to_frames(rate),
                length,
                fade: fade(timing, length),
                audio: audio(clip.source())?,
            })
        })
        .collect()
}

fn fade(timing: &Timing, frames: i64) -> i64 {
    let fade = i64::from(timing.sample_rate()) * FADE_MILLISECONDS / 1_000;
    fade.min(frames / 2).max(1)
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
    use sibilant_core::project::{
        AudioSource, Clip, Ids, Project, Region, SceneId, SourceId, Track, TrackId,
    };
    use sibilant_core::time::{
        Clock, Duration, Musical, Position, Quantize, SUPERCLOCKS_PER_SECOND, Tempo, TempoMap,
    };

    use crate::arrangement::{Arrangement, AudioBuffer, ClipPlayback, SlotPlayback};

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
    fn each_track_gets_its_gain_and_its_pan_and_the_output_gets_the_master_gain() {
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
        let arrangement = session.arrangement();
        let [left, right] = arrangement.tracks()[0].gains;
        assert!((left - 0.05).abs() < 1e-6, "left {left}");
        assert!((right - 0.1).abs() < 1e-6, "right {right}");
        let master = arrangement.master();
        assert!((master - 10_f32.powf(0.3)).abs() < 1e-6, "master {master}");
    }

    fn muted(mut session: Session) -> Session {
        let muted = Mix {
            mute: true,
            ..Mix::default()
        };
        session.project = session.project.set_mix(session.track, muted).unwrap();
        session
    }

    #[test]
    fn each_track_keeps_its_id_and_tells_if_it_is_heard() {
        let session = muted(Session::new(seconds(2)));
        let tracks: Vec<(TrackId, bool)> = session
            .arrangement()
            .tracks()
            .iter()
            .map(|track| (track.id, track.heard))
            .collect();
        let ids: Vec<TrackId> = session.project.tracks().map(Track::id).collect();
        assert_eq!(tracks, [(ids[0], false), (ids[1], true)]);
    }

    #[test]
    fn each_track_meters_into_the_place_of_its_track() {
        let arrangement = muted(Session::new(seconds(2))).arrangement();
        let meters: Vec<usize> = arrangement
            .tracks()
            .iter()
            .map(|track| track.meter)
            .collect();
        assert_eq!(meters, [0, 1]);
        assert_eq!(arrangement.meter_count(), 2);
    }

    fn first_scene(session: &Session) -> SceneId {
        session.project.scenes().next().unwrap().id()
    }

    fn with_slot(mut session: Session, region: Region) -> Session {
        let clip = Clip::new("Loop", session.source, region);
        let scene = first_scene(&session);
        session.project = session
            .project
            .put_clip_in_slot(session.track, scene, clip)
            .unwrap();
        session
    }

    #[test]
    fn each_slot_plays_its_region_in_frames_and_fades_for_4_ms() {
        let region = Region {
            start: milliseconds(500),
            length: seconds(1),
        };
        let session = with_slot(Session::new(seconds(2)), region);
        let slots: Vec<(SceneId, i64, i64, i64)> = session.arrangement().tracks()[0]
            .slots
            .iter()
            .map(|slot: &SlotPlayback| (slot.scene, slot.offset, slot.length, slot.fade))
            .collect();
        assert_eq!(slots, [(first_scene(&session), 24_000, 48_000, 192)]);
    }

    #[test]
    fn a_short_slot_fades_for_half_of_its_length() {
        let session = with_slot(Session::new(seconds(2)), whole(milliseconds(6)));
        assert_eq!(session.arrangement().tracks()[0].slots[0].fade, 144);
    }

    #[test]
    fn a_slot_shorter_than_one_frame_is_left_out() {
        let tiny = whole(Duration::from_superclocks(1));
        let session = with_slot(Session::new(seconds(2)), tiny);
        assert_eq!(session.arrangement().tracks()[0].slots, []);
    }

    #[test]
    fn slots_without_loaded_audio_are_left_out() {
        let session = with_slot(Session::new(seconds(2)), whole(seconds(1)));
        let arrangement = Arrangement::of_project(&session.project, RATE, |_| None);
        assert_eq!(arrangement.tracks()[0].slots, []);
    }

    #[test]
    fn the_arrangement_keeps_the_launch_quantize_of_the_project() {
        let mut session = Session::new(seconds(2));
        session.project = session.project.with_launch_quantize(Quantize::Beat);
        assert_eq!(session.arrangement().quantize(), Quantize::Beat);
    }

    #[test]
    fn the_arrangement_keeps_the_timing_of_the_project() {
        let session = Session::new(seconds(2));
        let timing = session.arrangement().timing().clone();
        assert_eq!(timing.sample_rate(), RATE);
        assert_eq!(timing.frame_of(quarter_notes(1)), 24_000);
    }
}
