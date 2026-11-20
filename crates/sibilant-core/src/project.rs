mod clip;
mod edit_error;
mod id;
mod launcher;
mod scene;
mod source;
mod timeline;
mod track;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::time::TempoMap;

pub use clip::{Clip, Region};
pub use edit_error::EditError;
pub use id::{SceneId, SourceId, TrackId};
pub use scene::Scene;
pub use source::AudioSource;
pub use track::Track;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Project {
    tempo_map: TempoMap,
    sources: Arc<BTreeMap<SourceId, AudioSource>>,
    scenes: Vec<Arc<Scene>>,
    tracks: Vec<Arc<Track>>,
}

impl Project {
    pub fn tempo_map(&self) -> &TempoMap {
        &self.tempo_map
    }

    #[must_use]
    pub fn with_tempo_map(&self, tempo_map: TempoMap) -> Self {
        Self {
            tempo_map,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::project::Project;
    use crate::time::{Position, Tempo, TempoMap};

    #[test]
    fn a_new_project_plays_at_the_default_tempo() {
        let project = Project::default();
        assert_eq!(
            project.tempo_map().tempo_at(Position::START),
            Tempo::default()
        );
    }

    #[test]
    fn a_project_takes_a_new_tempo_map() {
        let map = TempoMap::new(Tempo::from_bpm(90.0).unwrap());
        let project = Project::default().with_tempo_map(map.clone());
        assert_eq!(project.tempo_map(), &map);
    }
}
