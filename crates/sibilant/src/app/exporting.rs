use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Task;
use sibilant_media::Written;
use sibilant_ui::browser::Progress;
use sibilant_ui::command::Command;
use sibilant_ui::export::{Event, Format};

use super::{App, Message, Overlay, dialogs};
use crate::audio::Audio;
use crate::mixdown::{self, Job, Report};

const WAV: &str = "wav";

#[derive(Debug)]
pub(super) struct Exporting {
    name: String,
    stop: Arc<AtomicBool>,
}

impl App {
    pub(super) fn ask_how_to_export(&mut self) {
        if self.ready_to_export() {
            self.overlay = Some(Overlay::Export);
        }
    }

    pub(super) fn choose_export(&mut self, event: Event) -> Task<Message> {
        if self.overlay != Some(Overlay::Export) {
            return Task::none();
        }
        match event {
            Event::SampleRate(sample_rate) => self.export_format.sample_rate = sample_rate,
            Event::Depth(depth) => self.export_format.depth = depth,
            Event::Export => {
                self.overlay = None;
                return dialogs::export_file(&self.document.name()).map(Message::ExportTo);
            }
            Event::Cancel => self.overlay = None,
        }
        Task::none()
    }

    pub(super) fn export_by_key(&mut self, command: Command) -> Task<Message> {
        match command {
            Command::Confirm => self.choose_export(Event::Export),
            Command::Cancel => self.choose_export(Event::Cancel),
            _ => Task::none(),
        }
    }

    pub(super) fn start_export(&mut self, chosen: PathBuf) -> Task<Message> {
        if !self.ready_to_export() {
            return Task::none();
        }
        let job = self.export_job(wav_file(chosen));
        let name = job
            .target
            .file_name()
            .map_or_default(|name| name.to_string_lossy().into_owned());
        let stop = Arc::new(AtomicBool::new(false));
        self.browser.clear_notice();
        self.browser.set_progress(Some(progress(&name, 0)));
        self.exporting = Some(Exporting {
            name,
            stop: Arc::clone(&stop),
        });
        Task::run(mixdown::reports(job, stop), Message::Mixdown)
    }

    pub(super) fn follow_export(&mut self, report: Report) {
        let Some(name) = self.exporting.as_ref().map(|export| export.name.clone()) else {
            return;
        };
        match report {
            Report::Progress(percent) => self.browser.set_progress(Some(progress(&name, percent))),
            Report::Written(written) => {
                self.end_export();
                self.show_written(&name, written);
            }
            Report::Stopped => self.end_export(),
            Report::Failed(why) => {
                self.end_export();
                self.browser
                    .show_problem(format!("Sibilant did not export the project: {why}"));
            }
        }
    }

    pub(super) fn stop_export(&self) {
        if let Some(export) = &self.exporting {
            export.stop.store(true, Ordering::Relaxed);
        }
    }

    fn ready_to_export(&mut self) -> bool {
        if self.exporting.is_some() {
            return false;
        }
        let has_clips = self
            .history
            .present()
            .tracks()
            .any(|track| track.timeline().next().is_some());
        if !has_clips {
            self.browser.show_problem(
                "Sibilant did not export the project: the project has no clips".to_owned(),
            );
        }
        has_clips
    }

    fn export_job(&self, target: PathBuf) -> Job {
        let Format { sample_rate, depth } = self.export_format;
        let in_memory = self.audio.as_ref().map(Audio::sample_rate) == Some(sample_rate);
        Job {
            project: self.history.present().clone(),
            target,
            sample_rate,
            depth,
            audio: if in_memory {
                self.pool.clone()
            } else {
                BTreeMap::new()
            },
        }
    }

    fn end_export(&mut self) {
        self.exporting = None;
        self.browser.set_progress(None);
    }

    fn show_written(&mut self, name: &str, Written { peak, clipped }: Written) {
        let level = format!("{:+.1} dBFS", 20.0 * peak.log10()).replace('-', "\u{2212}");
        if clipped {
            self.browser.show_problem(format!(
                "Exported {name}, peak {level}. The export clipped."
            ));
        } else if peak > 0.0 {
            self.browser
                .show_info(format!("Exported {name}, peak {level}"));
        } else {
            self.browser
                .show_info(format!("Exported {name}. The file is silent."));
        }
    }
}

fn progress(name: &str, percent: u8) -> Progress {
    Progress {
        task: format!("Exporting {name}"),
        done: f32::from(percent) / 100.0,
    }
}

