# keb — spec & case matrix

## The job

> Given any filesystem path, rename its basename to kebab case — guaranteeing no file is ever lost or overwritten, that renaming twice changes nothing, and that two distinct files never collapse into one.

Three invariants, in priority order:

1. **No loss** — no file is ever destroyed, overwritten, or left unreachable.
2. **Idempotent** — `keb(keb(x)) == keb(x)`, always.
3. **Injective** — two distinct files never end up at the same path.

Invariant 3 cannot be satisfied by the transform alone: `My File` and `my_file` both produce `my-file`. The pure transform is deliberately **not** injective; the filesystem layer restores injectivity by suffixing. This is the seam the whole design rests on.

## Non-goals

The tool does **not**:

- touch file contents
- update inbound references (links, imports, `<img src>`) — different blast radius, different undo story
- decide *which* files to act on beyond `-r` and its filters — that is `find`'s job, and stdin composes
- renumber, zero-pad, or reformat dates
- deduplicate files
- read a config file or environment variables — see "Why no config file" below

## Architecture: two layers

| Layer | Type | Dominant quality | Testing |
|---|---|---|---|
| **Transform** — `String → String`, pure | Data transformer | Correctness, totality | Property tests + fuzz over arbitrary bytes |
| **Filesystem** — plan, journal, rename | Stateful tool | Safety | Atomicity, reversibility, TOCTOU |

Sections 1–5 and 9–11 specify the transform. Sections 6–7 and 12 specify the filesystem layer. Keep them separately testable.

## Properties (the Z invariant, as tests)

- `keb(keb(x)) == keb(x)` for all `x`
- `keb` never throws, for any byte string — including invalid UTF-8
- output is a valid filename on every target filesystem
- every rename is reconstructible from the journal
- distinct input paths never resolve to the same output path

## Every case the tool should handle

Grouped so behavior can be agreed per group.

## 1. Separator normalization

| Input | Output |
|---|---|
| `My File.md` | `my-file.md` |
| `my___file.md` | `my-file.md` |
| `my   file.md` (runs) | `my-file.md` |
| `my - file.md` | `my-file.md` |
| `my–file.md` (en/em dash) | `my-file.md` |
| `my\tfile.md`, NBSP ` `, ` `, zero-width `​` | `my-file.md` |
| `-my-file-.md` | `my-file.md` (trim edges) |

## 2. Case boundaries

| Input | Output | Note |
|---|---|---|
| `myFileName` | `my-file-name` | camel |
| `MyFileName` | `my-file-name` | Pascal |
| `MY_FILE_NAME` | `my-file-name` | screaming snake |
| `HTTPServer` | `http-server` | acronym → word boundary |
| `parseJSONData` | `parse-json-data` | acronym in middle |
| `XMLHttpRequest` | `xml-http-request` | |
| `IPv6Address` | `ipv6-address` *or* `ip-v6-address` | **decision** |
| `Chapter10Part2` | `chapter-10-part-2` | letter↔digit boundary |
| `iPhone14Pro` | `iphone14-pro` | decisions 1 + 13 |
| `1080p`, `4K`, `H264` | keep as-is? | **decision** — splitting gives `1080-p` |
| `file2go` | `file2go` or `file-2-go` | **decision** — lowercase→digit usually *not* a boundary |

## 3. Extensions & dots

| Input | Output |
|---|---|
| `My File.MD` | `my-file.md` (lowercase ext — **decision**) |
| `My.Archive.tar.gz` | `my-archive.tar.gz` (multi-part ext whitelist) |
| `Component.Test.tsx` | `component.test.tsx` or `component-test.tsx` — **decision** |
| `types.d.ts`, `app.min.js`, `file.spec.js` | preserve compound ext |
| `.gitignore`, `.env.Local` | `.gitignore`, `.env.local` (dotfile ≠ extension) |
| `README` (no ext) | `readme` |
| `My.File.Name.md` | `my-file-name.md` (interior dots → `-`) |
| `v1.2.3-Release.zip` | `v1.2.3-release.zip` — version dots must survive |
| `Backup.` (trailing dot) | `backup` |
| `.` / `..` | refuse |

## 4. Unicode

