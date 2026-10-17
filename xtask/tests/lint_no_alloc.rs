//! `cargo xtask lint no-alloc` (integrator_contract Part 2c rule 5; REQ-PERF-002): it passes on this tree; a `Vec`,
//! `Box`, `dyn`, `std::` or `alloc::` in the kernel's physics or driver layer fails it, named by file, line and token;
//! comments, strings, longer identifiers and paths that only end in `std` are not findings; files outside the two
//! layers are not read.

use std::fs;
use std::path::Path;
use std::process::Command;

use validation::negative_control;
use xtask::lint_no_alloc::{check, run, scan, Finding};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

/// The lint finds nothing under `root`.
fn check_clean(root: &Path) {
    let found = check(root).expect("lint ran");
    assert!(
        found.is_empty(),
        "allocation or std use(s) in the kernel's physics or driver layer: {found:?}"
    );
}

#[test]
fn lint_no_alloc_passes_on_the_tree() {
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
    lint_no_alloc_passes_on_the_tree,
    "a tree with a planted `Vec` in the physics layer must fail the clean check",
    expected = "allocation or std use(s) in the kernel's physics or driver layer",
    check_clean(&tree(
        "lint_no_alloc_control",
        &[(
            "crates/kernel/src/physics/force.rs",
            "fn f() -> Vec<f32> { todo!() }\n"
        )]
    ))
);

/// Uses that are not findings: comments, strings, longer identifiers, and `std` or `alloc` not followed by `::`.
const QUIET: &str = r#"// let v: Vec<f32> = Vec::new(); Box<dyn Fn()>; std::mem::swap; alloc::vec
/* Box::new(1) */
const S: &str = "Vec Box dyn std::f32 alloc::vec";
struct MyVec; struct Boxed; fn dynamic() {} fn vec2() {}
use spirv_std::num_traits::Float;
fn std() {} fn alloc() {} extern crate std as other;
"#;

/// The seeded violations, one of each token, in the physics and driver directories and a driver module file, beside
/// [`QUIET`]; and the same tokens outside the two layers.
fn seeded() -> Vec<(&'static str, String)> {
    vec![
        (
            "crates/kernel/src/physics/force.rs",
            format!("{QUIET}fn f() -> Vec<f32> {{ todo!() }}\nfn g(x: &dyn Fn()) {{}}\n"),
        ),
        (
            "crates/kernel/src/physics/deep/pairs.rs",
            format!("{QUIET}fn h() -> Box<f32> {{ todo!() }}\n"),
        ),
        (
            "crates/kernel/src/driver/step.rs",
            format!("{QUIET}use std :: vec;\nuse alloc::boxed;\n"),
        ),
        (
            "crates/kernel/src/driver.rs",
            format!("{QUIET}fn i() {{ let _ = ::std::mem::size_of::<u8>(); }}\n"),
        ),
        (
            "crates/kernel/src/decode/mod.rs",
            "fn j() -> Vec<f32> { todo!() }\n".to_owned(),
        ),
        (
            "crates/kernel/src/physics.txt",
            "Vec Box dyn std::\n".to_owned(),
        ),
        ("crates/kernel/src/physics/clean.rs", QUIET.to_owned()),
    ]
}

/// `found` is exactly the seeded violations, each with its file, line and token, in path order (by component, so
/// `driver/` before `driver.rs`).
fn check_seeded(found: &[Finding]) {
    let lines: Vec<String> = found.iter().map(ToString::to_string).collect();
    let rule = "in the kernel's physics or driver layer, which allocates nothing and uses no `std` (Part 2c rule 5)";
    assert_eq!(
        lines,
        [
            format!("crates/kernel/src/driver/step.rs:7: `std::` {rule}"),
            format!("crates/kernel/src/driver/step.rs:8: `alloc::` {rule}"),
            format!("crates/kernel/src/driver.rs:7: `std::` {rule}"),
            format!("crates/kernel/src/physics/deep/pairs.rs:7: `Box` {rule}"),
            format!("crates/kernel/src/physics/force.rs:7: `Vec` {rule}"),
            format!("crates/kernel/src/physics/force.rs:8: `dyn` {rule}"),
        ],
        "the lint does not find exactly the seeded violations"
    );
}

#[test]
fn lint_no_alloc_seeded_violations_fail() {
    let files = seeded();
    let refs: Vec<(&str, &str)> = files.iter().map(|(p, s)| (*p, s.as_str())).collect();
    let root = tree("lint_no_alloc_seeded", &refs);
    check_seeded(&check(&root).expect("lint ran"));
}

