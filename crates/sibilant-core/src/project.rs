mod clip;
mod edit_error;
mod id;
mod launcher;
mod scene;
mod source;
mod track;

use std::collections::BTreeMap;
use std::sync::Arc;

pub use clip::{Clip, Region};
pub use edit_error::EditError;
pub use id::{SceneId, SourceId, TrackId};
pub use scene::Scene;
pub use source::AudioSource;
pub use track::Track;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Project {
    sources: Arc<BTreeMap<SourceId, AudioSource>>,
    scenes: Vec<Arc<Scene>>,
    tracks: Vec<Arc<Track>>,
}
