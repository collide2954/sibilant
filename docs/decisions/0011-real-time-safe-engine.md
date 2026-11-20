# 0011. Make the engine real-time safe and ready for parallel processing

- Status: Accepted
- Date: 2026-11-19

## Context

The audio thread must supply each buffer before its time limit. One late buffer causes a gap in the sound. Large projects need more than one CPU core.

## Decision

- The audio thread does not allocate or free memory, does not lock a mutex, and does not wait for input or output.
- The engine compiles the routing graph into a schedule of tasks with explicit dependencies.
- Version 0.1 runs the schedule on the audio thread only.
- A later version runs the same schedule on a pool of real-time worker threads.

## Consequences

- Engine tests use `assert_no_alloc` to prove that the audio thread does not allocate memory.
- Changes from the user interface go to the engine through lock-free queues.
- The audio thread sends old objects to a different thread, which frees them.
- Parallel processing needs no change to the schedule format.

## Alternatives

- Parallel processing from the first version: high performance early, but difficult to test while the engine is new.
- Serial processing only: simple, but large projects reach a CPU limit.
