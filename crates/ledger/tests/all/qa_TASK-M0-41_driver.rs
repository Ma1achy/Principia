//! qa tests for TASK-M0-41 at the generator driver (`ledger::gen::run`), where the compare-and-skip of REQ-TOOL-138
//! lives: a generated file is written only when its content differs from the file on disk (R-284). "Differs" means any
//! difference in bytes, so every non-identical on-disk file (same length, prefix, extra byte, empty, not UTF-8, absent
//! under a directory that does not yet exist) is rewritten, and an identical one keeps its modification time. A second
//! file alongside, identical on disk, must be left untouched either way.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use ledger::gen::{self, Generated};
use ledger::schema::{Entry, Ledger, Word};
use validation::negative_control;

const TARGET: &str = "qa/deep/nested/target.txt";
const TARGET_CONTENTS: &str = "generated target\n";
const OTHER: &str = "qa/other.txt";
const OTHER_CONTENTS: &str = "generated other\n";

/// The modification time files are set back to, so any rewrite shows however coarse the filesystem clock is.
const PAST: Duration = Duration::from_secs(1_000_000_000);

fn two_files(_: &[Word], _: &[Entry]) -> Vec<Generated> {
    vec![
        Generated {
            path: PathBuf::from(TARGET),
            contents: TARGET_CONTENTS.to_owned(),
        },
        Generated {
            path: PathBuf::from(OTHER),
            contents: OTHER_CONTENTS.to_owned(),
        },
    ]
}

fn age(path: &Path) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .and_then(|f| f.set_modified(SystemTime::UNIX_EPOCH + PAST))
        .expect("age the file");
}

fn moved(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .expect("modification time")
        != SystemTime::UNIX_EPOCH + PAST
}

/// Under a fresh root named `case`: `other` already holds its generated content (aged); the target is `on_disk`
/// (aged), or absent with its directory absent when `None`. After one driver run, the target must be reported
/// `written` exactly when `rewritten`, its mtime must move exactly then, and it must hold the generated content; the
/// other file must be reported unchanged, keep its mtime and its content.
fn check_driver(case: &str, on_disk: Option<&[u8]>, rewritten: bool) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(case);
    let _ = std::fs::remove_dir_all(&root);
    let target = root.join(TARGET);
    let other = root.join(OTHER);
    std::fs::create_dir_all(other.parent().unwrap()).expect("create the root");
    std::fs::write(&other, OTHER_CONTENTS).expect("write the other file");
    age(&other);
    if let Some(bytes) = on_disk {
        std::fs::create_dir_all(target.parent().unwrap()).expect("create the target's directory");
        std::fs::write(&target, bytes).expect("write the target");
        age(&target);
    }

    let outcome = gen::run(&Ledger::default(), &[two_files], &root).expect("the driver refused");

    let (written, unchanged) = if rewritten {
        (vec![PathBuf::from(TARGET)], vec![PathBuf::from(OTHER)])
    } else {
        (vec![], vec![PathBuf::from(TARGET), PathBuf::from(OTHER)])
    };
    let mut reported_unchanged = outcome.unchanged.clone();
    reported_unchanged.sort();
    let mut expected_unchanged = unchanged;
    expected_unchanged.sort();
    assert_eq!(
        (outcome.written, reported_unchanged),
        (written, expected_unchanged),
        "report (written, unchanged)"
    );
    assert_eq!(
        moved(&target),
        rewritten,
        "target's modification time moved"
    );
    assert_eq!(
        std::fs::read(&target).expect("target after the run"),
        TARGET_CONTENTS.as_bytes(),
        "target's content after the run"
    );
    assert!(
        !moved(&other),
        "the unchanged file's modification time moved"
    );
    assert_eq!(
        std::fs::read_to_string(&other).expect("other after the run"),
        OTHER_CONTENTS,
        "the unchanged file's content"
    );
}

#[test]
fn qa_driver_identical_file_is_left_untouched() {
    check_driver("qa41_identical", Some(TARGET_CONTENTS.as_bytes()), false);
}

negative_control!(
    qa_driver_identical_file_is_left_untouched,
    "a one-byte edit on disk must be rewritten, so expecting it untouched fails",
    expected = "report (written, unchanged)",
    check_driver("qa41_identical_control", Some(b"generated targeT\n"), false)
);

#[test]
fn qa_driver_same_length_edit_is_rewritten() {
    check_driver("qa41_same_length", Some(b"generated targeT\n"), true);
}

negative_control!(
    qa_driver_same_length_edit_is_rewritten,
    "an identical file must be left, so expecting it rewritten fails",
    expected = "report (written, unchanged)",
    check_driver(
        "qa41_same_length_control",
        Some(TARGET_CONTENTS.as_bytes()),
        true
    )
);

#[test]
fn qa_driver_prefix_is_rewritten() {
    check_driver("qa41_prefix", Some(b"generated tar"), true);
}

negative_control!(
    qa_driver_prefix_is_rewritten,
    "an identical file must be left, so expecting it rewritten fails",
    expected = "report (written, unchanged)",
    check_driver(
        "qa41_prefix_control",
        Some(TARGET_CONTENTS.as_bytes()),
        true
    )
);

#[test]
fn qa_driver_extra_byte_is_rewritten() {
    check_driver("qa41_extra", Some(b"generated target\n\n"), true);
}

negative_control!(
    qa_driver_extra_byte_is_rewritten,
    "an identical file must be left, so expecting it rewritten fails",
    expected = "report (written, unchanged)",
    check_driver("qa41_extra_control", Some(TARGET_CONTENTS.as_bytes()), true)
);

#[test]
fn qa_driver_empty_is_rewritten() {
    check_driver("qa41_empty", Some(b""), true);
}

negative_control!(
    qa_driver_empty_is_rewritten,
    "an identical file must be left, so expecting it rewritten fails",
    expected = "report (written, unchanged)",
    check_driver("qa41_empty_control", Some(TARGET_CONTENTS.as_bytes()), true)
);

#[test]
fn qa_driver_not_utf8_is_rewritten() {
    check_driver("qa41_not_utf8", Some(b"generated \xff\xfe target\n"), true);
}

negative_control!(
    qa_driver_not_utf8_is_rewritten,
    "an identical file must be left, so expecting it rewritten fails",
    expected = "report (written, unchanged)",
    check_driver(
        "qa41_not_utf8_control",
        Some(TARGET_CONTENTS.as_bytes()),
        true
    )
);

#[test]
fn qa_driver_absent_file_in_absent_directory_is_written() {
    check_driver("qa41_absent", None, true);
}

negative_control!(
    qa_driver_absent_file_in_absent_directory_is_written,
    "an identical file must be left, so expecting it written fails",
    expected = "report (written, unchanged)",
    check_driver(
        "qa41_absent_control",
        Some(TARGET_CONTENTS.as_bytes()),
        true
    )
);
