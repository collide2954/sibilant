mod edit_error;
mod id;
mod track;

use std::sync::Arc;

pub use edit_error::EditError;
pub use id::TrackId;
pub use track::Track;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Project {
    tracks: Vec<Arc<Track>>,
}
