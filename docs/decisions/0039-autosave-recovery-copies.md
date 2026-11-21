# 0039. Write recovery copies automatically and offer them after a crash

- Status: Accepted
- Date: 2026-11-21

## Context

A crash or a power cut must not remove much work. Record 0038 gives an `autosave` folder in each project folder, but it gives no rules for the folder. The project file of the user must change only when the user saves, because Don't Save must discard changes (record 0038).

## Decision

- While a project has unsaved changes, Sibilant writes a recovery copy every 2 minutes. If nothing changed after the last copy, Sibilant does not write a copy.
- A recovery copy is a project file in the format of record 0038. Autosave does not change the project file of the user.
- For a project that has a project folder, the copies go into the `autosave` folder of the project. The name of a copy is the name of the project and the time in seconds after 1970, for example `MySong 1795100000.sibilant`. Sibilant keeps the 10 newest copies and deletes the older copies.
- Autosave copies each new audio file into the `audio` folder, as Save does. Thus a copy stays complete when the user moves the original audio files.
- For a project that has no project folder, the copies go into a recovery folder in the data folder of the user. Sibilant copies the audio of the project into this folder too.
  - Windows: `%LOCALAPPDATA%\Sibilant\Recovery`
  - macOS: `~/Library/Application Support/Sibilant/Recovery`
  - Linux: `$XDG_STATE_HOME/sibilant/recovery`, or `~/.local/state/sibilant/recovery`
- Each running Sibilant has a session folder in the recovery folder. The session folder contains a lock file and a small file that gives the project file and the newest copy. The copies and the audio of a project without a project folder are also in the session folder.
- Sibilant keeps the lock file locked while it runs. The operating system unlocks the file when the process stops, also after a crash.
- Sibilant removes its session folder when the window closes normally, after a new project, and after Open. After Save, Sibilant removes the copies and the audio of a project that had no project folder.
- When Sibilant starts, it looks for session folders that are not locked. A session folder has something to restore when it gives a copy that is newer than the project file. Sibilant removes the other session folders that are not locked.
- For each session folder that has something to restore, a dialog in the Sibilant window asks "Restore" or "Don't Restore". The dialog gives the name of the project and the age of the copy, for example "4 minutes ago".
  - Restore opens the copy. The title shows a dot (●), because the copy is not saved. Save writes the project file at its usual location. For a project without a project folder, Save does the same as Save As. Sibilant uses the session folder of the copy as its own session folder. After Restore, Sibilant asks about the other session folders at the next start.
  - Don't Restore removes the session folder. The copies in the `autosave` folder of the project stay. Then Sibilant asks about the next session folder.
  - The Enter key gives Restore. The Esc key closes the dialog, and Sibilant asks again at the next start.
- Autosave and Save do not write at the same time. If the user saves while autosave writes, the save starts when autosave ends. If the time for a copy comes while a save writes, Sibilant does not write that copy.

## Consequences

- A crash removes not more than 2 minutes of work.
- On macOS, Cmd+Q closes Sibilant without the dialog for unsaved changes (record 0038). The session folder stays, thus the next start offers the copy.
- After Don't Save, the `audio` folder can contain audio files that the saved project does not use. A later version can remove these files.
- A project without a project folder uses disk space in the recovery folder until the user saves or discards it.
- Iced has no timer for its thread pool. Thus a thread of Sibilant gives the 2-minute signal.

## Alternatives

- Save the project file automatically: no work is lost, but Don't Save cannot discard changes.
- A copy 30 seconds after each change: less work is lost, but Sibilant writes more often during edits.
- No autosave for a project without a project folder: simpler, but a crash removes all of that work.
- Restore without a question: one step less, but the user cannot discard a copy that they do not want.
