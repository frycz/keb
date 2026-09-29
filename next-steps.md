# keb — next steps

The plan for taking keb from "published" to "available everywhere, and known". Written so the next agent (or me, later) can pick up at the first unchecked box.

**Where we are:** distribution plumbing works — crates.io, npm (`@frycz/keb`), Homebrew tap (`frycz/tap/keb`), shell/PowerShell installers, 5 build targets via cargo-dist. See `publish.md` for the release procedure. The bottleneck now is attention: the "official" channels (homebrew-core, Debian, Fedora) only accept tools people already use.

**Strategy:** packaging readiness → demo → release → ungated package channels → launch → gated channels once there is traction.

---

## Rules for agents

- **Never commit, tag, push or publish.** The owner does all of that. Leave changes in the working tree and say what changed.
- **Never run `keb` outside `./playground/`** (gitignored). Create any test or demo folders there. VHS tapes must `cd` into a playground subdirectory before invoking keb.
- **Don't run `tests/cli.rs`** — it drives the real binary against dirs in the system temp dir, outside the playground. Safe to run: `cargo test --lib`, `cargo test --no-default-features --lib`, `cargo test --test generated`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`. Ask the owner to run the full `cargo test`.
- Anything outward-facing (opening PRs as the owner, `gh repo edit`, posting) — prepare it, hand it over, don't execute it.
- Verify any claim about keb's behaviour by running it in the playground before writing it into docs. (Batch 1 caught one wrong claim this way: collisions get a `-2` suffix, they are not refused.)

---

## Batch 1 — packaging readiness + demo ✅ done

- [x] **License → MIT only.** `Cargo.toml` said `MIT OR Apache-2.0` with no Apache text shipped. Now `license = "MIT"`. Releases ≤ 0.3.0 stay dual-licensed (irrevocable). Leave `release.yml`'s `SPDX-License-Identifier: MIT or Apache-2.0` alone — that is cargo-dist's license for the generated workflow, not keb's.
- [x] **Completions + man page.**
  - CLI definition moved from `src/main.rs` to `src/cli.rs` (no behaviour change).
  - `tests/generated.rs` renders `completions/` (bash, zsh, fish, PowerShell, elvish) and `man/keb.1` from it and fails on drift. Regenerate: `KEB_REGENERATE=1 cargo test --test generated`.
  - The man page embeds the version → regenerate after every bump (added to `publish.md` step 1).
  - dist `include = ["completions/", "man/"]` — verified in a local `dist build` archive. `release.yml` unchanged.
  - `.gitattributes` forces LF on generated files so Windows CI doesn't fail the byte comparison.
- [x] **Demo GIF.** `demo/setup.sh` recreates `playground/demo`; `demo/demo.tape` records `demo/keb.gif` (15 s, ~257 KB). Re-record: `cargo build --release && vhs demo/demo.tape`.
- [x] **README hero.** New tagline, GIF (absolute `raw.githubusercontent.com` URL so it renders on crates.io and npm — appears once pushed to `main`), and a "Why not `rename` or `mmv`?" section; every claim verified in the playground.
- [x] `demo/` and `next-steps.md` excluded from the crate.

**Owner, before committing batch 1:** run the full `cargo test`.

---

## Batch 2 — finish before the next release (agent)

Everything here must land *before* tagging, because batch 3 needs the release's asset URLs and checksums.

- [ ] **Man page polish.** Rename clap_mangen's default `EXTRA` section to `EXAMPLES` (render sections individually instead of `Man::render`), and give DESCRIPTION a real paragraph instead of repeating the one-line `about`. Regenerate; keep `tests/generated.rs` passing.
- [ ] **`.deb` / `.rpm` release assets** for x86_64 and aarch64 Linux.
  - dist does not build these natively. Add a job — dist custom `publish-jobs` or a separate workflow triggered by the release — using `cargo-deb` and `cargo-generate-rpm`, uploading to the GitHub Release.
  - Install paths: binary `/usr/bin/keb`, man page `/usr/share/man/man1/keb.1`, completions to the standard bash/zsh/fish locations.
  - Metadata in `Cargo.toml` (`[package.metadata.deb]`, `[package.metadata.generate-rpm]`).
  - Verify locally by building and installing in Docker (Debian + Fedora images). The CI job itself is only proven on the next tag — say so.
  - If `release.yml` is touched, re-run `dist init --yes` and confirm it is still in sync (see `publish.md`).
