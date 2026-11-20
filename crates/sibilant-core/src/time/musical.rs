use super::{Duration, Position};

pub const TICKS_PER_BEAT: i64 = 705_600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Musical {}

impl Position<Musical> {
    pub const fn from_beats(beats: i64) -> Self {
        Self::from_ticks(beats * TICKS_PER_BEAT)
    }

    pub const fn from_ticks(ticks: i64) -> Self {
        Self::new(ticks)
    }

    pub const fn ticks(self) -> i64 {
        self.value()
    }
}

impl Duration<Musical> {
    pub const fn from_beats(beats: i64) -> Self {
        Self::from_ticks(beats * TICKS_PER_BEAT)
    }

    pub const fn from_ticks(ticks: i64) -> Self {
        Self::new(ticks)
    }

    pub const fn ticks(self) -> i64 {
        self.value()
    }
}

#[cfg(test)]
mod tests {
    use crate::time::{Duration, Musical, Position, TICKS_PER_BEAT};

    #[test]
    fn a_beat_has_705_600_ticks() {
        assert_eq!(Duration::<Musical>::from_beats(1).ticks(), 705_600);
    }

    #[test]
    fn a_beat_divides_exactly_into_tuplets_down_to_256th_notes() {
        for division in [3, 5, 7, 9, 25, 49, 64] {
            assert_eq!(TICKS_PER_BEAT % division, 0, "division by {division}");
        }
    }

    #[test]
    fn positions_count_ticks_from_the_project_start() {
        assert_eq!(Position::<Musical>::from_beats(2).ticks(), 1_411_200);
    }
}
