use iced::widget::canvas::{self, Action, Canvas, Event, Frame, Geometry};
use iced::widget::{center, stack, text};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};
use sibilant_core::project::TrackId;
use sibilant_core::time::TimeSignatureMap;

use super::View;
use crate::command::Command;
use crate::theme::Palette;

const LINE_WIDTH: f32 = 2.0;
const HINT_SIZE: f32 = 11.0;

pub(super) fn drop_action(
    view: View<'_>,
    time_signatures: &TimeSignatureMap,
    track: Option<TrackId>,
    event: &Event,
    bounds: Rectangle,
    cursor: mouse::Cursor,
) -> Option<Action<Command>> {
    if !view.dragging {
        return None;
    }
    match event {
        Event::Mouse(mouse::Event::CursorMoved { .. }) => Some(Action::request_redraw()),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            let position = cursor.position_in(bounds)?;
            let start = view.viewport.snap(position.x, time_signatures);
            Some(Action::publish(Command::Drop { track, start }).and_capture())
        }
        _ => None,
    }
}

pub(super) fn draw_drop_line(
    frame: &mut Frame,
    view: View<'_>,
    time_signatures: &TimeSignatureMap,
    bounds: Rectangle,
    cursor: mouse::Cursor,
    palette: Palette,
) {
    let Some(position) = cursor.position_in(bounds).filter(|_| view.dragging) else {
        return;
    };
    let start = view.viewport.snap(position.x, time_signatures);
    if let Some(x) = view.viewport.visible_x(start, bounds.width) {
        frame.fill_rectangle(
            Point::new(x, 0.0),
            Size::new(LINE_WIDTH, bounds.height),
            iced::Color::from(palette.accent),
        );
    }
}

pub(super) fn drag_interaction(
    view: View<'_>,
    bounds: Rectangle,
    cursor: mouse::Cursor,
) -> mouse::Interaction {
    if view.dragging && cursor.is_over(bounds) {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::default()
    }
}

struct NewTrackLane<'a> {
    view: View<'a>,
    time_signatures: &'a TimeSignatureMap,
    palette: Palette,
}

impl canvas::Program<Command> for NewTrackLane<'_> {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Command>> {
        drop_action(self.view, self.time_signatures, None, event, bounds, cursor)
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if self.view.dragging {
            frame.fill_rectangle(
                Point::ORIGIN,
                bounds.size(),
                iced::Color::from(self.palette.raised),
            );
            draw_drop_line(
                &mut frame,
                self.view,
                self.time_signatures,
                bounds,
                cursor,
                self.palette,
            );
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        drag_interaction(self.view, bounds, cursor)
    }
}

pub fn new_track_lane<'a>(
    view: View<'a>,
    time_signatures: &'a TimeSignatureMap,
    palette: Palette,
) -> Element<'a, Command> {
    let lane = Canvas::new(NewTrackLane {
        view,
        time_signatures,
        palette,
    })
    .width(Length::Fill)
    .height(Length::Fill);
    if view.dragging {
        let hint = text("Drop here for a new track")
            .size(HINT_SIZE)
            .color(palette.text_muted);
        stack![lane, center(hint)].into()
    } else {
        lane.into()
    }
}
