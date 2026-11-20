# 0019. Accept only dependencies with permissive licenses

- Status: Accepted, with exceptions in [0026](0026-license-exceptions-for-iced.md) and BSD-1-Clause added by [0032](0032-bsd-1-clause-license.md)
- Date: 2026-11-19

## Context

Sibilant uses the Apache License 2.0. A dependency with a copyleft license, for example the GPL, can change the license terms of the Sibilant binary.

## Decision

- Sibilant accepts dependencies with these licenses only: Apache-2.0, MIT, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, and Unicode-3.0.
- `cargo deny check` makes sure that all dependencies obey this list. It also examines security advisories and accepts crates from crates.io only.
- To add a license to the list, write a new decision record.

## Consequences

- Crates with a GPL, LGPL, or AGPL license are not available to Sibilant.
- The MPL-2.0 license is not on the list. A decision about MPL-2.0 is necessary before Sibilant can use `symphonia` to decode audio files.

## Alternatives

- Also accept weak copyleft licenses (MPL-2.0, LGPL): more crates, but more obligations for each release.
