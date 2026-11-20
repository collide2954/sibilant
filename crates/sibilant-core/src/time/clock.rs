use super::{Duration, Position};

pub const SUPERCLOCKS_PER_SECOND: i64 = 282_240_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Clock {}

impl Position<Clock> {
    pub const fn from_seconds(seconds: i64) -> Self {
        Self::from_superclocks(seconds * SUPERCLOCKS_PER_SECOND)
    }

    pub const fn from_superclocks(superclocks: i64) -> Self {
        Self::new(superclocks)
    }

    pub const fn superclocks(self) -> i64 {
        self.value()
    }
}

impl Duration<Clock> {
    pub const fn from_seconds(seconds: i64) -> Self {
        Self::from_superclocks(seconds * SUPERCLOCKS_PER_SECOND)
    }

    pub const fn from_superclocks(superclocks: i64) -> Self {
        Self::new(superclocks)
    }

    pub fn from_frames(frames: i64, sample_rate: u32) -> Self {
        let superclocks = (i128::from(frames) * i128::from(SUPERCLOCKS_PER_SECOND))
            .div_euclid(i128::from(sample_rate));
        Self::from_superclocks(i64::try_from(superclocks).unwrap_or(i64::MAX))
    }

    pub const fn superclocks(self) -> i64 {
        self.value()
    }

    pub fn to_frames(self, sample_rate: u32) -> i64 {
        let frames = (i128::from(self.superclocks()) * i128::from(sample_rate))
            .div_euclid(i128::from(SUPERCLOCKS_PER_SECOND));
        i64::try_from(frames).unwrap_or(i64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use crate::time::{Clock, Duration, Position, SUPERCLOCKS_PER_SECOND};

    #[test]
    fn a_second_has_282_240_000_superclocks() {
        assert_eq!(
            Duration::<Clock>::from_seconds(1).superclocks(),
            282_240_000
        );
    }

    #[test]
    fn common_sample_rates_divide_a_second_exactly() {
        for rate in [
            8_000, 11_025, 16_000, 22_050, 32_000, 44_100, 48_000, 88_200, 96_000, 176_400,
            192_000, 352_800, 384_000,
        ] {
            assert_eq!(SUPERCLOCKS_PER_SECOND % rate, 0, "sample rate {rate}");
        }
    }

    #[test]
    fn frames_last_their_part_of_a_second_at_the_sample_rate() {
        assert_eq!(
            Duration::<Clock>::from_frames(96_000, 48_000),
            Duration::from_seconds(2)
        );
        assert_eq!(
            Duration::<Clock>::from_frames(22_050, 44_100).superclocks(),
            141_120_000
        );
    }

    #[test]
    fn a_duration_holds_the_whole_frames_that_fit_in_it() {
        assert_eq!(Duration::<Clock>::from_seconds(2).to_frames(48_000), 96_000);
        assert_eq!(
            Duration::<Clock>::from_superclocks(7_467).to_frames(37_800),
            1
        );
        assert_eq!(
            Duration::<Clock>::from_superclocks(7_466).to_frames(37_800),
            0
        );
    }

    #[test]
    fn frames_at_an_uncommon_sample_rate_round_down_to_a_superclock() {
        assert_eq!(
            Duration::<Clock>::from_frames(1, 37_800).superclocks(),
            7_466
        );
    }

    #[test]
    fn positions_count_superclocks_from_the_project_start() {
        assert_eq!(
            Position::<Clock>::from_seconds(2).superclocks(),
            564_480_000
        );
    }
}
