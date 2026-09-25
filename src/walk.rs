//! Recursive traversal for `-r` — `cases.md` §7, §12 "Recursion hazards".
//!
//! Two rules make recursion safe, and both are structural rather than defensive:
//!
//! - **Collect the whole list first.** Renaming while iterating a directory is
//!   undefined across filesystems; the entry you are standing on may be handed to you
//!   twice or not at all.
//! - **Never follow a symlink.** `keb` renames the link, not its target, so there is
//!   nothing to descend into — which also means a symlink loop cannot exist.

use std::io;
use std::path::{Path, PathBuf};

/// What `-r` should sweep up, from `--files-only` / `--dirs-only`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    FilesOnly,
    DirsOnly,
}

pub struct Walk {
    /// Every entry beneath the root, **deepest first**, so that a rename never
    /// invalidates a path still waiting in the queue.
    pub paths: Vec<PathBuf>,
    pub errors: Vec<(PathBuf, io::Error)>,
}

pub fn collect(root: &Path, filter: Filter) -> Walk {
    let mut found: Vec<(usize, PathBuf)> = Vec::new();
    let mut errors = Vec::new();
    let mut queue = vec![(0usize, root.to_path_buf())];

    while let Some((depth, dir)) = queue.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                errors.push((dir, e));
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    errors.push((dir.clone(), e));
                    continue;
                }
            };
            // `DirEntry::file_type` does not follow symlinks, so a link to a directory
            // is a leaf here — exactly what "rename the link, not the target" wants.
            let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
            let path = entry.path();

            let wanted = match filter {
                Filter::All => true,
                Filter::FilesOnly => !is_dir,
                Filter::DirsOnly => is_dir,
            };
            if wanted {
                found.push((depth + 1, path.clone()));
            }
            if is_dir {
                queue.push((depth + 1, path));
            }
        }
    }

    found.sort_by(|(a, pa), (b, pb)| b.cmp(a).then_with(|| pa.cmp(pb)));
    Walk { paths: found.into_iter().map(|(_, p)| p).collect(), errors }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch directory that cleans itself up.
    struct Temp(PathBuf);

    impl Temp {
        fn new(tag: &str) -> Temp {
            let path = std::env::temp_dir().join(format!("keb-walk-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Temp(path)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn deepest_first_and_filtered() {
        let t = Temp::new("order");
        std::fs::create_dir_all(t.0.join("a/b")).unwrap();
        std::fs::write(t.0.join("a/b/deep.txt"), "").unwrap();
        std::fs::write(t.0.join("top.txt"), "").unwrap();

        let all = collect(&t.0, Filter::All);
        assert!(all.errors.is_empty());
        let depths: Vec<usize> = all.paths.iter().map(|p| p.components().count()).collect();
        assert!(depths.windows(2).all(|w| w[0] >= w[1]), "not deepest-first: {depths:?}");
        assert_eq!(all.paths.len(), 4);

        let files = collect(&t.0, Filter::FilesOnly);
        assert_eq!(files.paths.len(), 2);
        assert!(files.paths.iter().all(|p| p.is_file()));

        let dirs = collect(&t.0, Filter::DirsOnly);
        assert_eq!(dirs.paths.len(), 2);
        assert!(dirs.paths.iter().all(|p| p.is_dir()));
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_symlinks() {
        let t = Temp::new("links");
        std::fs::create_dir_all(t.0.join("real")).unwrap();
        std::fs::write(t.0.join("real/inner.txt"), "").unwrap();
        // A loop: the link points at the directory that contains it.
        std::os::unix::fs::symlink(&t.0, t.0.join("loop")).unwrap();

        let walk = collect(&t.0, Filter::All);
        assert!(walk.paths.contains(&t.0.join("loop")));
        assert!(!walk.paths.contains(&t.0.join("loop/real")));
        assert_eq!(walk.paths.len(), 3);
    }
}
