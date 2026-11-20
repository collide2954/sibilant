use iced::keyboard::Modifiers;
use iced::widget::canvas::{self, Action, Canvas, Event, Frame, Geometry};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};
use sibilant_core::time::TimeSignatureMap;

use super::{Viewport, wheel};
use crate::command::Command;
use crate::fonts;
use crate::theme::Palette;

pub const HEIGHT: f32 = 24.0;
const LABEL_INSET: f32 = 4.0;
const LABEL_SIZE: f32 = 11.0;
const SHORT_LINE: f32 = HEIGHT / 3.0;

struct Ruler<'a> {
    viewport: Viewport,
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
        for line in self.viewport.grid(self.time_signatures, bounds.width).bars {
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
        vec![frame.into_geometry()]
    }
}

pub fn ruler(
    viewport: Viewport,
    time_signatures: &TimeSignatureMap,
    palette: Palette,
) -> Element<'_, Command> {
    let ruler = Ruler {
        viewport,
        time_signatures,
        palette,
    };
    Canvas::new(ruler).width(Length::Fill).height(HEIGHT).into()
}
