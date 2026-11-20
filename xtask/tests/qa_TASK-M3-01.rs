//! QA's tests for TASK-M3-01's `cargo xtask lint no-alloc` (REQ-PERF-002; integrator_contract Part 2c rule 5): the
//! kernel's hot path, its physics and driver layers, allocates nothing. Written from the requirement's verify ("no
//! Vec/Box/dyn/std allocation in step, deriv, Hamiltonians, physics/ or the driver loop") and the task's acceptance
//! ("a planted `Vec` … makes it fail"):
//! - each of `Vec`, `Box`, `dyn`, `std::` and `alloc::`, planted in a file nested deep in the driver layer, or in the
//!   driver's module file `driver.rs`, or in a physics file, fails the lint, through `run` as through `check`;
//! - glam's `Vec2`, `core::` paths and `#![no_std]` itself, which the kernel uses, are not findings;
//! - this tree passes.

use std::fs;
use std::path::Path;

use validation::negative_control;
use xtask::lint_no_alloc::{check, run, scan};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// A scratch workspace with a `Cargo.toml` and each `(path, source)` of `files`.
fn workspace(tag: &str, files: &[(&str, &str)]) -> Scratch {
    let root = Scratch::new(tag);
    fs::create_dir_all(&*root).expect("root");
    fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("manifest");
    for (path, source) in files {
        let file = root.join(path);
        fs::create_dir_all(file.parent().expect("parent")).expect("dirs");
        fs::write(file, source).expect("file");
    }
    root
}

/// The planted uses: each a hot-path allocation or `std` use the requirement forbids, in a different place.
const PLANTED: [(&str, &str, &str); 6] = [
    (
        "crates/kernel/src/driver/march/inner.rs",
        "pub fn f(n: usize) -> usize { let v: Vec<u8> = Vec::with_capacity(n); v.len() }\n",
        "Vec",
    ),
    (
        "crates/kernel/src/driver.rs",
        "pub fn g() -> Box<u8> { Box::new(1) }\n",
        "Box",
    ),
    (
        "crates/kernel/src/physics/step.rs",
        "pub fn h(f: &dyn Fn(f32) -> f32) -> f32 { f(1.0) }\n",
        "dyn",
    ),
    (
        "crates/kernel/src/physics/deriv.rs",
        "use std::collections::HashMap;\n",
        "std::",
    ),
    (
        "crates/kernel/src/physics/hamiltonian.rs",
        "pub fn k() { let _ = ::alloc::string::String::new(); }\n",
        "alloc::",
    ),
    (
        "crates/kernel/src/driver/mod.rs",
        "extern crate std as s;\n",
        "extern crate std",
    ),
];

/// Every planted use is found, at its file and line 1, and `run` fails on the tree.
fn check_planted(files: &[(&str, &str, &str)]) {
    let pairs: Vec<(&str, &str)> = files.iter().map(|(p, s, _)| (*p, *s)).collect();
    let root = workspace("qa_m3_01_planted", &pairs);
    let found = check(&root).expect("lint ran");
    for (path, _, token) in files {
        assert!(
            found
                .iter()
                .any(|f| f.path == Path::new(path) && f.line == 1 && f.token == *token),
            "planted `{token}` in {path} not found: {found:?}"
        );
    }
    assert!(
        run(&root.join("Cargo.toml")).is_err(),
        "the lint passes a tree with a planted allocation"
    );
}

#[test]
fn qa_no_alloc_planted_uses_fail() {
    check_planted(&PLANTED);
}

negative_control!(
    qa_no_alloc_planted_uses_fail,
    "a planted use the lint does not flag (a `Vec` in a non-.rs file) must fail the check",
    expected = "not found",
    check_planted(&[("crates/kernel/src/physics/notes.txt", "Vec<u8>\n", "Vec")])
);

/// What the kernel's no_std physics may write, and is no allocation: glam's vector types, `core::` paths, the crate
/// attribute, and identifiers merely containing a token.
const ALLOWED: &str =
    "#![no_std]\nuse glam::{Vec2, DVec2};\nuse core::ops::Add;\nfn dvec(v: Vec2) -> Vec2 { v }\n\
                       struct Boxes; fn dyne() {} fn stdev() {} fn realloc() {}\n";

fn check_allowed(source: &str) {
    let found = scan(source);
    assert!(found.is_empty(), "non-allocating code flagged: {found:?}");
}

#[test]
fn qa_no_alloc_allowed_uses_pass() {
    check_allowed(ALLOWED);
}

negative_control!(
    qa_no_alloc_allowed_uses_pass,
    "the same source with a `Vec<f32>` added must be flagged",
    expected = "non-allocating code flagged",
    check_allowed(&format!("{ALLOWED}fn v() -> Vec<f32> {{ todo!() }}\n"))
);

fn check_tree_clean(root: &Path) {
    let found = check(root).expect("lint ran");
    assert!(found.is_empty(), "the hot path allocates: {found:?}");
}

#[test]
fn qa_no_alloc_this_tree_passes() {
    check_tree_clean(&Path::new(env!("CARGO_MANIFEST_DIR")).join(".."));
}

negative_control!(
    qa_no_alloc_this_tree_passes,
    "a tree with a `Box` in the physics layer must fail",
    expected = "the hot path allocates",
    check_tree_clean(&workspace(
        "qa_m3_01_clean_control",
        &[(
            "crates/kernel/src/physics/force.rs",
            "fn b() -> Box<u8> { Box::new(0) }\n"
        )]
    ))
);
