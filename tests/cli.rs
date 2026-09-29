//! End-to-end tests for the filesystem layer — `cases.md` §6, §7 and §12.
//!
//! These drive the real binary against a real directory, because everything being
//! tested here is a property of the filesystem rather than of the transform: inode
//! identity on a case-insensitive volume, the ordering of a recursive sweep, what the
//! output looks like once a rename has actually happened. None of it is observable from
//! a pure function.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// A scratch directory, removed when the test ends.
struct Sandbox {
    dir: PathBuf,
}

static SEQ: AtomicU32 = AtomicU32::new(0);

impl Sandbox {
    fn new() -> Sandbox {
        let id = SEQ.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("keb-cli-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = root.join("work");
        fs::create_dir_all(&dir).unwrap();
        Sandbox { dir }
    }

    fn touch(&self, name: &str) -> PathBuf {
        let path = self.dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, name).unwrap();
        path
    }

    fn mkdir(&self, name: &str) -> PathBuf {
        let path = self.dir.join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }

    /// Run `keb` in the sandbox. Arguments are passed as an argv array, so a filename
    /// containing shell metacharacters stays a filename (`cases.md` §14).
    fn keb<I, S>(&self, args: I) -> Run
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let out = Command::new(env!("CARGO_BIN_EXE_keb"))
            .current_dir(&self.dir)
            .args(args)
            .output()
            .expect("keb should run");
        Run::from(out)
    }

    /// Every entry under the sandbox, relative and sorted — the whole observable state.
    /// Joined with `/` on every platform, so one expectation serves Windows too.
    fn tree(&self) -> Vec<String> {
        fn visit(dir: &Path, base: &Path, out: &mut Vec<String>) {
            let mut entries: Vec<_> =
                fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
            entries.sort();
            for path in entries {
                if path.file_name() == Some(OsStr::new(".git")) {
                    continue;
                }
                let rel = path.strip_prefix(base).unwrap();
                let parts: Vec<_> = rel.iter().map(|c| c.to_string_lossy()).collect();
                out.push(parts.join("/"));
                if path.is_dir() {
                    visit(&path, base, out);
                }
            }
        }
        let mut out = Vec::new();
        visit(&self.dir, &self.dir, &mut out);
        out
    }

    /// What `--absolute` should print for `rel`. `absolute()` prefixes the kernel's cwd,
    /// which on macOS is already symlink-resolved: the system temp dir is reached through
    /// one (/var -> /private/var). That is unavoidable and harmless, so resolve it here
    /// too. Not on Windows, where `canonicalize` returns the `\\?\` verbatim form and
    /// expands 8.3 short names (`RUNNER~1`) that the cwd keeps; `absolute()` there also
    /// turns every `/` into `\`, which is why the expectation goes through it as well.
    fn absolute(&self, rel: &str) -> String {
        #[cfg(unix)]
        let root = self.dir.canonicalize().unwrap();
        #[cfg(not(unix))]
        let root = self.dir.clone();
        std::path::absolute(root.join(rel)).unwrap().display().to_string()
    }

    fn exists(&self, name: &str) -> bool {
        fs::symlink_metadata(self.dir.join(name)).is_ok()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.dir.parent().unwrap());
    }
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

