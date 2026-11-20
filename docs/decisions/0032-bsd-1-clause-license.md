# 0032. Accept the BSD-1-Clause license

- Status: Accepted
- Date: 2026-11-20

## Context

Record 0011 uses `assert_no_alloc` to prove that the audio thread does not allocate memory. This crate has the BSD-1-Clause license. The list of record 0019 does not contain this license.

## Decision

- Sibilant accepts dependencies with the BSD-1-Clause license.
- `deny.toml` contains BSD-1-Clause in the list of accepted licenses.

## Consequences

- The engine tests can use `assert_no_alloc`.
- Other crates with the BSD-1-Clause license can come in without a new record.
- BSD-1-Clause only requires that copies of the source code keep the copyright notice. It is more permissive than BSD-2-Clause and BSD-3-Clause, which are on the list.

## Alternatives

- An exception for `assert_no_alloc` only: narrower, but each other crate with this license needs a new record.
- An allocation checker in the test code: no new license, but it needs unsafe code, which the workspace lints deny.
