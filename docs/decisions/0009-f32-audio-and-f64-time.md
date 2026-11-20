# 0009. Use f32 for audio and f64 for time calculations

- Status: Accepted
- Date: 2026-11-19

## Context

Audio samples need a number format that is accurate, fast, and compatible with plugins.

## Decision

- Audio buffers use 32-bit floating-point numbers (`f32`).
- Calculations where errors increase over time use 64-bit floating-point numbers (`f64`). Examples are tempo calculations, conversions between time domains, automation curves, and long sums.

## Consequences

- `f32` is the native sample format of CLAP and VST3 plugins. Because of this, Sibilant does not convert samples at each plugin.
- `f32` buffers use half the memory bandwidth of `f64` buffers. SIMD instructions process twice as many `f32` samples at a time.
- The rounding noise of `f32` is much lower than the noise of real audio equipment.

## Alternatives

- `f64` for all audio: a little less rounding noise, but twice the memory bandwidth and a conversion at each plugin.
- Generic sample types: flexible, but generic code in all DSP types and tests.
