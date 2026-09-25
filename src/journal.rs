//! The undo journal — `cases.md` §12, "Atomicity".
//!
//! `rename()` is atomic, but the two-step rename a case-only change needs on a
//! case-insensitive filesystem is not. A crash between the two steps leaves the file
//! under a temporary name, reachable from nothing. So the journal is write-**ahead**:
//! the intent, including the temporary name, is on disk and flushed *before* the
//! syscall that could be interrupted.
//!
//! The format is one record per line, tab-separated, with paths percent-encoded so
//! that a filename containing a tab or a newline cannot forge a record. On Unix a
//! filename is an arbitrary byte string, so the encoding is over bytes, not text —
//! a name that is not valid UTF-8 still round-trips exactly (decision 3).

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Runs kept when the journal is trimmed.
const KEEP_RUNS: usize = 20;

/// Trim once the file passes this size, so a long-lived journal cannot grow unbounded.
const TRIM_BYTES: u64 = 1 << 20;

/// One recorded rename, as read back for `--undo`.
pub struct Entry {
    pub from: PathBuf,
    pub to: PathBuf,
    /// The temporary name a two-step rename passes through. Present on a `PLAN` record
    /// that never reached `DONE`, which is how an interrupted run is recovered.
    pub via: Option<PathBuf>,
    pub committed: bool,
}

pub struct Journal {
    path: PathBuf,
    file: Option<File>,
    /// Whether this run's `RUN` marker has been written.
    open_run: bool,
}

impl Journal {
    /// Open the journal for appending, creating its directory if needed.
    pub fn open() -> io::Result<Journal> {
        let path = default_path()?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        Ok(Journal { path, file: None, open_run: false })
    }

    /// A journal that records nothing — for `--dry-run`, which must not leave a trace.
    pub fn disabled() -> Journal {
        Journal { path: PathBuf::new(), file: None, open_run: false }
    }

    pub fn location(&self) -> &Path {
        &self.path
    }

    /// Open the file for appending. The run marker itself is not written yet — see
    /// [`Journal::plan`].
    pub fn begin(&mut self) -> io::Result<()> {
        if self.path.as_os_str().is_empty() {
            return Ok(());
        }
        if fs::metadata(&self.path).is_ok_and(|m| m.len() > TRIM_BYTES) {
            self.trim()?;
        }
        self.file = Some(OpenOptions::new().create(true).append(true).open(&self.path)?);
        Ok(())
    }

    /// Record the intent to rename, and flush it. Must be called *before* the syscall.
    ///
    /// The run marker is written here, on the first rename, rather than when the run
    /// starts. A run that renames nothing must leave no trace at all: an empty `RUN`
    /// would become the most recent run and hide the real one from `--undo`.
    pub fn plan(&mut self, from: &Path, to: &Path, via: Option<&Path>) -> io::Result<()> {
        if self.file.is_some() && !self.open_run {
            self.open_run = true;
            self.write(&["RUN"])?;
        }
        let via = via.map(encode).unwrap_or_default();
        self.write(&["PLAN", &encode(from), &encode(to), &via])
    }

    /// Record that the rename completed.
    pub fn commit(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.write(&["DONE", &encode(from), &encode(to)])
    }

    fn write(&mut self, fields: &[&str]) -> io::Result<()> {
        let Some(file) = self.file.as_mut() else {
            return Ok(());
        };
        writeln!(file, "{}", fields.join("\t"))?;
        // Write-ahead is only write-ahead if it has actually reached the file.
        file.flush()?;
        file.sync_data()
    }

    /// The most recent run, oldest record first. Empty if there is nothing to undo.
    pub fn last_run(&self) -> io::Result<Vec<Entry>> {
        Ok(parse(&self.read_lines()?).pop().unwrap_or_default())
    }

    /// Forget the most recent run, after it has been undone.
    pub fn drop_last_run(&self) -> io::Result<()> {
        let lines = self.read_lines()?;
        let cut = lines.iter().rposition(|l| l.starts_with("RUN")).unwrap_or(lines.len());
        fs::write(&self.path, lines[..cut].join("\n") + if cut > 0 { "\n" } else { "" })
    }

