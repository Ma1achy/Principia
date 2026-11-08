//! The per-test kill limits of mutation test runs (the owner's order of 8 Nov 2026, amending R-712's basis, not its
//! multiplier): `.config/nextest.toml`'s `mutants` profile, selected by NEXTEST_PROFILE on the per-PR shards, the
//! nightly run and a local run, never on the per-PR baseline; `cargo xtask mutants-limits`, which fails when a baseline
//! test took more than half its limit, naming it with a regenerated value; `cargo xtask mutants-check`, which lists a
//! caught mutant whose log shows nextest ending a test at its limit as timed out, not a survivor, naming the test; and
//! the control-inventory tests the profile leaves out of per-mutant runs, each still run once by every CI run.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::mutants_limits::{
    attempt, findings, parse_filter, regenerated_period, test_times, timed_out_tests, Limits,
    PROFILE,
};

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn nextest_toml() -> String {
    read(".config/nextest.toml")
}

/// `text` with `from`, which must be in it, replaced by `to`: a control's one edit.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "the control's edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

// ---------------------------------------------------------------------------------------------------------------
// nextest's output, as cargo-mutants logs it.

/// A test run under the `mutants` profile with `--retries=2` whose test hung, as nextest 0.9.146 prints it (measured on a
/// probe crate, 8 Nov 2026), with CI's colours on the final line.
const HUNG: &str = "\
 Nextest run ID 31e1 with nextest profile: mutants
    Starting 2 tests across 2 binaries
        PASS [   0.013s] ( 1/2) exp::slow quick
        SLOW [>  2.000s] (───) exp::slow hangs
