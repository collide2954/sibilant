use std::fs;
use std::path::{Path, PathBuf};

use sibilant_core::project::Project;

use crate::open::read_project;
use crate::save::write_project;
use crate::{Copies, EXTENSION, Location, OpenError, SaveError};

const KEPT: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Autosaved {
    pub file: PathBuf,
    pub copies: Copies,
}

pub fn autosave(
    project: &Project,
    location: &Location,
    copies: &Copies,
    time: u64,
) -> Result<Autosaved, SaveError> {
    let folder = location.autosave();
    fs::create_dir_all(&folder).map_err(|error| SaveError::Folder(error.to_string()))?;
    let file = folder.join(format!("{} {time}.{EXTENSION}", location.name()));
    let copies = write_project(project, location, copies, &file)?;
    remove_old_copies(&folder);
    Ok(Autosaved { file, copies })
}

pub fn restore(file: &Path, location: &Location) -> Result<Project, OpenError> {
    read_project(file, location)
}

fn remove_old_copies(folder: &Path) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    let mut copies: Vec<(u64, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter_map(|path| Some((time_of(&path)?, path)))
        .collect();
    copies.sort();
    for (_, old) in copies.iter().rev().skip(KEPT) {
        let _ = fs::remove_file(old);
    }
}

fn time_of(copy: &Path) -> Option<u64> {
    if copy.extension()? != EXTENSION {
        return None;
    }
    copy.file_stem()?.to_str()?.rsplit(' ').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use sibilant_core::project::{AudioSource, Project, SourceId, TrackId};
    use sibilant_core::time::Duration;

    use crate::scratch::Scratch;
    use crate::{Copies, Location, autosave, open, restore, save};

    fn location(scratch: &Scratch) -> Location {
        Location::of_file(scratch.path().join("MySong").join("MySong.sibilant"))
    }

    fn with_kick(kick: &Path) -> Project {
        let source = AudioSource::new(SourceId::new(1), kick, Duration::from_seconds(1));
        Project::default().add_source(source).unwrap()
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
    fn autosave_writes_a_copy_and_leaves_the_project_file_alone() {
        let scratch = Scratch::new();
        let location = location(&scratch);
        save(&Project::default(), &location, &Copies::default()).unwrap();
        let changed = Project::default()
            .add_track(TrackId::new(1), "Drums")
            .unwrap();
        let autosaved = autosave(&changed, &location, &Copies::default(), 1_795_100_000).unwrap();
        assert_eq!(
            autosaved.file,
            location.autosave().join("MySong 1795100000.sibilant")
        );
        assert_eq!(open(location.file()).unwrap().project, Project::default());
        assert_eq!(restore(&autosaved.file, &location).unwrap(), changed);
    }

    #[test]
    fn a_copy_restores_with_the_audio_that_autosave_copied_into_the_project() {
        let scratch = Scratch::new();
        let kick = scratch.file("samples/Kick.wav", b"kick");
        let location = location(&scratch);
        let autosaved = autosave(&with_kick(&kick), &location, &Copies::default(), 7).unwrap();
        let copied = location.audio().join("Kick.wav");
        assert_eq!(fs::read(&copied).unwrap(), b"kick");
        assert_eq!(
            restore(&autosaved.file, &location).unwrap(),
            with_kick(&copied)
        );
    }

    #[test]
    fn save_does_not_copy_again_what_autosave_copied() {
        let scratch = Scratch::new();
        let kick = scratch.file("samples/Kick.wav", b"kick");
        let location = location(&scratch);
        let project = with_kick(&kick);
        let autosaved = autosave(&project, &location, &Copies::default(), 7).unwrap();
        save(&project, &location, &autosaved.copies).unwrap();
        assert_eq!(names_in(&location.audio()), ["Kick.wav"]);
    }

    #[test]
    fn autosave_keeps_the_10_newest_copies_and_other_files() {
        let scratch = Scratch::new();
        let location = location(&scratch);
        let take = scratch.file("MySong/autosave/Take 1.wav", b"take");
        for time in 1..=12 {
            autosave(&Project::default(), &location, &Copies::default(), time).unwrap();
        }
        let mut expected: Vec<String> = (3..=12)
            .map(|time| format!("MySong {time}.sibilant"))
            .collect();
        expected.push("Take 1.wav".into());
        expected.sort();
        assert_eq!(names_in(&location.autosave()), expected);
        assert!(take.exists());
    }
}
