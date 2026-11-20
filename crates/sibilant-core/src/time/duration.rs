use std::marker::PhantomData;
use std::ops::Mul;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration<D>(i64, PhantomData<D>);

impl<D> Duration<D> {
    pub(super) const fn new(value: i64) -> Self {
        Self(value, PhantomData)
    }

    pub(super) const fn value(self) -> i64 {
        self.0
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

    #[test]
    fn multiplying_a_duration_scales_its_length() {
        let duration = Duration::<Musical>::from_ticks(3);
        assert_eq!(duration * 4, Duration::from_ticks(12));
        assert_eq!(duration * -2, Duration::from_ticks(-6));
    }
}
