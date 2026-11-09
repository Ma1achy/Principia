//! qa's tests for the PR tier (tooling/pr-tier-restructure), written from the owner's CI model, approved 9 Nov 2026,
//! item 1: "Every PR (required; target < 30 min), in parallel, fmt first (fail fast) - fmt, Clippy, workspace build.
//! Product tests: changed crates + reverse deps (ci_quick_crates.py selection), excluding the slow tier … and in-depth
//! GPU tests. Product-rule commands run directly …: `cargo xtask deps`, `lint constants`, `lint wgsl`, `lint vocab`,
//! generated-WGSL-is-current, static 'every test has a registered control' listing (no control execution). lavapipe
//! GPU smoke set; Metal smoke set on GPU-path PRs. check_plan.py, reviews-complete, pr-check … Process tests only when
//! the PR changes .github/, xtask/, scripts/, plan/, decisions.md, REVIEW_QUEUE.md or docs/. `ci` no longer runs on
//! task-branch push events: pull_request and main push only. Required checks drop mutants-check, xtask-ci, full-tier
//! GPU jobs", and the owner's rider: "Golden runner over every suite (qa_TASK-M0-06_runner, _list) and the screenshot
//! runner self-test (qa_TASK-M0-20): product correctness - per merge, never retired."
//!
//! The PR tier's required checks are fmt, clippy, build, ci, gpu-lavapipe, gpu-metal, pr-check and reviews-complete.
//! Each test registers a negative control (R-176) that feeds its check the checked-in file changed in the one respect
//! the test turns on.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;

/// The required checks `ci.yml` bears.
const CI_REQUIRED: [&str; 6] = ["fmt", "clippy", "build", "ci", "gpu-lavapipe", "gpu-metal"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa pr tier: {rel}: {e}"))
}

fn ci() -> String {
    read(".github/workflows/ci.yml")
}

/// `text` with its first `from` replaced by `to`; a control whose target is gone fails here, not silently.
#[cfg(feature = "controls")]
fn swap(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa pr tier: control target {from:?} gone"
    );
    text.replacen(from, to, 1)
}

/// The jobs of a workflow, as (id, the job's lines below its id line), comments kept.
fn jobs(workflow: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#') {
            in_jobs = line.trim_end() == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        if line.starts_with("  ")
            && !line[2..].starts_with([' ', '#'])
            && line.trim_end().ends_with(':')
        {
            out.push((line.trim().trim_end_matches(':').to_owned(), Vec::new()));
        } else if let Some((_, lines)) = out.last_mut() {
            lines.push(line.to_owned());
        }
    }
    out
}

/// The check a job reports: its own `name:` (four spaces in), or its id.
fn check_name(id: &str, lines: &[String]) -> String {
    lines
        .iter()
        .find_map(|l| l.strip_prefix("    name: "))
        .map(|n| n.trim().trim_matches('"').to_owned())
        .unwrap_or_else(|| id.to_owned())
}

/// The job's own `if:` (four spaces in), or "" when it has none.
fn job_if(lines: &[String]) -> String {
    lines
        .iter()
        .find_map(|l| l.strip_prefix("    if: "))
        .unwrap_or("")
        .trim()
        .to_owned()
}

/// The one job of `workflow` reporting check `name`, as (id, lines); fails unless exactly one does.
fn the_job(workflow: &str, file: &str, name: &str) -> (String, Vec<String>) {
    let found: Vec<(String, Vec<String>)> = jobs(workflow)
        .into_iter()
        .filter(|(id, l)| check_name(id, l) == name)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "qa pr tier: {} jobs of {file} report the check `{name}`",
        found.len()
    );
    found.into_iter().next().unwrap()
}

/// The commands a job runs: every `run:` value, a block scalar's lines joined, comments dropped.
fn runs(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim().trim_start_matches("- ").trim();
        if let Some(cmd) = t.strip_prefix("run:") {
            let cmd = cmd.trim();
            if cmd == "|" || cmd == ">" {
                let base = lines[i].len() - lines[i].trim_start().len();
                let mut block = Vec::new();
                i += 1;
                while i < lines.len()
                    && (lines[i].trim().is_empty()
                        || lines[i].len() - lines[i].trim_start().len() > base)
                {
                    block.push(lines[i].trim().to_owned());
                    i += 1;
                }
                out.push(block.join("\n"));
                continue;
            }
            out.push(cmd.trim_matches('\'').to_owned());
        }
        i += 1;
    }
    out
}

