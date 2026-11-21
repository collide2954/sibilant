use std::mem;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicUsize, Ordering};

use rtrb::{Consumer, Producer, RingBuffer};
use sibilant_core::time::{Musical, Position};

use crate::arrangement::{Arrangement, AudioBuffer};
use crate::click::ClickVoice;
use crate::preview::PreviewVoice;

const QUEUE_CAPACITY: usize = 256;
const ARRANGEMENT_CAPACITY: usize = 16;
const PREVIEW_CAPACITY: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    Continue,
    Locate(Position<Musical>),
    SetMetronome(bool),
    SetPreviewGain(f32),
    Stop,
    StopPreview,
    TogglePlay,
}

#[derive(Debug, Default)]
struct Shared {
    playing: AtomicBool,
    previewing: AtomicBool,
    previews_started: AtomicUsize,
    playhead: AtomicI64,
    output: AtomicU32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Levels {
    pub tracks: Vec<f32>,
    pub output: f32,
}

#[derive(Debug)]
pub struct Controller {
    commands: Producer<Command>,
    arrangements: Producer<Box<Arrangement>>,
    released: Consumer<Box<Arrangement>>,
    previews: Producer<Arc<AudioBuffer>>,
    released_previews: Consumer<Arc<AudioBuffer>>,
    shared: Arc<Shared>,
    meters: Arc<[AtomicU32]>,
}

#[derive(Debug)]
pub struct Engine {
    arrangement: Box<Arrangement>,
    commands: Consumer<Command>,
    arrangements: Consumer<Box<Arrangement>>,
    released: Producer<Box<Arrangement>>,
    previews: Consumer<Arc<AudioBuffer>>,
    released_previews: Producer<Arc<AudioBuffer>>,
    preview: PreviewVoice,
    shared: Arc<Shared>,
    playing: bool,
    marker: Position<Musical>,
    stopped_at: i64,
    playhead: i64,
    metronome: bool,
    click: ClickVoice,
}

pub fn new(arrangement: Arrangement) -> (Controller, Engine) {
    let (commands, command_queue) = RingBuffer::new(QUEUE_CAPACITY);
    let (arrangements, arrangement_queue) = RingBuffer::new(ARRANGEMENT_CAPACITY);
    let (released_queue, released) = RingBuffer::new(ARRANGEMENT_CAPACITY);
    let (previews, preview_queue) = RingBuffer::new(PREVIEW_CAPACITY);
    let (released_preview_queue, released_previews) = RingBuffer::new(PREVIEW_CAPACITY);
    let shared = Arc::new(Shared::default());
    let marker = Position::START;
    let start = arrangement.timing().frame_of(marker);
    shared.playhead.store(start, Ordering::Relaxed);
    let controller = Controller {
        commands,
        arrangements,
        released,
        previews,
        released_previews,
        shared: Arc::clone(&shared),
        meters: Arc::clone(arrangement.meters()),
    };
    let sample_rate = arrangement.timing().sample_rate();
    let engine = Engine {
        arrangement: Box::new(arrangement),
        commands: command_queue,
        arrangements: arrangement_queue,
        released: released_queue,
        previews: preview_queue,
        released_previews: released_preview_queue,
        preview: PreviewVoice::new(sample_rate),
        shared,
        playing: false,
        marker,
        stopped_at: start,
        playhead: start,
        metronome: false,
        click: ClickVoice::new(sample_rate),
    };
    (controller, engine)
}

impl Controller {
    pub fn send(&mut self, command: Command) -> bool {
        self.commands.push(command).is_ok()
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    pub fn playhead(&self) -> i64 {
        self.shared.playhead.load(Ordering::Relaxed)
    }

    pub fn is_previewing(&self) -> bool {
        self.shared.previewing.load(Ordering::Relaxed)
    }

    pub fn previews_started(&self) -> usize {
        self.shared.previews_started.load(Ordering::Relaxed)
    }

    pub fn replace_arrangement(&mut self, arrangement: Arrangement) -> bool {
        let meters = Arc::clone(arrangement.meters());
        let sent = self.arrangements.push(Box::new(arrangement)).is_ok();
        if sent {
            self.meters = meters;
        }
        sent
    }

    pub fn take_levels(&self) -> Levels {
        Levels {
            tracks: self.meters.iter().map(take).collect(),
            output: take(&self.shared.output),
        }
    }

    pub fn preview(&mut self, audio: Arc<AudioBuffer>) -> bool {
        self.previews.push(audio).is_ok()
    }

    pub fn release_old(&mut self) {
        while self.released.pop().is_ok() {}
        while self.released_previews.pop().is_ok() {}
    }
}

impl Engine {
    pub fn process(&mut self, output: &mut [f32], channels: usize) {
        self.take_new_arrangements();
        self.take_new_previews();
        while let Ok(command) = self.commands.pop() {
            self.apply(command);
        }
        output.fill(0.0);
        let start = self.playhead;
        let end = start + frames_in(output, channels);
        if self.playing {
            mix_clips(&self.arrangement, output, channels, start, end);
        }
        let timing = self.arrangement.timing();
        let mut clicks = (self.playing && self.metronome).then(|| timing.clicks(start, end));
        let mut next_click = clicks.as_mut().and_then(Iterator::next);
        for (frame, samples) in (start..).zip(output.chunks_exact_mut(channels)) {
            while let Some(click) = next_click.filter(|click| click.frame == frame) {
                self.click.trigger(click.accent);
                next_click = clicks.as_mut().and_then(Iterator::next);
            }
            let click = self.click.next_sample();
            add(samples, [click, click]);
        }
        for samples in output.chunks_exact_mut(channels) {
            let Some(frame) = self.preview.next_frame() else {
                break;
            };
            add(samples, frame);
        }
        let peak = output
            .chunks_exact(channels)
            .flat_map(|samples| samples.iter().take(2))
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        self.shared
            .output
            .fetch_max(peak.to_bits(), Ordering::Relaxed);
        if self.playing {
            self.playhead = end;
        }
        self.shared.playing.store(self.playing, Ordering::Relaxed);
        self.shared
            .previewing
            .store(self.preview.is_playing(), Ordering::Relaxed);
        self.shared.playhead.store(self.playhead, Ordering::Relaxed);
    }

