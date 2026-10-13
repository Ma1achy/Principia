//! `cargo xtask lint wgsl` over the occupants that read the stain's `Ctx` (TASK-M1-13): a post or debug occupant under
//! `crates/render/shaders/wgsl/frag/` is linted after the stain's context, as a generated view is, and its
//! `// @uniform` block, so it parses, validates and is held to the float rules, its findings at its own lines; a
//! combiner, which reads no `Ctx`, keeps the prelude-and-library context (TASK-M7-04). Each test has a registered
//! negative control (R-176).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;
use xtask::lint_wgsl::{
    lint, Rule, GENERATED, LIB_FILES, OCCUPANT_DIR, PRELUDE, READ_SIDE_FILE, STAIN_DIR,
};

/// A post occupant reading `ctx.quad` and its uniforms, its sixth line comparing a float with itself.
const POST_FIRES: &str = "// post\n// @uniform width: f32 = 2.0\n\
fn post(ctx: Ctx, rgb: vec3<f32>) -> vec3<f32> {\n    \
let a = ctx.quad.uv.x * uniforms.width;\n    \
let t = ctx.tile.uv.y;\n    \
return select(rgb, vec3<f32>(a), t != t);\n}\n";

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

/// A scratch workspace holding the repository's prelude, library, generated layer, read side and stain context, and
/// `occupant` at `rel`.
fn workspace(rel: &str, occupant: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "lint_wgsl_ctx_occupant_{}_{}",
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
    let repo = repo();
    for file in [PRELUDE, GENERATED, READ_SIDE_FILE] {
        write(file, &read(&repo.join(file)));
    }
    for dir in [LIB_FILES, STAIN_DIR] {
        for entry in std::fs::read_dir(repo.join(dir)).expect("the directory") {
            let path = entry.expect("an entry").path();
            let name = path
                .file_name()
                .expect("a name")
                .to_string_lossy()
                .into_owned();
            if name.ends_with(".wgsl") {
                write(&format!("{dir}/{name}"), &read(&path));
            }
        }
    }
    write(rel, occupant);
    root
}

/// `POST_FIRES` at `rel` lints and fails at its own sixth line with the self-comparison rule, and nowhere else.
fn check_fires_at_its_line(rel: &str) {
    let root = workspace(rel, POST_FIRES);
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let reports = match result {
        Ok(r) => r,
        Err(e) => panic!("the occupant does not parse and validate as presented: {e}"),
    };
    let found: Vec<String> = reports
        .iter()
        .flat_map(|r| r.findings.iter().map(|f| f.at(&r.file)))
        .collect();
    let want = format!("{rel}:6: [{}] ", Rule::SelfCompare);
    assert!(
        found.iter().any(|l| l.starts_with(&want)),
        "the occupant's self-comparison did not fire at its line 6: {found:?}"
    );
    assert!(
        found.iter().all(|l| l.starts_with(rel)),
        "a finding was reported off the occupant: {found:?}"
    );
}

#[test]
fn lint_wgsl_ctx_occupant_is_presented_after_the_stain_context() {
    check_fires_at_its_line(&format!("{OCCUPANT_DIR}/post/test.wgsl"));
    check_fires_at_its_line(&format!("{OCCUPANT_DIR}/debug/test.wgsl"));
}

negative_control!(
    lint_wgsl_ctx_occupant_is_presented_after_the_stain_context,
    "a post placed among the combiners is presented without the stain's context, where `Ctx` is not in scope",
    expected = "the occupant does not parse and validate as presented",
    check_fires_at_its_line(&format!("{OCCUPANT_DIR}/combiner/test.wgsl"))
);

/// The repository's structural occupants each lint with every rule holding.
fn check_structural_lint(files: &[&str]) {
    let reports = lint(&repo()).unwrap_or_else(|e| panic!("the workspace did not lint: {e}"));
    for file in files {
        let rel = format!("{OCCUPANT_DIR}/{file}");
        let report = reports.iter().find(|r| r.file == rel);
        assert!(
            report.is_some_and(|r| r.findings.is_empty()),
            "{rel} was not linted clean: {report:?}"
        );
    }
}

#[test]
fn lint_wgsl_ctx_occupant_structural_occupants_lint_clean() {
    check_structural_lint(&[
        "post/edge_line.wgsl",
        "post/fallback_tint.wgsl",
        "post/pending_hatch.wgsl",
        "debug/s_depth.wgsl",
        "debug/s_state.wgsl",
        "debug/s_coherence.wgsl",
        "debug/s_impurity.wgsl",
        "debug/s_spread.wgsl",
        "debug/s_suspect.wgsl",
        "debug/s_priority.wgsl",
        "debug/s_cache_age.wgsl",
        "debug/s_ancestor_gap.wgsl",
    ]);
}

negative_control!(
    lint_wgsl_ctx_occupant_structural_occupants_lint_clean,
    "a structural occupant that is not there is not linted",
    expected = "was not linted clean",
    check_structural_lint(&["post/no_such_post.wgsl"])
);
