use std::collections::{BTreeMap, BTreeSet};
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
use sibilant_project::Opened;
use sibilant_ui::browser::{self, Browser, Request};
use sibilant_ui::command::Command;
use sibilant_ui::theme::Palette;
use sibilant_ui::timeline::{View, Viewport, Waveforms};
use sibilant_ui::transport::{self, Display};
use sibilant_ui::{shell, track_area};

use crate::audio::Audio;
use crate::library::{self, LoadedAudio};

mod dialogs;
mod document;
mod file;
mod preview;

use document::{Document, DocumentId};
use file::{Saving, Target};
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
    SaveAs(Option<PathBuf>),
    Saved(Box<Saving>),
    OpenFrom(Option<PathBuf>),
    Opened(Box<Result<Opened, String>>),
    SourceLoaded(DocumentId, SourceId, Result<LoadedAudio, String>),
    DragEnded,
    Frame,
}

#[derive(Debug)]
pub(crate) struct App {
    palette: Palette,
    history: History<Project>,
    document: Document,
    ids: Ids,
    viewport: Viewport,
    browser: Browser,
    dragging: Option<PathBuf>,
    previews: Previews,
    pool: BTreeMap<SourceId, Arc<AudioBuffer>>,
    waveforms: Waveforms,
    missing: BTreeSet<String>,
    audio: Option<Audio>,
    metronome: bool,
    playing: bool,
    saving: bool,
}

impl Default for App {
    fn default() -> Self {
        let mut ids = Ids::default();
        let project = Project::new(&mut ids);
        Self {
            palette: Palette::default(),
            document: Document::new(&project, None),
            history: History::new(project),
            ids,
            viewport: Viewport::default(),
            browser: Browser::default(),
            dragging: None,
            previews: Previews::default(),
            pool: BTreeMap::new(),
            waveforms: Waveforms::new(),
            missing: BTreeSet::new(),
            audio: None,
            metronome: false,
            playing: false,
            saving: false,
        }
    }
}

impl App {
    pub(crate) fn with_audio() -> (Self, Task<Message>) {
        let (mut app, task) = Self::boot();
        match Audio::start(app.history.present()) {
            Ok(audio) => app.attach_audio(audio),
            Err(error) => eprintln!("audio output is not available: {error}"),
        }
        (app, task)
    }

