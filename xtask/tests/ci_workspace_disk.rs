//! `ci-workspace`'s runner disk (the tooling task `ci-workspace-disk`, applied per R-369): the job ran out of space on a
//! pull request that only added xtask test files, so before any cargo command it runs, it writes a cargo config into the
//! runner's CARGO_HOME that builds every dependency without debug info, in the `dev` and the `test` profile, and leaves
//! the workspace's own crates their line tables (R-374); and its nextest run is under the disk sampler
//! (`.github/scripts/disk_sampler.sh`). Read as text from `.github/workflows/ci.yml`; each check has its negative
//! control (R-176).

use std::path::Path;

use validation::negative_control;

const JOB: &str = "ci-workspace";
const STEP: &str = "- name: Dependencies without debug info, dev and test";
const CARGO_HOME_CONFIG: &str = "\"${CARGO_HOME:-$HOME/.cargo}/config.toml\"";
const NEXTEST: &str = "- name: cargo nextest run -p xtask";
const SAMPLER_SHELL: &str = "shell: bash .github/scripts/disk_sampler.sh {0}";
/// The profiles the job's cargo runs build in: `dev` (`cargo build`, `cargo run`, the `cargo xtask` alias) and `test`
/// (`cargo test`, nextest).
const PROFILES: [&str; 2] = ["dev", "test"];

fn ci_yml() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root");
    std::fs::read_to_string(root.join(".github/workflows/ci.yml")).expect("ci.yml readable")
}

/// A line with its YAML comment removed (a `#` at the line's start or after whitespace).
fn uncommented(l: &str) -> &str {
    let cut = l
        .char_indices()
        .find(|&(i, c)| c == '#' && (i == 0 || l[..i].ends_with(char::is_whitespace)))
        .map_or(l.len(), |(i, _)| i);
    l[..cut].trim_end()
}

/// The non-empty uncommented lines of job `job`: from its header, at two spaces' indent, to the next job's header or
/// the next top-level key.
fn job_lines<'a>(text: &'a str, job: &str) -> Vec<&'a str> {
    let header = format!("  {job}:");
    let mut out = Vec::new();
    let mut inside = false;
    for l in text.lines().map(uncommented) {
        if l.is_empty() {
            continue;
        }
        let is_job_header = l.starts_with("  ") && !l.starts_with("   ") && l.ends_with(':');
        if !l.starts_with(' ') || is_job_header {
            inside = l == header;
            continue;
        }
        if inside {
            out.push(l);
        }
    }
    out
}

/// The lines of the step at `at` in `lines` after its first: up to the next step.
fn step_body<'a>(lines: &[&'a str], at: usize) -> Vec<&'a str> {
    lines[at + 1..]
        .iter()
        .take_while(|l| !l.trim_start().starts_with("- "))
        .copied()
        .collect()
}

fn position(lines: &[&str], what: &str, pred: impl Fn(&str) -> bool) -> usize {
    lines
        .iter()
        .position(|l| pred(l))
        .unwrap_or_else(|| panic!("{JOB} has no {what}"))
}

/// The config the step writes, its `\n` escapes read as printf reads them, and the file it appends to.
fn written_config(lines: &[&str], at: usize) -> (String, String) {
    let run = step_body(lines, at)
        .iter()
        .find_map(|l| l.trim().strip_prefix("run: printf '"))
        .unwrap_or_else(|| panic!("the step `{STEP}` has no `run: printf '…'` line"));
    let (payload, rest) = run
        .split_once('\'')
        .expect("the printf format is single-quoted");
    let target = rest
        .trim()
        .strip_prefix(">>")
        .expect("the step appends (>>) to a file")
        .trim()
        .to_owned();
    (payload.replace("\\n", "\n"), target)
}

