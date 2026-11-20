# Contribute to Sibilant

This document gives the rules for changes to Sibilant. All contributors must obey these rules.

## Prepare your computer

1. Install Rust with [rustup](https://rustup.rs).
2. Install cargo-deny:

   ```sh
   cargo install cargo-deny --locked --version 0.22.0
   ```

3. Activate the Git hooks of the repository:

   ```sh
   cargo xtask install-hooks
   ```

The Git hooks do these checks:

| Hook | Checks |
| --- | --- |
| `pre-commit` | Format and lints |
| `pre-push` | Tests |

Always let the hooks run. Do not use `--no-verify`.

## Write the test first

Sibilant uses test-driven development (TDD). Do these steps for each change:

1. Write a test for the change.
2. Run the test. Make sure that the test fails.
3. Write the minimum code that makes the test pass.
4. Run the tests. Make sure that all tests pass.
5. Refactor the code. Make sure that all tests pass again.

Write production code only when a test fails without it.

## Run the checks

Run all checks that the CI workflow runs:

```sh
cargo xtask ci
```

| Check | Command |
| --- | --- |
| Format | `cargo fmt --all --check` |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` |
| Tests | `cargo test --workspace` |
| Documentation | `cargo doc --workspace --no-deps`, with warnings as errors |
| Dependencies | `cargo deny check` |

The CI workflow runs `cargo xtask ci` on Linux, macOS, and Windows.

## Make commits

1. Run `cargo fmt --all`.
2. Make one commit for each complete test-first cycle. Keep each commit small.
3. Write the commit message on one line. Do not add a body or trailers.
4. Use the [Conventional Commits](https://www.conventionalcommits.org) format:

   ```text
   feat(engine): add a sine oscillator
   fix(ui): keep the playhead in the timeline
   test(core): add tests for the tempo map limits
   ```

5. Push the commit to the `main` branch.

The `main` branch must always pass all checks.

## Write code

- Keep comments to a minimum. Use clear names, types, and tests.
- Write a comment only to tell why the code does a thing that is not clear.
- Do not use `unsafe` code, except in crates that call C or operating system interfaces.
- On the audio thread, do not allocate or free memory, lock a mutex, or wait for input or output.

## Write documentation

- Write all documentation in ASD-STE100 Simplified Technical English.
- Record each design decision in [docs/decisions](docs/decisions/README.md).
- Write doc comments only for items that are not clear from their names and types.

## Add dependencies

Sibilant accepts only dependencies with permissive licenses. The `deny.toml` file contains the list of accepted licenses. Do not add a dependency with a GPL, LGPL, or AGPL license.

## Change the Rust version

1. Change `channel` in `rust-toolchain.toml`.
2. Change `rust-version` in `Cargo.toml` to the same version.
3. Correct all new lint warnings.
4. Make one commit for the change.
