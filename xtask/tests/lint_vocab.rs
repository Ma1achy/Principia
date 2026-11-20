//! `cargo xtask lint vocab` (canonical_spec §8; memory_tiers §1; R-111, R-259; REQ-SYS-002, REQ-SYS-003): it passes
//! on this tree; a fixture with each retired term, and with each identifier outside the locked taxonomy, fails naming
//! it, in code and in docs; the same term inside the `retired-terms` markers passes in docs and fails in code; an
//! unpaired marker fails; only `.md` and `.html` under `docs/` are read, not `docs/archive/` or `docs/reference/`, and
//! neither the term list nor the fixtures are read. No term appears in this file: the fixtures hold them.

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::lint_vocab::{check, Finding, Found, TERMS};

/// The lint's fixtures.
fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/vocab")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The fixture holding `TERMS[i]`.
fn term_fixture(i: usize) -> String {
    fixture(&format!("term_{i:02}.md"))
}

/// A fresh tree named `name`, holding each `(path, text)` of `files`.
fn tree(name: &str, files: &[(&str, String)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("lint_vocab")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("directory created");
        fs::write(&path, text).expect("fixture written");
    }
    root
}

fn findings(root: &Path) -> Vec<Finding> {
    check(root).expect("lint ran")
}

/// The lint finds nothing under `root`.
fn check_clean(root: &Path) {
    let found = findings(root);
    assert!(
        found.is_empty(),
        "retired term(s) found: {}",
        found
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    );
}

#[test]
fn lint_vocab_passes_on_the_tree() {
    check_clean(&Path::new(env!("CARGO_MANIFEST_DIR")).join(".."));
}

negative_control!(
    lint_vocab_passes_on_the_tree,
    "a tree with a retired term in a contract must fail the clean check",
    expected = "retired term(s) found",
    check_clean(&tree(
        "ctl_tree",
        &[("docs/contracts/c.md", term_fixture(0))]
    ))
);

/// Placed at `path`, the fixture of each term `i` fails naming `TERMS[pick(i)]`, on its line 2, and nothing else.
fn check_each(case: &str, path: &str, pick: fn(usize) -> usize) {
    let fixtures = fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vocab"))
        .expect("fixtures")
        .filter(|e| {
            e.as_ref()
                .is_ok_and(|e| e.file_name().to_string_lossy().starts_with("term_"))
        })
        .count();
    assert_eq!(fixtures, TERMS.len(), "not one fixture per term");
    for (i, term) in TERMS.iter().enumerate() {
        let root = tree(&format!("{case}_{i:02}"), &[(path, term_fixture(pick(i)))]);
        let found = findings(&root);
        let want = format!("{path}:2: `{}` (", term.text);
        assert!(
            found.len() == 1
                && found[0].found == Found::Term(term)
                && found[0].to_string().starts_with(&want),
            "fixture {i:02} does not fail naming `{}` alone at {path}:2: {found:?}",
            term.text
        );
    }
}

#[test]
fn lint_vocab_each_term_fails_naming_it_in_docs() {
    check_each("each_doc", "docs/design/d.md", |i| i);
}

negative_control!(
    lint_vocab_each_term_fails_naming_it_in_docs,
    "each term's check given the next term's fixture",
    expected = "does not fail naming",
    check_each("ctl_each_doc", "docs/design/d.md", |i| (i + 1)
        % TERMS.len())
);

#[test]
fn lint_vocab_each_term_fails_naming_it_in_code() {
    check_each("each_code", "crates/engine/src/lib.rs", |i| i);
}

negative_control!(
    lint_vocab_each_term_fails_naming_it_in_code,
    "each term's check given the next term's fixture",
    expected = "does not fail naming",
    check_each("ctl_each_code", "crates/engine/src/lib.rs", |i| (i + 1)
        % TERMS.len())
);

/// The fixture `name`, placed at `path`, yields exactly `want`, as (line, what) pairs.
fn check_yields(case: &str, name: &str, path: &str, want: &[(usize, Found)]) {
    let root = tree(case, &[(path, fixture(name))]);
    let found: Vec<(usize, Found)> = findings(&root).iter().map(|f| (f.line, f.found)).collect();
    assert_eq!(found, want, "{name} at {path} yields other findings");
}

/// The term the marker fixtures hold.
fn marked_term() -> Found {
    Found::Term(
        TERMS
            .iter()
            .find(|t| t.text.starts_with("Sim"))
            .expect("the term"),
    )
}

