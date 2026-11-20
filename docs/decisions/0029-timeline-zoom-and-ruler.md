# 0029. Zoom the timeline with Ctrl and the wheel, and show bars and beats

- Status: Accepted
- Date: 2026-11-20

## Context

The track area has a lane for each track (record 0014). The lanes need a ruler, a grid, and a method to zoom and to scroll.

## Decision

- The mouse wheel scrolls the tracks up and down. Shift and the wheel scroll the timeline left and right. A horizontal scroll from a trackpad also scrolls the timeline left and right.
- Ctrl and the wheel zoom the timeline around the mouse pointer. On macOS, Cmd replaces Ctrl.
- When the timeline zooms, the musical position below the mouse pointer stays below the mouse pointer.
- Each step of the wheel changes the zoom by a factor of 1.25. The zoom is between 2 px and 1000 px for each quarter note. At the start, the zoom is 32 px for each quarter note.
- The timeline does not scroll before the start of bar 1.
- The ruler shows bar numbers. When bars are narrow, the ruler shows only every second, fourth, or eighth bar number, and so on, so that the labels are at least 48 px apart.
- The lanes show a line at each bar. When a beat is at least 12 px wide, the lanes also show a line at each beat.
- A clip on a lane is a rectangle in the color of its track. The rectangle starts at the start of the clip and ends where the clip stops. The name of the clip is at the top left. Waveforms come with audio import.
- The zoom and the scroll position are part of the view, not of the project. Undo does not change them.

## Consequences

- The wheel operates as in Ableton and most editors.
- The ruler does not show clock time. A ruler for clock time can come later.

## Alternatives

- A wheel that zooms: fast zooming, but scrolling the tracks needs a key or the scroll bar.
- Bars and clock time in two rows: more information, but more height above the tracks.
