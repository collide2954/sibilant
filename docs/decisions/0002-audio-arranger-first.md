# 0002. Make the first milestone an audio arranger

- Status: Accepted
- Date: 2026-11-19

## Context

A small first milestone shows early that all parts of Sibilant operate together. These parts are the audio devices, the engine, the user interface, and the project files.

## Decision

Version 0.1 is an audio arranger. With version 0.1, the user can do these tasks:

- Import audio files.
- Arrange clips on the timeline.
- Start, loop, and stop clips in the clip launcher at quantized times.
- Mix tracks with gain, pan, mute, and solo.
- Play the project through an audio device.
- Export the project to a WAV file.

Version 0.1 does not change the speed of clips to match the tempo (warping). Recording, MIDI, and plugins come after version 0.1.

## Consequences

- Each layer of the architecture has a real use before the scope increases.
- Launcher clips play at their recorded speed. Loops at a different tempo do not stay in time until Sibilant has warping.

## Alternatives

- Audio recorder and editor: more useful at version 0.1, but the milestone takes longer.
- Full core with MIDI and plugins: a long time before the first release.
- MIDI sketchpad: fast results, but no audio path, and most users need audio first.
- Warping in version 0.1: time stretch is a large project and delays the release.
