use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::Task;
use sibilant_engine::arrangement::AudioBuffer;
use sibilant_engine::engine;

use super::{App, Message};
use crate::audio::Audio;
use crate::library::{self, LoadedAudio};

#[derive(Debug, Default)]
pub(super) struct Previews {
    wanted: Option<PathBuf>,
    playing: Option<PathBuf>,
    sent: usize,
    cached: Option<LoadedAudio>,
}

impl Previews {
    pub(super) fn is_playing(&self) -> bool {
        self.playing.is_some()
    }

    pub(super) fn cached(&self, file: &Path) -> Option<&LoadedAudio> {
        self.cached.as_ref().filter(|cached| cached.path == file)
    }
}

impl App {
    pub(super) fn start_preview(&mut self, file: PathBuf) -> Task<Message> {
        let Some(sample_rate) = self.audio.as_ref().map(Audio::sample_rate) else {
            return Task::none();
        };
        self.previews.wanted = Some(file.clone());
        self.browser.set_previewing(Some(file.clone()));
        if let Some(cached) = self.previews.cached(&file) {
            let playback = Arc::clone(&cached.playback);
            self.play_preview(file, playback);
            return Task::none();
        }
        let loading = file.clone();
        Task::perform(
            async move { library::load(&loading, Some(sample_rate)) },
            move |result| Message::PreviewLoaded(file, result),
        )
    }

    pub(super) fn preview_loaded(&mut self, file: PathBuf, result: Result<LoadedAudio, String>) {
        let wanted = self.previews.wanted.as_ref() == Some(&file);
        match result {
            Ok(loaded) => {
                let playback = Arc::clone(&loaded.playback);
                self.previews.cached = Some(loaded);
                if wanted {
                    self.play_preview(file, playback);
                }
            }
            Err(message) => {
                self.browser.set_notice(Some(message));
                if wanted {
                    self.stop_preview();
                }
            }
        }
    }

    pub(super) fn stop_preview(&mut self) {
        self.previews.wanted = None;
        self.previews.playing = None;
        self.browser.set_previewing(None);
        self.send(engine::Command::StopPreview);
    }

    pub(super) fn check_preview(&mut self) {
        let ended = self.audio.as_ref().is_some_and(|audio| {
            audio.controller.previews_started() >= self.previews.sent
                && !audio.controller.is_previewing()
        });
        if self.previews.is_playing() && ended {
            self.previews.wanted = None;
            self.previews.playing = None;
            self.browser.set_previewing(None);
        }
    }

    fn play_preview(&mut self, file: PathBuf, playback: Arc<AudioBuffer>) {
        let Some(audio) = self.audio.as_mut() else {
            return;
        };
        audio.controller.release_old();
        if audio.controller.preview(playback) {
            self.previews.sent += 1;
            self.previews.playing = Some(file);
        }
    }
}
