use iced::widget::canvas::{self, Canvas, Frame, Geometry};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

use super::fader_position;
use crate::theme::Palette;

const HANDLE: f32 = 12.0;
const CLIP: f32 = 4.0;

struct Meter {
    level: f32,
    clipped: bool,
    palette: Palette,
}

pub fn meter<'a, Message: 'a>(level: f32, clipped: bool, palette: Palette) -> Element<'a, Message> {
    Canvas::new(Meter {
        level,
        clipped,
        palette,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

impl<Message> canvas::Program<Message> for Meter {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let size = bounds.size();
        let palette = self.palette;
        frame.fill_rectangle(Point::ORIGIN, size, iced::Color::from(palette.background));
        let travel = (size.height - HANDLE).max(1.0);
        let place = fader_position(20.0 * f64::from(self.level).log10());
        let top = HANDLE / 2.0 + (1.0 - place) * travel;
        let bottom = HANDLE / 2.0 + travel;
        if top < bottom {
            frame.fill_rectangle(
                Point::new(0.0, top),
                Size::new(size.width, bottom - top),
                iced::Color::from(palette.accent),
            );
        }
        if self.clipped {
            frame.fill_rectangle(
                Point::ORIGIN,
                Size::new(size.width, CLIP),
                iced::Color::from(palette.record),
            );
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use iced::{Element, Size};

    use crate::mixer::meter;
    use crate::pixels::Pixels;
    use crate::theme::Palette;

    const SIZE: Size = Size::new(8.0, 200.0);

    fn render(level: f32, clipped: bool) -> Pixels {
        let meter: Element<'_, ()> = meter(level, clipped, Palette::GRAPHITE);
        Pixels::render(meter, SIZE)
    }

    #[test]
    fn the_level_fills_the_meter_up_to_its_place_on_the_fader_scale() {
        let palette = Palette::GRAPHITE;
        let pixels = render(1.0, false);
        assert_eq!(pixels.color_at(4, 100), palette.accent);
        assert_eq!(pixels.color_at(4, 40), palette.background);
    }

    #[test]
    fn silence_leaves_the_meter_empty() {
        let pixels = render(0.0, false);
        assert_eq!(pixels.color_at(4, 190), Palette::GRAPHITE.background);
    }

    #[test]
    fn a_clipped_level_turns_the_top_of_the_meter_red() {
        let palette = Palette::GRAPHITE;
        assert_eq!(render(0.5, true).color_at(4, 1), palette.record);
        assert_eq!(render(0.5, false).color_at(4, 1), palette.background);
    }
}
