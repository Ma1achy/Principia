//! A target directory of its own for a test's nested cargo run (REQ-VAL-164; R-227). Parallel copies of one fixture
//! are one package to cargo, so in a shared target directory each rebuilds the artifacts the others are running; and
//! every cargo run replaces each binary it uplifts, even when nothing is rebuilt, under any test spawning it.
//!
//! A [`Lease`] holds one directory of a pool under `$CARGO_TARGET_TMPDIR`, by a file lock, so no other test, in this
//! process or another, uses that directory until the lease is dropped. The directories are kept across runs, so each
//! builds cold once, not on every run: a cold build per call pushes a nested build past the 300 s timeout under load
//! (TASK-M0-31). Included with `#[path]` by `expected_message.rs`, `support/qa_m0_21_fixture.rs` and xtask's
//! `tests/controls.rs`, which share its pool of fixture target directories, and by `qa_TASK-M0-24.rs`, whose runs on
//! the workspace lease from a pool of their own.

use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

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
    /// most leases held at once.
    pub fn take(pool: &str) -> Lease {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(pool);
        std::fs::create_dir_all(&root).unwrap();
        (0..)
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
}