    fn take_new_previews(&mut self) {
        while self.released_previews.slots() > 0 {
            let Ok(audio) = self.previews.pop() else {
                break;
            };
            if let Some(old) = self.preview.start(audio) {
                let _ = self.released_previews.push(old);
            }
            self.shared.previews_started.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn take_new_arrangements(&mut self) {
        while self.released.slots() > 0 {
            let Ok(new) = self.arrangements.pop() else {
                break;
            };
            let old = mem::replace(&mut self.arrangement, new);
            let _ = self.released.push(old);
        }
    }

    fn apply(&mut self, command: Command) {
        let marker = self.arrangement.timing().frame_of(self.marker);
        match command {
            Command::TogglePlay if self.playing => {
                self.stopped_at = self.playhead;
                self.playhead = marker;
                self.playing = false;
            }
            Command::TogglePlay => {
                self.playhead = marker;
                self.playing = true;
            }
            Command::Continue if !self.playing => {
                self.playhead = self.stopped_at;
                self.playing = true;
            }
            Command::Continue => {}
            Command::Locate(position) => {
                self.marker = position;
                let frame = self.arrangement.timing().frame_of(position);
                self.stopped_at = frame;
                self.playhead = frame;
            }
            Command::Stop => {
                self.stopped_at = marker;
                self.playhead = marker;
                self.playing = false;
            }
            Command::SetMetronome(on) => self.metronome = on,
            Command::SetPreviewGain(gain) => self.preview.set_gain(gain),
            Command::StopPreview => self.preview.stop(),
        }
    }
}

fn mix_clips(arrangement: &Arrangement, output: &mut [f32], channels: usize, start: i64, end: i64) {
    let master = arrangement.master();
    for track in arrangement.tracks() {
        let mut peak = 0.0_f32;
        let first = track.clips.partition_point(|clip| clip.end <= start);
        for clip in track.clips[first..]
            .iter()
            .take_while(|clip| clip.start < end)
        {
            let from = clip.start.max(start);
            let skip = usize::try_from(from - start).unwrap_or(0);
            let frames =
                (from..clip.end.min(end)).zip(output.chunks_exact_mut(channels).skip(skip));
            for (frame, samples) in frames {
                if let Some([left, right]) = clip.sample(frame) {
                    let [left, right] = [left * track.gains[0], right * track.gains[1]];
                    peak = peak.max(left.abs()).max(right.abs());
                    add(samples, [left * master, right * master]);
                }
            }
        }
        if let Some(meter) = arrangement.meters().get(track.meter) {
            meter.fetch_max(peak.to_bits(), Ordering::Relaxed);
        }
    }
}

fn take(level: &AtomicU32) -> f32 {
    f32::from_bits(level.swap(0, Ordering::Relaxed))
}

fn add(samples: &mut [f32], [left, right]: [f32; 2]) {
    match samples {
        [mono] => *mono += f32::midpoint(left, right),
        [first, second, ..] => {
            *first += left;
            *second += right;
        }
        [] => {}
    }
}

fn frames_in(output: &[f32], channels: usize) -> i64 {
    i64::try_from(output.len() / channels).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use assert_no_alloc::assert_no_alloc;
    use sibilant_core::time::{Musical, Position, TempoMap, TimeSignatureMap};

    use crate::arrangement::{Arrangement, AudioBuffer, ClipPlayback, TrackPlayback};
    use crate::engine::{self, ARRANGEMENT_CAPACITY, Command, Controller, Engine};
    use crate::timing::Timing;

    fn timing() -> Timing {
        Timing::new(48_000, TempoMap::default(), TimeSignatureMap::default())
    }

    fn engine_with(tracks: Vec<TrackPlayback>) -> (Controller, Engine) {
        engine::new(Arrangement::new(timing(), tracks))
    }

    fn engine() -> (Controller, Engine) {
        engine_with(Vec::new())
    }

    fn playing(tracks: Vec<TrackPlayback>) -> Engine {
        let (mut controller, engine) = engine_with(tracks);
        controller.send(Command::TogglePlay);
        engine
    }

    fn track(gains: [f32; 2], clip: ClipPlayback) -> TrackPlayback {
        track_at(0, gains, clip)
    }

    fn track_at(meter: usize, gains: [f32; 2], clip: ClipPlayback) -> TrackPlayback {
        TrackPlayback {
            gains,
            meter,
            clips: vec![clip],
        }
    }

    fn two_tracks() -> Engine {
        playing(vec![
            track_at(0, [0.5, 0.25], clip(0, 100, 0, 1, vec![vec![0.8; 100]])),
            track_at(1, [1.0, 1.0], clip(0, 100, 0, 1, vec![vec![0.3; 100]])),
        ])
    }

    #[test]
    fn the_engine_reports_the_peak_of_each_track_and_of_the_output() {
        let (mut controller, mut engine) = engine_with(vec![
            track_at(0, [0.5, 0.25], clip(0, 100, 0, 1, vec![vec![0.8; 100]])),
            track_at(1, [1.0, 1.0], clip(0, 100, 0, 1, vec![vec![0.3; 100]])),
        ]);
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 100);
        let levels = controller.take_levels();
        assert!((levels.tracks[0] - 0.4).abs() < 1e-6, "{levels:?}");
        assert!((levels.tracks[1] - 0.3).abs() < 1e-6, "{levels:?}");
        assert!((levels.output - 0.7).abs() < 1e-6, "{levels:?}");
    }

    #[test]
    fn taking_the_levels_starts_them_again_from_silence() {
        let (mut controller, mut engine) = engine_with(vec![track(
            [1.0, 1.0],
            clip(0, 100, 0, 1, vec![vec![0.5; 100]]),
        )]);
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 100);
        let _ = controller.take_levels();
        let levels = controller.take_levels();
        assert_eq!((levels.tracks, levels.output), (vec![0.0], 0.0));
    }

