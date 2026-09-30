//! `cargo xtask mutants-check` (REQ-VAL-148; R-196, R-202): over the outcomes of `cargo mutants --in-diff` on
//! `fixtures/mutants/untested/` (recorded without its killing test), a surviving mutant not in the equivalent-mutants
//! list fails the check naming it, and one listed with a one-line justification passes.

use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;
use xtask::mutants_check::{equivalents, outcomes, run, unlisted, Equivalent};

/// The two mutants of the untested branch that survive without the killing test.
const SURVIVORS: [&str; 2] = [
    "src/lib.rs:9:17: replace < with > in sign",
    "src/lib.rs:10:9: delete - in sign",
];

/// The recorded run's output directory, holding `outcomes.json`.
fn missed_run() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mutants/missed")
}

/// The fixture list naming both survivors.
fn listed() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/mutants/untested/equivalent.toml")
}

/// A list file under this test's scratch directory, holding `text`.
fn list_file(name: &str, text: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("mutants_check_{name}.toml"));
    std::fs::write(&path, text).expect("list written");
    path
}

fn entry(mutant: &str) -> String {
    format!("[[equivalent]]\nmutant = \"{mutant}\"\njustification = \"one line\"\n")
}

/// The recorded outcomes with every `MissedMutant` turned into `with`.
fn recorded(with: &str) -> String {
    std::fs::read_to_string(missed_run().join("outcomes.json"))
        .expect("fixture read")
        .replace("\"MissedMutant\"", &format!("\"{with}\""))
}

/// The check fails, naming each of `named` and none of `not_named`.
fn fails_naming(list: &Path, named: &[&str], not_named: &[&str]) {
    let result = run(&missed_run(), list);
    let message = result.err().unwrap_or_default();
    assert!(
        named.iter().all(|n| message.contains(n)) && !not_named.iter().any(|n| message.contains(n)),
        "check did not fail naming exactly {named:?}: {message:?}"
    );
}

fn passes(list: &Path) {
    let result = run(&missed_run(), list);
    assert!(result.is_ok(), "check failed: {result:?}");
}

#[test]
fn mutants_check_unlisted_survivors_fail_naming_each() {
    fails_naming(&list_file("empty", ""), &SURVIVORS, &[]);
}

#[test]
fn mutants_check_listed_survivors_pass() {
    passes(&listed());
}

#[test]
fn mutants_check_fails_naming_only_the_unlisted_survivor() {
    let list = list_file("one", &entry(SURVIVORS[0]));
    fails_naming(&list, &SURVIVORS[1..], &SURVIVORS[..1]);
}

/// The run with the killing test, where every mutant is caught, and a timed-out mutant, which is not a survivor.
fn check_survivors(json: &str, missed: &[&str], timeouts: &[&str]) {
    let found = outcomes(json).expect("outcomes read");
    assert!(
        found.missed == missed && found.timeouts == timeouts && unlisted(&found, &[]) == missed,
        "survivors misread: {found:?}"
    );
}

#[test]
fn mutants_check_reads_survivors_and_timeouts() {
    check_survivors(&recorded("MissedMutant"), &SURVIVORS, &[]);
    check_survivors(&recorded("CaughtMutant"), &[], &[]);
    check_survivors(&recorded("Timeout"), &[], &SURVIVORS);
}

fn refuses_baseline(json: &str) {
    let result = outcomes(json);
    assert!(
        result
            .as_ref()
            .is_err_and(|e| e.contains("baseline did not pass")),
        "failed baseline not refused: {result:?}"
    );
}

#[test]
fn mutants_check_refuses_a_failed_baseline() {
    refuses_baseline(&recorded("MissedMutant").replacen("\"Success\"", "\"Failure\"", 1));
}

fn refuses_list(text: &str, naming: &str) {
    let result = equivalents(text);
    assert!(
        result.as_ref().is_err_and(|e| e.contains(naming)),
        "list not refused naming {naming:?}: {result:?}"
    );
}

