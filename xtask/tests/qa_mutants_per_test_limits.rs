//! qa's tests for the tooling PR "per-test kill limits for mutation runs" (branch `tooling/mutants-per-test-limits`),
//! written from the owner's words of 8 Nov 2026 and the PR's brief, not from the code:
//!
//! - "a hung mutant must be stopped in minutes. Use nextest's per-test terminate-after in a mutants profile ... A test
//!   killed this way counts as a timeout, which is caught, as today" and "Killed test recorded by mutants-check as
//!   'timed out (not a survivor)': yes". A real nextest run, as cargo-mutants runs a mutant's tests (the test arguments
//!   of `.cargo/mutants.toml`, NEXTEST_PROFILE=mutants, CI's colours and none), on a probe crate whose test hangs: the
//!   run ends within minutes and fails (so cargo-mutants counts the mutant caught), and `cargo xtask mutants-check` on
//!   that log passes, listing the mutant "timed out (not a survivor)" with the hung test's name; a mutant whose test
//!   merely failed is caught and not listed as timed out.
//! - The brief's limits: "each override 3x the largest baseline ..., period rounded up to 10 s, terminate-after 3",
//!   "default 60 s = R-376's floor", and "the default profile's job-queue override (120 s x 3) must be restated in the
//!   mutants profile": every kill limit the `mutants` profile sets, and every one it inherits, is at least 60 s, every
//!   measured override has `terminate-after = 3` and a whole number of 10 s periods, and the profile's own default
//!   ends a test (a period with no `terminate-after` would never end one).
//! - The brief's check: "fails when, in a run's baseline, any test uses more than half its limit, naming the test and a
//!   regenerated value": `cargo xtask mutants-limits` on a baseline log made from a real nextest run's lines.
//! - "Baseline with no kill limit; same limits for nightly and local": the per-PR baseline runs without the profile,
//!   the per-PR shards and the nightly run with it, a local run is told to select it, and `.cargo/mutants.toml`, which
//!   every run reads, its baseline included, does not select it.
//! - "The qa control-inventory tests (qa_TASK-M0-22/24/25 and similar, which inspect the whole workspace) run once per
//!   CI run as a step, not inside per-mutant test runs. Coverage unchanged": the profile's filter leaves each of those
//!   binaries out, and `ci.yml` and the nightly run still run the profiles that run them; and each test the `mutants`
//!   profile leaves out, whole binary or one test, is still run by `ci.yml` and by the nightly `cpu` job: by a step
//!   under the `ci` profile where that profile does not leave it out too, else by a step under `ci-workspace`, whose
//!   filter names it, selecting its package and binary with no test-name filter.
//! - "Baseline with no kill limit; same limits for nightly and local", for the nightly run and a local run: each nightly
//!   shard measures a baseline in a `cargo mutants` run of its own, outside the kill limits, and runs its mutants with
//!   `--baseline=skip`, so no baseline runs under the limits; the local procedure in `plan/OPERATIONS.md` measures its
//!   baseline (`--build-timeout 0`) without the profile, reads its timeout from that baseline's output, and selects the
//!   profile for the `--baseline=skip` run alone.
//!
//! Each test has its negative control (R-176), which feeds the same assertion an input differing in the one respect
//! the requirement turns on.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::json;
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("qa-mpl: workspace root")
        .to_path_buf()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("qa-mpl: {path}: {e}"))
}

/// `text` with `from`, which must be in it, replaced by `to`: a control's one edit.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa-mpl control: the edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

