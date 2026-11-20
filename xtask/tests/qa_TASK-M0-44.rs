//! QA tests for TASK-M0-44's lint, `cargo xtask lint compute-pipelines`, written from REQ-SYS-074 ("every compute
//! pipeline must be created through the one entry point that takes the setting ... vertex and fragment pipelines keep
//! wgpu's own path"; verify: "a lint finds no compute pipeline created outside the entry point and no vertex or fragment
//! pipeline created through the passthrough") and the task's acceptance line ("a seeded violation of each fails it").
//! The seeded violations are written here in shapes the implementer's tests don't use: a fully qualified call, a
//! renamed import, a call split over lines, a pipeline in a test file and in an example, and the passthrough beside a
//! render pipeline outside the entry point. Each test registers its negative control (R-176).
// The file name `qa_TASK-M0-44` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::lint_compute::{check, ENTRY_POINT};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// A scratch workspace holding each `(path, source)`.
fn tree(tag: &str, files: &[(&str, &str)]) -> Scratch {
    let root = Scratch::new(tag);
    for (path, source) in files {
        let file = root.join(path);
        fs::create_dir_all(file.parent().expect("a parent")).expect("tree created");
        fs::write(file, source).expect("file written");
    }
    root
}

/// The entry point's file, as a clean one: it creates a compute pipeline and uses the passthrough.
const ENTRY_CLEAN: &str = "pub fn pipeline(d: &wgpu::Device) {\n    \
    let m = unsafe { d.create_shader_module_passthrough(todo!()) };\n    \
    d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { module: &m, ..todo!() });\n}\n";

/// The display, as a clean one: a render pipeline through wgpu's own path.
const RENDER_CLEAN: &str = "pub fn display(d: &wgpu::Device) {\n    \
    let m = d.create_shader_module(todo!());\n    \
    d.create_render_pipeline(&wgpu::RenderPipelineDescriptor { vertex: wgpu::VertexState { module: &m, ..todo!() }, \
    ..todo!() });\n}\n";

/// The seeded violations, each `(file, source, line, identifier)` the lint must name.
fn seeded() -> Vec<(&'static str, String, usize, &'static str)> {
    vec![
        (
            "crates/kernel/src/qualified.rs",
            "fn f(d: &wgpu::Device) {\n    let _ = wgpu::Device::create_compute_pipeline(d, todo!());\n}\n".into(),
            2,
            "create_compute_pipeline",
        ),
        (
            "crates/render/src/renamed.rs",
            "use wgpu::ComputePipelineDescriptor as Desc;\nfn f(_: Desc<'_>) {}\n".into(),
            1,
            "ComputePipelineDescriptor",
        ),
        (
            "crates/validation/src/split.rs",
            "fn f(d: &wgpu::Device) {\n    d\n        .create_compute_pipeline(todo!());\n}\n".into(),
            3,
            "create_compute_pipeline",
        ),
        (
            "crates/engine/tests/in_a_test.rs",
            "#[test]\nfn t() {\n    let d: wgpu::Device = todo!();\n    d.create_compute_pipeline(todo!());\n}\n"
                .into(),
            4,
            "create_compute_pipeline",
        ),
        (
            "crates/gui/examples/demo.rs",
            "fn main() {\n    let d: wgpu::Device = todo!();\n    d.create_compute_pipeline(todo!());\n}\n".into(),
            3,
            "create_compute_pipeline",
        ),
        (
            "crates/render/src/fragment_passthrough.rs",
            format!(
                "{RENDER_CLEAN}fn g(d: &wgpu::Device) {{\n    let m = unsafe {{ d.create_shader_module_passthrough(todo!()) }};\n}}\n"
            ),
            6,
            "create_shader_module_passthrough",
        ),
        (
            ENTRY_POINT,
            format!("{ENTRY_CLEAN}fn display(d: &wgpu::Device) {{\n    d.create_render_pipeline(todo!());\n}}\n"),
            6,
            "create_render_pipeline",
        ),
    ]
}

/// Each seeded violation, alone in a tree beside a clean entry point and display, is a finding naming its file, line
/// and identifier.
fn check_each_seeded(violations: &[(&'static str, String, usize, &'static str)]) {
    for (path, source, line, ident) in violations {
        let mut files = vec![(*path, source.as_str())];
        if *path != ENTRY_POINT {
            files.push((ENTRY_POINT, ENTRY_CLEAN));
        }
        files.push(("crates/render/src/display.rs", RENDER_CLEAN));
        let root = tree("qa44_seeded", &files);
        let found = check(&root).expect("lint ran");
        let named: Vec<(PathBuf, usize, &str)> = found
            .iter()
            .map(|f| (f.path.clone(), f.line, f.ident))
            .collect();
        assert!(
            named.contains(&(PathBuf::from(path), *line, *ident)),
            "the lint misses a seeded violation: {path}:{line} `{ident}`; it found {named:?}"
        );
    }
}

#[test]
fn qa_lint_compute_pipelines_seeded_shapes_fail() {
    check_each_seeded(&seeded());
}

negative_control!(
    qa_lint_compute_pipelines_seeded_shapes_fail,
    "a seeded 'violation' that is only a comment",
    expected = "the lint misses a seeded violation",
    check_each_seeded(&[(
        "crates/kernel/src/comment.rs",
        "// d.create_compute_pipeline(x)\n".into(),
        1,
        "create_compute_pipeline",
    )])
);

/// A clean tree: the entry point creating its compute pipeline through the passthrough, the display its render
/// pipeline through wgpu's own path, and every forbidden identifier elsewhere in a comment, a string or a longer name.
fn check_clean(root: &Path) {
    let found = check(root).expect("lint ran");
    assert!(found.is_empty(), "the lint flags a clean tree: {found:?}");
}

#[test]
fn qa_lint_compute_pipelines_clean_tree_passes() {
    let root = tree(
        "qa44_clean",
        &[
            (ENTRY_POINT, ENTRY_CLEAN),
            ("crates/render/src/display.rs", RENDER_CLEAN),
            (
                "crates/kernel/src/quiet.rs",
                "/// Built by `create_compute_pipeline` in compute.rs, never here.\n\
                 const NOTE: &str = \"create_shader_module_passthrough\";\n\
                 fn my_create_compute_pipeline() {}\n",
            ),
        ],
    );
    check_clean(&root);
}

negative_control!(
    qa_lint_compute_pipelines_clean_tree_passes,
    "the clean tree with the display's render pipeline moved into the entry point",
    expected = "the lint flags a clean tree",
    check_clean(&tree(
        "qa44_clean_control",
        &[(ENTRY_POINT, &format!("{ENTRY_CLEAN}{RENDER_CLEAN}"))]
    ))
);
