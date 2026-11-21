use iced::keyboard::{self, Key, Modifiers, key};
use iced::mouse::ScrollDelta;
use sibilant_core::project::TrackId;
use sibilant_core::time::{Musical, Position};

const WHEEL_LINE: f32 = 48.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    AddTrack,
    Continue,
    Drop {
        track: Option<TrackId>,
        start: Position<Musical>,
    },
    New,
    Open,
    PlayOrStop,
    Redo,
    Save,
    SaveAs,
    StopPreview,
    ScrollTimeline {
        pixels: f32,
    },
    ToggleMetronome,
    ToggleMute(TrackId),
    ToggleSolo(TrackId),
    Undo,
    ZoomTimeline {
        steps: f32,
        anchor: f32,
    },
}

impl Command {
    pub fn from_wheel(delta: ScrollDelta, modifiers: Modifiers, anchor: f32) -> Option<Self> {
        let (x, y) = match delta {
            ScrollDelta::Lines { x, y } => (x * WHEEL_LINE, y * WHEEL_LINE),
            ScrollDelta::Pixels { x, y } => (x, y),
        };
        if modifiers.command() {
            let steps = y / WHEEL_LINE;
            return (steps != 0.0).then_some(Self::ZoomTimeline { steps, anchor });
        }
        let sideways = if modifiers.shift() && y != 0.0 {
            y
        } else if x.abs() > y.abs() {
            x
        } else {
            0.0
        };
        (sideways != 0.0).then_some(Self::ScrollTimeline { pixels: -sideways })
    }

    pub fn from_key_event(event: keyboard::Event) -> Option<Self> {
        match event {
            keyboard::Event::KeyPressed {
                key,
                modifiers,
                repeat: false,
                ..
            } => Self::from_shortcut(&key, modifiers),
            _ => None,
        }
    }

    pub fn from_shortcut(key: &Key, modifiers: Modifiers) -> Option<Self> {
        match key {
            Key::Named(key::Named::Space) if !modifiers.command() => Some(if modifiers.shift() {
                Self::Continue
            } else {
                Self::PlayOrStop
            }),
            Key::Character(character) if modifiers.command() => {
                Self::from_command_character(character, modifiers.shift())
            }
            Key::Named(key::Named::Escape) => Some(Self::StopPreview),
            _ => None,
        }
    }

