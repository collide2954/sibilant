use super::{Duration, Musical, Position, TimeSignature};

const FIRST_BAR: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSignatureMap {
    segments: Vec<Segment>,
}

impl Default for TimeSignatureMap {
    fn default() -> Self {
        Self::new(TimeSignature::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarBeat {
    pub bar: i64,
    pub beat: i64,
    pub offset: Duration<Musical>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Segment {
    bar: i64,
    start: Position<Musical>,
    time_signature: TimeSignature,
}

impl TimeSignatureMap {
    pub fn new(time_signature: TimeSignature) -> Self {
        Self {
            segments: vec![Segment {
                bar: FIRST_BAR,
                start: Position::START,
                time_signature,
            }],
        }
    }

    #[must_use]
    pub fn with_time_signature(mut self, bar: i64, time_signature: TimeSignature) -> Self {
        let bar = bar.max(FIRST_BAR);
        match self
            .segments
            .binary_search_by_key(&bar, |segment| segment.bar)
        {
            Ok(index) => self.segments[index].time_signature = time_signature,
            Err(index) => self.segments.insert(
                index,
                Segment {
                    bar,
                    start: Position::START,
                    time_signature,
                },
            ),
        }
        self.recompute_starts();
        self
    }

    pub fn time_signature_at(&self, position: Position<Musical>) -> TimeSignature {
        self.segment_at(position).time_signature
    }

    pub fn changes(&self) -> impl ExactSizeIterator<Item = (i64, TimeSignature)> {
        self.segments
            .iter()
            .map(|segment| (segment.bar, segment.time_signature))
    }

    pub fn bar_start(&self, bar: i64) -> Position<Musical> {
        self.segment_of_bar(bar).bar_start(bar)
    }

    pub fn to_bar_beat(&self, position: Position<Musical>) -> BarBeat {
        let segment = self.segment_at(position);
        let bar_length = segment.time_signature.bar_length().ticks();
        let beat_length = segment.time_signature.beat_length().ticks();
        let ticks = (position - segment.start).ticks();
        let ticks_in_bar = ticks.rem_euclid(bar_length);
        BarBeat {
            bar: segment.bar + ticks.div_euclid(bar_length),
            beat: ticks_in_bar / beat_length + 1,
            offset: Duration::from_ticks(ticks_in_bar % beat_length),
        }
    }

    pub fn to_position(&self, bar_beat: BarBeat) -> Position<Musical> {
        let segment = self.segment_of_bar(bar_beat.bar);
        let beat_length = segment.time_signature.beat_length();
        segment.bar_start(bar_beat.bar) + beat_length * (bar_beat.beat - 1) + bar_beat.offset
    }

    fn segment_at(&self, position: Position<Musical>) -> Segment {
        self.last_segment_where(|segment| segment.start <= position)
    }

    fn segment_of_bar(&self, bar: i64) -> Segment {
        self.last_segment_where(|segment| segment.bar <= bar)
    }

    fn last_segment_where(&self, predicate: impl FnMut(&Segment) -> bool) -> Segment {
        let index = self.segments.partition_point(predicate).saturating_sub(1);
        self.segments[index]
    }

    fn recompute_starts(&mut self) {
        let mut previous = self.segments[0];
        for segment in &mut self.segments[1..] {
            segment.start = previous.bar_start(segment.bar);
            previous = *segment;
        }
    }
}

impl Segment {
    fn bar_start(self, bar: i64) -> Position<Musical> {
        self.start + self.time_signature.bar_length() * (bar - self.bar)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::time::{
        BarBeat, Duration, Musical, Position, TICKS_PER_QUARTER_NOTE, TimeSignature,
        TimeSignatureMap,
    };

    const LONGEST_BAR: i64 = 99 * 4 * TICKS_PER_QUARTER_NOTE;

    fn time_signature(numerator: u8, denominator: u8) -> TimeSignature {
        TimeSignature::new(numerator, denominator).unwrap()
    }

    fn quarter_notes(quarter_notes: i64) -> Position<Musical> {
        Position::from_quarter_notes(quarter_notes)
    }

    #[test]
    fn the_default_map_has_the_default_time_signature_everywhere() {
        let map = TimeSignatureMap::default();
        assert_eq!(map, TimeSignatureMap::new(TimeSignature::default()));
    }

    #[test]
    fn bar_1_starts_at_the_project_start() {
        let map = TimeSignatureMap::new(time_signature(4, 4));
        assert_eq!(map.bar_start(1), Position::START);
    }

    #[test]
    fn each_bar_starts_one_bar_length_after_the_bar_before_it() {
        assert_eq!(
            TimeSignatureMap::new(time_signature(4, 4)).bar_start(3),
            quarter_notes(8)
        );
        assert_eq!(
            TimeSignatureMap::new(time_signature(6, 8)).bar_start(3),
            quarter_notes(6)
        );
    }

    #[test]
    fn the_first_time_signature_continues_before_bar_1() {
        let map = TimeSignatureMap::new(time_signature(4, 4));
        assert_eq!(map.bar_start(0), quarter_notes(-4));
        assert_eq!(map.bar_start(-1), quarter_notes(-8));
    }

    #[test]
    fn a_change_starts_at_the_bar_line_of_its_bar() {
        let map = TimeSignatureMap::new(time_signature(4, 4))
            .with_time_signature(3, time_signature(3, 4));
        assert_eq!(map.bar_start(3), quarter_notes(8));
        assert_eq!(map.bar_start(4), quarter_notes(11));
        assert_eq!(map.bar_start(5), quarter_notes(14));
    }

    #[test]
    fn a_change_sets_the_time_signature_from_its_bar() {
        let map = TimeSignatureMap::new(time_signature(4, 4))
            .with_time_signature(3, time_signature(3, 4));
        let change = map.bar_start(3);
        assert_eq!(
            map.time_signature_at(change - Duration::from_ticks(1)),
            time_signature(4, 4)
        );
        assert_eq!(map.time_signature_at(change), time_signature(3, 4));
    }

    #[test]
    fn changes_apply_in_bar_order() {
        let map = TimeSignatureMap::new(time_signature(4, 4))
            .with_time_signature(5, time_signature(7, 8))
            .with_time_signature(3, time_signature(3, 4));
        assert_eq!(map.bar_start(5), quarter_notes(14));
        assert_eq!(
            map.time_signature_at(quarter_notes(14)),
            time_signature(7, 8)
        );
    }

    #[test]
    fn a_change_at_the_same_bar_replaces_the_old_time_signature() {
        let map = TimeSignatureMap::new(time_signature(4, 4))
            .with_time_signature(3, time_signature(3, 4))
            .with_time_signature(3, time_signature(5, 4));
        assert_eq!(
            map.time_signature_at(map.bar_start(3)),
            time_signature(5, 4)
        );
    }

    #[test]
    fn a_change_at_or_before_bar_1_replaces_the_first_time_signature() {
        let map = TimeSignatureMap::new(time_signature(4, 4))
            .with_time_signature(-2, time_signature(3, 4));
        assert_eq!(
            map.time_signature_at(quarter_notes(-10)),
            time_signature(3, 4)
        );
        assert_eq!(map.bar_start(2), quarter_notes(3));
    }

    #[test]
    fn a_change_to_an_earlier_bar_moves_the_later_changes_with_their_bars() {
        let map = TimeSignatureMap::new(time_signature(4, 4))
            .with_time_signature(5, time_signature(3, 4))
            .with_time_signature(2, time_signature(2, 4));
        assert_eq!(map.bar_start(5), quarter_notes(10));
        assert_eq!(
            map.time_signature_at(quarter_notes(10)),
            time_signature(3, 4)
        );
    }

    #[test]
    fn lists_each_time_signature_with_the_bar_where_it_starts() {
        let map = TimeSignatureMap::new(time_signature(4, 4))
            .with_time_signature(5, time_signature(7, 8))
            .with_time_signature(3, time_signature(3, 4));
        let changes: Vec<_> = map.changes().collect();
        assert_eq!(
            changes,
            [
                (1, time_signature(4, 4)),
                (3, time_signature(3, 4)),
                (5, time_signature(7, 8)),
            ]
        );
    }

    fn bar_beat(bar: i64, beat: i64, ticks: i64) -> BarBeat {
        BarBeat {
            bar,
            beat,
            offset: Duration::from_ticks(ticks),
        }
    }

    fn eighth_notes(eighth_notes: i64) -> Duration<Musical> {
        Duration::from_ticks(eighth_notes * TICKS_PER_QUARTER_NOTE / 2)
    }

    #[test]
    fn the_project_start_is_bar_1_beat_1() {
        let map = TimeSignatureMap::new(time_signature(4, 4));
        assert_eq!(map.to_bar_beat(Position::START), bar_beat(1, 1, 0));
    }

    #[test]
    fn counts_beats_of_the_lower_number() {
        let map = TimeSignatureMap::new(time_signature(6, 8));
        let start = Position::START;
        assert_eq!(map.to_bar_beat(start + eighth_notes(5)), bar_beat(1, 6, 0));
        assert_eq!(map.to_bar_beat(start + eighth_notes(6)), bar_beat(2, 1, 0));
    }

    #[test]
    fn the_offset_counts_ticks_into_the_beat() {
        let map = TimeSignatureMap::new(time_signature(4, 4));
        let position = quarter_notes(1) + Duration::from_ticks(100);
        assert_eq!(map.to_bar_beat(position), bar_beat(1, 2, 100));
    }

    #[test]
    fn positions_before_the_start_are_in_bar_0_and_lower() {
        let map = TimeSignatureMap::new(time_signature(4, 4));
        let last_tick = TICKS_PER_QUARTER_NOTE - 1;
        assert_eq!(
            map.to_bar_beat(Position::from_ticks(-1)),
            bar_beat(0, 4, last_tick)
        );
        assert_eq!(map.to_bar_beat(quarter_notes(-4)), bar_beat(0, 1, 0));
        assert_eq!(map.to_bar_beat(quarter_notes(-5)), bar_beat(-1, 4, 0));
    }

    #[test]
    fn counts_bars_and_beats_after_a_change_in_the_new_time_signature() {
        let map = TimeSignatureMap::new(time_signature(4, 4))
            .with_time_signature(3, time_signature(6, 8));
        assert_eq!(
            map.to_bar_beat(quarter_notes(8) + eighth_notes(1)),
            bar_beat(3, 2, 0)
        );
        assert_eq!(map.to_bar_beat(quarter_notes(11)), bar_beat(4, 1, 0));
    }

    #[test]
    fn converts_bars_and_beats_back_to_a_position() {
        let map = TimeSignatureMap::new(time_signature(4, 4))
            .with_time_signature(3, time_signature(6, 8));
        assert_eq!(
            map.to_position(bar_beat(3, 2, 10)),
            quarter_notes(8) + eighth_notes(1) + Duration::from_ticks(10)
        );
    }

    fn time_signatures() -> impl Strategy<Value = TimeSignature> {
        (
            TimeSignature::NUMERATORS,
            prop::sample::select(TimeSignature::DENOMINATORS.to_vec()),
        )
            .prop_map(|(numerator, denominator)| time_signature(numerator, denominator))
    }

    fn time_signature_changes() -> impl Strategy<Value = (TimeSignature, Vec<(i64, TimeSignature)>)>
    {
        (
            time_signatures(),
            prop::collection::vec((-10..1_000_i64, time_signatures()), 0..8),
        )
    }

    fn time_signature_map(
        first: TimeSignature,
        changes: &[(i64, TimeSignature)],
    ) -> TimeSignatureMap {
        changes
            .iter()
            .fold(TimeSignatureMap::new(first), |map, &(bar, signature)| {
                map.with_time_signature(bar, signature)
            })
    }

    fn maps_and_positions() -> impl Strategy<Value = (TimeSignatureMap, Position<Musical>)> {
        (time_signature_changes(), -20_i64..1_020, 0..LONGEST_BAR).prop_map(
            |((first, changes), bar, offset)| {
                let map = time_signature_map(first, &changes);
                let position = map.bar_start(bar) + Duration::from_ticks(offset);
                (map, position)
            },
        )
    }

    proptest! {
        #[test]
        fn positions_survive_a_round_trip_through_bars_and_beats(
            (map, position) in maps_and_positions(),
        ) {
            prop_assert_eq!(map.to_position(map.to_bar_beat(position)), position);
        }

        #[test]
        fn a_position_is_inside_its_bar_and_its_beat(
            (map, position) in maps_and_positions(),
        ) {
            let bar_beat = map.to_bar_beat(position);
            let time_signature = map.time_signature_at(position);
            prop_assert!(map.bar_start(bar_beat.bar) <= position);
            prop_assert!(position < map.bar_start(bar_beat.bar + 1));
            prop_assert!((1..=i64::from(time_signature.numerator())).contains(&bar_beat.beat));
            prop_assert!((0..time_signature.beat_length().ticks()).contains(&bar_beat.offset.ticks()));
        }

        #[test]
        fn a_map_made_again_from_its_changes_is_the_same(
            (first, changes) in time_signature_changes(),
        ) {
            let map = time_signature_map(first, &changes);
            let again = map.changes().fold(TimeSignatureMap::new(first), |again, (bar, signature)| {
                again.with_time_signature(bar, signature)
            });
            prop_assert_eq!(again, map);
        }

        #[test]
        fn each_bar_lasts_the_bar_length_of_its_time_signature(
            (first, changes) in time_signature_changes(),
            bar in -10_000_i64..10_000,
        ) {
            let map = time_signature_map(first, &changes);
            let start = map.bar_start(bar);
            let length = map.time_signature_at(start).bar_length();
            prop_assert_eq!(map.bar_start(bar + 1) - start, length);
        }
    }
}
