# 0033. Bring audio files into a project through the Browser zone

- Status: Accepted
- Date: 2026-11-20

## Context

Users must put audio files on tracks. Iced 0.14 uses winit 0.30. When a user drops a file from a file manager, winit 0.30 does not give the position of the drop. On Wayland, winit 0.30 does not receive dropped files. Thus a drop from a file manager cannot put a clip at a bar of a track, and on Wayland users cannot drop files.

## Decision

- The Browser zone shows places at the top: Home, Music, Desktop, and Downloads. A place that does not exist does not show.
- Below the places, the browser shows the selected place as a folder tree. The user opens and closes each folder in the tree.
- The tree shows folders and the audio files that Sibilant can read. It does not show hidden files. Folders come before files, in alphabetical order.
- Sibilant reads the folders in the background. A slow drive does not stop the user interface.
- The user drags a file from the browser onto a lane. The clip starts at the nearest line of the visible grid: a beat line when the beat lines show, or a bar line when they do not show.
- When the user drops a file below the last track, Sibilant adds an audio track with the clip. One undo removes the track and the clip.
- A preview of files comes in a later version.
- Drops from a file manager come when Iced gives the position of a drop on all platforms.

## Consequences

- Import works the same on Windows, macOS, and Linux.
- Sibilant needs its own drag and drop between the zones of the window.
- Users cannot drag files from a file manager yet.
- The snap follows the zoom, as the grid does.

## Alternatives

- A drop from a file manager, then a click on a lane: no browser is necessary, but users on Wayland cannot import files.
- The browser and drops from a file manager together: more work before the first clip plays.
- One folder at a time with a path bar: simpler, but slower in a large sample library.
- A snap to the nearest bar only: the same at all zoom levels, but a clip cannot start on a beat.
- No snap: free placement, but clips do not align with the beats.
- A native file dialog: familiar, but on Linux it needs GTK 3 crates, which are not maintained, or D-Bus crates (record 0027).
