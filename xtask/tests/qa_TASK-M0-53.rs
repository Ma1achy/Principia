//! QA tests for REQ-SYS-079 as TASK-M0-53 builds it: "The gate report must not pass a requirement whose result comes
//! from the hosted suites (unit test, property test, numerical gate, golden image, GUI screenshot) when a
//! `cargo test -p <crate> [flags] <filter>` its verify detail names matches no test in that crate as the gate run built
//! it with those flags (cargo's own rule: the filter is a substring of the test's path name); such a requirement fails,
//! naming the crate, the filter and "no test matches", and the report fails. The test listing is the gate run's own
//! (`cargo test … -- --list` on that commit), not a scan of the source. A hosted requirement whose detail names no such
//! command keeps its suite's result, marked "no named test to check"." (the human's instruction, 3 Oct 2026: "fix the
//! gate report to check tests exist").
//!
//! Written from the statement and its verify detail: each test builds a scratch workspace (a gate block, requirements
//! in `plan/tools/reqio.py`'s one-line form, the suites' results) and reads the report `run_listed` writes, line by
//! line, so the assertions hold whatever the internals. Each test has its negative control (R-176).
// The file name `qa_TASK-M0-53` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::gate_report::{run_listed, Recorded};

/// The five verification methods whose result comes from the hosted suites (REQ-SYS-079's statement).
const HOSTED: [&str; 5] = [
    "unit test",
    "property test",
    "numerical gate",
    "golden image",
    "GUI screenshot",
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A requirement of the scratch workspace: id, verification method, verify detail, and the suites' result (`None`
/// for none, as for a benchmark).
struct Req<'a> {
    id: &'a str,
    method: &'a str,
    detail: &'a str,
    result: Option<&'a str>,
}

fn req<'a>(id: &'a str, method: &'a str, detail: &'a str) -> Req<'a> {
    Req {
        id,
        method,
        detail,
        result: Some("pass"),
    }
}

/// What `run_listed` gave on a scratch workspace: whether it failed, and its report's line per requirement.
struct Report {
    failed: bool,
    text: String,
}

impl Report {
    /// The report's text after `<id>  ` on that requirement's line.
    fn line(&self, id: &str) -> &str {
        let prefix = format!("{id}  ");
        self.text
            .lines()
            .find_map(|l| l.strip_prefix(prefix.as_str()))
            .unwrap_or_else(|| panic!("the report has no line for {id}:\n{}", self.text))
    }
}

