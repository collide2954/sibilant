# 0014. Show the clip launcher inline in each track row

- Status: Accepted
- Date: 2026-11-19

## Context

Sibilant has a timeline and a clip launcher (refer to record 0001). The user must see the two views together and must move clips between them easily.

## Decision

Each track row shows these parts, from left to right:

1. The track header.
2. The launcher slots of the track.
3. The timeline lane of the track.

A scene row below the tracks starts all clips in one launcher column. The user can change the width of the launcher area or hide it. A full-screen performance grid can come in a later version.

## Consequences

- The launcher slots and the timeline lane of a track are always on the same row.
- The user can drag a clip from a slot to the timeline, and from the timeline to a slot.
- On a narrow screen, the launcher area makes the timeline narrower.

## Alternatives

- Two full-window views and one key to change between them: more space for each view, but the user never sees the two views together.
- The two layouts in version 0.1: flexible, but more interface work early.
