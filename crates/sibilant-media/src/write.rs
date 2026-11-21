use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};

use hound::{SampleFormat, WavSpec, WavWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    Int16,
    Int24,
    Float32,
}

impl Depth {
    const fn bits(self) -> u16 {
        match self {
            Self::Int16 => 16,
            Self::Int24 => 24,
            Self::Float32 => 32,
        }
    }

    const fn format(self) -> SampleFormat {
        match self {
            Self::Int16 | Self::Int24 => SampleFormat::Int,
            Self::Float32 => SampleFormat::Float,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Written {
    pub peak: f32,
    pub clipped: bool,
}

pub struct WavFile {
    writer: WavWriter<BufWriter<File>>,
    depth: Depth,
    noise: Noise,
    peak: f32,
    temporary: PathBuf,
    target: PathBuf,
}

impl WavFile {
    pub fn create(target: &Path, sample_rate: u32, depth: Depth) -> io::Result<Self> {
        let name = target.file_name().unwrap_or_default().to_string_lossy();
        let temporary = target.with_file_name(format!(".{name}.tmp"));
        let spec = WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: depth.bits(),
            sample_format: depth.format(),
        };
        let writer = WavWriter::create(&temporary, spec).map_err(io::Error::other)?;
        Ok(Self {
            writer,
            depth,
            noise: Noise(0x9E37_79B9),
            peak: 0.0,
            temporary,
            target: target.to_path_buf(),
        })
    }

    pub fn write(&mut self, interleaved: &[f32]) -> io::Result<()> {
        for &sample in interleaved {
            self.peak = self.peak.max(sample.abs());
            match self.depth {
                Depth::Int16 => {
                    let dither = self.noise.triangular();
                    self.writer.write_sample(integer(sample, 32_768.0, dither))
                }
                Depth::Int24 => self.writer.write_sample(integer(sample, 8_388_608.0, 0.0)),
                Depth::Float32 => self.writer.write_sample(sample),
            }
            .map_err(io::Error::other)?;
        }
        Ok(())
    }

    pub fn finish(self) -> io::Result<Written> {
        let Self {
            writer,
            depth,
            peak,
            temporary,
            target,
            ..
        } = self;
        let finished = writer
            .finalize()
            .map_err(io::Error::other)
            .and_then(|()| OpenOptions::new().write(true).open(&temporary)?.sync_all())
            .and_then(|()| fs::rename(&temporary, &target));
        if finished.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        finished.map(|()| Written {
            peak,
            clipped: depth != Depth::Float32 && peak >= 1.0,
        })
    }

    pub fn discard(self) {
        drop(self.writer);
        let _ = fs::remove_file(&self.temporary);
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is a whole number in the range of the format"
)]
fn integer(sample: f32, full_scale: f32, dither: f32) -> i32 {
    (sample * full_scale + dither)
        .round()
        .clamp(-full_scale, full_scale - 1.0) as i32
}

struct Noise(u32);

impl Noise {
    fn triangular(&mut self) -> f32 {
        self.unit() - self.unit()
    }

    fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        f32::from_bits(0x3F80_0000 | (self.0 >> 9)) - 1.0
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::{env, fs, process};

    use hound::{SampleFormat, WavReader};

    use crate::{Depth, WavFile, Written};

    struct Folder(PathBuf);

    impl Folder {
        fn new() -> Self {
            static FOLDERS: AtomicUsize = AtomicUsize::new(0);
            let path = env::temp_dir().join(format!(
                "sibilant-write-{}-{}",
                process::id(),
                FOLDERS.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn file(&self) -> PathBuf {
            self.0.join("Song.wav")
        }

        fn names(&self) -> Vec<String> {
            fs::read_dir(&self.0)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn written(folder: &Folder, depth: Depth, samples: &[f32]) -> Written {
        let mut file = WavFile::create(&folder.file(), 48_000, depth).unwrap();
        file.write(samples).unwrap();
        file.finish().unwrap()
    }

    fn spec(folder: &Folder) -> (u16, u32, u16, SampleFormat) {
        let spec = WavReader::open(folder.file()).unwrap().spec();
        let hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample,
            sample_format,
        } = spec;
        (channels, sample_rate, bits_per_sample, sample_format)
    }

    fn integers(folder: &Folder) -> Vec<i32> {
        let mut reader = WavReader::open(folder.file()).unwrap();
        reader.samples::<i32>().map(Result::unwrap).collect()
    }

    #[test]
    fn a_24_bit_file_keeps_the_samples_of_a_24_bit_source() {
        let folder = Folder::new();
        let mut file = WavFile::create(&folder.file(), 96_000, Depth::Int24).unwrap();
        file.write(&[8_388_607.0 / 8_388_608.0, -1.0, 0.5, -0.25])
            .unwrap();
        file.finish().unwrap();
        assert_eq!(spec(&folder), (2, 96_000, 24, SampleFormat::Int));
        assert_eq!(
            integers(&folder),
            [8_388_607, -8_388_608, 4_194_304, -2_097_152]
        );
    }

    #[test]
    fn a_float_file_keeps_values_above_full_scale() {
        let folder = Folder::new();
        let result = written(&folder, Depth::Float32, &[1.5, -0.25]);
        assert_eq!(spec(&folder), (2, 48_000, 32, SampleFormat::Float));
        let mut reader = WavReader::open(folder.file()).unwrap();
        let samples: Vec<f32> = reader.samples::<f32>().map(Result::unwrap).collect();
        assert_eq!(samples, [1.5, -0.25]);
        assert_eq!(
            result,
            Written {
                peak: 1.5,
                clipped: false
            }
        );
    }

    #[test]
    fn an_integer_file_limits_samples_above_full_scale_and_tells_that_it_clipped() {
        let folder = Folder::new();
        assert!(written(&folder, Depth::Int24, &[1.5, -1.5]).clipped);
        assert_eq!(integers(&folder), [8_388_607, -8_388_608]);
        assert!(written(&folder, Depth::Int16, &[1.5, -1.5]).clipped);
        assert_eq!(integers(&folder), [32_767, -32_768]);
    }

    #[test]
    fn a_sample_at_full_scale_is_a_clip_as_on_the_meters() {
        let folder = Folder::new();
        assert!(!written(&folder, Depth::Int24, &[0.999, -0.999]).clipped);
        assert!(written(&folder, Depth::Int24, &[0.25, -1.0]).clipped);
    }

    #[test]
    fn the_peak_is_the_largest_sample_of_all_the_blocks() {
        let folder = Folder::new();
        let mut file = WavFile::create(&folder.file(), 48_000, Depth::Int24).unwrap();
        file.write(&[0.25, -0.75]).unwrap();
        file.write(&[0.5, 0.0]).unwrap();
        assert_eq!(
            file.finish().unwrap(),
            Written {
                peak: 0.75,
                clipped: false
            }
        );
    }

    #[test]
    fn a_16_bit_file_keeps_the_level_of_its_samples() {
        let folder = Folder::new();
        let _ = written(&folder, Depth::Int16, &[0.5; 1_000]);
        assert_eq!(spec(&folder), (2, 48_000, 16, SampleFormat::Int));
        let samples = integers(&folder);
        assert!(samples.iter().all(|&sample| (sample - 16_384).abs() <= 1));
        let offset: i32 = samples.iter().map(|sample| sample - 16_384).sum();
        assert!(offset.abs() < 100);
    }

    #[test]
    fn a_16_bit_file_turns_silence_into_triangular_dither_of_one_step() {
        let folder = Folder::new();
        let _ = written(&folder, Depth::Int16, &vec![0.0; 10_000]);
        let samples = integers(&folder);
        let count = |step| samples.iter().filter(|&&sample| sample == step).count();
        assert_eq!(count(-1) + count(0) + count(1), samples.len());
        assert!((1_000..1_500).contains(&count(1)));
        assert!((1_000..1_500).contains(&count(-1)));
    }

    #[test]
    fn the_file_appears_only_when_it_is_finished() {
        let folder = Folder::new();
        let mut file = WavFile::create(&folder.file(), 48_000, Depth::Int24).unwrap();
        file.write(&[0.5, 0.5]).unwrap();
        assert!(!folder.file().exists());
        file.finish().unwrap();
        assert_eq!(folder.names(), ["Song.wav"]);
    }

    #[test]
    fn a_discarded_file_leaves_nothing_behind() {
        let folder = Folder::new();
        let mut file = WavFile::create(&folder.file(), 48_000, Depth::Int24).unwrap();
        file.write(&[0.5, 0.5]).unwrap();
        file.discard();
        assert_eq!(folder.names(), Vec::<String>::new());
    }

    #[test]
    fn a_file_that_cannot_take_its_name_leaves_nothing_behind() {
        let folder = Folder::new();
        fs::create_dir(folder.file()).unwrap();
        let mut file = WavFile::create(&folder.file(), 48_000, Depth::Int24).unwrap();
        file.write(&[0.5, 0.5]).unwrap();
        assert!(file.finish().is_err());
        assert_eq!(folder.names(), ["Song.wav"]);
    }

    #[test]
    fn a_file_in_a_missing_folder_is_an_error() {
        let folder = Folder::new();
        let target = folder.0.join("Gone").join("Song.wav");
        assert!(WavFile::create(&target, 48_000, Depth::Int24).is_err());
    }
}
