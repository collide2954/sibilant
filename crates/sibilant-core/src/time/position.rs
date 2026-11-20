use std::marker::PhantomData;
use std::ops::{Add, Sub};

use super::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position<D>(i64, PhantomData<D>);

impl<D> Position<D> {
    pub const START: Self = Self::new(0);

    pub(super) const fn new(value: i64) -> Self {
        Self(value, PhantomData)
    }

    pub(super) const fn value(self) -> i64 {
        self.0
    }
}

impl<D> Add<Duration<D>> for Position<D> {
    type Output = Self;

    fn add(self, duration: Duration<D>) -> Self {
        Self::new(self.0 + duration.value())
    }
}

impl<D> Sub<Duration<D>> for Position<D> {
    type Output = Self;

    fn sub(self, duration: Duration<D>) -> Self {
        Self::new(self.0 - duration.value())
    }
}

impl<D> Sub for Position<D> {
    type Output = Duration<D>;

    fn sub(self, earlier: Self) -> Duration<D> {
        Duration::new(self.0 - earlier.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::time::{Clock, Duration, Musical, Position};

    #[test]
    fn the_start_is_the_zero_position_of_each_domain() {
        assert_eq!(Position::<Musical>::START, Position::from_ticks(0));
        assert_eq!(Position::<Clock>::START, Position::from_superclocks(0));
    }

    #[test]
    fn adding_a_duration_moves_a_position_forward() {
        let position = Position::<Musical>::from_ticks(2) + Duration::from_ticks(3);
        assert_eq!(position, Position::from_ticks(5));
    }

    #[test]
    fn subtracting_a_duration_moves_a_position_back() {
        let position = Position::<Musical>::from_ticks(5) - Duration::from_ticks(3);
        assert_eq!(position, Position::from_ticks(2));
    }

    #[test]
    fn subtracting_two_positions_gives_the_duration_between_them() {
        let duration = Position::<Musical>::from_ticks(5) - Position::from_ticks(2);
        assert_eq!(duration, Duration::from_ticks(3));
    }
}
