//! The `keb` binary — CLI parsing plus the stateful filesystem layer.
//!
//! This consumes the `keb` library exactly as any external crate would, which keeps
//! the pure transform independently testable (`cases.md` — "Architecture: two layers").
//!
//! The stream contract (`cases.md` §8) is the whole of the output design: renames go to
//! **stdout** as `old -> new`, warnings and errors to **stderr**. That split is what
//! `--verbose` and `--quiet` would have been, so neither flag exists.

mod journal;
mod protect;
mod rename;
mod walk;

use std::collections::HashSet;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

use journal::Journal;
use rename::{Claims, Mover, Target};
use walk::Filter;

/// Rename files to kebab case, safely and idempotently.
///
/// Only the basename changes; parent directories are never touched. With no paths,
/// reads a list from stdin, so `find . -name '*.md' | keb` works.
#[derive(Parser)]
#[command(name = "keb", version, about, long_about = None)]
struct Cli {
    /// Paths to rename. `-` reads a list from stdin.
    paths: Vec<PathBuf>,

    /// Print the plan, change nothing.
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Recurse into directories.
    #[arg(short = 'r', long)]
    recursive: bool,

    /// Overwrite on collision, and rename protected names under -r.
    #[arg(short = 'f', long)]
    force: bool,

    /// Undo the most recent run.
    #[arg(long, conflicts_with_all = ["recursive", "force", "paths"])]
    undo: bool,

    /// Paths on stdin are null-separated, for `find -print0`.
    #[arg(short = '0', long = "null")]
    null: bool,

    /// Under -r, visit directories only.
    #[arg(long, conflicts_with = "files_only", requires = "recursive")]
    dirs_only: bool,

    /// Under -r, visit files only.
    #[arg(long, requires = "recursive")]
    files_only: bool,

    /// Emit this character between words instead of `-`.
    #[arg(long, value_name = "CHAR")]
    separator: Option<char>,

    /// Narrow the output to ASCII, dropping scripts that are otherwise kept.
    #[arg(long)]
    ascii: bool,

    /// Override the 255 byte / UTF-16 unit name limit.
    #[arg(long, value_name = "N")]
    max_length: Option<usize>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(&cli) {
        Ok(Outcome::Clean) => ExitCode::from(0),
        Ok(Outcome::Partial) => ExitCode::from(1),
        Err(e) => {
            eprintln!("keb: {e}");
            ExitCode::from(2)
        }
    }
}

enum Outcome {
    Clean,
    Partial,
}

fn run(cli: &Cli) -> io::Result<Outcome> {
    let opts = options(cli)?;
    // `--undo` reads the journal even under `-n`; only a dry *rename* writes nothing.
    let mut journal = if cli.dry_run && !cli.undo { Journal::disabled() } else { Journal::open()? };

    if cli.undo {
        return undo(cli, &mut journal);
    }

    let inputs = inputs(cli)?;
    if inputs.is_empty() {
        return Err(io::Error::other("no paths given; pass paths as arguments or on stdin"));
    }

    let mut mover = Mover::new(cli.force, cli.dry_run);
    let mut claims = Claims::default();
    let mut partial = false;

    journal.begin()?;
    for item in expand(&inputs, cli, &mut partial) {
        if !step(&item, &opts, &mut mover, &mut claims, &mut journal)? {
            partial = true;
        }
    }

    Ok(if partial { Outcome::Partial } else { Outcome::Clean })
}

/// One path, and whether the user named it or `-r` swept it up. Decision 4 turns on
/// exactly that difference.
struct Item {
    path: PathBuf,
    explicit: bool,
}

