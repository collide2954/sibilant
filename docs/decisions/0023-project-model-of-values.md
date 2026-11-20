# 0023. Model the project as values with clips at musical positions

- Status: Accepted
- Date: 2026-11-20

## Context

Records 0001, 0002, and 0006 set the scope and the form of the project model. The model must also define the tracks, the clips, the scenes, and how the code finds each part.

## Decision

- A project contains the tempo map, the time signature map, the audio sources, the scenes, a flat list of audio tracks, and the gain of the master track.
- Version 0.1 has no group tracks. Group tracks come later with a file migration.
- A track has a name, a mix (gain, pan, mute, and solo), a timeline lane, and one launcher slot for each scene.
- A clip refers to a region of an audio source. The region has a start in the source and a length. These two values are clock durations.
- A clip on the timeline starts at a musical position. When the tempo changes, the clip stays at its bar and beat. Its audio plays at its recorded speed, so its end moves.
- On the timeline, a clip ends at the first tick after its audio. Thus each tick of a clip contains a part of its audio.
- Clips on a timeline lane do not overlap. A new clip replaces the part of each clip that it covers. If a tempo change moves the end of a clip after the start of the next clip, the next clip cuts it off.
- Each place has its own copy of a clip. There are no linked clips.
- Tracks, scenes, and audio sources have typed IDs. The caller supplies each new ID from a counter that does not decrease. Thus an undo never causes the use of an ID two times.
- A clip has no ID. The code finds a clip on the timeline by its track and its start. The code finds a clip in the launcher by its track and its scene.
- An edit is a method that receives the project and returns a new project or an error. The project uses `Arc` to share the parts that do not change.
- The undo history is a generic list of snapshots. Undo includes all changes to the project, and thus all changes to the mix. Undo does not include the selection, the view, or the transport.

## Consequences

- An edit cannot give a part an ID that another part uses. The ID of a part never changes.
- A tempo change does not change the clips. Only their ends move.
- When a clip moves on the timeline, the code must find it at its new start.
- A copy of a clip uses almost no memory until one of the copies changes.

## Alternatives

- Clips that keep their time in seconds: good for film and podcasts, but the bars move under the clips.
- A time base for each track: more control, but more interface and tests in version 0.1.
- Clips that cover other clips: nothing is lost, but parts of clips can be hidden.
- Linked clips: one edit changes all places, but the model needs a clip pool and an unlink action.
- Group tracks in version 0.1: more work now in the model, the mixer, and the engine.
- IDs from a counter in the project: simpler calls, but an undo can cause the use of an ID two times.
- Persistent collections such as `im`: more sharing, but their license is not permissive (record 0019).
