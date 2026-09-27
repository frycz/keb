# keb

Rename files to kebab case.

```console
$ keb "My File.md"
My File.md -> my-file.md
```

Or anything that lists paths:

```console
$ find . -name '*.md' | keb
./docs/Getting Started.md -> ./docs/getting-started.md
./README Draft.md -> ./readme-draft.md
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
curl -sSf https://github.com/frycz/keb/releases/download/v0.1.0/keb-installer.sh | sh
```

Prebuilt binaries for macOS (Apple Silicon and Intel), Linux (x64 and ARM64) and Windows x64 are on the [releases page](https://github.com/frycz/keb/releases).

## Usage

```
keb [OPTIONS] <PATHS>...

  -n, --dry-run       Print the plan, change nothing
  -r, --recursive     Recurse into directories
  -d, --allow-dirs    Permit renaming directories. Without it, only files are renamed
  -f, --force         Overwrite on collision, and rename protected names under -r
      --undo          Undo the most recent run
  -0, --null          Paths on stdin are null-separated, for `find -print0`
      --separator     Emit this character between words instead of `-`
      --ascii         Narrow the output to ASCII
      --max-length    Override the 255 byte / UTF-16 unit name limit
```

## Examples

### Many files at once

Pass as many names as you like. Files already in kebab case print nothing at all.

```console
$ keb *
My File.md -> my-file.md
Report (Final) [v2].md -> report-final-v2.md
XMLHttpRequest.MD -> xml-http-request.md
```

Globs are expanded by your shell, not by keb, so anything your shell matches works — and matching follows your shell's rules, including its case sensitivity.

```console
$ keb *Photos*.png
Summer Photos 2024.png -> summer-photos-2024.png
```

Only the basename changes. Parent directories are never touched.

```console
$ keb "notes/My Meeting Notes.md"
notes/My Meeting Notes.md -> notes/my-meeting-notes.md
```

### Look before you leap: `-n`

`-n` prints exactly what would happen and changes nothing.

```console
$ keb -n *
My File.md -> my-file.md
Report (Final) [v2].md -> report-final-v2.md
```

### Changed your mind: `--undo`

`--undo` replays the most recent run backwards.

```console
$ keb "My File.md"
My File.md -> my-file.md
$ keb --undo
my-file.md -> My File.md
```

Every run is journalled *before* it happens, not after, so even an interrupted rename can be unwound. The journal lives in `$XDG_STATE_HOME/keb/journal.tsv` (`%LOCALAPPDATA%\keb\journal.tsv` on Windows) and keeps the last 20 runs. Undo restores names; it never destroys whatever has taken a name back since.

### Whole trees: `-r`

`-r` recurses, renaming filenames at every depth and leaving folder names alone.

```console
$ keb -r docs
docs/Guides/Getting Started.md -> docs/Guides/getting-started.md
docs/README Draft.md -> docs/readme-draft.md
```

Deepest paths are renamed first, so a renamed folder can never invalidate a path still queued beneath it.

### Directories: `-d`

Directories are left alone unless you ask for them, because `keb dir1/dir2` is `keb dir1/dir2/file.png` with the Tab taken one stop early — and renaming a directory breaks every path that points into it.

```console
$ keb "Photos/2024 Summer Trip"
keb: Photos/2024 Summer Trip: is a directory, skipped (-d allows directories renaming)

$ keb -d "Photos/2024 Summer Trip"
Photos/2024 Summer Trip -> Photos/2024-summer-trip
```

That is also how you normalise folder names while leaving load-bearing filenames alone — handy when sidecar files pair with their raw by basename. Combine it with `-r` to rename both:

```console
$ keb -dr Photos
Photos/2024 Summer Trip/IMG_1234.NEF -> Photos/2024 Summer Trip/img-1234.nef
Photos/2024 Summer Trip -> Photos/2024-summer-trip
Photos -> photos
```

### Collisions and protected names: `-f`

Two different names can produce the same kebab form. keb suffixes rather than overwrite, and the suffixed name is itself already kebab case:

```console
$ keb "My File.md" "my_file.md"
My File.md -> my-file.md
my_file.md -> my-file-2.md
```

`-f` overwrites instead, and says so:

```console
$ keb -f "My File.md" "my_file.md"
My File.md -> my-file.md
keb: my-file.md: overwriting
my_file.md -> my-file.md
```

A directory in the way is never overwritten, at any force level — keb falls back to suffixing and tells you.

`-f` also overrules the protect list. A protected name you type yourself is renamed with a warning, because you picked it; one found under `-r` is skipped, because you did not:

```console
$ keb -r src
keb: src/Makefile: skipped, a name a build tool looks up literally (-f renames it)
src/My Component.tsx -> src/my-component.tsx

$ keb src/Makefile
keb: src/Makefile: renaming a name a build tool looks up literally
src/Makefile -> src/makefile
```

### Different output: `--separator`, `--ascii`, `--max-length`

`--separator` swaps the word separator, which is how you get snake case:

```console
$ keb --separator=_ "My File.md"
My File.md -> my_file.md
```

By default, scripts are transliterated only where that is deterministic; CJK, Thai, Arabic, Hebrew and Indic are kept as they are. `--ascii` drops them:

```console
$ keb "日本語 Notes.md"
日本語 Notes.md -> 日本語-notes.md

$ keb --ascii "日本語 Notes.md"
日本語 Notes.md -> notes.md
```

Accented Latin, Cyrillic and Greek romanize either way, so `--ascii` changes nothing for them:

```console
$ keb "Café Ärger.md"
Café Ärger.md -> cafe-arger.md
```

`--max-length` tightens the length budget, counted in both bytes and UTF-16 code units over the whole filename. The stem absorbs the truncation, on a grapheme boundary, and the extension is kept whole:

```console
$ keb --max-length 20 "Quarterly Report On Widget Sales.pdf"
Quarterly Report On Widget Sales.pdf -> quarterly-report.pdf
```

Reach for it when the destination is tighter than your filesystem — eCryptfs caps names at 143, ISO/Joliet at 64, and Windows budgets 260 characters for the whole path.

## Output and exit codes

Renames print to **stdout** as `old -> new`, one per line. Warnings and errors go to **stderr**. So `keb x >/dev/null` is a quiet mode and `keb x 2>/dev/null` silences warnings, without either needing a flag.

Exit codes: `0` all good, `1` partial, `2` error. A single failed rename does not abort the run.

## Composition

Composes with anything that lists paths:

```sh
find . -name '*.md' | keb
find . -name '*.md' -print0 | keb -0
```

Use `-0` with `find -print0` in scripts. A newline is a legal character in a filename, so a newline-separated list is ambiguous — and the failure is silent, because the two halves of a split name may both match something else.

Paths are sorted deepest-first however they arrive, so `find -type d | keb -d` is safe even though `find` emits parents first.

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

**Scripts are transliterated only where that is deterministic.** Latin-extended, Cyrillic (BGN/PCGN) and Greek romanize to ASCII. CJK, Thai, Arabic, Hebrew and Indic are kept as they are, because 東京 → `tokyo` needs a dictionary, not a codepoint table.

## What it refuses to do

A name whose *correct* kebab form breaks something is protected. That is the whole list, in four groups:

**Names a build tool looks up literally.** `Makefile`, `GNUmakefile`, `CMakeLists.txt`, `Dockerfile`, `Containerfile`, `Gemfile`, `Rakefile`, `Brewfile`, `Procfile`, `Vagrantfile`, `Jenkinsfile`, `Justfile`, `Caddyfile`, `LICENSE`, `LICENCE`, `COPYING`, `NOTICE`, `CODEOWNERS`, `Info.plist`, `AndroidManifest.xml`.

**Syntax rather than a name**, matched by prefix. `[slug].tsx` a dynamic route segment, `(marketing)` a route group, `+page.svelte` a framework route file, `__init__.py` a dunder name, `_index.md` a section index, `._Report.md` an AppleDouble companion, `~$doc.docx` an Office lock file, `.#main.c` an Emacs lock file.

**Bundle directories**, matched by extension, in any case. `.app`, `.framework`, `.bundle`, `.rtfd`, `.xcodeproj`, `.xcworkspace`, `.playground`, `.kext`, `.plugin`, `.lproj`, `.docset` — the wrapper name is part of a contract with the metadata inside it.

**Four one-offs.** `*.java`, whose name must match its public class; `*.icloud`, a placeholder for a file not downloaded yet; `Icon\r`, the macOS custom-icon file; and the Windows reserved device names `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9` and `LPT1`–`LPT9`, with or without an extension, in any case. `COM0`, `COM10` and `CONFIG` are not reserved, and are renamed like anything else.

A name that transforms to nothing — `🚀.md`, `___.md` — is refused rather than turned into `untitled.md`, which would throw away the only thing that distinguished it.

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

## License

MIT
