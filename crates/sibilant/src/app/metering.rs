use std::time::{Duration, Instant};

use sibilant_engine::engine::Levels;
use sibilant_ui::mixer::{Meter, Meters};

const FALL_PER_SECOND: f64 = 24.0;
const CLIP_HOLD: Duration = Duration::from_secs(2);
const SILENCE: f64 = -100.0;

#[derive(Debug, Default)]
pub(super) struct Metering {
    tracks: Vec<Falling>,
    master: Falling,
    last: Option<Instant>,
}

#[derive(Debug, Clone, Copy)]
struct Falling {
    db: f64,
    clipped_until: Option<Instant>,
}

impl Default for Falling {
    fn default() -> Self {
        Self {
            db: f64::NEG_INFINITY,
            clipped_until: None,
        }
    }
}

impl Metering {
    pub(super) fn update(&mut self, levels: &Levels, now: Instant) -> Meters {
        let elapsed = self
            .last
            .map_or(0.0, |last| now.duration_since(last).as_secs_f64());
        self.last = Some(now);
        self.tracks.resize(levels.tracks.len(), Falling::default());
        for (falling, &peak) in self.tracks.iter_mut().zip(&levels.tracks) {
            falling.take(peak, elapsed, now);
        }
        self.master.take(levels.output, elapsed, now);
        Meters {
            tracks: self
                .tracks
                .iter()
                .map(|falling| falling.meter(now))
                .collect(),
            master: self.master.meter(now),
        }
    }

    pub(super) fn is_moving(&self) -> bool {
        let Some(now) = self.last else {
            return false;
        };
        self.tracks
            .iter()
            .chain([&self.master])
            .any(|falling| falling.db > SILENCE || falling.meter(now).clipped)
    }
}

impl Falling {
    fn take(&mut self, peak: f32, elapsed: f64, now: Instant) {
        let db = 20.0 * f64::from(peak).log10();
        self.db = db.max(self.db - FALL_PER_SECOND * elapsed);
        if peak >= 1.0 {
            self.clipped_until = Some(now + CLIP_HOLD);
        }
    }

    fn meter(&self, now: Instant) -> Meter {
        Meter {
            level: to_f32(10_f64.powf(self.db / 20.0)),
            clipped: self.clipped_until.is_some_and(|until| now < until),
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "meter levels stay far inside the range of f32"
)]
fn to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use sibilant_engine::engine::Levels;

    use crate::app::metering::Metering;

    fn levels(tracks: &[f32], output: f32) -> Levels {
        Levels {
            tracks: tracks.to_vec(),
            output,
        }
    }

    fn later(start: Instant, milliseconds: u64) -> Instant {
        start + Duration::from_millis(milliseconds)
    }

    #[test]
    fn a_new_peak_shows_at_once() {
        let mut metering = Metering::default();
        let meters = metering.update(&levels(&[0.5], 0.25), Instant::now());
        assert_eq!(meters.tracks[0].level, 0.5);
        assert_eq!(meters.master.level, 0.25);
    }

    #[test]
    fn a_meter_falls_by_24_db_each_second() {
        let (mut metering, start) = (Metering::default(), Instant::now());
        let _ = metering.update(&levels(&[0.5], 0.0), start);
        let meters = metering.update(&levels(&[0.0], 0.0), later(start, 500));
        let expected = 10_f32.powf((20.0 * 0.5_f32.log10() - 12.0) / 20.0);
        let level = meters.tracks[0].level;
        assert!(
            (level - expected).abs() < 1e-4,
            "{level} instead of {expected}"
        );
    }

    #[test]
    fn a_higher_peak_replaces_the_falling_level() {
        let (mut metering, start) = (Metering::default(), Instant::now());
        let _ = metering.update(&levels(&[0.1], 0.0), start);
        let meters = metering.update(&levels(&[0.5], 0.0), later(start, 100));
        assert_eq!(meters.tracks[0].level, 0.5);
    }

    #[test]
    fn a_peak_at_full_scale_shows_red_for_2_seconds() {
        let (mut metering, start) = (Metering::default(), Instant::now());
        let clipped = |metering: &mut Metering, milliseconds| {
            let meters = metering.update(&levels(&[0.0], 0.0), later(start, milliseconds));
            meters.master.clipped
        };
        assert!(metering.update(&levels(&[0.0], 1.0), start).master.clipped);
        assert!(clipped(&mut metering, 1_900));
        assert!(!clipped(&mut metering, 2_100));
    }

    #[test]
    fn the_meters_stop_moving_when_they_are_silent_and_show_no_clip() {
        let (mut metering, start) = (Metering::default(), Instant::now());
        let _ = metering.update(&levels(&[0.5], 0.5), start);
        assert!(metering.is_moving());
        let _ = metering.update(&levels(&[0.0], 0.0), later(start, 10_000));
        assert!(!metering.is_moving());
    }

    #[test]
    fn the_meters_follow_the_number_of_tracks() {
        let mut metering = Metering::default();
        let meters = metering.update(&levels(&[0.1, 0.2, 0.3], 0.0), Instant::now());
        assert_eq!(meters.tracks.len(), 3);
    }
}
