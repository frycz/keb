//! The `keb` binary — CLI parsing plus the stateful filesystem layer.
//!
//! This consumes the `keb` library exactly as any external crate would, which keeps
//! the pure transform independently testable (`cases.md` — "Architecture: two layers").
//!
//! Status: stub. `--dry-run` prints the plan; renaming is not yet wired up.

use std::path::{Path, PathBuf};

use clap::Parser;

/// Rename files to kebab case, safely and idempotently.
#[derive(Parser)]
#[command(name = "keb", version, about, long_about = None)]
struct Cli {
    /// Paths to rename. Only the basename is changed; parents are untouched.
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Print the plan, change nothing.
    #[arg(short = 'n', long)]
    dry_run: bool,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let mut exit = 0u8;

    for path in &cli.paths {
        match plan(path) {
            // already kebab — silent no-op
            Some((from, to)) if from == to => {}
            Some((from, to)) => {
                // Stream contract (cases.md §8): renames on stdout, one per line.
                println!("{} -> {}", from.display(), to.display());
                if !cli.dry_run {
                    eprintln!("keb: renaming is not implemented yet (stub build)");
                    exit = 1;
                }
            }
            None => {
                eprintln!("keb: cannot rename {}", path.display());
                exit = 1;
            }
        }
    }

    std::process::ExitCode::from(exit)
}

/// Compute the rename for one path, or `None` if it has no usable basename.
fn plan(path: &Path) -> Option<(PathBuf, PathBuf)> {
    let name = path.file_name()?.to_str()?;
    let renamed = keb::kebab(name);
    if renamed.is_empty() {
        return None;
    }
    Some((path.to_path_buf(), path.with_file_name(renamed)))
}
