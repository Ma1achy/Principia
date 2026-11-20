//! `cargo xtask lint wgsl` over the debug views TASK-M1-12 adds: the catalogue's generated reductions,
//! `crates/render/frag/debug/reductions/`, and the hand-written debug views, `crates/render/shaders/wgsl/frag/debug/`.
//! Each is a colour occupant reading `ctx`, so each is linted as a generated view is, after the stain's context, whose
//! `Ctx` it reads, and its `// @uniform` block: it parses, validates and is held to the float rules (R-351, R-352), its
//! findings at its own lines. Each test has a registered negative control (R-176).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;
use xtask::lint_wgsl::{
    lint, Rule, DEBUG_OCCUPANTS, GENERATED, LIB_FILES, PRELUDE, READ_SIDE_FILE, REDUCTION_VIEWS,
    STAIN_DIR,
};

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

/// Each `.wgsl` file under the repo's `dir`, its subdirectories included, as (path relative to the root, source),
/// sorted.
fn files_under(dir: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(repo().join(dir)).expect("the directory") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        if path.is_dir() {
            out.extend(files_under(&format!("{dir}/{name}")));
        } else if name.ends_with(".wgsl") {
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
        "lint_wgsl_debug_views_{}_{}",
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
    let mut all: Vec<(String, String)> = files_under(LIB_FILES)
        .into_iter()
        .filter(|(p, _)| p.matches('/').count() == LIB_FILES.matches('/').count() + 1)
        .collect();
    for path in [GENERATED, READ_SIDE_FILE] {
        all.push((path.to_owned(), read(&repo().join(path))));
    }
    if context {
        all.extend(files_under(STAIN_DIR));
    }
    all.extend(files.iter().cloned());
    for (rel, text) in &all {
        write(rel, text);
    }
    assert!(root.join(PRELUDE).is_file(), "the prelude was not copied");
    root
}

/// The repo's reductions and hand-written debug views.
fn views() -> Vec<(String, String)> {
    let mut views = files_under(REDUCTION_VIEWS);
    views.extend(files_under(DEBUG_OCCUPANTS));
    for want in [
        "reductions/n_dircos.wgsl",
        "reductions/masses_ternary.wgsl",
        "debug/live_shape.wgsl",
        "debug/word/symbol_at_k.wgsl",
    ] {
        assert!(
            views.iter().any(|(p, _)| p.ends_with(want)),
            "the debug views have no `{want}`"
        );
    }
    views
}

/// The repo's reductions and hand-written debug views, with the stain's context when `context`, each lint with every
/// rule holding.
fn check_views_lint(context: bool) {
    let views = views();
    let root = workspace(context, &views);
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let reports = match result {
        Ok(r) => r,
        Err(e) => panic!("the debug views did not lint: {e}"),
    };
    for (file, _) in &views {
        let report = reports.iter().find(|r| &r.file == file);
        assert!(
            report.is_some_and(|r| r.findings.is_empty()),
            "{file} was not linted clean: {report:?}"
        );
    }
}

#[test]
fn lint_wgsl_debug_views_lint_after_the_stain_context() {
    check_views_lint(true);
}

negative_control!(
    lint_wgsl_debug_views_lint_after_the_stain_context,
    "without the stain's context a view's `Ctx` is not in scope",
    expected = "the debug views did not lint",
    check_views_lint(false)
);

/// A view whose second line compares a field with itself, and whose `u_k` uniform its header declares.
const VIEW_FIRES: &str = "// @uniform u_k: u32 = 0 [0, 75]\nfn colour(ctx: Ctx) -> vec3<f32> {\n    \
let unset = ctx.sample.d_min != ctx.sample.d_min || uniforms.u_k > 3u;\n    return dbg_flag(unset);\n}\n";

/// The same view, clean: the negative control's input.
#[cfg(feature = "controls")]
const VIEW_CLEAN: &str =
    "// @uniform u_k: u32 = 0 [0, 75]\nfn colour(ctx: Ctx) -> vec3<f32> {\n    \
let unset = ctx.sample.d_min > 0.0 || uniforms.u_k > 3u;\n    return dbg_flag(unset);\n}\n";

/// `view`, as a reduction and as a hand-written view in a subdirectory, lints and fails at its own third line with the
/// self-comparison rule.
fn check_view_fires(view: &str) {
    for path in [
        format!("{REDUCTION_VIEWS}/probe.wgsl"),
        format!("{DEBUG_OCCUPANTS}/word/probe.wgsl"),
    ] {
        let root = workspace(true, &[(path.clone(), view.to_owned())]);
        let result = lint(&root);
        let _ = std::fs::remove_dir_all(&root);
        let reports = result.unwrap_or_else(|e| panic!("{path} does not parse as presented: {e}"));
        let found: Vec<String> = reports
            .iter()
            .flat_map(|r| r.findings.iter().map(|f| f.at(&r.file)))
            .collect();
        let want = format!("{path}:3: [{}] ", Rule::SelfCompare);
        assert!(
            found.iter().any(|l| l.starts_with(&want)),
            "{path}'s self-comparison did not fire at its line 3: {found:?}"
        );
        assert!(
            found.iter().all(|l| l.starts_with(&path)),
            "a finding was reported off the view: {found:?}"
        );
    }
}

#[test]
fn lint_wgsl_debug_views_fire_at_their_line() {
    check_view_fires(VIEW_FIRES);
}

negative_control!(
    lint_wgsl_debug_views_fire_at_their_line,
    "a clean view must not fire",
    expected = "did not fire at its line 3",
    check_view_fires(VIEW_CLEAN)
);
