//! qa for the tooling task `ci-workspace-disk` (applied per R-369), written from the task, not from the step's text:
//!
//! 1. the `ci-workspace` step that strips the dependencies' debug info, run as the runner's bash runs it, writes a
//!    cargo config that a TOML reader takes to say exactly: the dependencies (`package."*"`) of the `dev` and `test`
//!    profiles carry no debug info; and it appends, keeping whatever the runner's CARGO_HOME config held;
//! 2. the workspace's own crates keep their line tables (R-374): the workflow's `CARGO_PROFILE_{DEV,TEST}_DEBUG` stay
//!    `line-tables-only` and `ci-workspace` overrides neither;
//! 3. the step precedes every step of the job that runs cargo, a multi-line `run: |` block among them;
//! 4. the nextest run is under the disk sampler, the sampler exists, and a later step logs the free space at the end.
//!
//! Each check has its negative control (R-176).

use std::path::{Path, PathBuf};
use std::process::Command;

use toml_edit::{DocumentMut, Item, Value};
use validation::negative_control;

const JOB: &str = "ci-workspace";
const STEP_NAME: &str = "Dependencies without debug info, dev and test";
const NEXTEST_RUN: &str = "cargo nextest run -p xtask";
const SAMPLER: &str = ".github/scripts/disk_sampler.sh";
/// What a runner's CARGO_HOME config might already hold, which the step must keep.
const EXISTING: &str = "[net]\nretry = 3\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn ci_yml() -> String {
    std::fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("ci.yml readable")
}

/// A YAML line without its comment (a `#` at the start or after whitespace), right-trimmed.
fn strip_comment(l: &str) -> &str {
    let cut = l
        .char_indices()
        .find(|&(i, c)| c == '#' && (i == 0 || l[..i].ends_with(char::is_whitespace)))
        .map_or(l.len(), |(i, _)| i);
    l[..cut].trim_end()
}

/// The non-empty, uncommented lines of job `job` (header at two spaces' indent, up to the next such header or a
/// top-level key).
fn job(text: &str, job: &str) -> Vec<String> {
    let header = format!("  {job}:");
    let mut inside = false;
    let mut out = Vec::new();
    for l in text.lines().map(strip_comment).filter(|l| !l.is_empty()) {
        let indent = l.len() - l.trim_start().len();
        if indent < 2 || (indent == 2 && l.ends_with(':')) {
            inside = l == header;
            continue;
        }
        if inside {
            out.push(l.to_owned());
        }
    }
    out
}

/// The job's steps: each the list of its lines, starting at its `- ` line under `steps:`.
fn steps(lines: &[String]) -> Vec<Vec<String>> {
    let at = lines
        .iter()
        .position(|l| l.trim() == "steps:")
        .unwrap_or_else(|| panic!("{JOB} has no steps"));
    let indent = |l: &str| l.len() - l.trim_start().len();
    let step_indent = indent(&lines[at + 1]);
    let mut out: Vec<Vec<String>> = Vec::new();
    for l in &lines[at + 1..] {
        if indent(l) == step_indent && l.trim_start().starts_with("- ") {
            out.push(vec![l.clone()]);
        } else if indent(l) > step_indent {
            out.last_mut().expect("a step").push(l.clone());
        } else {
            break;
        }
    }
    out
}

fn named(step: &[String], name: &str) -> bool {
    step.first()
        .is_some_and(|l| l.trim_start().trim_start_matches("- ").trim() == format!("name: {name}"))
}

