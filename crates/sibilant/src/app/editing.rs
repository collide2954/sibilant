use std::collections::BTreeSet;

use sibilant_core::project::{ClipKey, Track, TrackId};
use sibilant_core::time::{Musical, Position};
use sibilant_ui::timeline::{Grip, Marquee, Pointer};

use super::App;

const CLICK: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Spot {
    track: TrackId,
    at: Position<Musical>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Gesture {
    Box {
        from: Spot,
        to: Spot,
    },
    Grab {
        clip: ClipKey,
        grip: Grip,
        from: Spot,
        to: Spot,
        free: bool,
    },
}

impl App {
    pub(super) fn point(&mut self, pointer: Pointer) {
        match pointer {
            Pointer::Pressed {
                track,
                at,
                grip,
                toggle,
                free,
            } => self.press(Spot { track, at }, grip, toggle, free),
            Pointer::Moved { track, at, free } => self.drag_to(Spot { track, at }, free),
            Pointer::Released { track, at, free } => {
                self.drag_to(Spot { track, at }, free);
                self.finish_gesture();
            }
        }
    }

    pub(super) fn finish_gesture(&mut self) {
        match self.gesture.take() {
            Some(Gesture::Box { from, to }) => {
                let touched = self.touched(from, to);
                self.selection.extend(touched);
            }
            Some(Gesture::Grab {
                clip,
                grip: Grip::Body,
                from,
                to,
                ..
            }) if !self.moved(from, to) => self.selection = BTreeSet::from([clip]),
            Some(Gesture::Grab { .. }) | None => {}
        }
    }

    pub(super) fn marquee(&self) -> Option<Marquee> {
        match self.gesture {
            Some(Gesture::Box { from, to }) => Some(Marquee {
                from: from.at,
                to: to.at,
                tracks: (from.track, to.track),
            }),
            _ => None,
        }
    }

    fn press(&mut self, spot: Spot, grip: Option<(ClipKey, Grip)>, toggle: bool, free: bool) {
        match grip {
            Some((clip, Grip::Body)) if toggle => {
                if !self.selection.remove(&clip) {
                    self.selection.insert(clip);
                }
            }
            Some((clip, grip)) => {
                if grip != Grip::Body || !self.selection.contains(&clip) {
                    self.selection = BTreeSet::from([clip]);
                }
                self.gesture = Some(Gesture::Grab {
                    clip,
                    grip,
                    from: spot,
                    to: spot,
                    free,
                });
            }
            None => {
                if !toggle {
                    self.selection.clear();
                }
                self.gesture = Some(Gesture::Box {
                    from: spot,
                    to: spot,
                });
            }
        }
    }

    fn drag_to(&mut self, spot: Spot, now_free: bool) {
        match &mut self.gesture {
            Some(Gesture::Box { to, .. }) => *to = spot,
            Some(Gesture::Grab { to, free, .. }) => {
                *to = spot;
                *free = now_free;
            }
            None => {}
        }
    }

    fn moved(&self, from: Spot, to: Spot) -> bool {
        let distance = self.viewport.x_of(to.at) - self.viewport.x_of(from.at);
        from.track != to.track || distance.abs() >= CLICK
    }

    fn touched(&self, from: Spot, to: Spot) -> Vec<ClipKey> {
        let project = self.history.present();
        let rows: Vec<TrackId> = project.tracks().map(Track::id).collect();
        let row = |track| rows.iter().position(|&id| id == track);
        let (Some(first), Some(last)) = (row(from.track), row(to.track)) else {
            return Vec::new();
        };
        let (start, end) = (from.at.min(to.at), from.at.max(to.at));
        let mut touched = Vec::new();
        for track in project
            .tracks()
            .take(first.max(last) + 1)
            .skip(first.min(last))
        {
            for (clip_start, clip) in track.timeline() {
                if clip_start < end && project.clip_end(clip_start, clip) > start {
                    touched.push(ClipKey {
                        track: track.id(),
                        start: clip_start,
                    });
                }
            }
        }
        touched
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use sibilant_core::history::History;
    use sibilant_core::project::{
        AudioSource, Clip, ClipKey, Ids, Project, Region, Track, TrackId,
    };
    use sibilant_core::time::{Duration, Position};
    use sibilant_ui::command::Command;
    use sibilant_ui::timeline::{Grip, Marquee, Pointer};

    use crate::app::document::Document;
    use crate::app::{App, Message};

    fn with_clips(clips: &[(usize, i64)]) -> (App, Vec<TrackId>) {
        let mut ids = Ids::default();
        let project = Project::new(&mut ids);
        let source = ids.source();
        let tracks: Vec<TrackId> = project.tracks().map(Track::id).collect();
        let second = Duration::from_seconds(1);
        let region = Region {
            start: Duration::ZERO,
            length: second,
        };
        let project = clips.iter().fold(
            project
                .add_source(AudioSource::new(source, "kick.wav", second))
                .unwrap(),
            |project, &(row, quarter_notes)| {
                let start = Position::from_quarter_notes(quarter_notes);
                let clip = Clip::new("Kick", source, region);
                project.place_clip(tracks[row], start, clip).unwrap()
            },
        );
        let app = App {
            document: Document::new(&project, None),
            history: History::new(project),
            ids,
            ..App::default()
        };
        (app, tracks)
    }

    fn key(track: TrackId, quarter_notes: i64) -> ClipKey {
        ClipKey {
            track,
            start: Position::from_quarter_notes(quarter_notes),
        }
    }

    fn point(app: &mut App, pointer: Pointer) {
        let _ = app.update(Message::Command(Command::Pointer(pointer)));
    }

    fn press(
        app: &mut App,
        track: TrackId,
        quarter_notes: i64,
        grip: Option<ClipKey>,
        toggle: bool,
    ) {
        point(
            app,
            Pointer::Pressed {
                track,
                at: Position::from_quarter_notes(quarter_notes),
                grip: grip.map(|clip| (clip, Grip::Body)),
                toggle,
                free: false,
            },
        );
    }

    fn move_to(app: &mut App, track: TrackId, quarter_notes: i64) {
        let at = Position::from_quarter_notes(quarter_notes);
        point(
            app,
            Pointer::Moved {
                track,
                at,
                free: false,
            },
        );
    }

    fn release(app: &mut App, track: TrackId, quarter_notes: i64) {
        let at = Position::from_quarter_notes(quarter_notes);
        point(
            app,
            Pointer::Released {
                track,
                at,
                free: false,
            },
        );
    }

    fn click(app: &mut App, clip: ClipKey, toggle: bool) {
        let at = clip.start.ticks() / Position::from_quarter_notes(1).ticks() + 1;
        press(app, clip.track, at, Some(clip), toggle);
        release(app, clip.track, at);
    }

    fn selected(app: &App) -> Vec<ClipKey> {
        app.selection.iter().copied().collect()
    }

    #[test]
    fn a_click_on_a_clip_selects_only_that_clip() {
        let (mut app, tracks) = with_clips(&[(0, 0), (0, 4)]);
        click(&mut app, key(tracks[0], 0), false);
        assert_eq!(selected(&app), [key(tracks[0], 0)]);
        click(&mut app, key(tracks[0], 4), false);
        assert_eq!(selected(&app), [key(tracks[0], 4)]);
    }

    #[test]
    fn ctrl_click_adds_and_removes_a_clip() {
        let (mut app, tracks) = with_clips(&[(0, 0), (0, 4)]);
        click(&mut app, key(tracks[0], 0), true);
        click(&mut app, key(tracks[0], 4), true);
        assert_eq!(selected(&app), [key(tracks[0], 0), key(tracks[0], 4)]);
        click(&mut app, key(tracks[0], 0), true);
        assert_eq!(selected(&app), [key(tracks[0], 4)]);
    }

    #[test]
    fn a_click_on_one_of_several_selected_clips_selects_only_it() {
        let (mut app, tracks) = with_clips(&[(0, 0), (0, 4)]);
        click(&mut app, key(tracks[0], 0), true);
        click(&mut app, key(tracks[0], 4), true);
        press(&mut app, tracks[0], 1, Some(key(tracks[0], 0)), false);
        assert_eq!(selected(&app).len(), 2);
        release(&mut app, tracks[0], 1);
        assert_eq!(selected(&app), [key(tracks[0], 0)]);
    }

    #[test]
    fn a_click_on_empty_space_clears_the_selection() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        click(&mut app, key(tracks[0], 0), false);
        press(&mut app, tracks[0], 10, None, false);
        assert_eq!(selected(&app), []);
        release(&mut app, tracks[0], 10);
        assert_eq!(selected(&app), []);
    }

    #[test]
    fn a_box_selects_the_clips_that_it_touches() {
        let (mut app, tracks) = with_clips(&[(0, 0), (0, 4), (1, 4), (0, 8)]);
        press(&mut app, tracks[0], 3, None, false);
        move_to(&mut app, tracks[1], 5);
        release(&mut app, tracks[1], 5);
        assert_eq!(selected(&app), [key(tracks[0], 4), key(tracks[1], 4)]);
    }

    #[test]
    fn a_box_on_one_track_leaves_the_clips_of_the_other_tracks() {
        let (mut app, tracks) = with_clips(&[(0, 4), (1, 4)]);
        press(&mut app, tracks[1], 3, None, false);
        release(&mut app, tracks[1], 5);
        assert_eq!(selected(&app), [key(tracks[1], 4)]);
    }

    #[test]
    fn escape_during_a_box_ends_the_box_without_a_selection() {
        let (mut app, tracks) = with_clips(&[(0, 4)]);
        press(&mut app, tracks[0], 3, None, false);
        move_to(&mut app, tracks[0], 5);
        let _ = app.update(Message::Command(Command::Cancel));
        let _ = app.update(Message::DragEnded);
        assert_eq!(selected(&app), []);
    }

    #[test]
    fn the_lanes_see_the_selection_the_box_and_the_gesture() {
        let (mut app, tracks) = with_clips(&[(0, 4)]);
        click(&mut app, key(tracks[0], 4), false);
        press(&mut app, tracks[1], 3, None, true);
        let view = app.timeline_view(Position::START);
        assert!(view.pointing);
        assert_eq!(view.selection, &app.selection);
        assert_eq!(view.marquee, app.marquee());
        assert!(view.marquee.is_some());
    }

    #[test]
    fn ctrl_and_a_box_add_to_the_selection() {
        let (mut app, tracks) = with_clips(&[(0, 0), (1, 4)]);
        click(&mut app, key(tracks[0], 0), false);
        press(&mut app, tracks[1], 3, None, true);
        release(&mut app, tracks[1], 5);
        assert_eq!(selected(&app), [key(tracks[0], 0), key(tracks[1], 4)]);
    }

    #[test]
    fn the_box_shows_while_it_is_drawn() {
        let (mut app, tracks) = with_clips(&[]);
        press(&mut app, tracks[0], 3, None, false);
        move_to(&mut app, tracks[1], 5);
        assert_eq!(
            app.marquee(),
            Some(Marquee {
                from: Position::from_quarter_notes(3),
                to: Position::from_quarter_notes(5),
                tracks: (tracks[0], tracks[1]),
            })
        );
        release(&mut app, tracks[1], 5);
        assert_eq!(app.marquee(), None);
    }

    #[test]
    fn a_release_outside_the_lanes_ends_the_box_where_it_was() {
        let (mut app, tracks) = with_clips(&[(0, 4), (1, 4)]);
        press(&mut app, tracks[0], 3, None, false);
        move_to(&mut app, tracks[1], 5);
        let _ = app.update(Message::DragEnded);
        assert_eq!(selected(&app), [key(tracks[0], 4), key(tracks[1], 4)]);
        assert_eq!(app.gesture, None);
    }

    #[test]
    fn escape_clears_the_selection() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        click(&mut app, key(tracks[0], 0), false);
        let _ = app.update(Message::Command(Command::Cancel));
        assert_eq!(selected(&app), []);
    }

    #[test]
    fn undo_and_redo_clear_the_selection() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        let _ = app.update(Message::Command(Command::AddTrack));
        click(&mut app, key(tracks[0], 0), false);
        let _ = app.update(Message::Command(Command::Undo));
        assert_eq!(selected(&app), []);
        click(&mut app, key(tracks[0], 0), false);
        let _ = app.update(Message::Command(Command::Redo));
        assert_eq!(selected(&app), []);
    }

    #[test]
    fn a_new_project_starts_without_a_selection() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        click(&mut app, key(tracks[0], 0), false);
        let _ = app.update(Message::Command(Command::New));
        assert_eq!(app.selection, BTreeSet::new());
    }
}
