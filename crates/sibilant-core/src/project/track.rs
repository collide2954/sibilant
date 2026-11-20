use super::TrackId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub(super) id: TrackId,
    pub(super) name: String,
}

impl Track {
    pub fn id(&self) -> TrackId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
