# 0020. Write documentation in ASD-STE100 and keep comments to a minimum

- Status: Accepted
- Date: 2026-11-19

## Context

Users and contributors of Sibilant speak many languages. Clear and simple English is easier to read and to translate. Comments in code become incorrect when the code changes.

## Decision

- All documentation uses ASD-STE100 Simplified Technical English. This includes the README, the contributing guide, the decision records, and doc comments.
- Code contains few comments. Clear names, types, and tests explain the code.
- A comment tells why the code does a thing that is not clear. Examples are real-time limits and the conditions for `unsafe` code.
- Source files have no license header. The `LICENSE` and `NOTICE` files apply to all files.

## Consequences

- Each sentence is short and has one meaning.
- Each review examines the documentation for the STE rules.

## Alternatives

- Free English: faster to write, but more difficult to read and to translate.
- A license header in each file: usual in Apache projects, but it adds a comment to each file.