    fn attach_audio(&mut self, audio: Audio) {
        self.audio = Some(audio);
        self.send(engine::Command::SetPreviewGain(self.browser.preview_gain()));
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
                self.browser.set_places(places);
                return self.browse(&browser::Event::Choose(0));
            }
            Message::Listed(folder, result) => self.browser.listed(folder, result),
            Message::Loaded(placement, Ok(loaded)) => self.place(placement, loaded),
            Message::Loaded(_, Err(message)) => self.browser.set_notice(Some(message)),
            Message::PreviewLoaded(file, result) => self.preview_loaded(file, result),
            Message::SaveAs(Some(chosen)) => return self.save_to(Target::Chosen(chosen)),
            Message::Saved(saving) => self.finish_saving(*saving),
            Message::OpenFrom(Some(file)) => return file::open(file),
            Message::SaveAs(None) | Message::OpenFrom(None) => {}
            Message::Opened(result) => return self.finish_opening(*result),
            Message::SourceLoaded(document, source, result) => {
                self.source_loaded(document, source, result);
            }
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
            Some(Request::Volume(gain)) => {
                self.send(engine::Command::SetPreviewGain(gain));
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
            Command::Open => return dialogs::open_file().map(Message::OpenFrom),
            Command::Save => return self.save(),
            Command::SaveAs => return self.save_as(),
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

    pub(crate) fn title(&self) -> String {
        self.document.title(self.history.present())
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
    use sibilant_core::project::{AudioSource, Clip, Project, Region, SourceId, Track, TrackId};
    use sibilant_core::time::{Duration, Musical, Position};
    use sibilant_engine::arrangement::AudioBuffer;
    use sibilant_engine::engine::Engine;
    use sibilant_media::{Entry, EntryKind, Peaks, Place};
    use sibilant_project::{Copies, Location, Opened};
    use sibilant_ui::browser;
    use sibilant_ui::command::Command;
    use sibilant_ui::icons::Icon;
    use sibilant_ui::theme::Palette;
    use sibilant_ui::timeline::Viewport;

    use crate::app::file::{Saved, Saving};
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
    fn a_preview_plays_at_minus_6_db_when_the_audio_starts() {
        let (mut app, mut engine) = with_simulated_audio();
        hear_kick(&mut app, 48_000);
        let level = loudest(&render(&mut engine, 480));
        assert!((level - 0.5 * 0.501_19).abs() < 1e-3, "level {level}");
    }

    #[test]
    fn a_new_preview_volume_reaches_the_engine() {
        let (mut app, mut engine) = with_simulated_audio();
        let _ = app.update(Message::Browser(browser::Event::Volume(-12.0)));
        hear_kick(&mut app, 48_000);
        let level = loudest(&render(&mut engine, 480));
        assert!((level - 0.5 * 0.251_19).abs() < 1e-3, "level {level}");
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
        app.attach_audio(audio);
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

    fn my_song() -> PathBuf {
        PathBuf::from("songs")
            .join("MySong")
            .join("MySong.sibilant")
    }

    fn finish_saving(app: &mut App, project: Project, result: Result<Saved, String>) {
        let saving = Saving {
            document: app.document.id(),
            project,
            result,
        };
        let _ = app.update(Message::Saved(Box::new(saving)));
    }

    const KICK: SourceId = SourceId::new(5);
    const PAD: SourceId = SourceId::new(6);
    const DRUMS: TrackId = TrackId::new(7);

    fn opened_project() -> Project {
        let second = Duration::from_seconds(1);
        let source = |id, file: &str| AudioSource::new(id, Path::new("audio").join(file), second);
        let region = Region {
            start: Duration::ZERO,
            length: second,
        };
        Project::default()
            .add_source(source(KICK, "Kick.wav"))
            .and_then(|project| project.add_source(source(PAD, "Pad.wav")))
            .and_then(|project| project.add_track(DRUMS, "Drums"))
            .and_then(|project| {
                project.place_clip(DRUMS, Position::START, Clip::new("Kick", KICK, region))
            })
            .unwrap()
    }

    fn open_my_song(app: &mut App) -> Task<Message> {
        let opened = Opened {
            project: opened_project(),
            location: Location::of_file(my_song()),
        };
        app.update(Message::Opened(Box::new(Ok(opened))))
    }

    fn load(app: &mut App, source: SourceId, result: Result<LoadedAudio, String>) {
        let _ = app.update(Message::SourceLoaded(app.document.id(), source, result));
    }

    #[test]
    fn open_asks_which_project_to_open() {
        let mut app = App::default();
        assert_eq!(app.update(Message::Command(Command::Open)).units(), 1);
    }

    #[test]
    fn a_chosen_project_file_opens_in_the_background() {
        let mut app = App::default();
        assert_eq!(app.update(Message::OpenFrom(Some(my_song()))).units(), 1);
    }

    #[test]
    fn a_closed_open_dialog_opens_nothing() {
        let mut app = App::default();
        assert_eq!(app.update(Message::OpenFrom(None)).units(), 0);
    }

    #[test]
    fn an_opened_project_replaces_the_project_and_its_history() {
        let mut app = App::default();
        run(&mut app, Command::AddTrack);
        let loads = open_my_song(&mut app);
        assert_eq!(loads.units(), 2);
        assert_eq!(track_names(&app), ["Drums"]);
        assert_eq!(app.title(), "MySong — Sibilant");
        assert!(!app.history.can_undo());
    }

    #[test]
    fn new_ids_come_after_the_ids_of_the_opened_project() {
        let mut app = App::default();
        let _ = open_my_song(&mut app);
        run(&mut app, Command::AddTrack);
        assert_eq!(project(&app).tracks().last().unwrap().id(), TrackId::new(8));
    }

    #[test]
    fn the_audio_of_an_opened_project_plays_when_it_has_loaded() {
        let (mut app, mut engine) = with_simulated_audio();
        let _ = open_my_song(&mut app);
        load(
            &mut app,
            KICK,
            Ok(audio_file(PathBuf::from("Kick.wav"), 48_000)),
        );
        run(&mut app, Command::PlayOrStop);
        assert!(loudest(&render(&mut engine, 480)) > 0.4);
        assert!(app.waveforms.contains_key(&KICK));
    }

    #[test]
    fn audio_that_arrives_for_an_older_project_is_dropped() {
        let mut app = App::default();
        let _ = open_my_song(&mut app);
        let older = app.document.id();
        let _ = open_my_song(&mut app);
        let kick = audio_file(PathBuf::from("Kick.wav"), 480);
        let _ = app.update(Message::SourceLoaded(older, KICK, Ok(kick)));
        assert!(!app.waveforms.contains_key(&KICK));
    }

    #[test]
    fn the_audio_of_the_project_before_does_not_play_in_the_opened_project() {
        let (mut app, mut engine) = with_simulated_audio();
        let _ = open_my_song(&mut app);
        load(
            &mut app,
            KICK,
            Ok(audio_file(PathBuf::from("Kick.wav"), 48_000)),
        );
        let _ = open_my_song(&mut app);
        run(&mut app, Command::PlayOrStop);
        assert_eq!(loudest(&render(&mut engine, 480)), 0.0);
        assert!(!app.waveforms.contains_key(&KICK));
    }

    #[test]
    fn an_opened_project_shows_its_timeline_from_the_start() {
        let mut app = App::default();
        run(&mut app, Command::ScrollTimeline { pixels: 80.0 });
        let _ = open_my_song(&mut app);
        assert_eq!(app.viewport, Viewport::default());
    }

    #[test]
    fn an_opened_project_removes_the_notice_of_the_project_before() {
        let mut app = App::default();
        let _ = app.update(Message::Opened(Box::new(Err("no file".into()))));
        let _ = open_my_song(&mut app);
        let notice = "Sibilant did not open the project: no file";
        assert!(simulator(app.view()).find(notice).is_err());
    }

    #[test]
    fn the_notice_names_only_the_missing_files_of_the_opened_project() {
        let mut app = App::default();
        let _ = open_my_song(&mut app);
        load(&mut app, KICK, Err("Kick.wav: cannot open the file".into()));
        let _ = open_my_song(&mut app);
        load(&mut app, PAD, Err("Pad.wav: cannot open the file".into()));
        let notice = "These audio files are missing or damaged: Pad.wav";
        assert!(simulator(app.view()).find(notice).is_ok());
    }

    #[test]
    fn missing_audio_files_are_named_in_the_browser() {
        let mut app = App::default();
        let _ = open_my_song(&mut app);
        load(&mut app, KICK, Err("Kick.wav: cannot open the file".into()));
        load(&mut app, PAD, Err("Pad.wav: cannot open the file".into()));
        let notice = "These audio files are missing or damaged: Kick.wav, Pad.wav";
        assert!(simulator(app.view()).find(notice).is_ok());
    }

    #[test]
    fn a_project_that_cannot_be_opened_tells_why() {
        let mut app = App::default();
        let failure = Err("the file is not a Sibilant project".into());
        let _ = app.update(Message::Opened(Box::new(failure)));
        let notice = "Sibilant did not open the project: the file is not a Sibilant project";
        assert!(simulator(app.view()).find(notice).is_ok());
        assert_eq!(track_names(&app), ["Audio 1", "Audio 2"]);
    }

    #[test]
    fn opening_a_project_stops_the_playback() {
        let (mut app, mut engine) = with_simulated_audio();
        run(&mut app, Command::PlayOrStop);
        let _ = render(&mut engine, 480);
        let _ = open_my_song(&mut app);
        let _ = render(&mut engine, 480);
        assert!(!engine_is_playing(&app));
    }

    #[test]
    fn a_save_that_ends_after_an_open_leaves_the_opened_project_alone() {
        let mut app = App::default();
        let present = project(&app).clone();
        let older = app.document.id();
        let _ = app.update(Message::SaveAs(Some(PathBuf::from("Other.sibilant"))));
        let _ = open_my_song(&mut app);
        let saved = Saved {
            location: Location::of_file(Path::new("Other").join("Other.sibilant")),
            copies: Copies::default(),
        };
        let saving = Saving {
            document: older,
            project: present,
            result: Ok(saved),
        };
        let _ = app.update(Message::Saved(Box::new(saving)));
        assert_eq!(app.title(), "MySong — Sibilant");
        assert!(!app.saving);
    }

    fn saved_as_my_song(app: &mut App, project: Project) {
        let saved = Saved {
            location: Location::of_file(my_song()),
            copies: Copies::default(),
        };
        finish_saving(app, project, Ok(saved));
    }

    #[test]
    fn save_of_a_project_without_a_file_asks_where_to_save_it() {
        let mut app = App::default();
        assert_eq!(app.update(Message::Command(Command::Save)).units(), 1);
        assert!(!app.saving);
    }

    #[test]
    fn save_as_asks_where_to_save_the_project() {
        let mut app = App::default();
        assert_eq!(app.update(Message::Command(Command::SaveAs)).units(), 1);
        assert!(!app.saving);
    }

    #[test]
    fn a_chosen_file_saves_the_project_in_the_background() {
        let mut app = App::default();
        assert_eq!(app.update(Message::SaveAs(Some(my_song()))).units(), 1);
        assert!(app.saving);
    }

    #[test]
    fn a_closed_save_dialog_saves_nothing() {
        let mut app = App::default();
        assert_eq!(app.update(Message::SaveAs(None)).units(), 0);
        assert!(!app.saving);
    }

    #[test]
    fn a_saved_project_shows_its_name_and_saves_again_without_a_dialog() {
        let mut app = App::default();
        let present = project(&app).clone();
        saved_as_my_song(&mut app, present);
        assert_eq!(app.title(), "MySong — Sibilant");
        assert_eq!(app.update(Message::Command(Command::Save)).units(), 1);
        assert!(app.saving);
    }

    #[test]
    fn changes_made_during_a_save_stay_unsaved() {
        let mut app = App::default();
        let at_start = project(&app).clone();
        let _ = app.update(Message::SaveAs(Some(my_song())));
        run(&mut app, Command::AddTrack);
        saved_as_my_song(&mut app, at_start);
        assert_eq!(app.title(), "● MySong — Sibilant");
        assert!(!app.saving);
    }

    #[test]
    fn a_second_save_waits_until_the_first_is_done() {
        let mut app = App::default();
        let _ = app.update(Message::SaveAs(Some(my_song())));
        assert_eq!(app.update(Message::SaveAs(Some(my_song()))).units(), 0);
    }

    #[test]
    fn a_failed_save_tells_why_in_the_browser() {
        let mut app = App::default();
        let present = project(&app).clone();
        let _ = app.update(Message::SaveAs(Some(my_song())));
        finish_saving(
            &mut app,
            present,
            Err("cannot write the project file".into()),
        );
        let notice = "Sibilant did not save the project: cannot write the project file";
        assert!(simulator(app.view()).find(notice).is_ok());
        assert!(!app.saving);
    }

    #[test]
    fn the_window_title_shows_the_project_and_its_unsaved_changes() {
        let mut app = App::default();
        assert_eq!(app.title(), "Untitled — Sibilant");
        run(&mut app, Command::AddTrack);
        assert_eq!(app.title(), "● Untitled — Sibilant");
        run(&mut app, Command::Undo);
        assert_eq!(app.title(), "Untitled — Sibilant");
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
