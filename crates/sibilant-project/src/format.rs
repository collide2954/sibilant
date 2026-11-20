use std::error::Error;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sibilant_core::mix::{Gain, Mix, Pan};
use sibilant_core::project::{
    AudioSource, Clip, Project, Region, SceneId, SourceId, Track, TrackColor, TrackId,
};
use sibilant_core::time::{Duration, Position, Tempo, TempoMap, TimeSignature, TimeSignatureMap};

use crate::OpenError;

const VERSION: u32 = 1;

#[derive(Deserialize)]
struct Header {
    format: u32,
}

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    format: u32,
    tempo: Vec<TempoChange>,
    time_signatures: Vec<TimeSignatureChange>,
    #[serde(with = "decibels")]
    master_gain_db: f64,
    sources: Vec<SourceEntry>,
    scenes: Vec<SceneEntry>,
    tracks: Vec<TrackEntry>,
}

#[derive(Serialize, Deserialize)]
struct TempoChange {
    tick: i64,
    bpm: f64,
}

#[derive(Serialize, Deserialize)]
struct TimeSignatureChange {
    bar: i64,
    numerator: u8,
    denominator: u8,
}

#[derive(Serialize, Deserialize)]
struct SourceEntry {
    id: u64,
    file: String,
    length: i64,
}

