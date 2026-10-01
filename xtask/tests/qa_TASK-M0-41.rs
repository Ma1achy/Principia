//! QA tests for TASK-M0-41, written from REQ-TOOL-138 (R-284): "`cargo xtask codegen` must write a generated file only
//! when its content differs from what is on disk, so an unchanged file keeps its modification time and forces no
//! rebuild." Verify detail: running codegen twice leaves every generated file's mtime unchanged on the second run;
//! changing one ledger entry rewrites only the files it affects.
//!
//! The tests go through the command's entry point (`xtask::codegen::run`, with the real emitters and layout table) and
//! through `xtask::codegen::generate`. Which files a ledger change "affects" is computed independently of the writer,
//! from `ledger::gen::generate`'s output before and after the change. Each test has a negative control (R-176).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use ledger::gen::{Emitter, Generated};
use ledger::schema::{Entry, Ledger, Word};
use validation::negative_control;

/// The modification time files are set back to before a run, so any write shows whatever the filesystem's clock
/// resolution.
fn past() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000)
}

/// A fresh workspace root holding only a `Cargo.toml`; returns (root, manifest).
fn workspace(case: &str) -> (PathBuf, PathBuf) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m041_{case}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("root created");
    let manifest = root.join("Cargo.toml");
    fs::write(&manifest, "[workspace]\n").expect("Cargo.toml written");
    (root, manifest)
}

