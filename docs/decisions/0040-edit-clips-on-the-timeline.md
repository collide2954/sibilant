# 0040. Edit clips on the timeline with the mouse and with shortcuts

- Status: Accepted
- Date: 2026-11-21

## Context

Version 0.1 must let the user arrange clips on the timeline (record 0002). The user can put an audio file on a track (record 0033), but the user cannot move, trim, split, or delete a clip. Record 0015 makes the mouse the main input, with a shortcut for each command.

## Decision

- Selection:
  - A click on a clip selects only that clip.
  - Ctrl+click adds a clip to the selection or removes it. On macOS, Cmd+click does this.
  - A drag on an empty part of a lane draws a box. When the user releases the mouse button, Sibilant selects the clips that the box touches. With Ctrl (Cmd on macOS), the box adds to the selection.
  - A click on an empty part of a lane, or the Esc key, clears the selection.
  - A selected clip has an outline.
- Move:
  - A drag on a clip moves the selected clips. If the clip is not selected, the drag selects it first.
  - The clips move in time and to other tracks together. They keep their distances.
  - The clips cannot move above the first track or below the last track.
  - A drag of less than 4 pixels is a click. It does not move the clips.
- Trim: A drag on the left or the right edge of a clip moves that edge. The edge area is 6 pixels wide. The audio stays at its position in time. A clip cannot start before the start of its audio file or end after the end of its audio file.
- Snap: Moves and trims snap to the grid that the timeline shows. This is the beat when the timeline shows beat lines, and the bar when it does not. When the user holds Alt (Option on macOS) during the drag, the clip does not snap.
- When a moved or trimmed clip covers other clips, it replaces the parts that it covers. This is the same rule as for a dropped audio file.
- During a move or a trim, the timeline shows the result before the user releases the mouse button. The Esc key stops the drag, and the clips stay where they were.
- Shortcuts. On macOS, Cmd replaces Ctrl.
  - Ctrl+E splits each selected clip at the play start. After the split, the two parts are selected.
  - Delete or Backspace removes the selected clips.
  - Ctrl+D puts a copy of the selected clips directly after the end of the selection, on the same tracks. The copies become the selection.
- Each edit is one step in the undo history. The selection is not part of the history. Undo and Redo clear the selection.
- Play start:
  - A click on the ruler moves the play start to that position, with the same snap as a move. During playback, the playback continues from the new position.
  - A marker in the ruler shows the play start. Stop returns to the play start (record 0031).
  - The project file does not keep the play start. An opened project starts at bar 1.

## Consequences

- The user can arrange a project with the mouse only. An expert can do the same edits with shortcuts.
- A move, a trim, and a box can cross tracks. Thus the application keeps the state of the drag, not each lane.
- The engine needs a command that moves the play start.

## Alternatives

- One clip at a time: simpler, but slow for large arrangements.
- Shift to drag without snap, or a snap button in the top bar: Shift is free for other use, and a button gives more control, but Alt is the more usual key for fine moves.
- Copy, cut, and paste in the same slice: more complete, but the slice becomes larger.
- A click on an empty lane to move the play start: one click less, but that click cannot then clear the selection or start a box.
