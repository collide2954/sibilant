# 0005. Use a layered Cargo workspace

- Status: Accepted
- Date: 2026-11-19

## Context

A digital audio workstation has many parts. Clear borders between the parts make the code easier to test, to change, and to compile.

## Decision

Sibilant is a Cargo workspace. Each crate has one purpose:

| Crate | Purpose |
| --- | --- |
| `sibilant-core` | Time, tempo map, project model, and edits. No input or output. |
| `sibilant-engine` | Real-time audio graph, transport, clip launcher, and DSP. |
| `sibilant-device` | Audio and MIDI devices behind Sibilant traits. |
| `sibilant-project` | Project files, format versions, and migrations. |
| `sibilant-ui` | Iced theme, custom widgets, and views. |
| `sibilant` | The application. It connects the other crates. |
| `xtask` | Development tasks and checks. |

Dependencies go in one direction only. The engine, the project files, and the user interface use `sibilant-core`. Only the `sibilant` crate uses all crates.

Add each crate when its first function is necessary.

## Consequences

- The compiler prevents dependencies in the wrong direction.
- Each crate has tests that do not need the other crates.
- Incremental builds stay fast.
- All crates share one set of lints and one set of package settings.

## Alternatives

- Two crates (core and application): fewer crates, but the model and the engine can mix.
- One crate with modules: simple at the start, but no borders between parts and slower builds.