| Input | Output |
|---|---|
| `Żółć Ćma.md` | `zolc-cma.md` (deburr) |
| `Café Übung.md` | `cafe-ubung.md` |
| `Æther Straße.md` | `aether-strasse.md` (special folds) |
| `Привет.md` / `日本語.md` | transliterate / keep / strip — **decision** |
| `📁 Report 🚀.md` | `report.md` (strip emoji) |
| NFD vs NFC (macOS hands back decomposed `e` + `́`) | normalize before matching |
| Homoglyphs, RTL marks, `‮` | strip control/format chars |

## 5. Punctuation & symbols

| Input | Output |
|---|---|
| `Tom & Jerry.md` | `tom-and-jerry.md` (`&`→`and`) or `tom-jerry.md` — **decision** |
| `Don't Stop.md` | `dont-stop.md` (apostrophe deleted, not dashed) |
| `Report (Final) [v2].md` | `report-final-v2.md` |
| `50% Off, $20.md` | `50-off-20.md` or `50-percent-off-20-dollars` — **decision** |
| `C++ Notes.md`, `C#.md` | `c-notes.md` loses meaning — **decision** (maybe `cpp`, `csharp`) |
| `@user #tag.md` | `user-tag.md` |
| `a/b` (slash inside name) | refuse or → `-` |
| `file:name`, `file*?<>\|"` | strip (invalid on Windows / network volumes) |
| Newline / `\r` in filename | strip |

## 6. Path semantics

- `keb Pth_to-file.md` → only the **basename** is renamed; `Pth_to/` parent untouched.
- `keb "./A Dir/My File.md"` → `./A Dir/my-file.md`.
- Directories: rename the dir itself; `-r` to recurse into contents. Rename deepest-first so paths stay valid.
- Multiple args + shell globs: `keb *.md`.
- `-` / stdin list piped in (`find … | keb`).
- Filename starting with `-` → `--` handling so it isn't parsed as a flag.
- Absolute vs relative vs `~`.

## 7. Collisions & filesystem reality

- **Case-insensitive FS (macOS default):** `File.md` → `file.md` is a same-inode rename; a naive `exists()` check falsely reports a collision. Needs a two-step rename via a temp name.
- Target exists and is a *different* file → skip / `--force` / auto-suffix `-2`.
- Two inputs collapsing to one output: `My File.md` + `my_file.md`.
- Name becomes empty after cleaning: `___.md`, `🚀.md` → fallback (`untitled`) or refuse.
- 255-byte `NAME_MAX` — deburring/transliteration can *lengthen* a name.
- Symlinks (rename the link, not the target), broken symlinks, hardlinks.
- Read-only dir, no write permission, file open/locked, cross-device.
- Windows reserved names: `CON`, `PRN`, `NUL`, `COM1`, `LPT1`.
- Git-tracked files → use `git mv` when inside a repo? **decision**.
- **Idempotency:** running `keb x` twice must be a no-op. Non-negotiable.

## 8. CLI surface

`keb FILE...` with no flags is the whole tool for the common case. Everything below serves the tail.

### The test a flag must pass

> A flag earns its place if it expresses intent the tool cannot infer, **and** it does not change what "correct" means.

Flag *count* is a symptom, not the disease — `rg` has ~90 flags and is a deep tool because `rg pattern` needs none of them. The real cost driver is **where in the section 9 pipeline a flag sits**: flags at the edges are cheap, flags in the middle multiply the case matrix.

| Flag | Pipeline step | Cases it interacts with | Cost |
|---|---|---|---|
| `--separator` | 10 (emit) | zero — identical 14 sections, one char differs at the end | cheap |
| `--ascii` | 10 (whitelist) | one, and it is the output contract | cheap |
| `-n`, `-r`, `-f`, `--undo` | filesystem layer | none — never touches the transform | cheap |
| `--no-lowercase` | 8 | invalidates the output contract; result is not kebab | rejected |
| `--no-transliterate` | 4–6 | forks the deep core, doubles every Unicode case | rejected |

### Transform layer