/// Whether the workflow's `on:` block names `pull_request` as a trigger.
fn triggers_on_pull_request(workflow: &str) -> bool {
    let mut in_on = false;
    for line in workflow.lines() {
        if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#') {
            in_on = line.trim_end() == "on:" || line.starts_with("on: ");
            if line.starts_with("on: ") && line.contains("pull_request") {
                return true;
            }
            continue;
        }
        if in_on && line.trim_start().starts_with("pull_request") && !line.contains("pull_request_")
        {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------------------------------------------------
// The required checks of ci.yml, pr-check.yml and reviews.yml report on every pull request (no-atlas-params.yml's,
// the ninth, is `qa_mutants_check_always.rs`'s).

/// Each required check is one job, of a workflow triggered by `pull_request`, whose own `if:` never leaves a pull
/// request out: a check skipped at job level, or reported by two jobs, cannot be trusted by branch protection.
fn check_required_report(ci: &str, pr_check: &str, reviews: &str) {
    for (file, text, names) in [
        ("ci.yml", ci, &CI_REQUIRED[..]),
        ("pr-check.yml", pr_check, &["pr-check"][..]),
        ("reviews.yml", reviews, &["reviews-complete"][..]),
    ] {
        assert!(
            triggers_on_pull_request(text),
            "qa pr tier: {file} is not triggered by pull_request"
        );
        for name in names {
            let (id, lines) = the_job(text, file, name);
            let cond = job_if(&lines);
            assert!(
                !(cond.contains("== 'push'")
                    || cond.contains("!= 'pull_request'")
                    || cond.contains("needs.changes.outputs")),
                "qa pr tier: `{name}` ({file}'s `{id}`) can be skipped on a pull request at job level: `if: {cond}`"
            );
            // A job-level `continue-on-error` (the job's own key, four spaces in) passes the job whatever its steps end
            // in; a step's own, on a measurement step (a kernel log, a disk sample's upload), fails nothing else.
            assert!(
                !lines
                    .iter()
                    .any(|l| l.starts_with("    continue-on-error:")),
                "qa pr tier: `{name}` ({file}'s `{id}`) can pass with a failing step"
            );
        }
    }
    // `ci.yml`'s `changes` job, which every required job of it needs, runs on every event.
    let (_, changes) = the_job(ci, "ci.yml", "changes");
    assert_eq!(
        job_if(&changes),
        "",
        "qa pr tier: ci.yml's `changes` can be skipped"
    );
}

fn required_report_checked_in() {
    check_required_report(
        &ci(),
        &read(".github/workflows/pr-check.yml"),
        &read(".github/workflows/reviews.yml"),
    );
}

#[test]
fn qa_pr_tier_required_checks_report_on_every_pull_request() {
    required_report_checked_in();
}

negative_control!(
    qa_pr_tier_required_checks_report_on_every_pull_request,
    "ci.yml with `fmt` skipped at job level on a docs-only change",
    expected = "`fmt` (ci.yml's `fmt`) can be skipped on a pull request at job level",
    check_required_report(
        &swap(
            &ci(),
            "  fmt:\n    needs: changes\n    if: always() && github.event_name == 'pull_request'\n",
            "  fmt:\n    needs: changes\n    if: always() && github.event_name == 'pull_request' && needs.changes.outputs.docs_only != 'true'\n",
        ),
        &read(".github/workflows/pr-check.yml"),
        &read(".github/workflows/reviews.yml"),
    )
);

/// A second negative control: a required name borne by two jobs (an interim job renamed onto `gpu-metal`).
#[test]
fn qa_pr_tier_required_check_names_are_unique() {
    required_report_checked_in();
}

negative_control!(
    qa_pr_tier_required_check_names_are_unique,
    "ci.yml with the interim Metal suites renamed onto the required check `gpu-metal`",
    expected = "2 jobs of ci.yml report the check `gpu-metal`",
    check_required_report(
        &swap(
            &ci(),
            "    name: gpu-metal suites\n",
            "    name: gpu-metal\n"
        ),
        &read(".github/workflows/pr-check.yml"),
        &read(".github/workflows/reviews.yml"),
    )
);

// ---------------------------------------------------------------------------------------------------------------------
// fmt first, on its own; clippy over every target with warnings denied.

/// `fmt` runs `cargo fmt --all --check` and nothing that compiles, and waits on nothing but `changes`, so it reports
/// first ("fmt first (fail fast)").
fn check_fmt(ci: &str) {
    let (_, lines) = the_job(ci, "ci.yml", "fmt");
    let cmds = runs(&lines);
    assert!(
        cmds.iter().any(|c| c == "cargo fmt --all --check"),
        "qa pr tier: `fmt` does not run `cargo fmt --all --check`: {cmds:?}"
    );
    for c in &cmds {
        for heavy in [
            "cargo build",
            "cargo test",
            "cargo nextest",
            "cargo clippy",
            "cargo xtask",
            "cargo doc",
        ] {
            assert!(
                !c.contains(heavy),
                "qa pr tier: `fmt` compiles (`{c}`), so it no longer reports first"
            );
        }
    }
    let needs: Vec<&String> = lines
        .iter()
        .filter(|l| l.starts_with("    needs:"))
        .collect();
    assert!(
        needs.len() == 1 && needs[0].trim() == "needs: changes",
        "qa pr tier: `fmt` waits on more than `changes`: {needs:?}"
    );
}

#[test]
fn qa_pr_tier_fmt_reports_first() {
    check_fmt(&ci());
}

negative_control!(
    qa_pr_tier_fmt_reports_first,
    "ci.yml with a workspace build ahead of the format check in `fmt`",
    expected = "`fmt` compiles (`cargo build --workspace`)",
    check_fmt(&swap(
        &ci(),
        "        run: cargo fmt --all --check\n",
        "        run: cargo build --workspace\n      - run: cargo fmt --all --check\n",
    ))
);

fn check_clippy(ci: &str) {
    let (_, lines) = the_job(ci, "ci.yml", "clippy");
    let cmds = runs(&lines);
    assert!(
        cmds.iter()
            .any(|c| c == "cargo clippy --workspace --all-targets -- -D warnings"),
        "qa pr tier: `clippy` does not run `cargo clippy --workspace --all-targets -- -D warnings` (R-195): {cmds:?}"
    );
}

#[test]
fn qa_pr_tier_clippy_denies_warnings_on_every_target() {
    check_clippy(&ci());
}

negative_control!(
    qa_pr_tier_clippy_denies_warnings_on_every_target,
    "ci.yml with clippy's warnings allowed",
    expected = "`clippy` does not run `cargo clippy --workspace --all-targets -- -D warnings`",
    check_clippy(&swap(
        &ci(),
        "        run: cargo clippy --workspace --all-targets -- -D warnings\n",
        "        run: cargo clippy --workspace --all-targets\n",
    ))
);

// ---------------------------------------------------------------------------------------------------------------------
// build: the workspace build and the product-rule commands, run directly.

/// The commands the owner's model puts on every PR, as `build` must run them: the workspace build, the crate graph,
/// the constants, WGSL and vocabulary lints, the plan check, and the static control listing.
const BUILD_COMMANDS: [&str; 7] = [
    "cargo build --workspace",
    "cargo xtask deps",
    "cargo xtask lint constants",
    "cargo xtask lint wgsl",
    "cargo xtask lint vocab",
    "cargo xtask controls --list",
    "python3 plan/check_plan.py",
];

/// `build` runs each product-rule command, checks that `cargo xtask codegen` leaves the tree unchanged (the generated
/// WGSL current), and executes no control: no bare `cargo xtask ci` or `cargo xtask controls` without `--list`.
fn check_build(ci: &str) {
    let (_, lines) = the_job(ci, "ci.yml", "build");
    let cmds = runs(&lines);
    for want in BUILD_COMMANDS {
        assert!(
            cmds.iter().any(|c| c.lines().any(|l| l.trim() == want)),
            "qa pr tier: `build` does not run `{want}`"
        );
    }
    let codegen = cmds
        .iter()
        .find(|c| c.lines().any(|l| l.trim() == "cargo xtask codegen"))
        .unwrap_or_else(|| panic!("qa pr tier: `build` does not run `cargo xtask codegen`"));
    assert!(
        codegen.contains("git status --porcelain") && codegen.contains("exit 1"),
        "qa pr tier: `build`'s codegen step does not fail on a changed tree: {codegen}"
    );
    for c in &cmds {
        for l in c.lines().map(str::trim) {
            assert!(
                !(l == "cargo xtask ci"
                    || l.starts_with("cargo xtask ci ")
                    || (l.starts_with("cargo xtask controls") && !l.contains("--list"))),
                "qa pr tier: `build` executes controls (`{l}`): the PR tier lists them only"
            );
        }
    }
}

#[test]
fn qa_pr_tier_build_runs_the_product_rules() {
    check_build(&ci());
}

negative_control!(
    qa_pr_tier_build_runs_the_product_rules,
    "ci.yml with the WGSL lint dropped from `build`",
    expected = "`build` does not run `cargo xtask lint wgsl`",
    check_build(&swap(
        &ci(),
        "        run: cargo xtask lint wgsl\n",
        "        run: echo lint wgsl skipped\n",
    ))
);

#[test]
fn qa_pr_tier_build_runs_no_control() {
    check_build(&ci());
}

negative_control!(
    qa_pr_tier_build_runs_no_control,
    "ci.yml with `build` running every control in place of the listing",
    expected = "`build` executes controls (`cargo xtask controls`)",
    check_build(&swap(
        &ci(),
        "        run: cargo xtask controls --list\n",
        "        run: |\n          cargo xtask controls --list\n          cargo xtask controls\n",
    ))
);

#[test]
fn qa_pr_tier_build_checks_the_generated_files() {
    check_build(&ci());
}

negative_control!(
    qa_pr_tier_build_checks_the_generated_files,
    "ci.yml with the codegen step's tree check gone",
    expected = "`build`'s codegen step does not fail on a changed tree",
    check_build(
        &swap(
            &ci(),
            "          if [ -n \"$(git status --porcelain)\" ]; then\n",
            "          if false; then\n"
        )
        .replacen("            git status --porcelain\n", "", 1)
    )
);

// ---------------------------------------------------------------------------------------------------------------------
// ci: the product tests of the changed crates and their reverse dependencies, under the `pr` profile.

/// The job bearing `ci` runs nextest under the `pr` profile over `changes`' packages (ci_tier.sh's selection), with
/// their doctests.
fn check_ci_tests(ci: &str) {
    let (id, lines) = the_job(ci, "ci.yml", "ci");
    let text = lines.join("\n");
    assert!(
        text.contains("PACKAGES: ${{ needs.changes.outputs.packages }}")
            && runs(&lines).iter().any(|c| c == "cargo nextest run $PACKAGES"),
        "qa pr tier: `ci` ({id}) does not run nextest over the changed crates and their reverse dependencies"
    );
    let step = text
        .split("      - name: ")
        .find(|s| s.contains("run: cargo nextest run $PACKAGES"))
        .unwrap();
    assert!(
        step.contains("NEXTEST_PROFILE: pr\n"),
        "qa pr tier: `ci` ({id}) runs the changed crates' tests under a profile other than `pr`: {step}"
    );
    assert!(
        text.contains("${{ needs.changes.outputs.doc_packages }}")
            && runs(&lines)
                .iter()
                .any(|c| c == "cargo test $PACKAGES --doc"),
        "qa pr tier: `ci` ({id}) runs no doctests (R-231)"
    );
}

#[test]
fn qa_pr_tier_ci_runs_the_pr_profile_on_the_changed_crates() {
    check_ci_tests(&ci());
}

negative_control!(
    qa_pr_tier_ci_runs_the_pr_profile_on_the_changed_crates,
    "ci.yml with the product tests under the full `ci` profile",
    expected = "runs the changed crates' tests under a profile other than `pr`",
    check_ci_tests(&swap(
        &ci(),
        "          NEXTEST_PROFILE: pr\n          NEXTEST_NO_TESTS: pass\n",
        "          NEXTEST_PROFILE: ci\n          NEXTEST_NO_TESTS: pass\n",
    ))
);

// ---------------------------------------------------------------------------------------------------------------------
// No mutants, no full tier, no controls run on a pull request.

/// No workflow a pull request triggers runs `cargo mutants` in a job a pull request reaches; `mutants.yml` is not
/// triggered by `pull_request`; `ci-full.yml` (the label-triggered full attempt) is gone; and in `ci.yml` the
/// interim per-merge jobs (`xtask-ci`, `gpu-kernel` and the rest) are each kept off a pull request.
fn check_off_the_pr_path(workflows: &[(String, String)]) {
    for (file, text) in workflows {
        assert!(
            file != "ci-full.yml",
            "qa pr tier: ci-full.yml, the merge-ready full attempt, is still a workflow"
        );
        if !triggers_on_pull_request(text) {
            continue;
        }
        assert!(
            file != "mutants.yml",
            "qa pr tier: mutants.yml is triggered by pull_request"
        );
        for (id, lines) in jobs(text) {
            let cond = job_if(&lines);
            if cond.contains("!= 'pull_request'") || cond.contains("== 'push'") {
                continue;
            }
            for c in runs(&lines) {
                assert!(
                    !c.contains("cargo mutants"),
                    "qa pr tier: {file}'s `{id}` runs mutants on a pull request"
                );
            }
        }
    }
    let ci = &workflows.iter().find(|(f, _)| f == "ci.yml").unwrap().1;
    for interim in [
        "xtask-ci",
        "gpu-kernel",
        "ci-workspace",
        "ci-archive",
        "ci-checks",
        "ci-release-archive",
    ] {
        let (_, lines) = jobs(ci)
            .into_iter()
            .find(|(id, _)| id == interim)
            .unwrap_or_else(|| panic!("qa pr tier: ci.yml has no interim job `{interim}`"));
        assert_eq!(
            job_if(&lines),
            "github.event_name != 'pull_request'",
            "qa pr tier: the interim per-merge job `{interim}` runs on a pull request"
        );
    }
}

fn workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn qa_pr_tier_mutants_are_off_the_pr_path() {
    check_off_the_pr_path(&workflows());
}

negative_control!(
    qa_pr_tier_mutants_are_off_the_pr_path,
    "mutants.yml triggered by pull_request again",
    expected = "mutants.yml is triggered by pull_request",
    check_off_the_pr_path(
        &workflows()
            .into_iter()
            .map(|(f, t)| {
                let t = if f == "mutants.yml" {
                    swap(
                        &t,
                        "on:\n  workflow_dispatch:\n",
                        "on:\n  workflow_dispatch:\n  pull_request:\n",
                    )
                } else {
                    t
                };
                (f, t)
            })
            .collect::<Vec<_>>()
    )
);

#[test]
fn qa_pr_tier_the_controls_run_only_per_merge() {
    check_off_the_pr_path(&workflows());
}

negative_control!(
    qa_pr_tier_the_controls_run_only_per_merge,
    "ci.yml with the interim `xtask-ci` (every control) on pull requests",
    expected = "the interim per-merge job `xtask-ci` runs on a pull request",
    check_off_the_pr_path(
        &workflows()
            .into_iter()
            .map(|(f, t)| {
                let t = if f == "ci.yml" {
                    let at = t.find("\n  xtask-ci:\n").unwrap();
                    let (head, tail) = t.split_at(at);
                    format!(
                        "{head}{}",
                        swap(
                            tail,
                            "    if: github.event_name != 'pull_request'\n",
                            "    if: always()\n"
                        )
                    )
                } else {
                    t
                };
                (f, t)
            })
            .collect::<Vec<_>>()
    )
);

#[test]
fn qa_pr_tier_no_full_tier_workflow() {
    check_off_the_pr_path(&workflows());
}

negative_control!(
    qa_pr_tier_no_full_tier_workflow,
    "the workflows with ci-full.yml restored",
    expected = "ci-full.yml, the merge-ready full attempt, is still a workflow",
    check_off_the_pr_path(&{
        let mut w = workflows();
        w.push((
            "ci-full.yml".to_owned(),
            "name: ci-full\non:\n  pull_request:\n    types: [labeled]\njobs: {}\n".to_owned(),
        ));
        w
    })
);

// ---------------------------------------------------------------------------------------------------------------------
// The golden runner over every suite and the screenshot runner's self-test: per merge, never retired.

/// A nextest profile's `default-filter` text.
fn filter(toml: &str, profile: &str) -> String {
    let header = format!("[profile.{profile}]\n");
    let at = toml
        .find(&header)
        .unwrap_or_else(|| panic!("qa pr tier: no {header}"));
    let rest = &toml[at + header.len()..];
    let start = rest
        .find("default-filter = \"\"\"")
        .unwrap_or_else(|| panic!("qa pr tier: {profile} has no multi-line default-filter"));
    let body = &rest[start + "default-filter = \"\"\"".len()..];
    body[..body.find("\"\"\"").unwrap()].to_owned()
}

/// The `ci` profile, which `ci.yml`'s interim shards run on every push to `main`, is an exclusion list that names
/// neither the golden runner's binary nor the screenshot runner's, so it runs both; the `ci-workspace` profile runs the
/// golden listing's test; and both run in jobs of `ci.yml` on a push, never `continue-on-error`.
fn check_per_merge_runners(toml: &str, ci: &str) {
    let ci_filter = filter(toml, "ci");
    assert!(
        ci_filter.trim_start().starts_with("not ("),
        "qa pr tier: the `ci` profile is no longer an exclusion list; check its runners by hand"
    );
    for binary in ["xtask::qa_TASK-M0-06_runner", "xtask::qa_TASK-M0-20"] {
        assert!(
            !ci_filter.contains(&format!("binary_id(={binary})")),
            "qa pr tier: the `ci` profile run on `main` leaves out {binary} (product correctness, per merge, never retired)"
        );
    }
    assert!(
        filter(toml, "ci-workspace").contains(
            "(binary_id(=xtask::qa_TASK-M0-06_list) & test(=qa_m006_ci_list_lists_golden_cases_without_a_device))"
        ),
        "qa pr tier: the `ci-workspace` profile no longer runs the golden listing's test"
    );
    for (profile, job) in [("ci", "ci"), ("ci-workspace", "ci-workspace")] {
        let (_, lines) = jobs(ci)
            .into_iter()
            .find(|(id, _)| id == job)
            .unwrap_or_else(|| panic!("qa pr tier: ci.yml has no job `{job}`"));
        assert!(
            lines
                .iter()
                .any(|l| l.trim() == format!("NEXTEST_PROFILE: {profile}")),
            "qa pr tier: ci.yml's `{job}` does not run the `{profile}` profile"
        );
        assert!(
            !lines
                .iter()
                .any(|l| l.starts_with("    continue-on-error:")),
            "qa pr tier: ci.yml's `{job}` can pass with the runners failing"
        );
    }
    let on_push = ci
        .split("\non:\n")
        .nth(1)
        .and_then(|s| s.split("\n\n").next())
        .unwrap_or("");
    assert!(
        on_push.contains("  push:\n    branches: [main"),
        "qa pr tier: ci.yml does not run on a push to `main`, where the runners run"
    );
}

#[test]
fn qa_pr_tier_golden_and_screenshot_runners_run_per_merge() {
    check_per_merge_runners(&read(".config/nextest.toml"), &ci());
}

negative_control!(
    qa_pr_tier_golden_and_screenshot_runners_run_per_merge,
    "the `ci` profile with the screenshot runner's self-test left out",
    expected = "the `ci` profile run on `main` leaves out xtask::qa_TASK-M0-20",
    check_per_merge_runners(
        &swap(
            &read(".config/nextest.toml"),
            "[profile.ci]\ndefault-filter = \"\"\"\n    not (\n",
            "[profile.ci]\ndefault-filter = \"\"\"\n    not (\n        binary_id(=xtask::qa_TASK-M0-20)\n        |\n",
        ),
        &ci(),
    )
);

// ---------------------------------------------------------------------------------------------------------------------
// Process tests only on a change to the machinery; `ci` on no task-branch push.

fn process_of(paths: &str) -> String {
    let mut child = Command::new("bash")
        .arg(root().join("scripts/ci_changes.sh"))
        .arg("process")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("qa pr tier: bash");
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(paths.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "qa pr tier: ci_changes.sh process failed"
    );
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

/// The owner's list, a path under each, runs the process tests; a product-only change does not.
const MACHINERY: [&str; 7] = [
    ".github/workflows/ci.yml",
    "xtask/src/main.rs",
    "scripts/ci_changes.sh",
    "plan/OPERATIONS.md",
    "decisions.md",
    "REVIEW_QUEUE.md",
    "docs/read_first/principia_INDEX.md",
];
const PRODUCT: [&str; 3] = [
    "crates/engine/src/lib.rs",
    "crates/kernel/tests/roundtrip.rs",
    "Cargo.lock",
];

fn check_process(classify: impl Fn(&str) -> String, ci: &str) {
    for path in MACHINERY {
        assert_eq!(
            classify(&format!("{path}\n")),
            "process=true",
            "qa pr tier: a change to {path} does not run the process tests"
        );
    }
    for path in PRODUCT {
        assert_eq!(
            classify(&format!("{path}\n")),
            "process=false",
            "qa pr tier: a product-only change ({path}) runs the process tests"
        );
    }
    assert_eq!(
        classify(&format!("{}\n{}\n", PRODUCT[0], MACHINERY[4])),
        "process=true",
        "qa pr tier: a change mixing product and machinery does not run the process tests"
    );
    let (_, lines) = the_job(ci, "ci.yml", "process");
    assert!(
        job_if(&lines).contains("needs.changes.outputs.process == 'true'"),
        "qa pr tier: `process` does not run only on a change to the machinery"
    );
    assert!(
        lines.iter().any(|l| l.trim() == "NEXTEST_PROFILE: process"),
        "qa pr tier: `process` does not run the `process` profile"
    );
}

#[test]
fn qa_pr_tier_process_tests_only_on_the_machinery() {
    check_process(process_of, &ci());
}

negative_control!(
    qa_pr_tier_process_tests_only_on_the_machinery,
    "a classifier that runs the process tests on every change",
    expected = "a product-only change (crates/engine/src/lib.rs) runs the process tests",
    check_process(|_| "process=true".to_owned(), &ci())
);

#[test]
fn qa_pr_tier_process_job_reads_the_classification() {
    check_process(process_of, &ci());
}

negative_control!(
    qa_pr_tier_process_job_reads_the_classification,
    "ci.yml with `process` on every pull request",
    expected = "`process` does not run only on a change to the machinery",
    check_process(
        process_of,
        &swap(
            &ci(),
            "    if: github.event_name == 'pull_request' && needs.changes.outputs.process == 'true'\n",
            "    if: github.event_name == 'pull_request'\n",
        )
    )
);

/// `ci.yml` runs on `pull_request` and on a push to `main` (and `measure/`, which never has a pull request, R-272):
/// no task branch's push starts it.
fn check_triggers(ci: &str) {
    let on = ci
        .split("\non:\n")
        .nth(1)
        .and_then(|s| s.split("\n\n").next())
        .unwrap_or_else(|| panic!("qa pr tier: ci.yml has no `on:` block"));
    assert!(
        on.lines().any(|l| l == "  pull_request:"),
        "qa pr tier: ci.yml is not triggered by pull_request"
    );
    let branches = on
        .lines()
        .find_map(|l| l.trim().strip_prefix("branches: "))
        .unwrap_or_else(|| {
            panic!("qa pr tier: ci.yml's push names no branches, so every branch's push runs it")
        });
    let list: Vec<&str> = branches
        .trim_matches(['[', ']'])
        .split(',')
        .map(|b| b.trim().trim_matches('"'))
        .collect();
    assert!(
        list.contains(&"main"),
        "qa pr tier: ci.yml does not run on a push to main"
    );
    for b in &list {
        assert!(
            *b == "main" || b.starts_with("measure/"),
            "qa pr tier: ci.yml runs on a push to `{b}`, not only main"
        );
    }
}

#[test]
fn qa_pr_tier_ci_runs_on_no_task_branch_push() {
    check_triggers(&ci());
}

negative_control!(
    qa_pr_tier_ci_runs_on_no_task_branch_push,
    "ci.yml run on a push to every task branch",
    expected = "ci.yml runs on a push to `task/**`",
    check_triggers(&swap(
        &ci(),
        "    branches: [main, \"measure/**\"]\n",
        "    branches: [main, \"measure/**\", \"task/**\"]\n",
    ))
);

// ---------------------------------------------------------------------------------------------------------------------
// The GPU smoke sets: built kernel, the `gpu-smoke` profile, each named binary in that profile.

/// Each GPU smoke job builds the kernel and runs only under the `gpu-smoke` profile; each `--test <binary>` a smoke
/// step names is a binary the profile selects, so no step runs an empty set; `gpu-metal` runs on Metal on macOS.
fn check_gpu_smoke(ci: &str, toml: &str) {
    let smoke = filter(toml, "gpu-smoke");
    for (name, backend) in [("gpu-lavapipe", "vulkan"), ("gpu-metal", "metal")] {
        let (id, lines) = the_job(ci, "ci.yml", name);
        let cmds = runs(&lines);
        assert!(
            cmds.iter().any(|c| c == "cargo xtask build-kernel"),
            "qa pr tier: `{name}` ({id}) does not build the kernel"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.trim() == format!("PRIN_GPU_BACKEND: {backend}")),
            "qa pr tier: `{name}` ({id}) does not run on {backend}"
        );
        let text = lines.join("\n");
        for step in text
            .split("      - name: ")
            .filter(|s| s.contains("run: cargo nextest run"))
        {
            assert!(
                step.contains("NEXTEST_PROFILE: gpu-smoke"),
                "qa pr tier: `{name}` ({id}) runs nextest outside the `gpu-smoke` profile: {step}"
            );
            let cmd = step
                .lines()
                .find_map(|l| l.trim().strip_prefix("run: "))
                .unwrap();
            let words: Vec<&str> = cmd.split_whitespace().collect();
            let package = words
                .windows(2)
                .find(|w| w[0] == "-p")
                .map(|w| w[1])
                .unwrap();
            for w in words.windows(2).filter(|w| w[0] == "--test") {
                let binary = format!("binary_id(={package}::{})", w[1]);
                assert!(
                    smoke.contains(&binary),
                    "qa pr tier: `{name}` ({id}) names `--test {}` of {package}, which the `gpu-smoke` profile selects nothing of",
                    w[1]
                );
            }
        }
    }
    let (_, metal) = the_job(ci, "ci.yml", "gpu-metal");
    assert!(
        metal.iter().any(|l| l.trim().starts_with("runs-on: macos")),
        "qa pr tier: `gpu-metal` does not run on a macOS runner"
    );
}

#[test]
fn qa_pr_tier_gpu_smoke_sets_run_the_smoke_profile() {
    check_gpu_smoke(&ci(), &read(".config/nextest.toml"));
}

negative_control!(
    qa_pr_tier_gpu_smoke_sets_run_the_smoke_profile,
    "the `gpu-smoke` profile without the canonicaliser's test, which both smoke jobs name",
    expected =
        "names `--test canon_gpu` of engine, which the `gpu-smoke` profile selects nothing of",
    check_gpu_smoke(
        &ci(),
        &swap(
            &read(".config/nextest.toml"),
            "    | (binary_id(=engine::canon_gpu) & test(=canonicaliser_seam_sweep))\n",
            "",
        )
    )
);

#[test]
fn qa_pr_tier_gpu_metal_runs_on_metal() {
    check_gpu_smoke(&ci(), &read(".config/nextest.toml"));
}

negative_control!(
    qa_pr_tier_gpu_metal_runs_on_metal,
    "ci.yml with the Metal smoke set on lavapipe",
    expected = "`gpu-metal` (gpu-metal-smoke) does not run on metal",
    check_gpu_smoke(
        &swap(
            &ci(),
            "      PRIN_GPU_BACKEND: metal\n      RUN: ${{ needs.changes.outputs.gpu == 'true' }}\n",
            "      PRIN_GPU_BACKEND: vulkan\n      RUN: ${{ needs.changes.outputs.gpu == 'true' }}\n",
        ),
        &read(".config/nextest.toml"),
    )
);

// ---------------------------------------------------------------------------------------------------------------------
// The list the owner sets branch protection from.

/// `plan/HUMAN_SETUP.md` §2's required checks, the names in backticks on the "require status checks to pass" item up
/// to its first parenthesis, are exactly the PR tier's nine (R-909's PR tier: "check_plan.py, reviews-complete,
/// pr-check, no-atlas-params", `no-atlas-params` beside ci.yml's six): none of the dropped `xtask-ci`, `gpu-kernel` or
/// `mutants-check`, and none missing.
fn check_setup_list(setup: &str) {
    let item = setup
        .split("- require status checks to pass:")
        .nth(1)
        .unwrap_or_else(|| panic!("qa pr tier: HUMAN_SETUP.md has no required-checks item"));
    let item = &item[..item.find('(').unwrap_or(item.len())];
    let mut names: Vec<&str> = item.split('`').skip(1).step_by(2).collect();
    names.sort_unstable();
    let mut want: Vec<&str> = CI_REQUIRED
        .iter()
        .copied()
        .chain(["pr-check", "reviews-complete", "no-atlas-params"])
        .collect();
    want.sort_unstable();
    assert_eq!(
        names, want,
        "qa pr tier: HUMAN_SETUP.md's required checks are not the PR tier's nine"
    );
}

#[test]
fn qa_pr_tier_setup_lists_the_nine_required_checks() {
    check_setup_list(&read("plan/HUMAN_SETUP.md"));
}

negative_control!(
    qa_pr_tier_setup_lists_the_nine_required_checks,
    "HUMAN_SETUP.md still requiring `xtask-ci`",
    expected = "HUMAN_SETUP.md's required checks are not the PR tier's nine",
    check_setup_list(&swap(
        &read("plan/HUMAN_SETUP.md"),
        "`reviews-complete` and `no-atlas-params` (",
        "`reviews-complete`, `xtask-ci` and `no-atlas-params` (",
    ))
);

// ---------------------------------------------------------------------------------------------------------------------
// REQ-GUI-412: "a value-noise lattice corner shared by adjacent cells must be bitwise equal from each cell, on every
// backend the regression test runs on: CI's lavapipe and Metal block". With the PR tier's GPU jobs now the smoke sets,
// the blocking lattice run on a pull request is theirs (`gpu-lavapipe`, `gpu-metal`), on a GPU-path change (R-656).

/// The steps of a job (its lines below the id), each as its lines, split at the six-space `- ` that opens a step.
fn steps_of(lines: &[String]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut in_steps = false;
    for line in lines {
        if line.starts_with("    ") && !line.starts_with("     ") && !line.trim().starts_with('#') {
            in_steps = line.trim_end() == "    steps:";
            continue;
        }
        if !in_steps {
            continue;
        }
        if line.starts_with("      - ") {
            out.push(vec![line.clone()]);
        } else if let Some(step) = out.last_mut() {
            step.push(line.clone());
        }
    }
    out
}

/// A step's own key at eight spaces in (its `- ` line counted as the first key), or "".
fn step_key(step: &[String], key: &str) -> String {
    let want = format!("{key}:");
    step.iter()
        .find_map(|l| {
            let t = l
                .strip_prefix("      - ")
                .or_else(|| l.strip_prefix("        "))?;
            t.strip_prefix(want.as_str()).map(|v| v.trim().to_owned())
        })
        .unwrap_or_default()
}

/// The job's own `env:` value for `key` (six spaces in), or "".
fn job_env(lines: &[String], key: &str) -> String {
    let mut in_env = false;
    for l in lines {
        if l.starts_with("    ") && !l.starts_with("     ") {
            in_env = l.trim_end() == "    env:";
            continue;
        }
        if in_env {
            if let Some(v) = l.strip_prefix(&format!("      {key}:")) {
                return v.trim().to_owned();
            }
        }
    }
    String::new()
}

/// The step's own `env:` value for `key` (ten spaces in), or "".
fn step_env(step: &[String], key: &str) -> String {
    step.iter()
        .find_map(|l| l.strip_prefix(&format!("          {key}:")))
        .map(|v| v.trim().to_owned())
        .unwrap_or_default()
}

/// The lattice tests gui's library holds, as nextest names them, and whether each is `#[ignore]`d.
fn lattice_tests(source: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut attrs: Vec<&str> = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if t.starts_with("#[") {
            attrs.push(t);
            continue;
        }
        if let Some(rest) = t.strip_prefix("fn lattice_") {
            if attrs.contains(&"#[test]") {
                let name = rest.split('(').next().unwrap();
                let ignored = attrs.iter().any(|a| a.starts_with("#[ignore"));
                out.push((format!("tests::mock_lattice::lattice_{name}"), ignored));
            }
        }
        if !t.is_empty() && !t.starts_with("//") {
            attrs.clear();
        }
    }
    out
}

/// Whether a `default-filter`'s clauses select gui-library test `name`: a clause naming `binary_id(=gui)` and either
/// no `test(...)`, `test(=name)`, or `test(/^prefix/)` with a literal prefix of `name`.
fn selects(filter: &str, name: &str) -> bool {
    filter
        .split('|')
        .map(str::trim)
        .filter(|c| c.contains("binary_id(=gui)"))
        .any(|c| match c.find("test(") {
            None => !c.contains("not "),
            Some(at) => {
                let arg = &c[at + "test(".len()..];
                let arg = &arg[..arg.find(')').unwrap_or(arg.len())];
                if let Some(exact) = arg.strip_prefix('=') {
                    exact == name
                } else if let Some(re) = arg.strip_prefix("/^").and_then(|r| r.strip_suffix('/')) {
                    re.chars()
                        .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == ':')
                        && name.starts_with(re)
                } else {
                    false
                }
            }
        })
}

