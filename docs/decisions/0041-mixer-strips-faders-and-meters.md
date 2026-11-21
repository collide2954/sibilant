# 0041. Mix in channel strips with tapered faders and peak meters

- Status: Accepted
- Date: 2026-11-21

## Context

Version 0.1 must mix tracks with gain, pan, mute, and solo (record 0002). Record 0024 gives the ranges and the pan law. The bottom zone has a tab for the mixer (record 0013), but the user cannot change the gain or the pan of a track yet. The engine already applies the gain, the pan, and the master gain.

## Decision

- The Mixer tab of the bottom zone shows a channel strip for each track, in the order of the tracks. When the strips do not fit, they scroll horizontally. A master strip is always at the right end.
- A track strip contains these parts, from the top: the name and the color of the track, a pan slider, a gain fader with a meter, the gain value, and the mute and solo buttons.
- The master strip contains a gain fader, a meter, and the gain value. It has no pan, mute, or solo (record 0024).
- Fader:
  - The fader is vertical. The bottom of the travel is silence, and the top is +6 dB.
  - 0 dB is at three quarters of the travel. Between the ends, the steps become finer near 0 dB.
  - The value shows below the fader, for example "-6.0 dB" or "-inf dB".
- Pan: The pan is a horizontal slider from left to right. It shows "L50", "C", or "R50". The number is the percent of the full pan.
- Control:
  - A drag moves a fader or a pan from its current value. A click does not make the control jump.
  - With Ctrl (Cmd on macOS), a drag moves ten times more slowly.
  - A double-click puts a fader to 0 dB or a pan to the center.
  - The user hears each change during the drag. A drag or a reset is one step in the undo history. The Esc key during a drag puts the control back to its value before the drag.
- Meters:
  - Each strip has a peak meter on the same scale as its fader.
  - A track meter shows the level after the fader and the pan. The master meter shows the output of Sibilant.
  - A meter falls by 24 dB each second.
  - When a level reaches 0 dBFS, the top of the meter is red for 2 seconds.
- The engine keeps the highest level of each track and of the output since the last frame. The interface reads these values one time for each frame. The engine does not allocate memory and does not wait for this (record 0011).

## Consequences

- The user can mix a project with the mouse only.
- The meters need frames while a meter falls, also after playback stops.
- Typed values, control with the mouse wheel, and a peak hold for the meters can come in a later version.

## Alternatives

- A fader that is linear in dB: simpler, but coarse near 0 dB, where the user adjusts most.
- Horizontal sliders instead of strips: more tracks fit, but the zone does not look like a mixer.
- A rotary knob for the pan: compact, but difficult to drag precisely with a mouse.
- Typed values in this slice: exact values, but each control needs a text field.
- Meters in a later slice: a smaller slice, but the user cannot see the levels that the faders set.
