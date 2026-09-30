//! One build directory per fixture type, shared by that type's copies and held by one test at a time
//! (`support/own_target.rs`; REQ-VAL-164, REQ-VAL-165; R-270): a copy of a type builds in its type's directory whichever
//! test or thread asks for it, so a warm run of the type rebuilds nothing; a second test of the type waits while the
//! first holds it, so no two cargo runs share it at once; and a thread already holding it takes it again at once,
//! rather than waiting for itself.
//!
//! Each test registers a negative control (R-176).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

#[path = "support/own_target.rs"]
mod own_target;
use own_target::{fixture_files, Lease, FIXTURES};

/// How long the first lease is held after the second is asked for.
const HOLD: Duration = Duration::from_millis(300);

/// How long a thread taking a type it holds may take before it is judged to be waiting for itself.
const SELF_WAIT: Duration = Duration::from_secs(2);

/// A fresh pool of the test's own, named `pool`, and the files of a one-file source to copy.
fn fresh_pool(pool: &str) -> Vec<(PathBuf, Vec<u8>)> {
    assert_ne!(pool, FIXTURES, "the test must not disturb the shared pool");
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(pool);
    let _ = std::fs::remove_dir_all(&root);
    let source = root.with_extension("source");
    let _ = std::fs::remove_dir_all(&source);
    std::fs::create_dir_all(source.join("src")).unwrap();
    std::fs::write(source.join("src/lib.rs"), "pub fn g() {}\n").unwrap();
    fixture_files(&source)
}

/// Copy `a` of type `x`, made on one thread, then copy `b`, made on another thread in the lease `second` gives: both
/// must be in `x`'s one directory.
fn check_copies_share_their_type(pool: &str, second: fn(&str) -> Lease) {
    let files = fresh_pool(pool);
    let pool_name = pool.to_owned();
    let first = std::thread::spawn(move || {
        let lease = Lease::take(&pool_name, Some("x"));
        lease.copy("a", &files);
        (lease.dir().to_path_buf(), files)
    });
    let (built, files) = first.join().unwrap();
    let pool_name = pool.to_owned();
    let again = std::thread::spawn(move || {
        let lease = second(&pool_name);
        lease.copy("b", &files);
        lease.dir().to_path_buf()
    })
    .join()
    .unwrap();
    assert_eq!(
        again, built,
        "a second copy of type x built outside x's directory"
    );
}

#[test]
fn own_target_every_copy_of_a_type_builds_in_its_one_directory() {
    check_copies_share_their_type("own_target-share", |pool| Lease::take(pool, Some("x")));
}

validation::negative_control!(
    own_target_every_copy_of_a_type_builds_in_its_one_directory,
    "the second copy leased as another type, required to build in x's directory",
    expected = "a second copy of type x built outside x's directory",
    check_copies_share_their_type("own_target-share-control", |pool| Lease::take(
        pool,
        Some("y")
    ))
);

/// A lease of type `x` held on this thread while another thread takes a lease by `second`: that one must be granted
/// only once the first is dropped, in the same directory.
fn check_one_holder_at_a_time(pool: &str, second: fn(&str) -> Lease) {
    fresh_pool(pool);
    let first = Lease::take(pool, Some("x"));
    let held = first.dir().to_path_buf();
    let released = Arc::new(AtomicBool::new(false));
    let (asked, asking) = mpsc::channel();
    let pool_name = pool.to_owned();
    let seen = Arc::clone(&released);
    let waiter = std::thread::spawn(move || {
        asked.send(()).unwrap();
        let lease = second(&pool_name);
        (seen.load(Ordering::SeqCst), lease.dir().to_path_buf())
    });
    asking.recv().unwrap();
    std::thread::sleep(HOLD);
    released.store(true, Ordering::SeqCst);
    drop(first);
    let (after, dir) = waiter.join().unwrap();
    assert!(after, "two tests held a fixture type's directory at once");
    assert_eq!(
        dir, held,
        "the waiting lease of type x came to another directory"
    );
}

#[test]
fn own_target_a_type_is_held_by_one_test_at_a_time() {
    check_one_holder_at_a_time("own_target-wait", |pool| Lease::take(pool, Some("x")));
}

validation::negative_control!(
    own_target_a_type_is_held_by_one_test_at_a_time,
    "the second lease taken as a directory of no type, required to wait for the holder of x",
    expected = "two tests held a fixture type's directory at once",
    check_one_holder_at_a_time("own_target-wait-control", |pool| Lease::take(pool, None))
);

/// On a thread holding a lease of type `x`, `again` takes `x` once more: it must come at once, in the same directory,
/// not wait for the lease the thread itself holds.
fn check_holder_takes_it_again(pool: &str, again: fn(&str) -> PathBuf) {
    fresh_pool(pool);
    let (done, result) = mpsc::channel();
    let pool_name = pool.to_owned();
    std::thread::spawn(move || {
        let first = Lease::take(&pool_name, Some("x"));
        let second = again(&pool_name);
        let _ = done.send((first.dir().to_path_buf(), second));
    });
    let (first, second) = result
        .recv_timeout(SELF_WAIT)
        .expect("a thread holding type x waited for itself");
    assert_eq!(
        first, second,
        "the thread's second lease of x came to another directory"
    );
}

#[test]
fn own_target_a_thread_holding_a_type_takes_it_again_at_once() {
    check_holder_takes_it_again("own_target-again", |pool| {
        Lease::take(pool, Some("x")).dir().to_path_buf()
    });
}

validation::negative_control!(
    own_target_a_thread_holding_a_type_takes_it_again_at_once,
    "the second lease of x taken on another thread, which must wait for the first",
    expected = "a thread holding type x waited for itself",
    check_holder_takes_it_again("own_target-again-control", |pool| {
        let pool = pool.to_owned();
        std::thread::spawn(move || Lease::take(&pool, Some("x")).dir().to_path_buf())
            .join()
            .unwrap()
    })
);
