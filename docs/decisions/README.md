# Decision records

This folder contains one record for each important design decision. Each record gives the context, the decision, the consequences, and the alternatives.

## Add a record

1. Copy the structure of an existing record.
2. Give the file the next free number.
3. Write the record in ASD-STE100 Simplified Technical English.
4. Add the record to the list in this file.

To change a decision, write a new record. Then change the status of the old record to "Replaced by NNNN".

## Records

| Record | Decision |
| --- | --- |
| [0001](0001-hybrid-timeline-and-clip-launcher.md) | Use a hybrid timeline and clip launcher |
| [0002](0002-audio-arranger-first.md) | Make the first milestone an audio arranger |
| [0003](0003-clap-then-vst3.md) | Host CLAP plugins first, then VST3 |
| [0004](0004-progressive-disclosure.md) | Use progressive disclosure for all users |
| [0005](0005-layered-workspace.md) | Use a layered Cargo workspace |
| [0006](0006-immutable-project-and-snapshot-undo.md) | Use an immutable project and snapshot undo |
| [0007](0007-cpal-behind-a-device-trait.md) | Use cpal for audio devices behind a Sibilant trait |
| [0008](0008-project-folder-with-json-manifest.md) | Save projects as a folder with a JSON manifest |
| [0009](0009-f32-audio-and-f64-time.md) | Use f32 for audio and f64 for time calculations |
| [0010](0010-integer-time-in-two-domains.md) | Keep time as integers in two domains |
| [0011](0011-real-time-safe-engine.md) | Make the engine real-time safe and ready for parallel processing |
| [0012](0012-dark-theme-with-tokens.md) | Use a dark graphite theme with one accent color |
| [0013](0013-fixed-resizable-zones.md) | Use fixed window zones that the user can resize |
| [0014](0014-inline-clip-launcher.md) | Show the clip launcher inline in each track row |
| [0015](0015-mouse-first-with-full-keyboard.md) | Make the mouse the main input and give full keyboard control |
| [0016](0016-trunk-based-conventional-commits.md) | Push small commits to main with one-line Conventional Commits |
| [0017](0017-test-driven-development.md) | Use test-driven development for all code |
| [0018](0018-same-checks-everywhere.md) | Run the same checks on each computer and in CI |
| [0019](0019-permissive-licenses-only.md) | Accept only dependencies with permissive licenses |
| [0020](0020-ste-documentation-and-few-comments.md) | Write documentation in ASD-STE100 and keep comments to a minimum |
| [0021](0021-tick-resolution-and-time-types.md) | Use 705,600 ticks for each quarter note and typed time values |
| [0022](0022-time-signatures-at-bar-lines.md) | Change time signatures only at bar lines and use the lower number for the beat |
| [0023](0023-project-model-of-values.md) | Model the project as values with clips at musical positions |
| [0024](0024-faders-and-balance-pan.md) | Use faders to +6 dB and a balance pan that keeps the center level |
| [0025](0025-typefaces-icons-and-accent.md) | Use Inter, JetBrains Mono, Lucide icons, and a violet accent |
| [0026](0026-license-exceptions-for-iced.md) | Accept three crates with the BSL-1.0 and CC0-1.0 licenses for Iced |
| [0027](0027-unmaintained-crates.md) | Stop on all unmaintained crates and accept two that Iced needs |
| [0028](0028-track-area-defaults.md) | Start with two audio tracks and give tracks palette colors |
| [0029](0029-timeline-zoom-and-ruler.md) | Zoom the timeline with Ctrl and the wheel, and show bars and beats |
| [0030](0030-engine-frames-and-queues.md) | Count engine time in frames and control the engine through lock-free queues |
| [0031](0031-transport-and-metronome.md) | Return to the play start on stop, and use a synthesized metronome |
| [0032](0032-bsd-1-clause-license.md) | Accept the BSD-1-Clause license |
| [0033](0033-import-through-the-browser.md) | Bring audio files into a project through the Browser zone |
| [0034](0034-wav-files-in-a-media-crate.md) | Read WAV files in a media crate |
| [0035](0035-clip-audio-in-memory.md) | Keep the audio of clips in memory at the output sample rate |
| [0036](0036-fades-at-clip-edges.md) | Fade each clip edge for 4 ms |
| [0037](0037-preview-with-a-click.md) | Preview audio files with a click in the browser |
| [0038](0038-project-file-in-a-project-folder.md) | Save each project as a .sibilant file in a project folder |
| [0039](0039-autosave-recovery-copies.md) | Write recovery copies automatically and offer them after a crash |
| [0040](0040-edit-clips-on-the-timeline.md) | Edit clips on the timeline with the mouse and with shortcuts |
| [0041](0041-mixer-strips-faders-and-meters.md) | Mix in channel strips with tapered faders and peak meters |
| [0042](0042-export-to-a-wav-file.md) | Export the project to a WAV file in the background |
| [0043](0043-launch-clips-at-quantized-times.md) | Start and stop launcher clips at quantized times |
