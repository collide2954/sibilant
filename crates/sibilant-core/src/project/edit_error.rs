use std::error::Error;
use std::fmt;

use super::TrackId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    DuplicateTrack(TrackId),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateTrack(track) => write!(f, "{track} already exists"),
        }
    }
}

impl Error for EditError {}
