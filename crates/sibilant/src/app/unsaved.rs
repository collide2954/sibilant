use iced::{Task, window};
use sibilant_ui::command::Command;
use sibilant_ui::dialog::Choice;

use super::{App, Message, dialogs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Leave {
    New,
    Open,
    Close(window::Id),
}

impl App {
    pub(super) fn leave(&mut self, leave: Leave) -> Task<Message> {
        if self.saving {
            self.waiting = Some(leave);
            Task::none()
        } else if self.document.is_changed(self.history.present()) {
            self.asking = Some(leave);
            Task::none()
        } else {
            self.go(leave)
        }
    }

    pub(super) fn answer(&mut self, choice: Choice) -> Task<Message> {
        let Some(leave) = self.asking.take() else {
            return Task::none();
        };
        match choice {
            Choice::Save => {
                self.waiting = Some(leave);
                self.save()
            }
            Choice::DontSave => self.go(leave),
            Choice::Cancel => Task::none(),
        }
    }

    pub(super) fn answer_by_key(&mut self, command: Command) -> Task<Message> {
        match command {
            Command::Confirm => self.answer(Choice::Save),
            Command::Cancel => self.answer(Choice::Cancel),
            _ => Task::none(),
        }
    }

    pub(super) fn resume_leaving(&mut self) -> Task<Message> {
        match self.waiting.take() {
            Some(leave) => self.leave(leave),
            None => Task::none(),
        }
    }

    fn go(&mut self, leave: Leave) -> Task<Message> {
        match leave {
            Leave::New => {
                self.start_new_project();
                Task::none()
            }
            Leave::Open => dialogs::open_file().map(Message::OpenFrom),
            Leave::Close(window) => window::close(window),
        }
    }
}
