use iced::{Element, Theme};
use sibilant_core::history::History;
use sibilant_core::mix::Mix;
use sibilant_core::project::{EditError, Ids, Project, TrackId};
use sibilant_engine::engine;
use sibilant_ui::command::Command;
use sibilant_ui::theme::Palette;
use sibilant_ui::timeline::Viewport;
use sibilant_ui::{shell, track_area};

use crate::audio::Audio;

#[derive(Debug)]
pub(crate) struct App {
    palette: Palette,
    history: History<Project>,
    ids: Ids,
    viewport: Viewport,
    audio: Option<Audio>,
    metronome: bool,
}

impl Default for App {
    fn default() -> Self {
        let mut ids = Ids::default();
        let project = Project::new(&mut ids);
        Self {
            palette: Palette::default(),
            history: History::new(project),
            ids,
            viewport: Viewport::default(),
            audio: None,
            metronome: false,
        }
    }
}

impl App {
    pub(crate) fn with_audio() -> Self {
        let mut app = Self::default();
        match Audio::start(app.history.present()) {
            Ok(audio) => app.audio = Some(audio),
            Err(error) => eprintln!("audio output is not available: {error}"),
        }
        app
    }

    pub(crate) fn update(&mut self, command: Command) {
        match command {
            Command::AddTrack => {
                let id = self.ids.track();
                self.edit(|project| project.add_track(id, project.next_track_name()));
            }
            Command::ToggleMute(track) => {
                self.edit(|project| change_mix(project, track, |mix| mix.mute = !mix.mute));
            }
            Command::ToggleSolo(track) => {
                self.edit(|project| change_mix(project, track, |mix| mix.solo = !mix.solo));
            }
            Command::Undo => {
                self.history.undo();
            }
            Command::Redo => {
                self.history.redo();
            }
            Command::ScrollTimeline { pixels } => {
                self.viewport = self.viewport.scrolled(pixels);
            }
            Command::ZoomTimeline { steps, anchor } => {
                self.viewport = self.viewport.zoomed(steps, anchor);
            }
            Command::PlayOrStop => self.send(engine::Command::TogglePlay),
            Command::Continue => self.send(engine::Command::Continue),
            Command::ToggleMetronome => {
                self.metronome = !self.metronome;
                self.send(engine::Command::SetMetronome(self.metronome));
            }
        }
    }

