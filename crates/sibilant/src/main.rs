mod app;

use app::App;
use iced::keyboard;
use sibilant_ui::command::Command;
use sibilant_ui::{fonts, icons};

const WINDOW_SIZE: (f32, f32) = (1440.0, 900.0);

fn main() -> iced::Result {
    let application = iced::application(App::default, App::update, App::view)
        .subscription(|_| keyboard::listen().filter_map(Command::from_key_event))
        .title("Sibilant")
        .theme(App::theme)
        .default_font(fonts::TEXT)
        .window_size(WINDOW_SIZE);
    fonts::FILES
        .into_iter()
        .chain([icons::FILE])
        .fold(application, iced::Application::font)
        .run()
}
