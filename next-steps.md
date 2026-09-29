# keb — next steps

The plan for taking keb from "published" to "available everywhere, and known". Written so the next agent (or me, later) can pick up at the first unchecked box.

**Where we are:** distribution plumbing works — crates.io, npm (`@frycz/keb`), Homebrew tap (`frycz/tap/keb`), shell/PowerShell installers, 5 build targets via cargo-dist. See `publish.md` for the release procedure. The bottleneck now is attention: the "official" channels (homebrew-core, Debian, Fedora) only accept tools people already use.

**Strategy:** packaging readiness → demo → release → feedback launch → ungated package channels, ordered by what people asked for → full launch → gated channels once there is traction. Batch 4 runs before batch 3; the reasoning and the posting order are in `launch/strategy.md`.

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
- [x] **Demo GIF.** `demo/setup.sh` recreates `playground/demo` with three WhatsApp exports; `demo/demo.tape` records `demo/keb.gif` (list, `keb *`, list, with a reaction comment after each list; 16 s, ~164 KB). Re-record: `cargo build --release && vhs demo/demo.tape`.
- [x] **README hero.** New tagline, GIF (absolute `raw.githubusercontent.com` URL so it renders on crates.io and npm — appears once pushed to `main`), and a "Why not `rename` or `mmv`?" section; every claim verified in the playground.
- [x] `demo/` and `next-steps.md` excluded from the crate.

**Owner, before committing batch 1:** run the full `cargo test`.

---

## Batch 2 — finish before the next release ✅ done

