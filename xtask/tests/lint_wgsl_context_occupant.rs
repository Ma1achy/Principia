//! `cargo xtask lint wgsl` over a built-in occupant of a slot that reads the stain's context (`xtask::lint_wgsl::
//! CONTEXT_SLOTS`; TASK-M1-10): the outcome palette, `colour/outcome_state.wgsl`, reads `ctx.sample`, so it is linted
//! as the assembler presents it, after the prelude, the library, the unpack layer, the read side and the stain's
//! context, then its `// @uniform` block, as a generated debug view is. A combiner reads no context, so the same file
//! under `combiner/` does not lint. Each test has a registered negative control (R-176).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;
use xtask::lint_wgsl::{
    lint, CONTEXT_SLOTS, GENERATED, LIB_FILES, OCCUPANT_DIR, READ_SIDE_FILE, STAIN_DIR,
};

/// The workspace this crate is in.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Each `.wgsl` file directly in the repo's `dir`, by its path relative to the root, with its text.
fn files_in(dir: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(repo().join(dir)).expect("the directory") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        if name.ends_with(".wgsl") {
            out.push((format!("{dir}/{name}"), read(&path)));
        }
    }
    out
}

/// A scratch workspace holding the repo's library, unpack layer, read side and stain context, and the outcome
/// occupant under `slot/` of the occupant directory; and that occupant's path.
fn workspace(slot: &str) -> (PathBuf, String) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "lint_wgsl_context_occupant_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let occupant = format!("{OCCUPANT_DIR}/{slot}/outcome_state.wgsl");
    let mut files = files_in(LIB_FILES);
    files.extend(files_in(STAIN_DIR));
    for rel in [GENERATED, READ_SIDE_FILE] {
        files.push((rel.to_owned(), read(&repo().join(rel))));
    }
    files.push((
        occupant.clone(),
        read(&repo().join(format!("{OCCUPANT_DIR}/colour/outcome_state.wgsl"))),
    ));
    files.push(("Cargo.toml".to_owned(), String::new()));
    for (rel, text) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        std::fs::write(&path, text).expect("the file");
    }
    (root, occupant)
}

/// The outcome occupant, placed under `slot/`, lints with every rule holding.
fn check_lints_clean(slot: &str) {
    let (root, occupant) = workspace(slot);
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let reports = match result {
        Ok(r) => r,
        Err(e) => panic!("the occupant under `{slot}/` did not lint: {e}"),
    };
    let report = reports.iter().find(|r| r.file == occupant);
    assert!(
        report.is_some_and(|r| r.findings.is_empty()),
        "{occupant} was not linted clean: {report:?}"
    );
}

#[test]
fn lint_wgsl_context_occupant_lints_after_the_stain_context() {
    assert_eq!(CONTEXT_SLOTS, ["colour", "brightness", "post"]);
    for slot in CONTEXT_SLOTS {
        check_lints_clean(slot);
    }
}

negative_control!(
    lint_wgsl_context_occupant_lints_after_the_stain_context,
    "under `combiner/` the occupant is linted without the stain's context, where `Ctx` is not in scope",
    expected = "the occupant under `combiner/` did not lint",
    check_lints_clean("combiner")
);
