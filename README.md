# keb

Rename files to kebab case — guaranteeing no file is ever lost or overwritten, that
renaming twice changes nothing, and that two distinct files never collapse into one.

```console
$ keb "My File.md" "XMLHttpRequest.MD" "Report (Final) [v2].md"
My File.md -> my-file.md
XMLHttpRequest.MD -> xml-http-request.md
Report (Final) [v2].md -> report-final-v2.md
```

> **Status: early.** The transform handles separators, case boundaries and
> extensions; Unicode folding and the filesystem layer are in progress. Not yet
> recommended for files you care about.

## Install

```sh
brew install frycz/tap/keb        # macOS, Linux
npm  install -g @frycz/keb        # anywhere with Node
cargo install keb                 # from source
```

## Why it is not a one-line regex

Three invariants, in priority order:

1. **No loss** — no file is ever destroyed, overwritten, or left unreachable.
2. **Idempotent** — `keb(keb(x)) == keb(x)`, always.
3. **Injective** — two distinct files never end up at the same path.

Invariant 3 cannot be satisfied by the transform alone: `My File` and `my_file` both
produce `my-file`. The transform is deliberately *not* injective; the filesystem
layer restores injectivity by suffixing.

The rest is the long tail that a regex gets wrong — case-insensitive filesystems
where `File.md` → `file.md` needs a two-step rename, `Łódź.md` becoming `d.md` under
the usual NFD-and-strip-marks trick, lowercasing under a Turkish locale turning
`TITLE` into `tıtle`, `2024-01-02T10:30:00Z.log` being shredded into
`2024-01-02-t10-30-00-z.log`, and `Makefile` → `makefile` quietly breaking the build.

[`cases.md`](https://github.com/frycz/keb/blob/main/cases.md) is the full
specification: every case, every resolved decision, and the reasoning behind each.

## Usage

```
keb [OPTIONS] <PATHS>...

  -n, --dry-run     Print the plan, change nothing
  -r                Recurse into directories
  -f, --force       Overwrite on collision; rename protected names under -r
      --undo        Replay the journal backwards
      --ascii       Narrow output to strict [a-z0-9-]
      --separator   Emit a different separator
```

Only the basename is renamed; parent directories are untouched. Renames go to
**stdout** as `old -> new`, warnings and errors to **stderr**, so `keb x >/dev/null`
and `keb x 2>/dev/null` serve as `--quiet` and `--no-warn`. Exit codes: `0` ok,
`1` partial, `2` error.

Composes with `find`:

```sh
find . -name '*.md' -print0 | keb -0
```

## As a library

The transform is a pure `String -> String` function with no filesystem access,
published separately from the CLI:

```sh
cargo add keb --no-default-features
```

```rust
use keb::kebab;

assert_eq!(kebab("My File.md"),     "my-file.md");
assert_eq!(kebab("XMLHttpRequest"), "xml-http-request");
assert_eq!(kebab("My.Archive.tar.gz"), "my-archive.tar.gz");
```

`--no-default-features` drops the CLI dependencies and leaves only the transform.

## Design notes

No config file. `keb` operates on arbitrary paths, often outside any project, so a
config file up the tree would mean the same command doing different things in
different directories — and that matters more here than for a formatter, because
renames are destructive and one-shot. A dry-run in one directory would stop
predicting behavior in another.

The shell alias is the config file:

```sh
alias sn='keb --separator=_'
```

## License

MIT or Apache-2.0, at your option.