/// Runs the gate report for M0 on a scratch workspace `name` holding `reqs`, with `listing` written as the test
/// listing (`Some`), or with no `--test-list` at all (`None`).
fn report(name: &str, reqs: &[Req], listing: Option<&str>) -> Report {
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = RUN.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m053")
        .join(format!("{name}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("plan")).unwrap();
    let ids: Vec<&str> = reqs.iter().map(|r| r.id).collect();
    std::fs::write(
        root.join("plan/MILESTONES.md"),
        format!(
            "# Milestones (qa fixture)\n\n<!-- gate:M0 -->\n**Exit gate — {n} requirements**:\n\n- QAX ({n}): {}\n\
             <!-- /gate:M0 -->\n",
            ids.join(", "),
            n = ids.len()
        ),
    )
    .unwrap();
    let mut yaml = String::new();
    for r in reqs {
        yaml.push_str(&format!(
            "- id: {}\n  area: QAX\n  statement: \"qa fixture\"\n  verify:\n    method: {}\n    detail: {}\n  \
             milestone: M0\n",
            r.id,
            r.method,
            serde_json::to_string(r.detail).unwrap()
        ));
    }
    std::fs::write(root.join("plan/requirements.yaml"), yaml).unwrap();
    let results: serde_json::Map<String, serde_json::Value> = reqs
        .iter()
        .filter_map(|r| r.result.map(|v| (r.id.to_owned(), v.into())))
        .collect();
    let results_path = root.join("results.json");
    std::fs::write(&results_path, serde_json::to_string(&results).unwrap()).unwrap();
    let list_path = listing.map(|l| {
        let p = root.join("tests.txt");
        std::fs::write(&p, l).unwrap();
        p
    });
    let got = run_listed(
        &root,
        "M0",
        &results_path,
        None,
        list_path.as_deref(),
        &Recorded::default(),
    );
    let text = std::fs::read_to_string(root.join("target/gate-report/M0.txt"))
        .unwrap_or_else(|e| panic!("no report written ({e}); run gave {got:?}"));
    Report {
        failed: got.is_err(),
        text,
    }
}

/// Asserts that `line` fails for a missing named test, naming `krate`, `filter` and "no test matches".
fn assert_no_test(id: &str, line: &str, krate: &str, filter: &str) {
    assert!(
        line.starts_with("FAIL") && line.contains("no test matches"),
        "{id} was not failed for a missing named test: {line}"
    );
    assert!(
        line.contains(krate) && line.contains(filter),
        "{id}'s failure does not name the crate `{krate}` and the filter `{filter}`: {line}"
    );
}

/// A listing as `cargo test <build> -- --list` prints it for a crate with a lib target, an integration test target and
/// doctests: one block per binary, each ending with libtest's count line.
const ALPHA: &str = "== -p alpha\n\
    tests::unit_one: test\n\
    \n\
    1 test, 0 benchmarks\n\
    session_round_trip_holds: test\n\
    session_header_is_read: test\n\
    \n\
    2 tests, 0 benchmarks\n\
    src/lib.rs - parse (line 12): test\n\
    \n\
    1 test, 0 benchmarks\n";

// ---- A named test that exists passes; one in another build does not ---------------------------------------------------

/// A filter that is a strict substring of a listed test's path name, in the middle of it, passes the requirement, for
/// every hosted method; listed under another crate's build instead, it fails.
fn check_substring_passes(listing: &str) {
    let ids: Vec<String> = (1..=HOSTED.len())
        .map(|i| format!("REQ-QAX-00{i}"))
        .collect();
    let reqs: Vec<Req> = HOSTED
        .iter()
        .zip(&ids)
        .map(|(m, id)| {
            req(
                id,
                m,
                "cargo test -p alpha round_trip: the session survives",
            )
        })
        .collect();
    let r = report("substring", &reqs, Some(listing));
    for id in &ids {
        assert_eq!(
            r.line(id),
            "pass",
            "a hosted requirement whose named test exists did not pass:\n{}",
            r.text
        );
    }
    assert!(!r.failed, "the report failed:\n{}", r.text);
}

#[test]
fn qa_m053_existing_named_test_passes() {
    check_substring_passes(ALPHA);
}

negative_control!(
    qa_m053_existing_named_test_passes,
    "the test listed under another crate's build only",
    expected = "a hosted requirement whose named test exists did not pass",
    check_substring_passes(&ALPHA.replace("== -p alpha", "== -p beta"))
);

// ---- A filter matching nothing fails, for every hosted method, and fails the report ----------------------------------

fn check_missing_fails(listing: &str) {
    let ids: Vec<String> = (1..=HOSTED.len())
        .map(|i| format!("REQ-QAX-01{i}"))
        .collect();
    let reqs: Vec<Req> = HOSTED
        .iter()
        .zip(&ids)
        .map(|(m, id)| req(id, m, "`cargo test -p alpha lint_wgsl_finite_max` passes"))
        .collect();
    let r = report("missing", &reqs, Some(listing));
    for id in &ids {
        assert_no_test(id, r.line(id), "alpha", "lint_wgsl_finite_max");
    }
    assert!(
        r.failed,
        "the report passed with missing named tests:\n{}",
        r.text
    );
}

#[test]
fn qa_m053_missing_named_test_fails() {
    check_missing_fails(ALPHA);
}

negative_control!(
    qa_m053_missing_named_test_fails,
    "a listing that holds the named test",
    expected = "was not failed for a missing named test",
    check_missing_fails(&format!("{ALPHA}lint_wgsl_finite_max: test\n"))
);

/// The suites having failed too, the requirement still fails (a missing test never turns a failure into a pass).
fn check_missing_and_failed(result: &str, listing: &str) {
    let mut r0 = req(
        "REQ-QAX-021",
        "unit test",
        "cargo test -p alpha no_such_thing",
    );
    r0.result = Some(result);
    let r = report("missing_failed", &[r0], Some(listing));
    assert!(
        r.line("REQ-QAX-021").starts_with("FAIL"),
        "a requirement with a missing test and a {result} suite is not failed:\n{}",
        r.text
    );
    assert!(r.failed, "the report passed:\n{}", r.text);
}

#[test]
fn qa_m053_missing_test_fails_whatever_the_suite() {
    check_missing_and_failed("fail", ALPHA);
    check_missing_and_failed("pass", ALPHA);
}

negative_control!(
    qa_m053_missing_test_fails_whatever_the_suite,
    "the named test listed, the suites passing",
    expected = "a requirement with a missing test and a pass suite is not failed",
    check_missing_and_failed("pass", &format!("{ALPHA}no_such_thing_at_all: test\n"))
);

// ---- The crate and the flags make the build ---------------------------------------------------------------------------

/// A test listed for crate `alpha` doesn't make a command naming crate `beta` pass.
fn check_other_crate(krate: &str) {
    let detail = format!("cargo test -p {krate} session_header: reads the header");
    let r = report(
        &format!("crate_{krate}"),
        &[req("REQ-QAX-031", "unit test", &detail)],
        Some(&format!(
            "{ALPHA}== -p beta\nother::thing: test\n\n1 test, 0 benchmarks\n"
        )),
    );
    assert_no_test(
        "REQ-QAX-031",
        r.line("REQ-QAX-031"),
        krate,
        "session_header",
    );
}

#[test]
fn qa_m053_test_of_another_crate_does_not_count() {
    check_other_crate("beta");
}

negative_control!(
    qa_m053_test_of_another_crate_does_not_count,
    "the command naming the crate the test is in",
    expected = "was not failed for a missing named test",
    check_other_crate("alpha")
);

/// A command with `--features controls` is checked against that build only: the default build listing the test
/// doesn't make it pass, the controls build listing it does.
fn check_feature_build(listing: &str) {
    let r = report(
        "features",
        &[req(
            "REQ-QAX-041",
            "property test",
            "`cargo test -p alpha --features controls session_round_trip` with its control",
        )],
        Some(listing),
    );
    assert_no_test(
        "REQ-QAX-041",
        r.line("REQ-QAX-041"),
        "alpha",
        "session_round_trip",
    );
}

#[test]
fn qa_m053_feature_build_is_its_own_build() {
    check_feature_build(ALPHA);
}

negative_control!(
    qa_m053_feature_build_is_its_own_build,
    "a listing holding the controls build with the test",
    expected = "was not failed for a missing named test",
    check_feature_build(&format!(
        "{ALPHA}== -p alpha --features controls\nsession_round_trip_holds: test\n\
         session_round_trip_holds::negative_control: test\n\n2 tests, 0 benchmarks\n"
    ))
);

// ---- Several commands: every one must match ---------------------------------------------------------------------------

/// Three commands, the middle one matching nothing: the requirement fails naming that one alone.
fn check_every_one(middle: &str) {
    let detail = format!(
        "cargo test -p alpha unit_one, cargo test -p alpha {middle} and cargo test -p alpha session_header: each holds"
    );
    let r = report(
        "every",
        &[req("REQ-QAX-051", "numerical gate", &detail)],
        Some(ALPHA),
    );
    let line = r.line("REQ-QAX-051");
    assert_no_test("REQ-QAX-051", line, "alpha", middle);
    assert!(
        !line.contains("unit_one") && !line.contains("session_header"),
        "a command whose test exists is named as missing: {line}"
    );
}

#[test]
fn qa_m053_every_named_command_must_match() {
    check_every_one("gone_test");
}

negative_control!(
    qa_m053_every_named_command_must_match,
    "a middle command whose test exists",
    expected = "was not failed for a missing named test",
    check_every_one("round_trip_holds")
);

// ---- The filter is matched against the test's path name only ----------------------------------------------------------

/// libtest's `: test` kind marker is not part of a test's path name: a filter `test` matches no test whose name lacks
/// it.
fn check_kind_marker(listing: &str) {
    let r = report(
        "marker",
        &[req("REQ-QAX-061", "unit test", "cargo test -p gamma test")],
        Some(listing),
    );
    assert_no_test("REQ-QAX-061", r.line("REQ-QAX-061"), "gamma", "test");
}

#[test]
fn qa_m053_kind_marker_is_not_the_name() {
    check_kind_marker("== -p gamma\nalpha::one: test\nbeta_two: test\n\n2 tests, 0 benchmarks\n");
}

negative_control!(
    qa_m053_kind_marker_is_not_the_name,
    "a listing with a test whose name holds the filter",
    expected = "was not failed for a missing named test",
    check_kind_marker(
        "== -p gamma\nalpha::one: test\nbeta_test_two: test\n\n2 tests, 0 benchmarks\n"
    )
);

/// A filter written as a module path matches the tests under it (cargo's substring rule spans `::`).
fn check_module_path(filter: &str) {
    let r = report(
        "module_path",
        &[req(
            "REQ-QAX-062",
            "unit test",
            &format!("`cargo test -p alpha {filter}`"),
        )],
        Some(ALPHA),
    );
    assert_eq!(
        r.line("REQ-QAX-062"),
        "pass",
        "a module-path filter of a listed test did not pass:\n{}",
        r.text
    );
}

#[test]
fn qa_m053_module_path_filter_matches() {
    check_module_path("tests::unit");
}

negative_control!(
    qa_m053_module_path_filter_matches,
    "a module path no listed test is under",
    expected = "a module-path filter of a listed test did not pass",
    check_module_path("other::unit")
);

// ---- No named command: suite result kept, marked ----------------------------------------------------------------------

const UNNAMED: &str = "no named test to check";

/// A hosted requirement naming no command keeps its suites' result, marked: a pass passes the report, a fail fails
/// it.
fn check_unnamed(detail: &str) {
    for method in HOSTED {
        let mut ok = req("REQ-QAX-071", method, detail);
        ok.result = Some("pass");
        let r = report("unnamed_pass", &[ok], Some(ALPHA));
        let line = r.line("REQ-QAX-071");
        assert!(
            line.starts_with("pass") && line.contains(UNNAMED),
            "a passing {method} requirement naming no test is not a marked pass: {line}"
        );
        assert!(!r.failed, "a marked pass failed the report:\n{}", r.text);
        let mut bad = req("REQ-QAX-071", method, detail);
        bad.result = Some("fail");
        let r = report("unnamed_fail", &[bad], Some(ALPHA));
        let line = r.line("REQ-QAX-071");
        assert!(
            line.starts_with("FAIL") && line.contains(UNNAMED),
            "a failing {method} requirement naming no test is not a marked fail: {line}"
        );
        assert!(r.failed, "a marked fail passed the report:\n{}", r.text);
    }
}

#[test]
fn qa_m053_unnamed_keeps_its_result_marked() {
    check_unnamed("the capture shows every pane at 1x and 2x");
}

negative_control!(
    qa_m053_unnamed_keeps_its_result_marked,
    "a detail that names a listed test",
    expected = "requirement naming no test is not a marked pass",
    check_unnamed("cargo test -p alpha unit_one: holds")
);

// ---- A method outside the hosted suites is not checked ----------------------------------------------------------------

/// A benchmark requirement whose detail names a missing test is not failed for it (its result is the human's run).
fn check_benchmark(method: &str) {
    let mut b = req(
        "REQ-QAX-081",
        method,
        "cargo test -p alpha no_such_bench then prin profile",
    );
    // A benchmark has no suite result (the human's run supplies it); the same requirement as a unit test has one.
    b.result = (method != "benchmark").then_some("pass");
    let r = report("benchmark", &[b], Some(ALPHA));
    let line = r.line("REQ-QAX-081");
    assert!(
        !line.contains("no test matches") && !line.contains(UNNAMED),
        "a requirement outside the hosted suites was checked for named tests: {line}"
    );
}

#[test]
fn qa_m053_benchmark_is_not_checked() {
    check_benchmark("benchmark");
}

negative_control!(
    qa_m053_benchmark_is_not_checked,
    "the same requirement as a unit test",
    expected = "a requirement outside the hosted suites was checked for named tests",
    check_benchmark("unit test")
);

// ---- No listing: nothing shows the test exists ------------------------------------------------------------------------

fn check_no_listing(listing: Option<&str>) {
    let r = report(
        "no_listing",
        &[req(
            "REQ-QAX-091",
            "unit test",
            "cargo test -p alpha unit_one",
        )],
        listing,
    );
    assert_no_test("REQ-QAX-091", r.line("REQ-QAX-091"), "alpha", "unit_one");
    assert!(r.failed, "the report passed with no listing:\n{}", r.text);
}

#[test]
fn qa_m053_no_listing_fails_named_tests() {
    check_no_listing(None);
    check_no_listing(Some(""));
}

negative_control!(
    qa_m053_no_listing_fails_named_tests,
    "the listing given",
    expected = "was not failed for a missing named test",
    check_no_listing(Some(ALPHA))
);

/// A `--test-list` path that does not exist (the artifact never arrived) fails the named tests, not the run's reading.
fn check_unreadable(listing_exists: bool) {
    let root =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m053/unreadable_{listing_exists}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("plan")).unwrap();
    std::fs::write(
        root.join("plan/MILESTONES.md"),
        "<!-- gate:M0 -->\n**Exit gate — 1 requirements**:\n\n- QAX (1): REQ-QAX-092\n<!-- /gate:M0 -->\n",
    )
    .unwrap();
    std::fs::write(
        root.join("plan/requirements.yaml"),
        "- id: REQ-QAX-092\n  area: QAX\n  statement: \"x\"\n  verify:\n    method: unit test\n    \
         detail: \"cargo test -p alpha unit_one\"\n  milestone: M0\n",
    )
    .unwrap();
    std::fs::write(root.join("results.json"), r#"{"REQ-QAX-092": "pass"}"#).unwrap();
    let list = root.join("absent/tests.txt");
    if listing_exists {
        std::fs::create_dir_all(list.parent().unwrap()).unwrap();
        std::fs::write(&list, ALPHA).unwrap();
    }
    let got = run_listed(
        &root,
        "M0",
        &root.join("results.json"),
        None,
        Some(&list),
        &Recorded::default(),
    );
    assert!(got.is_err(), "an absent listing passed the report");
    let text = std::fs::read_to_string(root.join("target/gate-report/M0.txt"))
        .expect("an absent listing stopped the report being written");
    assert!(
        text.contains("REQ-QAX-092  FAIL") && text.contains("no test matches"),
        "an absent listing passed the report: {text}"
    );
}

#[test]
fn qa_m053_absent_listing_file_fails_named_tests() {
    check_unreadable(false);
}

negative_control!(
    qa_m053_absent_listing_file_fails_named_tests,
    "the listing file present",
    expected = "an absent listing passed the report",
    check_unreadable(true)
);

// ---- The listing is cargo's own `-- --list`, per build ---------------------------------------------------------------

/// A stand-in cargo that logs its arguments to `log` and prints, for `test -p alpha … -- --list`, `alpha` (and for
/// any other build, one unrelated test).
#[cfg(unix)]
fn stand_in(dir: &Path, alpha: &str) -> PathBuf {
    let cargo = dir.join("cargo");
    let log = dir.join("args.log");
    let script = format!(
        "#!/bin/sh\necho \"$*\" >> '{log}'\ncase \"$*\" in\n  \
         test\\ -p\\ alpha\\ --features\\ controls*--list*) printf '{alpha}' ;;\n  \
         *) printf 'unrelated::thing: test\\n\\n1 test, 0 benchmarks\\n' ;;\nesac\n",
        log = log.display()
    );
    validation::spawn::write_executable(&cargo, script).unwrap();
    cargo
}

/// `write_test_list` asks cargo for `test <build> … -- --list` of the build a gate requirement names, and the report
/// then finds exactly the tests cargo listed: one listed passes, one cargo did not list fails.
#[cfg(unix)]
fn check_written_listing(name: &str, alpha: &str) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_m053")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("plan")).unwrap();
    std::fs::write(
        root.join("plan/MILESTONES.md"),
        "<!-- gate:M0 -->\n**Exit gate — 2 requirements**:\n\n- QAX (2): REQ-QAX-101, REQ-QAX-102\n\
         <!-- /gate:M0 -->\n",
    )
    .unwrap();
    std::fs::write(
        root.join("plan/requirements.yaml"),
        "- id: REQ-QAX-101\n  area: QAX\n  statement: \"x\"\n  verify:\n    method: unit test\n    \
         detail: \"`cargo test -p alpha --features controls listed_one` and its control\"\n  milestone: M0\n\
         - id: REQ-QAX-102\n  area: QAX\n  statement: \"y\"\n  verify:\n    method: golden image\n    \
         detail: \"`cargo test -p alpha --features controls never_listed`\"\n  milestone: M0\n",
    )
    .unwrap();
    std::fs::write(
        root.join("results.json"),
        r#"{"REQ-QAX-101": "pass", "REQ-QAX-102": "pass"}"#,
    )
    .unwrap();
    let cargo = stand_in(&root, alpha);
    let out = root.join("listing/tests.txt");
    validation::spawn::while_no_spawn(|| {
        xtask::gate_report::write_test_list(&root, "M0", cargo.as_os_str(), &out)
    })
    .expect("the listing was not written");
    let log = std::fs::read_to_string(root.join("args.log")).expect("cargo was never run");
    assert!(
        log.lines()
            .any(|l| l.starts_with("test -p alpha --features controls") && l.contains("-- --list")),
        "cargo was not asked to list the named build: {log}"
    );
    let _ = run_listed(
        &root,
        "M0",
        &root.join("results.json"),
        None,
        Some(&out),
        &Recorded::default(),
    );
    let text = std::fs::read_to_string(root.join("target/gate-report/M0.txt")).unwrap();
    assert!(
        text.contains("REQ-QAX-101  pass\n"),
        "a test cargo listed was not found in the written listing:\n{text}"
    );
    assert!(
        text.contains("REQ-QAX-102  FAIL") && text.contains("never_listed"),
        "a test cargo did not list passed:\n{text}"
    );
}

#[cfg(unix)]
#[test]
fn qa_m053_listing_is_cargos_own() {
    check_written_listing(
        "write",
        "lib::listed_one_works: test\\n\\n1 test, 0 benchmarks\\nlisted_one_works::negative_control: test\\n\\n1 \
         test, 0 benchmarks\\n",
    );
}

#[cfg(unix)]
negative_control!(
    qa_m053_listing_is_cargos_own,
    "a cargo that lists no test of the build",
    expected = "a test cargo listed was not found in the written listing",
    check_written_listing("write_control", "\\n0 tests, 0 benchmarks\\n")
);

// ---- The gate run hands its own listing to the report ----------------------------------------------------------------

/// The jobs of a workflow, as (name, text): each `  <name>:` line under `jobs:` opens one.
fn jobs(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut inside = false;
    for l in text.lines() {
        if !l.starts_with(' ') && !l.is_empty() && !l.starts_with('#') {
            inside = l == "jobs:";
            continue;
        }
        if !inside {
            continue;
        }
        let name = l
            .strip_prefix("  ")
            .filter(|r| !r.starts_with(' ') && !r.starts_with('#'));
        if let Some(name) = name.and_then(|n| n.strip_suffix(':')) {
            out.push((name.to_owned(), String::new()));
        } else if let Some((_, body)) = out.last_mut() {
            if !l.trim_start().starts_with('#') {
                body.push_str(l);
                body.push('\n');
            }
        }
    }
    out
}

/// In `gate.yml`, the `cpu` job, which builds and runs the CPU suites on the commit under test, writes the listing
/// (`gate-report --write-test-list`), and the `gate-report` job reads it (`--test-list`) on the run that writes the
/// report.
fn check_gate_workflow(text: &str) {
    let jobs = jobs(text);
    let job = |name: &str| {
        jobs.iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b.as_str())
            .unwrap_or_else(|| panic!("gate.yml has no `{name}` job"))
    };
    let cpu = job("cpu");
    assert!(
        cpu.contains("gate-report --write-test-list") && cpu.contains("upload-artifact"),
        "the cpu job does not write and upload the gate run's test listing"
    );
    let report = job("gate-report");
    assert!(
        report.contains("download-artifact") && report.contains("--test-list"),
        "the gate-report job does not hand the run's listing to the report"
    );
    let run = report
        .lines()
        .skip_while(|l| {
            !(l.contains("cargo xtask gate-report --milestone") && l.contains("--results"))
        })
        .take(3)
        .collect::<String>();
    assert!(
        run.contains("--results") && run.contains("--test-list"),
        "the report's own command does not take the listing: {run}"
    );
}

