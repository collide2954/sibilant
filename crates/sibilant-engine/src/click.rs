use std::f64::consts::TAU;

const ACCENT_FREQUENCY: f64 = 1_500.0;
const BEAT_FREQUENCY: f64 = 1_000.0;
const LEVEL: f64 = 0.5;
const END_LEVEL: f64 = 0.001;
const LENGTH_PER_100_SECONDS: u32 = 3;

#[derive(Debug, Clone, Copy)]
pub struct ClickVoice {
    sample_rate: f64,
    length: u32,
    remaining: u32,
    phase: f64,
    phase_step: f64,
    gain: f64,
    decay: f64,
}

impl ClickVoice {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: f64::from(sample_rate),
            length: sample_rate * LENGTH_PER_100_SECONDS / 100,
            remaining: 0,
            phase: 0.0,
            phase_step: 0.0,
            gain: 0.0,
            decay: 0.0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.remaining > 0
    }

    pub fn trigger(&mut self, accent: bool) {
        let frequency = if accent {
            ACCENT_FREQUENCY
        } else {
            BEAT_FREQUENCY
        };
        self.remaining = self.length;
        self.phase = 0.0;
        self.phase_step = TAU * frequency / self.sample_rate;
        self.gain = LEVEL;
        self.decay = END_LEVEL.powf(1.0 / f64::from(self.length));
    }

    pub fn next_sample(&mut self) -> f32 {
        if self.remaining == 0 {
            return 0.0;
        }
        let sample = self.phase.sin() * self.gain;
        self.phase = (self.phase + self.phase_step) % TAU;
        self.gain *= self.decay;
        self.remaining -= 1;
        to_f32(sample)
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "audio samples are f32 by design (record 0009)"
)]
fn to_f32(sample: f64) -> f32 {
    sample as f32
}

#[cfg(test)]
mod tests {
    use crate::click::ClickVoice;

    const RATE: u32 = 48_000;

    fn render(voice: &mut ClickVoice, frames: usize) -> Vec<f32> {
        (0..frames).map(|_| voice.next_sample()).collect()
    }

    fn zero_crossings(samples: &[f32]) -> usize {
        samples
            .windows(2)
            .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
            .count()
    }

    fn peak(samples: &[f32]) -> f32 {
        samples
            .iter()
            .fold(0.0, |peak, sample| peak.max(sample.abs()))
    }

    #[test]
    fn an_idle_voice_is_silent() {
        let mut voice = ClickVoice::new(RATE);
        assert!(peak(&render(&mut voice, 1_000)) < f32::EPSILON);
        assert!(!voice.is_active());
    }

    #[test]
    fn a_click_sounds_and_ends_within_30_ms() {
        let mut voice = ClickVoice::new(RATE);
        voice.trigger(false);
        let click = render(&mut voice, 1_440);
        assert!(peak(&click[..240]) > 0.1);
        assert!(!voice.is_active());
        assert!(peak(&render(&mut voice, 1_000)) < f32::EPSILON);
    }

    #[test]
    fn a_click_starts_without_a_jump() {
        let mut voice = ClickVoice::new(RATE);
        voice.trigger(true);
        assert!(voice.next_sample().abs() < 0.01);
    }

    #[test]
    fn the_accent_has_a_higher_tone() {
        let mut accent = ClickVoice::new(RATE);
        let mut beat = ClickVoice::new(RATE);
        accent.trigger(true);
        beat.trigger(false);
        let accent_crossings = zero_crossings(&render(&mut accent, 480));
        let beat_crossings = zero_crossings(&render(&mut beat, 480));
        assert!(
            accent_crossings > beat_crossings,
            "{accent_crossings} <= {beat_crossings}"
        );
    }

    #[test]
    fn a_new_click_restarts_the_sound() {
        let mut voice = ClickVoice::new(RATE);
        voice.trigger(false);
        let _ = render(&mut voice, 1_000);
        voice.trigger(false);
        assert!(peak(&render(&mut voice, 240)) > 0.1);
    }
}
