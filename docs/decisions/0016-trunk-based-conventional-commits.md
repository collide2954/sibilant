# 0016. Push small commits to main with one-line Conventional Commits

- Status: Accepted
- Date: 2026-11-19

## Context

Sibilant has one maintainer at this time. Small and frequent changes are easy to examine and easy to revert.

## Decision

- Each commit contains one complete test-first cycle (refer to record 0017).
- Each commit message is one line in the Conventional Commits format, for example `feat(engine): add a sine oscillator`.
- A commit message has no body and no trailers.
- Contributors push commits directly to the `main` branch.
- The `main` branch must always pass all checks.

## Consequences

- The history is linear and easy to read.
- Tools can make a changelog from the commit messages.
- When more contributors work on Sibilant, the project can change to pull requests with required checks.

## Alternatives

- Short branches with pull requests: a safer `main` branch, but more work for each change.
- Area prefixes or plain messages: short, but tools cannot read them.
