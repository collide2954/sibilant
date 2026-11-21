use std::collections::BTreeSet;

use sibilant_core::project::{ClipKey, EditError, Edited, Project, Track, TrackId};
use sibilant_core::time::{Musical, Position};
use sibilant_ui::timeline::{Grip, Marquee, Pointer};

use super::App;

const CLICK: f32 = 4.0;

impl App {
    pub(super) fn delete_selection(&mut self) {
        self.edit_selection(|project, clips| {
            Ok(Edited {
                project: project.remove_clips(clips)?,
                clips: Vec::new(),
            })
        });
    }

    pub(super) fn split_selection(&mut self) {
        let at = self.play_start;
        self.edit_selection(|project, clips| project.split_clips(clips, at));
    }

    pub(super) fn duplicate_selection(&mut self) {
        self.edit_selection(Project::duplicate_clips);
    }

    pub(super) fn shown(&self) -> (&Project, &BTreeSet<ClipKey>) {
        match &self.preview {
            Some(preview) => (&preview.project, &preview.selection),
            None => (self.history.present(), &self.selection),
        }
    }

    fn edit_selection(
        &mut self,
        edit: impl FnOnce(&Project, &[ClipKey]) -> Result<Edited, EditError>,
    ) {
        let clips: Vec<ClipKey> = self.selection.iter().copied().collect();
        if let Ok(edited) = edit(self.history.present(), &clips) {
            self.commit(Preview::from(edited));
        }
    }

