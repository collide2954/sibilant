use std::error::Error;
use std::fmt;
use std::ops::RangeInclusive;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Pan(f64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelGains {
    pub left: f64,
    pub right: f64,
}

impl Pan {
    pub const RANGE: RangeInclusive<f64> = -1.0..=1.0;
    pub const CENTER: Self = Self(0.0);

    pub fn new(position: f64) -> Result<Self, PanOutOfRange> {
        if Self::RANGE.contains(&position) {
            Ok(Self(position))
        } else {
            Err(PanOutOfRange(position))
        }
    }

    pub const fn position(self) -> f64 {
        self.0
    }

    pub fn channel_gains(self) -> ChannelGains {
        ChannelGains {
            left: (1.0 - self.0).min(1.0),
            right: (1.0 + self.0).min(1.0),
        }
    }
}

impl Default for Pan {
    fn default() -> Self {
        Self::CENTER
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanOutOfRange(f64);

impl fmt::Display for PanOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pan {} is not in the range {} to {}",
            self.0,
            Pan::RANGE.start(),
            Pan::RANGE.end()
        )
    }
}

impl Error for PanOutOfRange {}

#[cfg(test)]
mod tests {
    use crate::mix::{ChannelGains, Pan};

    fn gains(position: f64) -> ChannelGains {
        Pan::new(position).unwrap().channel_gains()
    }

    #[test]
    fn accepts_positions_from_left_to_right() {
        for position in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            assert_eq!(Pan::new(position).map(Pan::position), Ok(position));
        }
    }

    #[test]
    fn rejects_positions_outside_left_and_right() {
        for position in [-1.01, 1.01, f64::NAN] {
            assert!(Pan::new(position).is_err(), "{position}");
        }
    }

    #[test]
    fn the_default_pan_is_the_center() {
        assert_eq!(Pan::default(), Pan::CENTER);
    }

    #[test]
    fn the_center_keeps_both_channels() {
        assert_eq!(
            gains(0.0),
            ChannelGains {
                left: 1.0,
                right: 1.0
            }
        );
    }

    #[test]
    fn a_pan_lowers_only_the_far_channel() {
        assert_eq!(
            gains(0.5),
            ChannelGains {
                left: 0.5,
                right: 1.0
            }
        );
        assert_eq!(
            gains(-0.25),
            ChannelGains {
                left: 1.0,
                right: 0.75
            }
        );
    }

    #[test]
    fn a_hard_pan_silences_the_far_channel() {
        assert_eq!(
            gains(1.0),
            ChannelGains {
                left: 0.0,
                right: 1.0
            }
        );
        assert_eq!(
            gains(-1.0),
            ChannelGains {
                left: 1.0,
                right: 0.0
            }
        );
    }

    #[test]
    fn the_error_tells_the_valid_range() {
        let error = Pan::new(1.5).unwrap_err();
        assert_eq!(error.to_string(), "pan 1.5 is not in the range -1 to 1");
    }
}
