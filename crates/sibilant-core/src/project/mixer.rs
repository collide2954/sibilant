use super::{EditError, Project, TrackId};
use crate::mix::Mix;

impl Project {
    pub fn set_mix(&self, track: TrackId, mix: Mix) -> Result<Self, EditError> {
        self.edit_track(track, |track| track.mix = mix)
    }
}

#[cfg(test)]
mod tests {
    use crate::mix::{Gain, Mix, Pan};
    use crate::project::{EditError, Project, TrackId};

    const DRUMS: TrackId = TrackId::new(1);
    const BASS: TrackId = TrackId::new(2);

    fn project() -> Project {
        Project::default()
            .add_track(DRUMS, "Drums")
            .and_then(|project| project.add_track(BASS, "Bass"))
            .unwrap()
    }

    fn mix_of(project: &Project, track: TrackId) -> Mix {
        project.track(track).unwrap().mix()
    }

    #[test]
    fn a_new_track_has_the_default_mix() {
        assert_eq!(mix_of(&project(), DRUMS), Mix::default());
    }

    #[test]
    fn setting_the_mix_changes_only_that_track() {
        let mix = Mix {
            gain: Gain::from_db(-6.0).unwrap(),
            pan: Pan::new(0.5).unwrap(),
            mute: true,
            solo: false,
        };
        let project = project().set_mix(BASS, mix).unwrap();
        assert_eq!(mix_of(&project, BASS), mix);
        assert_eq!(mix_of(&project, DRUMS), Mix::default());
    }

    #[test]
    fn setting_the_mix_of_an_unknown_track_is_an_error() {
        let unknown = TrackId::new(9);
        assert_eq!(
            project().set_mix(unknown, Mix::default()),
            Err(EditError::UnknownTrack(unknown))
        );
    }
}
