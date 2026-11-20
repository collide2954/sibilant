use std::path::{Component, Path, PathBuf};

use crate::SaveError;

pub const EXTENSION: &str = "sibilant";
pub(crate) const AUDIO: &str = "audio";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    file: PathBuf,
}

impl Location {
    pub fn of_file(file: impl Into<PathBuf>) -> Self {
        Self { file: file.into() }
    }

    pub fn for_save_as(chosen: &Path) -> Result<Self, SaveError> {
        let name = project_name(chosen).ok_or(SaveError::NoName)?;
        let parent = chosen.parent().unwrap_or(Path::new(""));
        let file_name = format!("{name}.{EXTENSION}");
        if parent.file_name().is_some_and(|folder| *folder == *name) {
            return Ok(Self::of_file(parent.join(file_name)));
        }
        let folder = parent.join(&name);
        let file = folder.join(file_name);
        if file.exists() {
            return Err(SaveError::NameTaken(folder));
        }
        Ok(Self::of_file(file))
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    pub fn folder(&self) -> &Path {
        self.file.parent().unwrap_or(Path::new(""))
    }

    pub fn audio(&self) -> PathBuf {
        self.folder().join(AUDIO)
    }

    pub fn name(&self) -> String {
        self.file
            .file_stem()
            .map_or_default(|stem| stem.to_string_lossy().into_owned())
    }

    pub(crate) fn relative(&self, path: &Path) -> Option<String> {
        let parts = path
            .strip_prefix(self.folder())
            .ok()?
            .components()
            .map(|component| match component {
                Component::Normal(part) => Some(part.to_string_lossy()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        Some(parts.join("/"))
    }
}

fn project_name(chosen: &Path) -> Option<String> {
    let file_name = chosen.file_name()?.to_string_lossy();
    let suffix = format!(".{EXTENSION}");
    let name = match file_name.len().checked_sub(suffix.len()) {
        Some(cut)
            if file_name
                .get(cut..)
                .is_some_and(|end| end.eq_ignore_ascii_case(&suffix)) =>
        {
            &file_name[..cut]
        }
        _ => &file_name[..],
    };
    (!name.is_empty()).then(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::scratch::Scratch;
    use crate::{Location, SaveError};

    #[test]
    fn save_as_puts_the_project_file_in_a_new_folder_with_the_name_of_the_project() {
        let scratch = Scratch::new();
        let location = Location::for_save_as(&scratch.path().join("MySong.sibilant")).unwrap();
        let folder = scratch.path().join("MySong");
        assert_eq!(location.folder(), folder);
        assert_eq!(location.file(), folder.join("MySong.sibilant"));
        assert_eq!(location.name(), "MySong");
    }

    #[test]
    fn a_name_without_the_extension_gets_the_extension() {
        let scratch = Scratch::new();
        let location = Location::for_save_as(&scratch.path().join("MySong")).unwrap();
        let folder = scratch.path().join("MySong");
        assert_eq!(location.file(), folder.join("MySong.sibilant"));
    }

    #[test]
    fn the_extension_counts_in_any_case() {
        let scratch = Scratch::new();
        let location = Location::for_save_as(&scratch.path().join("MySong.SIBILANT")).unwrap();
        assert_eq!(location.name(), "MySong");
    }

    #[test]
    fn a_dot_in_the_name_stays_in_the_name() {
        let scratch = Scratch::new();
        let location = Location::for_save_as(&scratch.path().join("My.Song")).unwrap();
        let folder = scratch.path().join("My.Song");
        assert_eq!(location.file(), folder.join("My.Song.sibilant"));
    }

    #[test]
    fn a_project_file_in_a_folder_with_its_name_stays_in_that_folder() {
        let scratch = Scratch::new();
        let chosen = scratch.file("MySong/MySong.sibilant", b"{}");
        let location = Location::for_save_as(&chosen).unwrap();
        assert_eq!(location.file(), chosen);
    }

    #[test]
    fn save_as_does_not_replace_a_different_project_with_the_same_name() {
        let scratch = Scratch::new();
        scratch.file("MySong/MySong.sibilant", b"{}");
        let result = Location::for_save_as(&scratch.path().join("MySong.sibilant"));
        assert_eq!(
            result,
            Err(SaveError::NameTaken(scratch.path().join("MySong")))
        );
    }

    #[test]
    fn a_project_needs_a_name() {
        let scratch = Scratch::new();
        let result = Location::for_save_as(&scratch.path().join(".sibilant"));
        assert_eq!(result, Err(SaveError::NoName));
    }

    #[test]
    fn an_opened_project_file_gives_the_folder_and_the_name() {
        let location = Location::of_file(Path::new("songs/MySong/MySong.sibilant"));
        assert_eq!(location.folder(), Path::new("songs/MySong"));
        assert_eq!(location.audio(), Path::new("songs/MySong/audio"));
        assert_eq!(location.name(), "MySong");
    }
}
