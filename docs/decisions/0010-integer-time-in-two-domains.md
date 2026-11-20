# 0010. Keep time as integers in two domains

- Status: Accepted
- Date: 2026-11-19

## Context

A DAW measures time in two different ways. Musical time uses bars and beats and changes with the tempo. Audio time uses seconds and samples and does not change with the tempo. Floating-point positions collect rounding errors and are difficult to compare exactly.

## Decision

- Musical time is an integer number of ticks. Each beat has a fixed, high number of ticks.
- Audio time is an integer number of superclock units. One second has 282,240,000 units. All common sample rates divide this number exactly.
- A tempo map converts time from one domain to the other domain.
- Each clip locks to musical time or to audio time. A clip in musical time follows tempo changes. A clip in audio time stays at the same time.

## Consequences

- Positions do not collect rounding errors. Tests can compare positions exactly.
- A change of the sample rate does not move content.
- Each conversion between the domains goes through the tempo map and has tests.
- The number of ticks in each beat is not set yet. A later record sets it.

## Alternatives

- Beats as `f64`: natural for music, but rounding makes exact comparisons not reliable.
- Seconds as `f64`: simple, but musical content moves when the tempo changes.
- Samples only: exact for audio, but each change of tempo or sample rate must move all content.
