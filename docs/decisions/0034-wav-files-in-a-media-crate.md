# 0034. Read WAV files in a media crate

- Status: Accepted
- Date: 2026-11-20

## Context

Sibilant must read audio files, change their sample rate, calculate waveform data, and find the audio files in folders. These operations use input and output, thus they cannot be in `sibilant-core`. They do not operate in real time, thus they do not belong in `sibilant-engine` (record 0005).

## Decision

- A new crate, `sibilant-media`, reads audio files, changes their sample rate, calculates waveform peaks, and finds the audio files in folders.
- The first version reads WAV files with `hound` (Apache-2.0). It reads 8-, 16-, 24-, and 32-bit integer samples and 32-bit float samples.
- Sibilant reads mono and stereo files. For a file with more channels, Sibilant shows an error.
- The reader changes integer samples to 32-bit float samples from -1 to 1. Float samples do not change.
- Other formats come later. They use the same function as WAV files.

## Consequences

- Users must convert other formats to WAV for now.
- The engine and the user interface do not know the file formats.

## Alternatives

- WAV and FLAC with `claxon`: more files, but `claxon` gets few updates.
- Most formats with Symphonia: one maintained crate, but its license is MPL-2.0 (record 0019).
- A WAV reader of our own: no dependency, but more code to test and to keep.