    fn from_command_character(character: &str, shift: bool) -> Option<Self> {
        match character.to_ascii_lowercase().as_str() {
            "n" if !shift => Some(Self::New),
            "o" if !shift => Some(Self::Open),
            "s" if !shift => Some(Self::Save),
            "s" => Some(Self::SaveAs),
            "t" if !shift => Some(Self::AddTrack),
            "z" if !shift => Some(Self::Undo),
            "z" => Some(Self::Redo),
            "y" if !shift && !cfg!(target_os = "macos") => Some(Self::Redo),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::{self, Key, Modifiers, key};
    use iced::mouse::ScrollDelta;

    use crate::command::Command;

    fn press(character: &str, modifiers: Modifiers) -> Option<Command> {
        Command::from_shortcut(&Key::Character(character.into()), modifiers)
    }

    #[test]
    fn command_t_adds_a_track() {
        assert_eq!(press("t", Modifiers::COMMAND), Some(Command::AddTrack));
    }

    #[test]
    fn command_z_undoes() {
        assert_eq!(press("z", Modifiers::COMMAND), Some(Command::Undo));
    }

    #[test]
    fn command_shift_z_redoes() {
        let modifiers = Modifiers::COMMAND | Modifiers::SHIFT;
        assert_eq!(press("z", modifiers), Some(Command::Redo));
        assert_eq!(press("Z", modifiers), Some(Command::Redo));
    }

    #[test]
    fn command_n_starts_a_new_project() {
        assert_eq!(press("n", Modifiers::COMMAND), Some(Command::New));
    }

    #[test]
    fn command_o_opens_a_project() {
        assert_eq!(press("o", Modifiers::COMMAND), Some(Command::Open));
    }

    #[test]
    fn command_s_saves_and_command_shift_s_saves_under_a_new_name() {
        let shift = Modifiers::COMMAND | Modifiers::SHIFT;
        assert_eq!(press("s", Modifiers::COMMAND), Some(Command::Save));
        assert_eq!(press("s", shift), Some(Command::SaveAs));
        assert_eq!(press("S", shift), Some(Command::SaveAs));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn ctrl_y_also_redoes() {
        assert_eq!(press("y", Modifiers::COMMAND), Some(Command::Redo));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn cmd_y_is_not_a_shortcut() {
        assert_eq!(press("y", Modifiers::COMMAND), None);
    }

    fn space(modifiers: Modifiers) -> Option<Command> {
        Command::from_shortcut(&Key::Named(key::Named::Space), modifiers)
    }

    #[test]
    fn space_plays_and_stops() {
        assert_eq!(space(Modifiers::empty()), Some(Command::PlayOrStop));
    }

    #[test]
    fn shift_space_continues_from_where_playback_stopped() {
        assert_eq!(space(Modifiers::SHIFT), Some(Command::Continue));
    }

    #[test]
    fn command_space_is_not_a_transport_shortcut() {
        assert_eq!(space(Modifiers::COMMAND), None);
    }

    #[test]
    fn escape_stops_the_preview() {
        let escape = Key::Named(key::Named::Escape);
        assert_eq!(
            Command::from_shortcut(&escape, Modifiers::empty()),
            Some(Command::StopPreview)
        );
    }

    #[test]
    fn keys_without_the_command_key_are_not_shortcuts() {
        assert_eq!(press("t", Modifiers::empty()), None);
        assert_eq!(press("z", Modifiers::SHIFT), None);
    }

    fn key_event(character: &str, repeat: bool) -> keyboard::Event {
        keyboard::Event::KeyPressed {
            key: Key::Character(character.into()),
            modified_key: Key::Character(character.into()),
            physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
            location: keyboard::Location::Standard,
            modifiers: Modifiers::COMMAND,
            text: None,
            repeat,
        }
    }

    #[test]
    fn a_key_press_sends_the_command_of_its_shortcut() {
        assert_eq!(
            Command::from_key_event(key_event("t", false)),
            Some(Command::AddTrack)
        );
    }

    #[test]
    fn a_key_that_repeats_while_held_sends_no_more_commands() {
        assert_eq!(Command::from_key_event(key_event("t", true)), None);
    }

    fn lines(x: f32, y: f32) -> ScrollDelta {
        ScrollDelta::Lines { x, y }
    }

    #[test]
    fn command_and_the_wheel_zoom_around_the_mouse() {
        let zoom_in = Command::from_wheel(lines(0.0, 1.0), Modifiers::COMMAND, 100.0);
        let zoom_out = Command::from_wheel(lines(0.0, -2.0), Modifiers::COMMAND, 100.0);
        let steps = |steps| Command::ZoomTimeline {
            steps,
            anchor: 100.0,
        };
        assert_eq!(zoom_in, Some(steps(1.0)));
        assert_eq!(zoom_out, Some(steps(-2.0)));
    }

    #[test]
    fn shift_and_the_wheel_scroll_the_timeline() {
        let down = Command::from_wheel(lines(0.0, -1.0), Modifiers::SHIFT, 0.0);
        let sideways = Command::from_wheel(lines(-1.0, 0.0), Modifiers::SHIFT, 0.0);
        let later = Some(Command::ScrollTimeline { pixels: 48.0 });
        assert_eq!(down, later);
        assert_eq!(sideways, later);
    }

    #[test]
    fn a_horizontal_swipe_scrolls_the_timeline() {
        let swipe = ScrollDelta::Pixels { x: -30.0, y: 0.0 };
        assert_eq!(
            Command::from_wheel(swipe, Modifiers::empty(), 0.0),
            Some(Command::ScrollTimeline { pixels: 30.0 })
        );
    }

    #[test]
    fn the_plain_wheel_leaves_the_scrolling_to_the_track_list() {
        assert_eq!(
            Command::from_wheel(lines(0.0, 1.0), Modifiers::empty(), 0.0),
            None
        );
    }
}
