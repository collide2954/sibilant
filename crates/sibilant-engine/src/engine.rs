use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use rtrb::{Consumer, Producer, RingBuffer};
use sibilant_core::time::{Musical, Position};

use crate::timing::Timing;

const QUEUE_CAPACITY: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Continue,
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
    let engine = Engine {
        timing,
        commands: consumer,
        shared,
        playing: false,
        marker,
        stopped_at: start,
        playhead: start,
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
        if self.playing {
            self.playhead += frames_in(output, channels);
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