#[test]
fn lint_vocab_term_inside_markers_passes_in_docs() {
    check_yields("marked_doc", "marked.md", "docs/design/d.md", &[]);
}

negative_control!(
    lint_vocab_term_inside_markers_passes_in_docs,
    "the same term with its markers unpaired",
    expected = "yields other findings",
    check_yields("ctl_marked_doc", "unclosed.md", "docs/design/d.md", &[])
);

#[test]
fn lint_vocab_markers_exempt_nothing_in_code() {
    check_yields(
        "marked_code",
        "marked.md",
        "crates/engine/src/lib.rs",
        &[(4, marked_term())],
    );
}

negative_control!(
    lint_vocab_markers_exempt_nothing_in_code,
    "the marked passage in docs, where the markers exempt it",
    expected = "yields other findings",
    check_yields(
        "ctl_marked_code",
        "marked.md",
        "docs/design/d.md",
        &[(4, marked_term())]
    )
);

#[test]
fn lint_vocab_unpaired_marker_fails() {
    check_yields(
        "unclosed",
        "unclosed.md",
        "docs/design/d.md",
        &[(3, Found::Unclosed), (4, marked_term())],
    );
    check_yields(
        "unopened",
        "unopened.md",
        "docs/design/d.md",
        &[(3, marked_term()), (4, Found::Unopened)],
    );
}

negative_control!(
    lint_vocab_unpaired_marker_fails,
    "paired markers, required to fail as unpaired",
    expected = "yields other findings",
    check_yields(
        "ctl_unpaired",
        "marked.md",
        "docs/design/d.md",
        &[(3, Found::Unclosed), (4, marked_term())]
    )
);

#[test]
fn lint_vocab_near_misses_pass() {
    check_yields("near_doc", "near_misses.md", "docs/design/d.md", &[]);
    check_yields(
        "near_code",
        "near_misses.md",
        "crates/engine/src/lib.rs",
        &[],
    );
}

negative_control!(
    lint_vocab_near_misses_pass,
    "a fixture holding a term, required to pass as a near miss",
    expected = "yields other findings",
    check_yields("ctl_near", "term_04.md", "crates/engine/src/lib.rs", &[])
);

/// With a term placed at each of `placed`, the lint reads exactly `read`, in order.
fn check_reads(case: &str, placed: &[&str], read: &[&str]) {
    let files: Vec<(&str, String)> = placed.iter().map(|p| (*p, term_fixture(0))).collect();
    let root = tree(case, &files);
    let found: Vec<PathBuf> = findings(&root).into_iter().map(|f| f.path).collect();
    let read: Vec<PathBuf> = read.iter().map(PathBuf::from).collect();
    assert_eq!(
        found, read,
        "the lint does not read exactly the files expected"
    );
}

/// A term in each of these: only the `.md` and `.html` files outside `docs/archive/` and `docs/reference/`, and every
/// file of the code directories but the term list and the fixtures, are read.
const PLACED: &[&str] = &[
    "crates/engine/src/lib.rs",
    "crates/gui/assets/notes.txt",
    "docs/archive/old.md",
    "docs/contracts/c.md",
    "docs/experiments/run.py",
    "docs/experiments/run.json",
    "docs/gui/reference/mock.html",
    "docs/reference/prin-rs/lib.rs",
    "docs/reference/ref.md",
    "fixtures/case.toml",
    "web/src/main.ts",
    "xtask/src/lint_vocab.rs",
    "xtask/src/other.rs",
    "xtask/tests/fixtures/vocab/term.md",
];

#[test]
fn lint_vocab_reads_only_the_scanned_files() {
    check_reads(
        "reads",
        PLACED,
        &[
            "crates/engine/src/lib.rs",
            "crates/gui/assets/notes.txt",
            "docs/contracts/c.md",
            "docs/gui/reference/mock.html",
            "fixtures/case.toml",
            "web/src/main.ts",
            "xtask/src/other.rs",
        ],
    );
}

negative_control!(
    lint_vocab_reads_only_the_scanned_files,
    "the excluded files, required to be read",
    expected = "the lint does not read exactly the files expected",
    check_reads(
        "ctl_reads",
        PLACED,
        &[
            "docs/archive/old.md",
            "docs/experiments/run.py",
            "docs/reference/ref.md",
            "xtask/src/lint_vocab.rs",
            "xtask/tests/fixtures/vocab/term.md",
        ]
    )
);
