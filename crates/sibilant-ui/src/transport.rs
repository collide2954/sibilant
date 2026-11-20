use sibilant_core::time::{
    Clock, Musical, Position, SUPERCLOCKS_PER_SECOND, TICKS_PER_QUARTER_NOTE, TimeSignatureMap,
};

const SIXTEENTH: i64 = TICKS_PER_QUARTER_NOTE / 4;
const SUPERCLOCKS_PER_MILLISECOND: i64 = SUPERCLOCKS_PER_SECOND / 1_000;

pub fn bars_beats_sixteenths(
    time_signatures: &TimeSignatureMap,
    position: Position<Musical>,
) -> String {
    let bar_beat = time_signatures.to_bar_beat(position);
    let sixteenth = bar_beat.offset.ticks() / SIXTEENTH + 1;
    format!("{}.{}.{sixteenth}", bar_beat.bar, bar_beat.beat)
}

pub fn clock_time(clock: Position<Clock>) -> String {
    let milliseconds = clock.superclocks().max(0) / SUPERCLOCKS_PER_MILLISECOND;
    let (minutes, rest) = (milliseconds / 60_000, milliseconds % 60_000);
    format!("{minutes}:{:02}.{:03}", rest / 1_000, rest % 1_000)
}

#[cfg(test)]
mod tests {
    use sibilant_core::time::{
        Clock, Duration, Musical, Position, SUPERCLOCKS_PER_SECOND, TICKS_PER_QUARTER_NOTE,
        TimeSignature, TimeSignatureMap,
    };

    use crate::transport::{bars_beats_sixteenths, clock_time};

    const SIXTEENTH: i64 = TICKS_PER_QUARTER_NOTE / 4;

    fn at(ticks: i64) -> Position<Musical> {
        Position::from_ticks(ticks)
    }

    #[test]
    fn the_project_start_reads_1_1_1() {
        let map = TimeSignatureMap::default();
        assert_eq!(bars_beats_sixteenths(&map, Position::START), "1.1.1");
    }

    #[test]
    fn the_position_counts_bars_beats_and_sixteenths_from_1() {
        let map = TimeSignatureMap::default();
        let quarter_note = TICKS_PER_QUARTER_NOTE;
        assert_eq!(bars_beats_sixteenths(&map, at(quarter_note)), "1.2.1");
        assert_eq!(
            bars_beats_sixteenths(&map, at(quarter_note + SIXTEENTH)),
            "1.2.2"
        );
        assert_eq!(bars_beats_sixteenths(&map, at(4 * quarter_note)), "2.1.1");
    }

    #[test]
    fn an_eighth_note_beat_has_two_sixteenths() {
        let map = TimeSignatureMap::new(TimeSignature::new(6, 8).unwrap());
        assert_eq!(bars_beats_sixteenths(&map, at(SIXTEENTH)), "1.1.2");
        assert_eq!(bars_beats_sixteenths(&map, at(2 * SIXTEENTH)), "1.2.1");
    }

    fn milliseconds(milliseconds: i64) -> Position<Clock> {
        Position::START + Duration::from_superclocks(milliseconds * SUPERCLOCKS_PER_SECOND / 1_000)
    }

    #[test]
    fn the_clock_time_shows_minutes_seconds_and_milliseconds() {
        assert_eq!(clock_time(Position::START), "0:00.000");
        assert_eq!(clock_time(milliseconds(61_500)), "1:01.500");
        assert_eq!(clock_time(milliseconds(600_007)), "10:00.007");
    }

    #[test]
    fn the_clock_time_drops_parts_of_a_millisecond() {
        let almost = milliseconds(1) - Duration::from_superclocks(1);
        assert_eq!(clock_time(almost), "0:00.000");
    }
}