    #[test]
    fn the_master_gain_scales_the_output_but_not_the_track_meters() {
        let arrangement = Arrangement::new(
            timing(),
            vec![track([1.0, 1.0], clip(0, 100, 0, 1, vec![vec![0.8; 100]]))],
        )
        .with_master(0.5);
        let (mut controller, mut engine) = engine::new(arrangement);
        controller.send(Command::TogglePlay);
        let buffer = run(&mut engine, 100);
        let levels = controller.take_levels();
        assert!((buffer[100] - 0.4).abs() < 1e-6, "{}", buffer[100]);
        assert!((levels.tracks[0] - 0.8).abs() < 1e-6, "{levels:?}");
        assert!((levels.output - 0.4).abs() < 1e-6, "{levels:?}");
    }

    #[test]
    fn the_levels_follow_a_new_arrangement() {
        let (mut controller, mut engine) = engine();
        assert!(controller.replace_arrangement(Arrangement::new(
            timing(),
            vec![track_at(
                2,
                [1.0, 1.0],
                clip(0, 100, 0, 1, vec![vec![0.5; 100]])
            )],
        )));
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 100);
        assert_eq!(controller.take_levels().tracks, vec![0.0, 0.0, 0.5]);
    }

    #[test]
    fn metering_never_allocates_memory_in_the_engine() {
        let mut engine = two_tracks();
        let mut buffer = vec![0.0; 1_024];
        assert_no_alloc(|| engine.process(&mut buffer, 2));
    }