/// The string array `key` of `.cargo/mutants.toml`, empty if it is not set.
fn mutants_args(key: &str) -> Vec<String> {
    let doc: toml_edit::DocumentMut = read(".cargo/mutants.toml")
        .parse()
        .expect("qa-mpl: .cargo/mutants.toml is TOML");
    doc.get(key)
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .map(|v| v.as_str().expect("qa-mpl: a string").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// `text` without ANSI escape sequences, for reading what a person would read.
fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for d in chars.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------------------------
// The probe: a crate whose test `stuck` passes, fails or hangs as QA_MPL_MODE says, run by real nextest under a
// `mutants` profile with a kill limit of its own, as cargo-mutants runs a mutant's tests.

const PROBE_LIB: &str = r#"
#[test]
fn quick() {}

#[test]
fn stuck() {
    match std::env::var("QA_MPL_MODE").as_deref() {
        Ok("hang") => loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        },
        Ok("fail") => panic!("the mutant's test fails"),
        _ => {}
    }
}
"#;

/// The probe's kill limit: 1 s periods, ended after 2, so 2 s; with `--retries=2`, three attempts.
const PROBE_NEXTEST: &str =
    "[profile.mutants]\nslow-timeout = { period = \"1s\", terminate-after = 2 }\n";

/// The probe crate, built.
fn probe(dir: &Path) {
    std::fs::create_dir_all(dir.join("src")).expect("qa-mpl: probe dir");
    std::fs::create_dir_all(dir.join(".config")).expect("qa-mpl: probe config dir");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"qa_mpl_probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n",
    )
    .expect("qa-mpl: probe manifest");
    std::fs::write(dir.join("src/lib.rs"), PROBE_LIB).expect("qa-mpl: probe lib");
    std::fs::write(dir.join(".config/nextest.toml"), PROBE_NEXTEST).expect("qa-mpl: probe config");
    let build = nextest(dir, true)
        .timed_output()
        .expect("qa-mpl: cargo ran");
    assert!(
        build.status.success(),
        "qa-mpl: the probe does not build: {}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// `cargo nextest run` on the probe as cargo-mutants runs a mutant's tests: `.cargo/mutants.toml`'s test arguments,
/// under NEXTEST_PROFILE=mutants, with nothing of the nextest run this test is part of.
fn nextest(dir: &Path, build: bool) -> Command {
    let mut cmd = Command::new(env!("CARGO"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("NEXTEST") {
            cmd.env_remove(key);
        }
    }
    cmd.args(["nextest", "run"]);
    if build {
        cmd.arg("--no-run");
    }
    cmd.args(mutants_args("additional_cargo_args"));
    if !build {
        cmd.args(mutants_args("additional_cargo_test_args"));
    }
    cmd.arg("--target-dir")
        .arg(dir.join("target"))
        .current_dir(dir)
        .env_remove("CARGO_TARGET_DIR")
        .env("NEXTEST_PROFILE", "mutants");
    cmd
}

/// One test run of the built probe in `mode`, coloured as CI's (`CARGO_TERM_COLOR=always`) or not: whether it
/// passed, how long it took, and its output as cargo-mutants logs it (stdout, then stderr).
fn run_probe(dir: &Path, mode: &str, colour: bool) -> (bool, Duration, String) {
    let start = Instant::now();
    let out = nextest(dir, false)
        .env("QA_MPL_MODE", mode)
        .env("CARGO_TERM_COLOR", if colour { "always" } else { "never" })
        .timed_output()
        .expect("qa-mpl: cargo ran");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), start.elapsed(), log)
}

/// Runs `xtask` with `args`: whether it passed, and its stdout and stderr.
fn xtask(args: &[&std::ffi::OsStr]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(root())
        .timed_output()
        .expect("qa-mpl: xtask ran");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// ---------------------------------------------------------------------------------------------------------------
// A hung test is ended at its limit, its mutant caught, and listed "timed out (not a survivor)", naming the test.

const HUNG: &str = "crates/x/src/lib.rs:1608:5: replace * with + in crossings";
const FAILED: &str = "crates/x/src/lib.rs:12:9: replace < with > in compare";

/// The probe's hung test (`hung` the QA_MPL_MODE of the mutant named [`HUNG`]) and failing test (that of [`FAILED`]):
/// each run ends in minutes and fails; `mutants-check` over the two caught mutants passes and lists exactly [`HUNG`]
/// as timed out, naming `stuck`.
fn hung_test_is_ended_and_listed(hung: &str, colour: bool) {
    let dir = Scratch::new("qa_mpl_probe");
    probe(&dir);
    let out = dir.join("mutants.out");
    std::fs::create_dir_all(out.join("log")).expect("qa-mpl: log dir");
    let mut outcomes = vec![json!({"scenario": "Baseline", "summary": "Success"})];
    for (name, mode, file) in [(HUNG, hung, "hung.log"), (FAILED, "fail", "failed.log")] {
        let (passed, took, log) = run_probe(&dir, mode, colour);
        assert!(
            !passed,
            "qa-mpl: the mutant's test run passed, so cargo-mutants would count `{name}` missed, not caught:\n{log}"
        );
        assert!(
            took < Duration::from_secs(120),
            "qa-mpl: the mutant's test run took {took:?}, not stopped in minutes by its 2 s kill limit"
        );
        std::fs::write(out.join("log").join(file), &log).expect("qa-mpl: log written");
        outcomes.push(json!({
            "scenario": {"Mutant": {"name": name}},
            "summary": "CaughtMutant",
            "log_path": format!("log/{file}"),
        }));
    }
    std::fs::write(
        out.join("outcomes.json"),
        json!({ "outcomes": outcomes }).to_string(),
    )
    .expect("qa-mpl: outcomes written");
    let (ok, stdout, stderr) = xtask(&["mutants-check".as_ref(), out.as_os_str()]);
    let text = plain(&format!("{stdout}{stderr}"));
    assert!(
        ok,
        "qa-mpl: mutants-check failed on two caught mutants, so a hung test's mutant is not counted caught:\n{text}"
    );
    let timed_out: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("timed out (not a survivor)"))
        .collect();
    assert!(
        timed_out
            .iter()
            .any(|l| l.contains(HUNG) && l.contains("stuck")),
        "qa-mpl: the hung test's mutant is not listed \"timed out (not a survivor)\" naming the test `stuck`:\n{text}"
    );
    assert!(
        !timed_out.iter().any(|l| l.contains(FAILED)),
        "qa-mpl: a mutant whose test only failed is listed as timed out:\n{text}"
    );
}

#[test]
fn qa_mpl_a_hung_test_is_ended_at_its_limit_and_listed_timed_out_yet_caught() {
    hung_test_is_ended_and_listed("hang", true);
    hung_test_is_ended_and_listed("hang", false);
}

negative_control!(
    qa_mpl_a_hung_test_is_ended_at_its_limit_and_listed_timed_out_yet_caught,
    "the first mutant's test fails instead of hanging, so nothing was ended at its limit",
    expected = "the hung test's mutant is not listed",
    hung_test_is_ended_and_listed("fail", true)
);

// ---------------------------------------------------------------------------------------------------------------
// `cargo xtask mutants-limits`: a baseline test over half its limit fails the check, named, with a regenerated value.

/// A `slow-timeout` table's limit in seconds, `None` where it sets no `terminate-after` (a period alone ends nothing).
fn limit_of(item: &toml_edit::Item) -> Option<(u64, u64)> {
    let table = item.as_table_like()?;
    let period = table.get("period")?.as_str()?;
    let after = u64::try_from(table.get("terminate-after")?.as_integer()?).ok()?;
    let period = if let Some(s) = period.strip_suffix('s') {
        s.parse::<u64>().ok()?
    } else {
        period.strip_suffix('m')?.parse::<u64>().ok()? * 60
    };
    Some((period, after))
}

/// The `mutants` profile's own limit, in seconds, and one measured override's binary id, test and limit.
fn profile_limits(toml: &str) -> (u64, (String, String, u64)) {
    let doc: toml_edit::DocumentMut = toml.parse().expect("qa-mpl: nextest.toml is TOML");
    let profile = &doc["profile"]["mutants"];
    let (period, after) =
        limit_of(&profile["slow-timeout"]).expect("qa-mpl: the mutants profile sets no kill limit");
    let one = profile["overrides"]
        .as_array_of_tables()
        .expect("qa-mpl: the mutants profile has no overrides")
        .iter()
        .find_map(|o| {
            let filter = o.get("filter")?.as_str()?;
            let (binary, test) = filter
                .strip_prefix("binary_id(=")?
                .split_once(") & test(=")?;
            let test = test.strip_suffix(')')?;
            let (p, a) = limit_of(o.get("slow-timeout")?)?;
            (!binary.contains('/')).then(|| (binary.to_owned(), test.to_owned(), p * a))
        })
        .expect("qa-mpl: no override names one binary and one test");
    (period * after, one)
}

/// A real nextest run's PASS line for the probe's `quick`, as `binary test` at `seconds`.
fn pass_line(real: &str, binary: &str, test: &str, seconds: f64) -> String {
    let line = real
        .lines()
        .find(|l| {
            let p = plain(l);
            p.trim_start().starts_with("PASS [") && p.trim_end().ends_with("qa_mpl_probe quick")
        })
        .unwrap_or_else(|| panic!("qa-mpl: no PASS line for `quick` in the probe's run:\n{real}"));
    // The time's bracket: the first `[` that opens no escape sequence.
    let open = line
        .char_indices()
        .find(|&(i, c)| c == '[' && !line[..i].ends_with('\u{1b}'))
        .map(|(i, _)| i)
        .expect("qa-mpl: a time");
    let close = line[open..].find("s]").expect("qa-mpl: a time's end") + open;
    let made = format!(
        "{}[{seconds:>8.3}{}",
        &line[..open],
        &line[close..]
            .replacen("qa_mpl_probe", binary, 1)
            .replacen("quick", test, 1)
    );
    assert!(
        plain(&made)
            .trim_end()
            .ends_with(&format!("{binary} {test}")),
        "qa-mpl: the probe's PASS line was not renamed: {made:?}"
    );
    made
}

/// `cargo xtask mutants-limits` on a baseline whose two tests, A under the profile's own limit (a test no override
/// names) and B under a measured override, took `share` of their limits plus `plus` seconds each; `over` says which
/// of them took more than half, as the caller reads the requirement. The check must fail exactly when one did, naming
/// each such test, and only those, with the period 3 times its time gives, rounded up to 10 s, and
/// `terminate-after = 3`.
fn half_limit_check(share: [f64; 2], plus: [f64; 2], over: [bool; 2]) {
    let (default_limit, (binary_b, test_b, limit_b)) =
        profile_limits(&read(".config/nextest.toml"));
    let dir = Scratch::new("qa_mpl_half");
    probe(&dir);
    let (passed, _, real) = run_probe(&dir, "pass", true);
    assert!(passed, "qa-mpl: the probe's passing run failed:\n{real}");
    let tests = [
        (
            "xtask::qa_mpl_unlisted".to_owned(),
            "qa_mpl_unlisted_test".to_owned(),
            default_limit,
        ),
        (binary_b, test_b, limit_b),
    ];
    let times = [0, 1].map(|i| share[i] * tests[i].2 as f64 + plus[i]);
    let log: String = (0..2)
        .map(|i| pass_line(&real, &tests[i].0, &tests[i].1, times[i]) + "\n")
        .collect();
    let out = dir.join("mutants.out");
    std::fs::create_dir_all(out.join("log")).expect("qa-mpl: log dir");
    std::fs::write(out.join("log/baseline.log"), &log).expect("qa-mpl: baseline log");
    let (ok, stdout, stderr) = xtask(&["mutants-limits".as_ref(), out.as_os_str()]);
    let text = plain(&format!("{stdout}{stderr}"));
    assert_eq!(
        ok,
        !(over[0] || over[1]),
        "qa-mpl: mutants-limits did not fail exactly when a baseline test took more than half its limit \
         (times {times:?} s against limits {} s and {} s):\n{text}",
        tests[0].2,
        tests[1].2
    );
    for (i, (binary, test, _)) in tests.iter().enumerate() {
        let named = text.contains(&format!("{binary} {test}"));
        assert_eq!(
            named,
            over[i],
            "qa-mpl: mutants-limits {} {binary} {test} ({:.3} s):\n{text}",
            if over[i] { "does not name" } else { "names" },
            times[i]
        );
        if over[i] {
            let period = ((times[i] / 10.0).ceil() as u64) * 10;
            assert!(
                text.contains(&format!("period = \"{period}s\"")) && text.contains("terminate-after = 3"),
                "qa-mpl: mutants-limits gives no regenerated value for {binary} {test} (3 x {:.3} s: period \
                 {period} s, terminate-after 3):\n{text}",
                times[i]
            );
        }
    }
}

#[test]
fn qa_mpl_half_limit_check_fails_naming_the_test_with_a_regenerated_value() {
    // Both at exactly half: within it, so the check passes.
    half_limit_check([0.5, 0.5], [0.0, 0.0], [false, false]);
    // Each just over half, alone and together.
    half_limit_check([0.5, 0.1], [1.5, 0.0], [true, false]);
    half_limit_check([0.1, 0.5], [0.0, 7.25], [false, true]);
    half_limit_check([0.5, 0.5], [0.25, 0.25], [true, true]);
}

negative_control!(
    qa_mpl_half_limit_check_fails_naming_the_test_with_a_regenerated_value,
    "test A over half its limit, read as within it",
    expected = "mutants-limits did not fail exactly when",
    half_limit_check([0.5, 0.1], [1.5, 0.0], [false, false])
);

// ---------------------------------------------------------------------------------------------------------------
// The limits: R-712's sizing, R-376's floor, every test ended, the job queue's limit restated.

/// Each limit the `mutants` profile sets or inherits holds the brief's rules; `Err` names the first that does not.
fn limits_hold(toml: &str) -> Result<usize, String> {
    let doc: toml_edit::DocumentMut = toml.parse().map_err(|e| format!("not TOML: {e}"))?;
    let profile = &doc["profile"]["mutants"];
    let (period, after) = limit_of(&profile["slow-timeout"])
        .ok_or("the mutants profile's own slow-timeout ends no test (no terminate-after)")?;
    if period * after < 60 {
        return Err(format!(
            "the mutants profile's own limit {} s is below R-376's 60 s floor",
            period * after
        ));
    }
    let own = profile["overrides"]
        .as_array_of_tables()
        .ok_or("the mutants profile has no overrides")?;
    let mut measured = 0;
    let mut restated = Vec::new();
    for o in own {
        let filter = o.get("filter").and_then(|f| f.as_str()).unwrap_or_default();
        let Some(timeout) = o.get("slow-timeout") else {
            continue;
        };
        let (p, a) = limit_of(timeout).ok_or(format!(
            "the override `{filter}` sets a slow-timeout that ends no test"
        ))?;
        if p * a < 60 {
            return Err(format!(
                "the override `{filter}`: {p} s x {a} is below R-376's 60 s floor"
            ));
        }
        restated.push((filter.to_owned(), (p, a)));
        if filter.contains("xtask::jobq") {
            continue;
        }
        if a != 3 || p % 10 != 0 {
            return Err(format!(
                "the override `{filter}`: period {p} s, terminate-after {a}, not R-712's 3 periods of whole 10 s"
            ));
        }
        measured += 1;
    }
    for o in doc["profile"]["default"]["overrides"]
        .as_array_of_tables()
        .ok_or("the default profile has no overrides")?
    {
        let Some(timeout) = o.get("slow-timeout") else {
            continue;
        };
        let filter = o.get("filter").and_then(|f| f.as_str()).unwrap_or_default();
        let limit = limit_of(timeout);
        if !restated
            .iter()
            .any(|(f, l)| f == filter && Some(*l) == limit)
        {
            return Err(format!(
                "the default profile's limit for `{filter}` is not restated in the mutants profile"
            ));
        }
    }
    Ok(measured)
}

fn limits_are_sized_and_floored(toml: &str) {
    match limits_hold(toml) {
        Ok(measured) => assert!(
            measured >= 231,
            "qa-mpl: the mutants profile names {measured} measured tests, not the brief's 231"
        ),
        Err(e) => panic!("qa-mpl: {e}"),
    }
}

#[test]
fn qa_mpl_every_kill_limit_is_three_periods_of_whole_10_s_and_never_below_60_s() {
    limits_are_sized_and_floored(&read(".config/nextest.toml"));
}

negative_control!(
    qa_mpl_every_kill_limit_is_three_periods_of_whole_10_s_and_never_below_60_s,
    "one override's limit cut to 3 x 10 s",
    expected = "below R-376's 60 s floor",
    limits_are_sized_and_floored(&{
        let toml = read(".config/nextest.toml");
        let at = toml
            .find("[[profile.mutants.overrides]]\nfilter = \"binary_id(=engine::")
            .expect("qa-mpl control: no measured override");
        let period = toml[at..]
            .find("period = \"")
            .expect("qa-mpl control: a period")
            + at;
        let end = toml[period..].find("s\"").expect("qa-mpl control: its end") + period;
        format!("{}period = \"10{}", &toml[..period], &toml[end..])
    })
);

// ---------------------------------------------------------------------------------------------------------------
// Where the limits bind: never the per-PR baseline; the per-PR shards, the nightly run and a local run.

/// Each step of `yml` whose script runs `cargo mutants` (a command line, not `--list`, not a fixture's `--dir` run):
/// its job, its script and the NEXTEST_PROFILE it runs under (the step's, else the job's, else the workflow's).
fn mutants_runs(yml: &str) -> Vec<(String, String, Option<String>)> {
    let dir = Scratch::new("qa_mpl_yaml");
    std::fs::create_dir_all(&*dir).expect("qa-mpl: yaml dir");
    let file = dir.join("workflow.yml");
    std::fs::write(&file, yml).expect("qa-mpl: workflow written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             top = (d.get('env') or {}).get('NEXTEST_PROFILE')\n\
             runs = []\n\
             for name, job in d['jobs'].items():\n\
             \x20   at = (job.get('env') or {}).get('NEXTEST_PROFILE', top)\n\
             \x20   for s in job.get('steps', []):\n\
             \x20       run = s.get('run') or ''\n\
             \x20       lines = [l.strip() for l in run.splitlines()]\n\
             \x20       if any(l.startswith('cargo mutants') and '--list' not in l and '--dir' not in l for l in lines):\n\
             \x20           runs.append([name, run, (s.get('env') or {}).get('NEXTEST_PROFILE', at)])\n\
             print(json.dumps(runs))",
        ])
        .arg(&file)
        .timed_output()
        .expect("qa-mpl: python3 ran");
    assert!(
        out.status.success(),
        "qa-mpl: the workflow does not parse: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa-mpl: PyYAML's JSON")
}