#[test]
fn qa_m053_gate_run_hands_its_listing_to_the_report() {
    check_gate_workflow(
        &std::fs::read_to_string(repo().join(".github/workflows/gate.yml")).unwrap(),
    );
}

negative_control!(
    qa_m053_gate_run_hands_its_listing_to_the_report,
    "a gate.yml whose report takes no listing",
    expected = "the gate-report job does not hand the run's listing to the report",
    check_gate_workflow(
        &std::fs::read_to_string(repo().join(".github/workflows/gate.yml"))
            .unwrap()
            .replace("--test-list \"$RUNNER_TEMP", "\"$RUNNER_TEMP")
    )
);

// ---- The real plan --------------------------------------------------------------------------------------------------

/// REQ-SYS-079 is in M0's gate, and its own named test is a build the M0 listing covers.
fn check_real_plan(milestones: &str) {
    let ids = xtask::gate_report::gate_ids(milestones, "M0").expect("M0's gate block");
    assert!(
        ids.iter().any(|i| i == "REQ-SYS-079"),
        "M0's gate does not hold REQ-SYS-079"
    );
    let reqs = std::fs::read_to_string(repo().join("plan/requirements.yaml")).unwrap();
    let verifies = xtask::gate_report::verifies(&reqs);
    assert!(
        xtask::gate_report::test_builds(&ids, &verifies).contains(&"-p xtask".to_owned()),
        "M0's listing does not cover REQ-SYS-079's own build"
    );
}

#[test]
fn qa_m053_real_plan_gates_req_sys_079() {
    check_real_plan(&std::fs::read_to_string(repo().join("plan/MILESTONES.md")).unwrap());
}

negative_control!(
    qa_m053_real_plan_gates_req_sys_079,
    "an M0 gate without REQ-SYS-079",
    expected = "M0's gate does not hold REQ-SYS-079",
    check_real_plan(
        &std::fs::read_to_string(repo().join("plan/MILESTONES.md"))
            .unwrap()
            .replace("REQ-SYS-063…079", "REQ-SYS-063…078, REQ-SYS-080")
    )
);