    fn clip(start: i64, end: i64, offset: i64, fade: i64, audio: Vec<Vec<f32>>) -> ClipPlayback {
        ClipPlayback {
            start,
            end,
            offset,
            fade,
            audio: Arc::new(AudioBuffer::new(audio)),
        }
    }

    fn render(engine: &mut Engine, frames: usize, channels: usize) -> Vec<f32> {
        let mut buffer = vec![1.0; frames * channels];
        engine.process(&mut buffer, channels);
        buffer
    }

    fn run(engine: &mut Engine, frames: usize) -> Vec<f32> {
        render(engine, frames, 2)
    }

    fn channel(buffer: &[f32], channels: usize, index: usize) -> Vec<f32> {
        buffer
            .iter()
            .skip(index)
            .step_by(channels)
            .copied()
            .collect()
    }

    fn left(buffer: &[f32]) -> Vec<f32> {
        channel(buffer, 2, 0)
    }

    fn peak(samples: &[f32]) -> f32 {
        samples
            .iter()
            .fold(0.0, |peak, sample| peak.max(sample.abs()))
    }

    #[test]
    fn a_new_engine_is_stopped_at_bar_1() {
        let (controller, mut engine) = engine();
        let _ = run(&mut engine, 512);
        assert!(!controller.is_playing());
        assert_eq!(controller.playhead(), 0);
    }

    #[test]
    fn a_stopped_engine_fills_the_buffer_with_silence() {
        let audio = vec![vec![0.5; 1_000]];
        let (_controller, mut engine) =
            engine_with(vec![track([1.0, 1.0], clip(0, 1_000, 0, 1, audio))]);
        assert!(run(&mut engine, 512).iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn playback_moves_the_playhead_by_the_frames_of_each_buffer() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 512);
        let _ = run(&mut engine, 256);
        assert!(controller.is_playing());
        assert_eq!(controller.playhead(), 768);
    }

