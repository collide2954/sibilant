# 0004. Use progressive disclosure for all users

- Status: Accepted
- Date: 2026-11-19

## Context

Sibilant must be easy for new users and fast for professional users. A beginner mode and a professional mode make two applications to test, to document, and to teach.

## Decision

Sibilant has one interface with clear default settings. Sibilant shows advanced controls in context: in the inspector, in context menus, and when the user pushes a modifier key. Sibilant has no beginner mode and no professional mode.

## Consequences

- New users learn the same interface that professional users use.
- Each advanced control must have a clear place in the context where it applies.
- The documentation and the tests cover one interface.

## Alternatives

- A mode switch: easy to explain, but it doubles the work for tests and documentation.
- Professional interface first, with guided tours: powerful, but difficult for new users.
- Beginner interface first: simple, but professional functions can look like additions later.
