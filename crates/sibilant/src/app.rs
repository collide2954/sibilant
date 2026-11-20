use iced::{Element, Theme};
use sibilant_core::history::History;
use sibilant_core::mix::Mix;
use sibilant_core::project::{EditError, Ids, Project, TrackId};
use sibilant_ui::command::Command;
use sibilant_ui::theme::Palette;
use sibilant_ui::timeline::Viewport;
use sibilant_ui::{shell, track_area};

#[derive(Debug)]
pub(crate) struct App {
    palette: Palette,
    history: History<Project>,
    ids: Ids,
    viewport: Viewport,
}

impl Default for App {
    fn default() -> Self {
        let mut ids = Ids::default();
        let project = Project::new(&mut ids);
        Self {
            palette: Palette::default(),
            history: History::new(project),
            ids,
            viewport: Viewport::default(),
        }
    }
}

impl App {
    pub(crate) fn update(&mut self, command: Command) {
        match command {
            Command::AddTrack => {
                let id = self.ids.track();
                self.edit(|project| project.add_track(id, project.next_track_name()));
            }
            Command::ToggleMute(track) => {
                self.edit(|project| change_mix(project, track, |mix| mix.mute = !mix.mute));
            }
            Command::ToggleSolo(track) => {
                self.edit(|project| change_mix(project, track, |mix| mix.solo = !mix.solo));
            }
            Command::Undo => {
                self.history.undo();
            }
            Command::Redo => {
                self.history.redo();
            }
            Command::ScrollTimeline { pixels } => {
                self.viewport = self.viewport.scrolled(pixels);
            }
            Command::ZoomTimeline { steps, anchor } => {
                self.viewport = self.viewport.zoomed(steps, anchor);
            }
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Command> {
        let tracks = track_area::view(self.history.present(), self.viewport, self.palette);
        shell::view(self.palette, tracks)
    }

    pub(crate) fn theme(&self) -> Theme {
        Theme::from(self.palette)
    }

    fn edit(&mut self, edit: impl FnOnce(&Project) -> Result<Project, EditError>) {
        if let Ok(project) = edit(self.history.present()) {
            self.history.commit(project);
        }
    }
}

fn change_mix(
    project: &Project,
    track: TrackId,
    change: impl FnOnce(&mut Mix),
) -> Result<Project, EditError> {
    let mut mix = project
        .track(track)
        .ok_or(EditError::UnknownTrack(track))?
        .mix();
    change(&mut mix);
    project.set_mix(track, mix)
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use sibilant_core::project::{Project, Track, TrackId};
    use sibilant_core::time::Position;
    use sibilant_ui::command::Command;
    use sibilant_ui::theme::Palette;

    use crate::app::App;

    fn project(app: &App) -> &Project {
        app.history.present()
    }

    fn track_names(app: &App) -> Vec<&str> {
        project(app).tracks().map(Track::name).collect()
    }

    fn first_track(app: &App) -> &Track {
        project(app).tracks().next().unwrap()
    }

    #[test]
    fn the_app_starts_with_a_new_project() {
        assert_eq!(track_names(&App::default()), ["Audio 1", "Audio 2"]);
    }

    #[test]
    fn adding_a_track_appends_the_next_audio_track() {
        let mut app = App::default();
        app.update(Command::AddTrack);
        assert_eq!(track_names(&app), ["Audio 1", "Audio 2", "Audio 3"]);
    }

    #[test]
    fn undo_and_redo_step_through_the_changes() {
        let mut app = App::default();
        app.update(Command::AddTrack);
        app.update(Command::Undo);
        assert_eq!(track_names(&app).len(), 2);
        app.update(Command::Redo);
        assert_eq!(track_names(&app).len(), 3);
    }

    #[test]
    fn a_track_added_after_an_undo_gets_a_new_id() {
        let mut app = App::default();
        app.update(Command::AddTrack);
        let undone: TrackId = project(&app).tracks().last().unwrap().id();
        app.update(Command::Undo);
        app.update(Command::AddTrack);
        assert_ne!(project(&app).tracks().last().unwrap().id(), undone);
    }

    #[test]
    fn zoom_and_scroll_change_the_view_but_not_the_undo_history() {
        let mut app = App::default();
        app.update(Command::ZoomTimeline {
            steps: 1.0,
            anchor: 0.0,
        });
        app.update(Command::ScrollTimeline { pixels: 80.0 });
        assert!((app.viewport.pixels_per_quarter_note() - 40.0).abs() < 1e-9);
        assert_eq!(app.viewport.start(), Position::from_quarter_notes(2));
        assert!(!app.history.can_undo());
    }

    #[test]
    fn mute_and_solo_switch_the_mix_of_a_track_on_and_off() {
        let mut app = App::default();
        let track = first_track(&app).id();
        app.update(Command::ToggleMute(track));
        app.update(Command::ToggleSolo(track));
        assert!(first_track(&app).mix().mute && first_track(&app).mix().solo);
        app.update(Command::ToggleMute(track));
        assert!(!first_track(&app).mix().mute);
        app.update(Command::Undo);
        assert!(first_track(&app).mix().mute);
    }

    #[test]
    fn the_app_uses_the_graphite_theme() {
        let theme = App::default().theme();
        assert_eq!(
            theme.palette(),
            iced::Theme::from(Palette::GRAPHITE).palette()
        );
    }

    #[test]
    fn the_app_shows_the_zones_of_the_main_window() {
        let app = App::default();
        let mut ui = simulator(app.view());
        for zone in ["Transport", "Browser", "Audio 1", "Inspector", "Mixer"] {
            assert!(ui.find(zone).is_ok(), "{zone}");
        }
    }
}