    #[test]
    fn stop_returns_the_playhead_to_the_play_start() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 1_000);
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 100);
        assert!(!controller.is_playing());
        assert_eq!(controller.playhead(), 0);
    }

    #[test]
    fn continue_plays_from_where_playback_stopped() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 1_000);
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 10);
        controller.send(Command::Continue);
        let _ = run(&mut engine, 500);
        assert_eq!(controller.playhead(), 1_500);
    }

    #[test]
    fn stop_ends_the_playback_and_returns_to_the_play_start() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 1_000);
        controller.send(Command::Stop);
        let _ = run(&mut engine, 10);
        assert!(!controller.is_playing());
        assert_eq!(controller.playhead(), 0);
    }

    #[test]
    fn continue_after_a_stop_plays_from_the_play_start() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 1_000);
        controller.send(Command::TogglePlay);
        controller.send(Command::Stop);
        controller.send(Command::Continue);
        let _ = run(&mut engine, 500);
        assert_eq!(controller.playhead(), 500);
    }

    fn quarter_notes(quarter_notes: i64) -> Position<Musical> {
        Position::from_quarter_notes(quarter_notes)
    }

    #[test]
    fn locate_moves_the_play_start() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::Locate(quarter_notes(1)));
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 100);
        assert_eq!(controller.playhead(), 24_100);
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 10);
        assert_eq!(controller.playhead(), 24_000);
    }

    #[test]
    fn locate_during_playback_plays_on_from_the_new_position() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 1_000);
        controller.send(Command::Locate(quarter_notes(2)));
        let _ = run(&mut engine, 100);
        assert!(controller.is_playing());
        assert_eq!(controller.playhead(), 48_100);
    }

    #[test]
    fn locate_while_stopped_moves_the_playhead_and_the_continue_point() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 1_000);
        controller.send(Command::TogglePlay);
        controller.send(Command::Locate(quarter_notes(1)));
        let _ = run(&mut engine, 10);
        assert_eq!(controller.playhead(), 24_000);
        controller.send(Command::Continue);
        let _ = run(&mut engine, 100);
        assert_eq!(controller.playhead(), 24_100);
    }

    #[test]
    fn stop_leaves_a_stopped_engine_stopped() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::Stop);
        let _ = run(&mut engine, 100);
        assert!(!controller.is_playing());
        assert_eq!(controller.playhead(), 0);
    }

    #[test]
    fn a_clip_plays_its_audio_between_its_start_and_its_end() {
        let audio = vec![vec![0.5; 2_000]];
        let mut engine = playing(vec![track([1.0, 1.0], clip(100, 1_100, 0, 10, audio))]);
        let left = left(&run(&mut engine, 1_500));
        assert_eq!(peak(&left[..100]), 0.0);
        assert_eq!(left[200..1_000], [0.5; 800]);
        assert_eq!(peak(&left[1_100..]), 0.0);
    }

    #[test]
    fn the_offset_chooses_the_first_frame_of_the_audio() {
        let ramp: Vec<f32> = (0..1_000_u16)
            .map(|index| f32::from(index) / 1_000.0)
            .collect();
        let mut engine = playing(vec![track(
            [1.0, 1.0],
            clip(0, 500, 300, 1, vec![ramp.clone()]),
        )]);
        let left = left(&run(&mut engine, 500));
        assert_eq!(left[50], ramp[350]);
    }

    #[test]
    fn a_clip_fades_in_and_out_over_its_fade_frames() {
        let mut engine = playing(vec![track(
            [1.0, 1.0],
            clip(0, 20, 0, 4, vec![vec![1.0; 20]]),
        )]);
        let left = left(&run(&mut engine, 20));
        assert_eq!(left[..5], [0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_eq!(left[15..], [1.0, 0.75, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn the_gains_of_a_track_set_the_level_of_each_channel() {
        let audio = vec![vec![1.0; 100], vec![1.0; 100]];
        let mut engine = playing(vec![track([0.5, 0.25], clip(0, 100, 0, 1, audio))]);
        let buffer = run(&mut engine, 100);
        assert_eq!(buffer[100..102], [0.5, 0.25]);
    }

    #[test]
    fn mono_audio_plays_on_the_left_and_the_right_channel() {
        let mut engine = playing(vec![track(
            [1.0, 1.0],
            clip(0, 100, 0, 1, vec![vec![0.5; 100]]),
        )]);
        let buffer = run(&mut engine, 100);
        assert_eq!(buffer[100..102], [0.5, 0.5]);
    }

    #[test]
    fn the_tracks_add_together() {
        let audio = || vec![vec![0.25; 100]];
        let mut engine = playing(vec![
            track([1.0, 1.0], clip(0, 100, 0, 1, audio())),
            track([1.0, 1.0], clip(0, 100, 0, 1, audio())),
        ]);
        assert_eq!(left(&run(&mut engine, 100))[50], 0.5);
    }

    #[test]
    fn a_mono_output_gets_the_average_of_the_left_and_the_right_channel() {
        let mut engine = playing(vec![track(
            [1.0, 0.0],
            clip(0, 100, 0, 1, vec![vec![1.0; 100]]),
        )]);
        assert_eq!(render(&mut engine, 100, 1)[50], 0.5);
    }

    #[test]
    fn only_the_first_two_channels_of_the_output_get_the_mix_and_the_click() {
        let (mut controller, mut engine) = engine_with(vec![track(
            [1.0, 1.0],
            clip(0, 4_800, 0, 1, vec![vec![0.5; 4_800]]),
        )]);
        controller.send(Command::SetMetronome(true));
        controller.send(Command::TogglePlay);
        let buffer = render(&mut engine, 4_800, 4);
        assert!(peak(&channel(&buffer, 4, 0)) > 0.1);
        assert!(peak(&channel(&buffer, 4, 1)) > 0.1);
        assert_eq!(peak(&channel(&buffer, 4, 2)), 0.0);
        assert_eq!(peak(&channel(&buffer, 4, 3)), 0.0);
    }

    fn metronome_on() -> (Controller, Engine) {
        let (mut controller, engine) = engine();
        controller.send(Command::SetMetronome(true));
        controller.send(Command::TogglePlay);
        (controller, engine)
    }

    fn busy() -> (Controller, Engine) {
        let audio = vec![vec![0.25; 30_000], vec![-0.25; 30_000]];
        let (mut controller, engine) =
            engine_with(vec![track([0.8, 0.6], clip(1_000, 31_000, 0, 192, audio))]);
        controller.send(Command::SetMetronome(true));
        controller.send(Command::TogglePlay);
        (controller, engine)
    }

    #[test]
    fn the_metronome_is_off_at_the_start() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::TogglePlay);
        assert_eq!(peak(&run(&mut engine, 48_000)), 0.0);
    }

    #[test]
    fn the_metronome_clicks_on_each_beat_while_it_is_on() {
        let (_controller, mut engine) = metronome_on();
        let left = left(&run(&mut engine, 48_000));
        assert!(peak(&left[..240]) > 0.1);
        assert_eq!(peak(&left[2_000..24_000]), 0.0);
        assert!(peak(&left[24_000..24_240]) > 0.1);
    }

    #[test]
    fn each_channel_gets_the_same_click() {
        let (_controller, mut engine) = metronome_on();
        let buffer = run(&mut engine, 4_800);
        let (frames, _) = buffer.as_chunks::<2>();
        assert!(
            frames
                .iter()
                .all(|[left, right]| left.to_bits() == right.to_bits())
        );
    }

    #[test]
    fn the_output_does_not_depend_on_the_buffer_size() {
        let (_one, mut whole) = busy();
        let (_other, mut pieces) = busy();
        let expected = run(&mut whole, 48_000);
        let actual: Vec<f32> = (0..480).flat_map(|_| run(&mut pieces, 100)).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn a_new_arrangement_plays_from_the_next_buffer() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::TogglePlay);
        let _ = run(&mut engine, 100);
        let audio = vec![vec![0.5; 1_000]];
        let arrangement = Arrangement::new(
            timing(),
            vec![track([1.0, 1.0], clip(0, 1_000, 0, 1, audio))],
        );
        assert!(controller.replace_arrangement(arrangement));
        assert_eq!(left(&run(&mut engine, 100))[50], 0.5);
    }

    #[test]
    fn the_engine_returns_the_old_arrangement_for_the_controller_to_release() {
        let audio = Arc::new(AudioBuffer::new(vec![vec![0.5; 100]]));
        let old = ClipPlayback {
            audio: Arc::clone(&audio),
            ..clip(0, 100, 0, 1, Vec::new())
        };
        let (mut controller, mut engine) = engine_with(vec![track([1.0, 1.0], old)]);
        controller.replace_arrangement(Arrangement::new(timing(), Vec::new()));
        let _ = run(&mut engine, 10);
        assert_eq!(Arc::strong_count(&audio), 2);
        controller.release_old();
        assert_eq!(Arc::strong_count(&audio), 1);
    }

    #[test]
    fn a_new_arrangement_waits_until_the_controller_releases_the_old_ones() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::TogglePlay);
        for _ in 0..ARRANGEMENT_CAPACITY {
            assert!(controller.replace_arrangement(Arrangement::new(timing(), Vec::new())));
        }
        let _ = run(&mut engine, 10);
        let audio = vec![vec![0.5; 1_000]];
        let with_clip = Arrangement::new(
            timing(),
            vec![track([1.0, 1.0], clip(0, 1_000, 0, 1, audio))],
        );
        assert!(controller.replace_arrangement(with_clip));
        assert_eq!(peak(&run(&mut engine, 100)), 0.0);
        controller.release_old();
        assert_eq!(left(&run(&mut engine, 100))[50], 0.5);
    }

    #[test]
    fn replacing_the_arrangement_never_allocates_memory_in_the_engine() {
        let (mut controller, mut engine) = busy();
        let mut buffer = vec![0.0; 1_024];
        controller.replace_arrangement(Arrangement::new(timing(), Vec::new()));
        assert_no_alloc(|| engine.process(&mut buffer, 2));
    }

    fn constant(value: f32, frames: usize) -> Arc<AudioBuffer> {
        Arc::new(AudioBuffer::new(vec![vec![value; frames]]))
    }

    #[test]
    fn a_preview_plays_while_the_transport_is_stopped() {
        let (mut controller, mut engine) = engine();
        assert!(controller.preview(constant(0.5, 1_000)));
        assert_eq!(left(&run(&mut engine, 100))[50], 0.5);
        assert!(controller.is_previewing());
    }

    #[test]
    fn a_preview_ends_with_its_audio() {
        let (mut controller, mut engine) = engine();
        controller.preview(constant(0.5, 100));
        let left = left(&run(&mut engine, 200));
        assert_eq!(left[99], 0.5);
        assert_eq!(peak(&left[100..]), 0.0);
        assert!(!controller.is_previewing());
    }

    #[test]
    fn the_controller_sets_the_preview_gain_and_stops_the_preview() {
        let (mut controller, mut engine) = engine();
        controller.send(Command::SetPreviewGain(0.5));
        controller.preview(constant(1.0, 10_000));
        assert_eq!(left(&run(&mut engine, 100))[50], 0.5);
        controller.send(Command::StopPreview);
        let left = left(&run(&mut engine, 1_000));
        assert!(left[0] > 0.0);
        assert_eq!(peak(&left[192..]), 0.0);
        assert!(!controller.is_previewing());
    }

    #[test]
    fn the_controller_counts_the_previews_that_the_engine_started() {
        let (mut controller, mut engine) = engine();
        controller.preview(constant(0.5, 10));
        controller.preview(constant(0.5, 10));
        assert_eq!(controller.previews_started(), 0);
        let _ = run(&mut engine, 100);
        assert_eq!(controller.previews_started(), 2);
        assert!(!controller.is_previewing());
    }

    #[test]
    fn the_engine_returns_old_previews_for_the_controller_to_release() {
        let (mut controller, mut engine) = engine();
        let first = constant(0.5, 1_000);
        controller.preview(Arc::clone(&first));
        let _ = run(&mut engine, 10);
        controller.preview(constant(0.25, 1_000));
        let _ = run(&mut engine, 10);
        assert_eq!(Arc::strong_count(&first), 2);
        controller.release_old();
        assert_eq!(Arc::strong_count(&first), 1);
    }

    #[test]
    fn previews_never_allocate_memory_in_the_engine() {
        let (mut controller, mut engine) = busy();
        let mut buffer = vec![0.0; 1_024];
        controller.preview(constant(0.5, 100_000));
        assert_no_alloc(|| engine.process(&mut buffer, 2));
        controller.send(Command::StopPreview);
        controller.preview(constant(0.25, 100_000));
        assert_no_alloc(|| engine.process(&mut buffer, 2));
    }

    #[test]
    fn filling_buffers_never_allocates_memory() {
        let (mut controller, mut engine) = busy();
        let mut buffer = vec![0.0; 1_024];
        assert_no_alloc(|| {
            for _ in 0..500 {
                engine.process(&mut buffer, 2);
            }
        });
        controller.send(Command::TogglePlay);
        assert_no_alloc(|| engine.process(&mut buffer, 2));
    }
}
