use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{io, process};

use serde::{Deserialize, Serialize};

use crate::save::write_through_temporary;
use crate::{EXTENSION, Location};

const LOCK: &str = "lock";
const NOTE: &str = "session.json";
const UNTITLED: &str = "Untitled";

#[derive(Debug)]
pub struct Session {
    folder: PathBuf,
    _lock: File,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Note {
    project: Option<PathBuf>,
    copy: Option<PathBuf>,
}

impl Session {
    pub fn start(recovery: &Path) -> io::Result<Self> {
        static STARTED: AtomicU64 = AtomicU64::new(0);
        let since_1970 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |time| time.as_nanos());
        let folder = recovery.join(format!(
            "session-{}-{since_1970}-{}",
            process::id(),
            STARTED.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(recovery)?;
        fs::create_dir(&folder)?;
        Self::lock(folder)
    }

    fn lock(folder: PathBuf) -> io::Result<Self> {
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(folder.join(LOCK))?;
        lock.try_lock()?;
        Ok(Self {
            folder,
            _lock: lock,
        })
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    pub fn untitled(&self) -> Location {
        untitled_in(&self.folder)
    }

    pub fn note(&self, project: Option<&Location>, copy: Option<&Path>) -> io::Result<()> {
        let note = Note {
            project: project.map(|location| location.file().to_path_buf()),
            copy: copy.map(Path::to_path_buf),
        };
        let text = serde_json::to_string_pretty(&note).map_err(io::Error::other)?;
        write_through_temporary(&self.folder.join(NOTE), |temporary| {
            fs::write(temporary, &text)
        })
    }

    pub fn clear_untitled(&self) -> io::Result<()> {
        let untitled = self.untitled();
        for folder in [untitled.audio(), untitled.autosave()] {
            match fs::remove_dir_all(folder) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
                _ => {}
            }
        }
        Ok(())
    }

    pub fn end(self) -> io::Result<()> {
        let Self {
            folder,
            _lock: lock,
        } = self;
        drop(lock);
        fs::remove_dir_all(folder)
    }
}

#[derive(Debug)]
pub struct Orphan {
    folder: PathBuf,
    project: Option<Location>,
    copy: Option<PathBuf>,
}

impl Orphan {
    pub fn copy(&self) -> Option<&Path> {
        self.copy.as_deref()
    }

    pub fn project(&self) -> Option<&Location> {
        self.project.as_ref()
    }

    pub fn location(&self) -> Location {
        self.project
            .clone()
            .unwrap_or_else(|| untitled_in(&self.folder))
    }

    pub fn adopt(self) -> io::Result<Session> {
        Session::lock(self.folder)
    }

