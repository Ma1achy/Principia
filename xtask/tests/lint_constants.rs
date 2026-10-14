//! `cargo xtask lint constants` (dd_generation_root §3.8; REQ-SYS-001, REQ-SYS-005): it passes on this tree, and a
//! bare numeric `const` in `crates/kernel` fails it, naming the file and line; comments, strings, `const fn` and
//! constants read from the register are not findings; the register and the generated files are exempt; test code, an
//! item under `#[cfg(test)]` and every file of a module declared under it, is not read.

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::lint_constants::{check, exempt};

/// The lint finds nothing under `root`, skipping what it exempts: the register and the generated files (dd_generation_root
/// §3.8).
fn check_clean(root: &Path) {
    let found = check(root, &exempt().expect("exemptions listed")).expect("lint ran");
    assert!(
        found.is_empty(),
        "numeric constant(s) not read from the register: {found:?}"
    );
}

#[test]
fn lint_constants_passes_on_the_tree() {
    check_clean(&Path::new(env!("CARGO_MANIFEST_DIR")).join(".."));
}

/// A tree whose `crates/kernel/src/lib.rs` is `source`.
fn tree(name: &str, source: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let src = root.join("crates/kernel/src");
    fs::create_dir_all(&src).expect("tree created");
    fs::write(src.join("lib.rs"), source).expect("lib.rs written");
    root
}

const KERNEL: &str = r#"//! A kernel. // const IN_DOC: u32 = 1;
/* const IN_BLOCK: f64 = 2.0; */
const fn limit(x: &'static str) -> usize { x.len() + 3 }
const READ: u32 = ledger::constants::FGW_CAPACITY.number() as u32;
const TEXT: &str = "const IN_STRING: u32 = 4;";
pub struct Grid<const N: usize>;
"#;

#[test]
fn lint_constants_bare_const_in_kernel_fails_naming_file_and_line() {
    let root = tree(
        "lint_bare",
        &format!("{KERNEL}pub const BARE: f64 = 2e-3;\n"),
    );
    let found: Vec<String> = check(&root, &[])
        .expect("lint ran")
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(found.len(), 1, "not exactly one finding: {found:?}");
    assert!(
        found[0].starts_with("crates/kernel/src/lib.rs:7: numeric constant `BARE`"),
        "{found:?}"
    );
}

negative_control!(
    lint_constants_passes_on_the_tree,
    "a tree with a bare numeric const must fail the clean check",
    expected = "numeric constant(s) not read from the register",
    check_clean(&tree("lint_control_bare", "static LIMIT: u16 = 65535;\n"))
);

negative_control!(
    lint_constants_bare_const_in_kernel_fails_naming_file_and_line,
    "comments, strings, const fn and register reads are not findings, so the finding check must fail on them",
    expected = "not exactly one finding",
    {
        let root = tree("lint_control_clean", KERNEL);
        let found = check(&root, &[]).expect("lint ran");
        assert_eq!(found.len(), 1, "not exactly one finding: {found:?}");
    }
);

/// A tree with a bare numeric `const` in `crates/kernel/src/payload/generated.rs`, the generator's kernel output.
fn generated_tree(name: &str) -> PathBuf {
    let root = tree(name, KERNEL);
    fs::create_dir_all(root.join("crates/kernel/src/payload")).expect("payload dir created");
    fs::write(
        root.join("crates/kernel/src/payload/generated.rs"),
        "pub const EMITTED: u32 = 0x7c00;\n",
    )
    .expect("generated.rs written");
    root
}

#[test]
fn lint_constants_exempts_the_generated_files() {
    let exempted = exempt().expect("exemptions listed");
    assert!(
        exempted.contains(&PathBuf::from("crates/kernel/src/payload/generated.rs")),
        "the kernel's generated file is not exempt: {exempted:?}"
    );
    check_clean(&generated_tree("lint_generated"));
}

negative_control!(
    lint_constants_exempts_the_generated_files,
    "the same generated file checked with only the register exempt must be a finding",
    expected = "numeric constant(s) not read from the register",
    {
        let root = generated_tree("lint_control_generated");
        let found =
            check(&root, &[PathBuf::from(xtask::lint_constants::REGISTER)]).expect("lint ran");
        assert!(
            found.is_empty(),
            "numeric constant(s) not read from the register: {found:?}"
        );
    }
);

/// A kernel whose `lib.rs` holds a test module inline, with a numeric `const` and an out-of-line module of its own
/// (`src/tests/deep.rs`), and declares another out of line (`src/probe.rs`, with `src/probe/inner.rs` beneath it), each
/// holding a numeric `const` or `static`, under `attribute`; beside them a module that is not test code, `src/real.rs`,
/// with one numeric `const`.
fn test_tree(name: &str, attribute: &str) -> PathBuf {
    let root = tree(
        name,
        &format!(
            "pub mod real;\n{attribute}\nmod tests {{\n    const N: usize = 3;\n    mod deep;\n}}\n{attribute}\n\
             #[allow(dead_code)]\nmod probe;\n"
        ),
    );
    let src = root.join("crates/kernel/src");
    fs::create_dir_all(src.join("tests")).expect("tests dir created");
    fs::create_dir_all(src.join("probe")).expect("probe dir created");
    fs::write(src.join("real.rs"), "pub const REAL: u32 = 5;\n").expect("real.rs written");
    fs::write(src.join("tests/deep.rs"), "static Z: u8 = 1;\n").expect("deep.rs written");
    fs::write(src.join("probe.rs"), "mod inner;\nconst M: u32 = 4;\n").expect("probe.rs written");
    fs::write(src.join("probe/inner.rs"), "const I: f64 = 0.5;\n").expect("inner.rs written");
    root
}

/// Checks that the lint over `root` finds exactly `want`, as `file:line: name`.
fn check_finds(root: &Path, want: &[&str]) {
    let found: Vec<String> = check(root, &[])
        .expect("lint ran")
        .iter()
        .map(|f| format!("{}:{}: {}", f.path.display(), f.line, f.name))
        .collect();
    assert_eq!(found, want, "the lint's findings are not the expected ones");
}

#[test]
fn lint_constants_skips_test_code() {
    check_finds(
        &test_tree("lint_test_code", "#[cfg(test)]"),
        &["crates/kernel/src/real.rs:1: REAL"],
    );
}

negative_control!(
    lint_constants_skips_test_code,
    "the same modules under another attribute are not test code, so their numbers must be findings",
    expected = "the lint's findings are not the expected ones",
    check_finds(
        &test_tree("lint_control_test_code", "#[cfg(feature = \"x\")]"),
        &["crates/kernel/src/real.rs:1: REAL"]
    )
);
