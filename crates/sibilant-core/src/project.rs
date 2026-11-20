mod edit_error;
mod id;
mod source;
mod track;

use std::collections::BTreeMap;
use std::sync::Arc;

pub use edit_error::EditError;
pub use id::{SourceId, TrackId};
pub use source::AudioSource;
pub use track::Track;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Project {
    sources: Arc<BTreeMap<SourceId, AudioSource>>,
    tracks: Vec<Arc<Track>>,
}
