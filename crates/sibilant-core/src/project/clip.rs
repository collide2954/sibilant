use super::{EditError, Project, SourceId};
use crate::time::{Clock, Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub start: Duration<Clock>,
    pub length: Duration<Clock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    name: String,
    source: SourceId,
    region: Region,
}

impl Clip {
    pub fn new(name: impl Into<String>, source: SourceId, region: Region) -> Self {
        Self {
            name: name.into(),
            source,
            region,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> SourceId {
        self.source
    }

    pub fn region(&self) -> Region {
        self.region
    }
}

impl Project {
    pub(super) fn check_clip(&self, clip: &Clip) -> Result<(), EditError> {
        let source = self
            .source(clip.source)
            .ok_or(EditError::UnknownSource(clip.source))?;
        let Region { start, length } = clip.region;
        if length <= Duration::ZERO {
            return Err(EditError::EmptyClip);
        }
        if start < Duration::ZERO || start + length > source.length() {
            return Err(EditError::RegionOutsideSource(clip.source));
        }
        Ok(())
    }
}
