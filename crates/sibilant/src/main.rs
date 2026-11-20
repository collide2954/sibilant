mod app;

use app::App;
use sibilant_ui::fonts;

const WINDOW_SIZE: (f32, f32) = (1440.0, 900.0);

fn main() -> iced::Result {
    let application = iced::application(App::default, (), App::view)
        .title("Sibilant")
        .theme(App::theme)
        .default_font(fonts::TEXT)
        .window_size(WINDOW_SIZE);
    fonts::FILES
        .into_iter()
        .fold(application, iced::Application::font)
        .run()
}
