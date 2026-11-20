# 0037. Preview audio files with a click in the browser

- Status: Accepted
- Date: 2026-11-20

## Context

Users must hear a file before they put it in a project. Record 0033 moved the preview to a later version. Now a press on a file in the browser starts a drag. A click to preview a file must not start a drag.

## Decision

- A click on a file in the browser plays the file from its start. A click on the file that plays stops it. A click on a different file stops the old preview and plays the new file.
- Esc stops the preview.
- A drag starts only when the pointer leaves the row of the file. A click that does not move the pointer out of the row starts a preview.
- The preview plays the file one time, at its own speed. It starts immediately, also while the song plays.
- The preview has its own volume control at the bottom of the browser. The range is from silence to 0 dB. When Sibilant starts, the value is -6 dB. The project does not keep this value, and undo does not change it.
- The preview goes directly to the first two channels of the output. The master gain does not change it.
- The engine plays the preview with its own voice. A lock-free queue moves the audio of each new preview to the engine. A second queue moves the old audio back, thus the audio thread does not release memory (record 0035).
- Sibilant keeps the audio of the last preview in memory. When the user drops that file on a track, Sibilant uses this audio and does not read the file again.
- The row of the file that plays shows a speaker icon.

## Consequences

- Users can hear files quickly, also during playback.
- Before a long file plays, there is a short wait, because Sibilant reads and converts the full file first.
- Each time Sibilant starts, the preview volume is -6 dB again.

## Alternatives

- A play button on each file: clear, but the targets are small and the browser shows more icons.
- A preview of each selected file, with the arrow keys: fast in a sample library, but the browser needs keyboard focus.
- A fixed level or the master gain: simpler, but users cannot set the preview level apart from the mix.
- A start on the next bar: in time with the song, but each click has a wait.
- A loop: good for loops, but tiring for short sounds.