fn limited(profile: &Option<String>, run: &str) -> bool {
    profile.as_deref() == Some("mutants")
        || run.contains("--profile mutants")
        || run.contains("--profile=mutants")
}

fn limits_bind_where_ordered(mutants: &str, nightly: &str, config: &str, operations: &str) {
    let doc: toml_edit::DocumentMut = config.parse().expect("qa-mpl: .cargo/mutants.toml is TOML");
    let args: Vec<&str> = ["additional_cargo_args", "additional_cargo_test_args"]
        .iter()
        .filter_map(|k| doc.get(k).and_then(|v| v.as_array()))
        .flat_map(|a| a.iter().filter_map(|v| v.as_str()))
        .collect();
    assert!(
        !args.iter().any(|a| a.starts_with("--profile") || a.contains("NEXTEST_PROFILE")),
        "qa-mpl: `.cargo/mutants.toml` selects a nextest profile, so every run's baseline runs under it: {args:?}"
    );
    let per_pr = mutants_runs(mutants);
    let (shards, baselines): (Vec<_>, Vec<_>) = per_pr
        .iter()
        .partition(|(_, run, _)| run.contains("--baseline=skip"));
    assert!(
        !shards.is_empty() && !baselines.is_empty(),
        "qa-mpl: mutants.yml has no shard run or no baseline run: {per_pr:?}"
    );
    for (job, run, profile) in &baselines {
        assert!(
            !limited(profile, run),
            "qa-mpl: the per-PR baseline (job `{job}`) runs under the kill limits; the owner: \"Baseline with no kill \
             limit\""
        );
    }
    for (job, run, profile) in &shards {
        assert!(
            limited(profile, run),
            "qa-mpl: the per-PR shards (job `{job}`) do not run under the kill limits"
        );
    }
    let full = mutants_runs(nightly);
    let mutant_runs: Vec<_> = full
        .iter()
        .filter(|(_, run, _)| !run.contains("--build-timeout 0"))
        .collect();
    assert!(
        !mutant_runs.is_empty()
            && mutant_runs
                .iter()
                .all(|(_, run, profile)| limited(profile, run)),
        "qa-mpl: the nightly run's mutants do not run under the per-PR limits; the owner: \"same limits for nightly \
         and local\": {full:?}"
    );
    assert!(
        operations.contains("NEXTEST_PROFILE=mutants"),
        "qa-mpl: plan/OPERATIONS.md does not tell a local run to select the kill limits (NEXTEST_PROFILE=mutants)"
    );
}