    fn send(&mut self, command: engine::Command) {
        if let Some(audio) = &mut self.audio {
            audio.controller.send(command);
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Command> {
        let tracks = track_area::view(self.history.present(), self.viewport, self.palette);
        shell::view(self.palette, tracks)
    }

    pub(crate) fn theme(&self) -> Theme {
        Theme::from(self.palette)
    }

    fn edit(&mut self, edit: impl FnOnce(&Project) -> Result<Project, EditError>) {
        if let Ok(project) = edit(self.history.present()) {
            self.history.commit(project);
        }
    }
}

fn change_mix(
    project: &Project,
    track: TrackId,
    change: impl FnOnce(&mut Mix),
) -> Result<Project, EditError> {
    let mut mix = project
        .track(track)
        .ok_or(EditError::UnknownTrack(track))?
        .mix();
    change(&mut mix);
    project.set_mix(track, mix)
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use sibilant_core::project::{Project, Track, TrackId};
    use sibilant_core::time::Position;
    use sibilant_engine::engine::Engine;
    use sibilant_ui::command::Command;

    use crate::audio::Audio;
    use sibilant_ui::theme::Palette;

    use crate::app::App;

    fn project(app: &App) -> &Project {
        app.history.present()
    }

    fn track_names(app: &App) -> Vec<&str> {
        project(app).tracks().map(Track::name).collect()
    }

    fn first_track(app: &App) -> &Track {
        project(app).tracks().next().unwrap()
    }

    #[test]
    fn the_app_starts_with_a_new_project() {
        assert_eq!(track_names(&App::default()), ["Audio 1", "Audio 2"]);
    }

    #[test]
    fn adding_a_track_appends_the_next_audio_track() {
        let mut app = App::default();
        app.update(Command::AddTrack);
        assert_eq!(track_names(&app), ["Audio 1", "Audio 2", "Audio 3"]);
    }

    #[test]
    fn undo_and_redo_step_through_the_changes() {
        let mut app = App::default();
        app.update(Command::AddTrack);
        app.update(Command::Undo);
        assert_eq!(track_names(&app).len(), 2);
        app.update(Command::Redo);
        assert_eq!(track_names(&app).len(), 3);
    }

    #[test]
    fn a_track_added_after_an_undo_gets_a_new_id() {
        let mut app = App::default();
        app.update(Command::AddTrack);
        let undone: TrackId = project(&app).tracks().last().unwrap().id();
        app.update(Command::Undo);
        app.update(Command::AddTrack);
        assert_ne!(project(&app).tracks().last().unwrap().id(), undone);
    }

    fn with_simulated_audio() -> (App, Engine) {
        let mut app = App::default();
        let (audio, engine) = Audio::without_device(app.history.present(), 48_000);
        app.audio = Some(audio);
        (app, engine)
    }

    fn engine_is_playing(app: &App) -> bool {
        app.audio.as_ref().unwrap().controller.is_playing()
    }

    #[test]
    fn play_or_stop_starts_and_stops_the_engine() {
        let (mut app, mut engine) = with_simulated_audio();
        app.update(Command::PlayOrStop);
        engine.process(&mut [0.0; 512], 2);
        assert!(engine_is_playing(&app));
        app.update(Command::PlayOrStop);
        engine.process(&mut [0.0; 512], 2);
        assert!(!engine_is_playing(&app));
    }

    #[test]
    fn the_metronome_button_switches_the_click_of_the_engine() {
        let (mut app, mut engine) = with_simulated_audio();
        app.update(Command::ToggleMetronome);
        app.update(Command::PlayOrStop);
        let mut buffer = [0.0; 960];
        engine.process(&mut buffer, 2);
        assert!(app.metronome);
        assert!(buffer.iter().any(|sample| sample.abs() > 0.1));
    }

    #[test]
    fn transport_commands_without_an_audio_device_change_only_the_buttons() {
        let mut app = App::default();
        app.update(Command::PlayOrStop);
        app.update(Command::ToggleMetronome);
        assert!(app.metronome);
    }

    #[test]
    fn zoom_and_scroll_change_the_view_but_not_the_undo_history() {
        let mut app = App::default();
        app.update(Command::ZoomTimeline {
            steps: 1.0,
            anchor: 0.0,
        });
        app.update(Command::ScrollTimeline { pixels: 80.0 });
        assert!((app.viewport.pixels_per_quarter_note() - 40.0).abs() < 1e-9);
        assert_eq!(app.viewport.start(), Position::from_quarter_notes(2));
        assert!(!app.history.can_undo());
    }

    #[test]
    fn mute_and_solo_switch_the_mix_of_a_track_on_and_off() {
        let mut app = App::default();
        let track = first_track(&app).id();
        app.update(Command::ToggleMute(track));
        app.update(Command::ToggleSolo(track));
        assert!(first_track(&app).mix().mute && first_track(&app).mix().solo);
        app.update(Command::ToggleMute(track));
        assert!(!first_track(&app).mix().mute);
        app.update(Command::Undo);
        assert!(first_track(&app).mix().mute);
    }

    #[test]
    fn the_app_uses_the_graphite_theme() {
        let theme = App::default().theme();
        assert_eq!(
            theme.palette(),
            iced::Theme::from(Palette::GRAPHITE).palette()
        );
    }

    #[test]
    fn the_app_shows_the_zones_of_the_main_window() {
        let app = App::default();
        let mut ui = simulator(app.view());
        for zone in ["Transport", "Browser", "Audio 1", "Inspector", "Mixer"] {
            assert!(ui.find(zone).is_ok(), "{zone}");
        }
    }
}
