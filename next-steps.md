# keb — next steps

Distribution plumbing is done (crates.io, npm, Homebrew tap, shell installer, 5 targets). The bottleneck is now attention: most "official" channels (homebrew-core, Debian, Fedora) gate on popularity. Order: packaging readiness → demo → launch → ungated channels → gated channels.

## 1. Packaging readiness

- [x] **Make the license MIT-only.** `Cargo.toml` declares `MIT OR Apache-2.0` but only `LICENSE-MIT` exists — a mismatch Debian/Fedora reviewers flag. Fine to drop Apache: sole author, no outside contributions.
  - `Cargo.toml` line 7: `license = "MIT OR Apache-2.0"` → `license = "MIT"`. The only change needed.
  - The npm `package.json` and Homebrew formula are generated from it by dist — they update on the next release.
  - Leave `.github/workflows/release.yml` line 4 (`SPDX-License-Identifier: MIT or Apache-2.0`) alone: it's the license of cargo-dist's generated workflow, not keb's.
  - README (`## License` → MIT) and `LICENSE-MIT` are already correct.
  - Already-published 0.1.0–0.3.0 stay dual-licensed; that can't be revoked. Only new releases are MIT-only.
- [x] **Shell completions + man page** via `clap_complete` / `clap_mangen`; ship them in dist archives. Distro packagers expect both (no man page = Debian lintian warning).
- [ ] **GitHub repo metadata:** topics, homepage URL, social preview image. Topics drive GitHub search.
- [ ] **`.deb` / `.rpm` release assets** (`cargo-deb` / `cargo-generate-rpm` or `nfpm`) — instant Linux install before any distro packages it.

## 2. Demo GIF

- Use **VHS** (charmbracelet): scripted `.tape` file, committed, reproducible. `vhs-action` can regenerate it in CI so it never goes stale.
- One idea, 10–15 s:
  1. `ls` a folder of realistic mess: `Final Report (v2) FINAL.docx`, `Screenshot 2026-09-29 at 10.14.03.png`, `Ünïcödé Notes.md`, `IMG_0042.JPG`, `MyComponent.tsx`
  2. `keb -n *` — show the plan
  3. `keb *` → `ls` again
  4. optional: `find … | keb` to show pipe support
- Place it directly under the tagline in the README, above Install.
- Other forms:
  - static before/after image for the social preview (cards don't animate)
  - asciinema cast (copyable text)
  - short "Why not `rename` / `mmv` / `perl-rename`?" section — the first question people will ask

## 3. Launch — the story beats the tool

`design/cases.md` is the most interesting asset: NFKC vs NFD, grapheme-safe truncation, UTF-16 name limits, case-only renames on case-insensitive filesystems, protected names. Write one post — **"Renaming a file to kebab-case is harder than you think"** — with keb as the punchline.

Post to:
- [ ] Show HN
- [ ] r/rust, r/commandline
- [ ] users.rust-lang.org "Crate of the Week" thread (feeds *This Week in Rust*)
- [ ] Lobsters (needs invite)

PRs to curated lists:
- [ ] awesome-rust
- [ ] awesome-cli-apps

## 4. Ungated channels — do now

| Channel | Effort | Notes |
|---|---|---|
| **AUR** (`keb-bin`, `keb`) | low | Anyone can publish |
| **nixpkgs** | low–med | PR with `buildRustPackage`; friendly to small Rust CLIs |
| **winget** | low | `wingetcreate` from release assets, automated PR |
| **Scoop** | low | Own bucket now, `extras` later |
| **aqua registry** | low | Also makes it installable via mise |
| **cargo-binstall** | ~free | Likely already works from dist assets — verify |
| Alpine `testing`, Gentoo GURU | med | Community tiers |

## 5. Gated channels — after traction

- **homebrew-core** — notability threshold (roughly 30 forks / 30 watchers / 75 stars, ~3× if self-submitted, repo must not be too new). Verify current rules before applying. Currently 0 stars → post-launch.
- **Debian / Ubuntu** — needs a Debian Developer sponsor; Rust crates go through the Debian Rust team (`debcargo`). Slow, but Debian → Ubuntu → derivatives.
- **Fedora** — sponsored packager, `rust2rpm`. Use **COPR** meanwhile.
- **Arch official repos** — a maintainer adopts it, usually after AUR popularity.

Often distro maintainers package popular tools themselves — make it easy: stable tags, consistent license, man page, completions, changelog.

Track coverage on **Repology** (repology.org) — one page showing every repo keb is in and at which version.

## Order

1. MIT-only license + completions + man page
2. VHS GIF + README hero
3. Blog post + launch
4. AUR / nixpkgs / winget / Scoop / aqua
5. homebrew-core + distros once stars exist
