use std::sync::Arc;

use super::{Clip, EditError, Project, SceneId, TrackId};
use crate::time::Quantize;

impl Project {
    pub fn launch_quantize(&self) -> Quantize {
        self.launch_quantize
    }

    #[must_use]
    pub fn with_launch_quantize(&self, launch_quantize: Quantize) -> Self {
        Self {
            launch_quantize,
            ..self.clone()
        }
    }

    pub fn put_clip_in_slot(
        &self,
        track: TrackId,
        scene: SceneId,
        clip: Clip,
    ) -> Result<Self, EditError> {
        self.check_scene(scene)?;
        self.check_clip(&clip)?;
        self.edit_track(track, |track| {
            track.slots.insert(scene, Arc::new(clip));
        })
    }

    pub fn clear_slot(&self, track: TrackId, scene: SceneId) -> Result<Self, EditError> {
        self.check_scene(scene)?;
        self.edit_track(track, |track| {
            track.slots.remove(&scene);
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::project::{
        AudioSource, Clip, EditError, Ids, Project, Region, SceneId, SourceId, TrackId,
    };
    use crate::time::{Clock, Duration, Quantize};

    const TRACK: TrackId = TrackId::new(1);
    const INTRO: SceneId = SceneId::new(1);
    const VERSE: SceneId = SceneId::new(2);
    const LOOP: SourceId = SourceId::new(1);

    fn seconds(seconds: i64) -> Duration<Clock> {
        Duration::from_seconds(seconds)
    }

    fn project() -> Project {
        Project::default()
            .add_source(AudioSource::new(LOOP, "loop.wav", seconds(4)))
            .and_then(|project| project.add_track(TRACK, "Drums"))
            .and_then(|project| project.add_scene(INTRO, "Intro"))
            .and_then(|project| project.add_scene(VERSE, "Verse"))
            .unwrap()
    }

    fn clip(start: i64, length: i64) -> Clip {
        let region = Region {
            start: seconds(start),
            length: seconds(length),
        };
        Clip::new("Loop", LOOP, region)
    }

    fn slot(project: &Project, scene: SceneId) -> Option<&Clip> {
        project.track(TRACK).unwrap().slot(scene)
    }

    #[test]
    fn a_new_track_has_empty_slots() {
        assert!(slot(&project(), INTRO).is_none());
    }

    #[test]
    fn a_new_project_starts_launcher_clips_at_the_next_bar() {
        let project = Project::new(&mut Ids::default());
        assert_eq!(project.launch_quantize(), Quantize::Bar);
    }

    #[test]
    fn a_project_takes_a_new_launch_quantize() {
        let project = project().with_launch_quantize(Quantize::FourBars);
        assert_eq!(project.launch_quantize(), Quantize::FourBars);
    }

    #[test]
    fn putting_a_clip_in_a_slot_fills_only_that_slot() {
        let project = project()
            .put_clip_in_slot(TRACK, VERSE, clip(0, 2))
            .unwrap();
        assert_eq!(slot(&project, VERSE), Some(&clip(0, 2)));
        assert!(slot(&project, INTRO).is_none());
    }

    #[test]
    fn putting_a_clip_in_a_full_slot_replaces_the_old_clip() {
        let project = project()
            .put_clip_in_slot(TRACK, INTRO, clip(0, 2))
            .and_then(|project| project.put_clip_in_slot(TRACK, INTRO, clip(1, 3)))
            .unwrap();
        assert_eq!(slot(&project, INTRO), Some(&clip(1, 3)));
    }

    #[test]
    fn a_track_lists_its_full_slots_in_the_order_of_the_scene_ids() {
        let project = project()
            .put_clip_in_slot(TRACK, VERSE, clip(0, 2))
            .and_then(|project| project.put_clip_in_slot(TRACK, INTRO, clip(1, 3)))
            .unwrap();
        let slots: Vec<_> = project.track(TRACK).unwrap().slots().collect();
        assert_eq!(slots, [(INTRO, &clip(1, 3)), (VERSE, &clip(0, 2))]);
    }

    #[test]
    fn clearing_a_slot_removes_its_clip() {
        let project = project()
            .put_clip_in_slot(TRACK, INTRO, clip(0, 2))
            .and_then(|project| project.clear_slot(TRACK, INTRO))
            .unwrap();
        assert!(slot(&project, INTRO).is_none());
    }

    #[test]
    fn a_slot_needs_a_known_track_and_a_known_scene() {
        let unknown_track = TrackId::new(9);
        let unknown_scene = SceneId::new(9);
        assert_eq!(
            project().put_clip_in_slot(unknown_track, INTRO, clip(0, 2)),
            Err(EditError::UnknownTrack(unknown_track))
        );
        assert_eq!(
            project().put_clip_in_slot(TRACK, unknown_scene, clip(0, 2)),
            Err(EditError::UnknownScene(unknown_scene))
        );
        assert_eq!(
            project().clear_slot(TRACK, unknown_scene),
            Err(EditError::UnknownScene(unknown_scene))
        );
    }

    #[test]
    fn a_clip_must_refer_to_a_known_audio_source() {
        let unknown = SourceId::new(9);
        let region = clip(0, 2).region();
        assert_eq!(
            project().put_clip_in_slot(TRACK, INTRO, Clip::new("Loop", unknown, region)),
            Err(EditError::UnknownSource(unknown))
        );
    }

    #[test]
    fn a_clip_must_have_a_length() {
        for length in [0, -1] {
            assert_eq!(
                project().put_clip_in_slot(TRACK, INTRO, clip(0, length)),
                Err(EditError::EmptyClip)
            );
        }
    }

    #[test]
    fn a_clip_region_must_be_inside_its_audio_source() {
        for (start, length) in [(-1, 2), (3, 2), (0, 5)] {
            assert_eq!(
                project().put_clip_in_slot(TRACK, INTRO, clip(start, length)),
                Err(EditError::RegionOutsideSource(LOOP)),
                "start {start} s, length {length} s"
            );
        }
    }
}
