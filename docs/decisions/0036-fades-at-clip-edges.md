# 0036. Fade each clip edge for 4 ms

- Status: Accepted
- Date: 2026-11-20

## Context

A clip can start or stop in the middle of a waveform. A sudden jump in the signal causes a click.

## Decision

- The engine fades in the first 4 ms of each clip. It fades out the last 4 ms of each clip.
- The fades are linear.
- If a clip is shorter than 8 ms, each fade is half of the clip.
- When the next clip cuts off a clip, the fade out ends at the start of the next clip.
- Fade handles come in a later version. Then each clip has its own fade lengths.

## Consequences

- Clip edges do not click.
- The first and the last 4 ms of each clip are quieter than in the file. A sharp attack at the start of a clip becomes a little softer.

## Alternatives

- No fades: each sample plays as in the file, but some edges click.
- Longer fades: softer edges, but sharp attacks lose more.
