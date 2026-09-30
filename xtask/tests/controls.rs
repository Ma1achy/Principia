//! `cargo xtask controls` over the fixture crates in `tests/fixtures/controls/` (REQ-VAL-147): a test with a
//! discriminating control passes; a test with no control, and one whose control leaves it passing, fail naming the
//! test; a crate without the `controls` feature is skipped (R-176); a unit test in `src/` pairs by name with the
//! control in the crate's `tests/` (R-199, R-201). A test's registered control replaces the inline one it duplicated
//! (REQ-VAL-152, REQ-VAL-158; R-215). `--list` pairs and checks the listing without running a control (R-226).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{PoisonError, RwLock};
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/own_target.rs"]
mod own_target;
use own_target::{fixture_files, Lease, FIXTURES};

use xtask::controls::{
    control_of, findings, name_wrong_panics, parse_list, parse_results, parse_wrong_panics, Finding,
};

/// The outcome of one `cargo xtask controls` run.
struct Verdict {
    ok: bool,
    stdout: String,
    stderr: String,
}

/// Held to write by `controls_on_this_workspace_skips_gui`, whose `xtask controls` runs cargo into the workspace's
/// target directory, and so replaces the `xtask` there (`CARGO_BIN_EXE_xtask`) each time; held to read by each run of
/// that `xtask` beside it, so none spawns it while it is being replaced (REQ-VAL-164). Under nextest each test is a
/// process of its own, and `.config/nextest.toml` runs that test alone instead (R-231).
static WORKSPACE_TARGET: RwLock<()> = RwLock::new(());

/// Runs `xtask controls` on a copy of the fixture `name`; see [`run_fixture_with`].
fn run_fixture(name: &str, remove: Option<&str>) -> Verdict {
    run_fixture_with(name, remove, &[])
}