/// The GPU-path condition under which a smoke set's steps may skip (R-656); anything else may skip them on a GPU change.
const RUN_ON_GPU: &str = "env.RUN == 'true'";

/// Each smoke set runs gui's library tests under the `gpu-smoke` profile on its backend, which selects every lattice
/// test that is not `#[ignore]`d, blocking: no `continue-on-error`, no step condition but the GPU-path one, no filter
/// or ignored-run that narrows or widens what the profile selects.
fn check_lattice_smoke(ci: &str, toml: &str, gui_source: &str) {
    let tests = lattice_tests(gui_source);
    let blocking: Vec<&String> = tests.iter().filter(|(_, i)| !i).map(|(n, _)| n).collect();
    assert!(
        blocking.len() >= 3,
        "qa pr tier: gui's library has {} lattice tests that are not ignored",
        blocking.len()
    );
    let smoke = filter(toml, "gpu-smoke");
    for name in &blocking {
        assert!(
            selects(&smoke, name),
            "qa pr tier: the `gpu-smoke` profile does not select gui's {name}"
        );
    }
    for (check, backend) in [("gpu-lavapipe", "vulkan"), ("gpu-metal", "metal")] {
        let (id, lines) = the_job(ci, "ci.yml", check);
        assert_eq!(
            job_env(&lines, "PRIN_GPU_BACKEND"),
            backend,
            "qa pr tier: `{check}` ({id}) is not on {backend}"
        );
        assert!(
            !lines
                .iter()
                .any(|l| l.starts_with("    continue-on-error:")),
            "qa pr tier: `{check}` ({id}) can pass with a failing step"
        );
        let gui: Vec<Vec<String>> = steps_of(&lines)
            .into_iter()
            .filter(|s| {
                runs(s).iter().any(|r| {
                    let w: Vec<&str> = r.split_whitespace().collect();
                    w.starts_with(&["cargo", "nextest", "run"])
                        && w.windows(2)
                            .any(|p| p == ["-p", "gui"] || p == ["--package", "gui"])
                })
            })
            .collect();
        assert_eq!(
            gui.len(),
            1,
            "qa pr tier: `{check}` ({id}) has {} steps running gui's tests",
            gui.len()
        );
        let step = &gui[0];
        let run = runs(step).remove(0);
        for word in run.split_whitespace().skip(3) {
            assert!(
                ["-p", "--package", "gui", "--lib", "--no-capture", "--no-fail-fast"].contains(&word),
                "qa pr tier: `{check}` ({id}) runs gui's tests with `{word}`, which can change what the profile selects"
            );
        }
        let profile = match step_env(step, "NEXTEST_PROFILE") {
            p if p.is_empty() => job_env(&lines, "NEXTEST_PROFILE"),
            p => p,
        };
        assert_eq!(
            profile, "gpu-smoke",
            "qa pr tier: `{check}` ({id}) runs gui's tests under another profile"
        );
        assert!(
            step_key(step, "continue-on-error").is_empty(),
            "qa pr tier: `{check}` ({id}) lets gui's lattice step fail"
        );
        let cond = step_key(step, "if");
        assert!(
            cond.is_empty() || cond == RUN_ON_GPU,
            "qa pr tier: `{check}` ({id}) runs gui's lattice step only when `{cond}`"
        );
    }
}

