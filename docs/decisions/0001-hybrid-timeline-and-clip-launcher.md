# 0001. Use a hybrid timeline and clip launcher

- Status: Accepted
- Date: 2026-11-19

## Context

Users make music in different ways. Some users arrange clips on a timeline from left to right. Other users start and stop clips in a grid, for example during a live performance. Sibilant must be easy to use for the two groups.

## Decision

Sibilant has a timeline and a clip launcher from the first release. The two views use the same tracks and the same clips. Each track plays its launcher clip or its timeline content. The user can move a clip from one view to the other view.

## Consequences

- The data model keeps a clip separate from the place where the clip plays.
- The engine changes each track from timeline playback to launcher playback at a quantized time.
- The first release has more scope than a release with only a timeline.

## Alternatives

- Timeline first and launcher later: less work at the start, but the launcher becomes an addition to the design.
- Launcher first: good for live use, but not good when you record and edit audio.
- Patterns in a playlist: good for beats, but not natural for recorded audio.
