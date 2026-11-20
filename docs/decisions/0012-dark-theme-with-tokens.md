# 0012. Use a dark graphite theme with one accent color

- Status: Accepted
- Date: 2026-11-19

## Context

Users work in Sibilant for many hours, frequently in dark rooms. The interface must be calm and clear. The content of the project must be more visible than the controls.

## Decision

- The default theme is dark, with neutral grey surfaces.
- One accent color shows focus, selection, and the playhead.
- Track colors identify the content of the project.
- A light theme is also available.
- All colors, corner radii, and spacing values are theme tokens. Users can make their own themes from these tokens.

## Consequences

- Each widget takes its visual values from the theme tokens. No widget contains a fixed color.
- The theme tokens are a public interface. A change to a token can break user themes.

## Alternatives

- A light theme first: bright and clean, but too bright for dark rooms.
- A bright and playful style: easy for new users, but not serious enough for professional users.
- A hardware style: familiar, but slow to draw and difficult to keep clean.
