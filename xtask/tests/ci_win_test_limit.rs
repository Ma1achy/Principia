//! A per-test hang limit for the self-hosted Windows runner's `win-tests` job (`.github/workflows/win-gpu.yml`): its
//! workspace step runs nextest's `ci` profile, and on 11 Nov 2026 one kernel GPU test hung there on the RTX for 3.6 h
//! (job 130531059795). `.config/nextest.toml` gives that profile, on Windows alone, a `slow-timeout` that ends a hung
//! test and reports it `TMT`, the job queue's tests keeping their own tighter limit.
//!
//! - `ci_win_test_limit_ends_a_hung_test_on_windows`: the override is there, on `cfg(windows)` only, over every test but
//!   the job queue's, ending a test at 900 s, between 1.5 times the longest passing test of that run on the PC
//!   (342.9 s) and 30 minutes; and `win-tests`' workspace step still runs the `ci` profile.
//!
//! Its negative controls (R-176) each edit the one thing the test turns on.

use std::path::{Path, PathBuf};

use validation::negative_control;

const CONFIG: &str = ".config/nextest.toml";
const WIN: &str = ".github/workflows/win-gpu.yml";
/// The longest passing test of `win-tests`' workspace step on 11 Nov 2026 (`xtask::qa_TASK-M0-20`), in seconds.
const LONGEST_PASSING: f64 = 342.9;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("ci_win_test_limit: {rel}: {e}"))
}

#[cfg(feature = "controls")]
fn edit(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "ci_win_test_limit: a control's edit is gone"
    );
    text.replacen(from, to, 1)
}

/// Each `[[profile.ci.overrides]]` table of the config, as its `key = value` lines.
fn ci_overrides(config: &str) -> Vec<Vec<(String, String)>> {
    let mut out: Vec<Vec<(String, String)>> = Vec::new();
    let mut inside = false;
    for line in config.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            inside = l == "[[profile.ci.overrides]]";
            if inside {
                out.push(Vec::new());
            }
        } else if inside && !l.starts_with('#') {
            if let Some((k, v)) = l.split_once(" = ") {
                out.last_mut()
                    .expect("a table")
                    .push((k.to_owned(), v.to_owned()));
            }
        }
    }
    out
}

/// Seconds a `slow-timeout = { period = "<n>s", terminate-after = <m> }` ends a test at.
fn ends_at(value: &str) -> Option<f64> {
    let period = value
        .split("period = \"")
        .nth(1)?
        .split("s\"")
        .next()?
        .parse::<f64>()
        .ok()?;
    let after = value
        .split("terminate-after = ")
        .nth(1)?
        .trim_end_matches([' ', '}'])
        .parse::<f64>()
        .ok()?;
    Some(period * after)
}

fn check_limit(config: &str, win: &str) {
    let windows: Vec<Vec<(String, String)>> = ci_overrides(config)
        .into_iter()
        .filter(|o| o.iter().any(|(k, _)| k == "slow-timeout"))
        .collect();
    assert_eq!(
        windows.len(),
        1,
        "ci_win_test_limit: the ci profile has {} overrides with a slow-timeout",
        windows.len()
    );
    let get = |k: &str| {
        windows[0]
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    assert_eq!(
        get("platform"),
        "\"cfg(windows)\"",
        "ci_win_test_limit: the hang limit is not on Windows alone"
    );
    assert_eq!(
        get("filter"),
        "\"not (binary_id(=xtask::jobq) | binary_id(/^xtask::qa_R-658/))\"",
        "ci_win_test_limit: the hang limit is not over every test but the job queue's"
    );
    let at = ends_at(&get("slow-timeout")).unwrap_or_else(|| {
        panic!(
            "ci_win_test_limit: the limit ends no test: {}",
            get("slow-timeout")
        )
    });
    assert!(
        (1.5 * LONGEST_PASSING..=1800.0).contains(&at),
        "ci_win_test_limit: a test is ended at {at} s, outside 1.5 times the longest passing one and 30 minutes"
    );
    let job = win
        .split("\n  win-tests:\n")
        .nth(1)
        .and_then(|rest| rest.split("\n  win-").next())
        .expect("ci_win_test_limit: win-gpu.yml has no win-tests job");
    let step = job
        .split("      - name: cargo nextest run --workspace\n")
        .nth(1)
        .and_then(|rest| rest.split("\n      - ").next())
        .expect("ci_win_test_limit: win-tests has no workspace step");
    assert!(
        step.contains("          NEXTEST_PROFILE: ci\n"),
        "ci_win_test_limit: win-tests' workspace step does not run the ci profile"
    );
}

#[test]
fn ci_win_test_limit_ends_a_hung_test_on_windows() {
    check_limit(&read(CONFIG), &read(WIN));
}

negative_control!(
    ci_win_test_limit_ends_a_hung_test_on_windows,
    "The limit on every platform, the hosted Linux runners' slow tests among them.",
    expected = "the hang limit is not on Windows alone",
    check_limit(
        &edit(&read(CONFIG), "platform = \"cfg(windows)\"\n", ""),
        &read(WIN)
    )
);

mod too_tight {
    use super::*;
    negative_control!(
        ci_win_test_limit_ends_a_hung_test_on_windows,
        "A limit of 5 minutes, under the PC's longest passing test.",
        expected = "a test is ended at 300 s",
        check_limit(
            &edit(
                &read(CONFIG),
                "terminate-after = 15 }",
                "terminate-after = 5 }"
            ),
            &read(WIN)
        )
    );
}

mod over_the_queue {
    use super::*;
    negative_control!(
        ci_win_test_limit_ends_a_hung_test_on_windows,
        "The limit over the job queue's tests too, in place of their tighter one.",
        expected = "the hang limit is not over every test but the job queue's",
        check_limit(
            &edit(
                &read(CONFIG),
                "filter = \"not (binary_id(=xtask::jobq) | binary_id(/^xtask::qa_R-658/))\"",
                "filter = \"all()\""
            ),
            &read(WIN)
        )
    );
}

mod another_profile {
    use super::*;
    negative_control!(
        ci_win_test_limit_ends_a_hung_test_on_windows,
        "win-tests' workspace step under another profile, which has no limit.",
        expected = "win-tests' workspace step does not run the ci profile",
        check_limit(
            &read(CONFIG),
            &edit(
                &read(WIN),
                "      - name: cargo nextest run --workspace\n        if: ${{ !cancelled() && steps.build.outcome == 'success' }}\n        env:\n          NEXTEST_PROFILE: ci\n",
                "      - name: cargo nextest run --workspace\n        if: ${{ !cancelled() && steps.build.outcome == 'success' }}\n        env:\n          NEXTEST_PROFILE: default\n"
            )
        )
    );
}
