use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use iced::Task;
use sibilant_core::project::Project;
use sibilant_project::{Autosaved, Copies, Location, Orphan, Session, SessionFolder};

use super::document::DocumentId;
use super::{App, Message, Writing};

pub(super) const INTERVAL: Duration = Duration::from_secs(120);

#[derive(Debug, Default)]
pub(super) struct Recovery {
    root: Option<PathBuf>,
    session: Option<Session>,
    orphans: Vec<Orphan>,
}

impl Recovery {
    pub(super) fn at(root: Option<PathBuf>) -> Self {
        Self {
            root,
            ..Self::default()
        }
    }

    pub(super) fn find_orphans(&mut self) {
        let Some(root) = &self.root else {
            return;
        };
        let (restorable, empty): (Vec<_>, Vec<_>) = sibilant_project::orphans(root)
            .into_iter()
            .partition(|orphan| orphan.copy().is_some());
        for orphan in empty {
            let _ = orphan.discard();
        }
        self.orphans = restorable;
    }

    pub(super) fn next_orphan(&self) -> Option<&Orphan> {
        self.orphans.first()
    }

    pub(super) fn take_orphan(&mut self) -> Option<Orphan> {
        (!self.orphans.is_empty()).then(|| self.orphans.remove(0))
    }

    pub(super) fn forget_orphans(&mut self) {
        self.orphans.clear();
    }

    pub(super) fn adopt(&mut self, orphan: Orphan) -> io::Result<()> {
        self.session = Some(orphan.adopt()?);
        Ok(())
    }

    pub(super) fn is_on(&self) -> bool {
        self.root.is_some()
    }

    pub(super) fn folder(&self) -> Option<SessionFolder> {
        self.session
            .as_ref()
            .map(|session| session.folder().clone())
    }

    pub(super) fn end(&mut self) {
        if let Some(session) = self.session.take() {
            let _ = session.end();
        }
    }

