use super::{
    Clock, Duration, Musical, Position, SUPERCLOCKS_PER_SECOND, TICKS_PER_QUARTER_NOTE, Tempo,
};

#[derive(Debug, Clone)]
pub struct TempoMap {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, Copy)]
struct Segment {
    start: Position<Musical>,
    clock_start: Position<Clock>,
    tempo: Tempo,
}

impl TempoMap {
    pub fn new(tempo: Tempo) -> Self {
        Self {
            segments: vec![Segment {
                start: Position::START,
                clock_start: Position::START,
                tempo,
            }],
        }
    }

    #[must_use]
    pub fn with_tempo(mut self, at: Position<Musical>, tempo: Tempo) -> Self {
        let start = at.max(Position::START);
        match self
            .segments
            .binary_search_by_key(&start, |segment| segment.start)
        {
            Ok(index) => self.segments[index].tempo = tempo,
            Err(index) => self.segments.insert(
                index,
                Segment {
                    start,
                    clock_start: Position::START,
                    tempo,
                },
            ),
        }
        self.recompute_clock_starts();
        self
    }

    pub fn tempo_at(&self, position: Position<Musical>) -> Tempo {
        self.segment_at(position).tempo
    }

    pub fn to_clock(&self, position: Position<Musical>) -> Position<Clock> {
        let segment = self.segment_at(position);
        segment.clock_start + to_clock_duration(position - segment.start, segment.tempo)
    }

    pub fn to_musical(&self, position: Position<Clock>) -> Position<Musical> {
        let segment = self.last_segment_where(|segment| segment.clock_start <= position);
        segment.start + to_musical_duration(position - segment.clock_start, segment.tempo)
    }

    fn segment_at(&self, position: Position<Musical>) -> Segment {
        self.last_segment_where(|segment| segment.start <= position)
    }

    fn last_segment_where(&self, predicate: impl FnMut(&Segment) -> bool) -> Segment {
        let index = self.segments.partition_point(predicate).saturating_sub(1);
        self.segments[index]
    }

    fn recompute_clock_starts(&mut self) {
        let mut previous = self.segments[0];
        for segment in &mut self.segments[1..] {
            let length = segment.start - previous.start;
            segment.clock_start = previous.clock_start + to_clock_duration(length, previous.tempo);
            previous = *segment;
        }
    }
}

fn to_clock_duration(duration: Duration<Musical>, tempo: Tempo) -> Duration<Clock> {
    let superclocks = to_f64(duration.ticks()) * superclocks_per_tick(tempo);
    Duration::from_superclocks(round(superclocks))
}

fn to_musical_duration(duration: Duration<Clock>, tempo: Tempo) -> Duration<Musical> {
    let ticks = to_f64(duration.superclocks()) / superclocks_per_tick(tempo);
    Duration::from_ticks(round(ticks))
}