TRY 1 TRMNTG [>  4.000s] (───) exp::slow hangs
   TRY 1 TMT [   4.006s] (───) exp::slow hangs
  TRY 2 SLOW [>  2.000s] (───) exp::slow hangs
   TRY 2 TMT [   4.004s] (───) exp::slow hangs
   TRY 3 TMT [   4.005s] (2/2) exp::slow hangs
     Summary [  12.023s] 2 tests run: 1 passed, 1 timed out, 0 skipped
   \u{1b}[31;1mTRY 3 TMT\u{1b}[0m [   4.005s] (2/2) \u{1b}[35;1mexp::slow\u{1b}[0m \u{1b}[34mhangs\u{1b}[0m
";

/// The same without retries: nextest's word is TIMEOUT.
const HUNG_ONCE: &str = "\
 TERMINATING [>  2.000s] (───) exp::slow hangs
     TIMEOUT [   2.003s] (1/1) exp::slow hangs
";

/// A finished attempt as [`attempt`] should read it: status, seconds, binary id and test; `None` for no attempt.
type Read<'a> = Option<(&'a str, f64, &'a str, &'a str)>;

fn reads_attempts(lines: &[(&str, Read)]) {
    for (line, want) in lines {
        let got = attempt(line).map(|a| (a.status, a.seconds, a.binary_id, a.test));
        let want = want.map(|(s, t, b, n)| (s.to_owned(), t, b.to_owned(), n.to_owned()));
        assert_eq!(got, want, "nextest's line misread: {line:?}");
    }
}

#[test]
fn mutants_limits_reads_each_finished_attempt() {
    reads_attempts(&[
        (
            "        PASS [   0.010s] ( 1/33) engine::sim_uniforms horizon_default",
            Some(("PASS", 0.010, "engine::sim_uniforms", "horizon_default")),
        ),
        (
            "   TRY 2 TMT [   4.004s] (───) exp::slow hangs",
            Some(("TMT", 4.004, "exp::slow", "hangs")),
        ),
        (
            "     TIMEOUT [   2.003s] (1/1) exp::slow mod_a::hangs",
            Some(("TIMEOUT", 2.003, "exp::slow", "mod_a::hangs")),
        ),
        ("        SLOW [>  2.000s] (───) exp::slow hangs", None),
        ("TRY 1 TRMNTG [>  4.000s] (───) exp::slow hangs", None),
        (
            "     Summary [  12.023s] 2 tests run: 1 passed, 1 timed out",
            None,
        ),
        ("TRY x TMT [   4.004s] (───) exp::slow hangs", None),
        ("    running 1 test", None),
    ]);
}

negative_control!(
    mutants_limits_reads_each_finished_attempt,
    "a running attempt's bound read as its time",
    expected = "nextest's line misread",
    reads_attempts(&[(
        "        SLOW [>  2.000s] (───) exp::slow hangs",
        Some(("SLOW", 2.0, "exp::slow", "hangs"))
    )])
);

fn names_the_ended(log: &str, want: &[&str]) {
    assert_eq!(
        timed_out_tests(log),
        want,
        "the tests nextest ended are misnamed"
    );
}

#[test]
fn mutants_limits_names_each_test_ended_at_its_limit_once() {
    names_the_ended(HUNG, &["exp::slow hangs"]);
    names_the_ended(HUNG_ONCE, &["exp::slow hangs"]);
    names_the_ended(&HUNG.replace("TMT", "FAIL"), &[]);
}

negative_control!(
    mutants_limits_names_each_test_ended_at_its_limit_once,
    "a log whose test failed, not timed out",
    expected = "the tests nextest ended are misnamed",
    names_the_ended(&HUNG.replace("TMT", "FAIL"), &["exp::slow hangs"])
);

fn times_are(log: &str, want: &[(&str, &str, f64)]) {
    let want: BTreeMap<(String, String), f64> = want
        .iter()
        .map(|(b, t, s)| ((b.to_string(), t.to_string()), *s))
        .collect();
    assert_eq!(test_times(log), want, "the baseline's times are misread");
}

#[test]
fn mutants_limits_takes_each_tests_longest_attempt() {
    times_are(
        HUNG,
        &[("exp::slow", "hangs", 4.006), ("exp::slow", "quick", 0.013)],
    );
}

negative_control!(
    mutants_limits_takes_each_tests_longest_attempt,
    "the last attempt's time in place of the longest",
    expected = "the baseline's times are misread",
    times_are(
        HUNG,
        &[("exp::slow", "hangs", 4.005), ("exp::slow", "quick", 0.013)]
    )
);

// ---------------------------------------------------------------------------------------------------------------
// The profile's filters and limits.

fn filter_selects(filter: &str, selects: &[(&str, &str)], leaves: &[(&str, &str)]) {
    let f = parse_filter(filter).unwrap_or_else(|e| panic!("filter refused: {e}"));
    for (b, t) in selects {
        assert!(f.matches(b, t), "`{filter}` does not select {b} {t}");
    }
    for (b, t) in leaves {
        assert!(!f.matches(b, t), "`{filter}` selects {b} {t}");
    }
}

#[test]
fn mutants_limits_reads_the_profiles_filters() {
    filter_selects(
        "binary_id(=a::b) & test(=t) | package(c) & not test(/^x_/)",
        &[("a::b", "t"), ("c::d", "y"), ("c", "z")],
        &[
            ("a::b", "u"),
            ("a::bb", "t"),
            ("c::d", "x_1"),
            ("cc::d", "y"),
        ],
    );
    filter_selects(
        "not (binary_id(/^xtask::qa_R-658/) | test(=t))",
        &[("xtask::jobq", "u")],
        &[("xtask::qa_R-658_round2", "u"), ("k", "t")],
    );
}

negative_control!(
    mutants_limits_reads_the_profiles_filters,
    "a filter whose `package(a)` selects the test",
    expected = "selects a::b u",
    filter_selects(
        "binary_id(=a::b) & test(=t) | package(a)",
        &[],
        &[("a::b", "u")]
    )
);

fn refuses(filter: &str, naming: &str) {
    let err = parse_filter(filter).err().unwrap_or_default();
    assert!(
        err.contains(naming),
        "`{filter}` not refused naming `{naming}`: {err:?}"
    );
}

#[test]
fn mutants_limits_refuses_a_filter_it_cannot_read() {
    refuses("test(hangs)", "only `=`");
    refuses("binary_id(/a.b/)", "only `=`");
    refuses("test(=a b)", "characters");
    refuses("test(=)", "characters");
    refuses("kind(test)", "not a predicate");
    refuses("package(/^c/)", "by prefix");
    refuses("(test(=a)", "no closing parenthesis");
    refuses("test(=a", "no closing parenthesis");
    refuses("test(=a) test(=b)", "more after the filter");
    refuses("nottest(=a)", "not a predicate");
}

negative_control!(
    mutants_limits_refuses_a_filter_it_cannot_read,
    "a filter it reads",
    expected = "not refused naming",
    refuses("test(=hangs)", "only `=`")
);

/// The profile's limit on `(binary, test)`, in seconds, `None` for none; and whether it runs the test.
fn limit_of(toml: &str, binary: &str, test: &str) -> (Option<f64>, bool) {
    let limits = Limits::read(toml, PROFILE).unwrap_or_else(|e| panic!("profile refused: {e}"));
    (
        limits.limit(binary, test).map(|l| l.seconds()),
        limits.runs(binary, test),
    )
}

fn limits_are(toml: &str, want: &[(&str, &str, Option<f64>, bool)]) {
    for (binary, test, seconds, runs) in want {
        assert_eq!(
            limit_of(toml, binary, test),
            (*seconds, *runs),
            "the mutants profile's limit on {binary} {test} is not the measured one"
        );
    }
}

#[test]
fn mutants_profile_sets_each_measured_limit() {
    limits_are(
        &nextest_toml(),
        &[
            // The slowest measured test, 1728.3 s: 1730 s x 3.
            (
                "validation::codegen_selftest",
                "codegen_selftest_pack_unpack_kernel_gpu",
                Some(5190.0),
                true,
            ),
            // A measured test at 20.6 s: 30 s x 3.
            (
                "validation::roundtrip_preset",
                "roundtrip_preset_golden_renders_its_reference",
                Some(90.0),
                true,
            ),
            // A test only #431's branch adds, by package and name.
            (
                "render::drift_views",
                "drift_view_neutral_band_is_drawn",
                Some(150.0),
                true,
            ),
            // Any other test: the floor, 60 s (R-376).
            ("ledger::derived", "no_such_test", Some(60.0), true),
            // The job queue's own, 120 s x 3.
            ("xtask::jobq", "anything", Some(360.0), true),
            ("xtask::qa_R-658_round4", "anything", Some(360.0), true),
            // Left out of per-mutant runs.
            (
                "validation::qa_TASK-M0-24",
                "qa_m0_24_validation_lists_no_doctest",
                Some(60.0),
                false,
            ),
            (
                "xtask::qa_TASK-M0-22",
                "qa_m022_ci_passes_when_every_control_trips",
                Some(540.0),
                false,
            ),
            // Its neighbour, which reads only the stand-in's listing, stays.
            (
                "xtask::qa_TASK-M0-22",
                "qa_m022_list_runs_no_control",
                Some(60.0),
                true,
            ),
        ],
    );
}

negative_control!(
    mutants_profile_sets_each_measured_limit,
    "the slowest test's override dropped",
    expected = "is not the measured one",
    limits_are(
        &edited(
            &nextest_toml(),
            "binary_id(=validation::codegen_selftest) & test(=codegen_selftest_pack_unpack_kernel_gpu)",
            "binary_id(=validation::codegen_selftest) & test(=renamed)"
        ),
        &[(
            "validation::codegen_selftest",
            "codegen_selftest_pack_unpack_kernel_gpu",
            Some(5190.0),
            true
        )]
    )
);

/// Each of the profile's overrides names one test, `binary_id(=…) & test(=…)` or `package(…) & test(=…)`, at most once,
/// with `terminate-after = 3`, but for the restated job-queue limit.
fn overrides_name_one_test_each(toml: &str) {
    let doc: toml_edit::DocumentMut = toml.parse().expect("nextest.toml parses");
    let list = doc["profile"][PROFILE]["overrides"]
        .as_array_of_tables()
        .expect("the mutants profile's overrides");
    let mut seen = std::collections::BTreeSet::new();
    for o in list.iter().skip(1) {
        let filter = o["filter"].as_str().expect("a filter");
        let (scope, test) = filter
            .split_once(" & test(=")
            .unwrap_or_else(|| panic!("an override that names no one test: {filter}"));
        assert!(
            (scope.starts_with("binary_id(=") || scope.starts_with("package("))
                && test.ends_with(')')
                && !test.contains(['(', '|', '&', ' ']),
            "an override that names no one test: {filter}"
        );
        assert!(
            seen.insert(filter.to_owned()),
            "an override given twice: {filter}"
        );
        let timeout = o["slow-timeout"].as_inline_table().expect("a slow-timeout");
        assert_eq!(
            timeout.get("terminate-after").and_then(|v| v.as_integer()),
            Some(3),
            "an override that does not end its test at the third period: {filter}"
        );
    }
    assert_eq!(seen.len(), 231, "not the 231 measured tests");
}

#[test]
fn mutants_profile_names_the_231_measured_tests_once_each() {
    overrides_name_one_test_each(&nextest_toml());
}

negative_control!(
    mutants_profile_names_the_231_measured_tests_once_each,
    "an override that ends its test at the second period",
    expected = "does not end its test at the third period",
    overrides_name_one_test_each(&edited(
        &nextest_toml(),
        "slow-timeout = { period = \"1730s\", terminate-after = 3 }",
        "slow-timeout = { period = \"1730s\", terminate-after = 2 }"
    ))
);

fn refuses_profile(toml: &str, naming: &str) {
    let err = Limits::read(toml, PROFILE).err().unwrap_or_default();
    assert!(
        err.contains(naming),
        "the profile is not refused naming `{naming}`: {err:?}"
    );
}

const PROFILE_HEAD: &str =
    "[profile.mutants]\nslow-timeout = { period = \"20s\", terminate-after = 3 }\n";

#[test]
fn mutants_limits_refuses_a_profile_it_cannot_read() {
    refuses_profile("[profile.ci]\n", "no profile `mutants`");
    refuses_profile(
        &format!("{PROFILE_HEAD}[[profile.mutants.overrides]]\nfilter = \"kind(test)\"\nslow-timeout = \"9s\"\n"),
        "override #1",
    );
    refuses_profile(
        &format!("{PROFILE_HEAD}[[profile.mutants.overrides]]\nplatform = \"cfg(unix)\"\nfilter = \"test(=a)\"\nslow-timeout = \"9s\"\n"),
        "one platform only",
    );
    refuses_profile(
        &format!("{PROFILE_HEAD}[[profile.mutants.overrides]]\nfilter = \"test(=a)\"\nslow-timeout = {{ period = \"9h\" }}\n"),
        "seconds or minutes",
    );
    refuses_profile(
        &format!("{PROFILE_HEAD}[[profile.mutants.overrides]]\nfilter = \"test(=a)\"\nslow-timeout = {{ period = \"9s\", terminate-after = 0 }}\n"),
        "positive whole number",
    );
    refuses_profile(
        "[profile.mutants]\ndefault-filter = \"kind(test)\"\n",
        "not a predicate",
    );
}

negative_control!(
    mutants_limits_refuses_a_profile_it_cannot_read,
    "an override it reads",
    expected = "the profile is not refused",
    refuses_profile(
        &format!("{PROFILE_HEAD}[[profile.mutants.overrides]]\nfilter = \"test(=a)\"\nslow-timeout = {{ period = \"9s\", terminate-after = 1 }}\n"),
        "positive whole number"
    )
);

/// A profile in which `a::b t` has a limit of 30 s x 3 and `a::b free` none (a duration alone), `a::b gone` is left
/// out, and every other test has 1 minute x 1 (a period in minutes).
const SMALL: &str = "\
[profile.mutants]
default-filter = \"not test(=gone)\"
slow-timeout = { period = \"1m\", terminate-after = 1 }
[[profile.mutants.overrides]]
filter = \"binary_id(=a::b) & test(=t)\"
slow-timeout = { period = \"30s\", terminate-after = 3 }
[[profile.mutants.overrides]]
filter = \"binary_id(=a::b) & test(=free)\"
slow-timeout = \"30s\"
";

fn finds(times: &[(&str, f64)], want: &[&str]) {
    let limits = Limits::read(SMALL, PROFILE).expect("the small profile");
    let times: BTreeMap<(String, String), f64> = times
        .iter()
        .map(|(t, s)| (("a::b".to_owned(), t.to_string()), *s))
        .collect();
    let got = findings(&times, &limits);
    assert_eq!(
        got.len(),
        want.len(),
        "the findings are not the tests over half their limit: {got:#?}"
    );
    for (g, w) in got.iter().zip(want) {
        assert!(g.contains(w), "the finding does not say `{w}`: {g}");
    }
}

#[test]
fn mutants_limits_finds_each_test_over_half_its_limit() {
    // Half of 90 s is 45 s: at it passes, past it fails, naming the test and its regenerated value.
    finds(&[("t", 45.0), ("other", 30.0), ("gone", 1000.0)], &[]);
    finds(
        &[("t", 45.01)],
        &["a::b t took 45.0 s, more than half its limit of 90 s"],
    );
    finds(
        &[("t", 45.01)],
        &["filter = \"binary_id(=a::b) & test(=t)\"\n    slow-timeout = { period = \"50s\", terminate-after = 3 }"],
    );
    finds(
        &[("other", 30.01)],
        &["a::b other took 30.0 s, more than half its limit of 60 s"],
    );
    finds(
        &[("free", 1.0)],
        &["a::b free took 1.0 s and has no kill limit"],
    );
}

negative_control!(
    mutants_limits_finds_each_test_over_half_its_limit,
    "a test past half its limit",
    expected = "the findings are not the tests over half their limit",
    finds(&[("t", 45.01)], &[])
);

fn regenerates(seconds: f64, period: u64) {
    assert_eq!(
        regenerated_period(seconds),
        period,
        "the period regenerated from {seconds} s"
    );
}

#[test]
fn mutants_limits_regenerates_the_period_rounded_up_to_10_s() {
    regenerates(20.0, 20);
    regenerates(20.001, 30);
    regenerates(1728.31, 1730);
}

negative_control!(
    mutants_limits_regenerates_the_period_rounded_up_to_10_s,
    "a time on a 10 s mark rounded past it",
    expected = "the period regenerated from",
    regenerates(20.0, 30)
);

// ---------------------------------------------------------------------------------------------------------------
// The binaries: `cargo xtask mutants-limits` and `cargo xtask mutants-check`.

/// `cargo xtask <args>`: whether it passed, its stdout and its stderr.
fn xtask(args: &[&Path]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .timed_output()
        .expect("xtask ran");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// `cargo xtask mutants-limits` on a run whose baseline's log is `log` (none if `None`).
fn limits_on(tag: &str, log: Option<&str>) -> (bool, String, String) {
    let dir = Scratch::new(&format!("mutants_limits_{tag}"));
    std::fs::create_dir_all(dir.join("log")).expect("log dir");
    if let Some(log) = log {
        std::fs::write(dir.join("log/baseline.log"), log).expect("log written");
    }
    xtask(&[Path::new("mutants-limits"), &dir])
}

/// A baseline log in which `validation::codegen_selftest codegen_selftest_pack_unpack_kernel_gpu` (limit 5190 s) took
/// `slow` seconds and `ledger::derived quick` 1.5 s.
fn baseline_log(slow: f64) -> String {
    format!(
        "        PASS [   1.500s] (1/2) ledger::derived quick\n\
         \u{1b}[32;1m        PASS\u{1b}[0m [{slow:8.3}s] (2/2) validation::codegen_selftest codegen_selftest_pack_unpack_kernel_gpu\n"
    )
}

fn binary_checks_the_baseline(slow: f64, pass: bool) {
    let (ok, out, err) = limits_on("binary", Some(&baseline_log(slow)));
    assert_eq!(
        ok, pass,
        "mutants-limits gave the wrong verdict: {out}{err}"
    );
    if pass {
        assert!(
            out.contains("2 test(s) in the baseline, each within half its kill limit"),
            "mutants-limits did not say what it checked: {out}"
        );
    } else {
        assert!(
            err.contains("validation::codegen_selftest codegen_selftest_pack_unpack_kernel_gpu took 2595.1 s")
                && err.contains("period = \"2600s\", terminate-after = 3"),
            "mutants-limits did not name the test and its regenerated value: {err}"
        );
    }
}

#[test]
fn mutants_limits_binary_checks_the_baseline() {
    binary_checks_the_baseline(1728.31, true);
    binary_checks_the_baseline(2595.1, false);
}

negative_control!(
    mutants_limits_binary_checks_the_baseline,
    "a baseline test past half its limit",
    expected = "mutants-limits gave the wrong verdict",
    binary_checks_the_baseline(2595.1, true)
);

fn refuses_an_unmeasured_run(log: Option<&str>, naming: &str) {
    let (ok, out, err) = limits_on("unmeasured", log);
    assert!(
        !ok && err.contains(naming),
        "mutants-limits passed a run it could not measure: {out}{err}"
    );
}

#[test]
fn mutants_limits_binary_refuses_a_run_it_cannot_measure() {
    refuses_an_unmeasured_run(None, "cannot read");
    refuses_an_unmeasured_run(Some("nothing nextest wrote\n"), "nothing was measured");
    let (ok, out, err) = limits_on(
        "no_tests",
        Some("     Summary [   0.001s] 0 tests run: 0 passed, 0 skipped\n"),
    );
    assert!(ok, "a baseline with no test was refused: {out}{err}");
}

negative_control!(
    mutants_limits_binary_refuses_a_run_it_cannot_measure,
    "a baseline that measured a test",
    expected = "mutants-limits passed a run it could not measure",
    refuses_an_unmeasured_run(Some(&baseline_log(1.0)), "nothing was measured")
);

/// `cargo xtask mutants-check` on a shard whose caught mutant `a.rs:1:1: caught` has the log `log` (no log path in
/// `outcomes.json` if `None`) and whose mutant `b.rs:2:2: slow` timed out at cargo-mutants' own timeout.
fn check_with_caught_log(tag: &str, log: Option<&str>) -> (bool, String, String) {
    let dir = Scratch::new(&format!("mutants_limits_check_{tag}"));
    std::fs::create_dir_all(dir.join("log")).expect("log dir");
    let mut caught =
        json!({"scenario": {"Mutant": {"name": "a.rs:1:1: caught"}}, "summary": "CaughtMutant"});
    if let Some(log) = log {
        std::fs::write(dir.join("log/a.log"), log).expect("log written");
        caught["log_path"] = Value::from("log/a.log");
    }
    let outcomes = json!({"outcomes": [
        {"scenario": "Baseline", "summary": "Success"},
        caught,
        {"scenario": {"Mutant": {"name": "b.rs:2:2: slow"}}, "summary": "Timeout"},
    ]});
    std::fs::write(dir.join("outcomes.json"), outcomes.to_string()).expect("outcomes written");
    xtask(&[Path::new("mutants-check"), &dir])
}

fn lists_the_ended(log: Option<&str>, ended: bool) {
    let (ok, out, err) = check_with_caught_log("ended", log);
    assert!(ok, "a caught or timed-out mutant failed the check: {err}");
    let line = "mutants-check: timed out (not a survivor): a.rs:1:1: caught \
                (nextest ended exp::slow hangs at its per-test limit)\n";
    let count = if ended { 2 } else { 1 };
    assert!(
        out.contains(&format!("mutants-check: {count} timed out over 1 shard(s)"))
            && out.contains("mutants-check: timed out (not a survivor): b.rs:2:2: slow\n")
            && out.contains(line) == ended
            && out.matches("timed out (not a survivor)").count() == count,
        "mutants-check does not list exactly the timed-out mutants: {out}"
    );
}

#[test]
fn mutants_check_lists_a_caught_mutant_nextest_ended_as_timed_out() {
    lists_the_ended(Some(HUNG), true);
    lists_the_ended(Some(HUNG_ONCE), true);
    lists_the_ended(Some(&HUNG.replace("TMT", "FAIL")), false);
    lists_the_ended(None, false);
}

negative_control!(
    mutants_check_lists_a_caught_mutant_nextest_ended_as_timed_out,
    "a caught mutant whose test failed, not timed out",
    expected = "mutants-check does not list exactly the timed-out mutants",
    lists_the_ended(Some(&HUNG.replace("TMT", "FAIL")), true)
);

// ---------------------------------------------------------------------------------------------------------------
// Where the profile is selected: the per-PR shards, the nightly run, never the per-PR baseline, never the config the
// fixtures' runs read.

/// `yml` as JSON, read by PyYAML, which `xtask/tests` already need (`merge_ready.rs`).
fn parse(yml: &str) -> Value {
    let dir = Scratch::new("mutants_limits_yaml");
    std::fs::create_dir_all(&dir).expect("yaml dir");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("workflow written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             if True in d: d['on'] = d.pop(True)\n\
             print(json.dumps(d))",
        ])
        .arg(&file)
        .timed_output()
        .expect("python3 ran");
    assert!(
        out.status.success(),
        "the workflow does not parse: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("PyYAML's JSON")
}

/// The step of `job` whose name starts with `name`.
fn step<'a>(job: &'a Value, name: &str) -> &'a Value {
    job["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|s| s["name"].as_str().unwrap_or_default().starts_with(name))
        .unwrap_or_else(|| panic!("no step `{name}`"))
}

/// The NEXTEST_PROFILE a step runs under: its own, else its job's.
fn profile_of<'a>(job: &'a Value, step: &'a Value) -> Option<&'a str> {
    step["env"]["NEXTEST_PROFILE"]
        .as_str()
        .or(job["env"]["NEXTEST_PROFILE"].as_str())
}

fn selected_where_it_should_be(mutants: &str, nightly: &str, config: &str) {
    let (m, n) = (parse(mutants), parse(nightly));
    let shards = &m["jobs"]["mutants"];
    assert_eq!(
        profile_of(shards, step(shards, "cargo mutants --in-diff --shard")),
        Some(PROFILE),
        "the per-PR shards do not run under the kill limits"
    );
    let baseline = &m["jobs"]["mutants-baseline"];
    assert_eq!(
        profile_of(
            baseline,
            step(baseline, "cargo mutants --in-diff, the unmutated")
        ),
        None,
        "the per-PR baseline runs under a kill limit"
    );
    let check = step(baseline, "cargo xtask mutants-limits");
    assert!(
        check["run"].as_str().unwrap_or_default()
            == "cargo xtask mutants-limits \"$RUNNER_TEMP/mutants.out\""
            && check["if"].as_str() == Some("steps.baseline.outputs.measured == 'true'"),
        "the per-PR baseline is not checked against the limits: {check}"
    );
    let full = &n["jobs"]["mutants"];
    assert_eq!(
        profile_of(full, step(full, "cargo mutants --package")),
        Some(PROFILE),
        "the nightly run does not run under the kill limits"
    );
    assert!(
        step(full, "cargo xtask mutants-limits")["run"]
            .as_str()
            .unwrap_or_default()
            .contains("cargo xtask mutants-limits \"$RUNNER_TEMP/mutants.out\""),
        "the nightly run's baselines are not checked against the limits"
    );
    assert!(
        !config.contains("--profile"),
        "`.cargo/mutants.toml` selects a nextest profile, which the fixtures' runs read too"
    );
}

#[test]
fn mutants_profile_is_selected_on_mutation_runs_only() {
    selected_where_it_should_be(
        &read(".github/workflows/mutants.yml"),
        &read(".github/workflows/nightly.yml"),
        &read(".cargo/mutants.toml"),
    );
}

negative_control!(
    mutants_profile_is_selected_on_mutation_runs_only,
    "the per-PR baseline run under the profile",
    expected = "the per-PR baseline runs under a kill limit",
    selected_where_it_should_be(
        &edited(
            &read(".github/workflows/mutants.yml"),
            "          HEARTBEAT_CONTEXT: mutants-heartbeat (baseline)\n",
            "          NEXTEST_PROFILE: mutants\n          HEARTBEAT_CONTEXT: mutants-heartbeat (baseline)\n"
        ),
        &read(".github/workflows/nightly.yml"),
        &read(".cargo/mutants.toml")
    )
);

// ---------------------------------------------------------------------------------------------------------------
// The control-inventory tests: out of per-mutant runs, and still run once by every CI run.

/// The tests the profile leaves out (the owner, 8 Nov 2026), each as its binary id and name.
const INVENTORY: &[(&str, &str)] = &[
    (
        "validation::qa_TASK-M0-24",
        "qa_m0_24_every_implementer_test_has_a_registered_control",
    ),
    (
        "validation::qa_TASK-M0-24",
        "qa_m0_24_every_registered_control_makes_its_test_fail",
    ),
    (
        "validation::qa_TASK-M0-24",
        "qa_m0_24_xtask_declares_controls_with_a_validation_dev_dependency",
    ),
    (
        "validation::qa_TASK-M0-24",
        "qa_m0_24_validation_lists_no_doctest",
    ),
    (
        "validation::qa_TASK-M0-25",
        "qa_m0_25_every_merged_qa_test_has_a_control_and_no_child_body_is_a_test",
    ),
    (
        "validation::qa_TASK-M0-25",
        "qa_m0_25_prin_every_merged_qa_test_has_a_control_and_no_child_body_is_a_test",
    ),
    (
        "xtask::qa_TASK-M0-22",
        "qa_m022_ci_fails_on_a_control_that_leaves_its_test_passing",
    ),
    (
        "xtask::qa_TASK-M0-22",
        "qa_m022_ci_fails_on_a_test_without_control",
    ),
    (
        "xtask::qa_TASK-M0-22",
        "qa_m022_ci_passes_when_every_control_trips",
    ),
    (
        "xtask::qa_TASK-M0-22_r235",
        "qa_r235_ci_list_fails_on_a_test_without_control",
    ),
];

/// Whether the nextest profile `profile`'s `default-filter` in `toml` selects the test.
fn profile_selects(toml: &str, profile: &str, binary: &str, test: &str) -> bool {
    let doc: toml_edit::DocumentMut = toml.parse().expect("nextest.toml parses");
    let filter = doc["profile"][profile]["default-filter"]
        .as_str()
        .unwrap_or_else(|| panic!("profile `{profile}` has no default-filter"));
    parse_filter(filter)
        .unwrap_or_else(|e| panic!("profile `{profile}`: {e}"))
        .matches(binary, test)
}

/// Each inventory test exists, is left out of the `mutants` profile, and is run once by `ci.yml` (the `ci` shards'
/// profile or `ci-workspace`'s, which are disjoint) and by the nightly `cpu` job; the profile leaves nothing else out of
/// those binaries but what its comment names.
fn inventory_runs_once_per_ci_run(toml: &str, ci: &str, nightly: &str) {
    for (binary, test) in INVENTORY {
        let (package, file) = binary.split_once("::").expect("a binary id");
        let dir = if package == "xtask" {
            "xtask".to_owned()
        } else {
            format!("crates/{package}")
        };
        assert!(
            read(&format!("{dir}/tests/{file}.rs")).contains(&format!("fn {test}()")),
            "the inventory test {binary} {test} is gone"
        );
        assert!(
            !profile_selects(toml, PROFILE, binary, test),
            "per-mutant runs still run the inventory test {binary} {test}"
        );
        let shards = profile_selects(toml, "ci", binary, test);
        let workspace = profile_selects(toml, "ci-workspace", binary, test);
        assert!(
            shards != workspace,
            "CI does not run the inventory test {binary} {test} exactly once (ci: {shards}, ci-workspace: {workspace})"
        );
        let runs = if shards {
            vec![
                (ci, "NEXTEST_PROFILE: ci\n"),
                (
                    nightly,
                    "NEXTEST_PROFILE: ci\n        run: cargo nextest run --workspace\n",
                ),
            ]
        } else {
            vec![
                (ci, "NEXTEST_PROFILE: ci-workspace\n        run: cargo nextest run -p validation --test qa_TASK-M0-24\n"),
                (nightly, "NEXTEST_PROFILE: ci-workspace\n        run: cargo nextest run -p validation --test qa_TASK-M0-24\n"),
            ]
        };
        for (workflow, run) in runs {
            assert!(
                workflow.contains(run),
                "no CI step runs the inventory test {binary} {test} ({run:?})"
            );
        }
    }
    for (binary, test) in [
        ("xtask::qa_TASK-M0-22", "qa_m022_list_runs_no_control"),
        (
            "validation::qa_TASK-M0-25",
            "qa_m0_25_moved_failing_property_reports_a_draw_the_property_fails_on",
        ),
        (
            "xtask::controls",
            "controls_runs_on_this_workspace_bare_and_in_ci",
        ),
    ] {
        assert!(
            profile_selects(toml, PROFILE, binary, test),
            "per-mutant runs leave out {binary} {test}, which is no control inventory"
        );
    }
}

#[test]
fn mutants_profile_leaves_out_the_control_inventories_which_ci_runs_once() {
    inventory_runs_once_per_ci_run(
        &nextest_toml(),
        &read(".github/workflows/ci.yml"),
        &read(".github/workflows/nightly.yml"),
    );
}

negative_control!(
    mutants_profile_leaves_out_the_control_inventories_which_ci_runs_once,
    "the r235 inventory back in per-mutant runs",
    expected = "per-mutant runs still run the inventory test",
    inventory_runs_once_per_ci_run(
        &edited(
            &nextest_toml(),
            "        | (binary_id(=xtask::qa_TASK-M0-22_r235) & test(=qa_r235_ci_list_fails_on_a_test_without_control))\n",
            ""
        ),
        &read(".github/workflows/ci.yml"),
        &read(".github/workflows/nightly.yml")
    )
);
