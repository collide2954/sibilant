use std::path::PathBuf;

use iced::Task;
use sibilant_core::history::History;
use sibilant_core::project::{Ids, Project, SourceId};
use sibilant_engine::engine;
use sibilant_project::{Copies, Location, Opened, SaveError};
use sibilant_ui::timeline::Viewport;

use super::document::{Document, DocumentId};
use super::{App, Message, dialogs};
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
) -> Result<Saved, SaveError> {
    let location = match target {
        Target::Current(location) => location,
        Target::Chosen(chosen) => Location::for_save_as(&chosen)?,
    };
    let copies = sibilant_project::save(project, &location, copies)?;
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
        if self.saving {
            return Task::none();
        }
        self.saving = true;
        let document = self.document.id();
        let project = self.history.present().clone();
        let copies = self.document.copies().clone();
        Task::perform(
            async move {
                let result = write(&project, target, &copies).map_err(|error| error.to_string());
                Box::new(Saving {
                    document,
                    project,
                    result,
                })
            },
            Message::Saved,
        )
    }

    pub(super) fn finish_saving(&mut self, saving: Saving) {
        self.saving = false;
        match saving.result {
            Ok(saved) if saving.document == self.document.id() => {
                self.document
                    .saved(saving.project, saved.location, saved.copies);
            }
            Ok(_) => {}
            Err(error) => self
                .browser
                .set_notice(Some(format!("Sibilant did not save the project: {error}"))),
        }
    }

    pub(super) fn finish_opening(&mut self, result: Result<Opened, String>) -> Task<Message> {
        match result {
            Ok(opened) => {
                self.start_document(opened.project, Some(opened.location));
                self.load_sources()
            }
            Err(error) => {
                self.browser
                    .set_notice(Some(format!("Sibilant did not open the project: {error}")));
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
            self.browser.set_notice(Some(format!(
                "These audio files are missing or damaged: {}",
                names.join(", ")
            )));
        }
    }

    pub(super) fn start_new_project(&mut self) {
        self.start_document(Project::new(&mut Ids::default()), None);
    }

    fn start_document(&mut self, project: Project, location: Option<Location>) {
        self.ids = Ids::after(&project);
        self.document = Document::new(&project, location);
        self.history = History::new(project);
        self.viewport = Viewport::default();
        self.pool.clear();
        self.waveforms.clear();
        self.missing.clear();
        self.browser.set_notice(None);
        self.send(engine::Command::Stop);
        self.sync();
    }

    fn load_sources(&self) -> Task<Message> {
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
    use sibilant_project::Copies;

    use crate::app::file::{Target, write};
    use crate::library::tests::write_tone;

    #[test]
    fn write_saves_into_a_chosen_place_and_then_into_the_same_folder() {
        let kick = write_tone("file-kick.wav", 48_000, 480);
        let chosen = kick.with_file_name("Saved Song.sibilant");
        let source = AudioSource::new(SourceId::new(1), &kick, Duration::from_seconds(1));
        let project = Project::default().add_source(source).unwrap();
        let saved = write(&project, Target::Chosen(chosen), &Copies::default()).unwrap();
        assert_eq!(saved.location.name(), "Saved Song");
        assert!(saved.location.audio().join("file-kick.wav").exists());
        let again = write(
            &project,
            Target::Current(saved.location.clone()),
            &saved.copies,
        );
        fs::remove_dir_all(saved.location.folder()).unwrap();
        assert_eq!(again, Ok(saved));
    }
}
