# r/commandline draft

Angle: usefulness. Show what it does to a messy folder, be upfront that it's early, and ask for feedback. Every example below was run against keb in the playground. Post with the GIF (`demo/keb.gif`) if the subreddit allows images, or link it.

## Title

keb: rename files to kebab case without writing a regex. Looking for feedback

## Body

I kept writing the same `rename`/`sed` one-liner to clean up filenames, and it kept getting cases wrong: `XMLHttpRequest` came out as `xmlhttprequest`, `.tar.gz` got mangled, `Łódź` lost its first letter, and two files that clean up to the same name overwrote each other. So I wrote a small tool that does one thing: rename files to kebab case, safely.

```console
$ keb -n *
IMG_1080p v1.2.3.mp4 -> img-1080p-v1.2.3.mp4
Łódź Trip.jpg -> lodz-trip.jpg
My File.md -> my-file.md
my_file.md -> my-file-2.md
Report (Final) [v2].pdf -> report-final-v2.pdf
Tom & Jerry.txt -> tom-and-jerry.txt
XMLHttpRequest.MD -> xml-http-request.md
```

What it's careful about:

- **Nothing is overwritten.** Two names that both become `my-file.md` get `my-file.md` and `my-file-2.md`.
- **Running it twice changes nothing.**
- **Versions, hashes and resolutions stay intact.** Digits never start a new word, so `v1.2.3`, `1080p` and `a1b2c3d4` survive.
- **Compound extensions survive**: `.tar.gz`, `.d.ts`, `.min.js`. Dotfiles like `.gitignore` are left alone.
- **Case-only renames** (`README.md` → `readme.md`) work on macOS and Windows, where the filesystem is case-insensitive.
- **Names that are load-bearing are skipped** by a recursive sweep: `Makefile`, `Dockerfile`, `*.java`, `[slug].tsx`, `__init__.py` and so on. `keb --list-protected` prints the full list.
- **Accented Latin, Cyrillic and Greek are transliterated** (`Привет` → `privet`); CJK and other scripts that need a dictionary are kept as they are.

It composes like a normal Unix tool: it reads paths from stdin (`find . -name '*.md' | keb`, or `-0` for `find -print0`), renames go to stdout, and warnings go to stderr. `--format=json` gives machine-readable output. There is no undo, so `-n` (dry run) is the habit to build.

Install:

```sh
brew install frycz/tap/keb
cargo install keb
npm install -g @frycz/keb
```

There are also prebuilt binaries for macOS, Linux and Windows, and `.deb` / `.rpm` packages, on the releases page.

Repo: https://github.com/frycz/keb

It's early, and I'd really like to hear:

- Is this something you'd use, or is a shell one-liner good enough for you?
- Did any output surprise you, or look wrong?
- Which package manager would you want it in?
