use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use iced::{Element, Subscription, Task, Theme, event, keyboard, mouse, window};
use sibilant_core::history::History;
use sibilant_core::mix::Mix;
use sibilant_core::project::{
    AudioSource, Clip, EditError, Ids, Project, Region, SourceId, TrackId,
};
use sibilant_core::time::{Duration, Musical, Position};
use sibilant_engine::arrangement::{Arrangement, AudioBuffer};
use sibilant_engine::engine;
use sibilant_media::{Entry, Place};
use sibilant_ui::browser::{self, Browser, Request};
use sibilant_ui::command::Command;
use sibilant_ui::theme::Palette;
use sibilant_ui::timeline::{View, Viewport, Waveforms};
use sibilant_ui::transport::{self, Display};
use sibilant_ui::{shell, track_area};

use crate::audio::Audio;
use crate::library::{self, LoadedAudio};

mod preview;

use preview::Previews;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placement {
    pub(crate) track: Option<TrackId>,
    pub(crate) start: Position<Musical>,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Command(Command),
    Browser(browser::Event),
    Places(Vec<Place>),
    Listed(PathBuf, Result<Vec<Entry>, String>),
    Loaded(Placement, Result<LoadedAudio, String>),
    PreviewLoaded(PathBuf, Result<LoadedAudio, String>),
    DragEnded,
    Frame,
}

