use sibilant_core::time::{Clock, Duration};

#[derive(Debug, Clone, PartialEq)]
pub struct Audio {
    sample_rate: u32,
    channels: Vec<Vec<f32>>,
}

impl Audio {
    pub fn new(sample_rate: u32, channels: Vec<Vec<f32>>) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> &[Vec<f32>] {
        &self.channels
    }

    pub fn frames(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }

    pub fn length(&self) -> Duration<Clock> {
        let frames = i64::try_from(self.frames()).unwrap_or(i64::MAX);
        Duration::from_frames(frames, self.sample_rate)
    }

    pub fn into_channels(self) -> Vec<Vec<f32>> {
        self.channels
    }
}

#[cfg(test)]
mod tests {
    use sibilant_core::time::Duration;

    use crate::Audio;

    #[test]
    fn the_frames_count_the_samples_of_one_channel() {
        let audio = Audio::new(48_000, vec![vec![0.0; 300], vec![0.0; 300]]);
        assert_eq!(audio.frames(), 300);
    }

    #[test]
    fn the_length_follows_the_frames_and_the_sample_rate() {
        let audio = Audio::new(44_100, vec![vec![0.0; 88_200]]);
        assert_eq!(audio.length(), Duration::from_seconds(2));
    }
}
