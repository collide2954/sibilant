use crate::arrangement::Arrangement;
use crate::engine::{self, Command};

const CHANNELS: usize = 2;

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

    use sibilant_core::time::{TempoMap, TimeSignatureMap};

    use crate::arrangement::{Arrangement, AudioBuffer, ClipPlayback, TrackPlayback};
    use crate::render::render;
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
