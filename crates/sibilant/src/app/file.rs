use std::path::PathBuf;

use iced::Task;
use sibilant_core::project::Project;
use sibilant_project::{Copies, Location, SaveError};

use super::{App, Message, dialogs};

#[derive(Debug, Clone)]
pub(crate) enum Target {
    Current(Location),
    Chosen(PathBuf),
}

#[derive(Debug, Clone)]
pub(crate) struct Saving {
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
        let project = self.history.present().clone();
        let copies = self.document.copies().clone();
        Task::perform(
            async move {
                let result = write(&project, target, &copies).map_err(|error| error.to_string());
                Box::new(Saving { project, result })
            },
            Message::Saved,
        )
    }

    pub(super) fn finish_saving(&mut self, saving: Saving) {
        self.saving = false;
        match saving.result {
            Ok(saved) => self
                .document
                .saved(saving.project, saved.location, saved.copies),
            Err(error) => self
                .browser
                .set_notice(Some(format!("Sibilant did not save the project: {error}"))),
        }
    }
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