    fn start(&mut self) -> Result<SessionFolder, String> {
        if let Some(folder) = self.folder() {
            return Ok(folder);
        }
        let root = self
            .root
            .as_ref()
            .ok_or("Sibilant has no recovery folder")?;
        let session = Session::start(root)
            .map_err(|error| format!("cannot make the recovery folder: {error}"))?;
        let folder = session.folder().clone();
        self.session = Some(session);
        Ok(folder)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Copying {
    pub(crate) document: DocumentId,
    pub(crate) project: Project,
    pub(crate) result: Result<Autosaved, String>,
}

pub(crate) fn write(
    project: &Project,
    location: Option<&Location>,
    copies: &Copies,
    folder: &SessionFolder,
    time: u64,
) -> Result<Autosaved, String> {
    let target = location.cloned().unwrap_or_else(|| folder.untitled());
    let autosaved = sibilant_project::autosave(project, &target, copies, time)
        .map_err(|error| error.to_string())?;
    folder
        .note(location, Some(&autosaved.file))
        .map_err(|error| format!("cannot write the recovery note: {error}"))?;
    Ok(autosaved)
}

impl App {
    pub(super) fn autosave(&mut self) -> Task<Message> {
        if self.writing.is_some() || !self.document.needs_copy(self.history.present()) {
            return Task::none();
        }
        let folder = match self.recovery.start() {
            Ok(folder) => folder,
            Err(error) => {
                self.autosave_failed(&error);
                return Task::none();
            }
        };
        self.writing = Some(Writing::Autosave);
        let document = self.document.id();
        let project = self.history.present().clone();
        let location = self.document.location().cloned();
        let copies = self.document.copies().clone();
        Task::perform(
            async move {
                let time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |since| since.as_secs());
                let result = write(&project, location.as_ref(), &copies, &folder, time);
                Box::new(Copying {
                    document,
                    project,
                    result,
                })
            },
            Message::Autosaved,
        )
    }

    pub(super) fn finish_autosave(&mut self, copying: Copying) -> Task<Message> {
        self.writing = None;
        match copying.result {
            Ok(autosaved) if copying.document == self.document.id() => {
                self.document.copied(copying.project, autosaved.copies);
            }
            Ok(_) => {}
            Err(error) => self.autosave_failed(&error),
        }
        match self.queued_save.take() {
            Some(target) => self.save_to(target),
            None => self.resume_leaving(),
        }
    }

    fn autosave_failed(&mut self, error: &str) {
        self.browser
            .show_problem(format!("Sibilant did not write a recovery copy: {error}"));
    }
}

pub(crate) fn recovery_folder(var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let folder = |name: &str| {
        var(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    if cfg!(target_os = "windows") {
        Some(folder("LOCALAPPDATA")?.join("Sibilant").join("Recovery"))
    } else if cfg!(target_os = "macos") {
        let support = folder("HOME")?.join("Library").join("Application Support");
        Some(support.join("Sibilant").join("Recovery"))
    } else {
        let state = folder("XDG_STATE_HOME")
            .or_else(|| Some(folder("HOME")?.join(".local").join("state")))?;
        Some(state.join("sibilant").join("recovery"))
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use crate::app::autosave::recovery_folder;

    #[cfg(target_os = "windows")]
    #[test]
    fn on_windows_the_recovery_folder_is_in_the_local_app_data() {
        let var =
            |name: &str| (name == "LOCALAPPDATA").then(|| r"C:\Users\user\AppData\Local".into());
        assert_eq!(
            recovery_folder(var),
            Some(PathBuf::from(
                r"C:\Users\user\AppData\Local\Sibilant\Recovery"
            ))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn on_macos_the_recovery_folder_is_in_the_application_support() {
        let var = |name: &str| (name == "HOME").then(|| "/Users/user".into());
        assert_eq!(
            recovery_folder(var),
            Some(PathBuf::from(
                "/Users/user/Library/Application Support/Sibilant/Recovery"
            ))
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn on_linux_the_recovery_folder_is_in_the_state_folder() {
        let home = |name: &str| (name == "HOME").then(|| "/home/user".into());
        assert_eq!(
            recovery_folder(home),
            Some(PathBuf::from("/home/user/.local/state/sibilant/recovery"))
        );
        let state = |name: &str| match name {
            "XDG_STATE_HOME" => Some("/state".into()),
            "HOME" => Some("/home/user".into()),
            _ => None,
        };
        assert_eq!(
            recovery_folder(state),
            Some(PathBuf::from("/state/sibilant/recovery"))
        );
    }

    #[test]
    fn a_relative_folder_in_the_environment_gives_no_recovery_folder() {
        assert_eq!(recovery_folder(|_| Some(OsString::from("relative"))), None);
        assert_eq!(recovery_folder(|_| None), None);
    }
}

#[cfg(test)]
mod flow {
    use std::path::PathBuf;

    use iced::Task;
    use iced_test::simulator;
    use sibilant_core::project::{AudioSource, Project, SourceId};
    use sibilant_core::time::Duration;
    use sibilant_project::{Autosaved, Copies, Location, Opened, Session, orphans};
    use sibilant_ui::command::Command;
    use sibilant_ui::dialog::Choice;

    use crate::app::autosave::{Copying, Recovery, write};
    use crate::app::{App, Message, Writing};
    use crate::library::tests::write_tone;
    use crate::scratch::Folder;

    fn recovering(folder: &Folder) -> App {
        App {
            recovery: Recovery::at(Some(folder.recovery())),
            ..App::default()
        }
    }

    fn change(app: &mut App) {
        let _ = app.update(Message::Command(Command::AddTrack));
    }

    fn tick(app: &mut App) -> Task<Message> {
        app.update(Message::Autosave)
    }

    fn copied(app: &mut App, result: Result<Autosaved, String>) -> Task<Message> {
        let copying = Copying {
            document: app.document.id(),
            project: app.history.present().clone(),
            result,
        };
        app.update(Message::Autosaved(Box::new(copying)))
    }

    fn autosaved() -> Autosaved {
        Autosaved {
            file: PathBuf::from("Untitled 7.sibilant"),
            copies: Copies::default(),
        }
    }

    fn asks_to_save(app: &App) -> bool {
        simulator(app.view())
            .find("Do you want to save the changes to “Untitled”?")
            .is_ok()
    }

    #[test]
    fn write_puts_the_copy_of_an_untitled_project_into_the_session_folder() {
        let folder = Folder::new();
        let session = Session::start(&folder.recovery()).unwrap();
        let written = write(
            &Project::default(),
            None,
            &Copies::default(),
            session.folder(),
            7,
        )
        .unwrap();
        let untitled = session.folder().untitled();
        assert_eq!(
            written.file,
            untitled.autosave().join("Untitled 7.sibilant")
        );
        drop(session);
        let found = orphans(&folder.recovery());
        assert_eq!(found[0].copy(), Some(written.file.as_path()));
        assert_eq!(found[0].project(), None);
    }

    #[test]
    fn write_puts_the_copy_of_a_saved_project_into_its_autosave_folder() {
        let folder = Folder::new();
        let location = Location::of_file(folder.path().join("MySong").join("MySong.sibilant"));
        let session = Session::start(&folder.recovery()).unwrap();
        let written = write(
            &Project::default(),
            Some(&location),
            &Copies::default(),
            session.folder(),
            7,
        )
        .unwrap();
        assert_eq!(written.file, location.autosave().join("MySong 7.sibilant"));
        drop(session);
        assert_eq!(orphans(&folder.recovery())[0].project(), Some(&location));
    }

    #[test]
    fn a_tick_writes_a_copy_only_when_the_project_has_unsaved_changes() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        assert_eq!(tick(&mut app).units(), 0);
        change(&mut app);
        assert_eq!(tick(&mut app).units(), 1);
        assert_eq!(app.writing, Some(Writing::Autosave));
    }

    #[test]
    fn the_first_copy_starts_a_locked_session_in_the_recovery_folder() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        change(&mut app);
        let _ = tick(&mut app);
        assert_eq!(folder.sessions(), 1);
        assert!(orphans(&folder.recovery()).is_empty());
    }

    #[test]
    fn each_copy_uses_the_same_session() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        change(&mut app);
        let _ = tick(&mut app);
        let _ = copied(&mut app, Ok(autosaved()));
        change(&mut app);
        let _ = tick(&mut app);
        assert_eq!(folder.sessions(), 1);
    }

    #[test]
    fn no_copy_is_written_when_nothing_changed_after_the_last_copy() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        change(&mut app);
        let _ = tick(&mut app);
        let _ = copied(&mut app, Ok(autosaved()));
        assert_eq!(tick(&mut app).units(), 0);
        change(&mut app);
        assert_eq!(tick(&mut app).units(), 1);
    }

    #[test]
    fn a_save_during_an_autosave_starts_when_the_autosave_ends() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        change(&mut app);
        let _ = tick(&mut app);
        let chosen = Message::SaveAs(Some(folder.path().join("MySong.sibilant")));
        assert_eq!(app.update(chosen).units(), 0);
        assert_eq!(copied(&mut app, Ok(autosaved())).units(), 1);
        assert_eq!(app.writing, Some(Writing::Save));
    }

    #[test]
    fn a_tick_during_a_save_writes_no_copy() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        change(&mut app);
        let _ = app.update(Message::SaveAs(Some(folder.path().join("MySong.sibilant"))));
        assert_eq!(tick(&mut app).units(), 0);
        assert_eq!(app.writing, Some(Writing::Save));
    }

    #[test]
    fn new_waits_for_an_autosave_to_end() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        change(&mut app);
        let _ = tick(&mut app);
        let _ = app.update(Message::Command(Command::New));
        assert!(!asks_to_save(&app));
        let _ = copied(&mut app, Ok(autosaved()));
        assert!(asks_to_save(&app));
    }

    #[test]
    fn a_failed_autosave_tells_why() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        change(&mut app);
        let _ = tick(&mut app);
        let _ = copied(&mut app, Err("disk full".into()));
        let notice = "Sibilant did not write a recovery copy: disk full";
        assert!(simulator(app.view()).find(notice).is_ok());
        assert_eq!(app.writing, None);
    }

