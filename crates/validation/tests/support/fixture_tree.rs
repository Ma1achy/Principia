//! Fixture workspaces written only when their content changes (REQ-VAL-165; R-231). A file already holding its bytes
//! is left alone, so its modification time holds and a warm build of the fixture, in its own target directory kept
//! across runs, rebuilds nothing; a file whose bytes differ is rewritten, and one no longer in the fixture is removed.
//! Included with `#[path]` by `own_target.rs`, `qa_TASK-M0-24.rs`, `qa_TASK-M0-26.rs`, and xtask's `tests/deps.rs` and
//! `tests/support/qa_m0_01_r19{1,3,4}.rs`.

use std::path::Path;

/// Makes the tree at `root` hold `files` (paths relative to `root`, with their bytes; of two entries for one path, the
/// later), writing only the files whose bytes differ, and removing every other file but the fixture's `target/` and
/// the `Cargo.lock` cargo writes.
pub fn write_tree<P: AsRef<Path>, B: AsRef<[u8]>>(root: &Path, files: &[(P, B)]) {
    for (i, (rel, bytes)) in files.iter().enumerate() {
        if files[i + 1..]
            .iter()
            .any(|(later, _)| later.as_ref() == rel.as_ref())
        {
            continue;
        }
        let path = root.join(rel);
        if std::fs::read(&path).ok().as_deref() != Some(bytes.as_ref()) {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, bytes).unwrap();
        }
    }
    remove_others(root, root, files);
}

/// Removes each file under `dir` that is not one of `files`, relative to `root`, but for `target/` and `Cargo.lock`.
fn remove_others<P: AsRef<Path>, B>(root: &Path, dir: &Path, files: &[(P, B)]) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let rel = path.strip_prefix(root).unwrap();
        if rel == Path::new("target") || rel == Path::new("Cargo.lock") {
            continue;
        }
        if path.is_dir() {
            remove_others(root, &path, files);
        } else if !files.iter().any(|(p, _)| p.as_ref() == rel) {
            std::fs::remove_file(&path).unwrap();
        }
    }
}
