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

#[test]
fn man_page() {
    let mut out = Vec::new();
    clap_mangen::Man::new(cli::Cli::command()).render(&mut out).unwrap();
    check("man/keb.1", &out);
}