negative_control!(
    lint_no_alloc_seeded_violations_fail,
    "the seeded tree with the driver's module file removed",
    expected = "the lint does not find exactly the seeded violations",
    {
        let files = seeded();
        let refs: Vec<(&str, &str)> = files
            .iter()
            .filter(|(p, _)| *p != "crates/kernel/src/driver.rs")
            .map(|(p, s)| (*p, s.as_str()))
            .collect();
        let root = tree("lint_no_alloc_seeded_control", &refs);
        check_seeded(&check(&root).expect("lint ran"));
    }
);

/// `scan` finds each token on its own line, and nothing in [`QUIET`].
fn check_scan(source: &str) {
    assert_eq!(
        scan(source),
        [
            (7, "Vec"),
            (8, "Box"),
            (9, "dyn"),
            (10, "std::"),
            (11, "alloc::"),
        ],
        "scan does not find the tokens alone"
    );
}

#[test]
fn lint_no_alloc_scan_whole_words() {
    check_scan(&format!(
        "{QUIET}let v: Vec<u8>;\nlet b = Box::new(1);\nlet d: &dyn Fn();\nstd ::mem::drop(1);\nalloc::vec![];\n"
    ));
}

negative_control!(
    lint_no_alloc_scan_whole_words,
    "a source with the tokens in comments only",
    expected = "scan does not find the tokens alone",
    check_scan(&format!(
        "{QUIET}// let v: Vec<u8>;\n// Box::new(1);\n// &dyn Fn();\n// std::mem::drop(1);\n// alloc::vec![];\n"
    ))
);

/// `run` fails on a tree with a violation, counting it, and passes on a clean one.
fn check_run(violating: &Path, clean: &Path) {
    let err = run(&violating.join("Cargo.toml")).expect_err("the lint passes a violation");
    assert!(
        err.starts_with(
            "1 `Vec`, `Box`, `dyn`, `std::` or `alloc::` use(s) in the kernel's physics and driver layers"
        ),
        "{err}"
    );
    assert_eq!(
        run(&clean.join("Cargo.toml")),
        Ok(()),
        "the lint does not pass a clean tree"
    );
}

#[test]
fn lint_no_alloc_run_fails_on_a_violation() {
    let violating = tree(
        "lint_no_alloc_run",
        &[(
            "crates/kernel/src/physics/energy.rs",
            "fn e() -> Vec<f64> { todo!() }\n",
        )],
    );
    let clean = tree(
        "lint_no_alloc_run_clean",
        &[("crates/kernel/src/physics/energy.rs", QUIET)],
    );
    check_run(&violating, &clean);
}

negative_control!(
    lint_no_alloc_run_fails_on_a_violation,
    "a violating tree taken as the clean one",
    expected = "the lint does not pass a clean tree",
    {
        let violating = tree(
            "lint_no_alloc_run_control",
            &[(
                "crates/kernel/src/physics/energy.rs",
                "fn e() -> Vec<f64> { todo!() }\n",
            )],
        );
        check_run(&violating, &violating);
    }
);

// The `ci` registry's `lint no-alloc` runner runs the lint on this workspace.

const CHILD: &str = "XTASK_LINT_NO_ALLOC_CI_CHILD";
const RAN: &str =
    "xtask lint no-alloc: no `Vec`, `Box`, `dyn`, `std::` or `alloc::` in the kernel's";

fn check_ran(stdout: &str) {
    assert!(
        stdout.contains(RAN),
        "the ci runner `lint no-alloc` did not run the lint; its output: {stdout}"
    );
}

/// Run as a child (with `CHILD` set), calls the registry's `lint no-alloc` runner; otherwise runs itself as that
/// child, output uncaptured, and checks the lint's report is in its output.
#[test]
fn lint_no_alloc_ci_runner_runs_the_lint() {
    if std::env::var_os(CHILD).is_some() {
        let runner = xtask::ci::RUNNERS
            .iter()
            .find(|r| r.name == "lint no-alloc")
            .expect("the registry has `lint no-alloc`");
        (runner.run)().expect("the lint passes on this workspace");
        return;
    }
    let out = Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "lint_no_alloc_ci_runner_runs_the_lint",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .output()
        .expect("the test binary runs");
    assert!(
        out.status.success(),
        "the child failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    check_ran(&String::from_utf8_lossy(&out.stdout));
}

negative_control!(
    lint_no_alloc_ci_runner_runs_the_lint,
    "output with no lint report is not the lint having run",
    expected = "did not run the lint",
    check_ran("xtask ci: [8/10] lint no-alloc ok\n")
);
