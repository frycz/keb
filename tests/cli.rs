//! End-to-end tests for the filesystem layer — `cases.md` §6, §7 and §12.
//!
//! These drive the real binary against a real directory, because everything being
//! tested here is a property of the filesystem rather than of the transform: inode
//! identity on a case-insensitive volume, the ordering of a recursive sweep, what the
//! journal holds after a crash. None of it is observable from a pure function.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// A scratch directory with its own journal, removed when the test ends.
struct Sandbox {
    dir: PathBuf,
    state: PathBuf,
}

static SEQ: AtomicU32 = AtomicU32::new(0);

impl Sandbox {
    fn new() -> Sandbox {
        let id = SEQ.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("keb-cli-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = root.join("work");
        let state = root.join("state");
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(&state).unwrap();
        Sandbox { dir, state }
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
            // The journal must not be the developer's own while tests are running.
            .env("XDG_STATE_HOME", &self.state)
            .env("LOCALAPPDATA", &self.state)
            .args(args)
            .output()
            .expect("keb should run");
        Run::from(out)
    }

    /// Every entry under the sandbox, relative and sorted — the whole observable state.
    fn tree(&self) -> Vec<String> {
        fn visit(dir: &Path, base: &Path, out: &mut Vec<String>) {
            let mut entries: Vec<_> =
                fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
            entries.sort();
            for path in entries {
                if path.file_name() == Some(OsStr::new(".git")) {
                    continue;
                }
                out.push(path.strip_prefix(base).unwrap().display().to_string());
                if path.is_dir() {
                    visit(&path, base, out);
                }
            }
        }
        let mut out = Vec::new();
        visit(&self.dir, &self.dir, &mut out);
        out
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
    // ...and leaves nothing behind to undo
    assert_eq!(s.keb(["--undo"]).stdout, "");
}

#[test]
fn reads_a_list_from_stdin() {
    let s = Sandbox::new();
    s.touch("One File.md");
    s.touch("Two File.md");

    let out = Command::new(env!("CARGO_BIN_EXE_keb"))
        .current_dir(&s.dir)
        .env("XDG_STATE_HOME", &s.state)
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

    let run = s.keb(["My Dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["my-dir", "my-dir/My File.md"]);
}

#[test]
fn recursion_renames_deepest_first() {
    // cases.md §6 — otherwise the parent's rename invalidates every queued child path
    let s = Sandbox::new();
    s.touch("My Dir/Sub Dir/Deep File.md");
    s.touch("My Dir/Top File.md");

    let run = s.keb(["-r", "My Dir"]);
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

#[test]
fn recursion_filters() {
    let s = Sandbox::new();
    s.touch("My Dir/Sub Dir/Deep File.md");

    let run = s.keb(["-r", "--files-only", "My Dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    // the named directory is still renamed — it was chosen, not swept up
    assert_eq!(s.tree(), ["my-dir", "my-dir/Sub Dir", "my-dir/Sub Dir/deep-file.md"]);
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

    let run = s.keb(["-r", "My Dir"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stderr.contains("skipped"));
    assert_eq!(s.tree(), ["my-dir", "my-dir/Makefile", "my-dir/MyClass.java", "my-dir/my-file.md"]);

    // ...and -f renames them after all
    let forced = s.keb(["-r", "-f", "my-dir"]);
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

// ── §8 / §12 Undo ────────────────────────────────────────────────────────────────

#[test]
fn undo_replays_the_run_backwards() {
    let s = Sandbox::new();
    s.touch("My Dir/Sub Dir/Deep File.md");
    s.touch("My Dir/Top File.md");

    assert_eq!(s.keb(["-r", "My Dir"]).code, 0);
    assert_eq!(s.tree()[0], "my-dir");

    let undone = s.keb(["--undo"]);
    assert_eq!(undone.code, 0, "{}", undone.stderr);
    assert_eq!(
        s.tree(),
        ["My Dir", "My Dir/Sub Dir", "My Dir/Sub Dir/Deep File.md", "My Dir/Top File.md"]
    );

    // the run is spent; a second undo has nothing left
    let again = s.keb(["--undo"]);
    assert_eq!(again.stdout, "");
    assert!(again.stderr.contains("nothing to undo"));
}

#[test]
fn undo_restores_a_suffixed_collision() {
    let s = Sandbox::new();
    s.touch("My File.md");
    s.touch("my_file.md");

    assert_eq!(s.keb(["My File.md", "my_file.md"]).code, 0);
    assert_eq!(s.keb(["--undo"]).code, 0);
    assert_eq!(s.tree(), ["My File.md", "my_file.md"]);
}

#[test]
fn a_run_that_renames_nothing_does_not_shadow_the_last_one() {
    // An empty run marker would become "the most recent run" and hide the real one
    let s = Sandbox::new();
    s.touch("My File.md");
    s.touch("already-kebab.md");

    assert_eq!(s.keb(["My File.md"]).code, 0);
    let noop = s.keb(["already-kebab.md"]);
    assert_eq!(noop.stdout, "");

    let undone = s.keb(["--undo"]);
    assert_eq!(undone.renames(), ["my-file.md -> My File.md"]);
    assert_eq!(s.tree(), ["My File.md", "already-kebab.md"]);
}

#[test]
fn undo_does_not_clobber_whatever_took_the_name_back() {
    let s = Sandbox::new();
    s.touch("My File.md");
    assert_eq!(s.keb(["My File.md"]).code, 0);

    // someone recreates the original name with different content
    s.touch("My File.md");

    let undone = s.keb(["--undo"]);
    assert_eq!(undone.code, 1);
    assert!(undone.stderr.contains("occupied"), "{}", undone.stderr);
    assert_eq!(fs::read_to_string(s.dir.join("My File.md")).unwrap(), "My File.md");
    assert!(s.exists("my-file.md"));
}

#[test]
fn undo_dry_run_reads_the_journal_without_spending_it() {
    let s = Sandbox::new();
    s.touch("My File.md");
    assert_eq!(s.keb(["My File.md"]).code, 0);

    let preview = s.keb(["--undo", "-n"]);
    assert_eq!(preview.renames(), ["my-file.md -> My File.md"]);
    assert_eq!(s.tree(), ["my-file.md"], "dry run must change nothing");

    // the run is still there to undo for real
    assert_eq!(s.keb(["--undo"]).code, 0);
    assert_eq!(s.tree(), ["My File.md"]);
}

#[test]
fn dry_run_predicts_a_multi_file_run() {
    // cases.md §12 warns that dry-run output can lie. It must at least not lie about
    // its own effects: the second file collides with the first one's *planned* name,
    // and the third takes a name the first one is vacating.
    let s = Sandbox::new();
    s.touch("My File.md");
    s.touch("my_file.md");
    s.touch("MY FILE.md.bak");

    let preview = s.keb(["-n", "My File.md", "my_file.md"]);
    let real = s.keb(["My File.md", "my_file.md"]);
    assert_eq!(preview.renames(), real.renames());
}

#[test]
fn an_interrupted_two_step_rename_is_recoverable() {
    // cases.md §12 — the journal is write-ahead precisely so that a crash between the
    // two halves of a case-only rename leaves a record pointing at the temporary name
    let s = Sandbox::new();
    let temp = ".keb-999-1";
    s.touch(temp);

    let record = |p: &Path| p.display().to_string();
    fs::create_dir_all(s.state.join("keb")).unwrap();
    fs::write(
        s.state.join("keb/journal.tsv"),
        format!(
            "RUN\nPLAN\t{}\t{}\t{}\n",
            record(&s.dir.join("My File.md")),
            record(&s.dir.join("my-file.md")),
            record(&s.dir.join(temp)),
        ),
    )
    .unwrap();

    let run = s.keb(["--undo"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(s.tree(), ["My File.md"], "the orphaned temp file was not recovered");
}

#[test]
fn undo_only_reaches_the_most_recent_run() {
    let s = Sandbox::new();
    s.touch("One File.md");
    s.touch("Two File.md");

    assert_eq!(s.keb(["One File.md"]).code, 0);
    assert_eq!(s.keb(["Two File.md"]).code, 0);
    assert_eq!(s.keb(["--undo"]).code, 0);
    assert_eq!(s.tree(), ["Two File.md", "one-file.md"]);
}
