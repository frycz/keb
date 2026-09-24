# keb

Rename files to kebab case, without losing any.

```console
$ keb *.md
My File.md -> my-file.md
XMLHttpRequest.MD -> xml-http-request.md
Report (Final) [v2].md -> report-final-v2.md
```

> [!WARNING]
> **v0.0.1 is a preview.** The name transform works and is well tested; the renaming half is not implemented — `keb` prints what it would do and exits without touching anything. See [Status](#status).

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

  -n, --dry-run    Print the plan, change nothing
  -h, --help       Print help
  -V, --version    Print version
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
| `.gitignore` | `.gitignore` |
| `A1B2C3D4-E5F6.bin` | `a1b2c3d4-e5f6.bin` |
| `1080p`, `sha256`, `v1` | unchanged |

Two rules do most of the work:

**Only case transitions split words.** Digits never start a new word, which is what keeps hashes, versions, resolutions and checksum names intact. `Chapter10Part2` becomes `chapter10-part2`, not `chapter-10-part-2`.

**Extensions are preserved and lowercased.** Compound extensions come from a whitelist — `.tar.gz`, `.d.ts`, `.min.js` and friends survive whole. A leading dot marks a dotfile, not an extension, so `.gitignore` is left alone.

## Why not a one-line regex

Three guarantees, in priority order:

1. **No loss** — no file is destroyed, overwritten, or left unreachable
2. **Idempotent** — running it twice changes nothing the second time
3. **Injective** — two different files never collapse into one

The third can't come from the transform alone: `My File` and `my_file` both produce `my-file`. So the transform is deliberately not injective, and the filesystem layer restores injectivity by suffixing.

The rest is the long tail a regex gets wrong:

- On a case-insensitive filesystem, `File.md` → `file.md` is a same-inode rename, and a naive existence check reports a collision that isn't one
- The usual "decompose and strip accents" trick turns `Łódź.md` into `d.md`, because `ł` has no decomposition
- Lowercasing under a Turkish locale turns `TITLE` into `tıtle`
- `2024-01-02T10:30:00Z.log` → `2024-01-02-t10-30-00-z.log` is technically correct and practically vandalism
- `Makefile` → `makefile` breaks the build; so does renaming `MyClass.java`

[`cases.md`](https://github.com/frycz/keb/blob/main/cases.md) is the full specification — every case, every decision, and the reasoning.

## Status

**Working:** separator normalization, case-boundary splitting, extension handling, and the idempotency guarantee. Verified by unit tests, property tests and doctests.

**Not yet:**

| | |
|---|---|
| Actual renaming | `-n` output is correct; the rename itself is not wired up |
| Unicode folding | `Żółć.md` becomes `żółć.md`, not `zolc.md` |
| `-r` recursion | |
| `--undo` journal | |
| `--ascii`, `--separator` | |
| Protected names | `Makefile`, `*.java`, `[slug].tsx` are not yet guarded |

Until renaming lands, `keb` is useful for previewing a rename plan and as a library.

## As a library

The transform is a pure function with no filesystem access, so it's usable on its own:

```sh
cargo add keb --no-default-features
```

```rust
use keb::kebab;

assert_eq!(kebab("My File.md"), "my-file.md");
assert_eq!(kebab("XMLHttpRequest"), "xml-http-request");
assert_eq!(kebab("My.Archive.tar.gz"), "my-archive.tar.gz");
```

`--no-default-features` drops the CLI dependencies. Docs at [docs.rs/keb](https://docs.rs/keb).

## No config file

`keb` operates on arbitrary paths, often outside any project. A config file up the tree would mean the same command doing different things in different directories — which matters more here than for a formatter, because renames are destructive and one-shot. A dry run in one directory would stop predicting behavior in another.

The shell alias is the config file:

```sh
alias sn='keb --separator=_'
```

## License

MIT or Apache-2.0, at your option.
