use std::path::PathBuf;

use iced::{Element, Subscription, Task, Theme, keyboard, window};
use sibilant_core::history::History;
use sibilant_core::mix::Mix;
use sibilant_core::project::{EditError, Ids, Project, TrackId};
use sibilant_core::time::Position;
use sibilant_engine::engine;
use sibilant_media::{Entry, Place};
use sibilant_ui::browser::{self, Browser};
use sibilant_ui::command::Command;
use sibilant_ui::theme::Palette;
use sibilant_ui::timeline::{View, Viewport, Waveforms};
use sibilant_ui::transport::{self, Display};
use sibilant_ui::{shell, track_area};

use crate::audio::Audio;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Command(Command),
    Browser(browser::Event),
    Places(Vec<Place>),
    Listed(PathBuf, Result<Vec<Entry>, String>),
    Frame,
}

#[derive(Debug)]
pub(crate) struct App {
    palette: Palette,
    history: History<Project>,
    ids: Ids,
    viewport: Viewport,
    browser: Browser,
    waveforms: Waveforms,
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
            browser: Browser::default(),
            waveforms: Waveforms::new(),
            audio: None,
            metronome: false,
            playing: false,
        }
    }
}

impl App {
    pub(crate) fn with_audio() -> (Self, Task<Message>) {
        let (mut app, task) = Self::boot();
        match Audio::start(app.history.present()) {
            Ok(audio) => app.audio = Some(audio),
            Err(error) => eprintln!("audio output is not available: {error}"),
        }
        (app, task)
    }

    fn boot() -> (Self, Task<Message>) {
        let places = Task::perform(async { sibilant_media::places() }, Message::Places);
        (Self::default(), places)
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Command(command) => self.run(command),
            Message::Browser(event) => return self.browse(&event),
            Message::Places(places) => {
                self.browser = Browser::new(places);
                return self.browse(&browser::Event::Choose(0));
            }
            Message::Listed(folder, result) => self.browser.listed(folder, result),
            Message::Frame => {
                self.playing = self
                    .audio
                    .as_ref()
                    .is_some_and(|audio| audio.controller.is_playing());
            }
        }
        Task::none()
    }

    fn browse(&mut self, event: &browser::Event) -> Task<Message> {
        let Some(folder) = self.browser.update(event) else {
            return Task::none();
        };
        let listed = folder.clone();
        Task::perform(
            async move { sibilant_media::list_folder(&folder).map_err(|error| error.to_string()) },
            move |result| Message::Listed(listed, result),
        )
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
            Command::Drop { .. } => {}
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
        let transport = transport::view(display, self.palette).map(Message::Command);
        let browser = browser::view(&self.browser, self.palette).map(Message::Browser);
        let timeline = View {
            viewport: self.viewport,
            playhead: position,
            waveforms: &self.waveforms,
            dragging: false,
        };
        let tracks = track_area::view(project, timeline, self.palette).map(Message::Command);
        shell::view(self.palette, transport, browser, tracks)
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
    use std::path::PathBuf;

    use iced_test::simulator;
    use sibilant_core::project::{Project, Track, TrackId};
    use sibilant_core::time::Position;
    use sibilant_engine::engine::Engine;
    use sibilant_media::{Entry, EntryKind, Place};
    use sibilant_ui::browser;
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
        let _ = app.update(Message::Command(command));
    }

    fn music() -> PathBuf {
        PathBuf::from("music")
    }

    fn with_music_place() -> App {
        let mut app = App::default();
        let _ = app.update(Message::Places(vec![Place {
            name: "Music".into(),
            path: music(),
        }]));
        app
    }

    #[test]
    fn the_app_looks_for_the_places_of_the_browser_when_it_starts() {
        assert_eq!(App::boot().1.units(), 1);
    }

    #[test]
    fn the_first_place_opens_in_the_background_when_the_places_arrive() {
        let mut app = App::default();
        let places = vec![Place {
            name: "Music".into(),
            path: music(),
        }];
        assert_eq!(app.update(Message::Places(places)).units(), 1);
        assert!(simulator(app.view()).find("Loading").is_ok());
    }

    #[test]
    fn the_browser_shows_the_entries_of_a_listed_folder() {
        let mut app = with_music_place();
        let kick = Entry {
            name: "kick.wav".into(),
            path: music().join("kick.wav"),
            kind: EntryKind::Audio,
        };
        let _ = app.update(Message::Listed(music(), Ok(vec![kick])));
        assert!(simulator(app.view()).find("kick.wav").is_ok());
    }

    #[test]
    fn a_folder_opens_in_the_background() {
        let mut app = with_music_place();
        let toggle = browser::Event::Toggle(music().join("Drums"));
        assert_eq!(app.update(Message::Browser(toggle)).units(), 1);
    }

    #[test]
    fn the_transport_shows_the_playhead_of_the_engine() {
        let (mut app, mut engine) = with_simulated_audio();
        run(&mut app, Command::PlayOrStop);
        engine.process(&mut vec![0.0; 96_000], 2);
        let _ = app.update(Message::Frame);
        let mut ui = simulator(app.view());
        assert!(ui.find("1.3.1").is_ok());
        assert!(ui.find("0:01.000").is_ok());
    }

    #[test]
    fn playing_follows_the_engine_on_each_frame() {
        let (mut app, mut engine) = with_simulated_audio();
        run(&mut app, Command::PlayOrStop);
        engine.process(&mut [0.0; 512], 2);
        let _ = app.update(Message::Frame);
        assert!(app.playing);
        run(&mut app, Command::PlayOrStop);
        engine.process(&mut [0.0; 512], 2);
        let _ = app.update(Message::Frame);
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
