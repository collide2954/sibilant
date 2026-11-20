use std::error::Error;
use std::fmt;
use std::ops::RangeInclusive;

use super::{Duration, Musical};

const WHOLE_NOTE: Duration<Musical> = Duration::from_quarter_notes(4);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSignature {
    numerator: u8,
    denominator: u8,
}

impl TimeSignature {
    pub const NUMERATORS: RangeInclusive<u8> = 1..=99;
    pub const DENOMINATORS: [u8; 7] = [1, 2, 4, 8, 16, 32, 64];

    pub fn new(numerator: u8, denominator: u8) -> Result<Self, InvalidTimeSignature> {
        if Self::NUMERATORS.contains(&numerator) && Self::DENOMINATORS.contains(&denominator) {
            Ok(Self {
                numerator,
                denominator,
            })
        } else {
            Err(InvalidTimeSignature {
                numerator,
                denominator,
            })
        }
    }

    pub const fn numerator(self) -> u8 {
        self.numerator
    }

    pub const fn denominator(self) -> u8 {
        self.denominator
    }

    pub fn beat_length(self) -> Duration<Musical> {
        Duration::from_ticks(WHOLE_NOTE.ticks() / i64::from(self.denominator))
    }

    pub fn bar_length(self) -> Duration<Musical> {
        self.beat_length() * i64::from(self.numerator)
    }
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTimeSignature {
    numerator: u8,
    denominator: u8,
}

impl fmt::Display for InvalidTimeSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let numerators = TimeSignature::NUMERATORS;
        let [smallest, .., largest] = TimeSignature::DENOMINATORS;
        write!(
            f,
            "time signature {}/{} is not valid: the upper number must be {} to {}, \
             and the lower number a power of two from {smallest} to {largest}",
            self.numerator,
            self.denominator,
            numerators.start(),
            numerators.end()
        )
    }
}

impl Error for InvalidTimeSignature {}

#[cfg(test)]
mod tests {
    use crate::time::{Duration, Musical, TICKS_PER_QUARTER_NOTE, TimeSignature};

    fn time_signature(numerator: u8, denominator: u8) -> TimeSignature {
        TimeSignature::new(numerator, denominator).unwrap()
    }

    fn ticks(ticks: i64) -> Duration<Musical> {
        Duration::from_ticks(ticks)
    }

    #[test]
    fn a_beat_is_the_note_value_of_the_lower_number() {
        let quarter_note = TICKS_PER_QUARTER_NOTE;
        assert_eq!(time_signature(2, 2).beat_length(), ticks(2 * quarter_note));
        assert_eq!(time_signature(3, 4).beat_length(), ticks(quarter_note));
        assert_eq!(time_signature(6, 8).beat_length(), ticks(quarter_note / 2));
        assert_eq!(
            time_signature(3, 64).beat_length(),
            ticks(quarter_note / 16)
        );
    }

    #[test]
    fn each_note_value_is_an_exact_number_of_ticks() {
        let whole_note = Duration::from_quarter_notes(4);
        for denominator in TimeSignature::DENOMINATORS {
            let beat = time_signature(1, denominator).beat_length();
            assert_eq!(beat * i64::from(denominator), whole_note, "1/{denominator}");
        }
    }

    #[test]
    fn a_bar_has_the_number_of_beats_of_the_upper_number() {
        let quarter_note = TICKS_PER_QUARTER_NOTE;
        assert_eq!(time_signature(4, 4).bar_length(), ticks(4 * quarter_note));
        assert_eq!(time_signature(6, 8).bar_length(), ticks(3 * quarter_note));
        assert_eq!(
            time_signature(7, 8).bar_length(),
            ticks(7 * quarter_note / 2)
        );
    }

    #[test]
    fn accepts_1_to_99_beats_of_each_note_value() {
        for numerator in [1, 99] {
            for denominator in [1, 2, 4, 8, 16, 32, 64] {
                let signature = time_signature(numerator, denominator);
                assert_eq!(signature.numerator(), numerator);
                assert_eq!(signature.denominator(), denominator);
            }
        }
    }

    #[test]
    fn rejects_an_upper_number_outside_1_to_99() {
        for numerator in [0, 100] {
            assert!(TimeSignature::new(numerator, 4).is_err(), "{numerator}/4");
        }
    }

    #[test]
    fn rejects_a_lower_number_that_is_not_a_note_value() {
        for denominator in [0, 3, 6, 128] {
            assert!(
                TimeSignature::new(4, denominator).is_err(),
                "4/{denominator}"
            );
        }
    }

    #[test]
    fn the_default_time_signature_is_4_4() {
        assert_eq!(TimeSignature::default(), time_signature(4, 4));
    }

    #[test]
    fn the_error_tells_the_valid_numbers() {
        let error = TimeSignature::new(7, 3).unwrap_err();
        assert_eq!(
            error.to_string(),
            "time signature 7/3 is not valid: the upper number must be 1 to 99, \
             and the lower number a power of two from 1 to 64"
        );
    }
}
