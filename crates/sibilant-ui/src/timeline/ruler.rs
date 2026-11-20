use iced::keyboard::Modifiers;
use iced::widget::canvas::{self, Action, Canvas, Event, Frame, Geometry};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};
use sibilant_core::time::TimeSignatureMap;

use super::{View, draw_playhead, wheel};
use crate::command::Command;
use crate::fonts;
use crate::theme::Palette;

pub const HEIGHT: f32 = 24.0;
const LABEL_INSET: f32 = 4.0;
const LABEL_SIZE: f32 = 11.0;
const SHORT_LINE: f32 = HEIGHT / 3.0;

struct Ruler<'a> {
    view: View,
    time_signatures: &'a TimeSignatureMap,
    palette: Palette,
}

impl canvas::Program<Command> for Ruler<'_> {
    type State = Modifiers;

    fn update(
        &self,
        modifiers: &mut Modifiers,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Command>> {
        wheel(modifiers, event, bounds, cursor)
    }

    fn draw(
        &self,
        _modifiers: &Modifiers,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = self.palette;
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            iced::Color::from(palette.raised),
        );
        for line in self
            .view
            .viewport
            .grid(self.time_signatures, bounds.width)
            .bars
        {
            let height = if line.labeled {
                bounds.height
            } else {
                SHORT_LINE
            };
            let top = Point::new(line.x, bounds.height - height);
            frame.fill_rectangle(
                top,
                Size::new(1.0, height),
                iced::Color::from(palette.text_muted),
            );
            if line.labeled {
                frame.fill_text(canvas::Text {
                    content: line.bar.to_string(),
                    position: Point::new(line.x + LABEL_INSET, LABEL_INSET),
                    color: palette.text_muted.into(),
                    size: LABEL_SIZE.into(),
                    font: fonts::NUMBERS,
                    ..canvas::Text::default()
                });
            }
        }
        draw_playhead(&mut frame, self.view, palette);
        vec![frame.into_geometry()]
    }
}

pub fn ruler(
    view: View,
    time_signatures: &TimeSignatureMap,
    palette: Palette,
) -> Element<'_, Command> {
    let ruler = Ruler {
        view,
        time_signatures,
        palette,
    };
    Canvas::new(ruler).width(Length::Fill).height(HEIGHT).into()
}

#[cfg(test)]
mod tests {
    use iced::Size;
    use sibilant_core::time::TimeSignatureMap;

    use crate::pixels::Pixels;
    use crate::theme::Palette;
    use crate::timeline::{RULER_HEIGHT, View, Viewport, ruler};

    #[test]
    fn the_playhead_is_a_line_in_the_accent_color() {
        let (viewport, palette) = (Viewport::default(), Palette::GRAPHITE);
        let view = View {
            viewport,
            playhead: viewport.position_at(144.0),
        };
        let time_signatures = TimeSignatureMap::default();
        let ruler = ruler(view, &time_signatures, palette);
        let pixels = Pixels::render(ruler, Size::new(400.0, RULER_HEIGHT));
        assert_eq!(pixels.color_at(144, 12), palette.accent);
        assert_eq!(pixels.color_at(146, 12), palette.raised);
    }
}
