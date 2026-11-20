use std::error::Error;
use std::fmt;

use super::{SceneId, SourceId, TrackId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    DuplicateScene(SceneId),
    DuplicateSource(SourceId),
    DuplicateTrack(TrackId),
    EmptyClip,
    RegionOutsideSource(SourceId),
    UnknownScene(SceneId),
    UnknownSource(SourceId),
    UnknownTrack(TrackId),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateScene(scene) => write!(f, "{scene} already exists"),
            Self::DuplicateSource(source) => write!(f, "{source} already exists"),
            Self::DuplicateTrack(track) => write!(f, "{track} already exists"),
            Self::EmptyClip => write!(f, "the clip has no length"),
            Self::RegionOutsideSource(source) => {
                write!(f, "the clip region is not inside {source}")
            }
            Self::UnknownScene(scene) => write!(f, "{scene} does not exist"),
            Self::UnknownSource(source) => write!(f, "{source} does not exist"),
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
        let unknown = EditError::UnknownScene(SceneId::new(5));
        assert_eq!(duplicate.to_string(), "scene 4 already exists");
        assert_eq!(unknown.to_string(), "scene 5 does not exist");
    }

    #[test]
    fn the_errors_name_the_audio_source() {
        let duplicate = EditError::DuplicateSource(SourceId::new(3));
        let unknown = EditError::UnknownSource(SourceId::new(6));
        let outside = EditError::RegionOutsideSource(SourceId::new(1));
        assert_eq!(duplicate.to_string(), "audio source 3 already exists");
        assert_eq!(unknown.to_string(), "audio source 6 does not exist");
        assert_eq!(
            outside.to_string(),
            "the clip region is not inside audio source 1"
        );
    }

    #[test]
    fn the_error_tells_that_a_clip_is_empty() {
        assert_eq!(EditError::EmptyClip.to_string(), "the clip has no length");
    }
}
