use std::sync::Arc;

use super::timeline::{first_tick_from, place};
use super::{Clip, EditError, Project, Region, TrackId};
use crate::time::{Duration, Musical, Position};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClipKey {
    pub track: TrackId,
    pub start: Position<Musical>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edited {
    pub project: Project,
    pub clips: Vec<ClipKey>,
}

impl Project {
    pub fn clip(&self, key: ClipKey) -> Result<&Clip, EditError> {
        let track = self
            .track(key.track)
            .ok_or(EditError::UnknownTrack(key.track))?;
        track
            .timeline
            .get(&key.start)
            .map(Arc::as_ref)
            .ok_or(EditError::NoClipAt(key.track, key.start))
    }

    pub fn split_clips(
        &self,
        clips: &[ClipKey],
        at: Position<Musical>,
    ) -> Result<Edited, EditError> {
        let mut edited = self.unedited();
        for &key in clips {
            let clip = edited.project.clip(key)?.clone();
            if at <= key.start || edited.project.clip_end(key.start, &clip) <= at {
                edited.clips.push(key);
                continue;
            }
            let cut = self.tempo_map.to_clock(at) - self.tempo_map.to_clock(key.start);
            edited.project = edited.project.edit_track(key.track, |track| {
                track
                    .timeline
                    .insert(key.start, Arc::new(clip.part_before(cut)));
                track.timeline.insert(at, Arc::new(clip.part_after(cut)));
            })?;
            edited.clips.extend([key, ClipKey { start: at, ..key }]);
        }
        Ok(edited)
    }

    pub fn trim_clip(
        &self,
        key: ClipKey,
        edge: Edge,
        to: Position<Musical>,
    ) -> Result<Edited, EditError> {
        let clip = self.clip(key)?;
        let source = self
            .source(clip.source())
            .ok_or(EditError::UnknownSource(clip.source()))?;
        let tempo = &self.tempo_map;
        let Region {
            start: offset,
            length,
        } = clip.region();
        let begin = tempo.to_clock(key.start);
        let (start, region) = match edge {
            Edge::Start => {
                let earliest = first_tick_from(tempo, begin - offset).max(Position::START);
                let latest = self.clip_end(key.start, clip) - Duration::from_ticks(1);
                let start = to.max(earliest).min(latest);
                let moved = tempo.to_clock(start) - begin;
                let region = Region {
                    start: offset + moved,
                    length: length - moved,
                };
                (start, region)
            }
            Edge::End => {
                let shortest = tempo.to_clock(key.start + Duration::from_ticks(1)) - begin;
                let longest = source.length() - offset;
                let wanted = tempo.to_clock(to) - begin;
                let region = Region {
                    start: offset,
                    length: wanted.max(shortest).min(longest),
                };
                (key.start, region)
            }
        };
        let trimmed = clip.with_region(region);
        let project = self.edit_track(key.track, |track| {
            track.timeline.remove(&key.start);
            place(&mut track.timeline, tempo, start, trimmed);
        })?;
        Ok(Edited {
            project,
            clips: vec![ClipKey { start, ..key }],
        })
    }

    pub fn move_clips(
        &self,
        clips: &[ClipKey],
        offset: Duration<Musical>,
        rows: isize,
    ) -> Result<Edited, EditError> {
        let moving = clips
            .iter()
            .map(|&key| Ok((key, self.clip(key)?.clone(), self.track_index(key.track)?)))
            .collect::<Result<Vec<_>, EditError>>()?;
        let (Some(earliest), Some(lowest), Some(highest)) = (
            clips.iter().map(|key| key.start).min(),
            moving.iter().map(|&(_, _, index)| index).min(),
            moving.iter().map(|&(_, _, index)| index).max(),
        ) else {
            return Ok(self.unedited());
        };
        let offset = offset.max(Position::START - earliest);
        let rows = rows.clamp(
            -isize::try_from(lowest).unwrap_or(isize::MAX),
            isize::try_from(self.tracks.len() - 1 - highest).unwrap_or(isize::MAX),
        );
        let targets: Vec<ClipKey> = moving
            .iter()
            .map(|&(key, _, index)| ClipKey {
                track: self.tracks[index.checked_add_signed(rows).unwrap_or(index)].id(),
                start: key.start + offset,
            })
            .collect();
        let mut project = self.clone();
        for &(key, _, _) in &moving {
            project = project.edit_track(key.track, |track| {
                track.timeline.remove(&key.start);
            })?;
        }
        for ((_, clip, _), target) in moving.into_iter().zip(&targets) {
            project = project.edit_track(target.track, |track| {
                place(&mut track.timeline, &self.tempo_map, target.start, clip);
            })?;
        }
        Ok(Edited {
            project,
            clips: targets,
        })
    }

    pub fn duplicate_clips(&self, clips: &[ClipKey]) -> Result<Edited, EditError> {
        let copies = clips
            .iter()
            .map(|&key| Ok((key, self.clip(key)?.clone())))
            .collect::<Result<Vec<_>, EditError>>()?;
        let (Some(first), Some(last)) = (
            clips.iter().map(|key| key.start).min(),
            copies
                .iter()
                .map(|(key, clip)| self.clip_end(key.start, clip))
                .max(),
        ) else {
            return Ok(self.unedited());
        };
        let mut edited = self.unedited();
        for (key, clip) in copies {
            let start = key.start + (last - first);
            edited.project = edited.project.place_clip(key.track, start, clip)?;
            edited.clips.push(ClipKey { start, ..key });
        }
        Ok(edited)
    }

    pub fn remove_clips(&self, clips: &[ClipKey]) -> Result<Self, EditError> {
        clips.iter().try_fold(self.clone(), |project, key| {
            project.remove_clip(key.track, key.start)
        })
    }

    fn unedited(&self) -> Edited {
        Edited {
            project: self.clone(),
            clips: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::project::{
        AudioSource, Clip, ClipKey, Edge, EditError, Project, Region, SourceId, TrackId,
    };
    use crate::time::{Clock, Duration, Musical, Position, Tempo, TempoMap};

    const FIRST: TrackId = TrackId::new(1);
    const SECOND: TrackId = TrackId::new(2);
    const TAPE: SourceId = SourceId::new(3);

    fn seconds(seconds: i64) -> Duration<Clock> {
        Duration::from_seconds(seconds)
    }

    fn at(quarter_notes: i64) -> Position<Musical> {
        Position::from_quarter_notes(quarter_notes)
    }

    fn quarter_notes(quarter_notes: i64) -> Duration<Musical> {
        Duration::from_quarter_notes(quarter_notes)
    }

    fn key(track: TrackId, quarter_notes: i64) -> ClipKey {
        ClipKey {
            track,
            start: at(quarter_notes),
        }
    }

    fn clip(offset: i64, length: i64) -> Clip {
        let region = Region {
            start: seconds(offset),
            length: seconds(length),
        };
        Clip::new("Tape", TAPE, region)
    }

    fn with(clips: &[(TrackId, i64, Clip)]) -> Project {
        let empty = Project::default()
            .with_tempo_map(TempoMap::new(Tempo::from_bpm(60.0).unwrap()))
            .add_source(AudioSource::new(TAPE, "tape.wav", seconds(100)))
            .and_then(|project| project.add_track(FIRST, "First"))
            .and_then(|project| project.add_track(SECOND, "Second"))
            .unwrap();
        clips.iter().fold(empty, |project, (track, start, clip)| {
            project
                .place_clip(*track, at(*start), clip.clone())
                .unwrap()
        })
    }

    fn lane(project: &Project, track: TrackId) -> Vec<(Position<Musical>, Clip)> {
        let track = project.track(track).unwrap();
        track
            .timeline()
            .map(|(start, clip)| (start, clip.clone()))
            .collect()
    }

    #[test]
    fn split_cuts_a_clip_in_two_at_the_position() {
        let project = with(&[(FIRST, 0, clip(0, 4))]);
        let edited = project.split_clips(&[key(FIRST, 0)], at(1)).unwrap();
        assert_eq!(
            lane(&edited.project, FIRST),
            [(at(0), clip(0, 1)), (at(1), clip(1, 3))]
        );
        assert_eq!(edited.clips, [key(FIRST, 0), key(FIRST, 1)]);
    }

    #[test]
    fn split_leaves_the_clips_that_the_position_does_not_cross() {
        let project = with(&[(FIRST, 0, clip(0, 2)), (FIRST, 4, clip(10, 2))]);
        let both = [key(FIRST, 0), key(FIRST, 4)];
        for position in [at(2), at(4)] {
            let edited = project.split_clips(&both, position).unwrap();
            assert_eq!(edited.project, project);
            assert_eq!(edited.clips, both);
        }
    }

    #[test]
    fn split_needs_clips_that_exist() {
        assert_eq!(
            with(&[]).split_clips(&[key(FIRST, 0)], at(1)),
            Err(EditError::NoClipAt(FIRST, at(0)))
        );
    }

    #[test]
    fn trimming_the_start_keeps_the_audio_at_its_place_in_time() {
        let project = with(&[(FIRST, 2, clip(2, 4))]);
        let edited = project
            .trim_clip(key(FIRST, 2), Edge::Start, at(3))
            .unwrap();
        assert_eq!(lane(&edited.project, FIRST), [(at(3), clip(3, 3))]);
        assert_eq!(edited.clips, [key(FIRST, 3)]);
    }

    #[test]
    fn the_start_cannot_move_before_the_start_of_the_audio() {
        let project = with(&[(FIRST, 4, clip(1, 2))]);
        let edited = project
            .trim_clip(key(FIRST, 4), Edge::Start, at(0))
            .unwrap();
        assert_eq!(lane(&edited.project, FIRST), [(at(3), clip(0, 3))]);
    }

    #[test]
    fn the_start_cannot_move_before_the_project_start() {
        let project = with(&[(FIRST, 1, clip(5, 2))]);
        let edited = project
            .trim_clip(key(FIRST, 1), Edge::Start, at(-10))
            .unwrap();
        assert_eq!(lane(&edited.project, FIRST), [(at(0), clip(4, 3))]);
    }

    #[test]
    fn the_start_cannot_pass_the_end() {
        let project = with(&[(FIRST, 0, clip(0, 2))]);
        let edited = project
            .trim_clip(key(FIRST, 0), Edge::Start, at(5))
            .unwrap();
        let (start, trimmed) = lane(&edited.project, FIRST).remove(0);
        assert_eq!(start, at(2) - Duration::from_ticks(1));
        assert_eq!(edited.project.clip_end(start, &trimmed), at(2));
    }

    #[test]
    fn trimming_the_end_changes_the_length() {
        let project = with(&[(FIRST, 0, clip(0, 4))]);
        let edited = project.trim_clip(key(FIRST, 0), Edge::End, at(2)).unwrap();
        assert_eq!(lane(&edited.project, FIRST), [(at(0), clip(0, 2))]);
        assert_eq!(edited.clips, [key(FIRST, 0)]);
    }

    #[test]
    fn the_end_cannot_move_after_the_end_of_the_audio() {
        let project = with(&[(FIRST, 0, clip(98, 1))]);
        let edited = project.trim_clip(key(FIRST, 0), Edge::End, at(10)).unwrap();
        assert_eq!(lane(&edited.project, FIRST), [(at(0), clip(98, 2))]);
    }

    #[test]
    fn the_end_cannot_pass_the_start() {
        let project = with(&[(FIRST, 4, clip(0, 2))]);
        let edited = project.trim_clip(key(FIRST, 4), Edge::End, at(1)).unwrap();
        let (start, trimmed) = lane(&edited.project, FIRST).remove(0);
        assert_eq!(start, at(4));
        assert_eq!(
            edited.project.clip_end(start, &trimmed),
            at(4) + Duration::from_ticks(1)
        );
    }

    #[test]
    fn a_longer_clip_replaces_the_part_of_the_next_clip_that_it_covers() {
        let project = with(&[(FIRST, 0, clip(0, 2)), (FIRST, 4, clip(10, 2))]);
        let edited = project.trim_clip(key(FIRST, 0), Edge::End, at(5)).unwrap();
        assert_eq!(
            lane(&edited.project, FIRST),
            [(at(0), clip(0, 5)), (at(5), clip(11, 1))]
        );
    }

    #[test]
    fn move_shifts_the_clips_in_time_and_keeps_their_distance() {
        let project = with(&[(FIRST, 0, clip(0, 1)), (FIRST, 4, clip(10, 1))]);
        let both = [key(FIRST, 0), key(FIRST, 4)];
        let edited = project.move_clips(&both, quarter_notes(2), 0).unwrap();
        assert_eq!(
            lane(&edited.project, FIRST),
            [(at(2), clip(0, 1)), (at(6), clip(10, 1))]
        );
        assert_eq!(edited.clips, [key(FIRST, 2), key(FIRST, 6)]);
    }

    #[test]
    fn move_takes_the_clips_to_another_track() {
        let project = with(&[(FIRST, 0, clip(0, 1))]);
        let edited = project
            .move_clips(&[key(FIRST, 0)], Duration::ZERO, 1)
            .unwrap();
        assert_eq!(lane(&edited.project, FIRST), []);
        assert_eq!(lane(&edited.project, SECOND), [(at(0), clip(0, 1))]);
        assert_eq!(edited.clips, [key(SECOND, 0)]);
    }

    #[test]
    fn the_clips_stay_between_the_first_and_the_last_track() {
        let project = with(&[(FIRST, 0, clip(0, 1)), (SECOND, 0, clip(10, 1))]);
        let both = [key(FIRST, 0), key(SECOND, 0)];
        for rows in [1, -1] {
            let edited = project.move_clips(&both, Duration::ZERO, rows).unwrap();
            assert_eq!(edited.clips, both);
        }
        let first = project
            .move_clips(&[key(FIRST, 0)], Duration::ZERO, 5)
            .unwrap();
        assert_eq!(first.clips, [key(SECOND, 0)]);
    }

    #[test]
    fn the_clips_cannot_move_before_the_project_start() {
        let project = with(&[(FIRST, 2, clip(0, 1)), (FIRST, 6, clip(10, 1))]);
        let both = [key(FIRST, 2), key(FIRST, 6)];
        let edited = project.move_clips(&both, quarter_notes(-5), 0).unwrap();
        assert_eq!(edited.clips, [key(FIRST, 0), key(FIRST, 4)]);
    }

    #[test]
    fn moved_clips_replace_the_parts_that_they_cover() {
        let project = with(&[(FIRST, 0, clip(0, 4)), (FIRST, 8, clip(10, 2))]);
        let edited = project
            .move_clips(&[key(FIRST, 8)], quarter_notes(-6), 0)
            .unwrap();
        assert_eq!(
            lane(&edited.project, FIRST),
            [(at(0), clip(0, 2)), (at(2), clip(10, 2))]
        );
    }

    #[test]
    fn move_needs_clips_that_exist() {
        assert_eq!(
            with(&[]).move_clips(&[key(FIRST, 0)], Duration::ZERO, 0),
            Err(EditError::NoClipAt(FIRST, at(0)))
        );
    }

    #[test]
    fn duplicate_puts_copies_right_after_the_selection() {
        let project = with(&[(FIRST, 0, clip(0, 1)), (FIRST, 2, clip(10, 1))]);
        let edited = project
            .duplicate_clips(&[key(FIRST, 0), key(FIRST, 2)])
            .unwrap();
        assert_eq!(
            lane(&edited.project, FIRST),
            [
                (at(0), clip(0, 1)),
                (at(2), clip(10, 1)),
                (at(3), clip(0, 1)),
                (at(5), clip(10, 1))
            ]
        );
        assert_eq!(edited.clips, [key(FIRST, 3), key(FIRST, 5)]);
    }

    #[test]
    fn duplicate_keeps_each_copy_on_the_track_of_its_clip() {
        let project = with(&[(FIRST, 0, clip(0, 2)), (SECOND, 1, clip(10, 2))]);
        let edited = project
            .duplicate_clips(&[key(FIRST, 0), key(SECOND, 1)])
            .unwrap();
        assert_eq!(edited.clips, [key(FIRST, 3), key(SECOND, 4)]);
    }

    #[test]
    fn remove_clips_takes_out_each_clip() {
        let project = with(&[(FIRST, 0, clip(0, 1)), (SECOND, 2, clip(10, 1))]);
        let removed = project
            .remove_clips(&[key(FIRST, 0), key(SECOND, 2)])
            .unwrap();
        assert_eq!(lane(&removed, FIRST), []);
        assert_eq!(lane(&removed, SECOND), []);
    }

    #[test]
    fn a_clip_is_found_by_its_track_and_its_start() {
        let project = with(&[(FIRST, 2, clip(0, 1))]);
        assert_eq!(project.clip(key(FIRST, 2)), Ok(&clip(0, 1)));
        assert_eq!(
            project.clip(key(FIRST, 1)),
            Err(EditError::NoClipAt(FIRST, at(1)))
        );
    }
}
