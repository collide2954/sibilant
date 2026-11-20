use sibilant_core::time::{Duration, Musical, Position, TICKS_PER_QUARTER_NOTE};

const DEFAULT_ZOOM: f64 = 32.0;
const MIN_ZOOM: f64 = 2.0;
const MAX_ZOOM: f64 = 1000.0;
const ZOOM_STEP: f64 = 1.25;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    start: Position<Musical>,
    pixels_per_quarter_note: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            start: Position::START,
            pixels_per_quarter_note: DEFAULT_ZOOM,
        }
    }
}

impl Viewport {
    pub fn start(&self) -> Position<Musical> {
        self.start
    }

    pub fn pixels_per_quarter_note(&self) -> f64 {
        self.pixels_per_quarter_note
    }

    pub fn x_of(&self, position: Position<Musical>) -> f32 {
        to_f32(self.pixels_of((position - self.start).ticks()))
    }

    pub fn width_of(&self, duration: Duration<Musical>) -> f32 {
        to_f32(self.pixels_of(duration.ticks()))
    }

    pub fn position_at(&self, x: f32) -> Position<Musical> {
        self.start + Duration::from_ticks(round(self.ticks_of(f64::from(x))))
    }

    #[must_use]
    pub fn scrolled(self, pixels: f32) -> Self {
        let start = self.start + Duration::from_ticks(round(self.ticks_of(f64::from(pixels))));
        Self {
            start: start.max(Position::START),
            ..self
        }
    }

    #[must_use]
    pub fn zoomed(self, steps: f32, anchor: f32) -> Self {
        let anchor = f64::from(anchor);
        let anchor_ticks = to_f64(self.start.ticks()) + self.ticks_of(anchor);
        let zoom = self.pixels_per_quarter_note * ZOOM_STEP.powf(f64::from(steps));
        let zoomed = Self {
            pixels_per_quarter_note: zoom.clamp(MIN_ZOOM, MAX_ZOOM),
            ..self
        };
        let start = Position::from_ticks(round(anchor_ticks - zoomed.ticks_of(anchor)));
        Self {
            start: start.max(Position::START),
            ..zoomed
        }
    }

    fn pixels_of(&self, ticks: i64) -> f64 {
        to_f64(ticks) / to_f64(TICKS_PER_QUARTER_NOTE) * self.pixels_per_quarter_note
    }

    fn ticks_of(&self, pixels: f64) -> f64 {
        pixels / self.pixels_per_quarter_note * to_f64(TICKS_PER_QUARTER_NOTE)
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "tick counts stay far below 2^53"
)]
fn to_f64(value: i64) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "pixel distances fit in f32"
)]
fn to_f32(value: f64) -> f32 {
    value as f32
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "rounded tick counts stay inside the range of i64"
)]
fn round(value: f64) -> i64 {
    value.round() as i64
}

#[cfg(test)]
mod tests {
    use sibilant_core::time::{Musical, Position};

    use crate::timeline::Viewport;

    fn quarter_notes(quarter_notes: i64) -> Position<Musical> {
        Position::from_quarter_notes(quarter_notes)
    }

    fn is_close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn the_view_starts_at_bar_1_with_32_px_for_each_quarter_note() {
        let view = Viewport::default();
        assert!(is_close(f64::from(view.x_of(Position::START)), 0.0));
        assert!(is_close(f64::from(view.x_of(quarter_notes(1))), 32.0));
    }

    #[test]
    fn pixels_map_back_to_positions() {
        let view = Viewport::default();
        assert_eq!(view.position_at(64.0), quarter_notes(2));
    }

    #[test]
    fn scrolling_moves_the_view_by_the_pixels() {
        let view = Viewport::default().scrolled(64.0);
        assert_eq!(view.start(), quarter_notes(2));
        assert!(is_close(f64::from(view.x_of(quarter_notes(3))), 32.0));
    }

    #[test]
    fn the_view_cannot_scroll_before_bar_1() {
        let view = Viewport::default().scrolled(64.0).scrolled(-500.0);
        assert_eq!(view.start(), Position::START);
    }

    #[test]
    fn each_zoom_step_changes_the_scale_by_a_factor_of_1_25() {
        let view = Viewport::default();
        assert!(is_close(
            view.zoomed(1.0, 0.0).pixels_per_quarter_note(),
            40.0
        ));
        assert!(is_close(
            view.zoomed(-1.0, 0.0).pixels_per_quarter_note(),
            25.6
        ));
    }

    #[test]
    fn the_zoom_stays_between_2_and_1000_px_for_each_quarter_note() {
        let view = Viewport::default();
        assert!(is_close(
            view.zoomed(100.0, 0.0).pixels_per_quarter_note(),
            1000.0
        ));
        assert!(is_close(
            view.zoomed(-100.0, 0.0).pixels_per_quarter_note(),
            2.0
        ));
    }

    #[test]
    fn zooming_keeps_the_position_under_the_mouse_in_place() {
        let view = Viewport::default().scrolled(256.0);
        let under_mouse = view.position_at(100.0);
        for steps in [1.0, -1.0, 3.5] {
            let zoomed = view.zoomed(steps, 100.0);
            let moved = (zoomed.position_at(100.0) - under_mouse).ticks().abs();
            assert!(moved <= 1, "{steps} steps moved {moved} ticks");
        }
    }

    #[test]
    fn zooming_out_near_the_start_does_not_go_before_bar_1() {
        let view = Viewport::default().scrolled(16.0).zoomed(-3.0, 600.0);
        assert_eq!(view.start(), Position::START);
    }
}
