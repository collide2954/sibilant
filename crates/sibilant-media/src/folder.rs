use std::fs::{self, DirEntry};
use std::io;
use std::path::{Path, PathBuf};

use crate::can_read;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntryKind {
    Folder,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub name: String,
    pub path: PathBuf,
}

pub fn list_folder(folder: &Path) -> io::Result<Vec<Entry>> {
    let mut entries: Vec<Entry> = fs::read_dir(folder)?
        .filter_map(Result::ok)
        .filter(|entry| !is_hidden(entry))
        .filter_map(|entry| {
            let path = entry.path();
            let kind = if path.is_dir() {
                EntryKind::Folder
            } else if can_read(&path) {
                EntryKind::Audio
            } else {
                return None;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            Some(Entry { name, path, kind })
        })
        .collect();
    entries.sort_by_cached_key(|entry| (entry.kind, entry.name.to_lowercase()));
    Ok(entries)
}

pub fn places() -> Vec<Place> {
    let home = std::env::home_dir();
    let user_dirs = home
        .as_deref()
        .and_then(|home| fs::read_to_string(config_folder(home).join("user-dirs.dirs")).ok());
    known_places(home.as_deref(), user_dirs.as_deref())
        .into_iter()
        .chain(roots())
        .filter(|place| place.path.is_dir())
        .collect()
}

fn known_places(home: Option<&Path>, user_dirs: Option<&str>) -> Vec<Place> {
    let Some(home) = home else {
        return Vec::new();
    };
    let folder = |key: &str, usual: &str| {
        user_dirs
            .and_then(|user_dirs| user_dir(home, user_dirs, key))
            .unwrap_or_else(|| home.join(usual))
    };
    vec![
        place("Home", home),
        place("Music", folder("XDG_MUSIC_DIR", "Music")),
        place("Desktop", folder("XDG_DESKTOP_DIR", "Desktop")),
        place("Downloads", folder("XDG_DOWNLOAD_DIR", "Downloads")),
    ]
}

fn user_dir(home: &Path, user_dirs: &str, key: &str) -> Option<PathBuf> {
    user_dirs.lines().find_map(|line| {
        let value = line
            .trim()
            .strip_prefix(key)?
            .trim_start()
            .strip_prefix('=')?
            .trim()
            .trim_matches('"');
        Some(match value.strip_prefix("$HOME") {
            Some(rest) => home.join(rest.trim_start_matches('/')),
            None => PathBuf::from(value),
        })
    })
}

fn config_folder(home: &Path) -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".config"))
}

#[cfg(windows)]
fn roots() -> Vec<Place> {
    ('C'..='Z')
        .map(|drive| place(&format!("{drive}:"), format!("{drive}:\\")))
        .collect()
}

#[cfg(not(windows))]
fn roots() -> Vec<Place> {
    vec![place("Computer", "/")]
}

fn place(name: &str, path: impl Into<PathBuf>) -> Place {
    Place {
        name: name.to_owned(),
        path: path.into(),
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with('.') || has_hidden_attribute(entry)
}

#[cfg(windows)]
fn has_hidden_attribute(entry: &DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const HIDDEN: u32 = 0x2;
    entry
        .metadata()
        .is_ok_and(|metadata| metadata.file_attributes() & HIDDEN != 0)
}

#[cfg(not(windows))]
fn has_hidden_attribute(_entry: &DirEntry) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::known_places;
    use crate::{Entry, EntryKind, Place, list_folder, places};

    struct Folder(PathBuf);

    impl Folder {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("sibilant-folder-{}-{name}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn file(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, b"").unwrap();
            path
        }

        fn folder(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::create_dir(&path).unwrap();
            path
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn names(entries: &[Entry]) -> Vec<(&str, EntryKind)> {
        entries
            .iter()
            .map(|entry| (entry.name.as_str(), entry.kind))
            .collect()
    }

    #[test]
    fn a_folder_lists_its_folders_first_and_then_its_audio_files_by_name() {
        let folder = Folder::new("order");
        folder.file("b.wav");
        folder.file("A.WAV");
        folder.folder("Zeta");
        folder.folder("alpha");
        let entries = list_folder(&folder.0).unwrap();
        assert_eq!(
            names(&entries),
            [
                ("alpha", EntryKind::Folder),
                ("Zeta", EntryKind::Folder),
                ("A.WAV", EntryKind::Audio),
                ("b.wav", EntryKind::Audio),
            ]
        );
        assert_eq!(entries[3].path, folder.0.join("b.wav"));
    }

    #[test]
    fn a_folder_does_not_list_hidden_entries_or_files_that_sibilant_cannot_read() {
        let folder = Folder::new("hidden");
        folder.file(".kick.wav");
        folder.folder(".git");
        folder.file("notes.txt");
        folder.file("kick.wav");
        assert_eq!(
            names(&list_folder(&folder.0).unwrap()),
            [("kick.wav", EntryKind::Audio)]
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_folder_does_not_list_entries_that_windows_hides() {
        let folder = Folder::new("attribute");
        let hidden = folder.file("hidden.wav");
        let status = std::process::Command::new("attrib")
            .arg("+h")
            .arg(&hidden)
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(list_folder(&folder.0).unwrap(), []);
    }

    #[test]
    fn a_missing_folder_is_an_error() {
        assert!(list_folder(Path::new("there-is-no-such-folder")).is_err());
    }

    fn place(name: &str, path: impl Into<PathBuf>) -> Place {
        Place {
            name: name.to_owned(),
            path: path.into(),
        }
    }

    #[test]
    fn the_known_places_are_folders_in_the_home_folder() {
        let home = Path::new("/home/ada");
        assert_eq!(
            known_places(Some(home), None),
            [
                place("Home", home),
                place("Music", home.join("Music")),
                place("Desktop", home.join("Desktop")),
                place("Downloads", home.join("Downloads")),
            ]
        );
    }

    #[test]
    fn the_xdg_user_folders_replace_the_usual_names() {
        let home = Path::new("/home/ada");
        let user_dirs = "# written by xdg-user-dirs-update\n\
            XDG_MUSIC_DIR=\"$HOME/Musik\"\n\
            XDG_PICTURES_DIR=\"$HOME/Bilder\"\n\
            XDG_DOWNLOAD_DIR=\"/data/downloads\"\n";
        let places = known_places(Some(home), Some(user_dirs));
        assert_eq!(places[1], place("Music", home.join("Musik")));
        assert_eq!(places[2], place("Desktop", home.join("Desktop")));
        assert_eq!(places[3], place("Downloads", "/data/downloads"));
    }

    #[test]
    fn without_a_home_folder_there_are_no_known_places() {
        assert_eq!(known_places(None, None), []);
    }

    #[test]
    fn each_place_is_a_folder_on_this_computer() {
        let places = places();
        assert_ne!(places, []);
        assert!(places.iter().all(|place| place.path.is_dir()));
    }
}
