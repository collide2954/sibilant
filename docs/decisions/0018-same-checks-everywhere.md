# 0018. Run the same checks on each computer and in CI

- Status: Accepted
- Date: 2026-11-19

## Context

Checks that run only in CI find problems late. Checks that are different on each computer give different results.

## Decision

- The `rust-toolchain.toml` file sets one Rust version for all contributors and for CI. A change of the version is a separate commit.
- The `xtask` crate contains the checks: format, lints, tests, documentation, and dependency policy.
- The Git hooks in `.githooks` run `cargo xtask pre-commit` before each commit and `cargo xtask pre-push` before each push.
- The CI workflow runs `cargo xtask ci` on Linux, macOS, and Windows.
- The workspace lints include the Clippy `pedantic` group. In the checks, a warning is an error.
- Code must not use `unsafe`, except in crates that call C or operating system interfaces.

## Consequences

- The same commands run on each computer and in CI.
- The checks are Rust code with tests. They do not depend on a shell.
- Each commit takes a little more time because the hooks run the checks.

## Alternatives

- Hooks that run all tests before each commit: a stronger gate, but slower commits when the number of tests increases.
- CI only: no local cost, but broken commits can reach `main`.
