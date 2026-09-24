# Publishing a new version

## 1. Prepare

```sh
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
# read it — anything not gitignored can be swept in
cargo package --list
cargo publish --dry-run
```

Bump `version` in `Cargo.toml`, update `CHANGELOG.md`, commit.

## 2. crates.io — by hand

```sh
# irreversible: the version can never be reused
cargo publish
```

## 3. Tag

```sh
git push
git tag v0.1.0 && git push --tags
gh run watch
```

CI then runs: `plan` → `build-local-artifacts` (5 targets) → `build-global-artifacts` → `host` (creates the Release) → `publish-homebrew-formula` + `publish-npm` → `announce`. Roughly 5-10 minutes.

`publish-npm` **will fail** until the npm gap below is closed. Everything else succeeds independently; only `announce` is skipped.

## 4. npm — by hand, after the tag

Trusted Publishing *is* configured, but it is only npm's half of the handshake: the workflow still has to present an OIDC token, and `dist`'s job sends `NODE_AUTH_TOKEN` instead. See [the npm gap](#the-npm-gap).

Order is forced: the npm package's `postinstall` downloads the binary from `releases/download/v<version>/`, and that URL is baked into `package.json` at build time. Publishing before the Release exists gives every user a failed install.

```sh
dist build --artifacts=global
# prompts for OTP
npm publish --access public ./target/distrib/keb-npm-package
```

## 5. Verify

```sh
brew update && brew upgrade keb && keb --version
# CDN lags a minute or two after publishing
npm view @frycz/keb version
cargo install keb --force
curl -sSf https://github.com/frycz/keb/releases/download/v0.1.0/keb-installer.sh | sh
```

Then check [crates.io/crates/keb](https://crates.io/crates/keb), [docs.rs/keb](https://docs.rs/keb) and [npmjs.com/package/@frycz/keb](https://www.npmjs.com/package/@frycz/keb) render.

---

# First-time setup

Already done for `keb`. Kept as the record, and for the next project.

## Repos

```
github.com/frycz/keb            public — source, CI, releases
github.com/frycz/homebrew-tap   public — one generated Formula/keb.rb
```

The tap **must** be named `homebrew-*` (Homebrew strips the prefix) and is a collection, not one formula — hence `homebrew-tap`, giving `brew install frycz/tap/keb`, not `homebrew-keb`.

The source repo **must be public**: Homebrew, the shell installer, `cargo binstall` and the npm `postinstall` all fetch Release assets by URL and cannot authenticate.

## Accounts

| Registry | Needs |
|---|---|
| crates.io | GitHub OAuth + **verified email** — publishing is refused otherwise, with a message that reads like an auth failure |
| npm | account + 2FA |
| Homebrew | nothing. There is no Homebrew account. |

Names are first-come and permanent. `keb` was free on crates.io; on npm it is squatted by a dead `0.0.0` stub, hence the `@frycz` scope. Package name and binary name are independent, so the binary is `keb` everywhere regardless.

## Secrets on `frycz/keb`

| Secret | Why |
|---|---|
| `HOMEBREW_TAP_TOKEN` | fine-grained PAT, `homebrew-tap` only, Contents: read+write. The built-in `GITHUB_TOKEN` is scoped to the repo the workflow runs in and cannot push to another repo. |
| `NPM_TOKEN` | present but **ineffective** — see below. |

Fine-grained PAT gotcha: the permissions list is a two-stage UI. Check **Contents** in the multi-select picker, close it, *then* set the access level on the row that appears.

## cargo-dist

The crate is `cargo-dist`; **the installed binary is `dist`**. `cargo dist …` does not work on 0.32.0.

```sh
# ~2 min, builds from source
cargo install cargo-dist --locked --version 0.32.0
# reads [workspace.metadata.dist]
dist init --yes
```

Config lives in `Cargo.toml`. `dist` rewrites the block with its own formatting, so do not hand-tidy it. After any change, re-run `dist init --yes` and commit the regenerated `.github/workflows/release.yml` — CI checks it is in sync.

---

# The npm gap

`dist` 0.32.0 cannot publish to npm from CI on this account, for two compounding reasons:

1. **`dist` has no OIDC support.** Its `publish-npm` job sends `NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}` and the workflow requests no `id-token: write` permission, so npm Trusted Publishing cannot be reached.
2. **Tokens now need an explicit 2FA bypass.** npm removed classic Automation tokens in November 2025. A granular token publishes from CI only with the **"Bypass two-factor authentication"** flag — and npm is removing direct publish via granular token entirely in **January 2027**.

So the token route is a dead end worth no investment, and step 4 stays manual until `dist` ships OIDC.

**Trusted Publishing is already configured** on `@frycz/keb` (repo `frycz/keb`, workflow `release.yml`, no environment name, "Allow npm publish" enabled). It is inert until the workflow can present an OIDC token. When `dist` adds support — or if you patch the generated workflow to add `id-token: write`, upgrade npm to ≥ 11.5.1 and drop `NODE_AUTH_TOKEN` — step 4 disappears and `NPM_TOKEN` can be deleted.

Both registries share a bootstrap rule: a trusted publisher is configured in an *existing* package's settings, so the first publish of anything is always manual.

---

# Recovery

Publishes are effectively permanent. crates.io versions can never be reused or deleted; `cargo yank` only stops *new* dependents resolving to them. npm allows `unpublish` within 72 hours, otherwise `npm deprecate`. Homebrew is the forgiving one — the tap is a git repo, so a bad formula is fixed by pushing a correction.

| Failure | Fix |
|---|---|
| CI failed *after* `cargo publish` | Do not re-tag. That version is spent; bump to the next patch. |
| `publish-homebrew-formula` 403 | `HOMEBREW_TAP_TOKEN` expired or lacks Contents: write |
| `npm error code EOTP` | Expected — token has no 2FA bypass. Publish by hand (step 4). |
| `npm 402 Payment Required` | Scoped package published without `--access public` |
| `npm view` 404 right after publishing | CDN lag. Wait a minute. |
| One target fails to build | Drop it from `targets`, `dist init --yes`, re-tag |
| `announce` skipped | A publish job failed; the Release itself is fine |
