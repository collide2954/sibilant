use std::path::{Path, PathBuf};
use std::sync::Arc;

use sibilant_core::time::{Clock, Duration};
use sibilant_engine::arrangement::AudioBuffer;
use sibilant_media::Peaks;

#[derive(Debug, Clone)]
pub(crate) struct LoadedAudio {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) length: Duration<Clock>,
    pub(crate) playback: Arc<AudioBuffer>,
    pub(crate) peaks: Arc<Peaks>,
}

pub(crate) fn load(path: &Path, sample_rate: Option<u32>) -> Result<LoadedAudio, String> {
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let failure = |error: &dyn std::error::Error| format!("{file_name}: {error}");
    let audio = sibilant_media::read(path).map_err(|error| failure(&error))?;
    let length = audio.length();
    let audio = match sample_rate {
        Some(rate) => sibilant_media::convert(audio, rate).map_err(|error| failure(&error))?,
        None => audio,
    };
    let peaks = Arc::new(Peaks::of(&audio));
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    Ok(LoadedAudio {
        path: path.to_path_buf(),
        name,
        length,
        playback: Arc::new(AudioBuffer::new(audio.into_channels())),
        peaks,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::{Path, PathBuf};

    use hound::{SampleFormat, WavSpec, WavWriter};
    use sibilant_core::time::Duration;

    use crate::library::load;

    pub(crate) fn write_tone(name: &str, sample_rate: u32, frames: usize) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sibilant-library-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&path, spec).unwrap();
        for _ in 0..frames {
            writer.write_sample(16_384_i16).unwrap();
        }
        writer.finalize().unwrap();
        path
    }

    #[test]
    fn a_loaded_file_has_its_name_its_length_and_audio_at_the_output_rate() {
        let path = write_tone("tone.wav", 44_100, 44_100);
        let loaded = load(&path, Some(48_000)).unwrap();
        assert_eq!(loaded.name, "tone");
        assert_eq!(loaded.path, path);
        assert_eq!(loaded.length, Duration::from_seconds(1));
        assert_eq!(loaded.playback.frames(), 48_000);
        assert_eq!(loaded.peaks.sample_rate(), 48_000);
    }

    #[test]
    fn without_an_output_the_audio_keeps_the_rate_of_its_file() {
        let path = write_tone("slow.wav", 22_050, 2_205);
        let loaded = load(&path, None).unwrap();
        assert_eq!(loaded.playback.frames(), 2_205);
        assert_eq!(loaded.peaks.sample_rate(), 22_050);
    }

    #[test]
    fn a_file_that_cannot_be_read_gives_a_message_with_its_name() {
        let message = load(Path::new("missing.wav"), None).unwrap_err();
        assert!(message.starts_with("missing.wav: "), "{message}");
    }
}
