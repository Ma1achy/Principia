//! `cargo xtask lint wgsl` over the fragment stage's shared library, `crates/render/shaders/` (render_gui_spec §10.1;
//! TASK-M1-03): the float rules (R-351, R-352) hold there as under `crates/render/frag/`. The generated prelude is
//! linted alone, and every other library file, which follows it at assembly and calls it, as its continuation, its
//! findings at its own lines. Each scratch workspace holds a clean generated layer, as `lint` requires. Each test has a
//! registered negative control (R-176).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;
use xtask::lint_wgsl::{lint, Rule, GENERATED, LIB_DIR, PRELUDE};

/// A prelude: one helper, and nothing a float rule fires on.
const PRELUDE_CLEAN: &str = "// prelude\nfn helper(x: f32) -> f32 { return x * 2.0; }\n";

/// A prelude whose second line tests NaN with `isnan`'s stand-in, a self-comparison.
const PRELUDE_FIRES: &str =
    "// prelude\nfn helper(x: f32) -> f32 { return select(x, 0.0, x != x); }\n";

/// A library file calling the prelude's helper, clean.
const PRESENT_CLEAN: &str =
    "// present\nfn shown(x: f32) -> f32 {\n    return helper(x) + 1.0;\n}\n";

/// The same file, its third line comparing a float with itself.
const PRESENT_FIRES: &str =
    "// present\nfn shown(x: f32) -> f32 {\n    return select(helper(x), 0.0, x != x);\n}\n";

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A scratch workspace: a `Cargo.toml`, the clean fixture as the generated layer, and `files` (each a path relative to
/// the root and its source).
fn workspace(files: &[(&str, &str)]) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "lint_wgsl_lib_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let write = |rel: &str, text: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        std::fs::write(&path, text).expect("the file");
    };
    write("Cargo.toml", "");
    let clean = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lint_wgsl/clean.wgsl");
    write(GENERATED, &read(&clean));
    for (rel, text) in files {
        write(rel, text);
    }
    root
}

fn present_path() -> String {
    format!("{LIB_DIR}/wgsl/lib/present.wgsl")
}

/// Each finding of `lint` over a workspace holding `prelude` and `present`, as `file:line: [rule] …`.
fn lib_findings(prelude: &str, present: &str) -> Vec<String> {
    let root = workspace(&[(PRELUDE, prelude), (&present_path(), present)]);
    let reports = lint(&root).expect("the files parse and validate");
    let _ = std::fs::remove_dir_all(&root);
    for file in [PRELUDE.to_owned(), present_path()] {
        assert!(
            reports.iter().any(|r| r.file == file),
            "{file} was not linted"
        );
    }
    reports
        .iter()
        .flat_map(|r| r.findings.iter().map(|f| f.at(&r.file)))
        .collect()
}

/// `present`, after a clean prelude, fails at its own third line with the self-comparison rule.
fn check_fires_at_its_line(present: &str) {
    let found = lib_findings(PRELUDE_CLEAN, present);
    let want = format!("{}:3: [{}] ", present_path(), Rule::SelfCompare);
    assert!(
        found.iter().any(|l| l.starts_with(&want)),
        "the library file's self-comparison did not fire at its line 3: {found:?}"
    );
}

#[test]
fn lint_wgsl_lib_file_fires_at_its_line_after_the_prelude() {
    check_fires_at_its_line(PRESENT_FIRES);
}

negative_control!(
    lint_wgsl_lib_file_fires_at_its_line_after_the_prelude,
    "a clean library file must not fire",
    expected = "did not fire at its line 3",
    check_fires_at_its_line(PRESENT_CLEAN)
);

/// A finding in the prelude is the prelude's alone: reported on it, at its line, never again on the file after it.
fn check_prelude_finding_stays(prelude: &str) {
    let found = lib_findings(prelude, PRESENT_CLEAN);
    let on_prelude = format!("{PRELUDE}:2: [{}] ", Rule::SelfCompare);
    assert!(
        found.iter().any(|l| l.starts_with(&on_prelude)),
        "the prelude's self-comparison was not reported on it: {found:?}"
    );
    assert!(
        !found.iter().any(|l| l.starts_with(&present_path())),
        "the prelude's finding was reported on the file after it: {found:?}"
    );
}

#[test]
fn lint_wgsl_lib_prelude_findings_stay_the_preludes() {
    check_prelude_finding_stays(PRELUDE_FIRES);
}

negative_control!(
    lint_wgsl_lib_prelude_findings_stay_the_preludes,
    "a clean prelude reports nothing on it",
    expected = "was not reported on it",
    check_prelude_finding_stays(PRELUDE_CLEAN)
);

/// A workspace with no library directory lints only `crates/render/frag/`; one with the directory but no prelude is
/// an error naming the prelude. `files` are the second workspace's files.
fn check_lib_dir_rules(files: &[(&str, &str)]) {
    let root = workspace(&[]);
    let reports = lint(&root).expect("a workspace without the library lints");
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        reports.iter().all(|r| !r.file.starts_with(LIB_DIR)),
        "a library file was linted where there is none: {reports:?}"
    );
    let root = workspace(files);
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let error = result.err().unwrap_or_default();
    assert!(
        error.contains("prelude.wgsl"),
        "a library without its prelude was not refused naming it: {error:?}"
    );
}

#[test]
fn lint_wgsl_lib_needs_its_prelude() {
    check_lib_dir_rules(&[(&present_path(), PRESENT_CLEAN)]);
}

negative_control!(
    lint_wgsl_lib_needs_its_prelude,
    "a library with its prelude lints",
    expected = "was not refused naming it",
    check_lib_dir_rules(&[(PRELUDE, PRELUDE_CLEAN), (&present_path(), PRESENT_CLEAN)])
);