#[derive(Serialize, Deserialize)]
struct SceneEntry {
    id: u64,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct TrackEntry {
    id: u64,
    name: String,
    color: u8,
    #[serde(with = "decibels")]
    gain_db: f64,
    pan: f64,
    mute: bool,
    solo: bool,
    timeline: Vec<TimelineClip>,
    slots: Vec<SlotClip>,
}

#[derive(Serialize, Deserialize)]
struct TimelineClip {
    tick: i64,
    clip: ClipEntry,
}

#[derive(Serialize, Deserialize)]
struct SlotClip {
    scene: u64,
    clip: ClipEntry,
}

#[derive(Serialize, Deserialize)]
struct ClipEntry {
    name: String,
    source: u64,
    offset: i64,
    length: i64,
}

struct Invalid(String);

impl<E: Error> From<E> for Invalid {
    fn from(error: E) -> Self {
        Self(error.to_string())
    }
}

pub(crate) fn read(
    text: &str,
    resolve: impl Fn(&str) -> Option<PathBuf>,
) -> Result<Project, OpenError> {
    let header: Header =
        serde_json::from_str(text).map_err(|error| OpenError::NotProject(error.to_string()))?;
    match header.format {
        VERSION => {
            let file: ProjectFile = serde_json::from_str(text)
                .map_err(|error| OpenError::Content(error.to_string()))?;
            file.into_project(resolve)
                .map_err(|Invalid(error)| OpenError::Content(error))
        }
        newer if newer > VERSION => Err(OpenError::Newer(newer)),
        unknown => Err(OpenError::NotProject(format!(
            "format {unknown} does not exist"
        ))),
    }
}

impl ProjectFile {
    fn into_project(self, resolve: impl Fn(&str) -> Option<PathBuf>) -> Result<Project, Invalid> {
        let project = Project::default()
            .with_tempo_map(tempo_map(&self.tempo)?)
            .with_time_signature_map(time_signature_map(&self.time_signatures)?)
            .with_master_gain(Gain::from_db(self.master_gain_db)?);
        let project = self
            .sources
            .into_iter()
            .try_fold(project, |project, source| {
                let path = resolve(&source.file).ok_or_else(|| {
                    Invalid(format!(
                        "the audio file {} is not in the project folder",
                        source.file
                    ))
                })?;
                let length = Duration::from_superclocks(source.length);
                Ok::<_, Invalid>(project.add_source(AudioSource::new(
                    SourceId::new(source.id),
                    path,
                    length,
                ))?)
            })?;
        let project = self
            .scenes
            .into_iter()
            .try_fold(project, |project, scene| {
                project.add_scene(SceneId::new(scene.id), scene.name)
            })?;
        self.tracks
            .into_iter()
            .try_fold(project, |project, track| track.restore(&project))
    }
}

fn tempo_map(changes: &[TempoChange]) -> Result<TempoMap, Invalid> {
    let first = changes
        .first()
        .ok_or_else(|| Invalid("the project file has no tempo".into()))?;
    changes
        .iter()
        .try_fold(TempoMap::new(Tempo::from_bpm(first.bpm)?), |map, change| {
            let tempo = Tempo::from_bpm(change.bpm)?;
            Ok(map.with_tempo(Position::from_ticks(change.tick), tempo))
        })
}

fn time_signature_map(changes: &[TimeSignatureChange]) -> Result<TimeSignatureMap, Invalid> {
    let first = changes
        .first()
        .ok_or_else(|| Invalid("the project file has no time signature".into()))?;
    changes
        .iter()
        .try_fold(TimeSignatureMap::new(first.signature()?), |map, change| {
            Ok(map.with_time_signature(change.bar, change.signature()?))
        })
}

impl TimeSignatureChange {
    fn signature(&self) -> Result<TimeSignature, Invalid> {
        Ok(TimeSignature::new(self.numerator, self.denominator)?)
    }
}

impl TrackEntry {
    fn restore(self, project: &Project) -> Result<Project, Invalid> {
        let id = TrackId::new(self.id);
        let mix = Mix {
            gain: Gain::from_db(self.gain_db)?,
            pan: Pan::new(self.pan)?,
            mute: self.mute,
            solo: self.solo,
        };
        let project = project
            .add_track(id, self.name)?
            .set_track_color(id, TrackColor::new(self.color)?)?
            .set_mix(id, mix)?;
        let project = self
            .timeline
            .into_iter()
            .try_fold(project, |project, placed| {
                let start = Position::from_ticks(placed.tick);
                project.restore_clip(id, start, placed.clip.into_clip())
            })?;
        Ok(self.slots.into_iter().try_fold(project, |project, slot| {
            project.put_clip_in_slot(id, SceneId::new(slot.scene), slot.clip.into_clip())
        })?)
    }
}

impl ClipEntry {
    fn into_clip(self) -> Clip {
        let region = Region {
            start: Duration::from_superclocks(self.offset),
            length: Duration::from_superclocks(self.length),
        };
        Clip::new(self.name, SourceId::new(self.source), region)
    }
}

pub(crate) fn write(project: &Project, mut file_of: impl FnMut(&AudioSource) -> String) -> String {
    let file = ProjectFile {
        format: VERSION,
        tempo: project
            .tempo_map()
            .changes()
            .map(|(start, tempo)| TempoChange {
                tick: start.ticks(),
                bpm: tempo.bpm(),
            })
            .collect(),
        time_signatures: project
            .time_signature_map()
            .changes()
            .map(|(bar, signature)| TimeSignatureChange {
                bar,
                numerator: signature.numerator(),
                denominator: signature.denominator(),
            })
            .collect(),
        master_gain_db: project.master_gain().db(),
        sources: project
            .sources()
            .map(|source| SourceEntry {
                id: source.id().value(),
                file: file_of(source),
                length: source.length().superclocks(),
            })
            .collect(),
        scenes: project
            .scenes()
            .map(|scene| SceneEntry {
                id: scene.id().value(),
                name: scene.name().to_owned(),
            })
            .collect(),
        tracks: project.tracks().map(TrackEntry::of).collect(),
    };
    serde_json::to_string_pretty(&file).expect("a project file contains only JSON values")
}

impl TrackEntry {
    fn of(track: &Track) -> Self {
        let mix = track.mix();
        Self {
            id: track.id().value(),
            name: track.name().to_owned(),
            color: track.color().index(),
            gain_db: mix.gain.db(),
            pan: mix.pan.position(),
            mute: mix.mute,
            solo: mix.solo,
            timeline: track
                .timeline()
                .map(|(start, clip)| TimelineClip {
                    tick: start.ticks(),
                    clip: ClipEntry::of(clip),
                })
                .collect(),
            slots: track
                .slots()
                .map(|(scene, clip)| SlotClip {
                    scene: scene.value(),
                    clip: ClipEntry::of(clip),
                })
                .collect(),
        }
    }
}

impl ClipEntry {
    fn of(clip: &Clip) -> Self {
        let region = clip.region();
        Self {
            name: clip.name().to_owned(),
            source: clip.source().value(),
            offset: region.start.superclocks(),
            length: region.length.superclocks(),
        }
    }
}

mod decibels {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    const SILENCE: &str = "-inf";

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Stored {
        Number(f64),
        Text(String),
    }

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde gives each field by reference"
    )]
    pub(super) fn serialize<S: Serializer>(db: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        if db.is_infinite() && db.is_sign_negative() {
            serializer.serialize_str(SILENCE)
        } else {
            serializer.serialize_f64(*db)
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        match Stored::deserialize(deserializer)? {
            Stored::Number(db) => Ok(db),
            Stored::Text(text) if text == SILENCE => Ok(f64::NEG_INFINITY),
            Stored::Text(text) => Err(D::Error::custom(format!("{text} is not a gain in dB"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use proptest::prelude::*;
    use serde_json::{Value, json};
    use sibilant_core::mix::{Gain, Mix, Pan};
    use sibilant_core::project::{AudioSource, Clip, Project, Region, SceneId, SourceId, TrackId};
    use sibilant_core::time::{
        Clock, Duration, Position, Tempo, TempoMap, TimeSignature, TimeSignatureMap,
    };

    use crate::{OpenError, format};

    const KICK: SourceId = SourceId::new(1);
    const INTRO: SceneId = SceneId::new(2);
    const DRUMS: TrackId = TrackId::new(3);

    fn kick(length: Duration<Clock>) -> Clip {
        let region = Region {
            start: Duration::ZERO,
            length,
        };
        Clip::new("Kick", KICK, region)
    }

    fn project() -> Project {
        let tempo = TempoMap::new(Tempo::default()).with_tempo(
            Position::from_quarter_notes(8),
            Tempo::from_bpm(90.0).unwrap(),
        );
        let time_signatures = TimeSignatureMap::new(TimeSignature::default())
            .with_time_signature(3, TimeSignature::new(3, 4).unwrap());
        let mix = Mix {
            gain: Gain::SILENT,
            pan: Pan::new(0.5).unwrap(),
            mute: true,
            solo: false,
        };
        Project::default()
            .with_tempo_map(tempo)
            .with_time_signature_map(time_signatures)
            .with_master_gain(Gain::from_db(-3.0).unwrap())
            .add_source(AudioSource::new(
                KICK,
                "Kick.wav",
                Duration::from_seconds(1),
            ))
            .and_then(|project| project.add_scene(INTRO, "Intro"))
            .and_then(|project| project.add_track(DRUMS, "Drums"))
            .and_then(|project| project.set_mix(DRUMS, mix))
            .and_then(|project| {
                project.place_clip(
                    DRUMS,
                    Position::from_quarter_notes(1),
                    kick(Duration::from_seconds(1)),
                )
            })
            .and_then(|project| {
                project.put_clip_in_slot(
                    DRUMS,
                    INTRO,
                    kick(Duration::from_superclocks(141_120_000)),
                )
            })
            .unwrap()
    }

    #[test]
    fn the_project_file_has_the_format_of_version_1() {
        let text = format::write(&project(), |_| "audio/Kick.wav".into());
        let clip =
            |length: i64| json!({ "name": "Kick", "source": 1, "offset": 0, "length": length });
        let expected = json!({
            "format": 1,
            "tempo": [{ "tick": 0, "bpm": 120.0 }, { "tick": 5_644_800, "bpm": 90.0 }],
            "time_signatures": [
                { "bar": 1, "numerator": 4, "denominator": 4 },
                { "bar": 3, "numerator": 3, "denominator": 4 }
            ],
            "master_gain_db": -3.0,
            "sources": [{ "id": 1, "file": "audio/Kick.wav", "length": 282_240_000 }],
            "scenes": [{ "id": 2, "name": "Intro" }],
            "tracks": [{
                "id": 3,
                "name": "Drums",
                "color": 0,
                "gain_db": "-inf",
                "pan": 0.5,
                "mute": true,
                "solo": false,
                "timeline": [{ "tick": 705_600, "clip": clip(282_240_000) }],
                "slots": [{ "scene": 2, "clip": clip(141_120_000) }]
            }]
        });
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), expected);
    }

    #[test]
    fn the_project_file_is_indented_text() {
        let text = format::write(&Project::default(), |_| String::new());
        assert!(text.starts_with("{\n  \"format\": 1,"), "{text}");
    }

    fn read_text(text: &str) -> Result<Project, OpenError> {
        format::read(text, |file| Some(PathBuf::from(file)))
    }

    fn read_back(project: &Project) -> Result<Project, OpenError> {
        let text = format::write(project, |source| {
            source.path().to_string_lossy().into_owned()
        });
        read_text(&text)
    }

    #[test]
    fn a_project_survives_the_project_file() {
        let project = project();
        assert_eq!(read_back(&project), Ok(project));
    }

    #[test]
    fn clips_that_overlap_after_a_tempo_change_stay_as_they_are() {
        let two_seconds = || kick(Duration::from_seconds(2));
        let project = Project::default()
            .with_tempo_map(TempoMap::new(Tempo::from_bpm(60.0).unwrap()))
            .add_source(AudioSource::new(
                KICK,
                "Kick.wav",
                Duration::from_seconds(4),
            ))
            .and_then(|project| project.add_track(DRUMS, "Drums"))
            .and_then(|project| project.place_clip(DRUMS, Position::START, two_seconds()))
            .and_then(|project| {
                project.place_clip(DRUMS, Position::from_quarter_notes(2), two_seconds())
            })
            .unwrap()
            .with_tempo_map(TempoMap::default());
        assert_eq!(read_back(&project), Ok(project));
    }

    proptest! {
        #[test]
        fn tempos_gains_and_pans_survive_the_project_file(
            bpm in 1.0..=1000.0_f64,
            db in -144.0..=6.0_f64,
            pan in -1.0..=1.0_f64,
        ) {
            let gain = Gain::from_db(db).unwrap();
            let mix = Mix { gain, pan: Pan::new(pan).unwrap(), ..Mix::default() };
            let project = Project::default()
                .with_tempo_map(TempoMap::new(Tempo::from_bpm(bpm).unwrap()))
                .with_master_gain(gain)
                .add_track(DRUMS, "Drums")
                .and_then(|project| project.set_mix(DRUMS, mix))
                .unwrap();
            prop_assert_eq!(read_back(&project), Ok(project));
        }
    }

    #[test]
    fn a_newer_format_needs_a_newer_sibilant() {
        assert_eq!(read_text(r#"{ "format": 2 }"#), Err(OpenError::Newer(2)));
    }

    #[test]
    fn text_without_a_known_format_is_not_a_project() {
        for text in [
            "[1, 2]",
            r#"{ "name": "MySong" }"#,
            r#"{ "format": 0 }"#,
            "{",
        ] {
            assert!(
                matches!(read_text(text), Err(OpenError::NotProject(_))),
                "{text}"
            );
        }
    }

    #[test]
    fn a_project_file_without_a_part_tells_which_part() {
        let result = read_text(r#"{ "format": 1 }"#);
        assert!(
            matches!(&result, Err(OpenError::Content(message)) if message.contains("tempo")),
            "{result:?}"
        );
    }

    fn changed(change: impl FnOnce(&mut Value)) -> String {
        let text = format::write(&Project::default(), |_| String::new());
        let mut file: Value = serde_json::from_str(&text).unwrap();
        change(&mut file);
        file.to_string()
    }

    #[test]
    fn a_value_outside_its_range_tells_what_is_wrong() {
        let text = changed(|file| file["tempo"][0]["bpm"] = json!(0.5));
        assert_eq!(
            read_text(&text),
            Err(OpenError::Content(
                "tempo 0.5 BPM is not in the range 1 to 1000 BPM".into()
            ))
        );
    }

    #[test]
    fn a_project_file_needs_a_tempo_and_a_time_signature() {
        let without_tempo = changed(|file| file["tempo"] = json!([]));
        let without_time_signature = changed(|file| file["time_signatures"] = json!([]));
        assert_eq!(
            read_text(&without_tempo),
            Err(OpenError::Content("the project file has no tempo".into()))
        );
        assert_eq!(
            read_text(&without_time_signature),
            Err(OpenError::Content(
                "the project file has no time signature".into()
            ))
        );
    }

    #[test]
    fn a_gain_text_other_than_minus_inf_is_wrong() {
        let text = changed(|file| file["master_gain_db"] = json!("loud"));
        assert!(
            matches!(read_text(&text), Err(OpenError::Content(message)) if message.contains("loud")),
        );
    }

    #[test]
    fn an_audio_file_outside_the_project_folder_is_refused() {
        let text = format::write(&project(), |_| "../Kick.wav".into());
        assert_eq!(
            format::read(&text, |_| None),
            Err(OpenError::Content(
                "the audio file ../Kick.wav is not in the project folder".into()
            ))
        );
    }
}
