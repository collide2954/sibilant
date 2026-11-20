use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use rtrb::{Consumer, Producer, RingBuffer};
use sibilant_core::time::{Musical, Position};

use crate::click::ClickVoice;
use crate::timing::Timing;

const QUEUE_CAPACITY: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Continue,
    SetMetronome(bool),
    TogglePlay,
}

#[derive(Debug, Default)]
struct Shared {
    playing: AtomicBool,
    playhead: AtomicI64,
}

#[derive(Debug)]
pub struct Controller {
    commands: Producer<Command>,
    shared: Arc<Shared>,
}

#[derive(Debug)]
pub struct Engine {
    timing: Timing,
    commands: Consumer<Command>,
    shared: Arc<Shared>,
    playing: bool,
    marker: Position<Musical>,
    stopped_at: i64,
    playhead: i64,
    metronome: bool,
    click: ClickVoice,
}

pub fn new(timing: Timing) -> (Controller, Engine) {
    let (producer, consumer) = RingBuffer::new(QUEUE_CAPACITY);
    let shared = Arc::new(Shared::default());
    let marker = Position::START;
    let start = timing.frame_of(marker);
    shared.playhead.store(start, Ordering::Relaxed);
    let controller = Controller {
        commands: producer,
        shared: Arc::clone(&shared),
    };
    let click = ClickVoice::new(timing.sample_rate());
    let engine = Engine {
        timing,
        commands: consumer,
        shared,
        playing: false,
        marker,
        stopped_at: start,
        playhead: start,
        metronome: false,
        click,
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
}

impl Engine {
    pub fn process(&mut self, output: &mut [f32], channels: usize) {
        while let Ok(command) = self.commands.pop() {
            self.apply(command);
        }
        output.fill(0.0);
        let start = self.playhead;
        let end = start + frames_in(output, channels);
        let mut clicks = (self.playing && self.metronome).then(|| self.timing.clicks(start, end));
        let mut next_click = clicks.as_mut().and_then(Iterator::next);
        for (frame, samples) in (start..).zip(output.chunks_exact_mut(channels)) {
            while let Some(click) = next_click.filter(|click| click.frame == frame) {
                self.click.trigger(click.accent);
                next_click = clicks.as_mut().and_then(Iterator::next);
            }
            samples.fill(self.click.next_sample());
        }
        if self.playing {
            self.playhead = end;
        }
        self.shared.playing.store(self.playing, Ordering::Relaxed);
        self.shared.playhead.store(self.playhead, Ordering::Relaxed);
    }

    fn apply(&mut self, command: Command) {
        match command {
            Command::TogglePlay if self.playing => {
                self.stopped_at = self.playhead;
                self.playhead = self.timing.frame_of(self.marker);
                self.playing = false;
            }
            Command::TogglePlay => {
                self.playhead = self.timing.frame_of(self.marker);
                self.playing = true;
            }
            Command::Continue if !self.playing => {
                self.playhead = self.stopped_at;
                self.playing = true;
            }
            Command::Continue => {}
            Command::SetMetronome(on) => self.metronome = on,
        }
    }
}

fn frames_in(output: &[f32], channels: usize) -> i64 {
    i64::try_from(output.len() / channels).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use sibilant_core::time::{TempoMap, TimeSignatureMap};

    use crate::engine::{self, Command, Controller, Engine};
    use crate::timing::Timing;

    fn engine() -> (Controller, Engine) {
        engine::new(Timing::new(
            48_000,
            TempoMap::default(),
            TimeSignatureMap::default(),
        ))
    }

    fn run(engine: &mut Engine, frames: usize) -> Vec<f32> {
        let mut buffer = vec![1.0; frames * 2];
        engine.process(&mut buffer, 2);
        buffer
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
        let (_controller, mut engine) = engine();
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

    fn left(buffer: &[f32]) -> Vec<f32> {
        buffer.iter().step_by(2).copied().collect()
    }

    fn peak(samples: &[f32]) -> f32 {
        samples
            .iter()
            .fold(0.0, |peak, sample| peak.max(sample.abs()))
    }

    fn metronome_on() -> (Controller, Engine) {
        let (mut controller, engine) = engine();
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
        let (_one, mut whole) = metronome_on();
        let (_other, mut pieces) = metronome_on();
        let expected = run(&mut whole, 48_000);
        let actual: Vec<f32> = (0..480).flat_map(|_| run(&mut pieces, 100)).collect();
        assert_eq!(actual, expected);
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
}
