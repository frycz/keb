//! The command-line surface: flags, help text, output formats.
//!
//! Kept apart from `main.rs` so `tests/generated.rs` can build the shell completions
//! and the man page from the same definition without going through the binary.

use std::path::PathBuf;

use clap::Parser;

/// Shown under both `-h` and `--help`, because a bare `keb` at a prompt prints the help
/// and this is the only place the one rule that matters — `-n` first, there is no undo —
/// reaches someone who has not read the README. `tests/generated.rs` also renders it as
/// the man page's EXAMPLES section, so it keeps this shape: a heading, one example per
/// line with the description after a run of spaces, a blank line, then notes.
pub const EXAMPLES: &str = "\
Examples:
  keb -n *                    Print the plan, change nothing
  keb *                       Rename every file here
  keb -r docs                 Whole tree, filenames only
  keb -dr Photos              ...directory names too
  keb --separator=_ *         Snake case instead
  find . -name '*.md' | keb   Anything that lists paths
  keb -n --format=json *      One JSON record per rename, safe to parse

Renames cannot be undone. Run -n first on anything you have not renamed before.
Some names are never renamed; --list-protected says which, and why.";

/// Rename files to kebab case, safely and idempotently.
///
/// Only the basename changes; parent directories are never touched. With no paths,
/// reads a list from stdin, so `find . -name '*.md' | keb` works.
#[derive(Parser)]
#[command(name = "keb", version, about, long_about = None, after_help = EXAMPLES)]
pub struct Cli {
    /// Paths to rename. `-` reads a list from stdin.
    pub paths: Vec<PathBuf>,

    /// Print the plan, change nothing.
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Recurse into directories.
    #[arg(short = 'r', long)]
    pub recursive: bool,

    /// Overwrite on collision, and rename protected names under -r.
    #[arg(short = 'f', long)]
    pub force: bool,

    /// Permit renaming directories. Without it, only files are renamed.
    #[arg(short = 'd', long)]
    pub allow_dirs: bool,

    /// Paths on stdin are null-separated, for `find -print0`.
    #[arg(short = '0', long = "null")]
    pub null: bool,

    /// Emit this character between words instead of `-`.
    #[arg(long, value_name = "CHAR")]
    pub separator: Option<char>,

    /// Narrow the output to ASCII, dropping scripts that are otherwise kept.
    #[arg(long)]
    pub ascii: bool,

    /// Override the 255 byte / UTF-16 unit name limit.
    #[arg(long, value_name = "N")]
    pub max_length: Option<usize>,

    /// How to print each rename; `json` and `null` are the parse-safe ones.
    #[arg(long, value_name = "FMT", default_value = "arrow")]
    pub format: Format,

    /// Print absolute paths. Lexical only — symlinks are not resolved.
    #[arg(long)]
    pub absolute: bool,

    /// List the names keb never renames, and why. Renames nothing.
    #[arg(long)]
    pub list_protected: bool,
}

/// One record per rename on stdout. `arrow` is for reading; `json` and `null` are the
/// only two that survive a filename containing a newline or a literal ` -> `, both of
/// which are legal on Unix and both of which make `arrow` ambiguous to parse.
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// `old -> new`
    Arrow,
    /// The name before the rename.
    Old,
    /// The name after the rename.
    New,
    /// One JSON object per line: `{"from":"...","to":"..."}`.
    Json,
    /// `old\0new\0`, for `xargs -0`.
    Null,
}
