# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `--format=arrow|old|new|json|null`, choosing what each rename looks like on
  stdout. `json` and `null` are the only two that survive a filename containing a
  newline or a literal ` -> `, both of which are legal and both of which make the
  default `arrow` form ambiguous to parse — the same hole `-0` closes on input.
- `--absolute`, printing full paths. Lexical (`std::path::absolute`), so symlinks
  are not resolved: keb renames the link, not its target.
- A summary line on stderr, so a dry run is distinguishable from a real one. The
  dry run leads with `dry run, nothing changed (N renames planned)`; a real run
  opens with `Renaming...` and closes with `N files renamed`. Shown only when
  stderr is a terminal, so pipeline output is unchanged.
- `-d/--allow-dirs`, required before any directory is renamed. `keb dir1/dir2`
  is `keb dir1/dir2/file.png` with the Tab taken one stop early, and a directory
  rename breaks every path pointing into it, so it now takes a deliberate flag.
  A directory named as the root of a `-r` sweep is exempt — naming it was how
  the sweep was asked for.

### Changed

- `-r` without `-d` is a files-only sweep: filenames throughout the tree,
  directory names untouched. `-dr` renames both.
- Paths given as arguments or on stdin are sorted deepest-first, as a recursive
  sweep already was. `find -type d | keb` renamed a parent and then failed to
  find its children; so did `keb A A/B`. Order within one depth is unchanged, so
  collision suffixes still go to whichever colliding name was given first.
- `keb` with no paths at a terminal prints `--help` and exits 2, instead of the
  one-line `no paths given`. A pipeline is unaffected: with stdin redirected the
  paths still come from there, and an empty list is still the one-line error.

### Removed

- `--undo`, and the write-ahead journal behind it. A backstop that repairs a
  mistake is worth less than `-n`, which prevents it, and the journal cost a
  module, a state directory and a flush on every rename. **There is now no way to
  reverse a run** — `-n` first. The accepted loss is that an interrupted two-step
  case-only rename is no longer recoverable. `$XDG_STATE_HOME/keb/journal.tsv`
  and `%LOCALAPPDATA%\keb\journal.tsv` are no longer read or written, and can be
  deleted.
- `--dirs-only` and `--files-only`. `-r` without `-d` is what `--files-only`
  spelled, and `--dirs-only` is `-d` with no `-r` for one directory, or
  `find -type d | keb -d` for a tree. `--files-only` also never applied to the
  directory named on the command line, which it renamed anyway.

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