fn wav_file(chosen: PathBuf) -> PathBuf {
    if chosen
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(WAV))
    {
        return chosen;
    }
    let mut file = chosen.into_os_string();
    file.push(format!(".{WAV}"));
    file.into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    use iced::Task;
    use iced_test::simulator;
    use sibilant_core::project::{AudioSource, Clip, Region, SourceId};
    use sibilant_core::time::{Duration, Position};
    use sibilant_engine::arrangement::AudioBuffer;
    use sibilant_media::{Depth, Written};
    use sibilant_ui::browser::{self, Browser, Progress};
    use sibilant_ui::command::Command;
    use sibilant_ui::export::{Event, Format};
    use sibilant_ui::menu;

    use crate::app::{App, Message};
    use crate::audio::Audio;
    use crate::mixdown::Report;

    const NO_CLIPS: &str = "Sibilant did not export the project: the project has no clips";

    fn with_a_clip() -> App {
        let mut app = App::default();
        let source = app.ids.source();
        let present = app.history.present();
        let track = present.tracks().next().unwrap().id();
        let second = Duration::from_seconds(1);
        let region = Region {
            start: Duration::ZERO,
            length: second,
        };
        let project = present
            .add_source(AudioSource::new(source, "kick.wav", second))
            .and_then(|project| {
                project.place_clip(track, Position::START, Clip::new("kick", source, region))
            })
            .unwrap();
        app.history.commit(project);
        app
    }

    fn run(app: &mut App, command: Command) -> Task<Message> {
        app.update(Message::Command(command))
    }

    fn choose(app: &mut App, event: Event) -> Task<Message> {
        app.update(Message::Export(event))
    }

    fn shows(app: &App, text: &str) -> bool {
        simulator(app.view()).find(text).is_ok()
    }

    fn asks_how_to_export(app: &App) -> bool {
        shows(app, "Export “Untitled”")
    }

    fn my_song() -> PathBuf {
        PathBuf::from("songs").join("MySong.wav")
    }

    fn export_to(app: &mut App, file: PathBuf) -> Task<Message> {
        app.update(Message::ExportTo(Some(file)))
    }

    fn exporting() -> App {
        let mut app = with_a_clip();
        let _ = export_to(&mut app, my_song());
        app
    }

    fn report(app: &mut App, report: Report) {
        let _ = app.update(Message::Mixdown(report));
    }

    fn written(peak: f32, clipped: bool) -> Report {
        Report::Written(Written { peak, clipped })
    }

    fn progress(done: f32) -> Progress {
        Progress {
            task: "Exporting MySong.wav".into(),
            done,
        }
    }

    #[test]
    fn export_asks_for_the_format_of_the_file() {
        let mut app = with_a_clip();
        let _ = run(&mut app, Command::Export);
        assert!(asks_how_to_export(&app));
    }

    #[test]
    fn the_export_item_of_the_file_menu_asks_for_the_format() {
        let mut app = with_a_clip();
        let _ = app.update(Message::Menu(menu::Event::Choose(Command::Export)));
        assert!(asks_how_to_export(&app));
    }

    #[test]
    fn export_of_a_project_without_clips_tells_why_and_asks_nothing() {
        let mut app = App::default();
        let _ = run(&mut app, Command::Export);
        assert!(!asks_how_to_export(&app));
        assert!(shows(&app, NO_CLIPS));
    }

    #[test]
    fn sibilant_keeps_the_chosen_format_for_the_next_export() {
        let mut app = with_a_clip();
        let _ = run(&mut app, Command::Export);
        let _ = choose(&mut app, Event::SampleRate(44_100));
        let _ = choose(&mut app, Event::Depth(Depth::Int16));
        let _ = choose(&mut app, Event::Cancel);
        assert!(!asks_how_to_export(&app));
        let _ = run(&mut app, Command::Export);
        let chosen = Format {
            sample_rate: 44_100,
            depth: Depth::Int16,
        };
        assert_eq!(app.export_format, chosen);
    }

    #[test]
    fn export_in_the_dialog_asks_where_to_save_the_file() {
        let mut app = with_a_clip();
        let _ = run(&mut app, Command::Export);
        assert_eq!(choose(&mut app, Event::Export).units(), 1);
        assert!(!asks_how_to_export(&app));
    }

    #[test]
    fn enter_exports_and_escape_cancels() {
        let mut app = with_a_clip();
        let _ = run(&mut app, Command::Export);
        assert_eq!(run(&mut app, Command::Confirm).units(), 1);
        assert!(!asks_how_to_export(&app));
        let _ = run(&mut app, Command::Export);
        assert_eq!(run(&mut app, Command::Cancel).units(), 0);
        assert!(!asks_how_to_export(&app));
    }

    #[test]
    fn other_shortcuts_do_nothing_while_the_dialog_asks() {
        let mut app = with_a_clip();
        let _ = run(&mut app, Command::Export);
        let _ = run(&mut app, Command::AddTrack);
        assert_eq!(app.history.present().tracks().len(), 2);
        assert!(asks_how_to_export(&app));
    }

    #[test]
    fn a_choice_that_arrives_after_the_dialog_closed_does_nothing() {
        let mut app = with_a_clip();
        assert_eq!(choose(&mut app, Event::Export).units(), 0);
        let _ = choose(&mut app, Event::SampleRate(96_000));
        assert_eq!(app.export_format, Format::default());
    }

    #[test]
    fn a_chosen_file_starts_the_export_in_the_background() {
        let mut app = with_a_clip();
        let mut expected = app.browser.clone();
        assert_eq!(export_to(&mut app, my_song()).units(), 1);
        expected.set_progress(Some(progress(0.0)));
        assert_eq!(app.browser, expected);
        assert!(shows(&app, "Exporting MySong.wav"));
    }

    #[test]
    fn a_closed_file_dialog_exports_nothing() {
        let mut app = with_a_clip();
        assert_eq!(app.update(Message::ExportTo(None)).units(), 0);
        assert!(app.exporting.is_none());
    }

    #[test]
    fn a_file_name_without_the_wav_extension_gets_it() {
        let mut app = with_a_clip();
        let _ = export_to(&mut app, PathBuf::from("songs").join("My.Song"));
        assert!(shows(&app, "Exporting My.Song.wav"));
        let mut app = with_a_clip();
        let _ = export_to(&mut app, PathBuf::from("songs").join("MySong.WAV"));
        assert!(shows(&app, "Exporting MySong.WAV"));
    }

    #[test]
    fn an_export_starts_with_an_empty_notice_line() {
        let mut app = with_a_clip();
        app.browser.show_problem("An older problem".into());
        let _ = export_to(&mut app, my_song());
        assert!(!shows(&app, "An older problem"));
    }

    #[test]
    fn a_project_that_lost_its_clips_before_the_file_was_chosen_exports_nothing() {
        let mut app = with_a_clip();
        let _ = run(&mut app, Command::Export);
        let _ = run(&mut app, Command::Confirm);
        let _ = run(&mut app, Command::Undo);
        assert_eq!(export_to(&mut app, my_song()).units(), 0);
        assert!(shows(&app, NO_CLIPS));
    }

    #[test]
    fn the_export_uses_the_chosen_format_and_the_project_as_it_is() {
        let mut app = with_a_clip();
        app.export_format = Format {
            sample_rate: 44_100,
            depth: Depth::Int16,
        };
        let job = app.export_job(my_song());
        assert_eq!((job.sample_rate, job.depth), (44_100, Depth::Int16));
        assert_eq!(&job.project, app.history.present());
        assert_eq!(job.target, my_song());
    }

    #[test]
    fn the_export_takes_the_audio_in_memory_only_at_the_rate_of_the_device() {
        let mut app = with_a_clip();
        let buffer = Arc::new(AudioBuffer::new(vec![vec![0.5; 480]]));
        app.pool.insert(SourceId::new(1), buffer);
        assert!(app.export_job(my_song()).audio.is_empty());
        let (audio, _engine) = Audio::without_device(app.history.present(), 48_000);
        app.attach_audio(audio);
        assert_eq!(app.export_job(my_song()).audio.len(), 1);
        app.export_format.sample_rate = 44_100;
        assert!(app.export_job(my_song()).audio.is_empty());
    }

    #[test]
    fn the_progress_line_follows_the_export() {
        let mut app = exporting();
        let mut expected = app.browser.clone();
        report(&mut app, Report::Progress(40));
        expected.set_progress(Some(progress(0.4)));
        assert_eq!(app.browser, expected);
    }

    fn ends_with(ending: Report, notice: impl FnOnce(&mut Browser)) {
        let mut app = exporting();
        let mut expected = app.browser.clone();
        report(&mut app, ending);
        expected.set_progress(None);
        notice(&mut expected);
        assert_eq!(app.browser, expected);
        assert!(app.exporting.is_none());
    }

    #[test]
    fn a_written_file_shows_its_name_and_its_peak() {
        ends_with(written(0.5, false), |browser| {
            browser.show_info("Exported MySong.wav, peak \u{2212}6.0 dBFS".into());
        });
    }

    #[test]
    fn a_clipped_export_tells_it_as_a_problem() {
        ends_with(written(1.5, true), |browser| {
            browser.show_problem("Exported MySong.wav, peak +3.5 dBFS. The export clipped.".into());
        });
    }

    #[test]
    fn a_silent_export_tells_it() {
        ends_with(written(0.0, false), |browser| {
            browser.show_info("Exported MySong.wav. The file is silent.".into());
        });
    }

    #[test]
    fn a_failed_export_tells_why() {
        ends_with(Report::Failed("disk full".into()), |browser| {
            browser.show_problem("Sibilant did not export the project: disk full".into());
        });
    }

    #[test]
    fn a_stopped_export_removes_only_the_progress_line() {
        ends_with(Report::Stopped, |_| {});
    }

    #[test]
    fn cancel_in_the_progress_line_stops_the_export() {
        let mut app = exporting();
        let stop = Arc::clone(&app.exporting.as_ref().unwrap().stop);
        let _ = app.update(Message::Browser(browser::Event::Stop));
        assert!(stop.load(Ordering::Relaxed));
    }

    #[test]
    fn a_second_export_waits_until_the_first_ends() {
        let mut app = exporting();
        let _ = run(&mut app, Command::Export);
        assert!(!asks_how_to_export(&app));
        assert_eq!(export_to(&mut app, my_song()).units(), 0);
        report(&mut app, written(0.5, false));
        let _ = run(&mut app, Command::Export);
        assert!(asks_how_to_export(&app));
    }
}
