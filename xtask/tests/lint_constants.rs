//! `cargo xtask lint constants` (dd_generation_root §3.8; REQ-SYS-001, REQ-SYS-005): it passes on this tree, and a
//! bare numeric `const` in `crates/kernel` fails it, naming the file and line; comments, strings, `const fn` and
//! constants read from the register are not findings; the register and the generated files are exempt.

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
