use std::fmt;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

use hound::{SampleFormat, WavReader};

use crate::Audio;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    File(String),
    Format(String),
    Channels(u16),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(message) => write!(f, "cannot open the file: {message}"),
            Self::Format(message) => write!(f, "cannot read the file as WAV: {message}"),
            Self::Channels(count) => write!(
                f,
                "Sibilant reads mono and stereo files, but this file has {count} channels"
            ),
        }
    }
}

impl std::error::Error for ReadError {}

impl From<hound::Error> for ReadError {
    fn from(error: hound::Error) -> Self {
        match error {
            hound::Error::IoError(error) => Self::File(error.to_string()),
            error => Self::Format(error.to_string()),
        }
    }
}

pub fn can_read(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

pub fn read(path: &Path) -> Result<Audio, ReadError> {
    let file = File::open(path).map_err(|error| ReadError::File(error.to_string()))?;
    read_wav(BufReader::new(file))
}

fn read_wav(reader: impl io::Read) -> Result<Audio, ReadError> {
    let reader = WavReader::new(reader)?;
    let spec = reader.spec();
    if !(1..=2).contains(&spec.channels) {
        return Err(ReadError::Channels(spec.channels));
    }
    if spec.sample_rate == 0 {
        return Err(ReadError::Format("the sample rate is 0 Hz".into()));
    }
    let count = usize::from(spec.channels);
    let frames = usize::try_from(reader.duration()).unwrap_or(0);
    let channels = match spec.sample_format {
        SampleFormat::Float => deinterleave(count, frames, reader.into_samples::<f32>())?,
        SampleFormat::Int => {
            let full_scale = 2_f32.powi(i32::from(spec.bits_per_sample) - 1);
            let samples = reader
                .into_samples::<i32>()
                .map(|sample| sample.map(|sample| to_f32(sample) / full_scale));
            deinterleave(count, frames, samples)?
        }
    };
    Ok(Audio::new(spec.sample_rate, channels))
}

fn deinterleave(
    count: usize,
    frames: usize,
    samples: impl Iterator<Item = hound::Result<f32>>,
) -> Result<Vec<Vec<f32>>, ReadError> {
    let mut channels = vec![Vec::with_capacity(frames); count];
    for (index, sample) in samples.enumerate() {
        channels[index % count].push(sample?);
    }
    Ok(channels)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "32-bit float audio keeps 24 bits of each sample (record 0009)"
)]
fn to_f32(sample: i32) -> f32 {
    sample as f32
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;
    use std::path::Path;

    use hound::{SampleFormat, WavSpec, WavWriter};

    use super::read_wav;
    use crate::{ReadError, can_read, read};

    fn spec(channels: u16, bits_per_sample: u16, sample_format: SampleFormat) -> WavSpec {
        WavSpec {
            channels,
            sample_rate: 44_100,
            bits_per_sample,
            sample_format,
        }
    }

    fn wav<S: hound::Sample + Copy>(spec: WavSpec, samples: &[S]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = WavWriter::new(&mut cursor, spec).unwrap();
        for &sample in samples {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        cursor.into_inner()
    }

    fn mono(bytes: Vec<u8>) -> Vec<f32> {
        read_wav(Cursor::new(bytes))
            .unwrap()
            .into_channels()
            .remove(0)
    }

    #[test]
    fn integer_samples_become_floats_from_minus_1_to_1() {
        let bytes = wav(spec(1, 16, SampleFormat::Int), &[0_i16, 16_384, -32_768]);
        assert_eq!(mono(bytes), [0.0, 0.5, -1.0]);
    }

    #[test]
    fn reads_8_24_and_32_bit_integer_samples() {
        let eight = wav(spec(1, 8, SampleFormat::Int), &[64_i8, -128]);
        let twenty_four = wav(spec(1, 24, SampleFormat::Int), &[4_194_304_i32, -8_388_608]);
        let thirty_two = wav(
            spec(1, 32, SampleFormat::Int),
            &[1_073_741_824_i32, i32::MIN],
        );
        for bytes in [eight, twenty_four, thirty_two] {
            assert_eq!(mono(bytes), [0.5, -1.0]);
        }
    }

    #[test]
    fn float_samples_stay_as_they_are() {
        let bytes = wav(spec(1, 32, SampleFormat::Float), &[0.25_f32, -0.75, 1.5]);
        assert_eq!(mono(bytes), [0.25, -0.75, 1.5]);
    }

    #[test]
    fn stereo_frames_split_into_a_left_and_a_right_channel() {
        let bytes = wav(
            spec(2, 16, SampleFormat::Int),
            &[16_384_i16, -16_384, 0, 8_192],
        );
        let audio = read_wav(Cursor::new(bytes)).unwrap();
        assert_eq!(audio.channels(), [vec![0.5, 0.0], vec![-0.5, 0.25]]);
    }

    #[test]
    fn the_audio_keeps_the_sample_rate_of_the_file() {
        let bytes = wav(spec(1, 16, SampleFormat::Int), &[0_i16]);
        assert_eq!(read_wav(Cursor::new(bytes)).unwrap().sample_rate(), 44_100);
    }

    #[test]
    fn files_with_more_than_two_channels_are_refused() {
        let bytes = wav(spec(3, 16, SampleFormat::Int), &[0_i16, 0, 0]);
        assert_eq!(read_wav(Cursor::new(bytes)), Err(ReadError::Channels(3)));
    }

    #[test]
    fn files_without_a_sample_rate_are_refused() {
        let mut bytes = wav(spec(1, 16, SampleFormat::Int), &[0_i16]);
        bytes[24..32].fill(0);
        let result = read_wav(Cursor::new(bytes));
        assert!(matches!(result, Err(ReadError::Format(_))));
    }

    #[test]
    fn data_that_is_not_a_wav_file_is_refused() {
        let result = read_wav(Cursor::new(b"not a wav file".to_vec()));
        assert!(matches!(result, Err(ReadError::Format(_))));
    }

    #[test]
    fn reads_a_wav_file_from_its_path() {
        let path = std::env::temp_dir().join(format!("sibilant-read-{}.wav", std::process::id()));
        fs::write(&path, wav(spec(1, 16, SampleFormat::Int), &[16_384_i16])).unwrap();
        let audio = read(&path);
        fs::remove_file(&path).unwrap();
        assert_eq!(audio.unwrap().channels(), [vec![0.5]]);
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let result = read(Path::new("there-is-no-such-file.wav"));
        assert!(matches!(result, Err(ReadError::File(_))));
    }

    #[test]
    fn only_wav_files_can_be_read() {
        assert!(can_read(Path::new("kick.wav")));
        assert!(can_read(Path::new("KICK.WAV")));
        assert!(!can_read(Path::new("kick.mp3")));
        assert!(!can_read(Path::new("wav")));
    }

    #[test]
    fn the_errors_tell_what_is_wrong() {
        assert_eq!(
            ReadError::Channels(6).to_string(),
            "Sibilant reads mono and stereo files, but this file has 6 channels"
        );
        assert_eq!(
            ReadError::File("access is denied".into()).to_string(),
            "cannot open the file: access is denied"
        );
        assert_eq!(
            ReadError::Format("no RIFF tag found".into()).to_string(),
            "cannot read the file as WAV: no RIFF tag found"
        );
    }
}
