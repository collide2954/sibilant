use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::{fmt, io};

use sibilant_core::project::Project;

use crate::Location;
use crate::format;
use crate::location::AUDIO;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Copies(BTreeMap<PathBuf, PathBuf>);

pub fn save(project: &Project, location: &Location, copies: &Copies) -> Result<Copies, SaveError> {
    fs::create_dir_all(location.audio()).map_err(|error| SaveError::Folder(error.to_string()))?;
    let mut copies = copies.clone();
    let files = project
        .sources()
        .map(|source| Ok((source.id(), copies.file_of(source.path(), location)?)))
        .collect::<Result<BTreeMap<_, _>, SaveError>>()?;
    let text = format::write(project, |source| files[&source.id()].clone());
    write_through_temporary(location.file(), |temporary| fs::write(temporary, &text))
        .map_err(|error| SaveError::Write(error.to_string()))?;
    Ok(copies)
}

impl Copies {
    fn file_of(&mut self, original: &Path, location: &Location) -> Result<String, SaveError> {
        let audio = location.audio();
        let inside = |path: &Path| {
            path.starts_with(&audio)
                .then(|| location.relative(path))
                .flatten()
        };
        let kept = self
            .0
            .get(original)
            .filter(|copy| copy.exists())
            .and_then(|copy| inside(copy));
        if let Some(file) = inside(original).or(kept) {
            return Ok(file);
        }
        let name = copy_into(&audio, original)
            .map_err(|error| SaveError::Copy(display_name(original), error.to_string()))?;
        self.0.insert(original.to_path_buf(), audio.join(&name));
        Ok(format!("{AUDIO}/{name}"))
    }
}

fn copy_into(folder: &Path, original: &Path) -> io::Result<String> {
    let name = free_name(folder, &display_name(original));
    write_through_temporary(&folder.join(&name), |temporary| {
        fs::copy(original, temporary).map(drop)
    })?;
    Ok(name)
}

fn free_name(folder: &Path, name: &str) -> String {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map_or_default(|stem| stem.to_string_lossy().into_owned());
    let extension = path
        .extension()
        .map_or_default(|extension| format!(".{}", extension.to_string_lossy()));
    let mut candidate = name.to_owned();
    let mut number = 1;
    while folder.join(&candidate).exists() {
        number += 1;
        candidate = format!("{stem} {number}{extension}");
    }
    candidate
}

fn display_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

fn write_through_temporary(
    target: &Path,
    write: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let temporary = target.with_file_name(format!(".{}.tmp", display_name(target)));
    let written = write(&temporary)
        .and_then(|()| OpenOptions::new().write(true).open(&temporary)?.sync_all())
        .and_then(|()| fs::rename(&temporary, target));
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    written
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    Copy(String, String),
    Folder(String),
    NameTaken(PathBuf),
    NoName,
    Write(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Copy(file, error) => write!(f, "cannot copy {file} into the project: {error}"),
            Self::Folder(error) => write!(f, "cannot make the project folder: {error}"),
            Self::NameTaken(folder) => write!(
                f,
                "{} already contains a project with this name",
                folder.display()
            ),
            Self::NoName => write!(f, "the project needs a name"),
            Self::Write(error) => write!(f, "cannot write the project file: {error}"),
        }
    }
}

impl Error for SaveError {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use serde_json::Value;
    use sibilant_core::project::{AudioSource, Project, SourceId};
    use sibilant_core::time::Duration;

    use crate::scratch::Scratch;
    use crate::{Copies, Location, SaveError, save};

    fn project_with(files: &[&Path]) -> Project {
        files
            .iter()
            .zip(1..)
            .fold(Project::default(), |project, (file, id)| {
                let source = AudioSource::new(SourceId::new(id), *file, Duration::from_seconds(1));
                project.add_source(source).unwrap()
            })
    }

    fn location(scratch: &Scratch) -> Location {
        Location::of_file(scratch.path().join("MySong").join("MySong.sibilant"))
    }

