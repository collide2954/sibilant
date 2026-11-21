use iced::keyboard::Modifiers;
use iced::widget::canvas::{self, Action, Canvas, Event, Frame, Geometry};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};
use sibilant_core::time::TimeSignatureMap;

use super::{View, draw_playhead, pointed, wheel};
use crate::command::Command;
use crate::fonts;
use crate::theme::Palette;

pub const HEIGHT: f32 = 24.0;
const LABEL_INSET: f32 = 4.0;
const LABEL_SIZE: f32 = 11.0;
const SHORT_LINE: f32 = HEIGHT / 3.0;
const MARKER_HALF_WIDTH: f32 = 5.0;
const MARKER_HEIGHT: f32 = 7.0;

struct Ruler<'a> {
    view: View<'a>,
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
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let position = cursor.position_in(bounds)?;
            let start = pointed(
                self.view.viewport,
                self.time_signatures,
                position.x,
                *modifiers,
            );
            return Some(Action::publish(Command::Locate(start)).and_capture());
        }
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
        if let Some(x) = self
            .view
            .viewport
            .visible_x(self.view.play_start, bounds.width)
        {
            let marker = canvas::Path::new(|path| {
                path.move_to(Point::new(x - MARKER_HALF_WIDTH, 0.0));
                path.line_to(Point::new(x + MARKER_HALF_WIDTH, 0.0));
                path.line_to(Point::new(x, MARKER_HEIGHT));
                path.close();
            });
            frame.fill(&marker, iced::Color::from(palette.text));
        }
        vec![frame.into_geometry()]
    }
}

pub fn ruler<'a>(
    view: View<'a>,
    time_signatures: &'a TimeSignatureMap,
    palette: Palette,
) -> Element<'a, Command> {
    let ruler = Ruler {
        view,
        time_signatures,
        palette,
    };
    Canvas::new(ruler).width(Length::Fill).height(HEIGHT).into()
}

#[cfg(test)]
mod tests {
    use iced::keyboard::{self, Modifiers};
    use iced::{Event, Point, Size, mouse};
    use iced_test::simulator;
    use sibilant_core::time::{Musical, Position, TimeSignatureMap};

    use crate::command::Command;
    use crate::pixels::Pixels;
    use crate::theme::Palette;
    use crate::timeline::{RULER_HEIGHT, View, Viewport, ruler};

    fn click_at(x: f32, modifiers: Modifiers) -> Vec<Command> {
        let time_signatures = TimeSignatureMap::default();
        let mut ui = simulator(ruler(View::default(), &time_signatures, Palette::GRAPHITE));
        ui.point_at(Point::new(x, 12.0));
        ui.simulate([
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        ]);
        ui.into_messages().collect()
    }

    #[test]
    fn a_click_on_the_ruler_moves_the_play_start_to_the_nearest_beat() {
        assert_eq!(
            click_at(40.0, Modifiers::empty()),
            [Command::Locate(Position::from_quarter_notes(1))]
        );
    }

    #[test]
    fn with_alt_a_click_on_the_ruler_does_not_snap() {
        let free: Position<Musical> = Position::from_ticks(882_000);
        assert_eq!(click_at(40.0, Modifiers::ALT), [Command::Locate(free)]);
    }

    #[test]
    fn the_play_start_shows_as_a_marker_at_the_top_of_the_ruler() {
        let (viewport, palette) = (Viewport::default(), Palette::GRAPHITE);
        let view = View {
            viewport,
            play_start: viewport.position_at(144.0),
            ..View::default()
        };
        let time_signatures = TimeSignatureMap::default();
        let pixels = Pixels::render(
            ruler(view, &time_signatures, palette),
            Size::new(400.0, RULER_HEIGHT),
        );
        assert_eq!(pixels.color_at(146, 1), palette.text);
        assert_eq!(pixels.color_at(146, 20), palette.raised);
    }

    #[test]
    fn the_playhead_is_a_line_in_the_accent_color() {
        let (viewport, palette) = (Viewport::default(), Palette::GRAPHITE);
        let view = View {
            viewport,
            playhead: viewport.position_at(144.0),
            ..View::default()
        };
        let time_signatures = TimeSignatureMap::default();
        let ruler = ruler(view, &time_signatures, palette);
        let pixels = Pixels::render(ruler, Size::new(400.0, RULER_HEIGHT));
        assert_eq!(pixels.color_at(144, 12), palette.accent);
        assert_eq!(pixels.color_at(146, 12), palette.raised);
    }
}
