mod app;
mod audio;
mod library;

use app::App;
use sibilant_ui::{fonts, icons};

const WINDOW_SIZE: (f32, f32) = (1440.0, 900.0);

fn main() -> iced::Result {
    let application = iced::application(App::with_audio, App::update, App::view)
        .subscription(App::subscription)
        .title(App::title)
        .theme(App::theme)
        .default_font(fonts::TEXT)
        .window_size(WINDOW_SIZE);
    fonts::FILES
        .into_iter()
        .chain([icons::FILE])
        .fold(application, iced::Application::font)
        .run()
}
