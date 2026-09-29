# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] — 2026-09-29

### Added

- Shell completions (bash, zsh, fish, PowerShell, elvish) and a man page, in
  `completions/` and `man/` and in every release archive. They are generated from
  the clap definition and committed, so they ship in the source tarball where
  distro packagers look for them; `tests/generated.rs` fails when they drift.
- `.deb` and `.rpm` packages for x86_64 and aarch64 Linux on every release. They install the binary to `/usr/bin`, the man page, and the bash, zsh and fish completions, and they package the release's own binaries rather than a rebuild.
- The man page has an EXAMPLES section, built from the same text as the examples under `--help`, an EXIT STATUS section, and a DESCRIPTION that says more than the one-line summary.

### Changed

- Licensed MIT only. `Cargo.toml` declared `MIT OR Apache-2.0`, but the Apache
  license text was never shipped. Releases up to 0.3.0 keep the dual license.
- `--help` describes `--format json` with `...` instead of `…`, which the man page could not render outside a UTF-8 locale.

## [0.3.0] — 2026-09-28

### Added

- `--list-protected`, printing the names keb never renames and why, then exiting
  without renaming anything. The protect list is the tool's most surprising
  behaviour, and finding out why a file was skipped should not require the
  README. It goes to stdout, so `keb --list-protected | grep Makefile` works, and
  it is built from the same constants `protect::reason` matches on, so the list
  and the skip messages cannot drift apart.
- Examples under `-h` as well as `--help`, including the rule that matters: there
  is no undo, run `-n` first. A bare `keb` at a prompt prints the help, so this is
  the one place it reaches someone who has not read the README.

### Changed

- `no paths given` now ends with `(try --help)`. The one-liner is what a pipeline
  with an empty list gets, and it has to be recoverable without a second guess.

## [0.2.0] — 2026-09-27

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

[0.4.0]: https://github.com/frycz/keb/releases/tag/v0.4.0
[0.3.0]: https://github.com/frycz/keb/releases/tag/v0.3.0
[0.2.0]: https://github.com/frycz/keb/releases/tag/v0.2.0
[0.1.0]: https://github.com/frycz/keb/releases/tag/v0.1.0