    fn trim(&self) -> io::Result<()> {
        let lines = self.read_lines()?;
        let starts: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.starts_with("RUN"))
            .map(|(i, _)| i)
            .collect();
        let cut = starts.len().saturating_sub(KEEP_RUNS);
        let keep = starts.get(cut).copied().unwrap_or(0);
        fs::write(&self.path, lines[keep..].join("\n") + "\n")
    }

    fn read_lines(&self) -> io::Result<Vec<String>> {
        match File::open(&self.path) {
            Ok(f) => BufReader::new(f).lines().collect(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }
}

/// Group records into runs. A record before the first `RUN` marker is ignored, as is
/// anything that does not parse — a corrupt line must not take the rest with it.
fn parse(lines: &[String]) -> Vec<Vec<Entry>> {
    let mut runs: Vec<Vec<Entry>> = Vec::new();

    for line in lines {
        let mut fields = line.split('\t');
        match (fields.next(), runs.last_mut()) {
            (Some("RUN"), _) => runs.push(Vec::new()),
            (Some("PLAN"), Some(run)) => {
                if let (Some(from), Some(to)) = (fields.next(), fields.next()) {
                    let via = fields.next().filter(|v| !v.is_empty()).map(decode);
                    run.push(Entry { from: decode(from), to: decode(to), via, committed: false });
                }
            }
            (Some("DONE"), Some(run)) => {
                if let (Some(from), Some(to)) = (fields.next(), fields.next()) {
                    let (from, to) = (decode(from), decode(to));
                    if let Some(e) = run.iter_mut().rfind(|e| e.from == from && e.to == to) {
                        e.committed = true;
                    }
                }
            }
            _ => {}
        }
    }

    runs
}

/// Percent-encode a path's bytes, keeping only characters that cannot be confused with
/// the record structure.
fn encode(path: &Path) -> String {
    let mut out = String::new();
    for &b in bytes(path).iter() {
        match b {
            b'%' => out.push_str("%25"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn decode(s: &str) -> PathBuf {
    let raw = s.as_bytes();
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let hex = (i + 2 < raw.len())
            .then(|| std::str::from_utf8(&raw[i + 1..i + 3]).ok())
            .flatten()
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (raw[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    PathBuf::from(from_bytes(out))
}

#[cfg(unix)]
fn bytes(path: &Path) -> &[u8] {
    std::os::unix::ffi::OsStrExt::as_bytes(path.as_os_str())
}

#[cfg(unix)]
fn from_bytes(v: Vec<u8>) -> OsString {
    <OsString as std::os::unix::ffi::OsStringExt>::from_vec(v)
}

#[cfg(not(unix))]
fn bytes(path: &Path) -> std::borrow::Cow<'_, [u8]> {
    match path.to_str() {
        Some(s) => std::borrow::Cow::Borrowed(s.as_bytes()),
        None => std::borrow::Cow::Owned(path.to_string_lossy().into_owned().into_bytes()),
    }
}

#[cfg(not(unix))]
fn from_bytes(v: Vec<u8>) -> OsString {
    OsString::from(String::from_utf8_lossy(&v).into_owned())
}

/// Where the journal lives.
///
/// This reads the platform's state-directory variables, which is not the configuration
/// that `cases.md` rules out: they say *where* to put a file, never what the tool does
/// to a name. There is no setting here that could make the same command behave
/// differently in two directories.
fn default_path() -> io::Result<PathBuf> {
    let missing = || io::Error::new(io::ErrorKind::NotFound, "cannot locate a state directory");

    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);

    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")));

    Ok(base.ok_or_else(missing)?.join("keb").join("journal.tsv"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_round_trip() {
        for p in ["a b.md", "wei\trd\nname", "100%.md", "./A Dir/My File.md", "ünïcødé.md"] {
            assert_eq!(decode(&encode(Path::new(p))), PathBuf::from(p));
        }
    }

    #[cfg(unix)]
    #[test]
    fn invalid_utf8_round_trips() {
        // cases.md decision 3 — on Linux a filename is bytes, not text
        let raw = <OsString as std::os::unix::ffi::OsStringExt>::from_vec(vec![b'c', 0xff, 0xfe]);
        let path = PathBuf::from(raw);
        assert_eq!(decode(&encode(&path)), path);
    }

    #[test]
    fn encoded_paths_cannot_forge_a_record() {
        let sneaky = Path::new("x\tDONE\ta\tb");
        assert!(!encode(sneaky).contains('\t'));
    }

    #[test]
    fn groups_runs_and_marks_commits() {
        let lines: Vec<String> = ["RUN", "PLAN\ta\tb\t", "DONE\ta\tb", "RUN", "PLAN\tc\td\tc.tmp"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let runs = parse(&lines);
        assert_eq!(runs.len(), 2);
        assert!(runs[0][0].committed);
        assert!(!runs[1][0].committed);
        assert_eq!(runs[1][0].via, Some(PathBuf::from("c.tmp")));
    }
}
