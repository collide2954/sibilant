# 0013. Use fixed window zones that the user can resize

- Status: Accepted
- Date: 2026-11-19

## Context

New users need a layout that they can predict. Professional users need to see much information at the same time.

## Decision

The main window has fixed zones:

```text
+--------------------------------------------------+
| Transport                                        |
+----------+---------------------------+-----------+
| Browser  | Track area                | Inspector |
+----------+---------------------------+-----------+
| Mixer and clip editor (tabs)                     |
+--------------------------------------------------+
```

The user can change the size of each zone. The user can hide each zone, except the track area.

## Consequences

- The layout is the same on each computer. The documentation can show the same layout to all users.
- Each zone must operate correctly at small sizes and at large sizes.

## Alternatives

- Panels that the user can dock anywhere: flexible, but layouts can become unclear and are difficult to make attractive.
- One view at a time: clear, but the user cannot see the mixer and the timeline together.
- Many windows: good with many monitors, but difficult on a laptop.
