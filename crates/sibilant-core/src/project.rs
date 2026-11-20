mod clip;
mod edit_error;
mod id;
mod launcher;
mod mixer;
mod scene;
mod source;
mod timeline;
mod track;
mod track_color;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::mix::Gain;
use crate::time::{TempoMap, TimeSignatureMap};

pub use clip::{Clip, Region};
pub use edit_error::EditError;
pub use id::{Ids, SceneId, SourceId, TrackId};
pub use scene::Scene;
pub use source::AudioSource;
pub use track::Track;
pub use track_color::{TrackColor, TrackColorOutOfRange};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Project {
    tempo_map: TempoMap,
    time_signature_map: TimeSignatureMap,
    sources: Arc<BTreeMap<SourceId, AudioSource>>,
    scenes: Vec<Arc<Scene>>,
    tracks: Vec<Arc<Track>>,
    master_gain: Gain,
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

    pub fn time_signature_map(&self) -> &TimeSignatureMap {
        &self.time_signature_map
    }

    #[must_use]
    pub fn with_time_signature_map(&self, time_signature_map: TimeSignatureMap) -> Self {
        Self {
            time_signature_map,
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::project::Project;
    use crate::time::{Position, Tempo, TempoMap, TimeSignature, TimeSignatureMap};

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

    #[test]
    fn a_new_project_is_in_the_default_time_signature() {
        let project = Project::default();
        assert_eq!(
            project
                .time_signature_map()
                .time_signature_at(Position::START),
            TimeSignature::default()
        );
    }

    #[test]
    fn a_project_takes_a_new_time_signature_map() {
        let map = TimeSignatureMap::new(TimeSignature::new(6, 8).unwrap());
        let project = Project::default().with_time_signature_map(map.clone());
        assert_eq!(project.time_signature_map(), &map);
    }
}
