# 0043. Start and stop launcher clips at quantized times

- Status: Accepted
- Date: 2026-11-22

## Context

Version 0.1 must start, loop, and stop clips in the clip launcher at quantized times (record 0002). Each track row shows one launcher slot for each scene (records 0014 and 0023), but the user cannot put a clip in a slot or start it. Each track plays its launcher clip or its timeline (record 0001). In version 0.1, Sibilant does not change the speed of clips to match the tempo.

## Decision

- Quantization:
  - A setting in the transport bar sets the time of each launch and each stop: None, 1 beat, 1 bar, 2 bars, or 4 bars.
  - The setting of a new project is 1 bar. The project file keeps the setting. A change of the setting is one step in the undo history.
  - Sibilant counts the quantized times from bar 1. Thus, with 1 bar, a clip starts at the next bar line. With None, a clip starts or stops at once.
- Slots:
  - A full slot shows the name of its clip and a play button.
  - A click on the play button starts the clip at the next quantized time. A click on the play button of a clip that plays stops it at the next quantized time. A click on the play button of a clip that waits to start cancels the start.
  - A play button flashes while its clip waits to start or to stop. The play button of a clip that plays has the accent color.
  - A click on a different part of a full slot selects that clip. Ctrl+click adds the clip to the selection or removes it. On macOS, Cmd+click does this. The selection contains timeline clips or slot clips, not the two together.
  - The Enter key starts the selected slot clips. The Delete and Backspace keys remove them.
- Tracks:
  - A track plays one launcher clip at a time. When the user starts a different clip on the same track, it replaces the clip that plays at the quantized time.
  - While a launcher clip plays on a track, the timeline of that track is silent. When the launcher clip stops, the track plays its timeline again from the playhead.
  - The mix of the track (gain, pan, mute, and solo) and its meter include the launcher clip.
- Loops:
  - A launcher clip plays its region again and again with no gap. Each start begins at the start of the region.
  - The clip fades in for 4 ms when it starts and fades out for 4 ms when it stops (record 0036). The loop point has no fade.
  - A loop stays on the bars only when the tempo of its audio matches the tempo of the project (record 0002).
- Scenes:
  - Each scene in the scene row has a launch button. A click starts all clips of that scene at the next quantized time. At the same time, the tracks with an empty slot in that scene stop.
  - A Stop all button at the start of the scene row stops all launcher clips at the next quantized time.
  - This slice keeps the four scenes of a new project. Later versions let the user add, rename, and delete scenes.
- Transport:
  - When the transport is stopped, a launch starts playback at the play start. The clip starts at once.
  - Space stops playback and all launcher clips at once. At the next start, all tracks play their timelines.
  - The export contains only the timeline (record 0042). The launcher clips are not in the file.
- Clips in the slots:
  - The user can drag an audio file from the browser onto a slot. Sibilant loads the file (record 0035) and puts a clip with all of its audio in the slot.
  - The user can drag a timeline clip onto a slot. The slot gets a copy of the clip, and the timeline keeps the clip. Only the clip under the pointer goes into the slot.
  - The user can drag a slot clip onto a timeline lane. The lane gets a copy of the clip at the drop position, with the same snap as a move (record 0040). The slot keeps the clip.
  - The user can drag a slot clip onto a different slot. The clip moves. If the target slot is full, the moved clip replaces its clip.
  - During a drag, the slot under the pointer has an outline.
  - Each change to a slot is one step in the undo history. A launch is not part of the undo history.
  - When an edit, an undo, or a redo removes or replaces a clip that plays, that clip stops at once.
- The project file keeps the clips in the slots. It does not keep the clips that play.

## Consequences

- The user can play a live set with the launcher and keep the song on the timeline.
- The engine keeps the launcher state of each track and changes between the launcher and the timeline at quantized times. When an edit replaces the arrangement, the launcher state continues.
- Loops with a different tempo move away from the bars. Warping in a later version will correct this.
- A loop that does not start and end at a zero crossing can make a click at the loop point.
- The selection must know two types of clips.

## Alternatives

- Always the next bar: no setting, but no instant starts and no long phrases.
- A quantization setting on each clip: more control for live sets, but more interface in version 0.1.
- The full slot as one button: a larger target, but no selection, so a clip can only be removed with a drag or a menu.
- A silent track until a Back to timeline button: a live set never goes back into the song by surprise, but the user must remember the button.
- Loops on whole bars: the clips stay on the bars, but longer audio is cut and shorter audio gets a gap.
- Clips that play one time: simpler, but most launcher clips are loops.
- Drags only from the browser: a smaller slice, but no way to move clips between the two views.
- A scene that only starts its clips: tracks with an empty slot continue to play, but the result of a scene is not easy to predict.
- A launch that waits for Play: the clips start together with the transport, but a launch with the transport stopped makes no sound.
