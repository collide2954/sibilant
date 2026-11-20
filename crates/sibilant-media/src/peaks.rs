use std::ops::Range;

use crate::Audio;

const FRAMES_PER_PEAK: usize = 32;
const PEAKS_PER_COARSER_PEAK: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peak {
    pub min: f32,
    pub max: f32,
}

impl Peak {
    const NONE: Self = Self {
        min: f32::INFINITY,
        max: f32::NEG_INFINITY,
    };

    fn with(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    fn of_sample(sample: f32) -> Self {
        Self {
            min: sample,
            max: sample,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Peaks {
    frames: usize,
    levels: Vec<Level>,
}

#[derive(Debug, Clone, PartialEq)]
struct Level {
    frames_per_peak: usize,
    peaks: Vec<Peak>,
}

impl Peaks {
    pub fn of(audio: &Audio) -> Self {
        let mut levels = vec![Level::finest(audio)];
        while let Some(coarser) = levels
            .last()
            .filter(|level| level.peaks.len() > 1)
            .map(Level::coarser)
        {
            levels.push(coarser);
        }
        Self {
            frames: audio.frames(),
            levels,
        }
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn range(&self, frames: Range<usize>) -> Option<Peak> {
        let end = frames.end.min(self.frames);
        if frames.start >= end {
            return None;
        }
        let level = self
            .levels
            .iter()
            .rev()
            .find(|level| level.frames_per_peak <= end - frames.start)
            .unwrap_or(&self.levels[0]);
        let first = frames.start / level.frames_per_peak;
        let last = (end - 1) / level.frames_per_peak;
        Some(combined(&level.peaks[first..=last]))
    }
}

impl Level {
    fn finest(audio: &Audio) -> Self {
        let frames = audio.frames();
        let peaks = (0..frames.div_ceil(FRAMES_PER_PEAK))
            .map(|block| {
                let start = block * FRAMES_PER_PEAK;
                let end = (start + FRAMES_PER_PEAK).min(frames);
                audio
                    .channels()
                    .iter()
                    .flat_map(|channel| &channel[start..end])
                    .fold(Peak::NONE, |peak, &sample| {
                        peak.with(Peak::of_sample(sample))
                    })
            })
            .collect();
        Self {
            frames_per_peak: FRAMES_PER_PEAK,
            peaks,
        }
    }

    fn coarser(&self) -> Self {
        Self {
            frames_per_peak: self.frames_per_peak * PEAKS_PER_COARSER_PEAK,
            peaks: self
                .peaks
                .chunks(PEAKS_PER_COARSER_PEAK)
                .map(combined)
                .collect(),
        }
    }
}

fn combined(peaks: &[Peak]) -> Peak {
    peaks
        .iter()
        .fold(Peak::NONE, |peak, &other| peak.with(other))
}

#[cfg(test)]
mod tests {
    use crate::{Audio, Peak, Peaks};

    fn peaks(channels: Vec<Vec<f32>>) -> Peaks {
        Peaks::of(&Audio::new(48_000, channels))
    }

    fn with_spike(frames: usize, at: usize, value: f32) -> Vec<f32> {
        let mut samples = vec![0.0; frames];
        samples[at] = value;
        samples
    }

    #[test]
    fn a_peak_holds_the_lowest_and_the_highest_sample_of_its_frames() {
        let peaks = peaks(vec![vec![0.1, -0.4, 0.3, 0.2]]);
        assert_eq!(
            peaks.range(0..4),
            Some(Peak {
                min: -0.4,
                max: 0.3
            })
        );
    }

    #[test]
    fn a_peak_covers_all_channels() {
        let peaks = peaks(vec![vec![0.5, 0.0], vec![0.0, -0.8]]);
        assert_eq!(
            peaks.range(0..2),
            Some(Peak {
                min: -0.8,
                max: 0.5
            })
        );
    }

    #[test]
    fn long_ranges_keep_a_single_short_spike() {
        let peaks = peaks(vec![with_spike(1_000_000, 777_777, 0.9)]);
        assert_eq!(peaks.range(0..1_000_000), Some(Peak { min: 0.0, max: 0.9 }));
        assert_eq!(
            peaks.range(500_000..1_000_000),
            Some(Peak { min: 0.0, max: 0.9 })
        );
    }

    #[test]
    fn a_range_shows_only_the_blocks_that_it_touches() {
        let peaks = peaks(vec![with_spike(1_000_000, 777_777, 0.9)]);
        assert_eq!(peaks.range(0..500_000), Some(Peak { min: 0.0, max: 0.0 }));
    }

    #[test]
    fn a_range_that_ends_after_the_audio_uses_the_frames_that_exist() {
        let peaks = peaks(vec![vec![0.25; 100]]);
        assert_eq!(
            peaks.range(90..200),
            Some(Peak {
                min: 0.25,
                max: 0.25
            })
        );
    }

    #[test]
    fn a_range_outside_the_audio_or_without_frames_has_no_peak() {
        let peaks = peaks(vec![vec![0.25; 100]]);
        assert_eq!(peaks.range(100..200), None);
        assert_eq!(peaks.range(50..50), None);
    }

    #[test]
    fn the_peaks_know_the_length_of_their_audio() {
        assert_eq!(peaks(vec![vec![0.0; 1_234]]).frames(), 1_234);
    }
}