fn gui_lattice_source() -> String {
    read("crates/gui/src/tests/mock_lattice.rs")
}

#[test]
fn qa_pr_tier_lattice_tests_block_on_both_smoke_sets() {
    check_lattice_smoke(&ci(), &read(".config/nextest.toml"), &gui_lattice_source());
}

/// `text` without its last `cut` (the Metal smoke set's step follows lavapipe's).
#[cfg(feature = "controls")]
fn cut_last(text: &str, cut: &str) -> String {
    let at = text
        .rfind(cut)
        .unwrap_or_else(|| panic!("qa pr tier: control target {cut:?} gone"));
    format!("{}{}", &text[..at], &text[at + cut.len()..])
}

negative_control!(
    qa_pr_tier_lattice_tests_block_on_both_smoke_sets,
    "ci.yml whose Metal smoke set runs no gui step",
    expected = "`gpu-metal` (gpu-metal-smoke) has 0 steps running gui's tests",
    check_lattice_smoke(
        &cut_last(
            &ci(),
            "        run: cargo nextest run -p gui --lib --no-capture\n"
        ),
        &read(".config/nextest.toml"),
        &gui_lattice_source(),
    )
);

#[test]
fn qa_pr_tier_smoke_profile_selects_every_lattice_test() {
    check_lattice_smoke(&ci(), &read(".config/nextest.toml"), &gui_lattice_source());
}

