# 0026. Accept three crates with the BSL-1.0 and CC0-1.0 licenses for Iced

- Status: Accepted
- Date: 2026-11-20

## Context

Iced needs three crates with licenses that are not on the list of record 0019:

- `clipboard-win` and `error-code` have the BSL-1.0 license. Iced uses them for the clipboard on Windows.
- `hexf-parse` has the CC0-1.0 license. The GPU shader compiler of wgpu uses it.

The two licenses are permissive.

## Decision

- `cargo deny check` accepts BSL-1.0 for `clipboard-win` and `error-code` only.
- `cargo deny check` accepts CC0-1.0 for `hexf-parse` only.
- The list of record 0019 does not change. Each other crate with one of these licenses needs a new decision record.

## Consequences

- Sibilant can use Iced on all three operating systems.
- BSL-1.0 does not require a notice in a binary release.
- CC0-1.0 does not give patent rights.

## Alternatives

- Add BSL-1.0 and CC0-1.0 to the list: simpler, but new crates with these licenses come in without a review.
- Do not accept the crates: then Sibilant cannot use Iced.
