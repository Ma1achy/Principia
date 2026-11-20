//! REQ-TOOL-138 (R-284): `cargo xtask codegen` writes a generated file only when its content differs from the file on
//! disk. A second run over an unchanged ledger leaves every generated file's modification time alone; changing one
//! ledger entry rewrites only the files it affects.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use ledger::gen::{Emitter, Generated};
use ledger::schema::{Entry, Ledger, Word};
use validation::negative_control;
use xtask::codegen::{self, Outcome};

/// The modification time every generated file is set back to between runs, so a rewrite shows however coarse the
/// filesystem's clock.
const PAST: Duration = Duration::from_secs(1_000_000_000);

/// A fresh, empty root under the test target's scratch directory.
fn fresh_root(case: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("codegen")
        .join(case);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create the root");
    root
}

fn run(ledger: &Ledger, emitters: &[Emitter], root: &Path) -> Outcome {
    codegen::generate(ledger::constants::REGISTER, ledger, emitters, root).expect("codegen refused")
}

/// Sets every file in `paths`, under `root`, back to [`PAST`].
fn age(root: &Path, paths: &[PathBuf]) {
    for path in paths {
        std::fs::File::options()
            .write(true)
            .open(root.join(path))
            .and_then(|f| f.set_modified(SystemTime::UNIX_EPOCH + PAST))
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}

/// The files in `paths`, under `root`, whose modification time is no longer [`PAST`].
fn touched(root: &Path, paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| {
            let modified = std::fs::metadata(root.join(path))
                .and_then(|m| m.modified())
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            modified != SystemTime::UNIX_EPOCH + PAST
        })
        .cloned()
        .collect()
}

/// One file per entry, `gen/<name>.txt`, holding the entry: changing an entry affects its file alone.
fn per_entry(_: &[Word], entries: &[Entry]) -> Vec<Generated> {
    entries
        .iter()
        .map(|e| Generated {
            path: PathBuf::from(format!("gen/{}.txt", e.name)),
            contents: format!("{e:?}\n"),
        })
        .collect()
}

/// As [`per_entry`], but every file also holds a ledger-wide count: changing one entry's tier gate affects them all.
#[cfg(feature = "controls")]
fn per_entry_with_count(words: &[Word], entries: &[Entry]) -> Vec<Generated> {
    let gated = entries.iter().filter(|e| e.tier_gate.is_some()).count();
    per_entry(words, entries)
        .into_iter()
        .map(|mut g| {
            g.contents
                .push_str(&format!("tier-gated entries: {gated}\n"));
            g
        })
        .collect()
}

/// Codegen over `ledger` with `emitters` under a fresh root named `case`; `between` then runs on the root, and a
/// second codegen must leave every generated file's modification time where the first left it.
fn check_second_run_keeps_every_mtime(
    case: &str,
    ledger: &Ledger,
    emitters: &[Emitter],
    between: fn(&Path, &[PathBuf]),
) {
    let root = fresh_root(case);
    let first = run(ledger, emitters, &root);
    assert!(
        !first.written.is_empty(),
        "the first run generated no file, so the check would see nothing"
    );
    age(&root, &first.written);
    between(&root, &first.written);
    let second = run(ledger, emitters, &root);
    let moved = touched(&root, &first.written);
    assert!(
        moved.is_empty(),
        "the second run moved the modification time of {moved:?}"
    );
    assert_eq!(
        second,
        Outcome {
            written: Vec::new(),
            unchanged: first.written
        },
        "the second run's report"
    );
}

fn nothing(_: &Path, _: &[PathBuf]) {}

/// Overwrites the first generated file with other content, as a hand edit would.
#[cfg(feature = "controls")]
fn edit_one(root: &Path, paths: &[PathBuf]) {
    std::fs::write(root.join(&paths[0]), "edited by hand\n").expect("edit the file");
    age(root, &paths[..1]);
}

#[test]
fn codegen_unchanged_second_run_keeps_every_mtime() {
    let emitters: Vec<Emitter> = [ledger::gen::EMITTERS, &[per_entry as Emitter]].concat();
    check_second_run_keeps_every_mtime("twice", &ledger::layout(), &emitters, nothing);
}

negative_control!(
    codegen_unchanged_second_run_keeps_every_mtime,
    "a generated file edited on disk between the runs must be rewritten, moving its modification time",
    expected = "the second run moved the modification time of",
    check_second_run_keeps_every_mtime(
        "twice_control",
        &ledger::layout(),
        &[ledger::gen::EMITTERS, &[per_entry as Emitter]].concat(),
        edit_one
    )
);

/// Codegen over the layout table with `emitter` under a fresh root named `case`; after its entry at `index` gains a
/// tier gate, a second codegen must rewrite that entry's file, `gen/<name>.txt`, alone.
fn check_one_entry_rewrites_only_its_file(case: &str, emitter: Emitter, index: usize) {
    let root = fresh_root(case);
    let mut ledger = ledger::layout();
    assert!(
        ledger.entries.len() > 1,
        "the layout table has too few entries to tell one file from the rest"
    );
    let first = run(&ledger, &[emitter], &root);
    age(&root, &first.written);
    let entry = &mut ledger.entries[index];
    assert_eq!(entry.tier_gate, None, "the entry already has a tier gate");
    entry.tier_gate = Some("codegen_unchanged");
    let affected = PathBuf::from(format!("gen/{}.txt", entry.name.expect("entry name")));
    let second = run(&ledger, &[emitter], &root);
    assert_eq!(
        touched(&root, &first.written),
        std::slice::from_ref(&affected),
        "the files whose modification time moved"
    );
    assert_eq!(second.written, [affected], "the files the second run wrote");
    assert_eq!(
        second.unchanged.len(),
        first.written.len() - 1,
        "the files the second run left unchanged"
    );
}

#[test]
fn codegen_unchanged_one_entry_change_rewrites_only_its_file() {
    check_one_entry_rewrites_only_its_file("one_entry", per_entry, 0);
}

negative_control!(
    codegen_unchanged_one_entry_change_rewrites_only_its_file,
    "an emitter whose every file depends on the changed entry must have them all rewritten",
    expected = "the files whose modification time moved",
    check_one_entry_rewrites_only_its_file("one_entry_control", per_entry_with_count, 0)
);
