# 0042. Export the project to a WAV file in the background

- Status: Accepted
- Date: 2026-11-21

## Context

Version 0.1 must export the project to a WAV file (record 0002). The user must be able to make a file for a CD, for a streaming service, or for more work in a different program. A long project must not stop the interface while Sibilant makes the file.

## Decision

- The File menu has the item "Export Audio…" after Save As. The shortcut is Ctrl+Shift+E. On macOS, the shortcut is Cmd+Shift+E.
- A dialog in the Sibilant window gives two choices:
  - The sample rate: 44.1 kHz, 48 kHz, or 96 kHz.
  - The bit depth: 16-bit, 24-bit, or 32-bit float.
- The first choices are 48 kHz and 24-bit. Sibilant keeps the last choices until it closes.
- The dialog has the buttons Export and Cancel. The Enter key gives Export, and the Esc key gives Cancel. After Export, the save dialog of the operating system asks for the file. The first name is the name of the project with the extension `.wav`.
- The file contains the project from bar 1 to the end of the last clip. Muted tracks and soloed tracks follow the rules of playback (record 0024). The metronome and the preview are not in the file. If the project has no clips, Sibilant does not make a file and shows a notice.
- Sibilant makes the audio in the background, faster than real time. When the export rate is different from the rate of the clips in memory (record 0035), Sibilant reads the audio files again at the export rate.
- 16-bit files get triangular (TPDF) dither. In 16-bit and 24-bit files, Sibilant limits each sample to the range of the format. 32-bit float files keep all values.
- Sibilant writes the file to a temporary file first. Then it renames the temporary file (record 0038).
- While the export runs, the notice line of the browser shows a progress bar and a Cancel button. Cancel stops the export and removes the temporary file. The user can continue to work. The export uses the project as it was when the export started.
- When the export is done, a notice gives the name of the file and the peak level, for example "Exported MySong.wav, peak -1.2 dBFS". If samples were limited, the notice also says that the export clipped.

## Consequences

- The user can share a finished song.
- An export at a different sample rate takes more time, because Sibilant reads and converts the audio files again.
- Sibilant does not keep the export choices after it closes. A later version can keep them with the other settings of the user.

## Alternatives

- A fixed format of 24-bit at 48 kHz: one click to export, but no 44.1 kHz or 16-bit files.
- The rate of the audio device: the file has the rate that the user hears, but the rate changes with the device.
- An export from the play start: useful for parts of a song, but the user must remember to move the play start to bar 1.
- Only a notice when the export is done: simpler, but a long export gives no sign that it runs.
- No shortcut: one shortcut less to learn, but slower for users who export often.
