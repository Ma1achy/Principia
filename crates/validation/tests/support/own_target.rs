//! Target directories for tests' nested cargo runs, kept across runs under `$CARGO_TARGET_TMPDIR` (REQ-VAL-164,
//! REQ-VAL-165; R-227, R-231, R-270).
//!
//! Each fixture type has one build directory, `<pool>/<type>`, shared by all of that type's copies (R-270): its
//! dependencies build there once, and each copy's own crates once, so a warm run of the type rebuilds nothing. A
//! [`Lease`] holds the directory by a file lock, `<pool>/<type>.lock`, so only one test at a time, in this process or
//! another, builds or runs anything in it. A test and its control never use it at once (R-224). Nor do two copies of
//! one type, so no cargo run in it replaces a binary that another test's run is spawning, and no copy is rewritten
//! while another test builds it (REQ-VAL-164). A lease for a type waits while another thread or process holds it; a
//! thread that already holds it shares its lease, since one thread's runs cannot overlap.
//!
//! Included with `#[path]` by `expected_message.rs`, `support/qa_m0_21_fixture.rs` and xtask's `tests/controls.rs`,
//! `tests/qa_TASK-M0-34.rs` and `tests/qa_TASK-M0-38.rs`, which share its pool, [`FIXTURES`].

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::thread::ThreadId;

#[path = "fixture_tree.rs"]
mod fixture_tree;

/// The pool the fixture types build in: one directory per type (R-270). CI caches it between runs.
pub const FIXTURES: &str = "fixture-targets";

/// The type directories leased in this process, each with the thread holding it and its lock.
static HELD: Mutex<Vec<(PathBuf, ThreadId, Weak<File>)>> = Mutex::new(Vec::new());

/// A directory of a pool, leased until dropped.
pub struct Lease {
    dir: PathBuf,
    // Held open for the lease, shared by one thread's leases of a type: closing it releases the lock.
    _lock: Arc<File>,
}

impl Lease {
    /// With `Some(kind)`, fixture type `kind`'s one build directory in `pool`, once no other thread or process holds
    /// it; a thread already holding it gets it at once. With `None`, the first numbered directory of `pool` that no
    /// one holds, or a new one when all are held, for a build of no fixture type.
    pub fn take(pool: &str, kind: Option<&str>) -> Lease {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(pool);
        std::fs::create_dir_all(&root).unwrap();
        match kind {
            Some(kind) => Lease::take_type(&root, kind),
            None => (0..)
                .find_map(|n: u32| Lease::try_take(&root, &n.to_string()))
                .unwrap(),
        }
    }

    /// Type `kind`'s directory in the pool at `root`, waiting for any other thread or process holding it.
    fn take_type(root: &Path, kind: &str) -> Lease {
        assert!(
            !kind.is_empty() && !kind.contains(['/', '\\', '.']),
            "a fixture type names one directory: {kind:?}"
        );
        let dir = root.join(kind);
        let me = std::thread::current().id();
        {
            let mut held = HELD.lock().unwrap_or_else(PoisonError::into_inner);
            held.retain(|(_, _, lock)| lock.strong_count() > 0);
            let mine = held
                .iter()
                .find(|(d, by, _)| *d == dir && *by == me)
                .and_then(|(_, _, lock)| lock.upgrade());
            if let Some(lock) = mine {
                return Lease { dir, _lock: lock };
            }
        }
        let lock = Lease::open(root, kind);
        lock.lock()
            .unwrap_or_else(|e| panic!("lock {}: {e}", dir.display()));
        let lock = Arc::new(lock);
        HELD.lock().unwrap_or_else(PoisonError::into_inner).push((
            dir.clone(),
            me,
            Arc::downgrade(&lock),
        ));
        Lease { dir, _lock: lock }
    }

    /// Directory `name` of the pool at `root`, if no one holds it.
    fn try_take(root: &Path, name: &str) -> Option<Lease> {
        let lock = Lease::open(root, name);
        match lock.try_lock() {
            Ok(()) => Some(Lease {
                dir: root.join(name),
                _lock: Arc::new(lock),
            }),
            Err(TryLockError::WouldBlock) => None,
            Err(TryLockError::Error(e)) => panic!("lock {}: {e}", root.join(name).display()),
        }
    }

    /// The lock file of directory `name` of the pool at `root`.
    fn open(root: &Path, name: &str) -> File {
        let path = root.join(format!("{name}.lock"));
        File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .unwrap_or_else(|e| panic!("open {}: {e}", path.display()))
    }

    /// The leased directory, for `CARGO_TARGET_DIR`.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// A copy named `name` holding `files`, at a path that is the same on every run (`<pool>/<dir>.src/<name>`),
    /// written only where its content changed (R-231): a fixture's path is part of its package id, so a copy kept at
    /// one path, building in its type's one directory, rebuilds nothing.
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
