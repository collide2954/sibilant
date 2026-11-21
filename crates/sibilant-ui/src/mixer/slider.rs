use std::time::{Duration, Instant};

use iced::keyboard::{self, Modifiers};
use iced::widget::canvas::{self, Action, Canvas, Event, Frame, Geometry};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};
use sibilant_core::project::TrackId;

use super::{Channel, Mixing, fader_db, fader_position};
use crate::command::Command;
use crate::theme::Palette;

const HANDLE: f32 = 12.0;
const GROOVE: f32 = 2.0;
const FINE: f32 = 0.1;
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
const DOUBLE_CLICK_DISTANCE: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Control {
    Gain(Channel),
    Pan(TrackId),
}

struct Slider {
    control: Control,
    value: f32,
    palette: Palette,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Drag {
    anchor: f32,
    from: f32,
    fine: bool,
}

#[derive(Debug, Default)]
struct State {
    drag: Option<Drag>,
    modifiers: Modifiers,
    last_press: Option<(Instant, Point)>,
    at: f32,
    value: f32,
}

impl State {
    fn is_double_click(&mut self, position: Point) -> bool {
        let now = Instant::now();
        let double = self.last_press.is_some_and(|(time, place)| {
            now.duration_since(time) <= DOUBLE_CLICK
                && place.distance(position) <= DOUBLE_CLICK_DISTANCE
        });
        self.last_press = (!double).then_some((now, position));
        double
    }
}

pub fn fader<'a>(channel: Channel, db: f64, palette: Palette) -> Element<'a, Command> {
    let slider = Slider {
        control: Control::Gain(channel),
        value: fader_position(db),
        palette,
    };
    Canvas::new(slider)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn pan<'a>(track: TrackId, position: f64, palette: Palette) -> Element<'a, Command> {
    let slider = Slider {
        control: Control::Pan(track),
        value: to_f32(f64::midpoint(position, 1.0)),
        palette,
    };
    Canvas::new(slider)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

impl Slider {
    fn is_vertical(&self) -> bool {
        matches!(self.control, Control::Gain(_))
    }

    fn along(&self, point: Point) -> f32 {
        if self.is_vertical() {
            -point.y
        } else {
            point.x
        }
    }

    fn travel(&self, size: Size) -> f32 {
        let length = if self.is_vertical() {
            size.height
        } else {
            size.width
        };
        (length - HANDLE).max(1.0)
    }

    fn changed(&self, value: f32) -> Mixing {
        match self.control {
            Control::Gain(channel) => Mixing::Gain(channel, fader_db(value)),
            Control::Pan(track) => Mixing::Pan(track, f64::from(value) * 2.0 - 1.0),
        }
    }

    fn reset(&self) -> Mixing {
        match self.control {
            Control::Gain(channel) => Mixing::ResetGain(channel),
            Control::Pan(track) => Mixing::ResetPan(track),
        }
    }
}

impl canvas::Program<Command> for Slider {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Command>> {
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
                if let Some(drag) = state.drag.filter(|drag| drag.fine != modifiers.command()) {
                    state.drag = Some(Drag {
                        anchor: state.at,
                        from: state.value,
                        fine: !drag.fine,
                    });
                }
                None
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                if state.is_double_click(position) {
                    state.drag = None;
                    return Some(Action::publish(Command::Mix(self.reset())).and_capture());
                }
                state.at = self.along(position);
                state.value = self.value;
                state.drag = Some(Drag {
                    anchor: state.at,
                    from: self.value,
                    fine: state.modifiers.command(),
                });
                Some(Action::capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let drag = state.drag?;
                state.at = self.along(Point::new(position.x - bounds.x, position.y - bounds.y));
                let speed = if drag.fine { FINE } else { 1.0 };
                let moved = (state.at - drag.anchor) / self.travel(bounds.size()) * speed;
                state.value = (drag.from + moved).clamp(0.0, 1.0);
                Some(Action::publish(Command::Mix(self.changed(state.value))).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.drag.take()?;
                Some(Action::publish(Command::Mix(Mixing::Done)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = self.palette;
        let size = bounds.size();
        let offset = (1.0 - self.value) * self.travel(size);
        if self.is_vertical() {
            let groove = Point::new((size.width - GROOVE) / 2.0, HANDLE / 2.0);
            frame.fill_rectangle(
                groove,
                Size::new(GROOVE, self.travel(size)),
                iced::Color::from(palette.border),
            );
            frame.fill_rectangle(
                Point::new(0.0, offset),
                Size::new(size.width, HANDLE),
                iced::Color::from(palette.text),
            );
        } else {
            let middle = (size.height - GROOVE) / 2.0;
            let travel = self.travel(size);
            frame.fill_rectangle(
                Point::new(HANDLE / 2.0, middle),
                Size::new(travel, GROOVE),
                iced::Color::from(palette.border),
            );
            let (center, value) = (
                HANDLE / 2.0 + travel / 2.0,
                HANDLE / 2.0 + travel * self.value,
            );
            frame.fill_rectangle(
                Point::new(center.min(value), middle),
                Size::new((value - center).abs(), GROOVE),
                iced::Color::from(palette.accent),
            );
            frame.fill_rectangle(
                Point::new(travel * self.value, 0.0),
                Size::new(HANDLE, size.height),
                iced::Color::from(palette.text),
            );
        }
        vec![frame.into_geometry()]
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "slider values are between 0 and 1"
)]
fn to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use iced::keyboard::{self, Modifiers};
    use iced::{Element, Event, Point, Settings, Size, mouse};
    use iced_test::Simulator;
    use sibilant_core::project::TrackId;

    use crate::command::Command;
    use crate::mixer::{Channel, Mixing, fader, fader_db, pan};
    use crate::pixels::Pixels;
    use crate::theme::Palette;

    const FADER: Size = Size::new(40.0, 200.0);
    const PAN: Size = Size::new(100.0, 16.0);
    const TRACK: TrackId = TrackId::new(3);

    fn press() -> Event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
    }

    fn release() -> Event {
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
    }

    fn move_to(x: f32, y: f32) -> Event {
        Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(x, y),
        })
    }

    fn modifiers(modifiers: Modifiers) -> Event {
        Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers))
    }

    fn messages(
        element: Element<'_, Command>,
        size: Size,
        at: Point,
        events: Vec<Event>,
    ) -> Vec<Command> {
        let mut ui = Simulator::with_size(Settings::default(), size, element);
        ui.point_at(at);
        ui.simulate(events);
        ui.into_messages().collect()
    }

    fn master_fader() -> Element<'static, Command> {
        fader(Channel::Master, 0.0, Palette::GRAPHITE)
    }