#[test]
fn qa_mpl_the_per_pr_baseline_has_no_kill_limit_and_every_mutant_run_does() {
    limits_bind_where_ordered(
        &read(".github/workflows/mutants.yml"),
        &read(".github/workflows/nightly.yml"),
        &read(".cargo/mutants.toml"),
        &read("plan/OPERATIONS.md"),
    );
}

negative_control!(
    qa_mpl_the_per_pr_baseline_has_no_kill_limit_and_every_mutant_run_does,
    "the per-PR baseline given the profile on its command line",
    expected = "the per-PR baseline (job `mutants-baseline`) runs under the kill limits",
    limits_bind_where_ordered(
        &edited(
            &read(".github/workflows/mutants.yml"),
            "--build-timeout 0 -o \"$RUNNER_TEMP\"",
            "--build-timeout 0 -o \"$RUNNER_TEMP\" -- --profile mutants"
        ),
        &read(".github/workflows/nightly.yml"),
        &read(".cargo/mutants.toml"),
        &read("plan/OPERATIONS.md")
    )
);

// ---------------------------------------------------------------------------------------------------------------
// The control inventories: out of per-mutant runs, still run once by every CI run and nightly.

/// The binaries the owner names, qa_TASK-M0-22/24/25 (with M0-22's r235 file), but qa_TASK-M0-25, whose two
/// inventories R-909 retired, its one remaining test no inventory.
const INVENTORIES: [&str; 3] = [
    "validation::qa_TASK-M0-24",
    "xtask::qa_TASK-M0-22",
    "xtask::qa_TASK-M0-22_r235",
];

