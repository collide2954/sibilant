# 0038. Save each project as a .sibilant file in a project folder

- Status: Accepted
- Date: 2026-11-20

## Context

Record 0008 gives a project folder with the name `MySong.sibilant` and a file `project.json` in it. The file dialogs of Windows, macOS, and Linux open files, not folders. Thus a project folder with an extension is difficult to open. Users must keep their work when they close Sibilant. They must not lose changes by accident.

## Decision

- A project is a folder with the name of the project. The folder contains the project file. The project file has the name of the project and the extension `.sibilant`:

  ```text
  MySong/
    MySong.sibilant   format version, tempo map, tracks, clips, scenes, mixer
    audio/            copies of the audio files of the project
    autosave/         automatic copies of the project file
    cache/            waveform data that Sibilant can make again
  ```

- The project file is JSON text with a format version. The migrations of record 0008 apply. Positions are in ticks, and lengths are in superclocks (record 0021). Each path in the file is relative to the project folder and uses the `/` character.
- Save copies each audio file that is not in the `audio` folder into that folder. Sibilant copies each file only one time. If the folder has a file with the same name, the copy gets a number, for example `Kick 2.wav`.
- Sibilant writes the project file and each copy to a temporary file first. Then it renames the temporary file (record 0008).
- Sibilant makes the `autosave` and `cache` folders when it needs them. Autosave and recovery after a crash come in a later version.
- The File menu at the left end of the top bar contains New, Open, Save, and Save As. The shortcuts are Ctrl+N, Ctrl+O, Ctrl+S, and Ctrl+Shift+S. On macOS, the shortcuts use Cmd.
- Open and Save As use the file dialogs of the operating system, through the `rfd` crate. On Linux, the dialogs come from the XDG desktop portal.
- Save As makes a new project folder with the name that the user gives. If the user selects a project file in a folder with the same name, Sibilant saves into that folder. Sibilant does not replace a different project with the same name. It shows an error.
- Save does the same as Save As for a project that has no project file yet.
- A dialog in the Sibilant window asks "Save", "Don't Save", or "Cancel" before Sibilant removes changes that are not saved. This occurs before a new project, before Open, and before the window closes.
  - The buttons have the order of each operating system. On Windows, the order is Save, Don't Save, and Cancel, at the right side. On macOS and Linux, Don't Save is at the left side, and Cancel and Save are at the right side.
  - The Enter key gives Save, and the Esc key gives Cancel. Don't Save has no key. Thus one key cannot remove changes by accident.
  - Save saves the project, and then Sibilant continues. If the save fails, or if the user closes the Save As dialog, Sibilant stays in the project.
  - If a save has not ended, Sibilant waits until it ends. Then it makes the new project, starts Open, or closes the window.
- The window title shows the name of the project. When the project has changes that are not saved, a dot (●) is in front of the name.
- Sibilant starts with a new project, as before.
- When an audio file of a project is missing, the project opens. The clips of the file stay in their positions, but they are silent and show no waveform. A notice in the browser gives the names of the missing files.
- The `sibilant-project` crate of record 0005 contains the format of the project file, Save, and Open.

## Consequences

- A project folder contains all of its audio. Users can move the folder or copy it to a different computer.
- A project uses more disk space, because Sibilant copies the audio files.
- In the dialogs of the three operating systems, users open a file, not a folder.
- On Linux, the file dialogs need an XDG desktop portal for GTK, GNOME, or KDE, or the zenity program.
- On macOS, Cmd+Q and the Quit item of the application menu close Sibilant without the dialog for unsaved changes. The menu of winit stops the application directly. A later version must catch these two commands.

## Alternatives

- The `MySong.sibilant` folder of record 0008: one name for the project, but the dialogs must select a folder, and this operation is different on each operating system.
- References to the audio files at their original locations: no more disk space, but the project breaks when the user moves, renames, or deletes a file.
- Dialogs in the Sibilant window: the same on all systems, but without the recent folders, the favorites, and the cloud drives of the system dialogs.
- A message box of the operating system for unsaved changes: a native dialog, but on Linux it needs zenity, and on Windows the button names need a manifest.
- Save without a question: no dialog, but users cannot discard changes that they do not want.
- One order of the buttons on all systems: one layout, but on Windows the layout is different from the other applications.
