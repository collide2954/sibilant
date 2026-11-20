use serde::{Serialize, Serializer};
use sibilant_core::project::{AudioSource, Clip, Project, Track};

const VERSION: u32 = 1;
const SILENCE: &str = "-inf";

#[derive(Serialize)]
struct ProjectFile {
    format: u32,
    tempo: Vec<TempoChange>,
    time_signatures: Vec<TimeSignatureChange>,
    #[serde(serialize_with = "decibels")]
    master_gain_db: f64,
    sources: Vec<SourceEntry>,
    scenes: Vec<SceneEntry>,
    tracks: Vec<TrackEntry>,
}

#[derive(Serialize)]
struct TempoChange {
    tick: i64,
    bpm: f64,
}

#[derive(Serialize)]
struct TimeSignatureChange {
    bar: i64,
    numerator: u8,
    denominator: u8,
}

#[derive(Serialize)]
struct SourceEntry {
    id: u64,
    file: String,
    length: i64,
}

#[derive(Serialize)]
struct SceneEntry {
    id: u64,
    name: String,
}

#[derive(Serialize)]
struct TrackEntry {
    id: u64,
    name: String,
    color: u8,
    #[serde(serialize_with = "decibels")]
    gain_db: f64,
    pan: f64,
    mute: bool,
    solo: bool,
    timeline: Vec<TimelineClip>,
    slots: Vec<SlotClip>,
}

#[derive(Serialize)]
struct TimelineClip {
    tick: i64,
    clip: ClipEntry,
}

#[derive(Serialize)]
struct SlotClip {
    scene: u64,
    clip: ClipEntry,
}

#[derive(Serialize)]
struct ClipEntry {
    name: String,
    source: u64,
    offset: i64,
    length: i64,
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

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde gives each field by reference"
)]
fn decibels<S: Serializer>(db: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    if db.is_infinite() && db.is_sign_negative() {
        serializer.serialize_str(SILENCE)
    } else {
        serializer.serialize_f64(*db)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use sibilant_core::mix::{Gain, Mix, Pan};
    use sibilant_core::project::{AudioSource, Clip, Project, Region, SceneId, SourceId, TrackId};
    use sibilant_core::time::{
        Clock, Duration, Position, Tempo, TempoMap, TimeSignature, TimeSignatureMap,
    };

    use crate::format;

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
}