#[derive(Debug)]
pub(crate) struct App {
    palette: Palette,
    history: History<Project>,
    ids: Ids,
    viewport: Viewport,
    browser: Browser,
    dragging: Option<PathBuf>,
    previews: Previews,
    pool: BTreeMap<SourceId, Arc<AudioBuffer>>,
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
            dragging: None,
            previews: Previews::default(),
            pool: BTreeMap::new(),
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
            Message::Command(command) => return self.run(command),
            Message::Browser(event) => return self.browse(&event),
            Message::Places(places) => {
                self.browser = Browser::new(places);
                return self.browse(&browser::Event::Choose(0));
            }
            Message::Listed(folder, result) => self.browser.listed(folder, result),
            Message::Loaded(placement, Ok(loaded)) => self.place(placement, loaded),
            Message::Loaded(_, Err(message)) => self.browser.set_notice(Some(message)),
            Message::PreviewLoaded(file, result) => self.preview_loaded(file, result),
            Message::DragEnded => self.dragging = None,
            Message::Frame => {
                if let Some(audio) = self.audio.as_mut() {
                    audio.controller.release_old();
                }
                self.playing = self
                    .audio
                    .as_ref()
                    .is_some_and(|audio| audio.controller.is_playing());
                self.check_preview();
            }
        }
        Task::none()
    }

    fn import(&mut self, placement: Placement) -> Task<Message> {
        let Some(path) = self.dragging.take() else {
            return Task::none();
        };
        if let Some(previewed) = self.previews.cached(&path).cloned() {
            self.place(placement, previewed);
            return Task::none();
        }
        let sample_rate = self.audio.as_ref().map(Audio::sample_rate);
        Task::perform(
            async move { library::load(&path, sample_rate) },
            move |result| Message::Loaded(placement, result),
        )
    }

    fn place(&mut self, placement: Placement, loaded: LoadedAudio) {
        let source = self.ids.source();
        let track = placement.track.unwrap_or_else(|| self.ids.track());
        let region = Region {
            start: Duration::ZERO,
            length: loaded.length,
        };
        let clip = Clip::new(loaded.name.clone(), source, region);
        let audio_source = AudioSource::new(source, loaded.path, loaded.length);
        let edited = self
            .history
            .present()
            .add_source(audio_source)
            .and_then(|project| match placement.track {
                Some(_) => Ok(project),
                None => project.add_track(track, project.next_track_name()),
            })
            .and_then(|project| project.place_clip(track, placement.start, clip));
        match edited {
            Ok(project) => {
                self.pool.insert(source, loaded.playback);
                self.waveforms.insert(source, loaded.peaks);
                self.history.commit(project);
                self.sync();
            }
            Err(error) => self
                .browser
                .set_notice(Some(format!("{}: {error}", loaded.name))),
        }
    }

    fn sync(&mut self) {
        let Some(audio) = self.audio.as_mut() else {
            return;
        };
        let pool = &self.pool;
        let arrangement =
            Arrangement::of_project(self.history.present(), audio.sample_rate(), |source| {
                pool.get(&source).cloned()
            });
        audio.controller.release_old();
        audio.controller.replace_arrangement(arrangement);
    }

    fn browse(&mut self, event: &browser::Event) -> Task<Message> {
        match self.browser.update(event) {
            Some(Request::List(folder)) => {
                let listed = folder.clone();
                Task::perform(
                    async move {
                        sibilant_media::list_folder(&folder).map_err(|error| error.to_string())
                    },
                    move |result| Message::Listed(listed, result),
                )
            }
            Some(Request::Drag(file)) => {
                self.dragging = Some(file);
                self.browser.set_notice(None);
                Task::none()
            }
            Some(Request::Preview(file)) => self.start_preview(file),
            Some(Request::StopPreview) => {
                self.stop_preview();
                Task::none()
            }
            None => Task::none(),
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let shortcuts = keyboard::listen()
            .filter_map(Command::from_key_event)
            .map(Message::Command);
        let frames = (self.playing || self.previews.is_playing())
            .then(|| window::frames().map(|_| Message::Frame));
        let drag_end = self
            .dragging
            .is_some()
            .then(|| event::listen_with(release_that_no_track_takes));
        Subscription::batch([Some(shortcuts), frames, drag_end].into_iter().flatten())
    }

    fn run(&mut self, command: Command) -> Task<Message> {
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
                self.sync();
            }
            Command::Redo => {
                self.history.redo();
                self.sync();
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
            Command::Drop { track, start } => return self.import(Placement { track, start }),
            Command::StopPreview => self.stop_preview(),
        }
        Task::none()
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
            dragging: self.dragging.is_some(),
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
            self.sync();
        }
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "event::listen_with gives each event by value"
)]
fn release_that_no_track_takes(
    event: iced::Event,
    status: event::Status,
    _window: window::Id,
) -> Option<Message> {
    let release = iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
    (event == release && status == event::Status::Ignored).then_some(Message::DragEnded)
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
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use iced::Task;
    use iced_test::simulator;
    use sibilant_core::project::{Project, Track, TrackId};
    use sibilant_core::time::{Duration, Musical, Position};
    use sibilant_engine::arrangement::AudioBuffer;
    use sibilant_engine::engine::Engine;
    use sibilant_media::{Entry, EntryKind, Peaks, Place};
    use sibilant_ui::browser;
    use sibilant_ui::command::Command;
    use sibilant_ui::icons::Icon;
    use sibilant_ui::theme::Palette;

    use crate::app::{App, Message, Placement, release_that_no_track_takes};
    use crate::audio::Audio;
    use crate::library::LoadedAudio;

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

    fn audio_file(path: PathBuf, frames: u32) -> LoadedAudio {
        let samples = vec![0.5; usize::try_from(frames).unwrap()];
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        LoadedAudio {
            path,
            name,
            length: Duration::from_frames(i64::from(frames), 48_000),
            playback: Arc::new(AudioBuffer::new(vec![samples.clone()])),
            peaks: Arc::new(Peaks::of(&sibilant_media::Audio::new(
                48_000,
                vec![samples],
            ))),
        }
    }

    fn loaded(name: &str) -> LoadedAudio {
        audio_file(PathBuf::from(format!("{name}.wav")), 48_000)
    }

    fn click(app: &mut App, file: PathBuf) -> Task<Message> {
        let _ = app.update(Message::Browser(browser::Event::Press(file.clone())));
        app.update(Message::Browser(browser::Event::Release(file)))
    }

    fn list_kick(app: &mut App) {
        let _ = app.update(Message::Places(vec![Place {
            name: "Music".into(),
            path: music(),
        }]));
        let entry = Entry {
            name: "kick.wav".into(),
            path: kick(),
            kind: EntryKind::Audio,
        };
        let _ = app.update(Message::Listed(music(), Ok(vec![entry])));
    }

    fn shows_a_speaker(app: &App) -> bool {
        let speaker = Icon::Speaker.codepoint().to_string();
        simulator(app.view()).find(speaker.as_str()).is_ok()
    }

    fn hear_kick(app: &mut App, frames: u32) {
        let _ = click(app, kick());
        let _ = app.update(Message::PreviewLoaded(
            kick(),
            Ok(audio_file(kick(), frames)),
        ));
    }

    fn render(engine: &mut Engine, frames: usize) -> Vec<f32> {
        let mut buffer = vec![0.0; frames * 2];
        engine.process(&mut buffer, 2);
        buffer
    }

    #[test]
    fn a_click_on_a_file_loads_it_for_a_preview_in_the_background() {
        let (mut app, _engine) = with_simulated_audio();
        assert_eq!(click(&mut app, kick()).units(), 1);
    }

    #[test]
    fn a_loaded_preview_plays_in_the_engine_with_a_speaker_on_its_file() {
        let (mut app, mut engine) = with_simulated_audio();
        list_kick(&mut app);
        hear_kick(&mut app, 48_000);
        assert!(loudest(&render(&mut engine, 480)) > 0.2);
        assert!(shows_a_speaker(&app));
    }

    #[test]
    fn a_preview_stopped_before_it_loaded_does_not_play() {
        let (mut app, mut engine) = with_simulated_audio();
        let _ = click(&mut app, kick());
        run(&mut app, Command::StopPreview);
        let _ = app.update(Message::PreviewLoaded(
            kick(),
            Ok(audio_file(kick(), 48_000)),
        ));
        assert_eq!(loudest(&render(&mut engine, 480)), 0.0);
    }

    #[test]
    fn a_second_click_on_the_file_that_plays_stops_it() {
        let (mut app, mut engine) = with_simulated_audio();
        list_kick(&mut app);
        hear_kick(&mut app, 48_000);
        let _ = render(&mut engine, 480);
        let _ = click(&mut app, kick());
        assert_eq!(loudest(&render(&mut engine, 480)[400..]), 0.0);
        assert!(!shows_a_speaker(&app));
    }

    #[test]
    fn escape_stops_the_preview() {
        let (mut app, mut engine) = with_simulated_audio();
        hear_kick(&mut app, 48_000);
        let _ = render(&mut engine, 480);
        run(&mut app, Command::StopPreview);
        assert_eq!(loudest(&render(&mut engine, 480)[400..]), 0.0);
    }

    #[test]
    fn the_speaker_stays_until_the_engine_ends_the_preview() {
        let (mut app, mut engine) = with_simulated_audio();
        list_kick(&mut app);
        hear_kick(&mut app, 100);
        let _ = app.update(Message::Frame);
        assert!(shows_a_speaker(&app));
        let _ = render(&mut engine, 480);
        let _ = app.update(Message::Frame);
        assert!(!shows_a_speaker(&app));
    }

    #[test]
    fn a_previewed_file_goes_on_a_track_without_a_second_load() {
        let (mut app, _engine) = with_simulated_audio();
        hear_kick(&mut app, 48_000);
        grab(&mut app);
        let track = first_track(&app).id();
        let drop = Command::Drop {
            track: Some(track),
            start: Position::START,
        };
        assert_eq!(app.update(Message::Command(drop)).units(), 0);
        assert_eq!(first_track(&app).timeline().len(), 1);
    }

    #[test]
    fn a_preview_that_cannot_be_loaded_tells_why() {
        let (mut app, _engine) = with_simulated_audio();
        let _ = click(&mut app, kick());
        let failure = Err("kick.wav: not a WAV file".into());
        let _ = app.update(Message::PreviewLoaded(kick(), failure));
        assert!(
            simulator(app.view())
                .find("kick.wav: not a WAV file")
                .is_ok()
        );
    }

    fn place(app: &mut App, track: Option<TrackId>, start: Position<Musical>, name: &str) {
        let placement = Placement { track, start };
        let _ = app.update(Message::Loaded(placement, Ok(loaded(name))));
    }

    fn kick() -> PathBuf {
        music().join("kick.wav")
    }

    fn grab(app: &mut App) {
        let _ = app.update(Message::Browser(browser::Event::Press(kick())));
        let _ = app.update(Message::Browser(browser::Event::Leave(kick())));
    }

    #[test]
    fn a_click_on_a_file_does_not_start_a_drag() {
        let mut app = App::default();
        let _ = app.update(Message::Browser(browser::Event::Press(kick())));
        let _ = app.update(Message::Browser(browser::Event::Release(kick())));
        assert!(!shows_the_new_track_hint(&app));
    }

    fn shows_the_new_track_hint(app: &App) -> bool {
        simulator(app.view())
            .find("Drop here for a new track")
            .is_ok()
    }

    fn drop_for_a_new_track() -> Message {
        Message::Command(Command::Drop {
            track: None,
            start: Position::START,
        })
    }

    #[test]
    fn a_press_on_a_file_in_the_browser_starts_a_drag() {
        let mut app = App::default();
        grab(&mut app);
        assert!(shows_the_new_track_hint(&app));
    }

    #[test]
    fn a_release_that_no_track_takes_ends_the_drag() {
        let mut app = App::default();
        grab(&mut app);
        let _ = app.update(Message::DragEnded);
        assert!(!shows_the_new_track_hint(&app));
    }

    #[test]
    fn a_drop_loads_the_dragged_file_in_the_background_and_ends_the_drag() {
        let mut app = App::default();
        grab(&mut app);
        assert_eq!(app.update(drop_for_a_new_track()).units(), 1);
        assert!(!shows_the_new_track_hint(&app));
    }

    #[test]
    fn only_a_left_release_that_no_track_takes_ends_a_drag() {
        let release = |button| iced::Event::Mouse(iced::mouse::Event::ButtonReleased(button));
        let left = release(iced::mouse::Button::Left);
        let window = iced::window::Id::unique();
        let ends = |event, status| {
            release_that_no_track_takes(event, status, window)
                .is_some_and(|message| matches!(message, Message::DragEnded))
        };
        assert!(ends(left.clone(), iced::event::Status::Ignored));
        assert!(!ends(left, iced::event::Status::Captured));
        let right = release(iced::mouse::Button::Right);
        assert!(!ends(right, iced::event::Status::Ignored));
    }

    #[test]
    fn a_drop_without_a_dragged_file_loads_nothing() {
        let mut app = App::default();
        assert_eq!(app.update(drop_for_a_new_track()).units(), 0);
    }

    #[test]
    fn a_loaded_file_becomes_a_clip_at_the_drop_position() {
        let mut app = App::default();
        let track = first_track(&app).id();
        place(
            &mut app,
            Some(track),
            Position::from_quarter_notes(4),
            "kick",
        );
        let (start, clip) = first_track(&app).timeline().next().unwrap();
        assert_eq!(start, Position::from_quarter_notes(4));
        assert_eq!(clip.name(), "kick");
        let source = clip.source();
        assert_eq!(
            project(&app).source(source).unwrap().path(),
            Path::new("kick.wav")
        );
        assert!(app.waveforms.contains_key(&source));
    }

    #[test]
    fn one_undo_removes_the_clip_and_its_source() {
        let mut app = App::default();
        let track = first_track(&app).id();
        place(&mut app, Some(track), Position::START, "kick");
        run(&mut app, Command::Undo);
        assert_eq!(first_track(&app).timeline().len(), 0);
        assert_eq!(project(&app).sources().len(), 0);
    }

    #[test]
    fn a_file_dropped_for_a_new_track_gets_its_own_track() {
        let mut app = App::default();
        place(&mut app, None, Position::START, "pad");
        assert_eq!(track_names(&app), ["Audio 1", "Audio 2", "Audio 3"]);
        let added = project(&app).tracks().last().unwrap();
        assert_eq!(added.timeline().next().unwrap().1.name(), "pad");
        run(&mut app, Command::Undo);
        assert_eq!(track_names(&app).len(), 2);
    }

    #[test]
    fn a_file_that_cannot_be_loaded_shows_why_in_the_browser() {
        let mut app = App::default();
        let placement = Placement {
            track: None,
            start: Position::START,
        };
        let failure = Err("kick.wav: not a WAV file".into());
        let _ = app.update(Message::Loaded(placement, failure));
        assert!(
            simulator(app.view())
                .find("kick.wav: not a WAV file")
                .is_ok()
        );
    }

    fn loudest(buffer: &[f32]) -> f32 {
        buffer
            .iter()
            .fold(0.0, |loudest, sample| loudest.max(sample.abs()))
    }

    #[test]
    fn the_engine_plays_a_placed_clip_until_an_undo_removes_it() {
        let (mut app, mut engine) = with_simulated_audio();
        let track = first_track(&app).id();
        place(&mut app, Some(track), Position::START, "kick");
        run(&mut app, Command::PlayOrStop);
        let mut buffer = vec![0.0; 9_600];
        engine.process(&mut buffer, 2);
        assert!(loudest(&buffer) > 0.4);
        run(&mut app, Command::Undo);
        run(&mut app, Command::PlayOrStop);
        run(&mut app, Command::PlayOrStop);
        engine.process(&mut buffer, 2);
        assert_eq!(loudest(&buffer), 0.0);
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
