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

    pub const fn superclocks(self) -> i64 {
        self.value()
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
    fn positions_count_superclocks_from_the_project_start() {
        assert_eq!(
            Position::<Clock>::from_seconds(2).superclocks(),
            564_480_000
        );
    }
}
