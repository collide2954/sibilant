use super::{Clock, Musical, Position, SUPERCLOCKS_PER_SECOND, TICKS_PER_BEAT, Tempo};

#[derive(Debug, Clone)]
pub struct TempoMap {
    tempo: Tempo,
}

impl TempoMap {
    pub const fn new(tempo: Tempo) -> Self {
        Self { tempo }
    }

    pub fn to_clock(&self, position: Position<Musical>) -> Position<Clock> {
        let superclocks = to_f64(position.ticks()) * superclocks_per_tick(self.tempo);
        Position::from_superclocks(round(superclocks))
    }

    pub fn to_musical(&self, position: Position<Clock>) -> Position<Musical> {
        let ticks = to_f64(position.superclocks()) / superclocks_per_tick(self.tempo);
        Position::from_ticks(round(ticks))
    }
}

fn superclocks_per_tick(tempo: Tempo) -> f64 {
    60.0 * to_f64(SUPERCLOCKS_PER_SECOND) / (tempo.bpm() * to_f64(TICKS_PER_BEAT))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "time values stay far below 2^53"
)]
fn to_f64(value: i64) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "rounded time values stay inside the range of i64"
)]
fn round(value: f64) -> i64 {
    value.round() as i64
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::time::{Clock, Musical, Position, SUPERCLOCKS_PER_SECOND, Tempo, TempoMap};

    const RANGE: i64 = 1 << 40;

    fn tempo(bpm: f64) -> Tempo {
        Tempo::from_bpm(bpm).unwrap()
    }

    #[test]
    fn at_60_bpm_a_beat_lasts_one_second() {
        let map = TempoMap::new(tempo(60.0));
        assert_eq!(
            map.to_clock(Position::from_beats(1)),
            Position::from_seconds(1)
        );
    }

    #[test]
    fn at_120_bpm_a_beat_lasts_half_a_second() {
        let map = TempoMap::new(tempo(120.0));
        assert_eq!(
            map.to_clock(Position::from_beats(1)),
            Position::from_superclocks(SUPERCLOCKS_PER_SECOND / 2)
        );
    }

    #[test]
    fn converts_clock_time_back_to_musical_time() {
        let map = TempoMap::new(tempo(120.0));
        assert_eq!(
            map.to_musical(Position::from_seconds(2)),
            Position::from_beats(4)
        );
    }

    #[test]
    fn rounds_halves_away_from_zero() {
        let map = TempoMap::new(tempo(128.0));
        assert_eq!(
            map.to_clock(Position::from_ticks(1)),
            Position::from_superclocks(188)
        );
        assert_eq!(
            map.to_clock(Position::from_ticks(-1)),
            Position::from_superclocks(-188)
        );
    }

    proptest! {
        #[test]
        fn musical_time_survives_a_round_trip(bpm in 1.0..=1000.0, ticks in -RANGE..RANGE) {
            let map = TempoMap::new(tempo(bpm));
            let position = Position::<Musical>::from_ticks(ticks);
            prop_assert_eq!(map.to_musical(map.to_clock(position)), position);
        }

        #[test]
        fn a_clock_round_trip_moves_at_most_half_a_tick(
            bpm in 1.0..=1000.0,
            superclocks in -RANGE..RANGE,
        ) {
            let map = TempoMap::new(tempo(bpm));
            let position = Position::<Clock>::from_superclocks(superclocks);
            let moved = (map.to_clock(map.to_musical(position)) - position).superclocks();
            let half_tick = map.to_clock(Position::from_ticks(1)).superclocks() / 2 + 1;
            prop_assert!(moved.abs() <= half_tick, "moved {} superclocks", moved);
        }

        #[test]
        fn later_musical_time_never_maps_to_earlier_clock_time(
            bpm in 1.0..=1000.0,
            a in -RANGE..RANGE,
            b in -RANGE..RANGE,
        ) {
            let map = TempoMap::new(tempo(bpm));
            let (early, late) = (a.min(b), a.max(b));
            prop_assert!(
                map.to_clock(Position::from_ticks(early)) <= map.to_clock(Position::from_ticks(late))
            );
        }
    }
}
