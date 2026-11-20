use std::error::Error;
use std::fmt;
use std::ops::RangeInclusive;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Gain(f64);

impl Gain {
    pub const RANGE: RangeInclusive<f64> = f64::NEG_INFINITY..=6.0;
    pub const SILENT: Self = Self(f64::NEG_INFINITY);
    pub const UNITY: Self = Self(0.0);

    pub fn from_db(db: f64) -> Result<Self, GainOutOfRange> {
        if Self::RANGE.contains(&db) {
            Ok(Self(db))
        } else {
            Err(GainOutOfRange(db))
        }
    }

    pub const fn db(self) -> f64 {
        self.0
    }

    pub fn amplitude(self) -> f64 {
        10_f64.powf(self.0 / 20.0)
    }
}

impl Default for Gain {
    fn default() -> Self {
        Self::UNITY
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GainOutOfRange(f64);

impl fmt::Display for GainOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "gain {} dB is not in the range from silence to {} dB",
            self.0,
            Gain::RANGE.end()
        )
    }
}

impl Error for GainOutOfRange {}

#[cfg(test)]
mod tests {
    use crate::mix::Gain;

    fn is_close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn accepts_gains_from_silence_to_6_db() {
        for db in [f64::NEG_INFINITY, -96.0, 0.0, 6.0] {
            assert_eq!(Gain::from_db(db).map(Gain::db), Ok(db));
        }
    }

    #[test]
    fn rejects_gains_above_6_db_and_values_that_are_not_numbers() {
        for db in [6.01, f64::INFINITY, f64::NAN] {
            assert!(Gain::from_db(db).is_err(), "{db} dB");
        }
    }

    #[test]
    fn the_default_gain_does_not_change_the_audio() {
        assert_eq!(Gain::default(), Gain::UNITY);
        assert!(is_close(Gain::UNITY.amplitude(), 1.0));
    }

    #[test]
    fn silence_has_no_amplitude() {
        assert!(is_close(Gain::SILENT.amplitude(), 0.0));
    }

    #[test]
    fn each_20_db_multiplies_the_amplitude_by_10() {
        let gain = |db| Gain::from_db(db).unwrap().amplitude();
        assert!(is_close(gain(-20.0), 0.1));
        assert!(is_close(gain(-40.0), 0.01));
        assert!(is_close(gain(6.0), 1.995_262_314_968_88));
    }

    #[test]
    fn the_error_tells_the_valid_range() {
        let error = Gain::from_db(7.0).unwrap_err();
        assert_eq!(
            error.to_string(),
            "gain 7 dB is not in the range from silence to 6 dB"
        );
    }
}
