use std::sync::Arc;

use super::{EditError, Project, TrackId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    id: TrackId,
    name: String,
}

impl Track {
    pub fn id(&self) -> TrackId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
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
        let name = name.into();
        project.tracks.push(Arc::new(Track { id, name }));
        Ok(project)
    }

    pub fn remove_track(&self, id: TrackId) -> Result<Self, EditError> {
        let index = self.track_index(id)?;
        let mut project = self.clone();
        project.tracks.remove(index);
        Ok(project)
    }

    pub fn move_track(&self, id: TrackId, index: usize) -> Result<Self, EditError> {
        let from = self.track_index(id)?;
        let mut project = self.clone();
        let track = project.tracks.remove(from);
        let to = index.min(project.tracks.len());
        project.tracks.insert(to, track);
        Ok(project)
    }

    pub fn rename_track(&self, id: TrackId, name: impl Into<String>) -> Result<Self, EditError> {
        let name = name.into();
        self.edit_track(id, |track| track.name = name)
    }

    fn edit_track(&self, id: TrackId, edit: impl FnOnce(&mut Track)) -> Result<Self, EditError> {
        let index = self.track_index(id)?;
        let mut project = self.clone();
        edit(Arc::make_mut(&mut project.tracks[index]));
        Ok(project)
    }

    fn track_index(&self, id: TrackId) -> Result<usize, EditError> {
        self.tracks
            .iter()
            .position(|track| track.id() == id)
            .ok_or(EditError::UnknownTrack(id))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

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
    fn removing_a_track_takes_it_out_of_the_list() {
        let project = with_tracks(&["Drums", "Bass", "Keys"]);
        let project = project.remove_track(TrackId::new(2)).unwrap();
        assert_eq!(track_names(&project), ["Drums", "Keys"]);
    }

    #[test]
    fn removing_an_unknown_track_is_an_error() {
        let project = with_tracks(&["Drums"]);
        assert_eq!(
            project.remove_track(TrackId::new(9)),
            Err(EditError::UnknownTrack(TrackId::new(9)))
        );
    }

    #[test]
    fn renaming_a_track_changes_only_its_name() {
        let project = with_tracks(&["Drums", "Bass"]);
        let project = project.rename_track(TrackId::new(2), "Sub").unwrap();
        assert_eq!(track_names(&project), ["Drums", "Sub"]);
        assert!(project.track(TrackId::new(2)).is_some());
    }

    #[test]
    fn renaming_an_unknown_track_is_an_error() {
        let project = with_tracks(&["Drums"]);
        assert_eq!(
            project.rename_track(TrackId::new(9), "Bass"),
            Err(EditError::UnknownTrack(TrackId::new(9)))
        );
    }

    #[test]
    fn moving_a_track_puts_it_at_the_new_index() {
        let project = with_tracks(&["Drums", "Bass", "Keys"]);
        let down = project.move_track(TrackId::new(1), 2).unwrap();
        let up = project.move_track(TrackId::new(3), 0).unwrap();
        assert_eq!(track_names(&down), ["Bass", "Keys", "Drums"]);
        assert_eq!(track_names(&up), ["Keys", "Drums", "Bass"]);
    }

    #[test]
    fn moving_a_track_past_the_end_puts_it_last() {
        let project = with_tracks(&["Drums", "Bass", "Keys"]);
        let project = project.move_track(TrackId::new(1), 10).unwrap();
        assert_eq!(track_names(&project), ["Bass", "Keys", "Drums"]);
    }

    #[test]
    fn moving_an_unknown_track_is_an_error() {
        let project = with_tracks(&["Drums"]);
        assert_eq!(
            project.move_track(TrackId::new(9), 0),
            Err(EditError::UnknownTrack(TrackId::new(9)))
        );
    }

    #[test]
    fn an_edit_shares_the_tracks_that_it_does_not_change() {
        let old = with_tracks(&["Drums", "Bass"]);
        let new = old.rename_track(TrackId::new(2), "Sub").unwrap();
        assert!(Arc::ptr_eq(&old.tracks[0], &new.tracks[0]));
        assert!(!Arc::ptr_eq(&old.tracks[1], &new.tracks[1]));
    }
}