/// Plan and perform one rename. `false` means the run is partial.
fn step(
    item: &Item,
    opts: &keb::Options,
    mover: &mut Mover,
    claims: &mut Claims,
    journal: &mut Journal,
) -> io::Result<bool> {
    let path = &item.path;

    let Some(raw) = path.file_name() else {
        eprintln!("keb: {}: no basename to rename", path.display());
        return Ok(false);
    };
    if raw == "." || raw == ".." {
        eprintln!("keb: {}: refusing to rename a path component", path.display());
        return Ok(false);
    }

    // Decision 3: a filename on Unix is bytes, not text. Decode lossily and rename
    // anyway, but say what was dropped — the new name cannot round-trip to the old.
    let name = raw.to_string_lossy();
    if matches!(name, std::borrow::Cow::Owned(_)) {
        let dropped = name.matches('\u{fffd}').count();
        eprintln!("keb: {}: not valid UTF-8, {dropped} byte(s) dropped", path.display());
    }

    let new = keb::kebab_with(&name, opts);
    if new.is_empty() {
        eprintln!("keb: {}: no kebab form for this name, skipped", path.display());
        return Ok(false);
    }
    if *new == *name {
        // Already kebab case — silent, per the stream contract, but still spoken for
        // so that a sibling renaming onto it gets suffixed instead.
        claims.taken.insert(path.clone());
        return Ok(true);
    }

    if let Some(why) = protect::reason(&name)
        && !mover.force
    {
        // Decision 4: a file named on the command line was chosen, so warn and do it.
        // A file `-r` found was not, so leave it alone.
        if item.explicit {
            eprintln!("keb: {}: renaming {why}", path.display());
        } else {
            eprintln!("keb: {}: skipped, {why} (-f renames it)", path.display());
            claims.taken.insert(path.clone());
            return Ok(true);
        }
    }

    let dir = path.parent().unwrap_or(Path::new(""));

    // `-f` overwrites a file, but never a directory: replacing one would mean deleting
    // whatever is inside it, which invariant 1 does not allow at any force level. Fall
    // back to suffixing, and say so rather than looking like the flag was ignored.
    if mover.force && dir.join(&new).is_dir() && !rename::same_file(path, &dir.join(&new)) {
        eprintln!(
            "keb: {}: a directory is in the way, suffixing instead",
            dir.join(&new).display()
        );
    }

    let to = match rename::resolve(dir, &new, path, claims, mover.force, opts) {
        Target::Free(to) => to,
        Target::Overwrite(to) => {
            eprintln!("keb: {}: overwriting", to.display());
            to
        }
        Target::Blocked(to) => {
            eprintln!("keb: {}: {} exists and is not the same file", path.display(), to.display());
            return Ok(false);
        }
    };

    if let Err(e) = mover.rename(path, &to, journal) {
        eprintln!("keb: {}: {e}", path.display());
        return Ok(false);
    }

    // The target, not the transformed name: a collision may have suffixed it, and the
    // line the user reads has to be the rename that actually happened.
    println!("{} -> {}", path.display(), to.display());
    claims.taken.insert(to);
    claims.vacated.insert(path.clone());
    Ok(true)
}

/// Turn the input list into the work list: every path, deepest first, with duplicates
/// removed.
fn expand(inputs: &[PathBuf], cli: &Cli, partial: &mut bool) -> Vec<Item> {
    let filter = match (cli.dirs_only, cli.files_only) {
        (true, _) => Filter::DirsOnly,
        (_, true) => Filter::FilesOnly,
        _ => Filter::All,
    };

    let mut items = Vec::new();
    let mut seen = HashSet::new();

    for input in inputs {
        if !rename::exists(input) {
            eprintln!("keb: {}: no such file or directory", input.display());
            *partial = true;
            continue;
        }

        // The children come first and deepest-first, so that renaming a directory
        // never invalidates a path still queued beneath it.
        if cli.recursive && input.is_dir() {
            let walk = walk::collect(input, filter);
            for (path, e) in walk.errors {
                eprintln!("keb: {}: {e}", path.display());
                *partial = true;
            }
            items.extend(walk.paths.into_iter().map(|path| Item { path, explicit: false }));
        }

        items.push(Item { path: input.clone(), explicit: true });
    }

    // A path that is both swept up and named on the command line was still chosen, and
    // decision 4 turns on that. The swept copy comes first, so mark before deduping.
    let named: HashSet<&PathBuf> = inputs.iter().collect();
    for item in &mut items {
        item.explicit |= named.contains(&item.path);
    }

    items.retain(|i| seen.insert(i.path.clone()));
    items
}