/// The step's `run:` script: the one-line value, or the lines of a `run: |` block.
fn run_script(step: &[String]) -> Option<String> {
    let at = step.iter().position(|l| {
        let t = l.trim_start().trim_start_matches("- ");
        t.starts_with("run:")
    })?;
    let t = step[at].trim_start().trim_start_matches("- ");
    let value = t["run:".len()..].trim();
    if value == "|" || value == "|-" {
        let indent = |l: &str| l.len() - l.trim_start().len();
        let key = indent(&step[at]);
        Some(
            step[at + 1..]
                .iter()
                .take_while(|l| indent(l) > key)
                .map(|l| l.trim())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    } else {
        Some(value.to_owned())
    }
}

/// Whether a script runs cargo: a `cargo` word in it.
fn runs_cargo(script: &str) -> bool {
    script
        .split(|c: char| c.is_whitespace() || c == ';' || c == '&' || c == '|' || c == '(')
        .any(|w| w == "cargo")
}

fn strip_step(ci: &str) -> Vec<String> {
    steps(&job(ci, JOB))
        .into_iter()
        .find(|s| named(s, STEP_NAME))
        .unwrap_or_else(|| panic!("{JOB} has no step `{STEP_NAME}`"))
}

/// A cargo `debug` value that means no debug info (the cargo book, "Profiles", `debug`: `false`, `0`, `"none"`).
fn no_debug(v: &Value) -> bool {
    v.as_bool() == Some(false) || v.as_integer() == Some(0) || v.as_str() == Some("none")
}

/// Runs `script` with bash, CARGO_HOME at a fresh directory holding `EXISTING` as its config, and returns the
/// config it leaves.
fn run_in_cargo_home(script: &str, tag: &str) -> String {
    let home =
        std::env::temp_dir().join(format!("qa-ci-workspace-disk-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let config = home.join("config.toml");
    std::fs::write(&config, EXISTING).unwrap();
    let status = Command::new("bash")
        .arg("-e")
        .arg("-c")
        .arg(script)
        .env("CARGO_HOME", &home)
        .status()
        .expect("bash runs");
    assert!(
        status.success(),
        "the step's script fails under bash: {status}"
    );
    let text = std::fs::read_to_string(&config).expect("the config is still there");
    let _ = std::fs::remove_dir_all(&home);
    text
}

/// Check 1: what the step writes, read as TOML.
fn check_config(ci: &str, tag: &str) {
    let script = run_script(&strip_step(ci)).expect("the strip step has a run");
    let text = run_in_cargo_home(&script, tag);
    let doc: DocumentMut = text.parse().unwrap_or_else(|e| {
        panic!("the CARGO_HOME config is not valid TOML after the step: {e}\n{text}")
    });
    assert_eq!(
        doc.get("net")
            .and_then(|n| n.get("retry"))
            .and_then(Item::as_integer),
        Some(3),
        "the step lost what the CARGO_HOME config already held"
    );
    let profile = doc
        .get("profile")
        .and_then(Item::as_table_like)
        .expect("the config has no [profile] table");
    let mut names: Vec<&str> = profile.iter().map(|(k, _)| k).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["dev", "test"],
        "the config's profiles are not exactly dev and test"
    );
    for p in ["dev", "test"] {
        let table = profile.get(p).and_then(Item::as_table_like).unwrap();
        let keys: Vec<&str> = table.iter().map(|(k, _)| k).collect();
        assert_eq!(
            keys,
            ["package"],
            "the {p} profile's own settings are changed, the workspace's crates with them"
        );
        let package = table.get("package").and_then(Item::as_table_like).unwrap();
        let pkgs: Vec<&str> = package.iter().map(|(k, _)| k).collect();
        assert_eq!(
            pkgs,
            ["*"],
            "the {p} profile overrides named packages, not every dependency"
        );
        let deps = package.get("*").and_then(Item::as_table_like).unwrap();
        let settings: Vec<&str> = deps.iter().map(|(k, _)| k).collect();
        assert_eq!(
            settings,
            ["debug"],
            "the {p} dependencies' override sets more than debug"
        );
        let debug = deps.get("debug").and_then(Item::as_value).unwrap();
        assert!(
            no_debug(debug),
            "the {p} profile's dependencies keep debug info: debug = {debug}"
        );
    }
}

#[test]
fn the_strip_step_writes_dependency_only_no_debug_for_dev_and_test() {
    check_config(&ci_yml(), "real");
}

/// `ci.yml` with the strip step's `run:` line replaced by `run`.
#[cfg(feature = "controls")]
fn with_run(run: &str) -> String {
    let ci = ci_yml();
    let old = format!("run: {}", run_script(&strip_step(&ci)).unwrap());
    assert!(ci.contains(&old));
    ci.replacen(&old, &format!("run: {run}"), 1)
}

negative_control!(
    the_strip_step_writes_dependency_only_no_debug_for_dev_and_test,
    "a step that overwrites (>) the runner's CARGO_HOME config instead of appending",
    expected = "lost what the CARGO_HOME config already held",
    check_config(
        &with_run(
            "printf '[profile.dev.package.\"*\"]\\ndebug = false\\n[profile.test.package.\"*\"]\\ndebug = false\\n' > \"${CARGO_HOME:-$HOME/.cargo}/config.toml\""
        ),
        "clobber"
    )
);

#[cfg(feature = "controls")]
mod line_tables {
    use super::*;
    negative_control!(
        the_strip_step_writes_dependency_only_no_debug_for_dev_and_test,
        "dependencies left at line tables in the test profile",
        expected = "test profile's dependencies keep debug info",
        check_config(
            &with_run(
                "printf '[profile.dev.package.\"*\"]\\ndebug = false\\n[profile.test.package.\"*\"]\\ndebug = \"line-tables-only\"\\n' >> \"${CARGO_HOME:-$HOME/.cargo}/config.toml\""
            ),
            "lines"
        )
    );
}

#[cfg(feature = "controls")]
mod whole_profile {
    use super::*;
    negative_control!(
        the_strip_step_writes_dependency_only_no_debug_for_dev_and_test,
        "the dev profile stripped whole, the workspace's crates included",
        expected = "dev profile's own settings are changed",
        check_config(
            &with_run(
                "printf '[profile.dev]\\ndebug = false\\n[profile.test.package.\"*\"]\\ndebug = false\\n' >> \"${CARGO_HOME:-$HOME/.cargo}/config.toml\""
            ),
            "whole"
        )
    );
}

#[cfg(feature = "controls")]
mod dev_missing {
    use super::*;
    negative_control!(
        the_strip_step_writes_dependency_only_no_debug_for_dev_and_test,
        "only the test profile's dependencies stripped",
        expected = "profiles are not exactly dev and test",
        check_config(
            &with_run(
                "printf '[profile.test.package.\"*\"]\\ndebug = false\\n' >> \"${CARGO_HOME:-$HOME/.cargo}/config.toml\""
            ),
            "devmissing"
        )
    );
}

#[cfg(feature = "controls")]
mod broken_toml {
    use super::*;
    negative_control!(
        the_strip_step_writes_dependency_only_no_debug_for_dev_and_test,
        "a table header missing its closing bracket",
        expected = "not valid TOML",
        check_config(
            &with_run(
                "printf '[profile.dev.package.\"*\"\\ndebug = false\\n[profile.test.package.\"*\"]\\ndebug = false\\n' >> \"${CARGO_HOME:-$HOME/.cargo}/config.toml\""
            ),
            "broken"
        )
    );
}

/// Check 2: R-374's line tables for the workspace's own crates are still what the job builds with.
fn check_line_tables(ci: &str) {
    for key in ["CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG"] {
        let top = ci
            .lines()
            .map(strip_comment)
            .find(|l| {
                l.trim_start().starts_with(&format!("{key}:"))
                    && l.starts_with("  ")
                    && !l.starts_with("   ")
            })
            .unwrap_or_else(|| panic!("the workflow env has no {key}"));
        assert_eq!(
            top.trim(),
            format!("{key}: line-tables-only"),
            "the workflow's {key} is not line-tables-only"
        );
        assert!(
            !job(ci, JOB).iter().any(|l| l.contains(key)),
            "{JOB} overrides {key}, so the workspace's crates lose their line tables"
        );
    }
}

#[test]
fn the_workspace_crates_keep_their_line_tables() {
    check_line_tables(&ci_yml());
}

negative_control!(
    the_workspace_crates_keep_their_line_tables,
    "ci-workspace setting CARGO_PROFILE_TEST_DEBUG in its own env",
    expected = "overrides CARGO_PROFILE_TEST_DEBUG",
    {
        let ci = ci_yml();
        let from = "  ci-workspace:\n    runs-on: ubuntu-24.04\n    env:\n";
        assert!(ci.contains(from));
        check_line_tables(&ci.replacen(
            from,
            &format!("{from}      CARGO_PROFILE_TEST_DEBUG: \"false\"\n"),
            1,
        ))
    }
);

/// Check 3: no step of the job runs cargo before the strip step.
fn check_order(ci: &str) {
    let all = steps(&job(ci, JOB));
    let strip = all
        .iter()
        .position(|s| named(s, STEP_NAME))
        .unwrap_or_else(|| panic!("{JOB} has no step `{STEP_NAME}`"));
    assert!(
        all.iter()
            .skip(strip + 1)
            .any(|s| run_script(s).is_some_and(|r| runs_cargo(&r))),
        "no cargo step follows the strip step"
    );
    if let Some(early) = all[..strip]
        .iter()
        .find(|s| run_script(s).is_some_and(|r| runs_cargo(&r)))
    {
        panic!(
            "a step runs cargo before the strip step: {}",
            early.join("\n")
        );
    }
}

#[test]
fn the_strip_step_precedes_every_cargo_step() {
    check_order(&ci_yml());
}

negative_control!(
    the_strip_step_precedes_every_cargo_step,
    "a multi-line run block that fetches with cargo, put before the strip step",
    expected = "runs cargo before the strip step",
    {
        let ci = ci_yml();
        let from = format!("      - name: {STEP_NAME}\n");
        assert!(ci.contains(&from));
        check_order(&ci.replacen(
            &from,
            &format!("      - name: fetch\n        run: |\n          echo fetching\n          cargo fetch --locked\n{from}"),
            1,
        ))
    }
);

/// Check 4: the nextest run is sampled by a sampler that exists, and a later step logs the free space.
fn check_sampled(ci: &str) {
    let all = steps(&job(ci, JOB));
    let at = all
        .iter()
        .position(|s| run_script(s).as_deref() == Some(NEXTEST_RUN))
        .unwrap_or_else(|| panic!("{JOB} has no `{NEXTEST_RUN}` step"));
    let shell = all[at]
        .iter()
        .find_map(|l| l.trim().strip_prefix("shell:"))
        .map(str::trim)
        .unwrap_or_else(|| panic!("{JOB}'s nextest run has the default shell, no disk sampler"));
    let script = shell
        .strip_prefix("bash ")
        .and_then(|s| s.strip_suffix(" {0}"))
        .unwrap_or_else(|| panic!("the shell `{shell}` is not `bash <sampler> {{0}}`"));
    assert!(
        root().join(script).is_file(),
        "{script} is not in the repository"
    );
    assert_eq!(
        script, SAMPLER,
        "the nextest run's shell is not the disk sampler"
    );
    assert!(
        all[at + 1..]
            .iter()
            .any(|s| run_script(s).is_some_and(|r| r.split_whitespace().next() == Some("df"))),
        "no step after the nextest run logs the runner's free space"
    );
}

#[test]
fn the_nextest_run_is_sampled_and_the_end_free_space_logged() {
    check_sampled(&ci_yml());
}

negative_control!(
    the_nextest_run_is_sampled_and_the_end_free_space_logged,
    "the free-space step after the tests removed",
    expected = "logs the runner's free space",
    {
        let ci = ci_yml();
        let from = "      - name: Runner disk after the tests\n        if: needs.changes.outputs.docs_only != 'true'\n        run: df -h /\n";
        assert!(ci.contains(from));
        check_sampled(&ci.replacen(from, "", 1))
    }
);

#[cfg(feature = "controls")]
mod sampler_missing {
    use super::*;
    negative_control!(
        the_nextest_run_is_sampled_and_the_end_free_space_logged,
        "the nextest shell pointing at a sampler the repository does not hold",
        expected = "is not in the repository",
        check_sampled(&ci_yml().replacen(
            "        run: cargo nextest run -p xtask\n        shell: bash .github/scripts/disk_sampler.sh {0}\n",
            "        run: cargo nextest run -p xtask\n        shell: bash .github/scripts/no_such_sampler.sh {0}\n",
            1,
        ))
    );
}
