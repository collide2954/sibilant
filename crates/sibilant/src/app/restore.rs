use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use iced::Task;
use sibilant_project::{Location, Orphan};
use sibilant_ui::command::Command;
use sibilant_ui::dialog::RestoreChoice;

use super::autosave::Recovery;
use super::document::{Document, UNTITLED};
use super::{App, Message, Overlay};

impl App {
    pub(super) fn start_recovery(&mut self, root: Option<PathBuf>) {
        self.recovery = Recovery::at(root);
        self.recovery.find_orphans();
        self.offer_next_copy();
    }

    pub(super) fn offered_copy(&self) -> Option<(String, Duration)> {
        let orphan = self.recovery.next_orphan()?;
        let name = orphan
            .project()
            .map_or_else(|| UNTITLED.to_owned(), Location::name);
        let age = orphan
            .written()
            .and_then(|written| SystemTime::now().duration_since(written).ok())
            .unwrap_or_default();
        Some((name, age))
    }

    pub(super) fn answer_restore(&mut self, choice: RestoreChoice) -> Task<Message> {
        self.overlay = None;
        let Some(orphan) = self.recovery.take_orphan() else {
            return Task::none();
        };
        match choice {
            RestoreChoice::Restore => {
                self.recovery.forget_orphans();
                self.restore(orphan)
            }
            RestoreChoice::DontRestore => {
                let _ = orphan.discard();
                self.offer_next_copy();
                Task::none()
            }
        }
    }