- [x] **Man page polish.** `tests/generated.rs` renders sections one by one. DESCRIPTION is five paragraphs (every claim verified in the playground), EXTRA is gone, and EXAMPLES is built from `cli::EXAMPLES` (now `pub`), so `--help` and the man page cannot drift. Added an EXIT STATUS section, checked against `main.rs`: a directory given without `-d` exits 1, but a protected name skipped under `-r` exits 0. New dev-dependency `roff = "1.1.1"`, which clap_mangen already pulled in. Also replaced `…` with `...` in the `--format json` help, because troff could not render it outside a UTF-8 locale; this touched `completions/_keb` and `keb.fish` too.
- [x] **`.deb` / `.rpm` release assets.**
  - `.github/workflows/linux-packages.yml`, a dist custom publish job (`publish-jobs = [..., "./linux-packages"]`). It runs after `host`. A separate `on: release` workflow would never fire, because the Release is created with `GITHUB_TOKEN`.
  - It repackages the Release's own `keb-<target>.tar.xz` binaries with `cargo deb --no-build --no-strip` and `cargo generate-rpm`, on the same runners dist uses (`ubuntu-22.04`, `ubuntu-22.04-arm`), then uploads the packages and their `.sha256` files with `--clobber`.
  - dist gives custom jobs only `id-token`/`packages` write, so the upload would be refused. Fixed with `github-custom-job-permissions = { "linux-packages" = { contents = "write" } }`. `dist init --yes` was re-run, and `dist generate --check` and actionlint pass (actionlint only flags shellcheck style notes in dist's own generated code).
  - Metadata is in `[package.metadata.deb]` / `[package.metadata.generate-rpm]`. The deb maintainer is `Adam Sawicki <frycz.dev@gmail.com>`.
  - Verified locally: built in `rust:1-bullseye` containers with dist's profile, then installed in `debian:bookworm` and `fedora:latest`. Checked `keb --version`, `man keb` (all sections, C locale), bash completion loading, file layout, auto-detected glibc dependency (`libc6 (>= 2.30)` / `GLIBC_2.30`), and clean removal. **The CI job itself is only proven on the next tag.**
- [x] **cargo-binstall.** Works with no metadata. `cargo binstall keb@0.3.0 --disable-strategies quick-install,compile` installs from dist's GitHub assets (tested in a container).
- [x] **Social preview image.** `demo/preview.tape` → `demo/preview.png` (1280×640, one frame of `keb *` then `ls -1` on the same WhatsApp files). Re-record: `cargo build --release && vhs demo/preview.tape`.
- [x] **Repo metadata command**, in "Owner, after batch 2" below.
- [x] `CHANGELOG.md` `[Unreleased]` and `publish.md` (job order, verify step, recovery row, new "Linux packages" section with by-hand container checks) updated.

### Owner, after batch 2

1. Run the full `cargo test`.
2. Bump to **0.4.0** (license change + new shipped files) following `publish.md`: `Cargo.toml`, README installer URL, regenerate man page, CHANGELOG.
3. Commit, `cargo publish`, tag, watch CI, `npm publish` by hand (the npm gap in `publish.md`).
4. Check the Release has the four .deb/.rpm assets (+ `.sha256`); `cargo binstall keb` works. If `custom-linux-packages` failed, the Release is still fine. Fix and re-run the job.
5. Upload `demo/preview.png` as the social preview (GitHub → Settings → General → Social preview; no API). Then run this, which replaces the current description "Rename files to kebab case." (the repo has no homepage or topics yet):

   ```sh
   gh repo edit frycz/keb \
     --description "Rename files to kebab case, safely and idempotently" \
     --homepage "https://crates.io/crates/keb" \
     --add-topic kebab-case,rename,cli,rust,filenames,slug,unicode,command-line-tool
   ```

---

## Batch 3 — package manifests (agent, after 0.4.0 is released)

### State when batch 2 ended (2026-09-29)

- **0.4.0 is released.** Published on crates.io, tagged `v0.4.0`, and the Release exists. The Release CI results: every build, `host`, Homebrew and both `custom-linux-packages` jobs succeeded. `publish-npm` failed as expected (the npm gap in `publish.md`), so `announce` was skipped.
- **Owner confirmed done:** `npm publish` by hand, social preview uploaded, `gh repo edit` run.
- **`ci.yml` Windows failures: fixed in the working tree, not yet proven in CI.** 13 tests in `tests/cli.rs` failed on `windows-latest` (run 36573886405).
  - keb now keeps the parent path exactly as typed: `rename::sibling` replaces only the basename and reuses the separator the user typed, so `./A Dir/My File.md -> ./A Dir/my-file.md` and `.\A Dir\My File.md -> .\A Dir\my-file.md`. This also fixes a latent Windows bug: `same_file` compares spellings there, so a case-only rename under a `/`-typed parent was taken for a collision. `same_file` now also normalises separators.
  - Tests: `tree()` joins components with `/`; the `--absolute` tests go through a `Sandbox::absolute` helper (no `canonicalize` on Windows, which returns the `\\?\` form and long names); the two JSON tests are `#[cfg(unix)]` because Windows forbids `"` and newlines in filenames; there is a new Windows-only `keeps_the_separator_the_user_typed` test.
  - `-r` paths still get the native separator below the root, because the walker uses `dir.join`. Old and new paths stay consistent with each other.
  - **Owner:** run the full `cargo test`, push, and check that Windows CI is green.
- **The `.deb`/`.rpm` job is now proven in CI.**

### Facts about the 0.4.0 Release (checked, don't re-derive)

- **Base URL:** `https://github.com/frycz/keb/releases/download/v0.4.0/`
- **Binary archives:**
  - `keb-x86_64-unknown-linux-gnu.tar.xz`
  - `keb-aarch64-unknown-linux-gnu.tar.xz`
  - `keb-x86_64-apple-darwin.tar.xz`
  - `keb-aarch64-apple-darwin.tar.xz`
  - `keb-x86_64-pc-windows-msvc.zip`
- **Linux packages:**
  - `keb_0.4.0-1_amd64.deb`
  - `keb_0.4.0-1_arm64.deb`
  - `keb-0.4.0-1.x86_64.rpm`
  - `keb-0.4.0-1.aarch64.rpm`
- **Other assets:**
  - `source.tar.gz`, dist's git archive of the tag
  - `sha256.sum`, all sums in one file
  - `keb-installer.sh`, `keb-installer.ps1`, `keb-npm-package.tar.gz`, `keb.rb`, `dist-manifest.json`
- **Checksums:** every archive and package has a `<asset>.sha256` next to it. The format is `<hex> *<filename>`: note the `*` (binary mode), which matters for Scoop/winget regexes. Get the sums with `gh release download v0.4.0 -R frycz/keb --pattern '*.sha256' --pattern sha256.sum` into the scratchpad. Don't copy hashes from memory.
- **Unix tarballs have a top-level directory:** `keb-<triple>/keb`, `keb-<triple>/man/keb.1`, `keb-<triple>/completions/{keb.bash,_keb,keb.fish,_keb.ps1,keb.elv}`, `keb-<triple>/{LICENSE-MIT,README.md,CHANGELOG.md}`.
- **The Windows zip is flat:** `keb.exe`, `man/`, `completions/`, `LICENSE-MIT` and so on at the root, with no top-level directory.
- **Source for from-source builds:** the crates.io tarball `https://static.crates.io/crates/keb/keb-0.4.0.crate` or the GitHub tag archive `https://github.com/frycz/keb/archive/refs/tags/v0.4.0.tar.gz`. Both include `man/` and `completions/`, so packagers install those files directly and never need to generate them.
- **Build notes:** MSRV 1.85, edition 2024. The binary needs the default `cli` feature. `cargo build --release --locked` works.
- **Tests:**
  - `tests/cli.rs` creates dirs under `std::env::temp_dir()` and calls `git` (the decision-10 tests), so a sandboxed check needs `git` available (nix: `nativeCheckInputs = [ git ]`).
  - keb itself shells out to `git mv` for tracked files, but it works without git installed, so git is not a runtime dependency.
  - Agents still must not run `tests/cli.rs` on the host. Inside a container or Nix sandbox it's fine: that's the point of the sandbox.
- **Identity for manifests:** maintainer `Adam Sawicki <frycz.dev@gmail.com>`, GitHub `frycz`, license `MIT` (0.4.0 is MIT only), homepage `https://github.com/frycz/keb`, description "Rename files to kebab case, safely and idempotently".

### Tasks

Put everything under `packaging/<channel>/` in this repo, and add `packaging/` to `exclude` in `Cargo.toml`. The owner copies each channel's files to its destination repo.

- [ ] **AUR:** two PKGBUILDs + `.SRCINFO`.
  - `keb` builds from the crate source with `cargo build --frozen --release`, running `cargo fetch --locked` in `prepare()`, per the Arch Rust package guidelines.
  - `keb-bin` uses the prebuilt tarballs, with `source_x86_64`/`source_aarch64` and `sha256sums_x86_64`/`sha256sums_aarch64`, and `provides=(keb)` / `conflicts=(keb)`.
  - Install the man page to `/usr/share/man/man1`, the completions to `/usr/share/bash-completion/completions/keb`, `/usr/share/zsh/site-functions/_keb` and `/usr/share/fish/vendor_completions.d/keb.fish`, and the license to `/usr/share/licenses/<pkgname>/`.
  - Verify with `makepkg` + `namcap` in an `archlinux` container, as a non-root user.
- [ ] **nixpkgs:** `pkgs/by-name/ke/keb/package.nix`.
  - Use `rustPlatform.buildRustPackage` with `fetchFromGitHub` (`tag = "v${version}"`), plus `cargoHash`: get it with a fake hash first and the real one from the error.
  - Use `installShellFiles` (`installManPage man/keb.1`, `installShellCompletion --cmd keb --bash completions/keb.bash --zsh completions/_keb --fish completions/keb.fish`), `meta.license = lib.licenses.mit` and `meta.mainProgram = "keb"`.
  - Verify with `nix-build` in a `nixos/nix` container. If sandbox tests fail, prefer fixing inputs (`git`) over `doCheck = false`, and explain any `checkFlags` skips in a comment.
- [ ] **winget:** manifest set for `frycz.keb` 0.4.0: version, `installer` and `defaultLocale` files.
  - Installer: `InstallerType: zip` with `NestedInstallerType: portable`, `NestedInstallerFiles: [{RelativeFilePath: keb.exe, PortableCommandAlias: keb}]`, x64 only.
  - Write the manifests by hand or with `wingetcreate new`. Validate with `winget validate` if a Windows host is available; otherwise check them against the published schema and say they are unvalidated.
- [ ] **Scoop:** `keb.json` for a new `frycz/scoop-bucket` repo.
  - The `64bit` URL is the Windows zip, with `bin: "keb.exe"`.
  - `checkver` uses GitHub releases. `autoupdate` puts `$version` in the URL, and `hash.url` points at `$url.sha256`; check that Scoop's default regex copes with the `*` prefix.
  - Include a minimal bucket `README.md`.
- [ ] **aqua registry:** `pkgs/frycz/keb/registry.yaml` with `type: github_release`, `repo_owner: frycz`, `repo_name: keb`.
  - `asset: keb-{{.Arch}}-{{.OS}}.{{.Format}}` with `replacements` amd64→x86_64, arm64→aarch64, darwin→apple-darwin, linux→unknown-linux-gnu, windows→pc-windows-msvc.
  - `format: tar.xz`, with an override to `zip` on windows.
  - The binary is at `files: [{name: keb, src: "keb-{{.Arch}}-{{.OS}}/keb"}]` on unix and flat `keb.exe` on windows.
  - Add a `checksum` block using the per-asset `.sha256` files.
  - Supported envs: darwin, linux, windows/amd64 (there is no windows/arm64 build). Also add `pkgs/frycz/keb/pkg.yaml`.
  - Verify with `aqua` in a container if practical: `aqua g`/`aqua i` against a local registry.
- [ ] Write `packaging.md`. For each channel: what was prepared, where it lives, exact submission steps (accounts, fork/PR, the reviewers' usual asks), and how to bump it on the next release. Also say which channels will update themselves (Scoop autoupdate, the nixpkgs `r-ryantm` bot, aqua Renovate) as input for batch 5.
- [ ] Add a pointer to `packaging.md` in `publish.md`'s "Verify" step, for checking each channel after a release.

**Done when:** every manifest builds/validates locally where tooling allows (say plainly which ones could not be validated), and `packaging.md` lists the owner's submission steps.

### Owner, after batch 3

- Create an AUR account, add SSH key, push both packages.
- Fork + PR: nixpkgs, winget-pkgs, aqua-registry. Expect review rounds over 1–2 weeks; answer them.
- Create the `frycz/scoop-bucket` repo and push `keb.json`. Add the Scoop and AUR install lines to the README once live.

---

## Batch 4 — launch material (agent; before batch 3, see `launch/strategy.md`)

**Blocker found while drafting, fixed in the working tree:** `keb -r .` (and `keb -r ..`) printed `keb: .: no basename to rename` and exited 1, although the sweep itself worked. `step` in `src/main.rs` now returns early for a `-r` root whose last component is `.`, `..`, a root or a prefix; three new tests in `tests/cli.rs` cover it. Release 0.4.1 (with the Windows separator fix) before the first post.

- [x] **Strategy:** `launch/strategy.md`, covering the order (r/commandline → adjust → r/rust → blog + Show HN → CotW and awesome lists → packaging), the objections to expect, and what to listen for.
- [x] **r/commandline** draft: `launch/r-commandline.md` (usefulness angle, asks for feedback).
- [x] **r/rust** draft: `launch/r-rust.md` (implementation angle). Update it with what came up on r/commandline before posting.
- [ ] **Blog post draft** (after the Reddit rounds, leading with what people reacted to): "Renaming a file to kebab-case is harder than you think". Source: `design/cases.md` — NFKC vs NFD, case boundaries (`XMLHttpRequest`), digits not splitting words, compound extensions, grapheme-safe truncation and the 255-byte / UTF-16 limits, case-only renames on case-insensitive filesystems, protected names. keb is the punchline, not the subject. Every example verified in the playground.
- [ ] **Show HN** title + first comment (the "why I built it / what was hard" text).
- [ ] **Crate of the Week** nomination for the users.rust-lang.org thread.
- [ ] **awesome-rust** and **awesome-cli-apps** entries, matching each list's format and contribution rules.
- [x] Put all drafts in `launch/` (excluded from the crate).

### Owner, after batch 4

- Follow `launch/strategy.md`: r/commandline first, then r/rust a few days later, then the blog post and Show HN.
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
| Batch 2 — man polish, .deb/.rpm, binstall, preview image, repo metadata | agent | ✅ done |
| Release 0.4.0, upload preview, set topics | owner | ✅ done |
| Batch 4 — Reddit drafts + strategy | agent | ✅ drafted; blog post, Show HN, CotW, awesome lists left |
| Fix `keb -r .` | agent | ✅ done, owner to run full `cargo test` |
| Release 0.4.1, post r/commandline, then r/rust | owner | the fix, batch 4 drafts |
| Batch 3 — AUR, nixpkgs, winget, Scoop, aqua manifests | agent | feedback from the Reddit rounds |
| Submit packages, answer reviews | owner | batch 3 |
| Full launch (blog post, Show HN) | owner | Reddit feedback, batch 4 |
| Batch 5 — release automation | agent | batch 3 accepted |
| Gated channels | owner | traction |