/// The step is in `ci-workspace`, after the cache restore and before the job's first cargo command, and writes, into
/// the runner's CARGO_HOME config, `debug = false` for the dependencies (`package."*"`) of the dev and the test
/// profile, and nothing else.
fn check_step(ci: &str) {
    let lines = job_lines(ci, JOB);
    let cache = position(&lines, "rust-cache step", |l| {
        l.contains("uses: Swatinem/rust-cache@")
    });
    let step = position(&lines, &format!("step `{STEP}`"), |l| l.trim() == STEP);
    let build = position(&lines, "cargo command", |l| {
        l.trim().starts_with("run: cargo ")
    });
    assert!(
        cache < step,
        "the config step runs before rust-cache, so the cache step could see it"
    );
    assert!(
        step < build,
        "the config step runs after the job's first cargo command, so that build never reads it"
    );

    let (payload, target) = written_config(&lines, step);
    assert_eq!(
        target, CARGO_HOME_CONFIG,
        "the dependency strip is not written to the runner's CARGO_HOME config"
    );
    let mut stripped = Vec::new();
    let mut table: Option<String> = None;
    for l in payload.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(header) = l.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
            let profile = header
                .strip_prefix("profile.")
                .and_then(|h| h.strip_suffix(".package.\"*\""));
            assert!(
                profile.is_some_and(|p| PROFILES.contains(&p)),
                "`[{header}]` is not the dependency-only table of the dev or test profile"
            );
            table = profile.map(str::to_owned);
        } else {
            let profile = table
                .clone()
                .unwrap_or_else(|| panic!("`{l}` sits outside any table"));
            let squeezed: String = l.chars().filter(|c| !c.is_whitespace()).collect();
            assert_eq!(
                squeezed, "debug=false",
                "the dependency strip sets `{l}`, not `debug = false`"
            );
            stripped.push(profile);
        }
    }
    for p in PROFILES {
        assert!(
            stripped.iter().any(|s| s == p),
            "the {p} profile's dependencies keep their debug info"
        );
    }
}

#[test]
fn ci_workspace_builds_dependencies_without_debug_info() {
    check_step(&ci_yml());
}

/// The byte range, in `ci`, of the step whose first line is `name` in job `ci-workspace`: from that line to the next
/// line at the step's indent or less (the next step, a comment above it, the next job). Another job's step of the
/// same name or text is outside it.
#[cfg(feature = "controls")]
fn step_range(ci: &str, name: &str) -> std::ops::Range<usize> {
    let header = format!("\n  {JOB}:\n");
    let mut at = ci
        .find(&header)
        .unwrap_or_else(|| panic!("ci.yml has no job {JOB}"))
        + header.len();
    let indent = |l: &str| l.len() - l.trim_start().len();
    let mut start: Option<(usize, usize)> = None;
    for line in ci[at..].split_inclusive('\n') {
        let l = line.trim_end();
        if !l.is_empty() {
            match start {
                None if indent(l) <= 2 => break,
                None if l.trim() == name => start = Some((at, indent(l))),
                Some((from, step)) if indent(l) <= step => return from..at,
                _ => {}
            }
        }
        at += line.len();
    }
    let (from, _) = start.unwrap_or_else(|| panic!("{JOB} has no step `{name}`"));
    from..at
}

/// `ci.yml` with `from` replaced by `to`, once, inside `ci-workspace`'s step `name`; an edit that no longer applies
/// there stops the control.
#[cfg(feature = "controls")]
fn edited(name: &str, from: &str, to: &str) -> String {
    let ci = ci_yml();
    let r = step_range(&ci, name);
    let step = &ci[r.clone()];
    assert!(
        step.contains(from),
        "{JOB}'s step `{name}` has no `{from}` to edit"
    );
    format!(
        "{}{}{}",
        &ci[..r.start],
        step.replacen(from, to, 1),
        &ci[r.end..]
    )
}

/// `ci-workspace`'s config step, as ci.yml holds it.
#[cfg(feature = "controls")]
fn step_text(ci: &str) -> String {
    ci[step_range(ci, STEP)].to_owned()
}

negative_control!(
    ci_workspace_builds_dependencies_without_debug_info,
    "a config step that strips the test profile alone, leaving `cargo run`'s dev builds their dependencies' line tables",
    expected = "the dev profile's dependencies keep their debug info",
    check_step(&edited(STEP, "[profile.dev.package.\"*\"]\\ndebug = false\\n", ""))
);

#[cfg(feature = "controls")]
mod whole_profile {
    use super::*;

