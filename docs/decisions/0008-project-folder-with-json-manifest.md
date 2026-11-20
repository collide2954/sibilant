# 0008. Save projects as a folder with a JSON manifest

- Status: Accepted
- Date: 2026-11-19

## Context

Project files must be safe after a crash. They must also open correctly in later versions of Sibilant. Users must be able to examine and repair a project.

## Decision

A project is a folder with this structure:

```text
MySong.sibilant/
  project.json   format version, tempo map, tracks, clips, scenes, mixer
  audio/         audio files of the project
  autosave/      automatic copies of project.json
  cache/         waveform data that Sibilant can make again
```

- `project.json` contains a format version.
- Each new format version has a migration from the previous version. Each migration has tests.
- Sibilant writes each file to a temporary file first. Then it renames the temporary file to the final name. Because of this, a crash cannot leave a partial file.

## Consequences

- Text files are easy to compare in version control and easy to repair.
- The `cache` folder is safe to delete.
- Sibilant must keep the migrations for all old format versions.

## Alternatives

- One bundle file: easy to share, but slow to save and not safe in a crash.
- An SQLite database: safe and fast, but difficult for persons to read.
- A binary file: fast and small, but difficult to read and to migrate.
