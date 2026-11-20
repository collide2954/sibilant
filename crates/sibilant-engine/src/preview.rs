use std::sync::Arc;

use crate::arrangement::AudioBuffer;

const FADE_MILLISECONDS: usize = 4;

#[derive(Debug)]
pub(crate) struct PreviewVoice {
    audio: Option<Arc<AudioBuffer>>,
    frame: usize,
    fade_length: usize,
    fade_left: Option<usize>,
    gain: f32,
}

impl PreviewVoice {
    pub(crate) fn new(sample_rate: u32) -> Self {
        let rate = usize::try_from(sample_rate).unwrap_or(usize::MAX);
        Self {
            audio: None,
            frame: 0,
            fade_length: (rate * FADE_MILLISECONDS / 1_000).max(1),
            fade_left: None,
            gain: 1.0,
        }
    }

    pub(crate) fn start(&mut self, audio: Arc<AudioBuffer>) -> Option<Arc<AudioBuffer>> {
        self.frame = 0;
        self.fade_left = None;
        self.audio.replace(audio)
    }

    pub(crate) fn stop(&mut self) {
        if self.is_playing() && self.fade_left.is_none() {
            self.fade_left = Some(self.fade_length);
        }
    }

    pub(crate) fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.fade_left != Some(0)
            && self
                .audio
                .as_ref()
                .is_some_and(|audio| self.frame < audio.frames())
    }

    pub(crate) fn next_frame(&mut self) -> Option<[f32; 2]> {
        if !self.is_playing() {
            return None;
        }
        let [left, right] = self.audio.as_ref()?.frame(self.frame)?;
        self.frame += 1;
        let fade = match &mut self.fade_left {
            Some(frames) => {
                *frames -= 1;
                ratio(*frames, self.fade_length)
            }
            None => 1.0,
        };
        let gain = self.gain * fade;
        Some([left * gain, right * gain])
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "fade lengths stay far below 2^24 frames"
)]
fn ratio(part: usize, whole: usize) -> f32 {
    part as f32 / whole as f32
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::arrangement::AudioBuffer;
    use crate::preview::PreviewVoice;

    fn audio(samples: &[f32]) -> Arc<AudioBuffer> {
        Arc::new(AudioBuffer::new(vec![samples.to_vec()]))
    }

    fn left(voice: &mut PreviewVoice, frames: usize) -> Vec<Option<f32>> {
        (0..frames)
            .map(|_| voice.next_frame().map(|[left, _]| left))
            .collect()
    }

    #[test]
    fn a_new_voice_is_silent() {
        let mut voice = PreviewVoice::new(48_000);
        assert!(!voice.is_playing());
        assert_eq!(voice.next_frame(), None);
    }

    #[test]
    fn a_voice_plays_each_frame_of_its_audio_one_time() {
        let mut voice = PreviewVoice::new(48_000);
        voice.start(audio(&[0.25, 0.5, 0.75]));
        assert!(voice.is_playing());
        assert_eq!(voice.next_frame(), Some([0.25, 0.25]));
        assert_eq!(left(&mut voice, 3), [Some(0.5), Some(0.75), None]);
        assert!(!voice.is_playing());
    }

    #[test]
    fn the_gain_sets_the_level_of_the_preview() {
        let mut voice = PreviewVoice::new(48_000);
        voice.set_gain(0.5);
        voice.start(audio(&[0.5]));
        assert_eq!(voice.next_frame(), Some([0.25, 0.25]));
    }

    #[test]
    fn a_stop_fades_the_preview_out_in_4_ms() {
        let mut voice = PreviewVoice::new(1_000);
        voice.start(audio(&[1.0; 100]));
        assert_eq!(left(&mut voice, 2), [Some(1.0), Some(1.0)]);
        voice.stop();
        assert_eq!(
            left(&mut voice, 5),
            [Some(0.75), Some(0.5), Some(0.25), Some(0.0), None]
        );
        assert!(!voice.is_playing());
    }

    #[test]
    fn a_new_preview_plays_from_its_start_and_gives_back_the_old_audio() {
        let mut voice = PreviewVoice::new(48_000);
        let first = audio(&[0.1; 10]);
        voice.start(Arc::clone(&first));
        let _ = left(&mut voice, 5);
        let old = voice.start(audio(&[0.9, 0.8]));
        assert!(Arc::ptr_eq(&old.unwrap(), &first));
        assert_eq!(left(&mut voice, 2), [Some(0.9), Some(0.8)]);
    }

    #[test]
    fn stopping_a_silent_voice_changes_nothing() {
        let mut voice = PreviewVoice::new(48_000);
        voice.stop();
        assert!(!voice.is_playing());
        assert_eq!(voice.next_frame(), None);
    }
}
