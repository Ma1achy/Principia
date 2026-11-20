//! `cargo xtask ci --partition k/n` runs the `controls` runner as its own shard of the controls (R-360): shard k runs
//! slice k, not every control in shard 1 and none in the rest.
//!
//! `ci::run_partition` runs in this process on a `controls` runner whose own check only notes that it ran; the
//! sharded run goes to a stand-in `cargo` (`CARGO`, set for this whole test binary) that answers `metadata` and the
//! listing from canned text, answers a run as every control it names making its test fail, and logs each control it
//! is asked to run.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Once};

use serde_json::json;
use xtask::ci::{run_partition, Runner};
use xtask::controls::{shard_of, Partition};

#[path = "support/stand_in.rs"]
mod stand_in;

/// The canned listing: 40 tests, each with its control.
fn listing() -> String {
    (0..40)
        .map(|i| format!("m::t{i:02}: test\nm::t{i:02}::negative_control: test\n"))
        .collect()
}

/// The directory of the stand-in `cargo`: this process's own, keyed by its id, since nextest runs the test and its
/// control as two processes at once, and each rewrites the stand-in and its `ran.log` here, which [`SERIAL`] keeps
/// apart only within one process.
fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ci_partition-{}", std::process::id()))
}

/// Writes the stand-in `cargo` once and points `CARGO` at it, for every check in this binary.
fn stand_in() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let dir = dir();
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let cargo = stand_in::stand_in_cargo(
            &dir,
            json!({
                "metadata": stand_in::metadata("ci_partition_fake"),
                "listed": listing(),
                "ran": dir.join("ran.log"),
            }),
        );
        std::env::set_var("CARGO", &cargo);
    });
}

/// Whether the `controls` runner's own, unsharded check ran.
static UNSHARDED: AtomicBool = AtomicBool::new(false);

/// Held for each whole check, so the test and its control, in one process under `cargo test`, keep the log apart.
static SERIAL: Mutex<()> = Mutex::new(());

fn unsharded() -> Result<(), String> {
    UNSHARDED.store(true, Ordering::SeqCst);
    Ok(())
}

/// A lone `controls` runner, named as `cargo xtask ci`'s is.
const RUNNERS: &[Runner] = &[Runner {
    name: "controls",
    run: unsharded,
    list: unsharded,
}];

/// How a check runs the runners as shard `slice` would.
type RunShard = fn(&[Runner], Partition) -> Result<(), String>;

/// Runs `RUNNERS` by `run` as shard `slice`; returns the controls the stand-in ran, and whether the unsharded
/// check ran.
fn ran_in(run: RunShard, slice: Partition) -> (BTreeSet<String>, bool) {
    let _ = fs::remove_file(dir().join("ran.log"));
    UNSHARDED.store(false, Ordering::SeqCst);
    run(RUNNERS, slice).unwrap();
    let ran = fs::read_to_string(dir().join("ran.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    (ran, UNSHARDED.load(Ordering::SeqCst))
}

/// `ci --partition k/4`, run by `run`: each shard runs exactly its own slice of the controls, by name, and never the
/// unsharded controls check.
fn check_controls_sharded(run: RunShard) {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    stand_in();
    for k in 1..=4 {
        let slice = Partition { k, n: 4 };
        let (ran, whole) = ran_in(run, slice);
        let mine: BTreeSet<String> = (0..40)
            .map(|i| format!("m::t{i:02}::negative_control"))
            .filter(|c| shard_of(c, 4) == k)
            .collect();
        assert!(
            !whole,
            "shard {slice} ran the unsharded controls check, not its slice"
        );
        assert_eq!(
            ran, mine,
            "shard {slice} did not run exactly its slice of the controls"
        );
    }
}

#[test]
fn ci_partition_runs_each_shards_slice_of_the_controls() {
    check_controls_sharded(run_partition);
}

validation::negative_control!(
    ci_partition_runs_each_shards_slice_of_the_controls,
    "`controls` run as any other runner: unsharded in shard 1 and skipped in the rest",
    expected = "ran the unsharded controls check",
    check_controls_sharded(|runners, slice| {
        if slice.k == 1 {
            xtask::ci::run(runners)
        } else {
            Ok(())
        }
    })
);