impl From<Output> for Run {
    fn from(out: Output) -> Run {
        Run {
            code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }
}

impl Run {
    /// Renames, one per line, in the order they happened.
    fn renames(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }
}

/// Is this a case-insensitive volume? The two-step rename only matters where it is,
/// and CI runs on both kinds.
fn case_insensitive(dir: &Path) -> bool {
    let probe = dir.join("KebCaseProbe");
    fs::write(&probe, "").unwrap();
    let insensitive = dir.join("kebcaseprobe").exists();
    fs::remove_file(&probe).unwrap();
    insensitive
}

// ── §6 Path semantics ────────────────────────────────────────────────────────────

#[test]
fn renames_only_the_basename() {
    let s = Sandbox::new();
    s.touch("A Dir/My File.md");

    let run = s.keb(["./A Dir/My File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.renames(), ["./A Dir/My File.md -> ./A Dir/my-file.md"]);
    assert_eq!(s.tree(), ["A Dir", "A Dir/my-file.md"]);
}

/// Only the basename changes, so a Windows user who typed `\` gets `\` back, and one who
/// typed `/` gets `/` — never a path that mixes the two because keb joined it.
#[cfg(windows)]
#[test]
fn keeps_the_separator_the_user_typed() {
    let s = Sandbox::new();
    s.touch("A Dir/My File.md");
    s.touch("B Dir/My File.md");

    let run = s.keb(["-n", r".\A Dir\My File.md", "./B Dir/My File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        run.renames(),
        [r".\A Dir\My File.md -> .\A Dir\my-file.md", "./B Dir/My File.md -> ./B Dir/my-file.md"]
    );
}

#[test]
fn already_kebab_is_silent() {
    let s = Sandbox::new();
    s.touch("my-file.md");

    let run = s.keb(["my-file.md"]);
    assert_eq!(run.code, 0);
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, "");
}

#[test]
fn running_twice_changes_nothing() {
    // cases.md §7 — "Idempotency: running keb x twice must be a no-op. Non-negotiable."
    let s = Sandbox::new();
    s.touch("My File.md");

    assert_eq!(s.keb(["My File.md"]).renames().len(), 1);
    let second = s.keb(["my-file.md"]);
    assert_eq!(second.stdout, "");
    assert_eq!(s.tree(), ["my-file.md"]);
}

#[test]
fn dry_run_changes_nothing() {
    let s = Sandbox::new();
    s.touch("My File.md");

    let run = s.keb(["-n", "My File.md"]);
    assert_eq!(run.renames(), ["My File.md -> my-file.md"]);
    assert_eq!(s.tree(), ["My File.md"]);
}

#[test]
fn reads_a_list_from_stdin() {
    let s = Sandbox::new();
    s.touch("One File.md");
    s.touch("Two File.md");

    let out = Command::new(env!("CARGO_BIN_EXE_keb"))
        .current_dir(&s.dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"./One File.md\n./Two File.md\n")?;
            child.wait_with_output()
        })
        .unwrap();

    assert!(out.status.success());
    assert_eq!(s.tree(), ["one-file.md", "two-file.md"]);
}

#[test]
fn a_filename_that_looks_like_a_flag() {
    // cases.md §14 — a file literally named `-rf`
    let s = Sandbox::new();
    s.touch("-rf");
    s.touch("Keep Me.md");

    let run = s.keb(["--", "-rf", "Keep Me.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["keep-me.md", "rf"]);
}

#[test]
fn shell_metacharacters_are_just_characters() {
    // cases.md §14 — harmless if and only if the tool never shells out
    let s = Sandbox::new();
    s.touch("$(touch pwned) `id`.md");

    let run = s.keb(["--", "$(touch pwned) `id`.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(!s.exists("pwned"));
    assert_eq!(s.tree(), ["touch-pwned-id.md"]);
}

// ── §7 Collisions and filesystem reality ─────────────────────────────────────────

#[test]
fn case_only_rename_is_not_a_collision() {
    // cases.md §7 — on macOS `File.md` -> `file.md` is a same-inode rename, and a naive
    // exists() check calls it a collision
    let s = Sandbox::new();
    s.touch("FILE.md");

    let run = s.keb(["FILE.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.renames(), ["FILE.md -> file.md"]);
    assert_eq!(s.tree(), ["file.md"]);
    assert_eq!(fs::read_to_string(s.dir.join("file.md")).unwrap(), "FILE.md");

    if case_insensitive(&s.dir) {
        // the two-step rename must not leave its scaffolding behind
        assert!(s.tree().iter().all(|n| !n.starts_with(".keb-")));
    }
}

#[test]
fn two_names_collapsing_to_one_get_suffixed() {
    // cases.md — the transform is not injective; the filesystem layer restores it
    let s = Sandbox::new();
    s.touch("My File.md");
    s.touch("my_file.md");

    let run = s.keb(["My File.md", "my_file.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["my-file-2.md", "my-file.md"]);
    // the reported line is the rename that happened, suffix and all
    assert_eq!(run.renames(), ["My File.md -> my-file.md", "my_file.md -> my-file-2.md"]);
    // no content was lost, and nothing was overwritten
    let mut contents: Vec<String> =
        s.tree().iter().map(|n| fs::read_to_string(s.dir.join(n)).unwrap()).collect();
    contents.sort();
    assert_eq!(contents, ["My File.md", "my_file.md"]);
}

#[test]
fn an_occupied_target_is_never_overwritten_by_default() {
    let s = Sandbox::new();
    s.touch("My File.md");
    s.touch("my-file.md");

    let run = s.keb(["My File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["my-file-2.md", "my-file.md"]);
    assert_eq!(fs::read_to_string(s.dir.join("my-file.md")).unwrap(), "my-file.md");
}

#[test]
fn force_overwrites() {
    let s = Sandbox::new();
    s.touch("My File.md");
    s.touch("my-file.md");

    let run = s.keb(["-f", "My File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["my-file.md"]);
    assert_eq!(fs::read_to_string(s.dir.join("my-file.md")).unwrap(), "My File.md");
}

#[test]
fn force_never_overwrites_a_directory() {
    // Replacing a directory means deleting what is inside it, which invariant 1 does
    // not allow at any force level (`cases.md` §14).
    let s = Sandbox::new();
    s.touch("My File.md");
    s.mkdir("my-file.md");

    let run = s.keb(["-f", "My File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stderr.contains("directory is in the way"));
    assert!(s.dir.join("my-file.md").is_dir());
    assert_eq!(fs::read_to_string(s.dir.join("my-file-2.md")).unwrap(), "My File.md");
}

#[test]
fn an_empty_result_is_refused_not_invented() {
    // cases.md §7 — `🚀.md` has no kebab form; inventing `untitled` loses the only
    // thing that distinguished it
    let s = Sandbox::new();
    s.touch("🚀.md");

    let run = s.keb(["🚀.md"]);
    assert_eq!(run.code, 1);
    assert!(run.stderr.contains("no kebab form"));
    assert_eq!(s.tree(), ["🚀.md"]);
}

#[test]
fn a_missing_path_is_partial_not_fatal() {
    // cases.md §8 — a failed rename does not abort the run
    let s = Sandbox::new();
    s.touch("My File.md");

    let run = s.keb(["Nope.md", "My File.md"]);
    assert_eq!(run.code, 1);
    assert!(run.stderr.contains("no such file"));
    assert_eq!(s.tree(), ["my-file.md"]);
}

#[cfg(unix)]
#[test]
fn renames_the_symlink_not_its_target() {
    let s = Sandbox::new();
    s.touch("target.md");
    std::os::unix::fs::symlink("target.md", s.dir.join("My Link.md")).unwrap();

    let run = s.keb(["My Link.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["my-link.md", "target.md"]);
    assert!(fs::symlink_metadata(s.dir.join("my-link.md")).unwrap().is_symlink());
}

#[cfg(unix)]
#[test]
fn a_broken_symlink_in_the_way_still_counts_as_occupied() {
    // cases.md §14 — `Path::exists` follows the link and reports false for a broken
    // one, which would let the rename land straight on top of it
    let s = Sandbox::new();
    s.touch("My File.md");
    std::os::unix::fs::symlink("nowhere", s.dir.join("my-file.md")).unwrap();

    let run = s.keb(["My File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(s.exists("my-file-2.md"));
    assert!(fs::symlink_metadata(s.dir.join("my-file.md")).unwrap().is_symlink());
}

// ── §6 / §7 Recursion ────────────────────────────────────────────────────────────

#[test]
fn a_directory_alone_renames_only_itself() {
    let s = Sandbox::new();
    s.touch("My Dir/My File.md");

    let run = s.keb(["-d", "My Dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["my-dir", "my-dir/My File.md"]);
}

/// `keb -r .` is the first thing anyone types in a project. The root has no name to
/// rename, and saying so would make every such run exit 1.
#[test]
fn recursion_from_dot_is_a_clean_run() {
    let s = Sandbox::new();
    s.touch("Sub Dir/My File.md");

    for flags in ["-r", "-dr"] {
        let run = s.keb(["-n", flags, "."]);
        assert_eq!((run.code, run.stderr.as_str()), (0, ""), "keb -n {flags} .");
    }

    let run = s.keb(["-r", "."]);
    assert_eq!((run.code, run.stderr.as_str()), (0, ""));
    assert_eq!(s.tree(), ["Sub Dir", "Sub Dir/my-file.md"]);
}

#[test]
fn recursion_from_dot_dot_is_a_clean_run() {
    let s = Sandbox::new();
    s.touch("Sub Dir/My File.md");

    let run = Command::new(env!("CARGO_BIN_EXE_keb"))
        .current_dir(s.dir.join("Sub Dir"))
        .args(["-n", "-r", ".."])
        .output()
        .map(Run::from)
        .unwrap();
    assert_eq!((run.code, run.stderr.as_str()), (0, ""));
    assert_eq!(run.renames().len(), 1, "{}", run.stdout);
}

/// Without `-r`, `.` was not named to be swept, so it is still an error.
#[test]
fn a_bare_dot_without_recursion_is_refused() {
    let s = Sandbox::new();

    let run = s.keb(["-d", "."]);
    assert_eq!(run.code, 1);
    assert!(run.stderr.contains("no basename to rename"), "{}", run.stderr);
}

#[test]
fn recursion_renames_deepest_first() {
    // cases.md §6 — otherwise the parent's rename invalidates every queued child path
    let s = Sandbox::new();
    s.touch("My Dir/Sub Dir/Deep File.md");
    s.touch("My Dir/Top File.md");

    let run = s.keb(["-dr", "My Dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        s.tree(),
        ["my-dir", "my-dir/sub-dir", "my-dir/sub-dir/deep-file.md", "my-dir/top-file.md"]
    );

    let renames = run.renames();
    let depth = |line: &str| line.split(" -> ").next().unwrap().matches('/').count();
    assert!(
        renames.windows(2).all(|w| depth(w[0]) >= depth(w[1])),
        "not deepest-first: {renames:?}"
    );
}

/// Without `-d` a recursive sweep is files-only, which is what `--files-only` used to
/// spell. The named directory is the traversal root, so it is passed over in silence
/// rather than refused: naming it was how the sweep was asked for.
#[test]
fn recursion_without_allow_dirs_renames_only_files() {
    let s = Sandbox::new();
    s.touch("My Dir/Sub Dir/Deep File.md");

    let run = s.keb(["-r", "My Dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stderr, "", "the traversal root must not be complained about");
    assert_eq!(s.tree(), ["My Dir", "My Dir/Sub Dir", "My Dir/Sub Dir/deep-file.md"]);
}

#[test]
fn recursion_with_allow_dirs_renames_directories_too() {
    let s = Sandbox::new();
    s.touch("My Dir/Sub Dir/Deep File.md");

    let run = s.keb(["-dr", "My Dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["my-dir", "my-dir/sub-dir", "my-dir/sub-dir/deep-file.md"]);
}

// ── §13 Protected names ──────────────────────────────────────────────────────────

#[test]
fn a_named_protected_file_is_renamed_with_a_warning() {
    // Decision 4: never blocks an explicitly named file
    let s = Sandbox::new();
    s.touch("Makefile");

    let run = s.keb(["Makefile"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stderr.contains("Makefile"));
    assert_eq!(s.tree(), ["makefile"]);
}

#[test]
fn a_swept_protected_file_is_skipped() {
    // Decision 4: under -r the user did not pick each file
    let s = Sandbox::new();
    s.touch("My Dir/Makefile");
    s.touch("My Dir/MyClass.java");
    s.touch("My Dir/My File.md");

    let run = s.keb(["-dr", "My Dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stderr.contains("skipped"));
    assert_eq!(s.tree(), ["my-dir", "my-dir/Makefile", "my-dir/MyClass.java", "my-dir/my-file.md"]);

    // ...and -f renames them after all
    let forced = s.keb(["-dr", "-f", "my-dir"]);
    assert_eq!(forced.code, 0, "{}", forced.stderr);
    assert_eq!(
        s.tree(),
        ["my-dir", "my-dir/makefile", "my-dir/my-class.java", "my-dir/my-file.md"]
    );
}

// ── Decision 10: git ─────────────────────────────────────────────────────────────

impl Sandbox {
    fn git(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .current_dir(&self.dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .args(args)
            .output()
            .expect("git should run");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn init_repo(&self) {
        self.git(&["init", "-q", "."]);
        self.git(&["config", "user.email", "keb@test"]);
        self.git(&["config", "user.name", "keb"]);
    }
}

#[test]
fn tracked_files_move_with_git_mv() {
    // Decision 10 — `git mv` preserves staged state and rename detection, so the
    // inference is never wrong
    let s = Sandbox::new();
    s.init_repo();
    s.touch("My File.md");
    s.touch("Untracked File.md");
    s.git(&["add", "My File.md"]);
    s.git(&["commit", "-qm", "init"]);

    let run = s.keb(["My File.md", "Untracked File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);

    let status = s.git(&["status", "--short"]);
    // the tracked one is a staged rename, not a delete plus an untracked file
    assert!(status.contains("R "), "expected a staged rename:\n{status}");
    assert!(status.contains("my-file.md"), "{status}");
    // the untracked one is renamed all the same, just outside the index
    assert!(status.contains("?? untracked-file.md"), "{status}");
}

#[test]
fn a_case_only_rename_of_a_tracked_file() {
    // The two hard cases at once: same-inode rename on a case-insensitive volume, and
    // an index that has to follow it
    let s = Sandbox::new();
    s.init_repo();
    s.touch("README.MD");
    s.git(&["add", "README.MD"]);
    s.git(&["commit", "-qm", "init"]);

    let run = s.keb(["README.MD"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["readme.md"]);
    assert!(s.git(&["status", "--short"]).contains("readme.md"));
}

// ── §8 Transform flags ───────────────────────────────────────────────────────────

#[test]
fn separator_and_ascii() {
    let s = Sandbox::new();
    s.touch("My File.md");
    s.touch("日本語 Report.md");

    assert_eq!(s.keb(["--separator=_", "My File.md"]).code, 0);
    assert!(s.exists("my_file.md"));

    assert_eq!(s.keb(["--ascii", "日本語 Report.md"]).code, 0);
    assert!(s.exists("report.md"));
}

#[test]
fn a_separator_that_would_break_the_path_is_refused() {
    let s = Sandbox::new();
    let run = s.keb(["--separator=/", "x"]);
    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("cannot separate"));
}

#[test]
fn max_length_truncates_without_eating_the_extension() {
    let s = Sandbox::new();
    s.touch("A Very Long Name Indeed.md");

    let run = s.keb(["--max-length=12", "A Very Long Name Indeed.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["a-very-lo.md"]);
}

// ── Usage ────────────────────────────────────────────────────────────────────────

/// A bare `keb` shows `--help`, but only at a terminal. This runs the binary with stdin
/// closed rather than on a pty, so it pins the *other* half of that rule: a pipeline
/// supplies its paths on stdin and legitimately passes no arguments, so the no-argument
/// case must not be intercepted during parsing. Reaching for clap's
/// `arg_required_else_help` is exactly the change this catches — it would also break
/// `reads_a_list_from_stdin`, which is the same rule seen from the other side.
#[test]
fn no_paths_without_a_terminal_is_an_error_not_help() {
    let s = Sandbox::new();
    s.touch("My File.md");

    let run = s.keb::<[&str; 0], &str>([]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(run.stderr.contains("no paths given"), "got: {}", run.stderr);
    assert!(!run.stderr.contains("Usage:"), "help does not belong in a script's stderr");
    assert!(run.stdout.is_empty(), "nothing was renamed, so stdout stays empty");
    // The one-liner has to be recoverable without a second guess.
    assert!(run.stderr.contains("--help"), "got: {}", run.stderr);
    assert_eq!(s.tree(), ["My File.md"]);
}

/// `-h` carries the examples and the no-undo warning, not just `--help`: a bare `keb` at
/// a prompt prints the help, and that is the only place someone who has not read the
/// README learns to run `-n` first.
#[test]
fn short_help_carries_the_examples_and_the_warning() {
    let s = Sandbox::new();

    let run = s.keb(["-h"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    for expected in ["Examples:", "keb -n *", "find . -name '*.md' | keb", "cannot be undone"] {
        assert!(run.stdout.contains(expected), "{expected:?} missing from: {}", run.stdout);
    }
}

/// The protect list is the tool's most surprising behaviour, so the binary can print it
/// rather than only the README describing it. It is requested output, so it goes to
/// stdout — `keb --list-protected | grep Makefile` is the point — and nothing is renamed.
#[test]
fn list_protected_prints_the_list_and_renames_nothing() {
    let s = Sandbox::new();
    s.touch("My File.md");

    let run = s.keb(["--list-protected"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    for expected in ["Makefile", "[*", "*.xcodeproj", "*.java", "COM1-COM9"] {
        assert!(run.stdout.contains(expected), "{expected:?} missing from: {}", run.stdout);
    }
    assert_eq!(s.tree(), ["My File.md"], "listing renames nothing");
}

// ── Input ordering ───────────────────────────────────────────────────────────────

/// `walk::collect` renames deepest-first so a renamed parent cannot invalidate a path
/// still queued beneath it. Paths given on the command line or on stdin need the same
/// ordering, because `find` emits parents first: `find -type d | keb` renamed the parent
/// and then could not find its children.
#[test]
fn an_ancestor_named_before_its_descendants_is_renamed_last() {
    let s = Sandbox::new();
    s.touch("Dir_A/Sub_B/Deep_X.md");

    let run = s.keb(["-d", "Dir_A", "Dir_A/Sub_B", "Dir_A/Sub_B/Deep_X.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["dir-a", "dir-a/sub-b", "dir-a/sub-b/deep-x.md"]);
}

/// The sort is by depth only, and stable, so two paths at the same depth keep the order
/// they were given. Collision suffixing turns on that: whichever of two colliding names
/// is reached first keeps the plain form.
#[test]
fn reordering_does_not_disturb_collision_suffixes() {
    let s = Sandbox::new();
    s.touch("My File.md");
    s.touch("my_file.md");

    let run = s.keb(["my_file.md", "My File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["my-file-2.md", "my-file.md"], "the first given name won");
}

// ── -d: the directory gate ───────────────────────────────────────────────────────

/// A directory named on its own is refused. `keb dir1/dir2` is `keb dir1/dir2/file.png`
/// with the Tab taken one stop early, and the mistake is otherwise silent.
#[test]
fn a_named_directory_needs_allow_dirs() {
    let s = Sandbox::new();
    s.touch("My Dir/Keep Me.md");

    let run = s.keb(["My Dir"]);
    assert_eq!(run.code, 1, "{}", run.stderr);
    assert!(
        run.stderr.contains("is a directory, skipped (-d allows directories renaming)"),
        "{}",
        run.stderr
    );
    assert_eq!(s.tree(), ["My Dir", "My Dir/Keep Me.md"], "nothing moved");
}

#[test]
fn allow_dirs_renames_the_directory_and_nothing_inside_it() {
    let s = Sandbox::new();
    s.touch("My Dir/Keep Me.md");

    let run = s.keb(["-d", "My Dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["my-dir", "my-dir/Keep Me.md"]);
}

/// The gate gets its answer from `symlink_metadata`, so a symlink pointing at a directory
/// is not a directory: renaming the link only changes a name and cannot restructure
/// anything.
#[cfg(unix)]
#[test]
fn a_symlink_to_a_directory_is_not_gated() {
    let s = Sandbox::new();
    s.mkdir("Real Dir");
    std::os::unix::fs::symlink(s.dir.join("Real Dir"), s.dir.join("Link To Dir")).unwrap();

    let run = s.keb(["Link To Dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["Real Dir", "link-to-dir"]);
}

/// A directory already in kebab case says nothing and fails nothing — the gate is tested
/// only once a rename is actually on the table.
#[test]
fn an_already_kebab_directory_is_silent_without_allow_dirs() {
    let s = Sandbox::new();
    s.mkdir("my-dir");

    let run = s.keb(["my-dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stderr, "");
    assert_eq!(run.stdout, "");
}

// ── §8 Output format ─────────────────────────────────────────────────────────────

/// The summary lines are gated on stderr being a terminal, so a pipeline sees exactly
/// what it saw before they existed. The harness captures output through pipes, which is
/// what makes this testable at all — and what every other test here relies on.
#[test]
fn summary_lines_are_suppressed_when_stderr_is_not_a_terminal() {
    let s = Sandbox::new();
    s.touch("My File.md");

    let dry = s.keb(["-n", "My File.md"]);
    assert_eq!(dry.stdout, "My File.md -> my-file.md\n");
    assert_eq!(dry.stderr, "", "no dry-run header");

    let run = s.keb(["My File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout, "My File.md -> my-file.md\n");
    assert_eq!(run.stderr, "", "no Renaming... header, no trailer");
}

#[test]
fn format_old_and_new_print_one_side() {
    let s = Sandbox::new();
    s.touch("My File.md");

    let old = s.keb(["-n", "--format=old", "My File.md"]);
    assert_eq!(old.stdout, "My File.md\n");

    let new = s.keb(["-n", "--format=new", "My File.md"]);
    assert_eq!(new.stdout, "my-file.md\n");
}

#[test]
fn format_null_separates_with_nul() {
    let s = Sandbox::new();
    s.touch("My File.md");

    let run = s.keb(["-n", "--format=null", "My File.md"]);
    assert_eq!(run.stdout, "My File.md\0my-file.md\0");
}

/// The arrow format cannot survive a newline in a filename — the record breaks in two.
/// `json` is the format that can, which is the reason it exists.
// Windows forbids control characters and `"` in filenames, so the next two cannot set up there.
#[cfg(unix)]
#[test]
fn format_json_survives_a_newline_in_a_filename() {
    let s = Sandbox::new();
    s.touch("Bad\nName.md");

    let arrow = s.keb(["-n", "Bad\nName.md"]);
    assert_eq!(arrow.stdout.lines().count(), 2, "arrow splits the record: {:?}", arrow.stdout);

    let json = s.keb(["-n", "--format=json", "Bad\nName.md"]);
    assert_eq!(json.stdout.lines().count(), 1, "json keeps it on one line");
    assert_eq!(json.stdout, "{\"from\":\"Bad\\nName.md\",\"to\":\"bad-name.md\"}\n");
}

#[cfg(unix)]
#[test]
fn format_json_escapes_quotes_and_backslashes() {
    let s = Sandbox::new();
    s.touch("A \"Quoted\" Name.md");

    let run = s.keb(["-n", "--format=json", "A \"Quoted\" Name.md"]);
    assert_eq!(run.stdout, "{\"from\":\"A \\\"Quoted\\\" Name.md\",\"to\":\"a-quoted-name.md\"}\n");
}

#[test]
fn absolute_prints_full_paths() {
    let s = Sandbox::new();
    s.touch("Sub Dir/My File.md");

    let run = s.keb(["-n", "--absolute", "Sub Dir/My File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let (from, to) = run.stdout.trim_end().split_once(" -> ").expect("arrow format");
    assert_eq!(from, s.absolute("Sub Dir/My File.md"));
    assert_eq!(to, s.absolute("Sub Dir/my-file.md"));
}

/// The property that matters: a symlink *named in the argument* is preserved, because keb
/// renames the link rather than its target and a canonical path could name something else.
#[cfg(unix)]
#[test]
fn absolute_does_not_resolve_a_symlink_in_the_path() {
    let s = Sandbox::new();
    s.touch("Real Dir/My File.md");
    std::os::unix::fs::symlink(s.dir.join("Real Dir"), s.dir.join("Link")).unwrap();

    let run = s.keb(["-n", "--absolute", "Link/My File.md"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let (from, _) = run.stdout.trim_end().split_once(" -> ").expect("arrow format");
    assert!(from.contains("/Link/"), "symlink was resolved away: {from}");
    assert!(!from.contains("/Real Dir/"), "symlink was resolved away: {from}");
}

#[test]
fn absolute_composes_with_format() {
    let s = Sandbox::new();
    s.touch("My File.md");

    let run = s.keb(["-n", "--absolute", "--format=new", "My File.md"]);
    assert_eq!(run.stdout.trim_end(), s.absolute("my-file.md"));
}
