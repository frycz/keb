# keb — publish finalisation

The remaining steps to get `keb 0.0.1` live. Everything here needs your credentials or
your decision, which is why it is not automated.

`publish.md` is the reference for *how the machinery works*. This file is the
**sequence to execute**, once.

## Already done

- [x] `keb` confirmed free on crates.io; `@frycz/keb` chosen for npm (`keb` is squatted)
- [x] `frycz/homebrew-tap` created, public, empty
- [x] `HOMEBREW_TAP_TOKEN` set as a repo secret on `frycz/keb`
- [x] crate scaffolded — `Cargo.toml`, `src/lib.rs`, `src/main.rs`, both licenses, README
- [x] `dist init` run; `.github/workflows/release.yml` generated and committed
- [x] `cargo test`, `cargo clippy -- -D warnings`, `cargo publish --dry-run` all clean
- [x] 3 commits on `main`, **not yet pushed**

---

## 1. npm credentials

`dist` 0.32.0 cannot use npm Trusted Publishing — its `publish-npm` job authenticates
with `NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}` and the workflow requests no
`id-token` permission. So a token is required for now.

Use a **granular** token, not a classic one. npm's warning about tokens targets
account-wide classic tokens; a granular token restricted to one scope with an expiry
is a different risk profile.

```sh
npm login                      # you are not currently logged in
npm whoami                     # confirm
```

Create the token at **npmjs.com/settings/~/tokens** → *Generate New Token* →
**Granular Access Token**:

| Field | Value |
|---|---|
| Name | `keb-release` |
| Expiration | 90 days (calendar-reminder to rotate) |
| Packages and scopes | **Only select packages and scopes** → `@frycz` |
| Permission | **Read and write** |

The `@frycz` scope is selectable before any package exists in it.

```sh
npm token list                 # sanity check it was created
gh secret set NPM_TOKEN --repo frycz/keb
gh secret list --repo frycz/keb    # expect HOMEBREW_TAP_TOKEN and NPM_TOKEN
```

## 2. Make the repo public

Homebrew formulas, the shell installer and `cargo binstall` all fetch GitHub Release
assets by URL, and cannot authenticate to a private repo. Nothing downstream works
until this is done.

Already verified safe: `.env` is gitignored and was **never committed**; tracked files
are only source, docs, licenses and workflows.

```sh
gh repo edit frycz/keb --visibility public --accept-visibility-change-consequences
gh repo view frycz/keb --json visibility
```

## 3. Push

```sh
git push -u origin main
```

Watch `ci.yml` go green on Linux, macOS and Windows before tagging. If `cargo fmt
--check` fails on CI but passed locally, CI is on a newer toolchain — run `cargo fmt`
and amend.

## 4. Publish to crates.io — manual, once

`dist` generates **no** `cargo publish` job. This step is yours on every release until
you adopt `release-plz`.

Your crates.io credentials already exist in `~/.cargo/credentials.toml`. Confirm the
account's **email is verified** first, at crates.io/settings/profile — publishing is
rejected otherwise, with a message that is easy to misread as an auth failure.

```sh
cargo package --list      # read it: 10 files, no .env, no cases.md
cargo publish --dry-run
cargo publish             # irreversible. 0.0.1 can never be reused or deleted.
```

Then, at **crates.io/crates/keb/settings**, add a **Trusted Publisher**:
repository `frycz/keb`, workflow `release.yml`. That is only useful once you add a
`cargo publish` step to CI, but configure it now while you are on the page.

## 5. Tag the release

```sh
git tag v0.0.1
git push --tags
gh run watch                  # ~5-10 min for 5 cross-compiled targets
```

Job order: `plan` → `build-local-artifacts` → `build-global-artifacts` → `host` →
`publish-homebrew-formula` + `publish-npm` → `announce`.

**Most likely failures, in order of likelihood:**

| Symptom | Cause |
|---|---|
| `publish-homebrew-formula` 403 | `HOMEBREW_TAP_TOKEN` lacks Contents: write on `homebrew-tap`, or expired |
| `publish-npm` 403 | `NPM_TOKEN` wrong scope, or not yet set |
| `publish-npm` 402 Payment Required | should not happen — the job passes `--access public` |
| `build-local-artifacts` fails on one target | a dependency without that target; drop it from `targets` and re-tag |

A failure here costs nothing but a version number. Bump to `v0.0.2` and re-tag —
do **not** delete and re-push the same tag, since `cargo publish` for `0.0.1` has
already happened by this point.

## 6. Verify all four install paths

The whole point of the rehearsal. Run each on a clean shell.

```sh
brew install frycz/tap/keb && keb --version
npx @frycz/keb@0.0.1 --version
cargo install keb --version 0.0.1 && keb --version
curl -sSf https://github.com/frycz/keb/releases/download/v0.0.1/keb-installer.sh | sh
```

Then confirm the binary is real, not just present:

```sh
keb -n "My File.md" "XMLHttpRequest.MD" "A1B2C3D4-E5F6.bin"
# My File.md -> my-file.md
# XMLHttpRequest.MD -> xml-http-request.md
# A1B2C3D4-E5F6.bin -> a1b2c3d4-e5f6.bin
```

Note the stub prints the plan and then refuses to rename — that is expected at
`0.0.1`. `.gitignore` correctly produces no output at all (already kebab → silent
no-op, per the stream contract in `cases.md` §8).

Also check the generated pages render:

- **crates.io/crates/keb** — README, both licenses, keywords
- **docs.rs/keb** — builds automatically; the three doctests are the API examples
- **npmjs.com/package/@frycz/keb** — README, and `optionalDependencies` listing all
  five platform packages

## 7. Prove the steady state

`0.0.1` had a hand-published crates.io step and first-ever npm packages. Tag once
more to confirm a *normal* release works end to end:

```sh
# bump version = "0.0.2" in Cargo.toml
cargo publish
git commit -am "release: v0.0.2" && git push
git tag v0.0.2 && git push --tags
gh run watch
```

If this is green, the pipeline is trustworthy and you can stop thinking about it
until `0.1.0`.

## 8. Clean up

```sh
npm token revoke <id>    # if you want to narrow or rotate early
```

Set a calendar reminder for the `NPM_TOKEN` and `HOMEBREW_TAP_TOKEN` expiries. When
either lapses, releases fail on that one job and everything else still publishes —
which is easy to miss.

---

## Then: build the actual tool

`0.0.1` is a stub. It implements `cases.md` §1 (separators), §2 (case boundaries) and
part of §3 (extensions). Still to do, roughly in dependency order:

1. **Unicode pipeline** — `cases.md` §9 steps 2-6: strip format chars, NFKC, the
   explicit fold table (`ß ł ø đ þ`), NFD + strip `Mn`, transliteration. The fold
   table at step 4 is the load-bearing part; without it `Łódź.md` → `d.md`.
2. **Symbol expansion and the token map** — decisions 7 and 8.
3. **Truncation** — grapheme-safe, byte-aware, extension-preserving (§12).
4. **Filesystem layer** — §6, §7, §12: the two-step case-only rename, the
   write-ahead journal, collision resolution, `git mv` detection, `--protect`.
5. **Fuzzing** — `cargo-fuzz` over arbitrary `&[u8]`, which is where the "never
   panics" invariant actually gets tested.

Ship `0.1.0` when the §1-§5 case matrix passes and the filesystem layer is
reversible. Until then the README's "not recommended for files you care about"
warning stays.