- `--separator=X` — emit `X` instead of `-`. Same algorithm, one character at step 10. Not advertised in the first line of `--help`, since the tool is named `keb`.
- `--ascii` — narrow the output alphabet to strict `[a-z0-9-]`, transliterating or dropping everything else. The one genuine fork in the output contract (decision 2).
- `--max-length=N` — override the length inferred from the target filesystem.

### Filesystem layer

- `-n` / `--dry-run` — print the plan, change nothing
- `-r` — recurse; `--dirs-only` / `--files-only` filter that traversal
- `-f` — force: overwrite on collision, and rename protected names under `-r`
- `--undo` — replay the journal backwards
- `-0` — null-separated paths on stdin, for `find -print0`

### Stream contract

- Silent on success except the renames themselves: `old -> new` on **stdout**, one per line
- Warnings and errors on **stderr**
- Exit codes: `0` ok / `1` partial / `2` error. A failed rename does not abort the run; the tool continues and exits non-zero at the end.

This split *is* `--verbose`/`--quiet`, done by convention — `keb x >/dev/null` and `keb x 2>/dev/null` cover both, so neither flag ships.

### Deliberately rejected

| Flag | Reason |
|---|---|
| `--no-lowercase` | Indecision — the output stops being kebab case |
| `--no-transliterate` | Forks the deep core and doubles the Unicode matrix |
| `--no-git` | The inference is never wrong: `git mv` on a tracked file is strictly better |
| `--keep-ext-case` | Nobody wants `.JPG` |
| `--no-warn` | That is `2>/dev/null` |
| `--verbose` / `--quiet` | The stdout/stderr split already is this |
| `-i` | `-n`, read the plan, then run it |
| `--check-references` | **Blast radius** — would edit the contents of arbitrary other files, breaking the scope of invariant 1 |

### Why no config file

Prettier's config file works because the repo is the unit of convergence — everyone formatting that repo should agree. `keb` has no equivalent scope: it operates on arbitrary paths, often outside any project. A config file up the tree would mean **the same command does different things in different directories**, which breaks "no hidden state" — and matters far more here than for a formatter, because renames are destructive and one-shot. A dry-run in one directory would stop predicting behavior in another.

Environment variables have the same defect with worse discoverability.

The shell alias is the config file:

```sh
alias sn='keb --separator=_'
```

Visible, versioned by the user, zero hidden state in the tool.

## 9. Pipeline order

Order is load-bearing. The tool does **not** enumerate characters — it transforms aggressively, then whitelists `[a-z0-9-]`, so unknown codepoints fall into a defined fallback instead of needing a table entry.

1. Split basename/extension **first** — dot rules are decided on the original string
2. Strip bidi / format / control characters
3. **NFKC** — folds fullwidth, math alphanumerics, ligatures, fractions, circled forms
4. **Special folds, before NFD** — `ß→ss`, `ł→l`, `ø→o`, `đ→d`, `þ→th`, `ħ`, `ŋ`, `ı`, `æ→ae`, `œ→oe`
5. NFD + strip combining marks (category `Mn`)
6. Script transliteration for whatever is left
7. **Word-boundary split** (camel / acronym / digit)
8. **Then** lowercase — locale-invariant
9. Symbol expansion (`&`→`and`)
10. Strip remaining non-`[a-z0-9]`, collapse runs, trim edges
11. Truncate — grapheme-safe, byte-aware, extension-preserving
12. Reserved/empty guard → collision resolution → rename

Two traps baked into this order:

- **Lowercasing before step 7 destroys camelCase.** It must come after boundary detection.
- **`ł ø đ þ ı ħ ŋ` have no NFD decomposition.** The standard "NFD + strip marks" deburring trick passes them through untouched, and step 10 then deletes them: `Łódź.md` → `d.md`. The explicit fold table at step 4 is what prevents this.

## 10. Language-specific hazards

