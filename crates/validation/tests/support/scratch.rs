//! A test's scratch folder or file (REQ-VAL-178, R-342): a fresh, uniquely named path per call, deleted when the test
//! passes, and kept, with its path printed, when the test panics; a control's panic is its passing (R-176). It lies
//! under `CARGO_TARGET_TMPDIR`, or the system temp folder where cargo does not set that (a crate's unit tests).
//! xtask's, prin's and validation's tests include this file with `#[path]`; every item here is used by each.

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A fresh scratch path, settled when the guard drops; see the [module docs](self).
pub struct Scratch {
    path: PathBuf,
    /// What is done with the path when the guard drops, given whether the test is panicking: [`settle`].
    pub settle: fn(&Path, bool),
}

impl Scratch {
    /// A fresh path named for `tag`, the process id and a per-process count, which no later run reuses. Nothing is
    /// made there; a folder or file left there by an earlier process of the same id is removed.
    pub fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root =
            option_env!("CARGO_TARGET_TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = root.join(format!("{tag}-{}-{n}", std::process::id()));
        remove(&path);
        Scratch { path, settle }
    }
}

/// Deletes `path` when the test passed; keeps it, printing its path, when the test failed (R-342).
pub fn settle(path: &Path, failed: bool) {
    if failed {
        eprintln!("scratch kept: {}", path.display());
    } else {
        remove(path);
    }
}

/// Removes the folder or file at `path`, if there is one.
fn remove(path: &Path) {
    let removed = match path.symlink_metadata() {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(_) => Ok(()),
    };
    if let Err(e) = removed {
        panic!("scratch {} is not removed: {e}", path.display());
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // A control passes by panicking (`#[should_panic]`, R-176): its panic is not a failure.
        let control = std::thread::current()
            .name()
            .is_some_and(|name| name.ends_with("::negative_control"));
        (self.settle)(&self.path, std::thread::panicking() && !control);
    }
}

impl Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