negative_control!(
    qa_pr_tier_smoke_profile_selects_every_lattice_test,
    "the `gpu-smoke` profile narrowed to one lattice test",
    expected = "the `gpu-smoke` profile does not select gui's",
    check_lattice_smoke(
        &ci(),
        &swap(
            &read(".config/nextest.toml"),
            "(binary_id(=gui) & test(/^tests::mock_lattice::lattice_/))",
            "(binary_id(=gui) & test(=tests::mock_lattice::lattice_shared_corners_bitwise))",
        ),
        &gui_lattice_source(),
    )
);

#[test]
fn qa_pr_tier_lattice_steps_let_no_failure_past() {
    check_lattice_smoke(&ci(), &read(".config/nextest.toml"), &gui_lattice_source());
}

negative_control!(
    qa_pr_tier_lattice_steps_let_no_failure_past,
    "ci.yml whose lavapipe lattice step is continue-on-error",
    expected = "`gpu-lavapipe` (gpu-lavapipe-smoke) lets gui's lattice step fail",
    check_lattice_smoke(
        &swap(
            &ci(),
            "        run: cargo nextest run -p gui --lib --no-capture\n",
            "        continue-on-error: true\n        run: cargo nextest run -p gui --lib --no-capture\n",
        ),
        &read(".config/nextest.toml"),
        &gui_lattice_source(),
    )
);

