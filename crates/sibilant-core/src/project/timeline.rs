use std::collections::BTreeMap;
use std::sync::Arc;

use super::{Clip, EditError, Project, Track, TrackId};
use crate::time::{Duration, Musical, Position, TempoMap};

pub(super) type Lane = BTreeMap<Position<Musical>, Arc<Clip>>;

impl Track {
    pub fn timeline(&self) -> impl ExactSizeIterator<Item = (Position<Musical>, &Clip)> {
        self.timeline
            .iter()
            .map(|(&start, clip)| (start, clip.as_ref()))
    }
}

impl Project {
    pub fn clip_end(&self, start: Position<Musical>, clip: &Clip) -> Position<Musical> {
        end_of(&self.tempo_map, start, clip)
    }

    pub fn place_clip(
        &self,
        track: TrackId,
        start: Position<Musical>,
        clip: Clip,
    ) -> Result<Self, EditError> {
        self.check_clip(&clip)?;
        self.edit_track(track, |track| {
            place(&mut track.timeline, &self.tempo_map, start, clip);
        })
    }
}

fn place(lane: &mut Lane, tempo: &TempoMap, start: Position<Musical>, clip: Clip) {
    let end = end_of(tempo, start, &clip);
    let remainder = remainder_after(lane, tempo, end);
    lane.retain(|&other, _| other < start || end <= other);
    if let Some((before, trimmed)) = trimmed_before(lane, tempo, start) {
        lane.insert(before, Arc::new(trimmed));
    }
    lane.insert(start, Arc::new(clip));
    if let Some(remainder) = remainder {
        lane.insert(end, Arc::new(remainder));
    }
}

fn trimmed_before(
    lane: &Lane,
    tempo: &TempoMap,
    start: Position<Musical>,
) -> Option<(Position<Musical>, Clip)> {
    let (&before, clip) = lane.range(..start).next_back()?;
    let kept = tempo.to_clock(start) - tempo.to_clock(before);
    (end_of(tempo, before, clip) > start).then(|| (before, clip.part_before(kept)))
}

fn remainder_after(lane: &Lane, tempo: &TempoMap, end: Position<Musical>) -> Option<Clip> {
    if lane.contains_key(&end) {
        return None;
    }
    let (&start, clip) = lane.range(..end).next_back()?;
    let cut = tempo.to_clock(end) - tempo.to_clock(start);
    (end_of(tempo, start, clip) > end).then(|| clip.part_after(cut))
}

