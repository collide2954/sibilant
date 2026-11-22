mod clip;
mod edit_error;
mod editing;
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
use crate::time::{Quantize, TempoMap, TimeSignatureMap};

pub use clip::{Clip, Region};
pub use edit_error::EditError;
pub use editing::{ClipKey, Edge, Edited};
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
    launch_quantize: Quantize,
}

const NEW_TRACKS: usize = 2;
const NEW_SCENES: usize = 4;

impl Project {
    pub fn new(ids: &mut Ids) -> Self {
        let tracks = (0..NEW_TRACKS).fold(Self::default(), |project, _| {
            let name = project.next_track_name();
            project
                .add_track(ids.track(), name)
                .expect("a new id is never in use")
        });
        (1..=NEW_SCENES).fold(tracks, |project, number| {
            project
                .add_scene(ids.scene(), format!("Scene {number}"))
                .expect("a new id is never in use")
        })
    }

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
    use crate::project::{Ids, Project, Scene, Track, TrackId};
    use crate::time::{Position, Tempo, TempoMap, TimeSignature, TimeSignatureMap};

    #[test]
    fn a_new_project_has_two_audio_tracks_and_four_scenes() {
        let project = Project::new(&mut Ids::default());
        let tracks: Vec<_> = project.tracks().map(Track::name).collect();
        let scenes: Vec<_> = project.scenes().map(Scene::name).collect();
        assert_eq!(tracks, ["Audio 1", "Audio 2"]);
        assert_eq!(scenes, ["Scene 1", "Scene 2", "Scene 3", "Scene 4"]);
    }

    #[test]
    fn a_new_project_takes_its_ids_from_the_counter() {
        let mut ids = Ids::default();
        let _ = Project::new(&mut ids);
        assert_eq!(ids.track(), TrackId::new(7));
    }

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
