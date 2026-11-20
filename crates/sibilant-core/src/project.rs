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

impl Project {
    pub fn tracks(&self) -> impl ExactSizeIterator<Item = &Track> {
        self.tracks.iter().map(Arc::as_ref)
    }

    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks().find(|track| track.id() == id)
    }

    pub fn add_track(&self, id: TrackId, name: impl Into<String>) -> Result<Self, EditError> {
        if self.track(id).is_some() {
            return Err(EditError::DuplicateTrack(id));
        }
        let mut project = self.clone();
        project.tracks.push(Arc::new(Track::new(id, name.into())));
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use crate::project::{EditError, Project, Track, TrackId};

    fn track_names(project: &Project) -> Vec<&str> {
        project.tracks().map(Track::name).collect()
    }

    fn with_tracks(names: &[&str]) -> Project {
        (1..)
            .zip(names)
            .fold(Project::default(), |project, (id, &name)| {
                project.add_track(TrackId::new(id), name).unwrap()
            })
    }

    #[test]
    fn a_new_project_has_no_tracks() {
        assert_eq!(Project::default().tracks().len(), 0);
    }

    #[test]
    fn adding_a_track_puts_it_at_the_end_of_the_list() {
        let project = with_tracks(&["Drums", "Bass"]);
        assert_eq!(track_names(&project), ["Drums", "Bass"]);
    }

    #[test]
    fn adding_a_track_leaves_the_old_project_unchanged() {
        let old = with_tracks(&["Drums"]);
        let new = old.add_track(TrackId::new(2), "Bass").unwrap();
        assert_eq!(track_names(&old), ["Drums"]);
        assert_eq!(track_names(&new), ["Drums", "Bass"]);
    }

    #[test]
    fn finds_a_track_by_its_id() {
        let project = with_tracks(&["Drums", "Bass"]);
        let track = project.track(TrackId::new(2)).unwrap();
        assert_eq!((track.id(), track.name()), (TrackId::new(2), "Bass"));
        assert!(project.track(TrackId::new(3)).is_none());
    }

    #[test]
    fn a_track_id_can_be_used_only_once() {
        let project = with_tracks(&["Drums"]);
        assert_eq!(
            project.add_track(TrackId::new(1), "Bass"),
            Err(EditError::DuplicateTrack(TrackId::new(1)))
        );
    }

    #[test]
    fn the_error_names_the_track() {
        let error = EditError::DuplicateTrack(TrackId::new(7));
        assert_eq!(error.to_string(), "track 7 already exists");
    }
}
