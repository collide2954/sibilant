use sibilant_core::time::TimeSignatureMap;

use super::Viewport;

const MIN_LABEL_SPACING: f32 = 48.0;
const MIN_BEAT_SPACING: f32 = 12.0;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Grid {
    pub bars: Vec<BarLine>,
    pub beats: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarLine {
    pub x: f32,
    pub bar: i64,
    pub labeled: bool,
}

impl Viewport {
    pub fn grid(&self, time_signatures: &TimeSignatureMap, width: f32) -> Grid {
        let mut grid = Grid::default();
        let first = time_signatures.to_bar_beat(self.position_at(0.0)).bar;
        for bar in first.. {
            let start = time_signatures.bar_start(bar);
            let x = self.x_of(start);
            if x > width {
                break;
            }
            let signature = time_signatures.time_signature_at(start);
            if x >= 0.0 {
                let interval = label_interval(self.width_of(signature.bar_length()));
                let labeled = (bar - 1).rem_euclid(interval) == 0;
                grid.bars.push(BarLine { x, bar, labeled });
            }
            let beat = signature.beat_length();
            if self.width_of(beat) >= MIN_BEAT_SPACING {
                let beats = (1..i64::from(signature.numerator()))
                    .map(|index| self.x_of(start + beat * index))
                    .filter(|x| (0.0..=width).contains(x));
                grid.beats.extend(beats);
            }
        }
        grid
    }
}

fn label_interval(bar_width: f32) -> i64 {
    let (mut interval, mut spacing) = (1, bar_width);
    while spacing < MIN_LABEL_SPACING && spacing > 0.0 {
        interval *= 2;
        spacing *= 2.0;
    }
    interval
}

#[cfg(test)]
mod tests {
    use sibilant_core::time::{TimeSignature, TimeSignatureMap};

    use crate::timeline::{BarLine, Viewport};

    fn common_time() -> TimeSignatureMap {
        TimeSignatureMap::default()
    }

    fn bar_xs(lines: &[BarLine]) -> Vec<f32> {
        lines.iter().map(|line| line.x).collect()
    }

    fn labeled_bars(lines: &[BarLine]) -> Vec<i64> {
        lines
            .iter()
            .filter(|line| line.labeled)
            .map(|line| line.bar)
            .collect()
    }

    #[test]
    fn bar_lines_sit_at_the_start_of_each_bar() {
        let grid = Viewport::default().grid(&common_time(), 300.0);
        assert_eq!(bar_xs(&grid.bars), [0.0, 128.0, 256.0]);
        assert_eq!(labeled_bars(&grid.bars), [1, 2, 3]);
    }

    #[test]
    fn beat_lines_sit_between_the_bar_lines() {
        let grid = Viewport::default().grid(&common_time(), 300.0);
        assert_eq!(grid.beats, [32.0, 64.0, 96.0, 160.0, 192.0, 224.0, 288.0]);
    }

    #[test]
    fn beats_narrower_than_12_px_have_no_lines() {
        let grid = Viewport::default()
            .with_zoom(8.0)
            .grid(&common_time(), 300.0);
        assert_eq!(grid.beats, Vec::<f32>::new());
    }

    #[test]
    fn narrow_bars_label_every_second_fourth_or_eighth_bar() {
        let map = common_time();
        let every_second = Viewport::default().with_zoom(8.0).grid(&map, 200.0);
        let every_fourth = Viewport::default().with_zoom(4.0).grid(&map, 200.0);
        assert_eq!(labeled_bars(&every_second.bars), [1, 3, 5, 7]);
        assert_eq!(labeled_bars(&every_fourth.bars), [1, 5, 9, 13]);
    }

    #[test]
    fn the_grid_follows_the_time_signature_changes() {
        let map = common_time().with_time_signature(2, TimeSignature::new(3, 4).unwrap());
        let grid = Viewport::default().grid(&map, 300.0);
        assert_eq!(bar_xs(&grid.bars), [0.0, 128.0, 224.0]);
        assert_eq!(grid.beats, [32.0, 64.0, 96.0, 160.0, 192.0, 256.0, 288.0]);
    }

    #[test]
    fn a_scrolled_view_starts_with_the_first_bar_line_that_it_shows() {
        let grid = Viewport::default()
            .scrolled(100.0)
            .grid(&common_time(), 200.0);
        assert_eq!(bar_xs(&grid.bars), [28.0, 156.0]);
        assert_eq!(labeled_bars(&grid.bars), [2, 3]);
    }
}
