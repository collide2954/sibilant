use super::{Musical, Position, TimeSignature};

const FIRST_BAR: i64 = 1;

#[derive(Debug, Clone)]
pub struct TimeSignatureMap {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, Copy)]
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
        self.last_segment_where(|segment| segment.start <= position)
            .time_signature
    }

    pub fn bar_start(&self, bar: i64) -> Position<Musical> {
        let segment = self.last_segment_where(|segment| segment.bar <= bar);
        segment.start + segment.time_signature.bar_length() * (bar - segment.bar)
    }

    fn last_segment_where(&self, predicate: impl FnMut(&Segment) -> bool) -> Segment {
        let index = self.segments.partition_point(predicate).saturating_sub(1);
        self.segments[index]
    }

    fn recompute_starts(&mut self) {
        let mut previous = self.segments[0];
        for segment in &mut self.segments[1..] {
            let bars = segment.bar - previous.bar;
            segment.start = previous.start + previous.time_signature.bar_length() * bars;
            previous = *segment;
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::time::{Duration, Musical, Position, TimeSignature, TimeSignatureMap};

    fn time_signature(numerator: u8, denominator: u8) -> TimeSignature {
        TimeSignature::new(numerator, denominator).unwrap()
    }

    fn quarter_notes(quarter_notes: i64) -> Position<Musical> {
        Position::from_quarter_notes(quarter_notes)
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

    proptest! {
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
