use std::path::PathBuf;

use iced::Task;
use sibilant_core::history::History;
use sibilant_core::project::{Ids, Project, SourceId};
use sibilant_core::time::Position;
use sibilant_engine::engine;
use sibilant_project::{Copies, Location, Opened, SaveError, SessionFolder};
use sibilant_ui::timeline::Viewport;

use super::document::{Document, DocumentId};
use super::{App, Message, Writing, dialogs};
use crate::audio::Audio;
use crate::library::{self, LoadedAudio};

#[derive(Debug, Clone)]
pub(crate) enum Target {
    Current(Location),
    Chosen(PathBuf),
}

#[derive(Debug, Clone)]
pub(crate) struct Saving {
    pub(crate) document: DocumentId,
    pub(crate) project: Project,
    pub(crate) result: Result<Saved, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Saved {
    pub(crate) location: Location,
    pub(crate) copies: Copies,
}

pub(crate) fn write(
    project: &Project,
    target: Target,
    copies: &Copies,
    session: Option<&SessionFolder>,
) -> Result<Saved, SaveError> {
    let location = match target {
        Target::Current(location) => location,
        Target::Chosen(chosen) => Location::for_save_as(&chosen)?,
    };
    let copies = sibilant_project::save(project, &location, copies)?;
    if let Some(session) = session {
        let _ = session.clear_untitled();
    }
    Ok(Saved { location, copies })
}

pub(super) fn open(file: PathBuf) -> Task<Message> {
    Task::perform(
        async move { sibilant_project::open(&file).map_err(|error| error.to_string()) },
        |result| Message::Opened(Box::new(result)),
    )
}

impl App {
    pub(super) fn save(&mut self) -> Task<Message> {
        match self.document.location().cloned() {
            Some(location) => self.save_to(Target::Current(location)),
            None => self.save_as(),
        }
    }

    pub(super) fn save_as(&self) -> Task<Message> {
        dialogs::save_file(&self.document.name()).map(Message::SaveAs)
    }

    pub(super) fn save_to(&mut self, target: Target) -> Task<Message> {
        match self.writing {
            Some(Writing::Save) => return Task::none(),
            Some(Writing::Autosave) => {
                self.queued_save = Some(target);
                return Task::none();
            }
            None => self.writing = Some(Writing::Save),
        }
        let document = self.document.id();
        let project = self.history.present().clone();
        let copies = self.document.copies().clone();
        let session = self.recovery.folder();
        Task::perform(
            async move {
                let result = write(&project, target, &copies, session.as_ref())
                    .map_err(|error| error.to_string());
                Box::new(Saving {
                    document,
                    project,
                    result,
                })
            },
            Message::Saved,
        )
    }

    pub(super) fn finish_saving(&mut self, saving: Saving) -> Task<Message> {
        self.writing = None;
        match saving.result {
            Ok(saved) if saving.document == self.document.id() => {
                self.document
                    .saved(saving.project, saved.location, saved.copies);
            }
            Ok(_) => {}
            Err(error) => {
                self.waiting = None;
                self.browser
                    .show_problem(format!("Sibilant did not save the project: {error}"));
            }
        }
        self.resume_leaving()
    }

    pub(super) fn finish_opening(&mut self, result: Result<Opened, String>) -> Task<Message> {
        match result {
            Ok(opened) => {
                let document = Document::new(&opened.project, Some(opened.location));
                self.start_document(opened.project, document);
                self.load_sources()
            }
            Err(error) => {
                self.browser
                    .show_problem(format!("Sibilant did not open the project: {error}"));
                Task::none()
            }
        }
    }

    pub(super) fn source_loaded(
        &mut self,
        document: DocumentId,
        source: SourceId,
        result: Result<LoadedAudio, String>,
    ) {
        if document != self.document.id() {
            return;
        }
        if let Ok(loaded) = result {
            self.pool.insert(source, loaded.playback);
            self.waveforms.insert(source, loaded.peaks);
            self.sync();
        } else {
            self.missing
                .extend(file_name(self.history.present(), source));
            let names: Vec<&str> = self.missing.iter().map(String::as_str).collect();
            self.browser.show_problem(format!(
                "These audio files are missing or damaged: {}",
                names.join(", ")
            ));
        }
    }

    pub(super) fn start_new_project(&mut self) {
        let project = Project::new(&mut Ids::default());
        let document = Document::new(&project, None);
        self.start_document(project, document);
    }

    pub(super) fn start_document(&mut self, project: Project, document: Document) {
        self.recovery.end();
        self.ids = Ids::after(&project);
        self.document = document;
        self.history = History::new(project);
        self.viewport = Viewport::default();
        self.pool.clear();
        self.waveforms.clear();
        self.missing.clear();
        self.selection.clear();
        self.gesture = None;
        self.preview = None;
        self.browser.clear_notice();
        self.send(engine::Command::Stop);
        self.locate(Position::START);
        self.sync();
    }

    pub(super) fn load_sources(&self) -> Task<Message> {
        let document = self.document.id();
        let sample_rate = self.audio.as_ref().map(Audio::sample_rate);
        Task::batch(self.history.present().sources().map(|source| {
            let (id, path) = (source.id(), source.path().to_path_buf());
            Task::perform(
                async move { library::load(&path, sample_rate) },
                move |result| Message::SourceLoaded(document, id, result),
            )
        }))
    }
}

fn file_name(project: &Project, source: SourceId) -> Option<String> {
    let path = project.source(source)?.path();
    Some(path.file_name()?.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use sibilant_core::project::{AudioSource, Project, SourceId};
    use sibilant_core::time::Duration;
    use sibilant_project::{Copies, Session};

    use crate::app::autosave;
    use crate::app::file::{Target, write};
    use crate::library::tests::write_tone;

    #[test]
    fn write_saves_into_a_chosen_place_and_then_into_the_same_folder() {
        let kick = write_tone("file-kick.wav", 48_000, 480);
        let chosen = kick.with_file_name("Saved Song.sibilant");
        let source = AudioSource::new(SourceId::new(1), &kick, Duration::from_seconds(1));
        let project = Project::default().add_source(source).unwrap();
        let saved = write(&project, Target::Chosen(chosen), &Copies::default(), None).unwrap();
        assert_eq!(saved.location.name(), "Saved Song");
        assert!(saved.location.audio().join("file-kick.wav").exists());
        let again = write(
            &project,
            Target::Current(saved.location.clone()),
            &saved.copies,
            None,
        );
        fs::remove_dir_all(saved.location.folder()).unwrap();
        assert_eq!(again, Ok(saved));
    }

    #[test]
    fn write_removes_the_recovery_copies_of_an_untitled_project() {
        let kick = write_tone("file-untitled-kick.wav", 48_000, 480);
        let session = Session::start(&kick.with_file_name("file-recovery")).unwrap();
        let source = AudioSource::new(SourceId::new(1), &kick, Duration::from_seconds(1));
        let project = Project::default().add_source(source).unwrap();
        let copy = autosave::write(&project, None, &Copies::default(), session.folder(), 7);
        let chosen = kick.with_file_name("Untitled Song.sibilant");
        let saved = write(
            &project,
            Target::Chosen(chosen),
            &copy.unwrap().copies,
            Some(session.folder()),
        )
        .unwrap();
        let untitled = session.folder().untitled();
        let gone = !untitled.autosave().exists() && !untitled.audio().exists();
        let copied = saved
            .location
            .audio()
            .join("file-untitled-kick.wav")
            .exists();
        session.end().unwrap();
        fs::remove_dir_all(saved.location.folder()).unwrap();
        assert!(gone && copied);
    }
}
