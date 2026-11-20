# 0030. Count engine time in frames and control the engine through lock-free queues

- Status: Accepted
- Date: 2026-11-20

## Context

Records 0007 and 0011 set the borders of the engine and its real-time rules. The first version of the engine plays the transport and the metronome. It needs a clock, a method to find the events in each buffer, and a connection to the user interface.

## Decision

- The engine counts time in frames at the sample rate of the device. A frame is one sample for each channel. The engine uses the tempo map to convert between frames and musical time.
- A musical event belongs to the first frame at or after its exact time. Thus each event plays exactly one time, for all buffer sizes.
- The user interface sends commands to the engine through a lock-free queue (`rtrb`). The engine reads the commands at the start of each buffer.
- The engine publishes the playhead and the play state through atomic values. The user interface reads them at each frame of the display.
- The engine fills interleaved `f32` buffers. A test with `assert_no_alloc` proves that the engine does not allocate memory while it fills a buffer.
- `sibilant-device` opens the default output device of the system with its default configuration. If the device does not use `f32` samples, `sibilant-device` converts the samples.
- On Linux, the build needs the ALSA development files.

## Consequences

- Engine tests send commands and buffers directly, without an audio device.
- The playhead on the screen can be one display frame late. This delay is not audible.
- A change of the sample rate needs a new engine.

## Alternatives

- Superclocks in the engine: the same unit as the project, but each buffer needs a conversion for each frame.
- A mutex between the user interface and the engine: simple, but the audio thread can wait.
- A general channel crate: flexible, but some operations can allocate memory.
