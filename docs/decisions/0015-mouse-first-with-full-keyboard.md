# 0015. Make the mouse the main input and give full keyboard control

- Status: Accepted
- Date: 2026-11-19

## Context

New users start with the mouse. Professional users work faster with the keyboard. Sibilant must be good for the two groups.

## Decision

- The user can edit all content directly with the mouse.
- Each action is a named command.
- Each command can have a keyboard shortcut. The user can change each shortcut.
- A command palette finds each command by its name. Ctrl+K opens the palette on Windows and Linux. Cmd+K opens the palette on macOS.

## Consequences

- Menus, shortcuts, the command palette, and tests use the same list of commands.
- Keymaps that copy the shortcuts of other DAWs are possible later.

## Alternatives

- Modal keyboard control: very fast for experts, but difficult for new users.
- Mouse and touch only: easy, but slow for professional users.