/// A profile's `default-filter`, or "" where it has none.
fn default_filter(doc: &toml_edit::DocumentMut, profile: &str) -> String {
    doc["profile"][profile]["default-filter"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn inventories_out_of_mutant_runs_and_in_ci(toml: &str, ci: &str, nightly: &str) {
    let doc: toml_edit::DocumentMut = toml.parse().expect("qa-mpl: nextest.toml is TOML");
    let mutants = default_filter(&doc, "mutants");
    assert!(
        mutants.trim_start().starts_with("not (") && mutants.trim_end().ends_with(')'),
        "qa-mpl: the mutants profile's filter is not an exclusion list, `not ( … )`: {mutants}"
    );
    let ci_out = default_filter(&doc, "ci");
    let workspace_in = default_filter(&doc, "ci-workspace");
    for binary in INVENTORIES {
        let named = format!("binary_id(={binary})");
        assert!(
            mutants.contains(&named),
            "qa-mpl: per-mutant runs still run every test of the control inventory {binary}"
        );
        // Run by the `ci` shards (not among its exclusions) or by `ci-workspace` (among its inclusions).
        assert!(
            !ci_out.contains(&named) || workspace_in.contains(&named),
            "qa-mpl: no CI profile runs the control inventory {binary}"
        );
    }
    for (workflow, text) in [("ci.yml", ci), ("nightly.yml", nightly)] {
        for profile in ["ci", "ci-workspace"] {
            assert!(
                text.contains(&format!("NEXTEST_PROFILE: {profile}\n")),
                "qa-mpl: {workflow} runs no step under the `{profile}` profile, so a control inventory goes unrun"
            );
        }
    }
}

#[test]
fn qa_mpl_control_inventories_leave_per_mutant_runs_and_still_run_in_ci_and_nightly() {
    inventories_out_of_mutant_runs_and_in_ci(
        &read(".config/nextest.toml"),
        &read(".github/workflows/ci.yml"),
        &read(".github/workflows/nightly.yml"),
    );
}

negative_control!(
    qa_mpl_control_inventories_leave_per_mutant_runs_and_still_run_in_ci_and_nightly,
    "qa_TASK-M0-24 back in per-mutant runs",
    expected =
        "per-mutant runs still run every test of the control inventory validation::qa_TASK-M0-24",
    inventories_out_of_mutant_runs_and_in_ci(
        &edited(
            &read(".config/nextest.toml"),
            "        binary_id(=validation::qa_TASK-M0-24)\n        | ",
            "        "
        ),
        &read(".github/workflows/ci.yml"),
        &read(".github/workflows/nightly.yml")
    )
);

// ---------------------------------------------------------------------------------------------------------------
// Every test the `mutants` profile leaves out still runs once in `ci.yml` and once nightly.

/// A filter's terms, one per line (`binary_id(=…)`, or `(binary_id(=…) & test(…))`), as (binary id, test filter).
fn terms(filter: &str) -> Vec<(String, Option<String>)> {
    filter
        .lines()
        .map(|l| l.trim().trim_start_matches("| ").trim())
        .filter(|l| l.contains("binary_id(="))
        .map(|l| {
            let l = l
                .strip_prefix('(')
                .and_then(|l| l.strip_suffix(')'))
                .unwrap_or(l);
            let (binary, test) = match l.split_once(" & ") {
                Some((b, t)) => (b, Some(t.to_owned())),
                None => (l, None),
            };
            let binary = binary
                .strip_prefix("binary_id(=")
                .and_then(|b| b.strip_suffix(')'))
                .unwrap_or_else(|| panic!("qa-mpl: a term that is not one binary: {l}"));
            (binary.to_owned(), test)
        })
        .collect()
}

/// Whether `term` falls under one of `list`: the same binary, the whole of it or the same test filter.
fn covers(list: &[(String, Option<String>)], term: &(String, Option<String>)) -> bool {
    list.iter()
        .any(|(b, t)| *b == term.0 && (t.is_none() || *t == term.1))
}

/// Each `cargo nextest run` command of `yml`, with the NEXTEST_PROFILE it runs under (the step's, the job's or the
/// workflow's).
fn nextest_steps(yml: &str) -> Vec<(Option<String>, String)> {
    let docs = yaml_rust2::YamlLoader::load_from_str(yml).expect("qa-mpl: the workflow is YAML");
    let doc = &docs[0];
    let top = doc["env"]["NEXTEST_PROFILE"].as_str();
    let mut out = Vec::new();
    for (_, job) in doc["jobs"].as_hash().expect("qa-mpl: jobs") {
        let at = job["env"]["NEXTEST_PROFILE"].as_str().or(top);
        for step in job["steps"].as_vec().map_or(&[][..], Vec::as_slice) {
            let profile = step["env"]["NEXTEST_PROFILE"]
                .as_str()
                .or(at)
                .map(str::to_owned);
            for line in step["run"].as_str().unwrap_or("").lines() {
                if let Some(pos) = line.find("cargo nextest run") {
                    out.push((profile.clone(), line[pos..].trim().to_owned()));
                }
            }
        }
    }
    out
}

/// Whether `cmd` runs every test of `binary` (`<package>::<binary>`, or `<package>` for its library) that its
/// profile's filter lets through: it selects the package (or the workspace), the binary if it names any, and no
/// test-name filter or feature narrows it.
fn selects(cmd: &str, binary: &str) -> bool {
    let (package, bin) = binary.split_once("::").unwrap_or((binary, ""));
    let words: Vec<&str> = cmd.split_whitespace().skip(3).collect();
    let mut packages = Vec::new();
    let mut tests = Vec::new();
    let mut workspace = false;
    let mut i = 0;
    while i < words.len() {
        match words[i] {
            "-p" | "--package" => {
                packages.push(words.get(i + 1).copied().unwrap_or(""));
                i += 1;
            }
            "--test" => {
                tests.push(words.get(i + 1).copied().unwrap_or(""));
                i += 1;
            }
            "--no-tests" => i += 1,
            "--workspace" => workspace = true,
            _ => return false,
        }
        i += 1;
    }
    (workspace || packages.contains(&package)) && (tests.is_empty() || tests.contains(&bin))
}

/// Each term of the `mutants` profile's exclusions is run by `ci.yml` and the nightly run: under `ci` where `ci` does
/// not exclude it (`ci.yml`'s `ci` shards; the nightly's `--workspace` step), else under `ci-workspace`, whose filter
/// names it, by a step selecting its binary.
fn every_exclusion_still_runs(toml: &str, ci: &str, nightly: &str) {
    let doc: toml_edit::DocumentMut = toml.parse().expect("qa-mpl: nextest.toml is TOML");
    let excluded = terms(&default_filter(&doc, "mutants"));
    assert!(
        excluded.len() >= 8,
        "qa-mpl: the mutants profile's exclusions did not parse as one term a line: {excluded:?}"
    );
    let ci_out = terms(&default_filter(&doc, "ci"));
    let workspace_in = terms(&default_filter(&doc, "ci-workspace"));
    for (workflow, text) in [("ci.yml", ci), ("nightly.yml", nightly)] {
        let steps = nextest_steps(text);
        for term in &excluded {
            let shown = format!("{} {}", term.0, term.1.as_deref().unwrap_or("(every test)"));
            let run = if !covers(&ci_out, term) {
                steps.iter().any(|(p, cmd)| {
                    p.as_deref() == Some("ci")
                        && (cmd.contains("--archive-file") || selects(cmd, &term.0))
                })
            } else {
                covers(&workspace_in, term)
                    && steps.iter().any(|(p, cmd)| {
                        p.as_deref() == Some("ci-workspace") && selects(cmd, &term.0)
                    })
            };
            assert!(
                run,
                "qa-mpl: {workflow} never runs {shown}, which per-mutant runs leave out"
            );
        }
    }
}

#[test]
fn qa_mpl_every_test_left_out_of_mutant_runs_still_runs_in_ci_and_nightly() {
    every_exclusion_still_runs(
        &read(".config/nextest.toml"),
        &read(".github/workflows/ci.yml"),
        &read(".github/workflows/nightly.yml"),
    );
}

negative_control!(
    qa_mpl_every_test_left_out_of_mutant_runs_still_runs_in_ci_and_nightly,
    "the nightly step that runs xtask's `nextest` once no longer names it",
    expected = "qa-mpl: nightly.yml never runs xtask::nextest (every test), which per-mutant runs leave out",
    every_exclusion_still_runs(
        &read(".config/nextest.toml"),
        &read(".github/workflows/ci.yml"),
        &edited(
            &read(".github/workflows/nightly.yml"),
            "run: cargo nextest run -p xtask --test nextest --test controls",
            "run: cargo nextest run -p xtask --test controls"
        )
    )
);

mod excluded_from_ci_too {
    use super::*;
    negative_control!(
        qa_mpl_every_test_left_out_of_mutant_runs_still_runs_in_ci_and_nightly,
        "the `ci` profile leaves out `controls_runs_on_this_workspace_bare_and_in_ci` too, and `ci-workspace` does not name it",
        expected = "qa-mpl: ci.yml never runs xtask::controls test(=controls_runs_on_this_workspace_bare_and_in_ci)",
        every_exclusion_still_runs(
            &edited(
                &read(".config/nextest.toml"),
                "        | binary_id(=validation::qa_TASK-M0-24)\n",
                "        | binary_id(=validation::qa_TASK-M0-24)\n        | (binary_id(=xtask::controls) & test(=controls_runs_on_this_workspace_bare_and_in_ci))\n"
            ),
            &read(".github/workflows/ci.yml"),
            &read(".github/workflows/nightly.yml")
        )
    );
}

// ---------------------------------------------------------------------------------------------------------------
// The nightly and local baselines: measured alone, with no kill limit; the limits bind the mutants alone.

/// Every backtick span of the text from the owner's "Baseline with no kill limit" in `plan/OPERATIONS.md` to the end
/// of its paragraph.
fn local_procedure(operations: &str) -> Vec<String> {
    let at = operations.find("Baseline with no kill limit").expect(
        "qa-mpl: plan/OPERATIONS.md does not give the owner's \"Baseline with no kill limit\"",
    );
    let rest = &operations[at..];
    let end = ["\n\n", "\n  - "]
        .iter()
        .filter_map(|m| rest.find(m))
        .min()
        .unwrap_or(rest.len());
    rest[..end]
        .split('`')
        .skip(1)
        .step_by(2)
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

fn baselines_have_no_kill_limit(nightly: &str, operations: &str) {
    let runs = mutants_runs(nightly);
    let (mutant_runs, baselines): (Vec<_>, Vec<_>) = runs
        .iter()
        .partition(|(_, run, _)| run.contains("--baseline=skip"));
    assert!(
        !baselines.is_empty() && !mutant_runs.is_empty(),
        "qa-mpl: the nightly run does not measure its baseline in a run of its own, apart from its mutants: {runs:?}"
    );
    for (job, run, profile) in &baselines {
        assert!(
            !limited(profile, run),
            "qa-mpl: the nightly baseline (job `{job}`) runs under the kill limits; the owner: \"Baseline with no kill \
             limit\""
        );
    }
    let spans = local_procedure(operations);
    let baseline = spans
        .iter()
        .find(|s| s.starts_with("cargo mutants") && s.contains("--build-timeout 0"))
        .unwrap_or_else(|| {
            panic!("qa-mpl: plan/OPERATIONS.md gives a local run no baseline run of its own: {spans:?}")
        });
    assert!(
        !baseline.contains("NEXTEST_PROFILE") && !baseline.contains("--profile"),
        "qa-mpl: the local baseline runs under the kill limits: {baseline}"
    );
    let out = baseline
        .split_whitespace()
        .skip_while(|w| *w != "-o")
        .nth(1)
        .expect("qa-mpl: the local baseline names no output directory");
    assert!(
        spans
            .iter()
            .any(|s| s.starts_with("cargo xtask mutants-timeout") && s.contains(&format!("{out}/mutants.out"))),
        "qa-mpl: the local run's timeout is not read from its own baseline's output ({out}/mutants.out): {spans:?}"
    );
    let skip = spans
        .iter()
        .position(|s| s.starts_with("cargo mutants") && s.contains("--baseline=skip"))
        .unwrap_or_else(|| {
            panic!("qa-mpl: the local run's mutants do not skip a limited baseline: {spans:?}")
        });
    let limited_at = spans
        .iter()
        .position(|s| s.contains("NEXTEST_PROFILE=mutants"))
        .unwrap_or(0);
    assert!(
        limited_at > skip,
        "qa-mpl: the local procedure selects the kill limits before its `--baseline=skip` run, not for it: {spans:?}"
    );
}

#[test]
fn qa_mpl_nightly_and_local_baselines_have_no_kill_limit() {
    baselines_have_no_kill_limit(
        &read(".github/workflows/nightly.yml"),
        &read("plan/OPERATIONS.md"),
    );
}

negative_control!(
    qa_mpl_nightly_and_local_baselines_have_no_kill_limit,
    "the nightly shard's baseline step given the mutants profile",
    expected = "the nightly baseline (job `mutants`) runs under the kill limits",
    baselines_have_no_kill_limit(
        &edited(
            &read(".github/workflows/nightly.yml"),
            "PRIN_CONTAIN: required\n          HEARTBEAT_CONTEXT",
            "PRIN_CONTAIN: required\n          NEXTEST_PROFILE: mutants\n          HEARTBEAT_CONTEXT"
        ),
        &read("plan/OPERATIONS.md")
    )
);

mod local_limited {
    use super::*;
    negative_control!(
        qa_mpl_nightly_and_local_baselines_have_no_kill_limit,
        "the local baseline told to run under the mutants profile",
        expected = "the local baseline runs under the kill limits",
        baselines_have_no_kill_limit(
            &read(".github/workflows/nightly.yml"),
            &edited(
                &read("plan/OPERATIONS.md"),
                "--build-timeout 0 -o <dir>/baseline`",
                "--build-timeout 0 -o <dir>/baseline -- --profile mutants`"
            )
        )
    );
}
