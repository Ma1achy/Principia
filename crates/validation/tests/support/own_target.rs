//! A target directory of its own for a test's nested cargo run (REQ-VAL-164; R-227). Parallel copies of one fixture
//! are one package to cargo, so in a shared target directory each rebuilds the artifacts the others are running; and
//! every cargo run replaces each binary it uplifts, even when nothing is rebuilt, under any test spawning it.
//!
//! A [`Lease`] holds one directory of a pool under `$CARGO_TARGET_TMPDIR`, by a file lock, so no other test, in this
//! process or another, uses that directory until the lease is dropped. The directories are kept across runs, so each
//! builds cold once, not on every run: a cold build per call pushes a nested build past the 300 s timeout under load
//! (TASK-M0-31). Each lease keeps its copies of the fixtures at paths of its own, written only when they change
//! (R-231). Included with `#[path]` by `expected_message.rs`, `support/qa_m0_21_fixture.rs` and xtask's
//! `tests/controls.rs` and `tests/qa_TASK-M0-34.rs`, which share its pool of fixture target directories.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

#[path = "fixture_tree.rs"]
mod fixture_tree;

/// The pool the fixture copies build in.
pub const FIXTURES: &str = "fixture-targets";

/// One directory of a pool, leased until dropped.
pub struct Lease {
    dir: PathBuf,
    // Held open for the lease: closing it releases the lock.
    _lock: File,
}

impl Lease {
    /// The first directory of `pool` that no one holds, a new one when all are held: the pool grows only to the
    /// most leases held at once. With `copy`, a free directory already holding the copy of that name (see
    /// [`Lease::copy`]) comes first, so a warm run finds the copy's build where it left it (R-231); failing that, a
    /// directory holding no copy yet, or a new one. A directory holding only other fixtures' copies never takes it,
    /// so each fixture keeps target directories of its own across runs, as many as its tests ever held at once, and
    /// a warm run's lease for it never lands where it was not built (REQ-VAL-165).
    pub fn take(pool: &str, copy: Option<&str>) -> Lease {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(pool);
        std::fs::create_dir_all(&root).unwrap();
        let built = (0..)
            .take_while(|n: &u32| root.join(format!("{n}.lock")).exists())
            .filter(|n| copy.is_some_and(|c| root.join(format!("{n}.src")).join(c).exists()))
            .collect::<Vec<_>>();
        // A directory with no copies: `<n>.src` absent or empty, as for every directory past the pool's end.
        let unused = |n: &u32| {
            std::fs::read_dir(root.join(format!("{n}.src")))
                .map_or(true, |mut entries| entries.next().is_none())
        };
        built
            .into_iter()
            .chain((0..).filter(|n| copy.is_none() || unused(n)))
            .find_map(|n: u32| {
                let path = root.join(format!("{n}.lock"));
                let lock = File::options()
                    .create(true)
                    .truncate(false)
                    .write(true)
                    .open(&path)
                    .unwrap();
                match lock.try_lock() {
                    Ok(()) => Some(Lease {
                        dir: root.join(n.to_string()),
                        _lock: lock,
                    }),
                    Err(TryLockError::WouldBlock) => None,
                    Err(TryLockError::Error(e)) => panic!("lock {}: {e}", path.display()),
                }
            })
            .unwrap()
    }

    /// The leased directory, for `CARGO_TARGET_DIR`.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// A copy named `name` holding `files`, at a path of this lease's own that is the same on every run
    /// (`<pool>/<n>.src/<name>`), written only where its content changed (R-231): a fixture's path is part of its
    /// package id, so a copy kept at one path, building in the one target directory of the lease, rebuilds nothing.
    pub fn copy<B: AsRef<[u8]>>(&self, name: &str, files: &[(PathBuf, B)]) -> PathBuf {
        let copy = self.dir.with_extension("src").join(name);
        fixture_tree::write_tree(&copy, files);
        copy
    }
}

/// Every file under the fixture directory `from`, by its path relative to `from`, with its bytes.
pub fn fixture_files(from: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut dirs = vec![from.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                files.push((path.strip_prefix(from).unwrap().to_path_buf(), bytes));
            }
        }
    }
    files
}