negative_control!(
    qa_pr_tier_lattice_steps_run_no_ignored_report,
    "ci.yml whose lavapipe lattice step runs the ignored reports too",
    expected = "runs gui's tests with `--run-ignored`",
    check_lattice_smoke(
        &swap(
            &ci(),
            "        run: cargo nextest run -p gui --lib --no-capture\n",
            "        run: cargo nextest run -p gui --lib --run-ignored all --no-capture\n",
        ),
        &read(".config/nextest.toml"),
        &gui_lattice_source(),
    )
);

#[test]
fn qa_pr_tier_lattice_steps_run_no_ignored_report() {
    check_lattice_smoke(&ci(), &read(".config/nextest.toml"), &gui_lattice_source());
}

// ---------------------------------------------------------------------------------------------------------------------
// REQ-TOOL-223 (as R-907..R-909 amend it): "A failure of `ci-release`, a shard or its archive, must fail the run it is
// in and never be read by eye (R-809): it runs on each merge to `main`, off the pull request's path, a job of the
// per-merge run conditional on nothing but the event not being a pull request, with no `continue-on-error` on its build
// or test steps, … no pull-request check gathers it".

/// The jobs a job `needs`, inline (`needs: a` or `needs: [a, b]`) or as a block list.
fn needs_of(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut block = false;
    for l in lines {
        if let Some(v) = l.strip_prefix("    needs:") {
            let v = v.trim().trim_start_matches('[').trim_end_matches(']');
            out.extend(
                v.split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty()),
            );
            block = v.is_empty();
            continue;
        }
        if block {
            match l.strip_prefix("      - ") {
                Some(v) => out.push(v.trim().to_owned()),
                None => block = false,
            }
        }
    }
    out
}