fn superclocks_per_tick(tempo: Tempo) -> f64 {
    60.0 * to_f64(SUPERCLOCKS_PER_SECOND) / (tempo.bpm() * to_f64(TICKS_PER_QUARTER_NOTE))
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

    const CHANGE_RANGE: i64 = 1 << 32;
    const MUSICAL_RANGE: i64 = 1 << 34;
    const CLOCK_RANGE: i64 = 1 << 44;

    fn tempo(bpm: f64) -> Tempo {
        Tempo::from_bpm(bpm).unwrap()
    }

    #[test]
    fn at_60_bpm_a_quarter_note_lasts_one_second() {
        let map = TempoMap::new(tempo(60.0));
        assert_eq!(map.to_clock(quarter_notes(1)), Position::from_seconds(1));
    }

    #[test]
    fn at_120_bpm_a_quarter_note_lasts_half_a_second() {
        let map = TempoMap::new(tempo(120.0));
        assert_eq!(
            map.to_clock(quarter_notes(1)),
            Position::from_superclocks(SUPERCLOCKS_PER_SECOND / 2)
        );
    }

    #[test]
    fn converts_clock_time_back_to_musical_time() {
        let map = TempoMap::new(tempo(120.0));
        assert_eq!(map.to_musical(Position::from_seconds(2)), quarter_notes(4));
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

    #[test]
    fn a_change_sets_the_tempo_from_its_position() {
        let map = TempoMap::new(tempo(120.0)).with_tempo(quarter_notes(4), tempo(60.0));
        assert_eq!(map.tempo_at(quarter_notes(3)), tempo(120.0));
        assert_eq!(map.tempo_at(quarter_notes(4)), tempo(60.0));
    }

    #[test]
    fn musical_time_after_a_change_advances_at_the_new_tempo() {
        let map = TempoMap::new(tempo(120.0)).with_tempo(quarter_notes(4), tempo(60.0));
        assert_eq!(map.to_clock(quarter_notes(4)), Position::from_seconds(2));
        assert_eq!(map.to_clock(quarter_notes(6)), Position::from_seconds(4));
    }

    #[test]
    fn clock_time_after_a_change_converts_back_at_the_new_tempo() {
        let map = TempoMap::new(tempo(120.0)).with_tempo(quarter_notes(4), tempo(60.0));
        assert_eq!(map.to_musical(Position::from_seconds(4)), quarter_notes(6));
    }

    #[test]
    fn changes_apply_in_position_order() {
        let map = TempoMap::new(tempo(120.0))
            .with_tempo(quarter_notes(8), tempo(240.0))
            .with_tempo(quarter_notes(4), tempo(60.0));
        assert_eq!(map.to_clock(quarter_notes(12)), Position::from_seconds(7));
    }

    #[test]
    fn a_change_at_the_same_position_replaces_the_old_tempo() {
        let map = TempoMap::new(tempo(120.0))
            .with_tempo(quarter_notes(4), tempo(60.0))
            .with_tempo(quarter_notes(4), tempo(240.0));
        assert_eq!(map.tempo_at(quarter_notes(4)), tempo(240.0));
    }

    #[test]
    fn a_change_at_or_before_the_start_replaces_the_first_tempo() {
        let map = TempoMap::new(tempo(120.0)).with_tempo(quarter_notes(-2), tempo(60.0));
        assert_eq!(map.tempo_at(quarter_notes(-4)), tempo(60.0));
        assert_eq!(map.to_clock(quarter_notes(1)), Position::from_seconds(1));
    }

    fn quarter_notes(quarter_notes: i64) -> Position<Musical> {
        Position::from_quarter_notes(quarter_notes)
    }

    fn tempo_changes() -> impl Strategy<Value = (f64, Vec<(i64, f64)>)> {
        (
            1.0..=1000.0,
            prop::collection::vec((1..CHANGE_RANGE, 1.0..=1000.0), 0..8),
        )
    }

    fn tempo_map(first: f64, changes: &[(i64, f64)]) -> TempoMap {
        changes
            .iter()
            .fold(TempoMap::new(tempo(first)), |map, &(ticks, bpm)| {
                map.with_tempo(Position::from_ticks(ticks), tempo(bpm))
            })
    }

    proptest! {
        #[test]
        fn musical_time_survives_a_round_trip(
            (first, changes) in tempo_changes(),
            ticks in -MUSICAL_RANGE..MUSICAL_RANGE,
        ) {
            let map = tempo_map(first, &changes);
            let position = Position::<Musical>::from_ticks(ticks);
            prop_assert_eq!(map.to_musical(map.to_clock(position)), position);
        }

        #[test]
        fn a_clock_round_trip_moves_at_most_half_a_tick_of_the_slowest_tempo(
            (first, changes) in tempo_changes(),
            superclocks in -CLOCK_RANGE..CLOCK_RANGE,
        ) {
            let map = tempo_map(first, &changes);
            let slowest = changes.iter().map(|&(_, bpm)| bpm).fold(first, f64::min);
            let slowest_tick = TempoMap::new(tempo(slowest)).to_clock(Position::from_ticks(1));
            let position = Position::<Clock>::from_superclocks(superclocks);
            let moved = (map.to_clock(map.to_musical(position)) - position).superclocks();
            prop_assert!(moved.abs() <= slowest_tick.superclocks() / 2 + 1, "moved {} superclocks", moved);
        }

        #[test]
        fn later_musical_time_never_maps_to_earlier_clock_time(
            (first, changes) in tempo_changes(),
            a in -MUSICAL_RANGE..MUSICAL_RANGE,
            b in -MUSICAL_RANGE..MUSICAL_RANGE,
        ) {
            let map = tempo_map(first, &changes);
            let (early, late) = (a.min(b), a.max(b));
            prop_assert!(
                map.to_clock(Position::from_ticks(early)) <= map.to_clock(Position::from_ticks(late))
            );
        }
    }
}
