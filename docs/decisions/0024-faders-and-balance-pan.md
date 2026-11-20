# 0024. Use faders to +6 dB and a balance pan that keeps the center level

- Status: Accepted
- Date: 2026-11-20

## Context

Version 0.1 mixes tracks with gain, pan, mute, and solo (record 0002). The mix needs a gain range, a pan law, and rules for mute and solo.

## Decision

- A gain is a value in decibels from silence to +6 dB. A gain of 0 dB does not change the audio. Silence has the value minus infinity.
- The code keeps the gain in decibels, because the user sees and types decibels. The engine calculates the amplitude when the gain changes.
- A pan is a position from -1 (left) to +1 (right). The center is 0.
- The pan is a balance control. At the center, the two channels do not change. When you pan to one side, the near channel does not change. The gain of the far channel is 1 minus the distance from the center.
- A muted track is not heard, also when it is soloed.
- If one or more tracks are soloed, only the soloed tracks are heard.
- The master track has a gain from silence to +6 dB.

## Consequences

- A pan never makes a track louder, so a pan cannot cause clipping.
- A mono clip at a hard pan is 3 dB quieter than at the center.
- The file format needs a special value for silence, because JSON has no value for infinity.
- Exclusive solo and solo safe are functions of the user interface. They can come later.

## Alternatives

- Faders to +12 dB: more boost, but less fine control near 0 dB.
- Equal loudness (constant power with 3 dB of compensation): a mono clip keeps its loudness, but hard pans can cause clipping.
- A console law with -3 dB at the center: even loudness, but each track at the center plays 3 dB below its file.
- A far channel that follows a cosine curve: the same end points, but the stereo image moves slowly near the center.
- A gain as an amplitude: no infinity in files, but each displayed value and each typed value needs a conversion.
- A pan law for each project: more control, but more settings and tests in version 0.1.
