# 0027. Stop on all unmaintained crates and accept two that Iced needs

- Status: Accepted
- Date: 2026-11-20

## Context

`cargo deny check` gave different results on the development computer and in CI. The two computers used different versions of cargo-deny, and these versions have different defaults for unmaintained crates. Also, the dependencies of Iced contain five crates with an "unmaintained" advisory.

## Decision

- `deny.toml` sets the rule for unmaintained crates explicitly: each unmaintained crate is an error, also when it is not a direct dependency.
- Sibilant does not use the Linux theme detection of Iced. This removes three unmaintained `zbus` crates.
- `cargo deny check` ignores these two advisories, because Iced needs the crates and they have no known vulnerability:
  - RUSTSEC-2024-0436: `paste`, for the Metal backend of wgpu on macOS.
  - RUSTSEC-2026-0192: `ttf-parser`, for the text engine of Iced.
- CI and the contributing guide use cargo-deny 0.22.0.

## Consequences

- A new advisory for an unmaintained crate stops CI. Then a person must remove the crate or record the reason to keep it.
- The development computer and CI give the same results.
- Sibilant does not follow the light or dark setting of Linux. Sibilant has its own themes.

## Alternatives

- Examine only the direct dependencies: fewer CI failures, but nobody sees the unmaintained crates in the tree.
- Use the newest cargo-deny on each computer: no updates by hand, but the results can be different.