    pub(super) fn restore_by_key(&mut self, command: Command) -> Task<Message> {
        match command {
            Command::Confirm => self.answer_restore(RestoreChoice::Restore),
            Command::Cancel => {
                self.overlay = None;
                self.recovery.forget_orphans();
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn offer_next_copy(&mut self) {
        if self.recovery.next_orphan().is_some() {
            self.overlay = Some(Overlay::Restore);
        }
    }

    fn restore(&mut self, orphan: Orphan) -> Task<Message> {
        let Some(copy) = orphan.copy() else {
            return Task::none();
        };
        match sibilant_project::restore(copy, &orphan.location()) {
            Ok(project) => {
                self.start_document(project, Document::restored(orphan.project().cloned()));
                if let Err(error) = self.recovery.adopt(orphan) {
                    self.browser.set_notice(Some(format!(
                        "Sibilant did not take over the recovery folder: {error}"
                    )));
                }
                self.load_sources()
            }
            Err(error) => {
                self.browser.set_notice(Some(format!(
                    "Sibilant did not restore the project: {error}"
                )));
                Task::none()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use iced_test::simulator;
    use sibilant_core::project::{AudioSource, Project, SourceId, Track, TrackId};
    use sibilant_core::time::Duration;
    use sibilant_project::{Copies, Location, Session, orphans};
    use sibilant_ui::command::Command;
    use sibilant_ui::dialog::RestoreChoice;

    use crate::app::scratch::Folder;
    use crate::app::{App, Message, autosave};
    use crate::library::tests::write_tone;

    fn left_behind(folder: &Folder, project: &Project, location: Option<&Location>) -> PathBuf {
        let session = Session::start(&folder.recovery()).unwrap();
        let written =
            autosave::write(project, location, &Copies::default(), session.folder(), 7).unwrap();
        drop(session);
        written.file
    }

    fn starting(folder: &Folder) -> App {
        let mut app = App::default();
        app.start_recovery(Some(folder.recovery()));
        app
    }

    fn drums() -> Project {
        Project::default()
            .add_track(TrackId::new(1), "Drums")
            .unwrap()
    }

    fn track_names(app: &App) -> Vec<&str> {
        app.history.present().tracks().map(Track::name).collect()
    }

    fn offers(app: &App, name: &str) -> bool {
        let question = format!("Restore the unsaved changes to “{name}”?");
        simulator(app.view()).find(question.as_str()).is_ok()
    }

    fn answer(app: &mut App, choice: RestoreChoice) -> iced::Task<Message> {
        app.update(Message::Restore(choice))
    }

    #[test]
    fn the_start_offers_the_copy_that_a_stopped_sibilant_left() {
        let folder = Folder::new();
        left_behind(&folder, &drums(), None);
        assert!(offers(&starting(&folder), "Untitled"));
    }

    #[test]
    fn the_start_offers_nothing_when_no_sibilant_stopped() {
        let folder = Folder::new();
        assert!(!offers(&starting(&folder), "Untitled"));
    }

    #[test]
    fn a_session_with_nothing_to_restore_is_removed_at_the_start() {
        let folder = Folder::new();
        drop(Session::start(&folder.recovery()).unwrap());
        let app = starting(&folder);
        assert!(!offers(&app, "Untitled"));
        assert_eq!(folder.sessions(), 0);
    }

    #[test]
    fn restore_opens_the_copy_as_an_unsaved_project() {
        let folder = Folder::new();
        left_behind(&folder, &drums(), None);
        let mut app = starting(&folder);
        let _ = answer(&mut app, RestoreChoice::Restore);
        assert_eq!(track_names(&app), ["Drums"]);
        assert_eq!(app.title(), "● Untitled — Sibilant");
        assert!(!offers(&app, "Untitled"));
        let _ = app.update(Message::Command(Command::AddTrack));
        assert_eq!(track_names(&app).len(), 2);
    }

    #[test]
    fn a_restored_copy_is_unsaved_even_when_it_is_empty() {
        let folder = Folder::new();
        left_behind(&folder, &Project::default(), None);
        let mut app = starting(&folder);
        let _ = answer(&mut app, RestoreChoice::Restore);
        assert_eq!(app.title(), "● Untitled — Sibilant");
    }

    #[test]
    fn a_restored_project_keeps_its_project_file() {
        let folder = Folder::new();
        let location = Location::of_file(folder.path().join("MySong").join("MySong.sibilant"));
        left_behind(&folder, &drums(), Some(&location));
        let mut app = starting(&folder);
        assert!(offers(&app, "MySong"));
        let _ = answer(&mut app, RestoreChoice::Restore);
        assert_eq!(app.title(), "● MySong — Sibilant");
        assert_eq!(app.document.location(), Some(&location));
    }

    #[test]
    fn restore_takes_over_the_session_of_the_copy() {
        let folder = Folder::new();
        left_behind(&folder, &drums(), None);
        let mut app = starting(&folder);
        let _ = answer(&mut app, RestoreChoice::Restore);
        assert!(orphans(&folder.recovery()).is_empty());
        assert_eq!(folder.sessions(), 1);
    }

    #[test]
    fn the_audio_of_a_restored_project_loads_in_the_background() {
        let folder = Folder::new();
        let kick = write_tone("restore-kick.wav", 48_000, 480);
        let source = AudioSource::new(SourceId::new(1), &kick, Duration::from_seconds(1));
        left_behind(
            &folder,
            &Project::default().add_source(source).unwrap(),
            None,
        );
        let mut app = starting(&folder);
        assert_eq!(answer(&mut app, RestoreChoice::Restore).units(), 1);
    }

    #[test]
    fn dont_restore_removes_the_copy_and_offers_the_next() {
        let folder = Folder::new();
        left_behind(&folder, &drums(), None);
        left_behind(&folder, &drums(), None);
        let mut app = starting(&folder);
        let _ = answer(&mut app, RestoreChoice::DontRestore);
        assert!(offers(&app, "Untitled"));
        assert_eq!(folder.sessions(), 1);
        let _ = answer(&mut app, RestoreChoice::DontRestore);
        assert!(!offers(&app, "Untitled"));
        assert_eq!(folder.sessions(), 0);
    }

    #[test]
    fn after_a_restore_the_other_copies_wait_for_the_next_start() {
        let folder = Folder::new();
        left_behind(&folder, &drums(), None);
        left_behind(&folder, &drums(), None);
        let mut app = starting(&folder);
        let _ = answer(&mut app, RestoreChoice::Restore);
        assert!(!offers(&app, "Untitled"));
        assert_eq!(orphans(&folder.recovery()).len(), 1);
    }

    #[test]
    fn escape_keeps_the_copies_for_the_next_start() {
        let folder = Folder::new();
        left_behind(&folder, &drums(), None);
        let mut app = starting(&folder);
        let _ = app.update(Message::Command(Command::Cancel));
        assert!(!offers(&app, "Untitled"));
        assert_eq!(orphans(&folder.recovery()).len(), 1);
        let _ = app.update(Message::Command(Command::AddTrack));
        assert_eq!(track_names(&app).len(), 3);
    }

    #[test]
    fn enter_restores() {
        let folder = Folder::new();
        left_behind(&folder, &drums(), None);
        let mut app = starting(&folder);
        let _ = app.update(Message::Command(Command::Confirm));
        assert_eq!(track_names(&app), ["Drums"]);
    }

    #[test]
    fn shortcuts_do_nothing_while_the_restore_dialog_asks() {
        let folder = Folder::new();
        left_behind(&folder, &drums(), None);
        let mut app = starting(&folder);
        let _ = app.update(Message::Command(Command::AddTrack));
        assert_eq!(track_names(&app).len(), 2);
        assert!(offers(&app, "Untitled"));
    }

    #[test]
    fn a_copy_that_cannot_be_read_tells_why_and_stays() {
        let folder = Folder::new();
        let copy = left_behind(&folder, &drums(), None);
        fs::write(&copy, br#"{"format": 2}"#).unwrap();
        let mut app = starting(&folder);
        let _ = answer(&mut app, RestoreChoice::Restore);
        let notice = "Sibilant did not restore the project: the project file needs a newer version of Sibilant (format 2)";
        assert!(simulator(app.view()).find(notice).is_ok());
        assert_eq!(orphans(&folder.recovery()).len(), 1);
    }
}
