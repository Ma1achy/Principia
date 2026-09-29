//! QA tests for TASK-M0-08's `cargo xtask lint constants`, written from REQ-SYS-001 and REQ-SYS-005 as the task's
//! acceptance line and dd_generation_root §3.8 ("Reading a constant") state them: a numeric `const` or `static` in
//! `crates/{kernel,ledger,engine}` not read from the register fails, naming file and line; the generated files and
//! the register are exempt. Each test has a registered negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::lint_constants::{check, REGISTER};

/// A fresh tree under the target's temp dir holding `files` (path relative to the root, contents).
fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m008_{name}"));
    let _ = fs::remove_dir_all(&root);
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("dirs created");
        fs::write(path, text).expect("file written");
    }
    root
}

/// The findings under `root` (the register excluded), each as `path:line:` followed by the constant's name.
fn findings(root: &Path) -> Vec<(String, String)> {
    check(root, &[PathBuf::from(REGISTER)])
        .expect("the lint ran")
        .iter()
        .map(|f| {
            let text = f.to_string();
            let head = text.split(": ").next().unwrap_or("").to_string();
            (head, f.name.clone())
        })
        .collect()
}

/// Asserts the findings under `root` are exactly `expected` (as `path:line`, name), in any order.
fn check_findings(root: &Path, expected: &[(&str, &str)]) {
    let mut got = findings(root);
    let mut want: Vec<(String, String)> = expected
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    got.sort();
    want.sort();
    assert_eq!(got, want, "the lint's findings are not the expected ones");
}

/// Numeric constants in each of the three crates, a nested module file, `build.rs`, a `static`, a negative literal,
/// an associated const and a const whose line follows a multi-byte comment and a raw string with a quote in it.
const KERNEL_NESTED: &str = "// é — ≤ ∞\n\
pub struct S;\n\
impl S {\n\
    pub const ASSOC: u32 = 1 << 4;\n\
}\n\
const RAW: &str = r#\"a \" b\"#;\n\
pub(crate) static NEG: f64 = -1.0;\n";

fn bad_tree() -> PathBuf {
    tree(
        "bad",
        &[
            ("crates/kernel/src/lib.rs", "pub mod deep;\n"),
            ("crates/kernel/src/deep/mod.rs", KERNEL_NESTED),
            (
                "crates/engine/src/lib.rs",
                "\n\npub const E: [f64; 2] = [0.5, 2.0];\n",
            ),
            ("crates/ledger/src/other.rs", "const L: usize = 76;\n"),
            ("crates/ledger/build.rs", "fn main() {}\nconst B: u8 = 7;\n"),
            (REGISTER, "pub const IN_REGISTER: f64 = 65504.0;\n"),
        ],
    )
}

#[test]
fn qa_lint_constants_names_file_and_line_in_every_linted_crate() {
    check_findings(
        &bad_tree(),
        &[
            ("crates/kernel/src/deep/mod.rs:4", "ASSOC"),
            ("crates/kernel/src/deep/mod.rs:7", "NEG"),
            ("crates/engine/src/lib.rs:3", "E"),
            ("crates/ledger/src/other.rs:1", "L"),
            ("crates/ledger/build.rs:2", "B"),
        ],
    );
}

negative_control!(
    qa_lint_constants_names_file_and_line_in_every_linted_crate,
    "a tree whose constants all read the register has no findings, so the expected-findings check must fail",
    expected = "the lint's findings are not the expected ones",
    check_findings(
        &tree(
            "clean",
            &[(
                "crates/kernel/src/lib.rs",
                "const R: f64 = ledger::constants::F16_FINITE_MAX.number();\n"
            )]
        ),
        &[("crates/kernel/src/lib.rs:1", "R")]
    )
);

/// A constant read from the register but altered by a literal is not "read from the register".
#[test]
fn qa_lint_constants_flags_a_register_read_altered_by_a_literal() {
    check_findings(
        &tree(
            "altered",
            &[(
                "crates/kernel/src/lib.rs",
                "const HALF: f64 = ledger::constants::F16_FINITE_MAX.number() / 2.0;\n",
            )],
        ),
        &[("crates/kernel/src/lib.rs:1", "HALF")],
    );
}

negative_control!(
    qa_lint_constants_flags_a_register_read_altered_by_a_literal,
    "an unaltered register read is no finding, so the expected-finding check must fail",
    expected = "the lint's findings are not the expected ones",
    check_findings(
        &tree(
            "unaltered",
            &[(
                "crates/kernel/src/lib.rs",
                "const HALF: f64 = ledger::constants::F16_FINITE_MAX.number();\n"
            )]
        ),
        &[("crates/kernel/src/lib.rs:1", "HALF")]
    )
);

/// The lint on this workspace, through `cargo xtask lint constants`'s entry point: it passes (acceptance).
#[test]
fn qa_lint_constants_run_passes_on_the_workspace() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
    xtask::lint_constants::run(&manifest).expect("the lint fails on the tree");
}

negative_control!(
    qa_lint_constants_run_passes_on_the_workspace,
    "a workspace with a bare numeric const in kernel must fail the run",
    expected = "the lint fails on the tree",
    {
        let root = tree(
            "run_bad",
            &[
                ("Cargo.toml", "[workspace]\n"),
                ("crates/kernel/src/lib.rs", "const BARE: f64 = 2e-3;\n"),
            ],
        );
        xtask::lint_constants::run(&root.join("Cargo.toml")).expect("the lint fails on the tree");
    }
);