    fn commit(&mut self, preview: Preview) {
        if preview.project != *self.history.present() {
            self.history.commit(preview.project);
            self.sync();
        }
        self.selection = preview.selection;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Preview {
    project: Project,
    selection: BTreeSet<ClipKey>,
}

impl From<Edited> for Preview {
    fn from(edited: Edited) -> Self {
        Self {
            project: edited.project,
            selection: edited.clips.into_iter().collect(),
        }
    }
}

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
                grip,
                from,
                to,
                ..
            }) => match self.preview.take() {
                Some(preview) => self.commit(preview),
                None if grip == Grip::Body && !self.moved(from, to) => {
                    self.selection = BTreeSet::from([clip]);
                }
                None => {}
            },
            None => {}
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
        self.preview = self.grabbed_edit().map(Preview::from);
    }

    fn grabbed_edit(&self) -> Option<Edited> {
        let Some(Gesture::Grab {
            clip,
            grip: Grip::Body,
            from,
            to,
            free,
        }) = self.gesture
        else {
            return None;
        };
        if !self.moved(from, to) {
            return None;
        }
        let offset = self.snapped(clip.start + (to.at - from.at), free) - clip.start;
        let rows = self.rows_between(from.track, to.track)?;
        let clips: Vec<ClipKey> = self.selection.iter().copied().collect();
        self.history.present().move_clips(&clips, offset, rows).ok()
    }

    fn snapped(&self, position: Position<Musical>, free: bool) -> Position<Musical> {
        if free {
            position
        } else {
            let time_signatures = self.history.present().time_signature_map();
            self.viewport
                .snap(self.viewport.x_of(position), time_signatures)
        }
    }

    fn rows_between(&self, from: TrackId, to: TrackId) -> Option<isize> {
        let row = |track| {
            let index = self
                .history
                .present()
                .tracks()
                .position(|other| other.id() == track)?;
            isize::try_from(index).ok()
        };
        Some(row(to)? - row(from)?)
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
                .add_source(AudioSource::new(
                    source,
                    "kick.wav",
                    Duration::from_seconds(10),
                ))
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

    fn run(app: &mut App, command: Command) {
        let _ = app.update(Message::Command(command));
    }

    fn starts(app: &App, track: TrackId) -> Vec<i64> {
        let quarter = Position::from_quarter_notes(1).ticks();
        app.history
            .present()
            .track(track)
            .unwrap()
            .timeline()
            .map(|(start, _)| start.ticks() / quarter)
            .collect()
    }

    #[test]
    fn delete_removes_the_selected_clips_in_one_step() {
        let (mut app, tracks) = with_clips(&[(0, 0), (0, 4), (1, 4)]);
        click(&mut app, key(tracks[0], 0), true);
        click(&mut app, key(tracks[1], 4), true);
        run(&mut app, Command::Delete);
        assert_eq!(
            (starts(&app, tracks[0]), starts(&app, tracks[1])),
            (vec![4], vec![])
        );
        assert_eq!(selected(&app), []);
        run(&mut app, Command::Undo);
        assert_eq!(
            (starts(&app, tracks[0]), starts(&app, tracks[1])),
            (vec![0, 4], vec![4])
        );
    }

    #[test]
    fn delete_without_a_selection_changes_nothing() {
        let (mut app, _) = with_clips(&[(0, 0)]);
        run(&mut app, Command::Delete);
        assert!(!app.history.can_undo());
    }

    #[test]
    fn split_cuts_the_selected_clips_at_the_play_start_and_selects_the_parts() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        run(&mut app, Command::Locate(Position::from_quarter_notes(1)));
        click(&mut app, key(tracks[0], 0), false);
        run(&mut app, Command::Split);
        assert_eq!(starts(&app, tracks[0]), [0, 1]);
        assert_eq!(selected(&app), [key(tracks[0], 0), key(tracks[0], 1)]);
    }

    #[test]
    fn split_without_a_selected_clip_at_the_play_start_changes_nothing() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        run(&mut app, Command::Locate(Position::from_quarter_notes(8)));
        click(&mut app, key(tracks[0], 0), false);
        run(&mut app, Command::Split);
        assert!(!app.history.can_undo());
        assert_eq!(selected(&app), [key(tracks[0], 0)]);
    }

    #[test]
    fn duplicate_puts_copies_after_the_selection_and_selects_them() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        click(&mut app, key(tracks[0], 0), false);
        run(&mut app, Command::Duplicate);
        assert_eq!(starts(&app, tracks[0]), [0, 2]);
        assert_eq!(selected(&app), [key(tracks[0], 2)]);
    }

    const QUARTER: i64 = 705_600;

    fn grab(app: &mut App, clip: ClipKey, grip: Grip, track: TrackId, ticks: i64, free: bool) {
        point(
            app,
            Pointer::Pressed {
                track,
                at: Position::from_ticks(ticks),
                grip: Some((clip, grip)),
                toggle: false,
                free,
            },
        );
    }

    fn drag_ticks(app: &mut App, track: TrackId, ticks: i64, free: bool) {
        let at = Position::from_ticks(ticks);
        point(app, Pointer::Moved { track, at, free });
    }

    fn drop_ticks(app: &mut App, track: TrackId, ticks: i64, free: bool) {
        drag_ticks(app, track, ticks, free);
        let at = Position::from_ticks(ticks);
        point(app, Pointer::Released { track, at, free });
    }

    fn start_ticks(app: &App, track: TrackId) -> Vec<i64> {
        app.history
            .present()
            .track(track)
            .unwrap()
            .timeline()
            .map(|(start, _)| start.ticks())
            .collect()
    }

    #[test]
    fn a_drag_moves_the_clip_and_snaps_its_start_to_the_grid() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        grab(
            &mut app,
            key(tracks[0], 0),
            Grip::Body,
            tracks[0],
            QUARTER,
            false,
        );
        drop_ticks(&mut app, tracks[0], 3 * QUARTER + 4 * QUARTER / 10, false);
        assert_eq!(start_ticks(&app, tracks[0]), [2 * QUARTER]);
        assert_eq!(selected(&app), [key(tracks[0], 2)]);
    }

    #[test]
    fn with_alt_a_drag_moves_the_clip_without_snap() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        grab(
            &mut app,
            key(tracks[0], 0),
            Grip::Body,
            tracks[0],
            QUARTER,
            true,
        );
        drop_ticks(&mut app, tracks[0], 3 * QUARTER + 4 * QUARTER / 10, true);
        assert_eq!(
            start_ticks(&app, tracks[0]),
            [2 * QUARTER + 4 * QUARTER / 10]
        );
    }

    #[test]
    fn a_drag_to_another_track_moves_the_clip_there() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        grab(
            &mut app,
            key(tracks[0], 0),
            Grip::Body,
            tracks[0],
            QUARTER,
            false,
        );
        drop_ticks(&mut app, tracks[1], QUARTER, false);
        assert_eq!(start_ticks(&app, tracks[0]), Vec::<i64>::new());
        assert_eq!(start_ticks(&app, tracks[1]), [0]);
        assert_eq!(selected(&app), [key(tracks[1], 0)]);
    }

    #[test]
    fn a_drag_moves_all_the_selected_clips_together() {
        let (mut app, tracks) = with_clips(&[(0, 0), (1, 4)]);
        click(&mut app, key(tracks[0], 0), true);
        click(&mut app, key(tracks[1], 4), true);
        grab(
            &mut app,
            key(tracks[0], 0),
            Grip::Body,
            tracks[0],
            QUARTER,
            false,
        );
        drop_ticks(&mut app, tracks[0], 3 * QUARTER, false);
        assert_eq!(start_ticks(&app, tracks[0]), [2 * QUARTER]);
        assert_eq!(start_ticks(&app, tracks[1]), [6 * QUARTER]);
    }

    #[test]
    fn the_moved_clips_show_before_the_release() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        grab(
            &mut app,
            key(tracks[0], 0),
            Grip::Body,
            tracks[0],
            QUARTER,
            false,
        );
        drag_ticks(&mut app, tracks[0], 3 * QUARTER, false);
        let (shown, selection) = app.shown();
        let first = shown.track(tracks[0]).unwrap().timeline().next().unwrap();
        assert_eq!(first.0, Position::from_quarter_notes(2));
        assert_eq!(selection, &BTreeSet::from([key(tracks[0], 2)]));
        assert_eq!(app.timeline_view(Position::START).selection, selection);
        assert_eq!(start_ticks(&app, tracks[0]), [0]);
    }

    #[test]
    fn a_move_is_one_undo_step() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        grab(
            &mut app,
            key(tracks[0], 0),
            Grip::Body,
            tracks[0],
            QUARTER,
            false,
        );
        drop_ticks(&mut app, tracks[0], 3 * QUARTER, false);
        run(&mut app, Command::Undo);
        assert_eq!(start_ticks(&app, tracks[0]), [0]);
        assert!(!app.history.can_undo());
    }

    #[test]
    fn escape_during_a_drag_leaves_the_clips_where_they_were() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        grab(
            &mut app,
            key(tracks[0], 0),
            Grip::Body,
            tracks[0],
            QUARTER,
            false,
        );
        drag_ticks(&mut app, tracks[0], 3 * QUARTER, false);
        run(&mut app, Command::Cancel);
        let _ = app.update(Message::DragEnded);
        assert_eq!(start_ticks(&app, tracks[0]), [0]);
        assert!(!app.history.can_undo());
        assert_eq!(app.shown().0, app.history.present());
    }

    #[test]
    fn a_drag_of_less_than_4_pixels_is_a_click() {
        let (mut app, tracks) = with_clips(&[(0, 0), (0, 4)]);
        click(&mut app, key(tracks[0], 0), true);
        click(&mut app, key(tracks[0], 4), true);
        grab(
            &mut app,
            key(tracks[0], 0),
            Grip::Body,
            tracks[0],
            QUARTER,
            false,
        );
        drop_ticks(&mut app, tracks[0], QUARTER + 3 * QUARTER / 32, false);
        assert_eq!(start_ticks(&app, tracks[0]), [0, 4 * QUARTER]);
        assert_eq!(selected(&app), [key(tracks[0], 0)]);
    }

    #[test]
    fn a_release_outside_the_lanes_moves_the_clips_where_they_were_shown() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        grab(
            &mut app,
            key(tracks[0], 0),
            Grip::Body,
            tracks[0],
            QUARTER,
            false,
        );
        drag_ticks(&mut app, tracks[0], 3 * QUARTER, false);
        let _ = app.update(Message::DragEnded);
        assert_eq!(start_ticks(&app, tracks[0]), [2 * QUARTER]);
    }

    #[test]
    fn a_new_project_starts_without_a_selection() {
        let (mut app, tracks) = with_clips(&[(0, 0)]);
        click(&mut app, key(tracks[0], 0), false);
        let _ = app.update(Message::Command(Command::New));
        assert_eq!(app.selection, BTreeSet::new());
    }
}