#[test]
fn mutants_check_refuses_an_entry_without_a_one_line_justification() {
    refuses_list("[[equivalent]]\nmutant = \"m\"\n", "entry #1 (`m`)");
    refuses_list(
        "[[equivalent]]\nmutant = \"m\"\njustification = \" \"\n",
        "entry #1 (`m`)",
    );
    refuses_list(
        &format!(
            "{}[[equivalent]]\nmutant = \"m\"\njustification = \"a\\nb\"\n",
            entry("k")
        ),
        "entry #2 (`m`)",
    );
    refuses_list(
        "[[equivalent]]\njustification = \"j\"\n",
        "entry #1 (`no mutant`)",
    );
    refuses_list("equivalent = \"m\"\n", "not an array of tables");
}

fn reads_list(text: &str, expected: &[Equivalent]) {
    let list = equivalents(text).expect("list read");
    assert!(list == expected, "list misread: {list:?}");
}

#[test]
fn mutants_check_reads_each_entry() {
    reads_list("", &[]);
    reads_list(
        &format!("{}{}", entry(SURVIVORS[0]), entry(SURVIVORS[1])),
        &SURVIVORS.map(|m| Equivalent {
            mutant: m.to_owned(),
            justification: "one line".to_owned(),
        }),
    );
}

/// `cargo xtask mutants-check <dir> --equivalent <list>`: whether it passed, and its stderr.
fn run_binary(list: &Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("mutants-check")
        .arg(missed_run())
        .arg("--equivalent")
        .arg(list)
        .timed_output()
        .expect("xtask ran");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn binary_verdict(list: &Path, pass: bool) {
    let (ok, stderr) = run_binary(list);
    assert!(
        ok == pass && (pass || SURVIVORS.iter().all(|s| stderr.contains(s))),
        "binary verdict wrong (expected pass: {pass}): {stderr}"
    );
}

#[test]
fn mutants_check_binary_reads_the_run_and_the_list() {
    binary_verdict(&list_file("binary_empty", ""), false);
    binary_verdict(&listed(), true);
}

negative_control!(
    mutants_check_unlisted_survivors_fail_naming_each,
    "with both survivors listed there is no failure to name them in",
    expected = "check did not fail naming exactly",
    fails_naming(&listed(), &SURVIVORS, &[])
);

negative_control!(
    mutants_check_listed_survivors_pass,
    "with no survivor listed the passing check must fail",
    expected = "check failed",
    passes(&list_file("control_empty", ""))
);

negative_control!(
    mutants_check_fails_naming_only_the_unlisted_survivor,
    "with neither survivor listed the listed one is named too",
    expected = "check did not fail naming exactly",
    fails_naming(
        &list_file("control_none", ""),
        &SURVIVORS[1..],
        &SURVIVORS[..1]
    )
);

negative_control!(
    mutants_check_reads_survivors_and_timeouts,
    "a run whose survivors timed out has none missed",
    expected = "survivors misread",
    check_survivors(&recorded("Timeout"), &SURVIVORS, &[])
);

negative_control!(
    mutants_check_refuses_a_failed_baseline,
    "a passing baseline is not refused",
    expected = "failed baseline not refused",
    refuses_baseline(&recorded("MissedMutant"))
);

negative_control!(
    mutants_check_refuses_an_entry_without_a_one_line_justification,
    "an entry with a one-line justification is not refused",
    expected = "list not refused",
    refuses_list(&entry("m"), "entry #1")
);

negative_control!(
    mutants_check_reads_each_entry,
    "a list of two entries does not read as empty",
    expected = "list misread",
    reads_list(&entry(SURVIVORS[0]), &[])
);

negative_control!(
    mutants_check_binary_reads_the_run_and_the_list,
    "the binary with no survivor listed must fail the passing verdict",
    expected = "binary verdict wrong",
    binary_verdict(&list_file("control_binary_empty", ""), true)
);
