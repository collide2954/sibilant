# 0006. Use an immutable project and snapshot undo

- Status: Accepted
- Date: 2026-11-19

## Context

Undo and redo must be correct for all edits. In a usual design, each edit is a command with a reverse operation. Each reverse operation is a possible source of errors.

## Decision

- The project is an immutable value with structural sharing. Copies of a project use little memory because they share all parts that did not change.
- Each edit is a pure function. It receives the old project and returns a new project.
- The undo history keeps earlier snapshots of the project. Undo restores the previous snapshot.
- After each change, Sibilant compiles the project into a real-time copy for the engine.

## Consequences

- Each edit is easy to test. Give an input project and compare the output project with the expected project.
- There are no reverse operations to write or to test.
- Snapshots use more memory than commands. Structural sharing keeps this increase small.
- The engine never reads the project directly. It reads only the compiled copy.

## Alternatives

- Commands with reverse operations: less memory, but each command needs a correct reverse operation.
- Event sourcing: a full history, but more difficult code in all parts.
