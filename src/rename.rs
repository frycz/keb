//! Performing the rename — `cases.md` §7 and §12.
//!
//! Three things make this more than a call to `fs::rename`:
//!
//! - **Case-only renames.** On a case-insensitive filesystem `File.md` and `file.md`
//!   are the same file, so `exists()` reports a collision that is not one. The tool
//!   compares inodes, not names, and goes through a temporary name when they match.
//! - **The journal.** The intent reaches disk before the syscall, so an interrupted
//!   two-step rename can be finished or reversed (§12, "Atomicity").
//! - **`git mv`.** Decision 10: inside a repository, a tracked file is moved with
//!   `git mv`, which preserves staged state and rename detection. It is run as an
//!   argv array with `--`, never a shell string, so a file named `$(rm -rf ~)` is
//!   just a name (§14).

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::journal::Journal;

pub struct Mover {
    git: Git,
    /// Overwrite an existing target, and rename protected names under `-r`.
    pub force: bool,
    dry_run: bool,
    counter: u32,
}

impl Mover {
    pub fn new(force: bool, dry_run: bool) -> Mover {
        Mover { git: Git::default(), force, dry_run, counter: 0 }
    }

    /// Rename `from` to `to`, journalling first.
    pub fn rename(&mut self, from: &Path, to: &Path, journal: &mut Journal) -> io::Result<()> {
        // A target that is the same file as the source is a case-only rename, and the
        // one case `fs::rename` cannot do in a single step on a case-insensitive
        // filesystem.
        let two_step = exists(to) && same_file(from, to);
        let via = two_step.then(|| self.temp_name(to)).flatten();

        if self.dry_run {
            return Ok(());
        }

        journal.plan(from, to, via.as_deref())?;

        if self.git.try_mv(from, to, self.force || two_step) {
            journal.commit(from, to)?;
            return Ok(());
        }

        match &via {
            Some(via) => {
                std::fs::rename(from, via)?;
                std::fs::rename(via, to)?;
            }
            None => std::fs::rename(from, to)?,
        }

        journal.commit(from, to)
    }

    /// An unused name in the target's own directory, so the intermediate step of a
    /// two-step rename never crosses a filesystem boundary.
    fn temp_name(&mut self, to: &Path) -> Option<PathBuf> {
        let dir = to.parent()?;
        for _ in 0..1000 {
            self.counter += 1;
            let candidate = dir.join(format!(".keb-{}-{}", std::process::id(), self.counter));
            if !exists(&candidate) {
                return Some(candidate);
            }
        }
        None
    }
}

/// Where a name should land, once collisions are resolved.
pub enum Target {
    /// Free, or the same file under a different spelling.
    Free(PathBuf),
    /// Taken by a different file, and `-f` says to replace it.
    Overwrite(PathBuf),
    /// Taken by a directory, or by so many suffixed siblings that there is no name
    /// left. Refuse: invariant 1 outranks finishing the job.
    Blocked(PathBuf),
}

/// Resolve `name` in `dir` to a free path, suffixing `-2`, `-3`… as needed.
///
/// This is where injectivity comes back. The transform is deliberately not injective —
/// `My File` and `my_file` both produce `my-file` — so two distinct files would
/// otherwise collapse into one (`cases.md`, "Architecture: two layers").
///
/// `claimed` holds every path this run has already taken or vacated, which is what
/// makes `--dry-run` predict a multi-file run correctly instead of reporting the same
/// free name twice.
pub fn resolve(
    dir: &Path,
    name: &str,
    from: &Path,
    claimed: &Claims,
    force: bool,
    opts: &keb::Options,
) -> Target {
    let mut candidate = name.to_string();

    for n in 2.. {
        let to = dir.join(&candidate);

        let taken = if claimed.vacated.contains(&to) {
            // The file that was here has already moved out of the way.
            None
        } else if claimed.taken.contains(&to) {
            Some(false)
        } else {
            exists(&to).then(|| same_file(from, &to))
        };

        match taken {
            None | Some(true) => return Target::Free(to),
            Some(false) if force && !to.is_dir() => return Target::Overwrite(to),
            Some(false) => {}
        }

        candidate = keb::with_ordinal(name, n, opts);
        if candidate.is_empty() || n > 9_999 {
            return Target::Blocked(to);
        }
    }

    unreachable!("the loop returns or exhausts its bound")
}

