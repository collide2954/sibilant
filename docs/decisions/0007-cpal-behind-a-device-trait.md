# 0007. Use cpal for audio devices behind a Sibilant trait

- Status: Accepted
- Date: 2026-11-19

## Context

Sibilant must use the native audio systems of each operating system:

- Windows: WASAPI and ASIO.
- macOS: Core Audio.
- Linux: ALSA, PipeWire, and JACK.

## Decision

- Sibilant uses the `cpal` crate for audio input and output.
- The `sibilant-device` crate puts `cpal` behind a Sibilant trait. Other crates do not use `cpal` directly.
- The engine does not know about devices. The `sibilant` application connects a device to the engine.
- ASIO support is an optional build feature. It is off by default.

## Consequences

- Engine tests give buffers to the engine directly. They do not need audio hardware.
- Application tests can use a simulated device that implements the trait.
- A different backend can replace `cpal` on one operating system without a change to the engine.
- The ASIO SDK uses a proprietary license or the GPLv3. A default Apache 2.0 build cannot include it. Before an official Windows release with ASIO, select the ASIO license and record the decision.
- WASAPI in shared mode has more latency than ASIO.

## Alternatives

- A native backend for each operating system: full control, but months of platform work before the first sound.
- PortAudio: used in many applications, but a C dependency on each platform.