/// Every file under `root` except the manifest, relative to `root`.
fn files_under(root: &Path) -> BTreeSet<PathBuf> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeSet<PathBuf>) {
        for entry in fs::read_dir(dir).expect("read dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if path != root.join("Cargo.toml") {
                out.insert(path.strip_prefix(root).expect("under root").to_path_buf());
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(root, root, &mut out);
    out
}

fn age(root: &Path, paths: &BTreeSet<PathBuf>) {
    for path in paths {
        fs::File::options()
            .write(true)
            .open(root.join(path))
            .and_then(|f| f.set_modified(past()))
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}

/// The files in `paths` whose modification time is no longer [`past`].
fn moved(root: &Path, paths: &BTreeSet<PathBuf>) -> BTreeSet<PathBuf> {
    paths
        .iter()
        .filter(|p| {
            fs::metadata(root.join(p))
                .and_then(|m| m.modified())
                .expect("mtime")
                != past()
        })
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------------------------------------------
// 1. Through the command's entry point: a second `cargo xtask codegen` touches no generated file.

/// Runs `xtask::codegen::run` on a fresh workspace, ages every file it generated, applies `between`, runs it again,
/// and asserts no generated file's modification time moved.
fn check_run_twice_keeps_mtimes(case: &str, between: fn(&Path)) {
    let (root, manifest) = workspace(case);
    xtask::codegen::run(&manifest).expect("first codegen run refused");
    let generated = files_under(&root);
    assert!(
        generated.contains(Path::new(ledger::gen::rust::PATH)),
        "the first run did not generate {} (got {generated:?}), so the check would see nothing",
        ledger::gen::rust::PATH
    );
    age(&root, &generated);
    between(&root);
    xtask::codegen::run(&manifest).expect("second codegen run refused");
    let touched = moved(&root, &generated);
    assert!(
        touched.is_empty(),
        "the second codegen run moved the modification time of {touched:?}"
    );
}

fn no_change(_: &Path) {}

/// A hand edit of the generated file between the runs: the second run must restore it, so its mtime moves.
#[cfg(feature = "controls")]
fn hand_edit_generated_rs(root: &Path) {
    let path = root.join(ledger::gen::rust::PATH);
    let mut text = fs::read_to_string(&path).expect("read generated.rs");
    text.push_str("// hand edit\n");
    fs::write(&path, text).expect("edit generated.rs");
    fs::File::options()
        .write(true)
        .open(&path)
        .and_then(|f| f.set_modified(past()))
        .expect("age");
}

#[test]
fn qa_codegen_unchanged_run_twice_keeps_every_mtime() {
    check_run_twice_keeps_mtimes("run_twice", no_change);
}

negative_control!(
    qa_codegen_unchanged_run_twice_keeps_every_mtime,
    "generated.rs hand-edited between the runs differs from the generated content, so it is rewritten and its mtime moves",
    expected = "the second codegen run moved the modification time of",
    check_run_twice_keeps_mtimes("run_twice_control", hand_edit_generated_rs)
);

// ---------------------------------------------------------------------------------------------------------------
// 2. "Only when its content differs": any difference, however small, is written, and the result is the generated
//    content. Stale files near the generated one (a prefix, one extra byte, empty, not UTF-8) must all be rewritten.

/// The generated content of the real emitters over the real layout table, by path.
fn real_generated() -> Vec<Generated> {
    ledger::gen::generate(&ledger::layout(), ledger::gen::EMITTERS).expect("generation refused")
}

/// Seeds every real generated file with `stale(contents)`, runs codegen, and asserts each file was reported written,
/// had its mtime moved, and now holds the generated content.
fn check_stale_is_rewritten(case: &str, stale: fn(&str) -> Vec<u8>) {
    let (root, _) = workspace(case);
    let files = real_generated();
    assert!(!files.is_empty(), "the real emitters generated nothing");
    let paths: BTreeSet<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    for f in &files {
        let path = root.join(&f.path);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(&path, stale(&f.contents)).expect("seed");
    }
    age(&root, &paths);
    let outcome = xtask::codegen::generate(
        ledger::constants::REGISTER,
        &ledger::layout(),
        ledger::gen::EMITTERS,
        &root,
    )
    .expect("codegen refused");
    let written: BTreeSet<PathBuf> = outcome.written.iter().cloned().collect();
    assert_eq!(written, paths, "a stale file was not rewritten");
    assert!(
        outcome.unchanged.is_empty(),
        "a stale file was reported unchanged"
    );
    assert_eq!(
        moved(&root, &paths),
        paths,
        "a stale file's mtime did not move"
    );
    for f in &files {
        let on_disk = fs::read_to_string(root.join(&f.path)).expect("read back");
        assert!(
            on_disk == f.contents,
            "{} does not hold the generated content",
            f.path.display()
        );
    }
}

fn drop_last_byte(s: &str) -> Vec<u8> {
    s.as_bytes()[..s.len() - 1].to_vec()
}
fn extra_newline(s: &str) -> Vec<u8> {
    format!("{s}\n").into_bytes()
}
fn empty(_: &str) -> Vec<u8> {
    Vec::new()
}
fn not_utf8(s: &str) -> Vec<u8> {
    let mut b = s.as_bytes().to_vec();
    b[0] = 0xff;
    b
}
fn one_byte_flipped(s: &str) -> Vec<u8> {
    let mut b = s.as_bytes().to_vec();
    let mid = b.len() / 2;
    b[mid] = if b[mid] == b'x' { b'y' } else { b'x' };
    b
}
#[cfg(feature = "controls")]
fn identical(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

#[test]
fn qa_codegen_unchanged_prefix_is_rewritten() {
    check_stale_is_rewritten("prefix", drop_last_byte);
}

#[test]
fn qa_codegen_unchanged_extra_byte_is_rewritten() {
    check_stale_is_rewritten("extra", extra_newline);
}

#[test]
fn qa_codegen_unchanged_empty_is_rewritten() {
    check_stale_is_rewritten("empty", empty);
}

#[test]
fn qa_codegen_unchanged_not_utf8_is_rewritten() {
    check_stale_is_rewritten("not_utf8", not_utf8);
}

#[test]
fn qa_codegen_unchanged_same_length_edit_is_rewritten() {
    check_stale_is_rewritten("flipped", one_byte_flipped);
}

negative_control!(
    qa_codegen_unchanged_prefix_is_rewritten,
    "a file already holding the generated content is left alone, so it is not reported written",
    expected = "a stale file was not rewritten",
    check_stale_is_rewritten("prefix_control", identical)
);

negative_control!(
    qa_codegen_unchanged_extra_byte_is_rewritten,
    "a file already holding the generated content is left alone, so it is not reported written",
    expected = "a stale file was not rewritten",
    check_stale_is_rewritten("extra_control", identical)
);

negative_control!(
    qa_codegen_unchanged_empty_is_rewritten,
    "a file already holding the generated content is left alone, so it is not reported written",
    expected = "a stale file was not rewritten",
    check_stale_is_rewritten("empty_control", identical)
);

negative_control!(
    qa_codegen_unchanged_not_utf8_is_rewritten,
    "a file already holding the generated content is left alone, so it is not reported written",
    expected = "a stale file was not rewritten",
    check_stale_is_rewritten("not_utf8_control", identical)
);

negative_control!(
    qa_codegen_unchanged_same_length_edit_is_rewritten,
    "a file already holding the generated content is left alone, so it is not reported written",
    expected = "a stale file was not rewritten",
    check_stale_is_rewritten("flipped_control", identical)
);

// ---------------------------------------------------------------------------------------------------------------
// 3. Changing one ledger entry rewrites only the files it affects. The affected set is computed from the generator's
//    output before and after the change (content differs), not from the writer's report.

/// A file independent of the ledger.
fn fixed(_: &[Word], _: &[Entry]) -> Vec<Generated> {
    vec![Generated {
        path: PathBuf::from("qa/fixed.txt"),
        contents: "fixed\n".into(),
    }]
}

/// A file listing every entry's tier gate: changes when any tier gate changes.
fn gates(_: &[Word], entries: &[Entry]) -> Vec<Generated> {
    let mut s = String::new();
    for e in entries {
        s.push_str(&format!("{}: {:?}\n", e.name, e.tier_gate));
    }
    vec![Generated {
        path: PathBuf::from("qa/nested/gates.txt"),
        contents: s,
    }]
}

/// One file per entry, holding just its name: unaffected by a tier-gate change.
fn names(_: &[Word], entries: &[Entry]) -> Vec<Generated> {
    entries
        .iter()
        .map(|e| Generated {
            path: PathBuf::from(format!("qa/names/{}.txt", e.name)),
            contents: format!("{}\n", e.name),
        })
        .collect()
}

fn emitters() -> Vec<Emitter> {
    [
        ledger::gen::EMITTERS,
        &[fixed as Emitter, gates as Emitter, names as Emitter],
    ]
    .concat()
}

fn contents(ledger: &Ledger, emitters: &[Emitter]) -> Vec<(PathBuf, String)> {
    ledger::gen::generate(ledger, emitters)
        .expect("generation refused")
        .into_iter()
        .map(|g| (g.path, g.contents))
        .collect()
}

/// Generates from the layout table, gives the entry at `index` a tier gate, applies `between`, generates again, and
/// asserts the files whose mtime moved, and the files reported written, are exactly those whose generated content
/// differs between the two ledgers; the rest are reported unchanged.
fn check_one_entry_rewrites_only_affected(case: &str, index: usize, between: fn(&Path)) {
    let (root, _) = workspace(case);
    let emitters = emitters();
    let before = ledger::layout();
    let mut after = ledger::layout();
    assert_eq!(after.entries[index].tier_gate, None, "entry already gated");
    after.entries[index].tier_gate = Some("qa_m041");

    let old = contents(&before, &emitters);
    let new = contents(&after, &emitters);
    let all: BTreeSet<PathBuf> = old.iter().map(|(p, _)| p.clone()).collect();
    assert_eq!(
        all,
        new.iter().map(|(p, _)| p.clone()).collect::<BTreeSet<_>>(),
        "the change altered the set of generated paths"
    );
    let affected: BTreeSet<PathBuf> = new
        .iter()
        .filter(|(p, c)| old.iter().find(|(q, _)| q == p).map(|(_, d)| d) != Some(c))
        .map(|(p, _)| p.clone())
        .collect();
    assert!(
        !affected.is_empty() && affected.len() < all.len(),
        "the change must affect some files and not others: {affected:?} of {all:?}"
    );

    xtask::codegen::generate(ledger::constants::REGISTER, &before, &emitters, &root)
        .expect("first codegen refused");
    assert_eq!(files_under(&root), all, "the first run's files");
    age(&root, &all);
    between(&root);
    let outcome = xtask::codegen::generate(ledger::constants::REGISTER, &after, &emitters, &root)
        .expect("second codegen refused");

    assert_eq!(
        moved(&root, &all),
        affected,
        "the files whose modification time moved are not the affected ones"
    );
    let written: BTreeSet<PathBuf> = outcome.written.iter().cloned().collect();
    let unchanged: BTreeSet<PathBuf> = outcome.unchanged.iter().cloned().collect();
    assert_eq!(written, affected, "the files reported written");
    assert_eq!(
        unchanged,
        all.difference(&affected).cloned().collect::<BTreeSet<_>>(),
        "the files reported unchanged"
    );
    for (p, c) in &new {
        assert!(
            &fs::read_to_string(root.join(p)).expect("read back") == c,
            "{} does not hold the new content",
            p.display()
        );
    }
}

/// A hand edit of a file the ledger change does not affect: it is rewritten too, so the moved set is not the affected
/// set.
#[cfg(feature = "controls")]
fn hand_edit_fixed(root: &Path) {
    let path = root.join("qa/fixed.txt");
    fs::write(&path, "edited\n").expect("edit");
    fs::File::options()
        .write(true)
        .open(&path)
        .and_then(|f| f.set_modified(past()))
        .expect("age");
}

#[test]
fn qa_codegen_unchanged_one_entry_rewrites_only_affected_files() {
    check_one_entry_rewrites_only_affected("one_entry", 0, no_change);
}

negative_control!(
    qa_codegen_unchanged_one_entry_rewrites_only_affected_files,
    "an unaffected file hand-edited between the runs is rewritten as well, so the moved set exceeds the affected set",
    expected = "the files whose modification time moved are not the affected ones",
    check_one_entry_rewrites_only_affected("one_entry_control", 0, hand_edit_fixed)
);

// ---------------------------------------------------------------------------------------------------------------
// 4. The constants gate still guards the write (dd_generation_root §3.8, REQ-SYS-001): with the write now in xtask,
//    a refused register leaves a stale generated file on disk as it was, content and modification time.

/// Seeds a stale `generated.rs`, runs `xtask::codegen::generate` with `register`, and asserts it refused and left
/// the file untouched.
fn check_refused_register_writes_nothing(
    case: &str,
    register: &[ledger::constants::ConstantBuilder],
) {
    let (root, _) = workspace(case);
    let path = root.join(ledger::gen::rust::PATH);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(&path, "stale\n").expect("seed");
    let only: BTreeSet<PathBuf> = [PathBuf::from(ledger::gen::rust::PATH)].into();
    age(&root, &only);
    let result =
        xtask::codegen::generate(register, &ledger::layout(), ledger::gen::EMITTERS, &root);
    assert!(
        moved(&root, &only).is_empty() && fs::read_to_string(&path).expect("read") == "stale\n",
        "codegen wrote generated.rs although the register ({} entries) should be refused: {result:?}",
        register.len()
    );
    assert!(result.is_err(), "codegen did not refuse the register");
}

/// The real register with its first entry's citation removed.
fn uncited_register() -> Vec<ledger::constants::ConstantBuilder> {
    let mut register = ledger::constants::REGISTER.to_vec();
    register[0].citation = None;
    register
}

#[test]
fn qa_codegen_unchanged_refused_register_writes_nothing() {
    check_refused_register_writes_nothing("refused", &uncited_register());
}

negative_control!(
    qa_codegen_unchanged_refused_register_writes_nothing,
    "the complete register passes the gate, so the stale file is rewritten and the check must fail",
    expected = "codegen wrote generated.rs although the register",
    check_refused_register_writes_nothing("refused_control", ledger::constants::REGISTER)
);