/// Replay the most recent run backwards (`cases.md` §8).
fn undo(cli: &Cli, journal: &mut Journal) -> io::Result<Outcome> {
    let entries = journal.last_run()?;
    if entries.is_empty() {
        eprintln!("keb: nothing to undo in {}", journal.location().display());
        return Ok(Outcome::Clean);
    }

    let mut mover = Mover::new(true, cli.dry_run);
    let mut discard = Journal::disabled();
    let mut partial = false;
    // Whether anything is left that a second `--undo` could still achieve. A record
    // whose file is simply gone is spent either way; a record that failed to move is
    // worth keeping, so the user can fix the permission and try again.
    let mut retryable = false;

    // The run renamed deepest-first, so undoing it means shallowest-first.
    for entry in entries.iter().rev() {
        // An uncommitted record is an interrupted two-step rename: the file is sitting
        // under its temporary name, which is why the journal is write-ahead.
        let from = match (entry.committed, &entry.via) {
            (true, _) => entry.to.clone(),
            (false, Some(via)) if rename::exists(via) => via.clone(),
            (false, _) => continue,
        };
        if !rename::exists(&from) {
            eprintln!("keb: {}: gone, cannot undo", from.display());
            partial = true;
            continue;
        }
        // Something has taken the original name back since. Undo restores names; it
        // does not destroy whatever moved in, at any force level.
        if rename::exists(&entry.from) && !rename::same_file(&from, &entry.from) {
            eprintln!("keb: {}: occupied, cannot undo", entry.from.display());
            partial = true;
            retryable = true;
            continue;
        }

        match mover.rename(&from, &entry.from, &mut discard) {
            Ok(()) => println!("{} -> {}", from.display(), entry.from.display()),
            Err(e) => {
                eprintln!("keb: {}: {e}", from.display());
                partial = true;
                retryable = true;
            }
        }
    }

    if !cli.dry_run && !retryable {
        journal.drop_last_run()?;
    }
    Ok(if partial { Outcome::Partial } else { Outcome::Clean })
}

fn options(cli: &Cli) -> io::Result<keb::Options> {
    let mut opts = keb::Options { ascii: cli.ascii, ..keb::Options::default() };

    if let Some(sep) = cli.separator {
        if sep == '/' || sep == '.' || sep.is_control() || std::path::is_separator(sep) {
            return Err(io::Error::other(format!("{sep:?} cannot separate words in a filename")));
        }
        opts.separator = sep;
    }
    if let Some(max) = cli.max_length {
        if max < 2 {
            return Err(io::Error::other("--max-length must be at least 2"));
        }
        opts.max_length = max;
    }

    Ok(opts)
}

/// The paths to work on: the arguments, with stdin spliced in wherever `-` appears —
/// or read from stdin outright when no argument is given and it is not a terminal.
fn inputs(cli: &Cli) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();

    if cli.paths.is_empty() {
        if io::stdin().is_terminal() {
            return Ok(out);
        }
        return read_stdin(cli.null);
    }

    for path in &cli.paths {
        match path.as_os_str() == "-" {
            true => out.extend(read_stdin(cli.null)?),
            false => out.push(expand_home(path)),
        }
    }
    Ok(out)
}

fn read_stdin(null: bool) -> io::Result<Vec<PathBuf>> {
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;

    let sep = if null { 0 } else { b'\n' };
    Ok(buf
        .split(|&b| b == sep)
        .map(|line| match null {
            true => line,
            // Tolerate a list produced on Windows.
            false => line.strip_suffix(b"\r").unwrap_or(line),
        })
        .filter(|line| !line.is_empty())
        .map(|line| expand_home(Path::new(&os_string(line))))
        .collect())
}

/// `~/x` means the home directory, including in a list arriving on stdin, where no
/// shell has had a chance to expand it (`cases.md` §6). A bare `~` is left alone — it
/// is a legal filename.
fn expand_home(path: &Path) -> PathBuf {
    let Some(rest) = path.to_str().and_then(|s| s.strip_prefix("~/")) else {
        return path.to_path_buf();
    };
    match std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        Some(home) => PathBuf::from(home).join(rest),
        None => path.to_path_buf(),
    }
}

#[cfg(unix)]
fn os_string(bytes: &[u8]) -> OsString {
    <OsString as std::os::unix::ffi::OsStringExt>::from_vec(bytes.to_vec())
}

#[cfg(not(unix))]
fn os_string(bytes: &[u8]) -> OsString {
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}