/// Whether a job's own `if:` keeps it off pull requests.
fn off_pull_requests(cond: &str) -> bool {
    cond.contains("github.event_name != 'pull_request'")
        || cond.contains("github.event_name == 'push'")
}

fn check_release_blocks(ci: &str) {
    let all = jobs(ci);
    let job = |id: &str| -> Vec<String> {
        all.iter()
            .find(|(j, _)| j == id)
            .map(|(_, l)| l.clone())
            .unwrap_or_else(|| panic!("qa pr tier: ci.yml has no `{id}`"))
    };
    let archive = job("ci-release-archive");
    assert_eq!(
        job_if(&archive),
        "github.event_name != 'pull_request'",
        "qa pr tier: `ci-release-archive` runs under another condition"
    );
    let shards = job("ci-release");
    assert!(
        needs_of(&shards).iter().any(|n| n == "ci-release-archive"),
        "qa pr tier: `ci-release` does not need its archive"
    );
    let cond = job_if(&shards);
    assert!(
        cond.is_empty() || cond == "github.event_name != 'pull_request'",
        "qa pr tier: `ci-release` runs under `{cond}`"
    );
    for (id, lines) in [("ci-release-archive", &archive), ("ci-release", &shards)] {
        assert!(
            !lines
                .iter()
                .any(|l| l.starts_with("    continue-on-error:")),
            "qa pr tier: `{id}` can pass with a failing step"
        );
        let cargo: Vec<Vec<String>> = steps_of(lines)
            .into_iter()
            .filter(|s| runs(s).iter().any(|r| r.contains("cargo ")))
            .collect();
        assert!(!cargo.is_empty(), "qa pr tier: `{id}` runs no cargo step");
        for step in &cargo {
            assert!(
                step_key(step, "continue-on-error").is_empty(),
                "qa pr tier: `{id}` lets its build or test step fail: {}",
                step[0].trim()
            );
        }
    }
    for (id, lines) in &all {
        let needs = needs_of(lines);
        if needs
            .iter()
            .any(|n| n == "ci-release" || n == "ci-release-archive")
        {
            assert!(
                off_pull_requests(&job_if(lines))
                    || needs
                        .iter()
                        .any(|n| n == "ci-release-archive" && id == "ci-release"),
                "qa pr tier: `{id}`, which runs on a pull request, gathers the release run"
            );
        }
    }
}

