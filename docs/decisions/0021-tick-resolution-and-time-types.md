# 0021. Use 705,600 ticks for each quarter note and typed time values

- Status: Accepted
- Date: 2026-11-19

## Context

Record 0010 keeps time as integers in two domains, but it does not set the number of ticks in each beat. The code also needs names and types for the two domains.

## Decision

- A beat is a quarter note. Tempo is the number of quarter notes in each minute.
- A beat has 705,600 ticks. This number is 2^6 × 3^2 × 5^2 × 7^2.
- The code calls the two domains musical time and clock time. Clock time is the "audio time" of record 0010.
- `Position<D>` and `Duration<D>` keep time values. `D` is the domain: `Musical` or `Clock`.
- The tempo map contains tempo changes at musical positions. Each change is immediate. Linear tempo ramps come later.
- A conversion between the domains rounds to the nearest unit. A value that is exactly between two units rounds away from zero.

## Consequences

- Triplets, quintuplets, septuplets, their nested forms, and notes to 1/256 are exact.
- One tick is shorter than one sample at all usual tempos and sample rates.
- MIDI files with 960 ticks for each quarter note import exactly.
- The compiler rejects the sum of two positions and the mix of the two domains.
- Time signatures change only the display of bars and beats, and the snap grid.

## Alternatives

- 960 or 1920 ticks for each beat: compatible with MIDI files, but septuplets are not exact.
- A beat that follows the time signature: closer to how some musicians count, but each MIDI, plugin, and tempo conversion must translate.
- Four concrete types: the same safety, but the arithmetic is written two times.
- One integer for each domain: less code, but the compiler accepts the sum of two positions.
