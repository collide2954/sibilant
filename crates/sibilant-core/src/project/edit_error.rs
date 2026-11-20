use std::error::Error;
use std::fmt;

use super::{SceneId, SourceId, TrackId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    DuplicateScene(SceneId),
    DuplicateSource(SourceId),
    DuplicateTrack(TrackId),
    UnknownTrack(TrackId),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateScene(scene) => write!(f, "{scene} already exists"),
            Self::DuplicateSource(source) => write!(f, "{source} already exists"),
            Self::DuplicateTrack(track) => write!(f, "{track} already exists"),
            Self::UnknownTrack(track) => write!(f, "{track} does not exist"),
        }
    }
}

impl Error for EditError {}

#[cfg(test)]
mod tests {
    use crate::project::{EditError, SceneId, SourceId, TrackId};

    #[test]
    fn the_errors_name_the_track() {
        let duplicate = EditError::DuplicateTrack(TrackId::new(7));
        let unknown = EditError::UnknownTrack(TrackId::new(9));
        assert_eq!(duplicate.to_string(), "track 7 already exists");
        assert_eq!(unknown.to_string(), "track 9 does not exist");
    }

    #[test]
    fn the_errors_name_the_scene() {
        let duplicate = EditError::DuplicateScene(SceneId::new(4));
        assert_eq!(duplicate.to_string(), "scene 4 already exists");
    }

    #[test]
    fn the_errors_name_the_audio_source() {
        let duplicate = EditError::DuplicateSource(SourceId::new(3));
        assert_eq!(duplicate.to_string(), "audio source 3 already exists");
    }
}
