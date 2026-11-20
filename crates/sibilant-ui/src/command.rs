use iced::keyboard::{Key, Modifiers};
use sibilant_core::project::TrackId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    AddTrack,
    Redo,
    ToggleMute(TrackId),
    ToggleSolo(TrackId),
    Undo,
}

impl Command {
    pub fn from_shortcut(key: &Key, modifiers: Modifiers) -> Option<Self> {
        let Key::Character(character) = key else {
            return None;
        };
        if !modifiers.command() {
            return None;
        }
        let shift = modifiers.shift();
        match character.to_ascii_lowercase().as_str() {
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
    use iced::keyboard::{Key, Modifiers};

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

    #[test]
    fn keys_without_the_command_key_are_not_shortcuts() {
        assert_eq!(press("t", Modifiers::empty()), None);
        assert_eq!(press("z", Modifiers::SHIFT), None);
    }
}
