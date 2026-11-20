use std::fmt;

use super::Project;

macro_rules! id {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            pub const fn value(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($label, " {}"), self.0)
            }
        }
    };
}

id!(SceneId, "scene");
id!(SourceId, "audio source");
id!(TrackId, "track");

#[derive(Debug, Clone)]
pub struct Ids {
    next: u64,
}

impl Ids {
    pub fn after(project: &Project) -> Self {
        let highest = project
            .tracks()
            .map(|track| track.id().value())
            .chain(project.scenes().map(|scene| scene.id().value()))
            .chain(project.sources().map(|source| source.id().value()))
            .max()
            .unwrap_or(0);
        Self { next: highest + 1 }
    }

    pub fn track(&mut self) -> TrackId {
        TrackId::new(self.take())
    }

    pub fn scene(&mut self) -> SceneId {
        SceneId::new(self.take())
    }

    pub fn source(&mut self) -> SourceId {
        SourceId::new(self.take())
    }

    fn take(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }
}

impl Default for Ids {
    fn default() -> Self {
        Self { next: 1 }
    }
}

#[cfg(test)]
mod tests {
    use crate::project::{AudioSource, Ids, Project, SceneId, SourceId, TrackId};
    use crate::time::Duration;

    #[test]
    fn hands_out_a_new_id_each_time() {
        let mut ids = Ids::default();
        assert_eq!(ids.track(), TrackId::new(1));
        assert_eq!(ids.scene(), SceneId::new(2));
        assert_eq!(ids.source(), SourceId::new(3));
        assert_eq!(ids.track(), TrackId::new(4));
    }

    #[test]
    fn continues_after_the_highest_id_in_a_project() {
        let source = AudioSource::new(SourceId::new(9), "a.wav", Duration::from_seconds(1));
        let project = Project::default()
            .add_track(TrackId::new(7), "Drums")
            .and_then(|project| project.add_scene(SceneId::new(3), "Intro"))
            .and_then(|project| project.add_source(source))
            .unwrap();
        assert_eq!(Ids::after(&project).track(), TrackId::new(10));
    }

    #[test]
    fn an_empty_project_starts_the_ids_at_1() {
        assert_eq!(Ids::after(&Project::default()).track(), TrackId::new(1));
    }
}