fn end_of(tempo: &TempoMap, start: Position<Musical>, clip: &Clip) -> Position<Musical> {
    let audio_end = tempo.to_clock(start) + clip.region().length;
    let nearest = tempo.to_musical(audio_end);
    if tempo.to_clock(nearest) < audio_end {
        nearest + Duration::from_ticks(1)
    } else {
        nearest
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::project::{AudioSource, Clip, EditError, Project, Region, SourceId, TrackId};
    use crate::time::{Clock, Duration, Musical, Position, Tempo, TempoMap};

    const TRACK: TrackId = TrackId::new(1);
    const TAPE: SourceId = SourceId::new(1);
    const MILLISECOND: i64 = 282_240;

    fn seconds(seconds: i64) -> Duration<Clock> {
        Duration::from_seconds(seconds)
    }

    fn quarter_notes(quarter_notes: i64) -> Position<Musical> {
        Position::from_quarter_notes(quarter_notes)
    }

    fn tempo(bpm: f64) -> TempoMap {
        TempoMap::new(Tempo::from_bpm(bpm).unwrap())
    }

    fn project() -> Project {
        Project::default()
            .with_tempo_map(tempo(60.0))
            .add_source(AudioSource::new(TAPE, "tape.wav", seconds(100)))
            .and_then(|project| project.add_track(TRACK, "Tape"))
            .unwrap()
    }

    fn tape(start: Duration<Clock>, length: Duration<Clock>) -> Clip {
        Clip::new("Tape", TAPE, Region { start, length })
    }

    fn clip(start: i64, length: i64) -> Clip {
        tape(seconds(start), seconds(length))
    }

    fn timeline(project: &Project) -> Vec<(Position<Musical>, Clip)> {
        let track = project.track(TRACK).unwrap();
        track
            .timeline()
            .map(|(start, clip)| (start, clip.clone()))
            .collect()
    }

    fn placed(clips: &[(i64, Clip)]) -> Project {
        clips.iter().fold(project(), |project, (start, clip)| {
            project
                .place_clip(TRACK, quarter_notes(*start), clip.clone())
                .unwrap()
        })
    }

    #[test]
    fn a_clip_on_an_empty_lane_starts_at_its_position() {
        let project = placed(&[(4, clip(0, 2))]);
        assert_eq!(timeline(&project), [(quarter_notes(4), clip(0, 2))]);
    }

    #[test]
    fn clips_that_do_not_overlap_stay_as_they_are() {
        let project = placed(&[(0, clip(0, 2)), (2, clip(10, 2))]);
        assert_eq!(
            timeline(&project),
            [
                (quarter_notes(0), clip(0, 2)),
                (quarter_notes(2), clip(10, 2))
            ]
        );
    }

    #[test]
    fn a_new_clip_replaces_the_end_of_the_clip_before_it() {
        let project = placed(&[(0, clip(0, 4)), (2, clip(20, 4))]);
        assert_eq!(
            timeline(&project),
            [
                (quarter_notes(0), clip(0, 2)),
                (quarter_notes(2), clip(20, 4))
            ]
        );
    }

    #[test]
    fn a_new_clip_replaces_the_start_of_the_clip_after_it() {
        let project = placed(&[(4, clip(0, 4)), (2, clip(20, 4))]);
        assert_eq!(
            timeline(&project),
            [
                (quarter_notes(2), clip(20, 4)),
                (quarter_notes(6), clip(2, 2))
            ]
        );
    }

    #[test]
    fn a_new_clip_inside_a_longer_clip_splits_it() {
        let project = placed(&[(0, clip(0, 8)), (2, clip(20, 2))]);
        assert_eq!(
            timeline(&project),
            [
                (quarter_notes(0), clip(0, 2)),
                (quarter_notes(2), clip(20, 2)),
                (quarter_notes(4), clip(4, 4)),
            ]
        );
    }

    #[test]
    fn a_new_clip_removes_the_clips_that_it_covers() {
        let project = placed(&[(2, clip(0, 1)), (4, clip(10, 1)), (0, clip(20, 8))]);
        assert_eq!(timeline(&project), [(quarter_notes(0), clip(20, 8))]);
    }

    #[test]
    fn a_new_clip_at_the_start_of_an_old_clip_replaces_its_first_part() {
        let project = placed(&[(2, clip(0, 4)), (2, clip(20, 2))]);
        assert_eq!(
            timeline(&project),
            [
                (quarter_notes(2), clip(20, 2)),
                (quarter_notes(4), clip(2, 2))
            ]
        );
    }

    #[test]
    fn a_tempo_change_moves_only_the_end_of_a_clip() {
        let project = placed(&[(4, clip(0, 4))]).with_tempo_map(tempo(120.0));
        let (start, clip) = timeline(&project).remove(0);
        assert_eq!(start, quarter_notes(4));
        assert_eq!(project.clip_end(start, &clip), quarter_notes(12));
    }

    #[test]
    fn a_clip_ends_at_the_first_tick_after_its_audio() {
        let short = tape(Duration::ZERO, Duration::from_superclocks(100));
        let longer = tape(Duration::ZERO, Duration::from_superclocks(401));
        let start = quarter_notes(0);
        let project = project();
        assert_eq!(project.clip_end(start, &short), Position::from_ticks(1));
        assert_eq!(project.clip_end(start, &longer), Position::from_ticks(2));
    }

    #[test]
    fn a_clip_on_the_timeline_needs_a_known_track_and_a_valid_region() {
        let unknown = TrackId::new(9);
        assert_eq!(
            project().place_clip(unknown, quarter_notes(0), clip(0, 2)),
            Err(EditError::UnknownTrack(unknown))
        );
        assert_eq!(
            project().place_clip(TRACK, quarter_notes(0), clip(99, 2)),
            Err(EditError::RegionOutsideSource(TAPE))
        );
    }

    fn heard_at(project: &Project, position: Position<Musical>) -> Option<Duration<Clock>> {
        let tempo = project.tempo_map();
        let (start, clip) = project
            .track(TRACK)
            .unwrap()
            .timeline()
            .take_while(|&(start, _)| start <= position)
            .last()?;
        let into = tempo.to_clock(position) - tempo.to_clock(start);
        (into < clip.region().length).then(|| clip.region().start + into)
    }

    fn tempo_maps() -> impl Strategy<Value = TempoMap> {
        (
            30.0..=300.0,
            prop::collection::vec((1..1_i64 << 26, 30.0..=300.0), 0..3),
        )
            .prop_map(|(first, changes)| {
                changes.into_iter().fold(tempo(first), |map, (ticks, bpm)| {
                    map.with_tempo(Position::from_ticks(ticks), Tempo::from_bpm(bpm).unwrap())
                })
            })
    }

    fn placements() -> impl Strategy<Value = (i64, Clip)> {
        let half = 50_000 * MILLISECOND;
        (0..1_i64 << 26, 0..half, MILLISECOND..half).prop_map(|(start, offset, length)| {
            let clip = tape(
                Duration::from_superclocks(offset),
                Duration::from_superclocks(length),
            );
            (start, clip)
        })
    }

    proptest! {
        #[test]
        fn a_new_clip_replaces_exactly_the_part_of_the_lane_that_it_covers(
            tempo_map in tempo_maps(),
            existing in prop::collection::vec(placements(), 0..6),
            (start, new) in placements(),
            probes in prop::collection::vec(-(1_i64 << 20)..1_i64 << 28, 32),
        ) {
            let before = existing.iter().fold(project().with_tempo_map(tempo_map), |project, (start, clip)| {
                project.place_clip(TRACK, Position::from_ticks(*start), clip.clone()).unwrap()
            });
            let start = Position::from_ticks(start);
            let after = before.place_clip(TRACK, start, new.clone()).unwrap();
            let end = after.clip_end(start, &new);
            let tempo = after.tempo_map();
            let one = Duration::from_ticks(1);
            let edges = [start - one, start, end - one, end];
            for position in probes.into_iter().map(Position::from_ticks).chain(edges) {
                let expected = if start <= position && position < end {
                    Some(new.region().start + (tempo.to_clock(position) - tempo.to_clock(start)))
                } else {
                    heard_at(&before, position)
                };
                prop_assert_eq!(heard_at(&after, position), expected, "at {:?}", position);
            }
        }
    }
}
