use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{EditError, Project, SourceId};
use crate::time::{Clock, Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSource {
    id: SourceId,
    path: PathBuf,
    length: Duration<Clock>,
}

impl AudioSource {
    pub fn new(id: SourceId, path: impl Into<PathBuf>, length: Duration<Clock>) -> Self {
        Self {
            id,
            path: path.into(),
            length,
        }
    }

    pub fn id(&self) -> SourceId {
        self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn length(&self) -> Duration<Clock> {
        self.length
    }
}

impl Project {
    pub fn sources(&self) -> impl ExactSizeIterator<Item = &AudioSource> {
        self.sources.values()
    }

    pub fn source(&self, id: SourceId) -> Option<&AudioSource> {
        self.sources.get(&id)
    }

    pub fn add_source(&self, source: AudioSource) -> Result<Self, EditError> {
        if self.sources.contains_key(&source.id) {
            return Err(EditError::DuplicateSource(source.id));
        }
        let mut project = self.clone();
        Arc::make_mut(&mut project.sources).insert(source.id, source);
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::project::{AudioSource, EditError, Project, SourceId};
    use crate::time::Duration;

    fn kick() -> AudioSource {
        AudioSource::new(SourceId::new(1), "kick.wav", Duration::from_seconds(2))
    }

    #[test]
    fn a_new_project_has_no_audio_sources() {
        assert_eq!(Project::default().sources().len(), 0);
    }

    #[test]
    fn adding_a_source_makes_it_available_by_its_id() {
        let project = Project::default().add_source(kick()).unwrap();
        let source = project.source(SourceId::new(1)).unwrap();
        assert_eq!(source.id(), SourceId::new(1));
        assert_eq!(source.length(), Duration::from_seconds(2));
        assert!(project.source(SourceId::new(2)).is_none());
    }

    #[test]
    fn a_source_keeps_the_path_of_its_audio_file() {
        let path = Path::new("samples").join("kick.wav");
        let source = AudioSource::new(SourceId::new(1), &path, Duration::from_seconds(2));
        assert_eq!(source.path(), path);
    }

    #[test]
    fn a_source_id_can_be_used_only_once() {
        let project = Project::default().add_source(kick()).unwrap();
        assert_eq!(
            project.add_source(kick()),
            Err(EditError::DuplicateSource(SourceId::new(1)))
        );
    }
}