- [ ] **cargo-binstall.** Check whether `cargo binstall keb` resolves dist's release assets; if not, add `[package.metadata.binstall]` with the correct `pkg-url` / `pkg-fmt`.
- [ ] **Social preview image** (1280×640, static before → after). Generate with a second tape, e.g. `demo/preview.tape` using VHS `Screenshot`, so it's reproducible like the GIF.
- [ ] **Repo metadata command.** Prepare (don't run) a `gh repo edit frycz/keb --description … --homepage … --add-topic …` with ~8 topics (e.g. `kebab-case`, `rename`, `cli`, `rust`, `filenames`, `slug`, `unicode`, `command-line-tool`).
- [ ] Update `CHANGELOG.md` `[Unreleased]` and `publish.md` for anything new (e.g. where .deb/.rpm come from, how to verify them).

**Done when:** all safe checks pass, a local .deb and .rpm install and run `keb --version` + `man keb` in containers, and the owner has the preview image and `gh` command in hand.

### Owner, after batch 2

1. Run the full `cargo test`.
2. Bump to **0.4.0** (license change + new shipped files) following `publish.md`: `Cargo.toml`, README installer URL, regenerate man page, CHANGELOG.
3. Commit, `cargo publish`, tag, watch CI, `npm publish` by hand (the npm gap in `publish.md`).
4. Check the Release has the .deb/.rpm assets; `cargo binstall keb` works.
5. Upload the social preview image (GitHub → Settings → General → Social preview; no API). Run the `gh repo edit` command.

---

## Batch 3 — package manifests (agent, after 0.4.0 is released)

Needs the 0.4.0 release asset URLs and sha256 sums (from the Release's `.sha256` files).

- [ ] **AUR:** two PKGBUILDs + `.SRCINFO` — `keb` (builds from the crate source) and `keb-bin` (prebuilt tarballs, x86_64 + aarch64). Install man page, completions and license. Verify with `makepkg` + `namcap` in an `archlinux` container.
- [ ] **nixpkgs:** `pkgs/by-name/ke/keb/package.nix` using `rustPlatform.buildRustPackage`, `installShellFiles` for man + completions, `meta.license = lib.licenses.mit`, `meta.mainProgram = "keb"`. Verify with `nix-build` (local Nix or a `nixos/nix` container). Note: `tests/cli.rs` needs a writable temp dir; set `checkFlags`/`doCheck` if the sandbox breaks it.
- [ ] **winget:** manifest set (version, installer, locale) for the Windows x64 zip, via `wingetcreate new` or hand-written; validate with `winget validate` if available.
- [ ] **Scoop:** create `frycz/scoop-bucket` content — `keb.json` with `checkver` + `autoupdate` so future versions update themselves.
- [ ] **aqua registry:** `pkgs/frycz/keb/registry.yaml` (github_release type, asset template per OS/arch). Makes `mise use aqua:frycz/keb` work.
- [ ] Write `packaging.md`: per channel — what was prepared, where it lives, exact submission steps, and how to bump it on the next release.

**Done when:** every manifest builds/validates locally where tooling allows, and `packaging.md` lists the owner's submission steps.

### Owner, after batch 3

- Create an AUR account, add SSH key, push both packages.
- Fork + PR: nixpkgs, winget-pkgs, aqua-registry. Expect review rounds over 1–2 weeks; answer them.
- Create the `frycz/scoop-bucket` repo and push `keb.json`. Add the Scoop and AUR install lines to the README once live.

---

## Batch 4 — launch material (agent; can run in parallel, publish after 0.4.0)

- [ ] **Blog post draft:** "Renaming a file to kebab-case is harder than you think". Source: `design/cases.md` — NFKC vs NFD, case boundaries (`XMLHttpRequest`), digits not splitting words, compound extensions, grapheme-safe truncation and the 255-byte / UTF-16 limits, case-only renames on case-insensitive filesystems, protected names. keb is the punchline, not the subject. Every example verified in the playground.
- [ ] **Show HN** title + first comment (the "why I built it / what was hard" text).
- [ ] **r/rust** and **r/commandline** posts (different angles: implementation vs usefulness).
- [ ] **Crate of the Week** nomination for the users.rust-lang.org thread.
- [ ] **awesome-rust** and **awesome-cli-apps** entries, matching each list's format and contribution rules.
- [ ] Put all drafts in `launch/` (exclude it from the crate).

### Owner, after batch 4

- Publish the blog post, then post Show HN (weekday morning US time is typical), then Reddit a day or two apart — not all at once.
- Be around to answer comments for the first day. Collect feedback into issues.
- Open the awesome-list PRs.
- Lobsters needs an invite from an existing member.

---

## Batch 5 — release automation (agent, optional, after batch 3 shows what got accepted)

Every accepted channel needs a version bump on each release. Reduce that to near zero:

- [ ] winget: `winget-releaser` action on release.
- [ ] Scoop: `autoupdate` in the manifest already handles it (verify).
- [ ] AUR: CI job that updates `pkgver`/checksums and pushes (needs an AUR SSH key secret), or a documented one-command bump.
- [ ] nixpkgs: nothing — `r-ryantm` bot opens update PRs automatically.
- [ ] aqua: Renovate in aqua-registry usually handles it (verify).
- [ ] Update `publish.md` with the post-release checklist.

---

## After the batches

The agent-side work ends here. What remains is the owner's, and it's ongoing:

- **Per release:** follow `publish.md`; check each channel picked up the new version. Track all channels on **Repology** (repology.org/project/keb).
- **Gated channels, once there is traction** — revisit after the launch settles:
  - **homebrew-core** — notability threshold, roughly ≥ 30 forks / 30 watchers / 75 stars, about 3× that if self-submitted, and the repo must not be very new. *Verify the current rules before applying.* The tap formula is a starting point; core wants a from-source build formula.
  - **Debian / Ubuntu** — needs a Debian Developer sponsor; Rust crates go through the Debian Rust team (`debcargo`). Slow, but Debian → Ubuntu → derivatives.
  - **Fedora** — sponsored packager, `rust2rpm`. Meanwhile a **COPR** repo can be self-served.
  - **Arch official repos** — a maintainer adopts it, usually after AUR votes.
  - **Alpine `testing`, Gentoo GURU** — community tiers, open to anyone, medium effort.
- Often distro maintainers package popular tools on their own. Keep it easy for them: stable tags, consistent license, man page, completions, changelog — all in place after batch 1–2.
- If an agent is needed for any of these, the artifacts from batches 1–3 (man page, completions, .deb/.rpm metadata, manifests) are the starting material.

## At a glance

| Step | Who | Depends on |
|---|---|---|
| Batch 1 — license, completions, man, GIF, README | agent | ✅ done |
| Commit batch 1 | owner | full `cargo test` |
| Batch 2 — man polish, .deb/.rpm, binstall, preview image, repo metadata | agent | — |
| Release 0.4.0, upload preview, set topics | owner | batch 2 |
| Batch 3 — AUR, nixpkgs, winget, Scoop, aqua manifests | agent | 0.4.0 checksums |
| Submit packages, answer reviews | owner | batch 3 |
| Batch 4 — blog post + launch posts | agent | anytime |
| Launch | owner | 0.4.0 live, batch 4 |
| Batch 5 — release automation | agent | batch 3 accepted |
| Gated channels | owner | traction |
