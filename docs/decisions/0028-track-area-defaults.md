# 0028. Start with two audio tracks and give tracks palette colors

- Status: Accepted
- Date: 2026-11-20

## Context

The track area shows the tracks of the project (record 0014). A new user must see where to put audio. The track headers must stay calm, and the track colors must stay readable in each theme.

## Decision

- A new project has two audio tracks and four scenes. The tracks have the names "Audio 1" and "Audio 2". The scenes have the names "Scene 1" to "Scene 4".
- A new track gets the name "Audio" and the number after the highest number of the other "Audio" tracks.
- Each track has a color from a palette of 16 colors. The project keeps the number of the color in the palette, not the color value. Each theme sets the 16 color values.
- A new track gets the color that follows the color of the last track in the list.
- A track header shows the color, the name, and the mute and solo buttons. Volume and pan are in the mixer and in the inspector.
- A track row is 64 px high. A launcher slot is 80 px wide.
- The button "Add track" below the tracks adds a track. Ctrl+T also adds a track.
- Ctrl+Z undoes the last change. Ctrl+Shift+Z and Ctrl+Y do the change again.
- On macOS, Cmd replaces Ctrl in these shortcuts, and Cmd+Y is not a shortcut.

## Consequences

- A new user can put audio on a track or in a slot immediately.
- A project looks correct in each theme, because the theme gives the color values.
- A change to the palette of a theme changes the track colors of all projects in that theme.
- The shortcuts move into the list of commands of record 0015 when that list exists.

## Alternatives

- An empty project: clean, but a blank screen can confuse beginners.
- Any color from a color picker: unlimited choice, but colors can clash with the theme.
- A palette and a color picker: most flexible, but more interface and more tests.
- Volume and pan in each header: more control without the mixer, but busier headers.
