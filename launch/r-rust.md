# r/rust draft

Angle: implementation. What turned out to be hard, and how the code is structured. Post a few days after r/commandline, and update it with anything that came up there. Every example below was run against keb in the playground. The "naive" results come from a Python sketch of the usual NFD + strip-marks + lowercase + `[^a-z0-9]` → `-` approach.

## Title

What I learned writing a kebab-case file renamer in Rust (keb)

## Body

keb renames files to kebab case: `XMLHttpRequest.MD` → `xml-http-request.md`. That sounds like a one-line regex, and most of the work turned out to be in why it isn't. A few things I ran into:

**Order is load-bearing.** The transform is a fixed pipeline: split off the extension, strip control and bidi characters, NFKC, a small fold table, NFD and strip combining marks, transliterate, split words, *then* lowercase, and finally whitelist `[a-z0-9]`. Swap two steps and things break:

| Input | Naive approach | keb |
|---|---|---|
| `XMLHttpRequest` | `xmlhttprequest` (lowercased before splitting words) | `xml-http-request` |
| `Łódź` | `odz` (`Ł` has no NFD decomposition, so it's deleted) | `lodz` |
| `Straße` | `stra-e` | `strasse` |

NFKC before anything else is what turns fullwidth `Ｆｉｌｅ` and "fancy text" like `𝓗𝓮𝓵𝓵𝓸` back into plain letters (`hello.md`).

**Digits don't start words.** Only case transitions split, so `Chapter10Part2` becomes `chapter10-part2`, and hashes and versions (`a1b2c3d4`, `v1.2.3`, `1080p`) survive untouched.

**The transform is deliberately not injective.** `My File.md` and `my_file.md` both map to `my-file.md`. Rather than fight that in the pure function, the filesystem layer restores it by suffixing (`my-file-2.md`). So the crate is two layers: a pure `&str → String` transform in the library, which you can use with `--no-default-features` and no CLI dependencies, and a filesystem layer in the binary that deals with collisions, recursion and the actual renames.

**Property tests carry the transform.** proptest checks that `kebab(kebab(x)) == kebab(x)` for arbitrary strings and option combinations, that it never panics, and that the result always fits the length budget.

**Filesystem reality:**

- Name limits are 255 *bytes* on ext4 but 255 *UTF-16 code units* on APFS and NTFS, so truncation checks both, cuts on a grapheme boundary (unicode-segmentation), and never eats the extension.
- A case-only rename on a case-insensitive filesystem looks like a collision with itself. keb checks whether source and target are the same file (by inode on Unix), and goes through a temporary name when they are.
- `-r` collects the whole tree first and renames deepest-first, so renaming a directory never invalidates a path still in the queue. Symlinks are never followed.
- Inside a git repo, tracked files are moved with `git mv`, run as an argv array with `--`, so a file named `$(rm -rf ~)` is just a name.

The dependencies are clap (CLI only), unicode-normalization and unicode-segmentation. Transliteration is a hand-written table, because keb only romanizes scripts where a codepoint table is enough (Latin, Cyrillic, Greek) and keeps CJK and others as they are.

Repo: https://github.com/frycz/keb — crate: https://crates.io/crates/keb

I'd appreciate a review of the approach, and I'm curious whether anyone has hit filename cases I haven't. `design/cases.md` in the repo is the full list I worked from.
