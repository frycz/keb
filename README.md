# keb

Rename files to kebab case.

```console
$ keb *.md
My File.md -> my-file.md
XMLHttpRequest.MD -> xml-http-request.md
Report (Final) [v2].md -> report-final-v2.md
```

## Install

```sh
brew install frycz/tap/keb
```

```sh
npm install -g @frycz/keb
```

```sh
cargo install keb
```

```sh
curl -sSf https://github.com/frycz/keb/releases/download/v0.0.1/keb-installer.sh | sh
```

Prebuilt binaries for macOS (Apple Silicon and Intel), Linux (x64 and ARM64) and Windows x64 are on the [releases page](https://github.com/frycz/keb/releases).

## Usage

```
keb [OPTIONS] <PATHS>...

  -n, --dry-run       Print the plan, change nothing
  -r, --recursive     Recurse into directories
  -f, --force         Overwrite on collision, and rename protected names under -r
      --undo          Undo the most recent run
  -0, --null          Paths on stdin are null-separated, for `find -print0`
      --dirs-only     Under -r, visit directories only
      --files-only    Under -r, visit files only
      --separator     Emit this character between words instead of `-`
      --ascii         Narrow the output to ASCII
      --max-length    Override the 255 byte / UTF-16 unit name limit
```

Only the basename changes — parent directories are never touched.

```console
$ keb "notes/My Meeting Notes.md"
notes/My Meeting Notes.md -> notes/my-meeting-notes.md
```

Renames print to **stdout** as `old -> new`, one per line. Warnings and errors go to **stderr**. So `keb x >/dev/null` is a quiet mode and `keb x 2>/dev/null` silences warnings, without either needing a flag.

Files that are already kebab case print nothing at all.

Exit codes: `0` all good, `1` partial, `2` error. A single failed rename does not abort the run.

Composes with anything that lists paths:

```sh
find . -name '*.md' | keb
find . -name '*.md' -print0 | keb -0
```

Changed your mind:

```console
$ keb --undo
my-meeting-notes.md -> My Meeting Notes.md
```

## What it does to a name

| Input | Output |
|---|---|
| `My File.md` | `my-file.md` |
| `my___file.md` | `my-file.md` |
| `myFileName.txt` | `my-file-name.txt` |
| `XMLHttpRequest.MD` | `xml-http-request.md` |
| `Report (Final) [v2].md` | `report-final-v2.md` |
| `My.Archive.tar.gz` | `my-archive.tar.gz` |
| `Żółć Ćma.md` | `zolc-cma.md` |
| `Æther Straße.md` | `aether-strasse.md` |
| `Привет.md` | `privet.md` |
| `日本語 Report.md` | `日本語-report.md` |
| `Tom & Jerry.md` | `tom-and-jerry.md` |
| `C++ Notes.md` | `cpp-notes.md` |
| `📁 Report 🚀.md` | `report.md` |
| `.gitignore` | `.gitignore` |
| `A1B2C3D4-E5F6.bin` | `a1b2c3d4-e5f6.bin` |
| `v1.2.3-Release.zip` | `v1.2.3-release.zip` |
| `1080p`, `sha256`, `v1` | unchanged |

Three rules do most of the work:

**Only case transitions split words.** Digits never start a new word, which is what keeps hashes, versions, resolutions and checksum names intact. `Chapter10Part2` becomes `chapter10-part2`, not `chapter-10-part-2`.

**Extensions are preserved and lowercased.** Compound extensions come from a whitelist — `.tar.gz`, `.d.ts`, `.min.js` and friends survive whole. A leading dot marks a dotfile, not an extension, so `.gitignore` is left alone. A dot between digits is a version segment, not a separator, so `v1.2.3` and `192.168.1.1` survive.

**Scripts are transliterated only where that is deterministic.** Latin-extended, Cyrillic (BGN/PCGN) and Greek romanize to ASCII. CJK, Thai, Arabic, Hebrew and Indic are kept as they are, because 東京 → `tokyo` needs a dictionary, not a codepoint table. `--ascii` narrows the output to `[a-z0-9-]` and drops the rest.

## What it refuses to do

A name whose *correct* kebab form breaks something is protected: `Makefile`, `Dockerfile`, `CMakeLists.txt`, `LICENSE`, `*.java`, `[slug].tsx`, `+page.svelte`, `__init__.py`, `Foo.app`, `CON`, and the rest of the list in [`design/cases.md`](https://github.com/frycz/keb/blob/main/design/cases.md#13-semantic-landmines).

Naming one on the command line renames it anyway, with a warning — you picked it. Finding one under `-r` skips it, because you did not:

```console
$ keb -r src
keb: src/Makefile: skipped, a name a build tool looks up literally (-f renames it)
src/My Component.tsx -> src/my-component.tsx
```

A name that transforms to nothing — `🚀.md`, `___.md` — is refused rather than turned into `untitled.md`, which would throw away the only thing that distinguished it.

## Why not a one-line regex

Three guarantees, in priority order:

1. **No loss** — no file is destroyed, overwritten, or left unreachable
2. **Idempotent** — running it twice changes nothing the second time
3. **Injective** — two different files never collapse into one

The third can't come from the transform alone: `My File` and `my_file` both produce `my-file`. So the transform is deliberately not injective, and the filesystem layer restores it by suffixing:

```console
$ keb "My File.md" "my_file.md"
My File.md -> my-file.md
my_file.md -> my-file-2.md
```

The rest is the long tail a regex gets wrong:

- On a case-insensitive filesystem, `File.md` → `file.md` is a same-inode rename, and a naive existence check reports a collision that isn't one. `keb` compares inodes and goes through a temporary name
- The usual "decompose and strip accents" trick turns `Łódź.md` into `d.md`, because `ł` has no decomposition
- Lowercasing under a Turkish locale turns `TITLE` into `tıtle`
- `2024-01-02T10:30:00Z.log` → `2024-01-02-t10-30-00-z.log` is technically correct and practically vandalism
- `Makefile` → `makefile` breaks the build; so does renaming `MyClass.java`
- Inside a git repository a tracked file moves with `git mv`, so staged state and rename detection survive
- A filename on Linux is a byte string, not text; an invalid-UTF-8 name is renamed anyway, and every dropped byte is reported

[`design/cases.md`](https://github.com/frycz/keb/blob/main/design/cases.md) is the full specification — every case, every decision, and the reasoning.

## Undo

Every run is journalled before it happens, not after, so an interrupted two-step rename can still be unwound. `--undo` replays the most recent run backwards:

```console
$ keb -r docs
docs/Getting Started.md -> docs/getting-started.md
$ keb --undo
docs/getting-started.md -> docs/Getting Started.md
```

The journal lives in `$XDG_STATE_HOME/keb/journal.tsv` (`%LOCALAPPDATA%\keb\journal.tsv` on Windows) and keeps the last 20 runs.

## As a library

The transform is a pure function with no filesystem access, so it's usable on its own:

```sh
cargo add keb --no-default-features
```

```rust
use keb::{kebab, kebab_with, Options};

assert_eq!(kebab("My File.md"),        "my-file.md");
assert_eq!(kebab("XMLHttpRequest"),    "xml-http-request");
assert_eq!(kebab("My.Archive.tar.gz"), "my-archive.tar.gz");

let snake = Options { separator: '_', ..Options::default() };
assert_eq!(kebab_with("My File.md", &snake), "my_file.md");
```

An empty return value means there is no kebab name for that input — the caller should leave the file alone.

`--no-default-features` drops the CLI dependencies. Docs at [docs.rs/keb](https://docs.rs/keb).

## No config file

`keb` operates on arbitrary paths, often outside any project. A config file up the tree would mean the same command doing different things in different directories — which matters more here than for a formatter, because renames are destructive and one-shot. A dry run in one directory would stop predicting behavior in another.

The shell alias is the config file:

```sh
alias sn='keb --separator=_'
```

## License

MIT or Apache-2.0, at your option.
