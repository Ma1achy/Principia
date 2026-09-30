//! QA tests for TASK-M0-33, written from REQ-VAL-165, R-231 and R-270: "fixture workspaces must be written only when
//! their content changes, with one build directory per fixture type shared by its copies, kept across runs";
//! verify: "a warm second run of a fixture test rebuilds nothing for its fixture type". A fixture copied into the
//! shared pool of target directories (`support/own_target.rs`, REQ-VAL-164) keeps its build only if every lease for
//! its type comes back to the type's one directory, whatever else is held.
//!
//! Each test registers a negative control (R-176, R-212).

use std::path::Path;

use validation::negative_control;

#[path = "support/own_target.rs"]
mod own_target;
use own_target::{fixture_files, Lease, FIXTURES};

/// In a pool of this test's own, a copy of fixture type `qa_m0_33_copy` made in a lease `take` gives, then dropped;
/// then, while a build of no fixture type holds a directory of the pool, a second lease `take` gives: it comes back to
/// the directory holding the copy (so its build is warm).
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
    let first = take(pool);
    let copy = first.copy("qa_m0_33_copy", &files);
    assert!(copy.join("src/lib.rs").exists(), "the copy was not written");
    let built = first.dir().to_path_buf();
    drop(first);
    let other = Lease::take(pool, None);
    let again = take(pool);
    assert_eq!(
        again.dir(),
        built,
        "the lease for a copy did not come back to the directory that built it"
    );
    drop(other);
}

#[test]
fn qa_m0_33_a_lease_returns_to_the_directory_holding_its_copy() {
    check_lease_returns("qa_m0_33-lease", |pool| {
        Lease::take(pool, Some("qa_m0_33_copy"))
    });
}

negative_control!(
    qa_m0_33_a_lease_returns_to_the_directory_holding_its_copy,
    "a lease taken without naming the copy's type, required to come back to the directory holding it",
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

/// Two runs of a suite in a pool of this test's own. In the first, tests of the copies `a` and `b` of fixture type
/// `x` each take the lease `take` gives, one after the other. In the second, a build of no fixture type holds a
/// directory of the pool, and the tests of `a` and `b` take leases again: each lands in a directory that built `x` in
/// the first run ("one build directory per fixture type shared by its copies, kept across runs"; "a warm second run
/// of a fixture test rebuilds nothing for its fixture type").
fn check_warm_run_keeps_its_directories(pool: &str, take: fn(&str) -> Lease) {
    let files = fresh_pool(pool);
    // The first run: the copies of `x`, one after the other.
    let mut built: Vec<std::path::PathBuf> = Vec::new();
    for copy in ["a", "b"] {
        let lease = take(pool);
        lease.copy(copy, &files);
        built.push(lease.dir().to_path_buf());
    }
    // The second run: a build of no fixture type starts first, then the tests of `x`'s copies.
    let other = Lease::take(pool, None);
    for _ in ["a", "b"] {
        let lease = take(pool);
        assert!(
            built.iter().any(|dir| dir == lease.dir()),
            "a warm run's lease for fixture x landed in {}, a target directory that never built it (built in \
             {built:?})",
            lease.dir().display()
        );
    }
    drop(other);
}

#[test]
fn qa_m0_33_a_warm_run_builds_each_fixture_where_it_was_built() {
    check_warm_run_keeps_its_directories("qa_m0_33-warm-run", |pool| Lease::take(pool, Some("x")));
}

negative_control!(
    qa_m0_33_a_warm_run_builds_each_fixture_where_it_was_built,
    "leases taken without naming the fixture type, required to land where x was built",
    expected = "a target directory that never built it",
    check_warm_run_keeps_its_directories("qa_m0_33-warm-run-control", |pool| Lease::take(
        pool, None
    ))
);
