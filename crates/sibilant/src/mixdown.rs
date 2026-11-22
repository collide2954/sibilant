use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use iced::futures::channel::mpsc;
use iced::futures::{Stream, StreamExt, stream};
use sibilant_core::project::{Project, SourceId};
use sibilant_engine::arrangement::{Arrangement, AudioBuffer};
use sibilant_engine::render;
use sibilant_media::{Depth, WavFile, Written};

use crate::library;

const BLOCK: usize = 4_096;
const CHANNELS: usize = 2;

#[derive(Debug, Clone)]
pub(crate) struct Job {
    pub(crate) project: Project,
    pub(crate) target: PathBuf,
    pub(crate) sample_rate: u32,
    pub(crate) depth: Depth,
    pub(crate) audio: BTreeMap<SourceId, Arc<AudioBuffer>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Report {
    Progress(u8),
    Written(Written),
    Stopped,
    Failed(String),
}

pub(crate) fn reports(job: Job, stop: Arc<AtomicBool>) -> impl Stream<Item = Report> {
    stream::once(async move {
        let (sender, receiver) = mpsc::unbounded();
        thread::spawn(move || {
            run(job, &stop, |report| {
                let _ = sender.unbounded_send(report);
            });
        });
        receiver
    })
    .flatten()
}

pub(crate) fn run(job: Job, stop: &AtomicBool, mut report: impl FnMut(Report)) {
    let Job {
        project,
        target,
        sample_rate,
        depth,
        mut audio,
    } = job;
    let missing = read_missing(&project, &audio, sample_rate);
    audio.extend(missing);
    let frames = render::length(&project, sample_rate);
    let arrangement =
        Arrangement::of_project(&project, sample_rate, |source| audio.get(&source).cloned());
    let mut file = match WavFile::create(&target, sample_rate, depth) {
        Ok(file) => file,
        Err(error) => return report(Report::Failed(error.to_string())),
    };
    let (mut done, mut shown, mut failed) = (0, 0, None);
    let finished = render::render(arrangement, frames, BLOCK, |samples| {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        if let Err(error) = file.write(samples) {
            failed = Some(error);
            return false;
        }
        done += samples.len() / CHANNELS;
        let percent = u8::try_from(done * 100 / frames).unwrap_or(100);
        if percent > shown {
            shown = percent;
            report(Report::Progress(percent));
        }
        true
    });
    let ended = match (finished, failed) {
        (true, _) => file
            .finish()
            .map_or_else(|error| Report::Failed(error.to_string()), Report::Written),
        (false, Some(error)) => {
            file.discard();
            Report::Failed(error.to_string())
        }
        (false, None) => {
            file.discard();
            Report::Stopped
        }
    };
    report(ended);
}

fn read_missing(
    project: &Project,
    audio: &BTreeMap<SourceId, Arc<AudioBuffer>>,
    sample_rate: u32,
) -> Vec<(SourceId, Arc<AudioBuffer>)> {
    project
        .sources()
        .filter(|source| !audio.contains_key(&source.id()))
        .filter_map(|source| {
            let loaded = library::load(source.path(), Some(sample_rate)).ok()?;
            Some((source.id(), loaded.playback))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use hound::{SampleFormat, WavReader};
    use iced::futures::StreamExt;
    use iced::futures::executor::block_on;
    use sibilant_core::mix::Mix;
    use sibilant_core::project::{AudioSource, Clip, Ids, Project, Region, SourceId, Track};
    use sibilant_core::time::{Duration, Musical, Position};
    use sibilant_engine::arrangement::AudioBuffer;
    use sibilant_media::{Depth, Written};

    use crate::library::{self, tests::write_tone};
    use crate::mixdown::{Job, Report, reports, run};
    use crate::scratch::Folder;

    const RATE: u32 = 48_000;

    fn with_tone(name: &str, frames: usize) -> (Project, SourceId) {
        with_file(write_tone(name, RATE, frames))
    }

    fn with_file(path: PathBuf) -> (Project, SourceId) {
        let mut ids = Ids::default();
        let source = ids.source();
        let project = Project::new(&mut ids);
        (with_clip(&project, source, path, Position::START), source)
    }

    fn with_clip(
        project: &Project,
        source: SourceId,
        path: PathBuf,
        start: Position<Musical>,
    ) -> Project {
        let length = library::load(&path, None).unwrap().length;
        let track = project.tracks().next().map(Track::id).unwrap();
        let region = Region {
            start: Duration::ZERO,
            length,
        };
        project
            .add_source(AudioSource::new(source, path, length))
            .and_then(|project| project.place_clip(track, start, Clip::new("Tone", source, region)))
            .unwrap()
    }

    fn job(project: Project, target: &Path, sample_rate: u32) -> Job {
        Job {
            project,
            target: target.to_path_buf(),
            sample_rate,
            depth: Depth::Int24,
            audio: BTreeMap::new(),
        }
    }

    fn reports_of(job: Job) -> Vec<Report> {
        let mut reports = Vec::new();
        run(job, &AtomicBool::new(false), |report| reports.push(report));
        reports
    }

    fn file(folder: &Folder) -> PathBuf {
        folder.path().join("MySong.wav")
    }

    fn written(target: &Path) -> (u32, u16, u16, SampleFormat, Vec<i32>) {
        let mut reader = WavReader::open(target).unwrap();
        let spec = reader.spec();
        let samples = reader.samples::<i32>().map(Result::unwrap).collect();
        let hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample,
            sample_format,
        } = spec;
        (
            sample_rate,
            channels,
            bits_per_sample,
            sample_format,
            samples,
        )
    }

    fn peak(peak: f32) -> Report {
        Report::Written(Written {
            peak,
            clipped: false,
        })
    }

    #[test]
    fn an_export_writes_the_project_from_the_start_to_the_end_of_the_last_clip() {
        let folder = Folder::new();
        let (project, _) = with_tone("mixdown-whole.wav", 24_000);
        let reports = reports_of(job(project, &file(&folder), RATE));
        assert_eq!(reports.last(), Some(&peak(0.5)));
        let (rate, channels, bits, format, samples) = written(&file(&folder));
        assert_eq!(
            (rate, channels, bits, format),
            (RATE, 2, 24, SampleFormat::Int)
        );
        assert_eq!(samples.len(), 2 * 24_000);
        assert_eq!(samples[2 * 12_000..2 * 12_000 + 2], [4_194_304, 4_194_304]);
    }

    fn two_levels(folder: &Folder, frames: usize) -> PathBuf {
        let path = folder.path().join("Steps.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: RATE,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for frame in 0..frames {
            let level: i16 = if frame < frames / 2 { 16_384 } else { 8_192 };
            writer.write_sample(level).unwrap();
        }
        writer.finalize().unwrap();
        path
    }

    #[test]
    fn an_export_at_a_different_rate_reads_the_audio_files_at_that_rate() {
        let folder = Folder::new();
        let (project, _) = with_file(two_levels(&folder, 24_000));
        let target = folder.path().join("MySong 44.wav");
        let _ = reports_of(job(project, &target, 44_100));
        let (rate, _, _, _, samples) = written(&target);
        assert_eq!(rate, 44_100);
        assert_eq!(samples.len(), 2 * 22_050);
        assert!((samples[2 * 10_500] - 4_194_304).abs() < 2_000);
        assert!((samples[2 * 11_500] - 2_097_152).abs() < 2_000);
    }

    #[test]
    fn an_export_that_cannot_take_the_name_of_its_file_tells_why() {
        let folder = Folder::new();
        let (project, _) = with_tone("mixdown-taken.wav", 24_000);
        fs::create_dir(file(&folder)).unwrap();
        let reports = reports_of(job(project, &file(&folder), RATE));
        assert!(matches!(reports.last(), Some(Report::Failed(why)) if !why.is_empty()));
        assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 1);
    }

    #[test]
    fn an_export_with_the_audio_in_memory_does_not_read_the_files_again() {
        let folder = Folder::new();
        let (project, source) = with_tone("mixdown-memory.wav", 24_000);
        let quieter = Arc::new(AudioBuffer::new(vec![vec![0.25; 24_000]]));
        let job = Job {
            audio: BTreeMap::from([(source, quieter)]),
            ..job(project, &file(&folder), RATE)
        };
        assert_eq!(reports_of(job).last(), Some(&peak(0.25)));
        assert_eq!(written(&file(&folder)).4[2 * 12_000], 2_097_152);
    }

    #[test]
    fn an_export_reads_the_audio_files_that_are_not_in_memory() {
        let folder = Folder::new();
        let (project, kept) = with_tone("mixdown-kept.wav", 24_000);
        let read = SourceId::new(100);
        let tone = write_tone("mixdown-read.wav", RATE, 24_000);
        let project = with_clip(&project, read, tone, Position::from_quarter_notes(2));
        let quieter = Arc::new(AudioBuffer::new(vec![vec![0.25; 24_000]]));
        let job = Job {
            audio: BTreeMap::from([(kept, quieter)]),
            ..job(project, &file(&folder), RATE)
        };
        let _ = reports_of(job);
        let samples = written(&file(&folder)).4;
        assert_eq!(samples[2 * 12_000], 2_097_152);
        assert_eq!(samples[2 * 60_000], 4_194_304);
    }

    #[test]
    fn a_muted_track_is_silent_in_the_file_but_keeps_its_length() {
        let folder = Folder::new();
        let (project, _) = with_tone("mixdown-muted.wav", 24_000);
        let track = project.tracks().next().map(Track::id).unwrap();
        let muted = Mix {
            mute: true,
            ..Mix::default()
        };
        let project = project.set_mix(track, muted).unwrap();
        assert_eq!(
            reports_of(job(project, &file(&folder), RATE)).last(),
            Some(&peak(0.0))
        );
        assert_eq!(written(&file(&folder)).4.len(), 2 * 24_000);
    }

    #[test]
    fn the_progress_rises_one_percent_at_a_time_to_100_before_the_file_is_written() {
        let folder = Folder::new();
        let (project, _) = with_tone("mixdown-progress.wav", 480_000);
        let reports = reports_of(job(project, &file(&folder), RATE));
        let percents: Vec<u8> = reports
            .iter()
            .filter_map(|report| match report {
                Report::Progress(percent) => Some(*percent),
                _ => None,
            })
            .collect();
        assert_eq!(percents, (1..=100).collect::<Vec<u8>>());
        assert!(matches!(reports.last(), Some(Report::Written(_))));
    }

    #[test]
    fn an_export_that_is_stopped_leaves_no_file() {
        let folder = Folder::new();
        let (project, _) = with_tone("mixdown-stop.wav", 96_000);
        let stop = AtomicBool::new(false);
        let mut reports = Vec::new();
        run(job(project, &file(&folder), RATE), &stop, |report| {
            stop.store(true, Ordering::Relaxed);
            reports.push(report);
        });
        assert_eq!(reports, [Report::Progress(4), Report::Stopped]);
        assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 0);
    }

    #[test]
    fn an_export_that_cannot_make_its_file_tells_why() {
        let folder = Folder::new();
        let (project, _) = with_tone("mixdown-gone.wav", 24_000);
        let target = folder.path().join("Gone").join("MySong.wav");
        let reports = reports_of(job(project, &target, RATE));
        assert!(matches!(reports.as_slice(), [Report::Failed(why)] if !why.is_empty()));
    }

    #[test]
    fn the_reports_come_from_a_thread_in_the_background() {
        let folder = Folder::new();
        let (project, _) = with_tone("mixdown-thread.wav", 24_000);
        let stream = reports(job(project, &file(&folder), RATE), Arc::default());
        let reports: Vec<Report> = block_on(stream.collect());
        assert_eq!(reports.last(), Some(&peak(0.5)));
    }
}