    fn gain_after(events: Vec<Event>) -> f64 {
        match messages(master_fader(), FADER, Point::new(20.0, 100.0), events)[..] {
            [Command::Mix(Mixing::Gain(Channel::Master, db)), ..] => db,
            ref other => panic!("no gain change in {other:?}"),
        }
    }

    #[test]
    fn dragging_the_fader_up_raises_the_gain_from_its_value() {
        let db = gain_after(vec![press(), move_to(20.0, 60.0)]);
        let expected = fader_db(0.75 + 40.0 / 188.0);
        assert!((db - expected).abs() < 1e-6, "{db} instead of {expected}");
    }

    #[test]
    fn with_ctrl_the_fader_moves_ten_times_more_slowly() {
        let db = gain_after(vec![
            modifiers(Modifiers::COMMAND),
            press(),
            move_to(20.0, 60.0),
        ]);
        let expected = fader_db(0.75 + 4.0 / 188.0);
        assert!((db - expected).abs() < 1e-6, "{db} instead of {expected}");
    }

    #[test]
    fn the_fader_stops_at_plus_6_db() {
        let db = gain_after(vec![press(), move_to(20.0, -500.0)]);
        assert!((db - 6.0).abs() < 1e-9, "{db}");
    }

    #[test]
    fn a_click_on_the_fader_changes_nothing_and_ends_the_drag() {
        assert_eq!(
            messages(
                master_fader(),
                FADER,
                Point::new(20.0, 100.0),
                vec![press(), release(), move_to(20.0, 60.0)]
            ),
            [Command::Mix(Mixing::Done)]
        );
    }

