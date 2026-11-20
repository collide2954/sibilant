# 0017. Use test-driven development for all code

- Status: Accepted
- Date: 2026-11-19

## Context

Sibilant must be reliable. A test that comes before the code shows what the code must do. Tests also make sure that later changes do not break the code.

## Decision

All production code comes from test-driven development (TDD):

1. Write a test that fails.
2. Write the minimum code that makes the test pass.
3. Refactor the code. All tests must continue to pass.

## Consequences

- Each function has a test that shows its purpose.
- The design must make tests possible. Examples are pure functions, traits for devices, and user interface tests without a window (`iced_test`).
- Property tests (`proptest`) examine the rules of the time model and the project model.

## Alternatives

- Tests after the code: faster at the start, but tests often miss cases, and some code has no tests.
