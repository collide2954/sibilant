# 0031. Return to the play start on stop, and use a synthesized metronome

- Status: Accepted
- Date: 2026-11-20

## Context

The transport and the metronome are the first parts of Sibilant that make sound. Their behavior must be easy to predict for new users and fast for professional users.

## Decision

- Space starts playback at the play start marker. Space during playback stops playback and moves the playhead back to the play start marker.
- Shift+Space starts playback at the position where playback stopped last.
- The play start marker is at bar 1. A later version lets the user move it.
- The metronome is a synthesized click: a short sine tone that decays fast. The first beat of each bar has a higher tone.
- The metronome is off when Sibilant starts. A button in the transport bar turns it on and off.
- The transport bar shows the position as bars, beats, and sixteenths (1.1.1), and the clock time as minutes, seconds, and milliseconds (0:00.000). These numbers use JetBrains Mono.

## Consequences

- A user can play the same part again with one key.
- The metronome needs no sample files.
- Playback without clips and without the metronome makes no sound.

## Alternatives

- A playhead that stays where it stopped: like a tape machine, but to repeat a part needs more steps.
- A recorded woodblock sample: more natural, but it needs a sample file with a suitable license.
- Bars and beats without sixteenths and clock time: simpler, but less precise.
- A metronome that is on at the start: helps beginners keep time, but it clicks in each new session.
