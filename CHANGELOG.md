# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-09-25

The first release that actually renames files. `0.0.1` printed the plan and
refused, with `renaming is not implemented yet (stub build)`.

### Added

- Renaming, with the three guarantees: no loss, idempotent, injective.
  Two names collapsing to one get an ordinal suffix rather than overwriting.
- `-r/--recursive`, deepest-first so a renamed parent never invalidates a
  pending child path. `--dirs-only` and `--files-only` narrow the walk.
- `--undo`, replaying the most recent run backwards. Every run is journalled
  before it happens, so an interrupted rename can still be unwound.
  The journal keeps the last 20 runs.
- Paths on stdin, with `-0/--null` for `find -print0`.
- A protect list for names whose correct kebab form breaks something —
  `Makefile`, `*.java`, `[slug].tsx`, bundle directories, Windows device
  names. Named on the command line they are renamed with a warning; swept up
  by `-r` they are skipped.
- Transliteration for Latin-extended, Cyrillic (BGN/PCGN) and Greek. Scripts
  that need a dictionary rather than a codepoint table are left verbatim.
- `--separator`, `--ascii`, `--max-length`, and `-f/--force`.
- Tracked files move with `git mv`, so staged state and rename detection
  survive.
- Library API: `Options`, `kebab_with`, `kebab_word_with`, `with_ordinal`,
  `DEFAULT_MAX_LENGTH`. `kebab` and `kebab_word` are unchanged.

### Fixed

- Case-only renames on case-insensitive filesystems go through a temporary
  name, instead of tripping a collision check against themselves.
- Accents are folded by table, not by decomposition — `Łódź` no longer
  degrades to `d`.
- Lowercasing is locale-independent, so a Turkish locale cannot turn `TITLE`
  into `tıtle`.
- Names are truncated on grapheme boundaries.
- Invalid-UTF-8 names are renamed anyway, with every dropped byte reported.

[0.1.0]: https://github.com/frycz/keb/releases/tag/v0.1.0