- **Turkish `I` / `İ`** — `"TITLE".toLowerCase()` under a `tr` locale yields `tıtle`. Always lowercase locale-invariant, never with the system locale.
- **Greek final sigma** — `Σ` lowercases to `ς` or `σ` depending on word position.
- **Ligatures** — `ﬁ ﬀ ﬃ` (U+FB0x) fold only under NFKC, not NFC.
- **Cyrillic** has competing romanizations (ISO 9 / BGN+PCGN / GOST): `щ` = `shch` or `šč`; Ukrainian `г`=`h` vs Russian `г`=`g`. Pick one standard and document it.
- **CJK has no word boundaries**, and romanization needs a dictionary — 東京 → `tokyo` is not derivable from codepoints. Kanji readings are ambiguous.
- **Hangul** — NFD explodes syllables into Jamo. Guard against decomposing it.
- **Thai / Khmer / Lao** — no spaces between words at all.
- **Indic** — ZWJ/ZWNJ are semantically meaningful (क्ष); stripping them changes the word.
- **Vietnamese** — stacked diacritics (`ế`) plus non-decomposable `đ`.
- **German** — `ß→ss`, capital `ẞ` (U+1E9E). **Nordic** — `å→a` vs Danish `aa` convention.

## 11. Unicode trickery

- **Fullwidth forms** — `Ｆｉｌｅ．ｔｘｔ`; U+FF0E fullwidth period is *not* an extension separator.
- **Math alphanumerics** — `𝓗𝓮𝓵𝓵𝓸`, `𝟏𝟐𝟑` ("fancy text"). NFKC handles them; without NFKC they are stripped entirely.
- **Vulgar fractions** — `⅓` NFKC-expands to `1⁄3` using U+2044 fraction slash, which *looks* like a path separator but is not.
- **Roman numerals** `Ⅻ`, circled `①`, parenthesized `⒜`, superscripts `²`.
- **Non-ASCII digits** — Arabic-Indic `٣`, Devanagari `३`, fullwidth `３`. Map to `3` or strip? **decision**
- **Invisible characters** — soft hyphen U+00AD, ZWSP U+200B, ZWNJ, BOM U+FEFF at position 0, word joiner U+2060, non-breaking hyphen U+2011.
- **Unicode spaces** beyond NBSP — U+2000–200A, U+202F narrow NBSP, U+3000 ideographic space.
- **Emoji are not single characters** — ZWJ sequences (👨‍👩‍👧), skin-tone modifiers, flags (regional-indicator pairs), variation selector U+FE0F. Edgiest: **keycap `1️⃣.png`** — stripping FE0F + U+20E3 leaves `1.png`, which may collide with an existing `1.png`.
- **Zalgo** — hundreds of combining marks on one base; also a length-blowup vector.
- **Bidi override in filenames** — a name with U+202E renders as `photo.jpg` while being something else. Strip, and warn.
- **Invalid UTF-8 filenames.** On Linux a filename is an arbitrary byte string, not text. Latin-1 names from old archives are common and will crash a naive decoder — needs surrogateescape / lossy decoding.

## 12. Deeper filesystem reality

- **`NAME_MAX` units differ** — 255 *bytes* on ext4, 255 *UTF-16 code units* on APFS/NTFS. Truncation must not split a codepoint or a grapheme cluster, and must not eat the extension.
- **Normalization behavior differs** — HFS+ stores NFD, APFS is normalization-*insensitive*, ext4 stores raw bytes. So on Linux, `café` (NFC) and `café` (NFD) are **two different files in one directory** that collapse to a single kebab name.
- **Windows / SMB silently strip trailing dots and spaces** — the rename becomes a no-op or targets a different file.
- **Companion files** — `._file` (AppleDouble), `~$doc.docx` (Office lock), `.#file` (emacs), `file.icloud` stubs. Renaming one of a pair breaks the pairing.
- **`Icon\r`** — a real macOS file whose name ends in a literal carriage return.
- **Bundle directories** — `Foo.app`, `Bar.framework`, `.rtfd`. The directory name is part of a contract with `Info.plist`; renaming breaks the bundle. Refuse or warn.
- **Atomicity** — `rename()` is atomic, but the two-step case-only rename is not. A crash leaves the temp name behind, so the undo journal must be write-**ahead**, not write-after.
- **TOCTOU** — a file can move or vanish between plan and execute; dry-run output can lie.
- **Recursion hazards** — symlink loops, mount-point crossing, mutating a directory while iterating it (collect the full list first, rename deepest-first).
- **Immutable flags** (`chflags uchg`), SIP-protected paths, cloud placeholder/dataless files that download on access.
- **FAT32 / exFAT** volumes — no `:`, case-insensitive, legacy 8.3 baggage.

