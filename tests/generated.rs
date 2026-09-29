//! Shell completions and the man page, generated from `src/cli.rs` and committed.
//!
//! Committed rather than produced by a `build.rs`, so they ship in the source tarball
//! where distro packagers look for them, and so library consumers never pay for the
//! generators. This test fails when a committed file drifts from the CLI definition —
//! a changed flag, or a version bump, which the man page header carries. To rewrite
//! them:
//!
//!     KEB_REGENERATE=1 cargo test --test generated

#[allow(dead_code)]
#[path = "../src/cli.rs"]
mod cli;

use std::fs;
use std::path::Path;

use clap::CommandFactory;
use clap_complete::{Generator, Shell};
use roff::{Roff, bold, roman};

fn check(relative: &str, generated: &[u8]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    if std::env::var_os("KEB_REGENERATE").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, generated).unwrap();
        return;
    }
    let committed = fs::read(&path).unwrap_or_default();
    assert!(
        committed == generated,
        "{relative} is out of date; run `KEB_REGENERATE=1 cargo test --test generated`",
    );
}

#[test]
fn completions() {
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell, Shell::Elvish] {
        let mut out = Vec::new();
        clap_complete::generate(shell, &mut cli::Cli::command(), "keb", &mut out);
        check(&format!("completions/{}", shell.file_name("keb")), &out);
    }
}

/// The man page's DESCRIPTION. `about` is one line, and `--help` stays short on purpose,
/// so the paragraph a man page reader expects lives here instead. Paragraphs are
/// separated by blank lines.
const DESCRIPTION: &str = "\
keb renames files so their names are kebab case: lowercase words joined by hyphens. \
The extension is kept and lowercased, and only the basename changes; parent \
directories are never touched. Names already in kebab case are left alone and print \
nothing, so running keb twice changes nothing.

With no paths, or with -, keb reads a list of paths from stdin, one per line, or \
null-separated with -0. Paths are renamed deepest first, so renaming a directory never \
invalidates a path still queued beneath it.

Only files are renamed unless -d is given. -r recurses into directories and renames \
the files it finds; with -d as well, it renames the directories too.

When several names would become the same, one keeps it and the others get a numeric \
suffix (-2, -3, ...); nothing is overwritten unless -f is given. Names that a tool \
looks up literally, such as Makefile, LICENSE or __init__.py, are protected: skipped \
under -r, renamed with a warning when named explicitly. -f renames them under -r too. \
--list-protected prints the full list and the reason for each.

Each rename is printed to stdout, one per line. Warnings, errors and the summary line \
go to stderr.";

/// Sections are rendered one by one, rather than with `Man::render`, to replace
/// clap_mangen's catch-all EXTRA section with EXAMPLES and EXIT STATUS, and to give
/// DESCRIPTION more than the one-line `about`.
#[test]
fn man_page() {
    let man = clap_mangen::Man::new(cli::Cli::command());
    let mut out = Vec::new();
    man.render_title(&mut out).unwrap();
    man.render_name_section(&mut out).unwrap();
    man.render_synopsis_section(&mut out).unwrap();
    out.extend(description().render().into_bytes());
    man.render_options_section(&mut out).unwrap();
    out.extend(examples().render().into_bytes());
    out.extend(exit_status().render().into_bytes());
    man.render_version_section(&mut out).unwrap();
    check("man/keb.1", &out);
}

fn description() -> Roff {
    let mut roff = Roff::new();
    roff.control("SH", ["DESCRIPTION"]);
    for (i, paragraph) in DESCRIPTION.split("\n\n").enumerate() {
        if i > 0 {
            roff.control("PP", []);
        }
        roff.text([roman(paragraph)]);
    }
    roff
}

/// Built from the same text `--help` prints, so the two cannot drift: each example
/// becomes a tagged paragraph, the command in bold, and the notes follow as prose.
fn examples() -> Roff {
    let (list, notes) = cli::EXAMPLES.split_once("\n\n").expect("examples, then notes");
    let mut roff = Roff::new();
    roff.control("SH", ["EXAMPLES"]);
    for line in list.lines().skip(1) {
        let (command, what) = line.trim().split_once("  ").expect("command, then description");
        roff.control("TP", []).text([bold(command)]).text([roman(what.trim())]);
    }
    roff.control("PP", []).text([roman(notes.replace('\n', " "))]);
    roff
}

/// Mirrors `main`'s mapping from outcome to exit code.
fn exit_status() -> Roff {
    let mut roff = Roff::new();
    roff.control("SH", ["EXIT STATUS"]);
    for (code, meaning) in [
        (
            "0",
            "Every path was renamed or needed no change. Protected names -r skipped count as no change.",
        ),
        (
            "1",
            "At least one path was not renamed: it does not exist, it is a directory given without -d, or its name has no kebab form. The other paths were still processed.",
        ),
        (
            "2",
            "A usage error, such as an unknown option or an empty list of paths, or an error that stopped the run.",
        ),
    ] {
        roff.control("TP", []).text([bold(code)]).text([roman(meaning)]);
    }
    roff
}
