use std::marker::PhantomData;
use std::ops::{Add, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration<D>(i64, PhantomData<D>);

impl<D> Duration<D> {
    pub const ZERO: Self = Self::new(0);

    pub(super) const fn new(value: i64) -> Self {
        Self(value, PhantomData)
    }

    pub(super) const fn value(self) -> i64 {
        self.0
    }
}

impl<D> Add for Duration<D> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.0 + other.0)
    }
}

impl<D> Sub for Duration<D> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.0 - other.0)
    }
}

impl<D> Mul<i64> for Duration<D> {
    type Output = Self;

    fn mul(self, factor: i64) -> Self {
        Self::new(self.0 * factor)
    }
}

#[cfg(test)]
mod tests {
    use crate::time::{Duration, Musical};

    fn ticks(ticks: i64) -> Duration<Musical> {
        Duration::from_ticks(ticks)
    }

    #[test]
    fn zero_is_the_empty_duration() {
        assert_eq!(Duration::ZERO, ticks(0));
    }

    #[test]
    fn adding_two_durations_sums_their_lengths() {
        assert_eq!(ticks(3) + ticks(4), ticks(7));
    }

    #[test]
    fn subtracting_a_duration_gives_the_difference() {
        assert_eq!(ticks(3) - ticks(4), ticks(-1));
    }

    #[test]
    fn multiplying_a_duration_scales_its_length() {
        assert_eq!(ticks(3) * 4, ticks(12));
        assert_eq!(ticks(3) * -2, ticks(-6));
    }

    #[test]
    fn durations_order_by_length() {
        assert!(ticks(-1) < Duration::ZERO);
        assert!(Duration::ZERO < ticks(1));
    }
}
