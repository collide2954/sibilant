use std::error::Error;
use std::path::Path;
use std::{fmt, fs};

use sibilant_core::project::Project;

use crate::{Location, format};

#[derive(Debug, Clone, PartialEq)]
pub struct Opened {
    pub project: Project,
    pub location: Location,
}

pub fn open(file: &Path) -> Result<Opened, OpenError> {
    let location = Location::of_file(file);
    let project = read_project(file, &location)?;
    Ok(Opened { project, location })
}

pub(crate) fn read_project(file: &Path, location: &Location) -> Result<Project, OpenError> {
    let text = fs::read_to_string(file).map_err(|error| OpenError::File(error.to_string()))?;
    format::read(&text, |stored| location.resolve(stored))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    Content(String),
    File(String),
    Newer(u32),
    NotProject(String),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Content(error) => write!(f, "the project file has an error: {error}"),
            Self::File(error) => write!(f, "cannot read the project file: {error}"),
            Self::Newer(format) => write!(
                f,
                "the project file needs a newer version of Sibilant (format {format})"
            ),
            Self::NotProject(error) => write!(f, "the file is not a Sibilant project: {error}"),
        }
    }
}

impl Error for OpenError {}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sibilant_core::project::{AudioSource, Clip, Project, Region, SourceId, TrackId};
    use sibilant_core::time::{Duration, Position};

    use crate::scratch::Scratch;
    use crate::{Copies, Location, OpenError, open, save};

    fn project(kick: &Path) -> Project {
        let source = SourceId::new(1);
        let track = TrackId::new(2);
        let region = Region {
            start: Duration::ZERO,
            length: Duration::from_seconds(1),
        };
        Project::default()
            .add_source(AudioSource::new(source, kick, Duration::from_seconds(1)))
            .and_then(|project| project.add_track(track, "Drums"))
            .and_then(|project| {
                project.place_clip(track, Position::START, Clip::new("Kick", source, region))
            })
            .unwrap()
    }

    #[test]
    fn a_saved_project_opens_with_its_audio_in_the_project_folder() {
        let scratch = Scratch::new();
        let kick = scratch.file("samples/Kick.wav", b"kick");
        let location = Location::of_file(scratch.path().join("MySong").join("MySong.sibilant"));
        save(&project(&kick), &location, &Copies::default()).unwrap();
        let opened = open(location.file()).unwrap();
        assert_eq!(opened.project, project(&location.audio().join("Kick.wav")));
        assert_eq!(opened.location, location);
    }

    #[test]
    fn a_missing_project_file_tells_why() {
        let scratch = Scratch::new();
        let result = open(&scratch.path().join("MySong.sibilant"));
        assert!(matches!(result, Err(OpenError::File(_))));
    }

    #[test]
    fn the_errors_tell_why_the_project_cannot_be_opened() {
        let messages = [
            (
                OpenError::Content("track 1 does not exist".into()),
                "the project file has an error: track 1 does not exist",
            ),
            (
                OpenError::File("no file".into()),
                "cannot read the project file: no file",
            ),
            (
                OpenError::Newer(2),
                "the project file needs a newer version of Sibilant (format 2)",
            ),
            (
                OpenError::NotProject("expected value".into()),
                "the file is not a Sibilant project: expected value",
            ),
        ];
        for (error, message) in messages {
            assert_eq!(error.to_string(), message);
        }
    }
}