/// Runs `xtask controls <args>` on a copy of the fixture `name`, outside this workspace, with the workspace's lockfile
/// and the `validation` path made absolute, building in a target directory of its own while it runs, since other
/// copies of the fixture build at the same time (REQ-VAL-164). `remove` names a file left out of the copy. The copy is
/// the lease's own, one per fixture and `remove`, kept across runs (R-231).
fn run_fixture_with(name: &str, remove: Option<&str>, args: &[&str]) -> Verdict {
    let xtask = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = xtask.parent().unwrap();
    let mut files: Vec<(PathBuf, Vec<u8>)> =
        fixture_files(&xtask.join("tests/fixtures/controls").join(name))
            .into_iter()
            .filter(|(path, _)| remove.is_none_or(|file| path != Path::new(file)))
            .collect();
    for (path, bytes) in &mut files {
        if path == Path::new("Cargo.toml") {
            *bytes = String::from_utf8_lossy(bytes)
                .replace(
                    "../../../../../crates/validation",
                    root.join("crates/validation").to_str().unwrap(),
                )
                .into_bytes();
        }
    }
    files.push((
        PathBuf::from("Cargo.lock"),
        std::fs::read(root.join("Cargo.lock")).unwrap(),
    ));
    let removed = remove.map_or(String::new(), |file| {
        format!("-{}", file.replace(['/', '.'], "_"))
    });
    let spawning = WORKSPACE_TARGET
        .read()
        .unwrap_or_else(PoisonError::into_inner);
    // Taken after the read lock, so no copy holds a directory while it waits for the workspace's run.
    let copy = format!("controls-{name}{removed}");
    let target = Lease::take(FIXTURES, Some(&copy));
    let manifest = target.copy(&copy, &files).join("Cargo.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("controls")
        .args(args)
        .arg("--manifest-path")
        .arg(&manifest)
        .env("CARGO_TARGET_DIR", target.dir())
        .timed_output()
        .expect("run xtask");
    drop(spawning);
    Verdict {
        ok: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Asserts that `text` contains `needle`.
fn has(text: &str, needle: &str) {
    assert!(text.contains(needle), "{needle:?} not in:\n{text}");
}

/// Asserts that `text` does not contain `needle`.
fn lacks(text: &str, needle: &str) {
    assert!(!text.contains(needle), "{needle:?} in:\n{text}");
}

#[test]
fn controls_discriminating_fixture_passes() {
    let v = run_fixture("discriminating", None);
    assert!(
        v.ok,
        "a discriminated test fails the command:\n{}",
        v.stderr
    );
    has(
        &v.stdout,
        "controls_discriminating: 1 test(s), each failed by its control",
    );
}

#[test]
fn controls_test_without_control_fails_naming_it() {
    let v = run_fixture("uncontrolled", None);
    assert!(!v.ok, "a test with no control passed the command");
    has(
        &v.stderr,
        "controls_uncontrolled: test `lacks_control` has no control",
    );
    has(
        &v.stderr,
        "xtask: 1 test(s) without a control that makes them fail",
    );
    // Control: the test beside it has a control named for it, and is not reported: the pairing is by name.
    lacks(&v.stderr, "`has_control`");
}

#[test]
fn controls_control_leaving_test_passing_fails_naming_it() {
    let v = run_fixture("leaky", None);
    assert!(
        !v.ok,
        "a control that leaves its test passing passed the command"
    );
    has(
        &v.stderr,
        "controls_leaky: test `round_trips`: its control did not make it fail",
    );
}

#[test]
fn controls_crate_without_feature_is_skipped() {
    let v = run_fixture("featureless", None);
    assert!(
        v.ok,
        "a crate without the feature failed the command:\n{}",
        v.stderr
    );
    has(
        &v.stdout,
        "controls_featureless: skipped: it declares no `controls` feature",
    );
}

#[test]
fn controls_unit_test_pairs_with_control_in_tests_dir() {
    let v = run_fixture("cross_target", None);
    assert!(
        v.ok,
        "the cross-target pair failed the command:\n{}",
        v.stderr
    );
    has(
        &v.stdout,
        "controls_cross_target: 1 test(s), each failed by its control",
    );
    lacks(&v.stderr, "has no control");
}

#[test]
fn controls_unit_test_without_its_control_fails_naming_it() {
    let v = run_fixture("cross_target", Some("tests/controls.rs"));
    assert!(!v.ok, "the unit test passed with its control removed");
    has(
        &v.stderr,
        "controls_cross_target: test `tests::triples` has no control",
    );
}

/// Through `--list`, which runs no control: the controls of this workspace run in `cargo xtask ci` (R-226).
#[test]
fn controls_on_this_workspace_skips_gui() {
    let replacing = WORKSPACE_TARGET
        .write()
        .unwrap_or_else(PoisonError::into_inner);
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["controls", "--list"])
        .timed_output()
        .expect("run xtask");
    drop(replacing);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // gui has no route to validation (R-187), so it never declares the feature.
    has(&stdout, "xtask controls: gui: skipped");
    // Control: validation declares the feature, and is not reported skipped.
    lacks(&stdout, "validation: skipped");
}

/// `v`, a `--list` run on the leaky fixture, passes and pairs `round_trips` with its control: the leak shows only when
/// the control runs.
fn check_listed_and_not_run(v: &Verdict) {
    assert!(
        v.ok,
        "the leaky fixture failed, so its control was run:\n{}",
        v.stderr
    );
    has(
        &v.stdout,
        "controls_leaky: test `round_trips`: control `round_trips::negative_control`",
    );
}

/// R-226: `--list` pairs each test with its controls, and runs none.
#[test]
fn controls_list_pairs_each_test_and_runs_no_control() {
    check_listed_and_not_run(&run_fixture_with("leaky", None, &["--list"]));
}

/// `--list` still fails on what the listing shows: a test with no control.
#[test]
fn controls_list_fails_naming_a_test_without_control() {
    let v = run_fixture_with("uncontrolled", None, &["--list"]);
    assert!(!v.ok, "--list passed a test with no control");
    has(
        &v.stderr,
        "controls_uncontrolled: test `lacks_control` has no control",
    );
}

/// `xtask <command>` with a `CARGO` that does not exist, so the first cargo run it starts fails, and no control runs.
fn without_cargo(command: &str) -> Verdict {
    let spawning = WORKSPACE_TARGET
        .read()
        .unwrap_or_else(PoisonError::into_inner);
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg(command)
        .env(
            "CARGO",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-such-cargo"),
        )
        .timed_output()
        .expect("run xtask");
    drop(spawning);
    Verdict {
        ok: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// `v`, the run of `xtask <command>` by `without_cargo`, failed at the check's first cargo run.
fn check_reached_cargo(command: &str, v: &Verdict) {
    assert!(!v.ok, "xtask {command} passed with no cargo to run");
    has(
        &format!("{}{}", v.stdout, v.stderr),
        "cannot run cargo metadata",
    );
}

/// Bare `xtask controls`, and the `controls` runner of `xtask ci`, run the check on this workspace: with no cargo to
/// run, each fails at `cargo metadata`, not at its arguments or with a pass. Their passing runs are `cargo xtask ci`,
/// in CI (R-198, R-226).
#[test]
fn controls_runs_on_this_workspace_bare_and_in_ci() {
    for command in ["controls", "ci"] {
        check_reached_cargo(command, &without_cargo(command));
    }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn controls_control_of_reads_the_module_name() {
    assert_eq!(control_of("a::doubles::negative_control"), Some("doubles"));
    assert_eq!(control_of("doubles::negative_control"), Some("doubles"));
    assert_eq!(control_of("negative_control"), None);
    assert_eq!(control_of("doubles::xnegative_control"), None);
}

#[test]
fn controls_parse_list_and_results() {
    let list = "doubles: test\ndoubles::negative_control: test\n\n2 tests, 0 benchmarks\nbench_it: benchmark\n";
    assert_eq!(
        parse_list(list),
        names(&["doubles", "doubles::negative_control"])
    );
    let run = "running 2 tests\ntest a::negative_control - should panic ... ok\n\
               test b::negative_control - should panic ... FAILED\ntest c ... ok\n\ntest result: FAILED.";
    let results = parse_results(run);
    let expected: BTreeMap<String, Vec<bool>> = [
        ("a::negative_control".to_owned(), vec![true]),
        ("b::negative_control".to_owned(), vec![false]),
        ("c".to_owned(), vec![true]),
    ]
    .into();
    assert_eq!(results, expected);
    // Control: a FAILED line reads as failed, so `true` above is read from the outcome.
    assert_eq!(
        parse_results("test d ... FAILED").get("d"),
        Some(&vec![false])
    );
}

#[test]
fn controls_findings_pair_by_name() {
    let listed = names(&[
        "tests::doubles",
        "halves",
        "leaks",
        "other::negative_control",
        "doubles::negative_control",
        "leaks::negative_control",
    ]);
    let results: BTreeMap<String, Vec<bool>> = [
        ("doubles::negative_control".to_owned(), vec![true]),
        ("leaks::negative_control".to_owned(), vec![false]),
        ("other::negative_control".to_owned(), vec![true]),
    ]
    .into();
    assert_eq!(
        findings(&listed, &results),
        Ok(vec![
            Finding::NoControl("halves".to_owned()),
            Finding::ControlPasses("leaks".to_owned()),
        ])
    );
    // Control: a paired control with no result is an error, not a pass.
    let err = findings(&listed, &BTreeMap::new()).unwrap_err();
    has(
        &err,
        "`doubles::negative_control` of test `tests::doubles` did not run",
    );
}

/// Two test targets each register a control named `doubles`, so libtest reports the same control name once per
/// target. Every run is judged on its own: a leaky one fails `doubles` whichever target reports last, and the test
/// listed in both targets is reported once.
#[test]
fn controls_same_name_in_two_targets_judged_each() {
    let listed = names(&[
        "doubles",
        "doubles::negative_control",
        "doubles",
        "doubles::negative_control",
    ]);
    for run in [
        "test doubles::negative_control - should panic ... FAILED\n\
         test doubles::negative_control - should panic ... ok\n",
        "test doubles::negative_control - should panic ... ok\n\
         test doubles::negative_control - should panic ... FAILED\n",
    ] {
        let results = parse_results(run);
        assert_eq!(
            results.get("doubles::negative_control").map(Vec::len),
            Some(2)
        );
        assert_eq!(
            findings(&listed, &results),
            Ok(vec![Finding::ControlPasses("doubles".to_owned())]),
            "a leaky control of `doubles` was masked by a sound one of the same name:\n{run}"
        );
    }
    // Control: with both runs discriminating, `doubles` has no finding, so the one above comes from the leak.
    let sound = parse_results(
        "test doubles::negative_control - should panic ... ok\n\
         test doubles::negative_control - should panic ... ok\n",
    );
    assert_eq!(findings(&listed, &sound), Ok(vec![]));
    // Control: a control listed in two targets but run in one is an error, not a pass.
    let once = parse_results("test doubles::negative_control - should panic ... ok\n");
    let err = findings(&listed, &once).unwrap_err();
    has(
        &err,
        "`doubles::negative_control` of test `doubles` did not run",
    );
}

/// Applied per R-204: within a controls crate a control covers exactly one test. `doubles` in `tests/a.rs` has a
/// control; `doubles` in `tests/b.rs` has none, and `a.rs`'s control does not stand for it.
#[test]
fn controls_same_named_tests_need_a_control_each() {
    let v = run_fixture("collision", None);
    assert!(!v.ok, "two tests `doubles` passed with one control");
    has(
        &v.stderr,
        "controls_collision: tests `doubles` share the name `doubles`: listed 2 time(s), but 1 control(s)",
    );
    has(
        &v.stderr,
        "xtask: 1 test(s) without a control that makes them fail",
    );
}

/// The same rule on unit tests in two modules of `src/`: one control named `round_trips` does not cover both
/// `a::tests::round_trips` and `b::tests::round_trips`.
#[test]
fn controls_findings_same_named_unit_tests_collide() {
    let listed = names(&[
        "a::tests::round_trips",
        "b::tests::round_trips",
        "round_trips::negative_control",
    ]);
    let results: BTreeMap<String, Vec<bool>> =
        [("round_trips::negative_control".to_owned(), vec![true])].into();
    let found = findings(&listed, &results).unwrap();
    assert_eq!(
        found,
        vec![Finding::Collision {
            name: "round_trips".to_owned(),
            tests: names(&["a::tests::round_trips", "b::tests::round_trips"]),
            listings: 2,
            controls: 1,
        }]
    );
    assert_eq!(found[0].tests(), 1);
    // Control: three tests of a name with one control leave two without their own, and are counted as two.
    let three = Finding::Collision {
        name: "round_trips".to_owned(),
        tests: names(&["a::round_trips", "b::round_trips", "c::round_trips"]),
        listings: 3,
        controls: 1,
    };
    assert_eq!(three.tests(), 2, "control: a collision counted as one test");
    // Control: a second control of that name, from another target, gives each test its own, and nothing is found.
    let mut both = listed.clone();
    both.push("round_trips::negative_control".to_owned());
    let twice: BTreeMap<String, Vec<bool>> =
        [("round_trips::negative_control".to_owned(), vec![true, true])].into();
    assert_eq!(findings(&both, &twice), Ok(vec![]));
}

/// Applied per R-204: a doctest in a controls crate counts as a test without a control. The `doctest` fixture is
/// the `discriminating` one with a doctest added.
#[test]
fn controls_doctest_fails_naming_it() {
    let v = run_fixture("doctest", None);
    assert!(!v.ok, "a doctest passed the command with no control");
    has(
        &v.stderr,
        "controls_doctest: doctest `src/lib.rs - double (line 6)` has no control",
    );
    has(
        &v.stderr,
        "xtask: 1 test(s) without a control that makes them fail",
    );
}

/// R-212: a control that panics in its own setup, before its test's check, does not count as failing the test; the
/// command names the test with libtest's note. The control beside it, which trips the check, is not reported.
#[test]
fn controls_control_panicking_without_its_message_fails_naming_it() {
    let v = run_fixture("wrong_message", None);
    assert!(
        !v.ok,
        "a control that panicked in its setup passed the command"
    );
    has(
        &v.stderr,
        "controls_wrong_message: test `doubles_again`: its control did not make it fail (philosophy §4.4): it \
         panicked without its expected message (R-212)",
    );
    has(&v.stderr, "the fixture's setup failed");
    lacks(&v.stderr, "test `doubles`:");
}

/// libtest's note for a control that panicked without its expected message, which it appends to the control's output.
const B_NOTE: &str = "note: panic did not contain expected string
      panic message: \"setup failed\"
 expected substring: \"the check\"";

/// `b`'s panic, as its output shows it before libtest's note.
const B_PANIC: &str = "thread 'b::negative_control' (7) panicked at tests/b.rs:6:36:
setup failed";

/// A child's libtest report, as a control that embeds a child's `cargo test` prints it: a header for the child's
/// test `x`, and the child's own list of failures.
const CHILD_REPORT: &str = "running 1 test
test x ... FAILED

failures:

---- x stdout ----
child output

failures:
    x

test result: FAILED. 0 passed; 1 failed
";

/// libtest's report of two failed controls: `a` did not panic, `b` printed `b_output` (its panic and libtest's note).
fn wrong_panic_run(b_output: &str) -> String {
    format!(
        "running 2 tests
test a::negative_control - should panic ... FAILED
test b::negative_control - should panic ... FAILED

failures:

---- a::negative_control stdout ----
note: test did not panic as expected at tests/a.rs:3:60
---- b::negative_control stdout ----
{b_output}

failures:
    a::negative_control
    b::negative_control

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
"
    )
}

/// The run in which `b` printed only its panic and its note.
fn plain_wrong_panic_run() -> String {
    wrong_panic_run(&format!("\n{B_PANIC}\n{B_NOTE}"))
}

/// `stdout`'s one wrong panic is `b`'s, and it turns `b`'s `ControlPasses` into `WrongPanic`, leaving `a`'s alone.
fn check_wrong_panics(stdout: &str) {
    let wrong = parse_wrong_panics(stdout);
    let note = "panic message: \"setup failed\" expected substring: \"the check\"";
    assert_eq!(
        wrong,
        [("b::negative_control".to_owned(), note.to_owned())].into(),
        "the wrong panic of `b` was not read"
    );
    let found = vec![
        Finding::ControlPasses("a".to_owned()),
        Finding::ControlPasses("tests::b".to_owned()),
        Finding::NoControl("b".to_owned()),
    ];
    assert_eq!(
        name_wrong_panics(found, &wrong),
        vec![
            Finding::ControlPasses("a".to_owned()),
            Finding::WrongPanic {
                test: "tests::b".to_owned(),
                note: note.to_owned(),
            },
            Finding::NoControl("b".to_owned()),
        ]
    );
}

#[test]
fn controls_parse_wrong_panics_reads_libtest_notes() {
    check_wrong_panics(&plain_wrong_panic_run());
}

/// REQ-SYS-069: `b`'s output embeds a child's report, with its own `---- x stdout ----` header, before `b`'s panic;
/// `b` keeps its whole note.
#[test]
fn controls_parse_wrong_panics_keeps_a_note_after_an_embedded_header() {
    check_wrong_panics(&wrong_panic_run(&format!(
        "{CHILD_REPORT}\n{B_PANIC}\n{B_NOTE}"
    )));
}

validation::negative_control!(
    controls_control_panicking_without_its_message_fails_naming_it,
    "the discriminating fixture, whose control trips its check, required to report a wrong panic",
    expected = "\"it panicked without its expected message (R-212)\" not in",
    has(
        &run_fixture("discriminating", None).stderr,
        "it panicked without its expected message (R-212)"
    )
);

validation::negative_control!(
    controls_parse_wrong_panics_reads_libtest_notes,
    "the same run with `b`'s note replaced by a did-not-panic one, required to read a wrong panic",
    expected = "the wrong panic of `b` was not read",
    check_wrong_panics(&plain_wrong_panic_run().replace(
        "note: panic did not contain expected string",
        "note: test did not panic as expected"
    ))
);

validation::negative_control!(
    controls_parse_wrong_panics_keeps_a_note_after_an_embedded_header,
    "the same run with `b`'s note only inside the child's report it embeds, required to read `b`'s wrong panic",
    expected = "the wrong panic of `b` was not read",
    check_wrong_panics(&wrong_panic_run(&format!(
        "{}\n{B_PANIC}",
        CHILD_REPORT.replace("child output", B_NOTE)
    )))
);

validation::negative_control!(
    controls_discriminating_fixture_passes,
    "the leaky fixture, required to pass",
    expected = "control: the leaky fixture did not pass",
    assert!(
        run_fixture("leaky", None).ok,
        "control: the leaky fixture did not pass"
    )
);

validation::negative_control!(
    controls_test_without_control_fails_naming_it,
    "the discriminating fixture, required to fail",
    expected = "control: the discriminating fixture passed",
    assert!(
        !run_fixture("discriminating", None).ok,
        "control: the discriminating fixture passed"
    )
);

validation::negative_control!(
    controls_control_leaving_test_passing_fails_naming_it,
    "the discriminating fixture, required to report a control leaving its test passing",
    expected = "\"did not make it fail\" not in",
    has(
        &run_fixture("discriminating", None).stderr,
        "did not make it fail"
    )
);

validation::negative_control!(
    controls_crate_without_feature_is_skipped,
    "the discriminating fixture, which declares the feature, required to be skipped",
    expected = "\"skipped: it declares no `controls` feature\" not in",
    has(
        &run_fixture("discriminating", None).stdout,
        "skipped: it declares no `controls` feature"
    )
);

validation::negative_control!(
    controls_unit_test_pairs_with_control_in_tests_dir,
    "the cross-target fixture without its control in tests/, required to pass",
    expected = "control: the cross-target fixture without its control in tests/ did not pass",
    assert!(
        run_fixture("cross_target", Some("tests/controls.rs")).ok,
        "control: the cross-target fixture without its control in tests/ did not pass"
    )
);

validation::negative_control!(
    controls_unit_test_without_its_control_fails_naming_it,
    "the cross-target fixture with its control, required to fail",
    expected = "control: the cross-target fixture with its control passed",
    assert!(
        !run_fixture("cross_target", None).ok,
        "control: the cross-target fixture with its control passed"
    )
);

// On a fixture, never on this workspace: the command there would run this control again, without end.
validation::negative_control!(
    controls_on_this_workspace_skips_gui,
    "a fixture workspace, which has no gui, required to report gui skipped",
    expected = "\"xtask controls: gui: skipped\" not in",
    has(
        &run_fixture("discriminating", None).stdout,
        "xtask controls: gui: skipped"
    )
);

validation::negative_control!(
    controls_control_of_reads_the_module_name,
    "a plain test's name, required to name the test it controls",
    expected = "control: a plain test's name names no test",
    assert_eq!(
        control_of("a::doubles"),
        Some("doubles"),
        "control: a plain test's name names no test"
    )
);

validation::negative_control!(
    controls_parse_list_and_results,
    "a benchmark line, required to list as a test",
    expected = "control: a benchmark line is not listed as a test",
    assert_eq!(
        parse_list("doubles: benchmark\n"),
        names(&["doubles"]),
        "control: a benchmark line is not listed as a test"
    )
);

validation::negative_control!(
    controls_findings_pair_by_name,
    "a test with no control, required to have no finding",
    expected = "control: a test with no control has a finding",
    assert_eq!(
        findings(&names(&["halves"]), &BTreeMap::new()),
        Ok(vec![]),
        "control: a test with no control has a finding"
    )
);

validation::negative_control!(
    controls_same_name_in_two_targets_judged_each,
    "one sound and one leaky run of `doubles`, required to have no finding",
    expected = "control: a leaky run of `doubles` has a finding",
    {
        let listed = names(&["doubles", "doubles::negative_control"]);
        let listed = [listed.clone(), listed].concat();
        let run = "test doubles::negative_control - should panic ... ok\n\
                   test doubles::negative_control - should panic ... FAILED\n";
        assert_eq!(
            findings(&listed, &parse_results(run)),
            Ok(vec![]),
            "control: a leaky run of `doubles` has a finding"
        );
    }
);

validation::negative_control!(
    controls_same_named_tests_need_a_control_each,
    "the collision fixture without its uncontrolled test, required to fail",
    expected = "control: the collision fixture without b.rs passed",
    assert!(
        !run_fixture("collision", Some("tests/b.rs")).ok,
        "control: the collision fixture without b.rs passed"
    )
);

validation::negative_control!(
    controls_findings_same_named_unit_tests_collide,
    "two unit tests `round_trips` with one control, required to have no finding",
    expected = "control: two unit tests `round_trips` with one control have a finding",
    {
        let listed = names(&[
            "a::tests::round_trips",
            "b::tests::round_trips",
            "round_trips::negative_control",
        ]);
        let results = [("round_trips::negative_control".to_owned(), vec![true])].into();
        assert_eq!(
            findings(&listed, &results),
            Ok(vec![]),
            "control: two unit tests `round_trips` with one control have a finding"
        );
    }
);

validation::negative_control!(
    controls_doctest_fails_naming_it,
    "the discriminating fixture, which has no doctest, required to report one",
    expected = "cannot name a doctest\" not in",
    has(
        &run_fixture("discriminating", None).stderr,
        "has no control: `negative_control!` cannot name a doctest"
    )
);

validation::negative_control!(
    controls_list_pairs_each_test_and_runs_no_control,
    "the leaky fixture run without --list, which runs its leaky control",
    expected = "the leaky fixture failed, so its control was run",
    check_listed_and_not_run(&run_fixture("leaky", None))
);

validation::negative_control!(
    controls_list_fails_naming_a_test_without_control,
    "the discriminating fixture under --list, required to report a test with no control",
    expected = "\"has no control\" not in",
    has(
        &run_fixture_with("discriminating", None, &["--list"]).stderr,
        "has no control"
    )
);

validation::negative_control!(
    controls_runs_on_this_workspace_bare_and_in_ci,
    "an unknown command, which stops at its arguments",
    expected = "\"cannot run cargo metadata\" not in",
    check_reached_cargo("m022-unknown", &without_cargo("m022-unknown"))
);