    pub fn discard(self) -> io::Result<()> {
        fs::remove_dir_all(self.folder)
    }
}

pub fn orphans(recovery: &Path) -> Vec<Orphan> {
    let Ok(entries) = fs::read_dir(recovery) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|folder| folder.is_dir() && is_unlocked(folder))
        .map(read_orphan)
        .collect()
}

fn is_unlocked(folder: &Path) -> bool {
    File::options()
        .read(true)
        .write(true)
        .open(folder.join(LOCK))
        .map_or(true, |lock| lock.try_lock().is_ok())
}

fn read_orphan(folder: PathBuf) -> Orphan {
    let note: Note = fs::read_to_string(folder.join(NOTE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    Orphan {
        project: note.project.map(Location::of_file),
        copy: note.copy.filter(|copy| copy.exists()),
        folder,
    }
}

fn untitled_in(folder: &Path) -> Location {
    Location::of_file(folder.join(format!("{UNTITLED}.{EXTENSION}")))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use sibilant_core::project::Project;

    use crate::scratch::Scratch;
    use crate::{Copies, Location, Session, autosave, orphans};

    fn folders_in(folder: &Path) -> Vec<PathBuf> {
        fs::read_dir(folder)
            .map_or_default(|entries| entries.map(|entry| entry.unwrap().path()).collect())
    }

    fn my_song(scratch: &Scratch) -> Location {
        Location::of_file(scratch.path().join("MySong").join("MySong.sibilant"))
    }

    #[test]
    fn a_running_session_is_not_an_orphan() {
        let scratch = Scratch::new();
        let _session = Session::start(scratch.path()).unwrap();
        assert!(orphans(scratch.path()).is_empty());
    }

    #[test]
    fn two_sessions_get_their_own_folders() {
        let scratch = Scratch::new();
        let first = Session::start(scratch.path()).unwrap();
        let second = Session::start(scratch.path()).unwrap();
        assert_ne!(first.folder(), second.folder());
    }

    #[test]
    fn a_session_that_ends_leaves_nothing_behind() {
        let scratch = Scratch::new();
        let session = Session::start(scratch.path()).unwrap();
        session.end().unwrap();
        assert_eq!(folders_in(scratch.path()), Vec::<PathBuf>::new());
    }

    #[test]
    fn a_session_of_a_stopped_program_is_an_orphan_with_its_copy() {
        let scratch = Scratch::new();
        let location = my_song(&scratch);
        let copy = scratch.file("MySong/autosave/MySong 7.sibilant", b"{}");
        let root = scratch.path().join("Recovery");
        let session = Session::start(&root).unwrap();
        session.note(Some(&location), Some(&copy)).unwrap();
        drop(session);
        let found = orphans(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].copy(), Some(copy.as_path()));
        assert_eq!(found[0].project(), Some(&location));
        assert_eq!(found[0].location(), location);
    }

    #[test]
    fn an_orphan_whose_copy_is_gone_has_nothing_to_restore() {
        let scratch = Scratch::new();
        let location = my_song(&scratch);
        let session = Session::start(scratch.path()).unwrap();
        let gone = location.autosave().join("MySong 7.sibilant");
        session.note(Some(&location), Some(&gone)).unwrap();
        drop(session);
        assert_eq!(orphans(scratch.path())[0].copy(), None);
    }

    #[test]
    fn a_session_folder_without_a_lock_file_is_an_orphan() {
        let scratch = Scratch::new();
        fs::create_dir(scratch.path().join("session-1")).unwrap();
        scratch.file("stray.txt", b"stray");
        assert_eq!(orphans(scratch.path()).len(), 1);
    }

    #[test]
    fn an_orphan_without_a_copy_has_nothing_to_restore() {
        let scratch = Scratch::new();
        drop(Session::start(scratch.path()).unwrap());
        let found = orphans(scratch.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].copy(), None);
    }

    #[test]
    fn an_untitled_project_restores_from_the_folder_of_its_session() {
        let scratch = Scratch::new();
        let session = Session::start(scratch.path()).unwrap();
        let untitled = session.untitled();
        assert_eq!(untitled.folder(), session.folder());
        session
            .note(None, Some(&untitled.autosave().join("Untitled 7.sibilant")))
            .unwrap();
        drop(session);
        let found = orphans(scratch.path());
        assert_eq!(found[0].project(), None);
        assert_eq!(found[0].location(), untitled);
    }

    #[test]
    fn an_adopted_orphan_is_a_running_session_again() {
        let scratch = Scratch::new();
        drop(Session::start(scratch.path()).unwrap());
        let orphan = orphans(scratch.path()).remove(0);
        let folder = orphan.location().folder().to_path_buf();
        let session = orphan.adopt().unwrap();
        assert_eq!(session.folder(), folder);
        assert!(orphans(scratch.path()).is_empty());
    }

    #[test]
    fn discard_removes_the_folder_of_an_orphan() {
        let scratch = Scratch::new();
        drop(Session::start(scratch.path()).unwrap());
        orphans(scratch.path()).remove(0).discard().unwrap();
        assert_eq!(folders_in(scratch.path()), Vec::<PathBuf>::new());
    }

    #[test]
    fn clear_untitled_removes_the_copies_and_the_audio_of_an_untitled_project() {
        let scratch = Scratch::new();
        let kick = scratch.file("samples/Kick.wav", b"kick");
        let root = scratch.path().join("Recovery");
        let session = Session::start(&root).unwrap();
        let untitled = session.untitled();
        let source = sibilant_core::project::AudioSource::new(
            sibilant_core::project::SourceId::new(1),
            &kick,
            sibilant_core::time::Duration::from_seconds(1),
        );
        let project = Project::default().add_source(source).unwrap();
        autosave(&project, &untitled, &Copies::default(), 7).unwrap();
        session.clear_untitled().unwrap();
        assert!(!untitled.audio().exists());
        assert!(!untitled.autosave().exists());
        assert!(session.folder().exists());
    }
}
