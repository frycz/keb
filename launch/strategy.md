# keb — launch strategy

The goal of the first round is to learn whether anyone wants keb, and what they would change, before spending effort on package channels or a big launch. Treat it as a feedback round with a small audience first. The big audience comes later.

## Why launch before packaging (batch 4 before batch 3)

- Packaging for AUR, nixpkgs, winget, Scoop and aqua only pays off if people want the tool, and a launch is how to find out.
- keb is already easy to install for almost everyone who reads HN or Reddit: `cargo install` / `cargo binstall`, the Homebrew tap, npm, the shell and PowerShell installers, and `.deb` / `.rpm` on every release.
- Feedback decides which channels to package first. If people ask for Scoop, do Scoop. New channels become "now also on …" follow-ups, which give you a reason to post again.

## Before the first post

1. **Fix `keb -r .`** ✅ (in the working tree). It is the first thing a new user will type in a project directory, and it used to print `keb: .: no basename to rename` and exit 1, even though the sweep worked. `keb -r ..` did the same.
2. **Release 0.4.1** with that fix and the Windows separator fix. The Windows fix matters: under 0.4.0, a case-only rename under a folder typed with `/` can come out as `name-2.ext`. A launch brings new users, so they should get the fixed version.
3. **Check the README install line** points at the new version (`publish.md` covers this).

## Order

1. **r/commandline** — the "is this useful?" crowd, lowest stakes. Draft: `r-commandline.md`. Post, answer every comment for the first day, and write down what people ask for.
2. **Adjust.** Update the README and the pitch from what came up: confusing flags, missing features, the most common objection.
3. **r/rust**, a few days later — the "how is it built?" angle. Draft: `r-rust.md`.
4. **Blog post, then Show HN.** The blog post ("Renaming a file to kebab-case is harder than you think") leads with the hard cases people actually reacted to on Reddit. Show HN links to the post or the repo. You only really get one shot at the HN front page, so use it after the pitch has been tested. Weekday morning, US time, is typical.
5. **Crate of the Week** nomination (users.rust-lang.org thread), **awesome-rust** and **awesome-cli-apps** PRs. These are low-effort and not time-sensitive.
6. **Batch 3 packaging**, ordered by what people asked for.

Space the posts a few days apart, not all at once: each round should shape the next, and posting everywhere on the same day looks like spam.

## What to listen for

- **Does the problem resonate?** Do people say "I have a folder like that", or "why would I need this"? The second one means the pitch needs a sharper use case.
- **Objections to expect**, so the answers are ready:
  - "`rename 's/ /-/g'` does this." The README's "Why not `rename` or `mmv`?" section is the answer: the value is in the cases a one-line regex gets wrong.
  - "No undo is scary." The answer is `-n`. If this comes up a lot, consider making the dry-run advice louder, not bringing `--undo` back.
  - "`keb *` renamed my `Makefile`." A name you pass yourself, including through a glob, is renamed with a warning, because you chose it. Only `-r` skips protected names. Expect someone to find this surprising. If several people do, reconsider it: a glob is not really a choice of each file.
  - "Why kebab only? I want snake case." `--separator=_` exists. If people ask for camelCase or other styles, that's a scope question for later.
  - "`10.15.22` in `Screenshot … at 10.15.22.png` stays dotted." A dot between digits is kept on purpose (versions, IPs). Some will want it split for timestamps.
- **Platforms**: who is on Windows, and whether anything breaks there.
- **Install friction**: which install method people use, and which one they ask for that doesn't exist yet.

## After each round

- Turn every concrete request or bug into a GitHub issue, even the ones you won't do, and close those with the reason. It shows the project is maintained.
- Note in this file what came up, so the next round's post can lead with it.
