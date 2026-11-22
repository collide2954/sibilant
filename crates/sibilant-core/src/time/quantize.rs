use super::{BarBeat, Duration, Musical, Position, TimeSignatureMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Quantize {
    Off,
    Beat,
    #[default]
    Bar,
    TwoBars,
    FourBars,
}

impl Quantize {
    pub fn at_or_after(
        self,
        position: Position<Musical>,
        map: &TimeSignatureMap,
    ) -> Position<Musical> {
        let bars = match self {
            Self::Off => return position,
            Self::Beat => return next_beat(position, map),
            Self::Bar => 1,
            Self::TwoBars => 2,
            Self::FourBars => 4,
        };
        let BarBeat { bar, .. } = map.to_bar_beat(position);
        let first = if map.bar_start(bar) == position {
            bar
        } else {
            bar + 1
        };
        map.bar_start(first + (1 - first).rem_euclid(bars))
    }
}

fn next_beat(position: Position<Musical>, map: &TimeSignatureMap) -> Position<Musical> {
    let at = map.to_bar_beat(position);
    if at.offset == Duration::ZERO {
        return position;
    }
    map.to_position(BarBeat {
        beat: at.beat + 1,
        offset: Duration::ZERO,
        ..at
    })
}

#[cfg(test)]
mod tests {
    use crate::time::{
        Duration, Musical, Position, Quantize, TICKS_PER_QUARTER_NOTE, TimeSignature,
        TimeSignatureMap,
    };

    fn quarter_notes(quarter_notes: i64) -> Position<Musical> {
        Position::from_quarter_notes(quarter_notes)
    }

    fn tick() -> Duration<Musical> {
        Duration::from_ticks(1)
    }

    fn four_four() -> TimeSignatureMap {
        TimeSignatureMap::default()
    }

    #[test]
    fn off_starts_at_once() {
        let position = quarter_notes(5) + tick();
        assert_eq!(Quantize::Off.at_or_after(position, &four_four()), position);
    }

    #[test]
    fn a_beat_starts_at_the_next_beat() {
        let position = quarter_notes(5) + tick();
        assert_eq!(
            Quantize::Beat.at_or_after(position, &four_four()),
            quarter_notes(6)
        );
    }

    #[test]
    fn a_bar_starts_at_the_next_bar_line() {
        let in_bar_1 = quarter_notes(1);
        let in_bar_2 = quarter_notes(5) + tick();
        assert_eq!(
            Quantize::Bar.at_or_after(in_bar_1, &four_four()),
            quarter_notes(4)
        );
        assert_eq!(
            Quantize::Bar.at_or_after(in_bar_2, &four_four()),
            quarter_notes(8)
        );
    }

    #[test]
    fn two_and_four_bars_count_from_bar_1() {
        let in_bar_1 = quarter_notes(1);
        let in_bar_2 = quarter_notes(5);
        let in_bar_3 = quarter_notes(9);
        assert_eq!(
            Quantize::FourBars.at_or_after(in_bar_1, &four_four()),
            quarter_notes(16)
        );
        assert_eq!(
            Quantize::TwoBars.at_or_after(in_bar_2, &four_four()),
            quarter_notes(8)
        );
        assert_eq!(
            Quantize::TwoBars.at_or_after(in_bar_3, &four_four()),
            quarter_notes(16)
        );
        assert_eq!(
            Quantize::FourBars.at_or_after(in_bar_2, &four_four()),
            quarter_notes(16)
        );
    }

    #[test]
    fn a_time_on_the_grid_starts_at_that_time() {
        let bar_5 = quarter_notes(16);
        for quantize in [
            Quantize::Beat,
            Quantize::Bar,
            Quantize::TwoBars,
            Quantize::FourBars,
        ] {
            assert_eq!(
                quantize.at_or_after(bar_5, &four_four()),
                bar_5,
                "{quantize:?}"
            );
        }
        let beat_2 = quarter_notes(1);
        assert_eq!(Quantize::Beat.at_or_after(beat_2, &four_four()), beat_2);
    }

    #[test]
    fn beats_and_bars_follow_the_time_signature() {
        let six_eight = TimeSignature::new(6, 8).unwrap();
        let map = TimeSignatureMap::default().with_time_signature(2, six_eight);
        let eighth_note = Duration::from_ticks(TICKS_PER_QUARTER_NOTE / 2);
        let in_bar_2 = quarter_notes(4) + tick();
        assert_eq!(
            Quantize::Beat.at_or_after(in_bar_2, &map),
            quarter_notes(4) + eighth_note
        );
        assert_eq!(Quantize::Bar.at_or_after(in_bar_2, &map), quarter_notes(7));
        let last_beat_of_bar_1 = quarter_notes(3) + tick();
        assert_eq!(
            Quantize::Beat.at_or_after(last_beat_of_bar_1, &map),
            quarter_notes(4)
        );
    }
}
