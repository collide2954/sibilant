use std::fmt;

use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{Fft, FixedSync, Resampler};

use crate::Audio;

const CHUNK_FRAMES: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertError(String);

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot convert the sample rate: {}", self.0)
    }
}

impl std::error::Error for ConvertError {}

pub fn convert(audio: Audio, sample_rate: u32) -> Result<Audio, ConvertError> {
    if audio.sample_rate() == sample_rate || audio.frames() == 0 {
        return Ok(Audio::new(sample_rate, audio.into_channels()));
    }
    let (count, frames) = (audio.channels().len(), audio.frames());
    let input = SequentialSliceOfVecs::new(audio.channels(), count, frames).map_err(failure)?;
    let mut resampler = Fft::<f32>::new(
        audio.sample_rate() as usize,
        sample_rate as usize,
        CHUNK_FRAMES,
        count,
        FixedSync::Both,
    )
    .map_err(failure)?;
    let interleaved = resampler
        .process_all(&input, frames, None)
        .map_err(failure)?
        .take_data();
    let length = converted_length(frames, audio.sample_rate(), sample_rate);
    let channels = (0..count)
        .map(|channel| {
            let mut samples: Vec<f32> = interleaved
                .iter()
                .skip(channel)
                .step_by(count)
                .copied()
                .collect();
            samples.resize(length, 0.0);
            samples
        })
        .collect();
    Ok(Audio::new(sample_rate, channels))
}

fn converted_length(frames: usize, from: u32, to: u32) -> usize {
    let scaled = (frames as u128 * u128::from(to) + u128::from(from) / 2) / u128::from(from);
    usize::try_from(scaled).unwrap_or(usize::MAX)
}

fn failure(error: impl fmt::Display) -> ConvertError {
    ConvertError(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::f64::consts::TAU;

    use crate::{Audio, convert};

    #[expect(clippy::cast_possible_truncation, reason = "a test signal fits in f32")]
    fn sine(frequency: f64, sample_rate: u32, frames: u32) -> Vec<f32> {
        (0..frames)
            .map(|frame| (TAU * frequency * f64::from(frame) / f64::from(sample_rate)).sin() as f32)
            .collect()
    }

    fn largest_difference(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b)
            .fold(0.0, |largest, (a, b)| largest.max((a - b).abs()))
    }

    #[test]
    fn audio_at_the_same_rate_does_not_change() {
        let audio = Audio::new(48_000, vec![sine(440.0, 48_000, 1_000)]);
        assert_eq!(convert(audio.clone(), 48_000), Ok(audio));
    }

    #[test]
    fn the_converted_audio_lasts_as_long_as_the_original() {
        let audio = Audio::new(44_100, vec![vec![0.0; 44_100], vec![0.0; 44_100]]);
        let converted = convert(audio, 48_000).unwrap();
        assert_eq!(converted.sample_rate(), 48_000);
        assert_eq!(converted.frames(), 48_000);
        assert_eq!(converted.channels().len(), 2);
    }

    fn difference_from_tone(samples: &[f32], first_frame: u32, offset: f64) -> f64 {
        samples
            .iter()
            .zip(first_frame..)
            .map(|(&sample, frame)| {
                let time = (f64::from(frame) + offset) / 48_000.0;
                (f64::from(sample) - (TAU * 1_000.0 * time).sin()).abs()
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn a_converted_tone_keeps_its_frequency_and_level_and_moves_less_than_a_sample() {
        let audio = Audio::new(44_100, vec![sine(1_000.0, 44_100, 44_100)]);
        let converted = convert(audio, 48_000).unwrap();
        let window = &converted.channels()[0][24_000..25_000];
        let closest = (-500..=500)
            .map(|step| difference_from_tone(window, 24_000, f64::from(step) / 500.0))
            .fold(f64::MAX, f64::min);
        assert!(closest < 1e-3, "difference {closest}");
    }

    #[test]
    fn each_channel_keeps_its_own_signal() {
        let tone = sine(1_000.0, 44_100, 44_100);
        let audio = Audio::new(44_100, vec![tone, vec![0.0; 44_100]]);
        let converted = convert(audio, 48_000).unwrap();
        let [left, right] = converted.channels() else {
            panic!("two channels")
        };
        assert!(largest_difference(left, &vec![0.0; left.len()]) > 0.9);
        assert!(largest_difference(right, &vec![0.0; right.len()]) < 1e-6);
    }
}
