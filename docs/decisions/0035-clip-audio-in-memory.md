# 0035. Keep the audio of clips in memory at the output sample rate

- Status: Accepted
- Date: 2026-11-20

## Context

The engine must play the audio of clips without delays. The audio thread must not read files, wait for locks, or release memory (record 0011). An audio file can have a sample rate that is different from the sample rate of the output.

## Decision

- Sibilant reads each audio file one time, in the background. It keeps the samples in memory as 32-bit float values.
- If the sample rate of a file is different from the sample rate of the output, Sibilant converts the samples in the background with `rubato` (MIT). The file does not change.
- The same background task calculates the waveform peaks.
- The user interface thread prepares an arrangement for the engine. The arrangement contains the tempo map, the clips of each track in frames, the gains of each track, and the master gain. An arrangement does not change after it is prepared.
- A lock-free queue moves each new arrangement to the engine. A second queue moves the old arrangement back. The user interface thread releases its memory.
- A mono clip plays on the left and the right channels. The engine writes the stereo mix and the metronome to the first two channels of the output. A mono output gets the average of the left and the right channels.
- An audio source keeps the path of its original file. A later record about saved projects decides if Sibilant copies the audio files into the project folder.

## Consequences

- A slow disk cannot interrupt playback.
- A minute of stereo audio at 48 kHz uses approximately 23 MB of memory. Long sessions need much memory.
- When the sample rate of the output changes, Sibilant must convert the audio again.
- An undo does not remove the audio from memory. Thus a redo is immediate.

## Alternatives

- Read the files during playback: less memory, but more complex, and a slow disk can interrupt playback.
- Convert the sample rate in real time: no wait after an import, but more processing for each buffer.
- A mutex around the arrangement: simple, but the audio thread can wait.
