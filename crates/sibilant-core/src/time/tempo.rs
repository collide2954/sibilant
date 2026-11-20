use std::error::Error;
use std::fmt;
use std::ops::RangeInclusive;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tempo(f64);

impl Tempo {
    pub const RANGE: RangeInclusive<f64> = 1.0..=1000.0;

    pub fn from_bpm(bpm: f64) -> Result<Self, TempoOutOfRange> {
        if Self::RANGE.contains(&bpm) {
            Ok(Self(bpm))
        } else {
            Err(TempoOutOfRange(bpm))
        }
    }

    pub const fn bpm(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempoOutOfRange(f64);

impl fmt::Display for TempoOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tempo {} BPM is not in the range {} to {} BPM",
            self.0,
            Tempo::RANGE.start(),
            Tempo::RANGE.end()
        )
    }
}

impl Error for TempoOutOfRange {}

#[cfg(test)]
mod tests {
    use crate::time::Tempo;

    #[test]
    fn accepts_tempos_from_1_to_1000_bpm() {
        for bpm in [1.0, 120.0, 1000.0] {
            assert_eq!(Tempo::from_bpm(bpm).map(Tempo::bpm), Ok(bpm));
        }
    }

    #[test]
    fn rejects_tempos_outside_1_to_1000_bpm() {
        for bpm in [-120.0, 0.0, 0.99, 1000.01] {
            assert!(Tempo::from_bpm(bpm).is_err(), "{bpm} BPM");
        }
    }

    #[test]
    fn rejects_values_that_are_not_finite() {
        for bpm in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(Tempo::from_bpm(bpm).is_err(), "{bpm} BPM");
        }
    }

    #[test]
    fn the_error_tells_the_valid_range() {
        let error = Tempo::from_bpm(0.5).unwrap_err();
        assert_eq!(
            error.to_string(),
            "tempo 0.5 BPM is not in the range 1 to 1000 BPM"
        );
    }
}