    #[test]
    fn a_double_click_resets_the_fader() {
        assert_eq!(
            messages(
                master_fader(),
                FADER,
                Point::new(20.0, 100.0),
                vec![press(), release(), press()]
            ),
            [
                Command::Mix(Mixing::Done),
                Command::Mix(Mixing::ResetGain(Channel::Master))
            ]
        );
    }

    #[test]
    fn dragging_the_pan_to_the_right_moves_it_right() {
        let pan = pan(TRACK, 0.0, Palette::GRAPHITE);
        let moved = messages(
            pan,
            PAN,
            Point::new(50.0, 8.0),
            vec![press(), move_to(70.0, 8.0)],
        );
        let expected = (0.5 + 20.0 / 88.0) * 2.0 - 1.0;
        match moved[..] {
            [Command::Mix(Mixing::Pan(TRACK, position))] => {
                assert!((position - expected).abs() < 1e-6, "{position}");
            }
            ref other => panic!("no pan change in {other:?}"),
        }
    }

    #[test]
    fn a_double_click_centers_the_pan() {
        let pan = pan(TRACK, 0.5, Palette::GRAPHITE);
        let clicked = messages(
            pan,
            PAN,
            Point::new(50.0, 8.0),
            vec![press(), release(), press()],
        );
        assert_eq!(clicked.last(), Some(&Command::Mix(Mixing::ResetPan(TRACK))));
    }

    #[test]
    fn pressing_ctrl_during_a_drag_continues_from_where_the_fader_is() {
        let db = match messages(
            master_fader(),
            FADER,
            Point::new(20.0, 100.0),
            vec![
                press(),
                move_to(20.0, 60.0),
                modifiers(Modifiers::COMMAND),
                move_to(20.0, 20.0),
            ],
        )[..]
        {
            [_, Command::Mix(Mixing::Gain(Channel::Master, db))] => db,
            ref other => panic!("no second gain change in {other:?}"),
        };
        let expected = fader_db(0.75 + 40.0 / 188.0 + 4.0 / 188.0);
        assert!((db - expected).abs() < 1e-6, "{db} instead of {expected}");
    }

    #[test]
    fn the_pan_stops_at_the_right() {
        let pan = pan(TRACK, 0.0, Palette::GRAPHITE);
        let moved = messages(
            pan,
            PAN,
            Point::new(50.0, 8.0),
            vec![press(), move_to(900.0, 8.0)],
        );
        assert_eq!(moved, [Command::Mix(Mixing::Pan(TRACK, 1.0))]);
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        assert_eq!(
            messages(
                master_fader(),
                FADER,
                Point::new(20.0, 100.0),
                vec![release()]
            ),
            []
        );
    }

    #[test]
    fn two_presses_far_apart_are_two_single_presses() {
        let mut ui = Simulator::with_size(Settings::default(), FADER, master_fader());
        ui.point_at(Point::new(20.0, 100.0));
        ui.simulate([press(), release()]);
        ui.point_at(Point::new(20.0, 150.0));
        ui.simulate([press(), release()]);
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            [Command::Mix(Mixing::Done), Command::Mix(Mixing::Done)]
        );
    }

    #[test]
    fn the_fader_handle_sits_at_the_gain() {
        let palette = Palette::GRAPHITE;
        let pixels = Pixels::render(master_fader(), FADER);
        assert_eq!(pixels.color_at(20, 53), palette.text);
        assert_ne!(pixels.color_at(20, 120), palette.text);
    }
}
