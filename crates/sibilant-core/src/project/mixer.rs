use super::{EditError, Project, Track, TrackId};
use crate::mix::{Gain, Mix};

impl Project {
    pub fn master_gain(&self) -> Gain {
        self.master_gain
    }

    #[must_use]
    pub fn with_master_gain(&self, master_gain: Gain) -> Self {
        Self {
            master_gain,
            ..self.clone()
        }
    }

    pub fn set_mix(&self, track: TrackId, mix: Mix) -> Result<Self, EditError> {
        self.edit_track(track, |track| track.mix = mix)
    }

    pub fn heard_tracks(&self) -> impl Iterator<Item = &Track> {
        let solo = self.tracks().any(|track| track.mix.solo);
        self.tracks()
            .filter(move |track| !track.mix.mute && (track.mix.solo || !solo))
    }
}

#[cfg(test)]
mod tests {
    use crate::mix::{Gain, Mix, Pan};
    use crate::project::{EditError, Project, Track, TrackId};

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

    fn switches(mute: bool, solo: bool) -> Mix {
        Mix {
            mute,
            solo,
            ..Mix::default()
        }
    }

    fn heard(project: &Project) -> Vec<TrackId> {
        project.heard_tracks().map(Track::id).collect()
    }

    #[test]
    fn without_solo_each_track_that_is_not_muted_is_heard() {
        assert_eq!(heard(&project()), [DRUMS, BASS]);
        let project = project().set_mix(DRUMS, switches(true, false)).unwrap();
        assert_eq!(heard(&project), [BASS]);
    }

    #[test]
    fn with_solo_only_the_soloed_tracks_are_heard() {
        let project = project().set_mix(BASS, switches(false, true)).unwrap();
        assert_eq!(heard(&project), [BASS]);
    }

    #[test]
    fn a_muted_track_is_not_heard_when_it_is_soloed() {
        let project = project().set_mix(BASS, switches(true, true)).unwrap();
        assert_eq!(heard(&project), []);
    }

    #[test]
    fn a_new_project_has_a_master_gain_that_does_not_change_the_audio() {
        assert_eq!(Project::default().master_gain(), Gain::UNITY);
    }

    #[test]
    fn a_project_takes_a_new_master_gain() {
        let gain = Gain::from_db(-3.0).unwrap();
        assert_eq!(project().with_master_gain(gain).master_gain(), gain);
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
