//! A fixture's lease waits for a target directory that built it, rather than building it anew in another, while a
//! test on another thread or in another process holds that directory (REQ-VAL-165: "a warm second run of a fixture
//! test rebuilds nothing"; R-231).
//!
//! The test registers a negative control (R-176).

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

#[path = "support/own_target.rs"]
mod own_target;
use own_target::{fixture_files, Lease, FIXTURES};

/// How long the first lease is held after the second is asked for.
const HOLD: Duration = Duration::from_millis(300);

/// In a fresh pool of this test's own, a lease for copy `x` builds it and is held on one thread while another thread
/// takes a second lease by `take`: the second must come to the directory that built `x`, once the first is dropped.
fn check_waits_for_its_own(pool: &str, take: fn(&str) -> Lease) {
    assert_ne!(pool, FIXTURES, "the test must not disturb the shared pool");
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(pool);
    let _ = std::fs::remove_dir_all(&root);
    let source = root.with_extension("source");
    let _ = std::fs::remove_dir_all(&source);
    std::fs::create_dir_all(source.join("src")).unwrap();
    std::fs::write(source.join("src/lib.rs"), "pub fn g() {}\n").unwrap();
    let first = Lease::take(pool, Some("x"));
    first.copy("x", &fixture_files(&source));
    let built = first.dir().to_path_buf();
    let (tx, rx) = mpsc::channel();
    let pool_name = pool.to_owned();
    let second = std::thread::spawn(move || {
        tx.send(()).unwrap();
        take(&pool_name).dir().to_path_buf()
    });
    rx.recv().unwrap();
    std::thread::sleep(HOLD);
    drop(first);
    let again = second.join().unwrap();
    assert_eq!(
        again, built,
        "a lease for x did not wait for the directory that built it"
    );
}

#[test]
fn own_target_lease_waits_for_the_directory_that_built_its_copy() {
    check_waits_for_its_own("own_target-wait", |pool| Lease::take(pool, Some("x")));
}

validation::negative_control!(
    own_target_lease_waits_for_the_directory_that_built_its_copy,
    "a lease that names no copy, required to wait for the directory that built x",
    expected = "a lease for x did not wait for the directory that built it",
    check_waits_for_its_own("own_target-wait-control", |pool| Lease::take(pool, None))
);
