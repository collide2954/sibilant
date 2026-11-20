# 0025. Use Inter, JetBrains Mono, Lucide icons, and a violet accent

- Status: Accepted
- Date: 2026-11-20

## Context

Record 0012 sets a dark graphite theme with one accent color. It does not select the typefaces, the icons, or the accent color. Sibilant must look the same on Windows, macOS, and Linux.

## Decision

- Sibilant bundles its typefaces. It does not use the fonts of the operating system.
- Inter is the typeface for all text.
- JetBrains Mono is the typeface for counters, time displays, and other numbers that change during playback. Its digits have equal widths, so the numbers do not move.
- The icons come from the Lucide icon font. We draw audio icons that Lucide does not have in the same style.
- The accent color is violet, #8B7CF6. It shows focus, selection, the playhead, and active controls.
- The base text size is 13 px at 100 % zoom.
- The windows use the title bar of the operating system.

## Consequences

- The font files and their licenses are in the repository. Inter and JetBrains Mono use the SIL Open Font License 1.1. Lucide uses the ISC license, and the MIT license for the icons that come from Feather.
- The Open Font License lets Sibilant bundle the fonts, but nobody can sell the fonts alone.
- Text and icons look the same on all three operating systems.
- Red for record and yellow for solo do not look like the accent.

## Alternatives

- IBM Plex Sans and IBM Plex Mono: one family, but a more technical character.
- System fonts: no files to bundle, but a different look on each operating system.
- Phosphor icons: six weights, but larger files.
- Custom icons only: a unique look, but much work to draw and to keep consistent.
- An azure, aqua, or amber accent: azure is close to other DAWs, aqua is close to the green of play, and amber is close to the yellow of solo.
- Text at 12 px or 14 px: more content on the screen, or text that is easier to read.
- A custom title bar: more space and a custom look, but more work on each operating system.
