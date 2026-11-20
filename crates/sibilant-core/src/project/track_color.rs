use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TrackColor(u8);

impl TrackColor {
    pub const COUNT: u8 = 16;

    pub fn new(index: u8) -> Result<Self, TrackColorOutOfRange> {
        if index < Self::COUNT {
            Ok(Self(index))
        } else {
            Err(TrackColorOutOfRange(index))
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn next(self) -> Self {
        Self((self.0 + 1) % Self::COUNT)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackColorOutOfRange(u8);

impl fmt::Display for TrackColorOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "track color {} is not in the range 0 to {}",
            self.0,
            TrackColor::COUNT - 1
        )
    }
}

impl Error for TrackColorOutOfRange {}

#[cfg(test)]
mod tests {
    use crate::project::TrackColor;

    fn color(index: u8) -> TrackColor {
        TrackColor::new(index).unwrap()
    }

    #[test]
    fn accepts_the_16_colors_of_the_palette() {
        for index in 0..16 {
            assert_eq!(TrackColor::new(index).map(TrackColor::index), Ok(index));
        }
    }

    #[test]
    fn rejects_numbers_above_15() {
        for index in [16, 255] {
            assert!(TrackColor::new(index).is_err(), "{index}");
        }
    }

    #[test]
    fn the_color_after_the_last_color_is_the_first_color() {
        assert_eq!(color(3).next(), color(4));
        assert_eq!(color(15).next(), color(0));
    }

    #[test]
    fn the_error_tells_the_valid_range() {
        let error = TrackColor::new(16).unwrap_err();
        assert_eq!(
            error.to_string(),
            "track color 16 is not in the range 0 to 15"
        );
    }
}