## 13. Semantic landmines

Correct kebab transforms that break builds. Highest-value category — most of the pain in this tool is not Unicode, it is confidently renaming something that was named that way on purpose.

- **`Makefile` → `makefile`** breaks `make` on case-sensitive filesystems. Same class: `Dockerfile`, `Gemfile`, `Rakefile`, `CMakeLists.txt`, `LICENSE`, `CODEOWNERS`, `Info.plist`, `AndroidManifest.xml`.
- **`MyClass.java` → `my-class.java` does not compile.** Java requires filename == public class name.
- **Framework-significant punctuation** — Next.js `[slug].tsx`, `[...slug].tsx`, `(group)/`; SvelteKit `+page.svelte`, `+layout.server.ts`; Python `__init__.py`. Stripping brackets / plus / underscores breaks routing or imports outright.
- **Hashes and UUIDs** — `A1B2C3D4-E5F6.bin`; digit-boundary splitting would shred these into `a1-b2-c3-d4`.
- **Timestamps** — `2024-01-02T10:30:00Z.log` → `2024-01-02-t10-30-00-z.log`. Technically correct, practically vandalism.
- **IPs / versions / dates** — `192.168.1.1`, `v1.2.3`, `2024.01.02` must survive the interior-dot rule.
- **Date-prefixed content** — Hugo/Jekyll `2024-01-02-post-title.md`, `_index.md`.
- **Inbound references** — renaming breaks every markdown link, import, and `<img src>` pointing at the old name. At minimum warn; at most offer a `--check-references` mode.

**Mitigation:** a `--protect` list (`Makefile`, `Dockerfile`, `*.java`, `[*]`, `+page*`, `__*__`, `*.app`, `*.framework`) that refuses by default and requires `--force`.

## 14. Weird-but-real

- A file literally named `-rf`, `--force`, or `-`.
- A filename containing `$(rm -rf ~)`, backticks, or `;` — harmless **if and only if the tool never shells out**. Rule: use the `rename()` syscall; if `git mv` is used, exec an argv array with `--`, never a shell string.
- A name that transforms to `.`, `..`, `-`, or empty.
- `..config.md` — a legitimate hidden file beginning with two dots.
- Rename target exists but is a *directory*.
- Rename target is a dangling symlink pointing back at the source.
- Two long names that truncate to the same string — truncation and collision resolution interact and must be ordered.
- Case-only rename where a case-sensitive disk image is mounted inside a case-insensitive parent.
- A filename that is 255 bytes of combining marks.

## Resolved defaults

All open decisions are settled. Each row is the shipped behavior plus the flag that overrides it.

| # | Decision | Default | Override |
|---|---|---|---|
| 1 | Digit word boundaries | **None.** Only case transitions split. `Chapter10Part2` → `chapter10-part2` | — |
| 2 | Non-Latin scripts | **Transliterate what is deterministic** (Latin-ext, Cyrillic, Greek); **keep** what is not (CJK, Thai, Arabic, Hebrew, Indic). `日本語 Report.md` → `日本語-report.md` | `--ascii` forces strict ASCII output |
| 3 | Invalid-UTF-8 filenames | **Lossy-decode and rename**, logging every dropped byte | — |
| 4 | Protected names | **Never blocks an explicitly named file** — warn on stderr, then rename. Under `-r` the user did not pick each file, so protected names are **skipped and reported** | `-f` renames protected names under `-r` too (warnings are stderr — `2>/dev/null`) |
| 5 | `IPv6Address` | `ipv6-address` — an uppercase run plus following lowercase plus trailing digits is one token. **Mid-uppercase-run only**: after a *digit* the boundary always stands, or decision 1's `Chapter10Part2` → `chapter10-part2` would regress to `chapter10part2` | — |
| 6 | Compound extensions | **Whitelist**: `.tar.gz`, `.tar.bz2`, `.tar.xz`, `.d.ts`, `.min.js`, `.test.*`, `.spec.*`, `.stories.*`, `.module.css`. Everything else: last dot only | — |
| 7 | `&` and `%` | `&` → `and`; `%` `$` `#` `@` are dropped | — |
| 8 | `C++` / `C#` | Token map: `c++`→`cpp`, `c#`→`csharp`, `f#`→`fsharp`, `.net`→`dotnet` | — |
| 9 | Extension case | **Lowercased.** `.JPG` → `.jpg` | — (no demand for `.JPG`) |
| 10 | `git mv` in repos | **Auto-detect**, tracked files only — preserves staged state and rename detection | — (inference is never wrong) |
| 11 | Cyrillic romanization | **BGN/PCGN** — emits ASCII directly, unlike ISO 9's `šč` | — |
| 12 | Non-ASCII digits | **Mapped to ASCII.** `٣`→`3`, `३`→`3` via an explicit `Nd`→numeric-value table (NFKC covers fullwidth but not Arabic-Indic or Devanagari) | — |
| 13 | Stem-initial lowercase letter | **Glued to the word that follows.** `iPhone14Pro` → `iphone14-pro`, `eBay` → `ebay`. A single leading lowercase letter is a prefix, not a word | — |
| 14 | Uppercase with no lowercase mapping | **Never a case boundary.** `𝔍` and other math alphanumerics are `Uppercase` but fold to themselves, so treating them as a transition splits again on every pass and breaks idempotency. Moot once NFKC (step 3) folds them to ASCII — the guard is defence in depth | — |

