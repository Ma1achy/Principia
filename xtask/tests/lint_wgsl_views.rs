//! `cargo xtask lint wgsl` over the debug catalogue's generated views, `crates/render/frag/debug/generated/` (RQ-219;
//! TASK-M1-08), and the stain's context, `crates/render/shaders/wgsl/stain/`: the context is linted after the prelude,
//! the library, the unpack layer and the read side, whose `SimState` it holds, and each view after all of those and the
//! context, whose `Ctx` it reads, as the assembler presents a node. So a view parses, validates and is held to the
//! float rules (R-351, R-352), its findings at its own lines. Each test has a registered negative control (R-176).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;
use xtask::lint_wgsl::{
    lint, Rule, DEBUG_VIEWS, GENERATED, LIB_FILES, PRELUDE, READ_SIDE_FILE, STAIN_DIR,
};

/// A view of `d_min` whose second line compares the field with itself.
const VIEW_FIRES: &str = "fn colour(ctx: Ctx) -> vec3<f32> {\n    \
let unset = ctx.sample.d_min != ctx.sample.d_min;\n    return dbg_flag(unset);\n}\n";

/// The same view, clean: the negative control's input.
#[cfg(feature = "controls")]
const VIEW_CLEAN: &str = "fn colour(ctx: Ctx) -> vec3<f32> {\n    \
let unset = ctx.sample.d_min > 0.0;\n    return dbg_flag(unset);\n}\n";

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The workspace this crate is in.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

/// Each `.wgsl` file directly in the repo's `dir`, as (path relative to the root, source).
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
    out.sort();
    out
}

/// A scratch workspace holding the repo's prelude, library, unpack layer and read side, the stain's context when
/// `context`, and `files` (each a path relative to the root and its source).
fn workspace(context: bool, files: &[(String, String)]) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "lint_wgsl_views_{}_{}",
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
    let mut all = files_in(LIB_FILES);
    for path in [GENERATED, READ_SIDE_FILE] {
        all.push((path.to_owned(), read(&repo().join(path))));
    }
    if context {
        all.extend(files_in(STAIN_DIR));
    }
    all.extend(files.iter().cloned());
    for (rel, text) in &all {
        write(rel, text);
    }
    assert!(root.join(PRELUDE).is_file(), "the prelude was not copied");
    root
}

/// The repo's generated views, with the stain's context when `context`, each lint with every rule holding, and so
/// does the context.
fn check_views_lint(context: bool) {
    let views = files_in(DEBUG_VIEWS);
    assert!(
        views.iter().any(|(p, _)| p.ends_with("/state.wgsl")),
        "the generated views are missing: run `cargo xtask codegen`"
    );
    let root = workspace(context, &views);
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let reports = match result {
        Ok(r) => r,
        Err(e) => panic!("the generated views did not lint: {e}"),
    };
    let stain = files_in(STAIN_DIR);
    for (file, _) in views.iter().chain(&stain) {
        let report = reports.iter().find(|r| &r.file == file);
        assert!(
            report.is_some_and(|r| r.findings.is_empty()),
            "{file} was not linted clean: {report:?}"
        );
    }
}

#[test]
fn lint_wgsl_views_lint_after_the_stain_context() {
    check_views_lint(true);
}

negative_control!(
    lint_wgsl_views_lint_after_the_stain_context,
    "without the stain's context a view's `Ctx` is not in scope",
    expected = "the generated views did not lint",
    check_views_lint(false)
);

/// `view`, as a generated view, lints and fails at its own second line with the self-comparison rule.
fn check_view_fires(view: &str) {
    let path = format!("{DEBUG_VIEWS}/d_min.wgsl");
    let root = workspace(true, &[(path.clone(), view.to_owned())]);
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let reports = result.expect("the view parses and validates as presented");
    let found: Vec<String> = reports
        .iter()
        .flat_map(|r| r.findings.iter().map(|f| f.at(&r.file)))
        .collect();
    let want = format!("{path}:2: [{}] ", Rule::SelfCompare);
    assert!(
        found.iter().any(|l| l.starts_with(&want)),
        "the view's self-comparison did not fire at its line 2: {found:?}"
    );
    assert!(
        found.iter().all(|l| l.starts_with(&path)),
        "a finding was reported off the view: {found:?}"
    );
}

#[test]
fn lint_wgsl_views_fire_at_their_line() {
    check_view_fires(VIEW_FIRES);
}

negative_control!(
    lint_wgsl_views_fire_at_their_line,
    "a clean view must not fire",
    expected = "did not fire at its line 2",
    check_view_fires(VIEW_CLEAN)
);