#[test]
fn qa_pr_tier_release_failure_fails_its_run() {
    check_release_blocks(&ci());
}

negative_control!(
    qa_pr_tier_release_failure_fails_its_run,
    "ci.yml whose release shards' test step is continue-on-error",
    expected = "`ci-release` lets its build or test step fail",
    check_release_blocks(&swap(
        &ci(),
        "        run: cargo nextest run --archive-file $RUNNER_TEMP/nextest-release.tar.zst",
        "        continue-on-error: true\n        run: cargo nextest run --archive-file $RUNNER_TEMP/nextest-release.tar.zst",
    ))
);

#[test]
fn qa_pr_tier_release_archive_failure_fails_its_run() {
    check_release_blocks(&ci());
}

negative_control!(
    qa_pr_tier_release_archive_failure_fails_its_run,
    "ci.yml whose release archive job is continue-on-error",
    expected = "`ci-release-archive` can pass with a failing step",
    check_release_blocks(&swap(
        &ci(),
        "  ci-release-archive:\n    if: github.event_name != 'pull_request'\n",
        "  ci-release-archive:\n    if: github.event_name != 'pull_request'\n    continue-on-error: true\n",
    ))
);

#[test]
fn qa_pr_tier_no_pull_request_check_gathers_the_release_run() {
    check_release_blocks(&ci());
}

negative_control!(
    qa_pr_tier_no_pull_request_check_gathers_the_release_run,
    "ci.yml whose `ci` check needs the release shards again",
    expected = "`tests`, which runs on a pull request, gathers the release run",
    check_release_blocks(&swap(
        &ci(),
        "  tests:\n    name: ci\n    needs: changes\n",
        "  tests:\n    name: ci\n    needs: [changes, ci-release]\n",
    ))
);
