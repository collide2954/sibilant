use super::TrackId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    id: TrackId,
    name: String,
}

impl Track {
    pub(super) fn new(id: TrackId, name: String) -> Self {
        Self { id, name }
    }

    pub fn id(&self) -> TrackId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
