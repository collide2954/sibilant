use iced::{Element, Subscription, Theme, keyboard, window};
use sibilant_core::history::History;
use sibilant_core::mix::Mix;
use sibilant_core::project::{EditError, Ids, Project, TrackId};
use sibilant_core::time::Position;
use sibilant_engine::engine;
use sibilant_ui::command::Command;
use sibilant_ui::theme::Palette;
use sibilant_ui::timeline::Viewport;
use sibilant_ui::transport::{self, Display};
use sibilant_ui::{shell, track_area};

use crate::audio::Audio;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Message {
    Command(Command),
    Frame,
}

#[derive(Debug)]
pub(crate) struct App {
    palette: Palette,
    history: History<Project>,
    ids: Ids,
    viewport: Viewport,
    audio: Option<Audio>,
    metronome: bool,
    playing: bool,
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
            playing: false,
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

    pub(crate) fn update(&mut self, message: Message) {
        match message {
            Message::Command(command) => self.run(command),
            Message::Frame => {
                self.playing = self
                    .audio
                    .as_ref()
                    .is_some_and(|audio| audio.controller.is_playing());
            }
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let shortcuts = keyboard::listen()
            .filter_map(Command::from_key_event)
            .map(Message::Command);
        if self.playing {
            Subscription::batch([shortcuts, window::frames().map(|_| Message::Frame)])
        } else {
            shortcuts
        }
    }

    fn run(&mut self, command: Command) {
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
            Command::PlayOrStop => self.send_transport(engine::Command::TogglePlay),
            Command::Continue => self.send_transport(engine::Command::Continue),
            Command::ToggleMetronome => {
                self.metronome = !self.metronome;
                self.send(engine::Command::SetMetronome(self.metronome));
            }
        }
    }

    fn send(&mut self, command: engine::Command) -> bool {
        self.audio
            .as_mut()
            .is_some_and(|audio| audio.controller.send(command))
    }

    fn send_transport(&mut self, command: engine::Command) {
        if self.send(command) {
            self.playing = true;
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let project = self.history.present();
        let clock = self.audio.as_ref().map_or(Position::START, Audio::playhead);
        let position = project.tempo_map().to_musical(clock);
        let display = Display {
            playing: self.playing,
            metronome: self.metronome,
            position,
            clock,
            tempo: project.tempo_map().tempo_at(position),
            time_signatures: project.time_signature_map(),
        };
        let transport = transport::view(display, self.palette);
        let tracks = track_area::view(project, self.viewport, position, self.palette);
        shell::view(self.palette, transport, tracks).map(Message::Command)
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

    use crate::app::Message;
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
        run(&mut app, Command::AddTrack);
        assert_eq!(track_names(&app), ["Audio 1", "Audio 2", "Audio 3"]);
    }

    #[test]
    fn undo_and_redo_step_through_the_changes() {
        let mut app = App::default();
        run(&mut app, Command::AddTrack);
        run(&mut app, Command::Undo);
        assert_eq!(track_names(&app).len(), 2);
        run(&mut app, Command::Redo);
        assert_eq!(track_names(&app).len(), 3);
    }

    #[test]
    fn a_track_added_after_an_undo_gets_a_new_id() {
        let mut app = App::default();
        run(&mut app, Command::AddTrack);
        let undone: TrackId = project(&app).tracks().last().unwrap().id();
        run(&mut app, Command::Undo);
        run(&mut app, Command::AddTrack);
        assert_ne!(project(&app).tracks().last().unwrap().id(), undone);
    }

    fn run(app: &mut App, command: Command) {
        app.update(Message::Command(command));
    }

    #[test]
    fn the_transport_shows_the_playhead_of_the_engine() {
        let (mut app, mut engine) = with_simulated_audio();
        run(&mut app, Command::PlayOrStop);
        engine.process(&mut vec![0.0; 96_000], 2);
        app.update(Message::Frame);
        let mut ui = simulator(app.view());
        assert!(ui.find("1.3.1").is_ok());
        assert!(ui.find("0:01.000").is_ok());
    }

    #[test]
    fn playing_follows_the_engine_on_each_frame() {
        let (mut app, mut engine) = with_simulated_audio();
        run(&mut app, Command::PlayOrStop);
        engine.process(&mut [0.0; 512], 2);
        app.update(Message::Frame);
        assert!(app.playing);
        run(&mut app, Command::PlayOrStop);
        engine.process(&mut [0.0; 512], 2);
        app.update(Message::Frame);
        assert!(!app.playing);
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
        run(&mut app, Command::PlayOrStop);
        engine.process(&mut [0.0; 512], 2);
        assert!(engine_is_playing(&app));
        run(&mut app, Command::PlayOrStop);
        engine.process(&mut [0.0; 512], 2);
        assert!(!engine_is_playing(&app));
    }

    #[test]
    fn the_metronome_button_switches_the_click_of_the_engine() {
        let (mut app, mut engine) = with_simulated_audio();
        run(&mut app, Command::ToggleMetronome);
        run(&mut app, Command::PlayOrStop);
        let mut buffer = [0.0; 960];
        engine.process(&mut buffer, 2);
        assert!(app.metronome);
        assert!(buffer.iter().any(|sample| sample.abs() > 0.1));
    }

    #[test]
    fn transport_commands_without_an_audio_device_change_only_the_buttons() {
        let mut app = App::default();
        run(&mut app, Command::PlayOrStop);
        run(&mut app, Command::ToggleMetronome);
        assert!(app.metronome);
    }

    #[test]
    fn zoom_and_scroll_change_the_view_but_not_the_undo_history() {
        let mut app = App::default();
        run(
            &mut app,
            Command::ZoomTimeline {
                steps: 1.0,
                anchor: 0.0,
            },
        );
        run(&mut app, Command::ScrollTimeline { pixels: 80.0 });
        assert!((app.viewport.pixels_per_quarter_note() - 40.0).abs() < 1e-9);
        assert_eq!(app.viewport.start(), Position::from_quarter_notes(2));
        assert!(!app.history.can_undo());
    }

    #[test]
    fn mute_and_solo_switch_the_mix_of_a_track_on_and_off() {
        let mut app = App::default();
        let track = first_track(&app).id();
        run(&mut app, Command::ToggleMute(track));
        run(&mut app, Command::ToggleSolo(track));
        assert!(first_track(&app).mix().mute && first_track(&app).mix().solo);
        run(&mut app, Command::ToggleMute(track));
        assert!(!first_track(&app).mix().mute);
        run(&mut app, Command::Undo);
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
        for zone in ["1.1.1", "Browser", "Audio 1", "Inspector", "Mixer"] {
            assert!(ui.find(zone).is_ok(), "{zone}");
        }
    }
}
