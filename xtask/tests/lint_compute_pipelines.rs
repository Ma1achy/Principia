//! `cargo xtask lint compute-pipelines` (R-297; REQ-SYS-074): it passes on this tree; a compute pipeline created
//! outside the entry point fails it, and so does a vertex or fragment pipeline beside the passthrough, or the
//! passthrough outside the entry point, each named by file, line and identifier; comments, strings and longer
//! identifiers are not findings; `target` directories are not read.

use std::fs;
use std::path::Path;

use validation::negative_control;
use xtask::lint_compute::{check, scan, Finding, ENTRY_POINT};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// The lint finds nothing under `root`.
fn check_clean(root: &Path) {
    let found = check(root).expect("lint ran");
    assert!(
        found.is_empty(),
        "compute pipeline or passthrough use(s) outside the entry point: {found:?}"
    );
}

#[test]
fn lint_compute_pipelines_passes_on_the_tree() {
    check_clean(&Path::new(env!("CARGO_MANIFEST_DIR")).join(".."));
}

/// A scratch tree holding each `(path, source)` of `files`.
fn tree(tag: &str, files: &[(&str, &str)]) -> Scratch {
    let root = Scratch::new(tag);
    for (path, source) in files {
        let file = root.join(path);
        fs::create_dir_all(file.parent().expect("a parent")).expect("tree created");
        fs::write(file, source).expect("file written");
    }
    root
}

negative_control!(
    lint_compute_pipelines_passes_on_the_tree,
    "a tree that creates a compute pipeline outside the entry point must fail the clean check",
    expected = "compute pipeline or passthrough use(s) outside the entry point",
    check_clean(&tree(
        "lint_compute_control",
        &[(
            "crates/render/src/lib.rs",
            "fn f(d: &wgpu::Device) { d.create_compute_pipeline(todo!()); }\n"
        )]
    ))
);

/// Uses that are not findings anywhere: comments, strings, and identifiers that only contain a forbidden one.
const QUIET: &str = r#"// device.create_compute_pipeline(...)
/* device.create_shader_module_passthrough(...) */
const S: &str = "create_render_pipeline ComputePipelineDescriptor";
fn create_compute_pipeline_twice() {}
struct MyVertexState;
"#;

/// The seeded violations, one of each kind, beside [`QUIET`], and nothing in a `target` directory.
fn seeded() -> Vec<(&'static str, String)> {
    vec![
        (
            "crates/render/src/lib.rs",
            format!(
                "{QUIET}fn f(d: &wgpu::Device) {{\n    d.create_compute_pipeline(todo!());\n}}\n"
            ),
        ),
        (
            "xtask/src/x.rs",
            format!(
                "{QUIET}fn g() -> wgpu::ShaderModuleDescriptorPassthrough<'static> {{ todo!() }}\n"
            ),
        ),
        (
            ENTRY_POINT,
            format!(
                "{QUIET}fn h(d: &wgpu::Device) {{ d.create_compute_pipeline(todo!()); }}\n\
                 fn i(d: &wgpu::Device) {{ d.create_render_pipeline(todo!()); }}\n"
            ),
        ),
        (
            "crates/render/target/debug/build/x.rs",
            "fn j(d: &wgpu::Device) { d.create_compute_pipeline(todo!()); }\n".to_owned(),
        ),
        ("crates/render/src/clean.rs", QUIET.to_owned()),
    ]
}

/// `found` is exactly the seeded violations: the compute pipeline in render, the passthrough in xtask, and the render
/// pipeline beside the passthrough in the entry point, each with its file, line and rule.
fn check_seeded(found: &[Finding]) {
    let lines: Vec<String> = found.iter().map(ToString::to_string).collect();
    assert_eq!(
        lines,
        [
            format!(
                "{ENTRY_POINT}:7: `create_render_pipeline`: a vertex or fragment pipeline beside the passthrough in \
                 {ENTRY_POINT} (R-297)"
            ),
            format!(
                "crates/render/src/lib.rs:7: `create_compute_pipeline`: a compute pipeline created outside the entry \
                 point, {ENTRY_POINT} (R-297)"
            ),
            format!(
                "xtask/src/x.rs:6: `ShaderModuleDescriptorPassthrough`: wgpu's passthrough outside the compute \
                 entry point, {ENTRY_POINT} (R-297)"
            ),
        ],
        "the lint does not find exactly the seeded violations"
    );
}

#[test]
fn lint_compute_pipelines_seeded_violations_fail() {
    let files = seeded();
    let refs: Vec<(&str, &str)> = files.iter().map(|(p, s)| (*p, s.as_str())).collect();
    let root = tree("lint_compute_seeded", &refs);
    check_seeded(&check(&root).expect("lint ran"));
}

negative_control!(
    lint_compute_pipelines_seeded_violations_fail,
    "the seeded tree with the render pipeline beside the passthrough removed",
    expected = "the lint does not find exactly the seeded violations",
    {
        let files = seeded();
        let refs: Vec<(&str, &str)> = files
            .iter()
            .filter(|(p, _)| *p != ENTRY_POINT)
            .map(|(p, s)| (*p, s.as_str()))
            .collect();
        let root = tree("lint_compute_seeded_control", &refs);
        check_seeded(&check(&root).expect("lint ran"));
    }
);

/// `scan` finds each identifier on its own line and nothing in [`QUIET`].
fn check_scan(source: &str) {
    assert_eq!(
        scan(source, &["create_compute_pipeline", "VertexState"]),
        [(6, "create_compute_pipeline"), (7, "VertexState")],
        "scan does not find the identifiers alone"
    );
}

#[test]
fn lint_compute_pipelines_scan_whole_words() {
    check_scan(&format!(
        "{QUIET}x.create_compute_pipeline(d);\nlet v: wgpu::VertexState;\n"
    ));
}

negative_control!(
    lint_compute_pipelines_scan_whole_words,
    "a source with the uses in comments only",
    expected = "scan does not find the identifiers alone",
    check_scan(&format!(
        "{QUIET}// x.create_compute_pipeline(d);\n// let v: wgpu::VertexState;\n"
    ))
);
