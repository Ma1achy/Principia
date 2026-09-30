//! QA tests for TASK-M0-33, written from REQ-VAL-165 and R-231: "fixture workspaces must be written only when their
//! content changes, each with its own target directory kept across runs"; verify: "a warm second run of a fixture test
//! rebuilds nothing". A fixture copied into the shared pool of target directories (`support/own_target.rs`,
//! REQ-VAL-164) keeps its build only if its lease comes back to the directory that built it.
//!
//! Each test registers a negative control (R-176, R-212).

use std::path::Path;

use validation::negative_control;

#[path = "support/own_target.rs"]
mod own_target;
use own_target::{fixture_files, Lease, FIXTURES};

/// In a pool of this test's own, a copy made in the second of two leases held at once: a later lease for that copy
/// comes back to the directory holding it (so its build is warm), taken by `take`.
fn check_lease_returns(pool: &str, take: fn(&str) -> Lease) {
    assert_ne!(pool, FIXTURES, "the test must not disturb the shared pool");
    let _ = std::fs::remove_dir_all(Path::new(env!("CARGO_TARGET_TMPDIR")).join(pool));
    let source = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_33")
        .join(format!("{pool}-source"));
    let _ = std::fs::remove_dir_all(&source);
    std::fs::create_dir_all(source.join("src")).unwrap();
    std::fs::write(source.join("src/lib.rs"), "pub fn g() {}\n").unwrap();
    let files = fixture_files(&source);
    let first = Lease::take(pool, None);
    let second = Lease::take(pool, None);
    assert_ne!(
        first.dir(),
        second.dir(),
        "two leases held at once share a directory"
    );
    let copy = second.copy("qa_m0_33_copy", &files);
    assert!(copy.join("src/lib.rs").exists(), "the copy was not written");
    let built = second.dir().to_path_buf();
    drop(first);
    drop(second);
    let again = take(pool);
    assert_eq!(
        again.dir(),
        built,
        "the lease for a copy did not come back to the directory that built it"
    );
}

#[test]
fn qa_m0_33_a_lease_returns_to_the_directory_holding_its_copy() {
    check_lease_returns("qa_m0_33-lease", |pool| {
        Lease::take(pool, Some("qa_m0_33_copy"))
    });
}

negative_control!(
    qa_m0_33_a_lease_returns_to_the_directory_holding_its_copy,
    "a lease taken without naming the copy, required to come back to the directory holding it",
    expected = "the lease for a copy did not come back to the directory that built it",
    check_lease_returns("qa_m0_33-lease-control", |pool| Lease::take(pool, None))
);

/// A fresh pool named `pool` and the files of a one-file source to copy into its leases.
fn fresh_pool(pool: &str) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    assert_ne!(pool, FIXTURES, "the test must not disturb the shared pool");
    let _ = std::fs::remove_dir_all(Path::new(env!("CARGO_TARGET_TMPDIR")).join(pool));
    let source = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m0_33")
        .join(format!("{pool}-source"));
    let _ = std::fs::remove_dir_all(&source);
    std::fs::create_dir_all(source.join("src")).unwrap();
    std::fs::write(source.join("src/lib.rs"), "pub fn g() {}\n").unwrap();
    fixture_files(&source)
}

/// Two runs of a suite in a pool of this test's own. In the first, two tests build the fixture `x` at once, each in
/// the target directory its lease gives. In the second, a test of another fixture, `y`, holds a lease, and `warm`
/// tests of `x` take leases at once: each lands in a directory that built `x` in the first run, so its build is warm
/// ("a warm second run of a fixture test rebuilds nothing"; "each with its own target directory kept across runs").
fn check_warm_run_keeps_its_directories(pool: &str, warm: usize) {
    let files = fresh_pool(pool);
    // The first run: two tests of `x` at once.
    let cold: Vec<Lease> = (0..2).map(|_| Lease::take(pool, Some("x"))).collect();
    for lease in &cold {
        lease.copy("x", &files);
    }
    let built: Vec<std::path::PathBuf> = cold.iter().map(|l| l.dir().to_path_buf()).collect();
    drop(cold);
    // The second run: a test of `y` starts first, then the tests of `x`.
    let other = Lease::take(pool, Some("y"));
    other.copy("y", &files);
    let again: Vec<Lease> = (0..warm).map(|_| Lease::take(pool, Some("x"))).collect();
    for lease in &again {
        assert!(
            built.iter().any(|dir| dir == lease.dir()),
            "a warm run's lease for fixture x landed in {}, a target directory that never built it (built in {built:?})",
            lease.dir().display()
        );
    }
    drop(other);
}

#[test]
fn qa_m0_33_a_warm_run_builds_each_fixture_where_it_was_built() {
    check_warm_run_keeps_its_directories("qa_m0_33-warm-run", 2);
}

negative_control!(
    qa_m0_33_a_warm_run_builds_each_fixture_where_it_was_built,
    "three tests of x at once in the warm run, where only two built it, required to all land where x was built",
    expected = "a target directory that never built it",
    check_warm_run_keeps_its_directories("qa_m0_33-warm-run-control", 3)
);