    negative_control!(
        ci_workspace_builds_dependencies_without_debug_info,
        "a strip that takes the workspace's own crates' line tables as well (R-374)",
        expected = "is not the dependency-only table",
        check_step(&edited(
            STEP,
            "[profile.test.package.\"*\"]",
            "[profile.test]"
        ))
    );
}

#[cfg(feature = "controls")]
mod other_key {
    use super::*;

    negative_control!(
        ci_workspace_builds_dependencies_without_debug_info,
        "a strip that also changes the dependencies' optimisation",
        expected = "not `debug = false`",
        check_step(&edited(
            STEP,
            "[profile.test.package.\"*\"]\\ndebug = false\\n",
            "[profile.test.package.\"*\"]\\ndebug = false\\nopt-level = 1\\n",
        ))
    );
}

#[cfg(feature = "controls")]
mod repository_config {
    use super::*;

    negative_control!(
        ci_workspace_builds_dependencies_without_debug_info,
        "a strip written into the repository's .cargo/config.toml, which local builds read (R-233)",
        expected = "not written to the runner's CARGO_HOME config",
        {
            let ci = ci_yml();
            let step = step_text(&ci);
            check_step(&ci.replacen(
                &step,
                &step.replace(CARGO_HOME_CONFIG, "\".cargo/config.toml\""),
                1,
            ))
        }
    );
}

#[cfg(feature = "controls")]
mod after_build {
    use super::*;

    negative_control!(
        ci_workspace_builds_dependencies_without_debug_info,
        "the config step moved just below the nextest run, the job's first cargo command",
        expected = "runs after the job's first cargo command",
        {
            let ci = ci_yml();
            let step = step_text(&ci);
            let run = "        run: cargo nextest run -p xtask\n";
            assert!(ci.contains(run));
            check_step(
                &ci.replacen(&step, "", 1)
                    .replacen(run, &format!("{run}{step}"), 1),
            )
        }
    );
}

#[cfg(feature = "controls")]
mod before_cache {
    use super::*;

    negative_control!(
        ci_workspace_builds_dependencies_without_debug_info,
        "the config step moved just above rust-cache",
        expected = "runs before rust-cache",
        {
            let ci = ci_yml();
            let step = step_text(&ci);
            let cache = "      - uses: Swatinem/rust-cache@v2\n        with:\n          prefix-key: cargo-registry\n          \
                         shared-key: ci-workspace\n";
            assert!(ci.contains(cache));
            check_step(
                &ci.replacen(&step, "", 1)
                    .replacen(cache, &format!("{step}{cache}"), 1),
            )
        }
    );
}

#[cfg(feature = "controls")]
mod other_job {
    use super::*;

    negative_control!(
        ci_workspace_builds_dependencies_without_debug_info,
        "the config step in another job, not ci-workspace",
        expected =
            "ci-workspace has no step `- name: Dependencies without debug info, dev and test`",
        {
            let ci = ci_yml();
            let step = step_text(&ci);
            check_step(&ci.replacen(&step, "", 1).replacen(
                "\n  ci-checks:\n",
                &format!("\n  extra:\n    steps:\n{step}  ci-checks:\n"),
                1,
            ))
        }
    );
}

/// The nextest run of `ci-workspace` has the disk sampler as its shell.
fn check_sampler(ci: &str) {
    let lines = job_lines(ci, JOB);
    let at = position(&lines, &format!("step `{NEXTEST}`"), |l| {
        l.trim() == NEXTEST
    });
    assert!(
        step_body(&lines, at)
            .iter()
            .any(|l| l.trim() == SAMPLER_SHELL),
        "{JOB}'s nextest run is not under the disk sampler"
    );
}

#[test]
fn ci_workspace_tests_run_under_the_disk_sampler() {
    check_sampler(&ci_yml());
}

negative_control!(
    ci_workspace_tests_run_under_the_disk_sampler,
    "the nextest run back on the default shell",
    expected = "nextest run is not under the disk sampler",
    check_sampler(&edited(
        NEXTEST,
        "        run: cargo nextest run -p xtask\n        shell: bash .github/scripts/disk_sampler.sh {0}\n",
        "        run: cargo nextest run -p xtask\n",
    ))
);