    #[test]
    fn an_autosave_that_ends_after_an_open_leaves_the_opened_project_alone() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        change(&mut app);
        let _ = tick(&mut app);
        let older = app.document.id();
        let opened = Opened {
            project: Project::default(),
            location: Location::of_file(folder.path().join("Other").join("Other.sibilant")),
        };
        let _ = app.update(Message::Opened(Box::new(Ok(opened))));
        let kick = write_tone("autosave-kick.wav", 48_000, 480);
        let source = AudioSource::new(SourceId::new(1), &kick, Duration::from_seconds(1));
        let project = Project::default().add_source(source).unwrap();
        let elsewhere = Session::start(&folder.path().join("Elsewhere")).unwrap();
        let written = write(&project, None, &Copies::default(), elsewhere.folder(), 7);
        let copying = Copying {
            document: older,
            project,
            result: written,
        };
        let _ = app.update(Message::Autosaved(Box::new(copying)));
        assert_eq!(app.document.copies(), &Copies::default());
        assert_eq!(app.writing, None);
    }

    #[test]
    fn new_ends_the_session() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        change(&mut app);
        let _ = tick(&mut app);
        let _ = copied(&mut app, Ok(autosaved()));
        let _ = app.update(Message::Command(Command::New));
        let _ = app.update(Message::Unsaved(Choice::DontSave));
        assert_eq!(folder.sessions(), 0);
    }

    #[test]
    fn closing_the_window_ends_the_session() {
        let folder = Folder::new();
        let mut app = recovering(&folder);
        change(&mut app);
        let _ = tick(&mut app);
        let _ = copied(&mut app, Ok(autosaved()));
        let _ = app.update(Message::CloseRequested(iced::window::Id::unique()));
        let _ = app.update(Message::Unsaved(Choice::DontSave));
        assert_eq!(folder.sessions(), 0);
    }
}