    fn stored_files(location: &Location) -> Vec<String> {
        let text = fs::read_to_string(location.file()).unwrap();
        let file: Value = serde_json::from_str(&text).unwrap();
        file["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| source["file"].as_str().unwrap().to_owned())
            .collect()
    }

    fn names_in(folder: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn save_writes_the_project_file_into_the_project_folder() {
        let scratch = Scratch::new();
        let location = location(&scratch);
        save(&Project::default(), &location, &Copies::default()).unwrap();
        let text = fs::read_to_string(location.file()).unwrap();
        let file: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(file["format"], 1);
    }

    #[test]
    fn save_copies_each_audio_file_into_the_audio_folder() {
        let scratch = Scratch::new();
        let kick = scratch.file("samples/Kick.wav", b"kick");
        let location = location(&scratch);
        save(&project_with(&[&kick]), &location, &Copies::default()).unwrap();
        assert_eq!(
            fs::read(location.audio().join("Kick.wav")).unwrap(),
            b"kick"
        );
        assert_eq!(stored_files(&location), ["audio/Kick.wav"]);
    }

    #[test]
    fn a_second_save_does_not_copy_a_file_again() {
        let scratch = Scratch::new();
        let kick = scratch.file("samples/Kick.wav", b"kick");
        let location = location(&scratch);
        let project = project_with(&[&kick]);
        let copies = save(&project, &location, &Copies::default()).unwrap();
        fs::write(&kick, b"new kick").unwrap();
        save(&project, &location, &copies).unwrap();
        assert_eq!(names_in(&location.audio()), ["Kick.wav"]);
        assert_eq!(
            fs::read(location.audio().join("Kick.wav")).unwrap(),
            b"kick"
        );
    }

    #[test]
    fn a_copy_gets_a_number_when_the_audio_folder_has_its_name() {
        let scratch = Scratch::new();
        let first = scratch.file("old/Kick.wav", b"old");
        let second = scratch.file("new/Kick.wav", b"new");
        let location = location(&scratch);
        save(
            &project_with(&[&first, &second]),
            &location,
            &Copies::default(),
        )
        .unwrap();
        assert_eq!(
            stored_files(&location),
            ["audio/Kick.wav", "audio/Kick 2.wav"]
        );
        assert_eq!(
            fs::read(location.audio().join("Kick 2.wav")).unwrap(),
            b"new"
        );
    }

    #[test]
    fn two_sources_of_one_file_share_one_copy() {
        let scratch = Scratch::new();
        let kick = scratch.file("samples/Kick.wav", b"kick");
        let location = location(&scratch);
        save(
            &project_with(&[&kick, &kick]),
            &location,
            &Copies::default(),
        )
        .unwrap();
        assert_eq!(
            stored_files(&location),
            ["audio/Kick.wav", "audio/Kick.wav"]
        );
        assert_eq!(names_in(&location.audio()), ["Kick.wav"]);
    }

    #[test]
    fn a_file_in_the_audio_folder_stays_where_it_is() {
        let scratch = Scratch::new();
        let kick = scratch.file("MySong/audio/Kick.wav", b"kick");
        let location = location(&scratch);
        let copies = save(&project_with(&[&kick]), &location, &Copies::default()).unwrap();
        assert_eq!(stored_files(&location), ["audio/Kick.wav"]);
        assert_eq!(copies, Copies::default());
    }

    #[test]
    fn a_deleted_copy_is_made_again() {
        let scratch = Scratch::new();
        let kick = scratch.file("samples/Kick.wav", b"kick");
        let location = location(&scratch);
        let project = project_with(&[&kick]);
        let copies = save(&project, &location, &Copies::default()).unwrap();
        fs::remove_file(location.audio().join("Kick.wav")).unwrap();
        save(&project, &location, &copies).unwrap();
        assert_eq!(
            fs::read(location.audio().join("Kick.wav")).unwrap(),
            b"kick"
        );
    }

    #[test]
    fn a_missing_audio_file_stops_the_save_and_gives_its_name() {
        let scratch = Scratch::new();
        let missing = scratch.path().join("samples").join("Kick.wav");
        let result = save(
            &project_with(&[&missing]),
            &location(&scratch),
            &Copies::default(),
        );
        assert!(matches!(result, Err(SaveError::Copy(file, _)) if file == "Kick.wav"));
    }

    #[test]
    fn save_leaves_no_temporary_files() {
        let scratch = Scratch::new();
        let kick = scratch.file("samples/Kick.wav", b"kick");
        let location = location(&scratch);
        save(&project_with(&[&kick]), &location, &Copies::default()).unwrap();
        save(&project_with(&[&kick]), &location, &Copies::default()).unwrap();
        assert_eq!(names_in(location.folder()), ["MySong.sibilant", "audio"]);
        assert_eq!(names_in(&location.audio()), ["Kick 2.wav", "Kick.wav"]);
    }

    #[test]
    fn the_errors_tell_why_the_project_cannot_be_saved() {
        let messages = [
            (
                SaveError::Copy("Kick.wav".into(), "no file".into()),
                "cannot copy Kick.wav into the project: no file",
            ),
            (
                SaveError::Folder("access denied".into()),
                "cannot make the project folder: access denied",
            ),
            (
                SaveError::NameTaken(PathBuf::from("MySong")),
                "MySong already contains a project with this name",
            ),
            (SaveError::NoName, "the project needs a name"),
            (
                SaveError::Write("disk full".into()),
                "cannot write the project file: disk full",
            ),
        ];
        for (error, message) in messages {
            assert_eq!(error.to_string(), message);
        }
    }
}