### Settled during implementation

The rows above were decided before the code existed. These came up while writing it — each one was an open choice in the sections above, or a contradiction between two of them.

| # | Decision | Default | Override |
|---|---|---|---|
| 15 | Name transforms to nothing (§7) | **Refuse**, warn, exit 1. `untitled.md` would discard the only thing that distinguished `🚀.md`, and two such files in a directory would then need suffixing to tell apart the names the tool itself invented. The transform returns `""` and the filesystem layer leaves the file alone | — |
| 16 | Interior dots (§3 vs §13) | **A dot between two ASCII digits survives; every other interior dot becomes a separator.** §3's table says interior dots split and also that `v1.2.3-Release.zip` keeps its dots; §13 requires `192.168.1.1` and `2024.01.02` to survive. The digit test is what separates `My.File.Name` from a version. It also means an extension that is all digits is not an extension | — |
| 17 | Target exists, different file (§7) | **Auto-suffix** `-2`, `-3`… Skipping would leave the job undone with no way to finish it; suffixing is what restores invariant 3, and the suffixed name is a fixed point of the transform | `-f` overwrites |
| 18 | Target is a *directory* | **Never overwritten, at any force level** — replacing it means deleting its contents, which invariant 1 outranks. `-f` falls back to suffixing and says so | — |
| 19 | Combining marks on a kept script (§10, Indic) | **A mark is stripped only when its base is ASCII.** After steps 4 and 6 every romanized script *is* ASCII, so an ASCII base means "Latin letter wearing an accent" and stripping is the point. Devanagari keeps its virama and vowel signs. A mark orphaned by a stripped base (an emoji, say) is dropped, not re-attached — keeping it breaks idempotency | `--ascii` drops them all |
| 20 | Zero-width space (§1 vs §11) | **A separator, not a deletion.** §1 groups U+200B with NBSP, §11 lists it among the invisibles to strip. It is a *space*; the joiners and marks around it are not, and those are still deleted | — |
| 21 | Journal location | `$XDG_STATE_HOME/keb/journal.tsv`, or `%LOCALAPPDATA%\keb\journal.tsv`; last 20 runs. Reading a state-directory variable is not the configuration §8 rules out — it says where to put a file, never what the tool does to a name | — |

### Consequences worth restating

- Decision 2 means the output alphabet is **not** `[a-z0-9-]` by default — it is `[a-z0-9-]` plus any letters from scripts the tool does not transliterate. `--ascii` is what narrows it to the strict whitelist.
- Decision 1 is what protects `1080p`, `h264`, `utf8`, `sha256`, `base64`, `mp4`, `v1` and every hash or UUID in section 13.
- Decision 4 means the protect list from section 13 is a **warning** list for direct arguments and a **skip** list for recursive sweeps.