/// Paths this run has spoken for.
#[derive(Default)]
pub struct Claims {
    /// Targets already assigned, plus sources left in place because they were already
    /// kebab case.
    pub taken: HashSet<PathBuf>,
    /// Sources renamed away, whose old names are free again.
    pub vacated: HashSet<PathBuf>,
}

/// Does the path exist, without following a final symlink?
///
/// `Path::exists` follows links and so reports `false` for a broken one — which would
/// let the tool rename straight over it (§14, "dangling symlink").
pub fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Are these two paths the same file?
#[cfg(unix)]
pub fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::symlink_metadata(a), std::fs::symlink_metadata(b)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

/// Without inode numbers the best available test is the one the filesystem itself
/// uses: NTFS is case-insensitive, so two paths differing only in case are one file.
#[cfg(not(unix))]
pub fn same_file(a: &Path, b: &Path) -> bool {
    let key = |p: &Path| p.as_os_str().to_string_lossy().to_lowercase();
    exists(a) && exists(b) && key(a) == key(b)
}

/// Repository lookup, cached so that `-r` over a large tree spawns `git` twice rather
/// than twice per file.
#[derive(Default)]
struct Git {
    /// Directory -> the repository it belongs to, if any.
    roots: HashMap<PathBuf, Option<PathBuf>>,
    /// Repository root -> every path it tracks.
    tracked: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl Git {
    /// Move a tracked file with `git mv`. Returns whether it succeeded; a failure is
    /// not an error, it just means the caller falls back to the syscall.
    fn try_mv(&mut self, from: &Path, to: &Path, force: bool) -> bool {
        // `git ls-files` speaks in absolute paths once joined to the repository root,
        // while the tool is usually handed relative ones. Resolve the *parent* only:
        // canonicalizing the whole path would follow a final symlink, and `keb`
        // renames the link rather than its target.
        let (Some(from), Some(to)) = (absolute(from), absolute(to)) else {
            return false;
        };
        let Some(root) = self.root_of(&from).cloned() else {
            return false;
        };
        // Directories never appear in `ls-files`, so they are offered to `git mv`
        // directly; it declines an untracked one and the caller falls back.
        if !from.is_dir() && !self.tracked_in(&root).contains(&from) {
            return false;
        }

        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(&root).arg("mv");
        if force {
            cmd.arg("-f");
        }
        // `--` first: a file really can be named `--force` (§14).
        cmd.arg("--").arg(&from).arg(&to);
        run(&mut cmd).is_some()
    }

    fn root_of(&mut self, path: &Path) -> Option<&PathBuf> {
        let dir = path.parent()?.to_path_buf();
        self.roots
            .entry(dir.clone())
            .or_insert_with(|| {
                let mut cmd = Command::new("git");
                cmd.arg("-C").arg(&dir).args(["rev-parse", "--show-toplevel"]);
                let root = PathBuf::from(run(&mut cmd)?.trim_end());
                // `--show-toplevel` resolves symlinks; the tracked set is keyed on
                // paths built the same way, so the two must agree.
                Some(root.canonicalize().unwrap_or(root))
            })
            .as_ref()
    }

    fn tracked_in(&mut self, root: &Path) -> &HashSet<PathBuf> {
        self.tracked.entry(root.to_path_buf()).or_insert_with(|| {
            let mut cmd = Command::new("git");
            cmd.arg("-C").arg(root).args(["ls-files", "-z"]);
            let Some(out) = run_raw(&mut cmd) else {
                return HashSet::new();
            };
            out.split(|&b| b == 0).filter(|s| !s.is_empty()).map(|s| root.join(os_str(s))).collect()
        })
    }
}

/// An absolute path to `path`, resolving every component but the last.
fn absolute(path: &Path) -> Option<PathBuf> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    Some(dir.canonicalize().ok()?.join(path.file_name()?))
}

fn run(cmd: &mut Command) -> Option<String> {
    run_raw(cmd).map(|out| String::from_utf8_lossy(&out).into_owned())
}

fn run_raw(cmd: &mut Command) -> Option<Vec<u8>> {
    let out = cmd.stderr(Stdio::null()).stdin(Stdio::null()).output().ok()?;
    out.status.success().then_some(out.stdout)
}

#[cfg(unix)]
fn os_str(bytes: &[u8]) -> &OsStr {
    std::os::unix::ffi::OsStrExt::from_bytes(bytes)
}

#[cfg(not(unix))]
fn os_str(bytes: &[u8]) -> std::ffi::OsString {
    std::ffi::OsString::from(String::from_utf8_lossy(bytes).into_owned())
}
