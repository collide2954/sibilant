# 0003. Host CLAP plugins first, then VST3

- Status: Accepted
- Date: 2026-11-19

## Context

Users have plugins in different formats. The most important formats are VST3 and CLAP. Audio Units (AU) is available only on macOS. LV2 is common on Linux. The license of each format must be compatible with the Apache License 2.0.

## Decision

Sibilant hosts CLAP plugins first. VST3 support comes next and uses the same host abstraction. AU and LV2 support come later.

## Consequences

- CLAP uses the MIT license, which is compatible with Apache 2.0. The `clack-host` crate gives a safe Rust interface to CLAP.
- The VST3 SDK uses the MIT license from version 3.8 (October 2025). Because of this, VST3 support is compatible with Apache 2.0.
- The host abstraction must not depend on one plugin format.
- Users with only VST3 plugins must wait for the second plugin milestone.

## Alternatives

- VST3 first: more plugins sooner, but the Rust tools for a VST3 host are less complete.
- All formats together: maximum coverage, but a much larger scope.
- CLAP only: simple, but many commercial plugins are available only as VST3 or AU.
