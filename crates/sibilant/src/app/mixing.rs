use sibilant_core::mix::{Gain, Mix, Pan};
use sibilant_core::project::{Project, TrackId};
use sibilant_ui::mixer::{Channel, Mixing};

use super::App;
use super::editing::Preview;

impl App {
    pub(super) fn mix(&mut self, mixing: Mixing) {
        match mixing {
            Mixing::Gain(channel, db) => {
                self.preview_mix(|project| with_gain(project, channel, db));
            }
            Mixing::Pan(track, position) => {
                self.preview_mix(|project| with_pan(project, track, position));
            }
            Mixing::Done => {
                if let Some(preview) = self.preview.take() {
                    self.commit(preview);
                }
            }
            Mixing::ResetGain(channel) => {
                self.commit_mix(|project| with_gain(project, channel, 0.0));
            }
            Mixing::ResetPan(track) => self.commit_mix(|project| with_pan(project, track, 0.0)),
        }
    }

    fn preview_mix(&mut self, edit: impl FnOnce(&Project) -> Option<Project>) {
        if let Some(project) = edit(self.history.present()) {
            self.preview = Some(Preview::new(project, self.selection.clone()));
            self.sync();
        }
    }

    fn commit_mix(&mut self, edit: impl FnOnce(&Project) -> Option<Project>) {
        if let Some(project) = edit(self.history.present()) {
            self.commit(Preview::new(project, self.selection.clone()));
        }
    }
}

fn with_gain(project: &Project, channel: Channel, db: f64) -> Option<Project> {
    let gain = Gain::from_db(db).ok()?;
    match channel {
        Channel::Master => Some(project.with_master_gain(gain)),
        Channel::Track(track) => {
            let mix = project.track(track)?.mix();
            project.set_mix(track, Mix { gain, ..mix }).ok()
        }
    }
}

fn with_pan(project: &Project, track: TrackId, position: f64) -> Option<Project> {
    let pan = Pan::new(position).ok()?;
    let mix = project.track(track)?.mix();
    project.set_mix(track, Mix { pan, ..mix }).ok()
}

#[cfg(test)]
mod tests {
    use sibilant_core::mix::Mix;
    use sibilant_core::project::{Track, TrackId};
    use sibilant_ui::command::Command;
    use sibilant_ui::mixer::{Channel, Mixing};

    use crate::app::{App, Message};

    fn first_track(app: &App) -> TrackId {
        app.history
            .present()
            .tracks()
            .next()
            .map(Track::id)
            .unwrap()
    }

    fn mix(app: &mut App, mixing: Mixing) {
        let _ = app.update(Message::Command(Command::Mix(mixing)));
    }

    fn present_mix(app: &App, track: TrackId) -> Mix {
        app.history.present().track(track).unwrap().mix()
    }

    fn shown_mix(app: &App, track: TrackId) -> Mix {
        app.shown().0.track(track).unwrap().mix()
    }

    #[test]
    fn a_fader_drag_changes_the_gain_at_once_and_commits_one_step() {
        let mut app = App::default();
        let track = first_track(&app);
        mix(&mut app, Mixing::Gain(Channel::Track(track), -6.0));
        mix(&mut app, Mixing::Gain(Channel::Track(track), -12.0));
        assert_eq!(shown_mix(&app, track).gain.db(), -12.0);
        assert_eq!(present_mix(&app, track).gain.db(), 0.0);
        mix(&mut app, Mixing::Done);
        assert_eq!(present_mix(&app, track).gain.db(), -12.0);
        let _ = app.update(Message::Command(Command::Undo));
        assert_eq!(present_mix(&app, track).gain.db(), 0.0);
        assert!(!app.history.can_undo());
    }

    #[test]
    fn a_pan_drag_changes_the_pan_in_one_step() {
        let mut app = App::default();
        let track = first_track(&app);
        mix(&mut app, Mixing::Pan(track, 0.5));
        mix(&mut app, Mixing::Done);
        assert_eq!(present_mix(&app, track).pan.position(), 0.5);
    }

    #[test]
    fn the_master_fader_changes_the_master_gain() {
        let mut app = App::default();
        mix(&mut app, Mixing::Gain(Channel::Master, 3.0));
        mix(&mut app, Mixing::Done);
        assert_eq!(app.history.present().master_gain().db(), 3.0);
    }

    #[test]
    fn a_click_on_a_fader_changes_nothing() {
        let mut app = App::default();
        mix(&mut app, Mixing::Done);
        assert!(!app.history.can_undo());
    }

    #[test]
    fn a_double_click_resets_the_gain_and_the_pan_in_one_step_each() {
        let mut app = App::default();
        let track = first_track(&app);
        mix(&mut app, Mixing::Gain(Channel::Track(track), -6.0));
        mix(&mut app, Mixing::Pan(track, -0.5));
        mix(&mut app, Mixing::Done);
        mix(&mut app, Mixing::Gain(Channel::Master, -3.0));
        mix(&mut app, Mixing::Done);
        mix(&mut app, Mixing::ResetGain(Channel::Track(track)));
        mix(&mut app, Mixing::ResetPan(track));
        mix(&mut app, Mixing::ResetGain(Channel::Master));
        assert_eq!(present_mix(&app, track).gain.db(), 0.0);
        assert_eq!(present_mix(&app, track).pan.position(), 0.0);
        assert_eq!(app.history.present().master_gain().db(), 0.0);
        let _ = app.update(Message::Command(Command::Undo));
        assert_eq!(app.history.present().master_gain().db(), -3.0);
    }

    #[test]
    fn escape_during_a_fader_drag_puts_the_gain_back() {
        let mut app = App::default();
        let track = first_track(&app);
        mix(&mut app, Mixing::Gain(Channel::Track(track), -6.0));
        let _ = app.update(Message::Command(Command::Cancel));
        mix(&mut app, Mixing::Done);
        assert_eq!(shown_mix(&app, track).gain.db(), 0.0);
        assert!(!app.history.can_undo());
    }
}
