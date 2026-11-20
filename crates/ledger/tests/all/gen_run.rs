//! The generator driver's write step (dd_generation_root §1): `gen::run` writes each emitted file under the root and
//! returns the paths written; a file whose content matches the file on disk is left untouched and reported unchanged
//! (R-284).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use ledger::gen::{self, Emitter, Generated};
use ledger::schema::{Entry, Ledger, Word};
use validation::negative_control;

fn stub(_: &[Word], _: &[Entry]) -> Vec<Generated> {
    vec![Generated {
        path: PathBuf::from("gen/stub.txt"),
        contents: "stub\n".to_owned(),
    }]
}

/// `run` with `emitters` under a fresh root named `case` writes `gen/stub.txt`, with the stub's contents.
fn check_run_writes_the_stub(case: &str, emitters: &[Emitter]) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(case);
    let _ = std::fs::remove_dir_all(&root);
    let outcome = gen::run(&Ledger::default(), emitters, &root).expect("run failed");
    assert_eq!(
        outcome.written,
        [PathBuf::from("gen/stub.txt")],
        "paths written"
    );
    let contents =
        std::fs::read_to_string(root.join("gen/stub.txt")).expect("stub file not written");
    assert_eq!(contents, "stub\n", "stub file contents");
}

#[test]
fn gen_run_writes_each_emitted_file_under_the_root() {
    check_run_writes_the_stub("gen_run", &[stub]);
}

negative_control!(
    gen_run_writes_each_emitted_file_under_the_root,
    "with no emitter nothing is written, so the paths check must fail",
    expected = "paths written",
    check_run_writes_the_stub("gen_run_control", &[])
);

/// The modification time the stub file is set back to before the second run, so a rewrite shows however coarse the
/// filesystem's clock.
const PAST: Duration = Duration::from_secs(1_000_000_000);

/// Runs `run` with the stub emitter under a fresh root named `case`, replaces the stub file with `on_disk` and ages it,
/// runs again, and asserts the file is `rewritten` or not: its report, its modification time, and that it then holds
/// the stub's contents.
fn check_second_run(case: &str, on_disk: &str, rewritten: bool) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(case);
    let _ = std::fs::remove_dir_all(&root);
    gen::run(&Ledger::default(), &[stub], &root).expect("first run failed");
    let file = root.join("gen/stub.txt");
    std::fs::write(&file, on_disk).expect("replace the stub file");
    std::fs::File::options()
        .write(true)
        .open(&file)
        .and_then(|f| f.set_modified(SystemTime::UNIX_EPOCH + PAST))
        .expect("age the stub file");
    let outcome = gen::run(&Ledger::default(), &[stub], &root).expect("second run failed");
    let (written, unchanged) = if rewritten {
        (vec![PathBuf::from("gen/stub.txt")], vec![])
    } else {
        (vec![], vec![PathBuf::from("gen/stub.txt")])
    };
    assert_eq!(
        (outcome.written, outcome.unchanged),
        (written, unchanged),
        "second run's report (written, unchanged)"
    );
    let moved = std::fs::metadata(&file)
        .and_then(|m| m.modified())
        .expect("stub file's modification time")
        != SystemTime::UNIX_EPOCH + PAST;
    assert_eq!(moved, rewritten, "stub file's modification time moved");
    let contents = std::fs::read_to_string(&file).expect("stub file");
    assert_eq!(
        contents, "stub\n",
        "stub file contents after the second run"
    );
}

#[test]
fn gen_run_unchanged_file_is_left_untouched() {
    check_second_run("gen_run_unchanged", "stub\n", false);
}

negative_control!(
    gen_run_unchanged_file_is_left_untouched,
    "the file on disk differs from the stub's contents, so the driver must rewrite it",
    expected = "second run's report (written, unchanged)",
    check_second_run("gen_run_unchanged_control", "stub edited\n", false)
);

#[test]
fn gen_run_changed_file_is_rewritten() {
    check_second_run("gen_run_changed", "stub edited\n", true);
}

negative_control!(
    gen_run_changed_file_is_rewritten,
    "the file on disk already holds the stub's contents, so the driver must leave it",
    expected = "second run's report (written, unchanged)",
    check_second_run("gen_run_changed_control", "stub\n", true)
);
