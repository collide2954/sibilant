use std::collections::BTreeMap;
use std::sync::Arc;

use super::timeline::Lane;
use super::{Clip, EditError, Project, SceneId, TrackColor, TrackId};
use crate::mix::Mix;

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub(super) id: TrackId,
    pub(super) name: String,
    pub(super) color: TrackColor,
    pub(super) mix: Mix,
    pub(super) slots: BTreeMap<SceneId, Arc<Clip>>,
    pub(super) timeline: Lane,
}

impl Track {
    pub fn id(&self) -> TrackId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn color(&self) -> TrackColor {
        self.color
    }

    pub fn mix(&self) -> Mix {
        self.mix
    }

    pub fn slot(&self, scene: SceneId) -> Option<&Clip> {
        self.slots.get(&scene).map(Arc::as_ref)
    }

    pub fn slots(&self) -> impl ExactSizeIterator<Item = (SceneId, &Clip)> {
        self.slots
            .iter()
            .map(|(&scene, clip)| (scene, clip.as_ref()))
    }
}

impl Project {
    pub fn tracks(&self) -> impl ExactSizeIterator<Item = &Track> {
        self.tracks.iter().map(Arc::as_ref)
    }

    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks().find(|track| track.id() == id)
    }

    pub fn next_track_name(&self) -> String {
        let highest = self
            .tracks()
            .filter_map(|track| track.name.strip_prefix("Audio ")?.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        format!("Audio {}", highest + 1)
    }

    pub fn add_track(&self, id: TrackId, name: impl Into<String>) -> Result<Self, EditError> {
        if self.track(id).is_some() {
            return Err(EditError::DuplicateTrack(id));
        }
        let color = self
            .tracks
            .last()
            .map_or(TrackColor::default(), |track| track.color.next());
        let mut project = self.clone();
        project.tracks.push(Arc::new(Track {
            id,
            name: name.into(),
            color,
            mix: Mix::default(),
            slots: BTreeMap::new(),
            timeline: Lane::new(),
        }));
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

    pub fn set_track_color(&self, id: TrackId, color: TrackColor) -> Result<Self, EditError> {
        self.edit_track(id, |track| track.color = color)
    }

    pub(super) fn edit_track(
        &self,
        id: TrackId,
        edit: impl FnOnce(&mut Track),
    ) -> Result<Self, EditError> {
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

    use crate::project::{EditError, Project, Track, TrackColor, TrackId};

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
    fn the_first_new_track_is_audio_1() {
        assert_eq!(Project::default().next_track_name(), "Audio 1");
    }

    #[test]
    fn a_new_track_name_takes_the_number_after_the_highest_audio_number() {
        let project = with_tracks(&["Audio 1", "Audio 5", "Drums", "Audio x"]);
        assert_eq!(project.next_track_name(), "Audio 6");
    }

    fn colors(project: &Project) -> Vec<u8> {
        project
            .tracks()
            .map(|track| track.color().index())
            .collect()
    }

    #[test]
    fn new_tracks_take_the_colors_of_the_palette_in_turn() {
        let project = with_tracks(&["Drums", "Bass", "Keys"]);
        assert_eq!(colors(&project), [0, 1, 2]);
    }

    #[test]
    fn a_new_track_takes_the_color_after_the_color_of_the_last_track() {
        let project = with_tracks(&["Drums", "Bass"])
            .set_track_color(TrackId::new(2), TrackColor::new(7).unwrap())
            .and_then(|project| project.add_track(TrackId::new(3), "Keys"))
            .unwrap();
        assert_eq!(colors(&project), [0, 7, 8]);
    }

    #[test]
    fn setting_the_color_of_an_unknown_track_is_an_error() {
        let project = with_tracks(&["Drums"]);
        assert_eq!(
            project.set_track_color(TrackId::new(9), TrackColor::default()),
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
