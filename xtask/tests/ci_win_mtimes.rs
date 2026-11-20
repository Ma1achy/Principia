//! Warm rebuilds on the Windows runner (TASK-M0-89, the orchestrator's call under R-1033), read from
//! `.github/workflows/win-gpu.yml` and `.github/scripts/mtime_restore.ps1`:
//!
//! - each job that compiles on the runner has, right after its checkout and before its first build, the one step that
//!   fetches `mtime_restore.ps1` at the job's own commit, as the guard is fetched, refuses if it can't, and starts it
//!   fail closed with the checkout and the job's own record on E:; no workflow runs the checkout's copy;
//! - the guard's step, `clean: true` and the checkout's other inputs are as TASK-M0-77 and R-711 left them: this task
//!   may not change them;
//! - each job that restores builds into a target folder no workflow before the restore names (`<job id>-m89`), so no
//!   run that builds without the restore writes into a folder whose record, beside it, it serves;
//! - the script sets a mtime in one place only, inside the branch taken when the manifest's mode and blob id equal the
//!   index's; it deletes the old manifest before that and writes the new one after, through a rename.
//!
//! What runs where: these read text and run everywhere `cargo test -p xtask` runs. There is no PowerShell on the Mac or
//! the hosted runners (`xtask-ci` deletes it), so the script's behaviour on a temporary repo is
//! `ci_self_hosted.rs`'s `mtime_restore_keeps_only_unchanged_files_mtimes`, compiled everywhere and run only under
//! `cfg(windows)`: on the PC, in the nightly `win-self-check`, which runs that file.
//!
//! Each test has its negative controls (R-176).

use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const SCRIPT: &str = ".github/scripts/mtime_restore.ps1";
/// The jobs that compile on the runner itself (`ci_compile_cache.rs` holds the same list).
const COMPILES: [&str; 2] = ["win-vulkan", "win-self-check"];
const STEP_NAME: &str = "Unchanged sources keep their mtimes";
/// The step's script, whole: fetched at the job's own commit, a failed fetch refusing, started fail closed.
const STEP_RUN: &str = "$restore = Join-Path $env:RUNNER_TEMP 'mtime_restore.ps1'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
try { Invoke-WebRequest -UseBasicParsing -OutFile $restore -Uri \"https://raw.githubusercontent.com/$env:GITHUB_REPOSITORY/$env:GITHUB_SHA/.github/scripts/mtime_restore.ps1\" } catch { \"could not fetch the mtime restore: $_\"; exit 1 }
powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $restore $env:GITHUB_WORKSPACE \"${env:CARGO_TARGET_DIR}.mtimes.txt\"
exit $LASTEXITCODE
";
/// The guard's step as R-711 and TASK-M0-77 left it, which this task may not change.
const GUARD_RUN: &str = "$guard = Join-Path $env:RUNNER_TEMP 'git_guard.ps1'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
try { Invoke-WebRequest -UseBasicParsing -OutFile $guard -Uri \"https://raw.githubusercontent.com/$env:GITHUB_REPOSITORY/$env:GITHUB_SHA/.github/scripts/git_guard.ps1\" } catch { \"could not fetch the guard: $_\"; exit 1 }
powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $guard
exit $LASTEXITCODE
";
const CHECKOUT: &str = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the edit's target is gone: {from}");
    text.replacen(from, to, 1)
}

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .unwrap_or_else(|e| panic!("{WORKFLOW}: {e}"))
        .remove(0)
}

fn steps(job: &Yaml) -> &[Yaml] {
    job["steps"].as_vec().map_or(&[], Vec::as_slice)
}

fn run(step: &Yaml) -> &str {
    step["run"].as_str().unwrap_or("")
}

fn is_checkout(step: &Yaml) -> bool {
    step["uses"]
        .as_str()
        .is_some_and(|u| u.starts_with("actions/checkout@"))
}

/// Whether the step builds: it starts cargo through `win_measure.ps1 run`, as every cargo command on the runner does.
fn builds(step: &Yaml) -> bool {
    run(step).lines().any(|l| {
        let words: Vec<&str> = l.split_whitespace().collect();
        words
            .windows(4)
            .any(|w| w[0].ends_with("win_measure.ps1") && w[1] == "run" && w[3] == "cargo")
    })
}

fn check_step(text: &str) {
    let doc = parse(text);
    let jobs = doc["jobs"].as_hash().expect("the jobs");
    let compiling: Vec<&str> = jobs
        .iter()
        .filter(|(_, job)| steps(job).iter().any(builds))
        .map(|(id, _)| id.as_str().expect("a job id"))
        .collect();
    assert_eq!(compiling, COMPILES, "{WORKFLOW}'s jobs that compile");
    for (id, job) in jobs {
        let id = id.as_str().expect("a job id");
        let steps = steps(job);
        let named: Vec<usize> = (0..steps.len())
            .filter(|&i| {
                steps[i]["name"].as_str() == Some(STEP_NAME)
                    || run(&steps[i]).contains("mtime_restore")
            })
            .collect();
        if !COMPILES.contains(&id) {
            assert!(
                named.is_empty(),
                "{WORKFLOW}/{id} restores mtimes but builds nothing"
            );
            continue;
        }
        let checkout = steps
            .iter()
            .position(is_checkout)
            .unwrap_or_else(|| panic!("{WORKFLOW}/{id} has no checkout"));
        let first = steps.iter().position(builds).expect("a build");
        assert_eq!(
            named,
            [checkout + 1],
            "{WORKFLOW}/{id} doesn't restore mtimes once, in the step right after its checkout"
        );
        let step = &steps[checkout + 1];
        assert!(
            checkout + 1 < first,
            "{WORKFLOW}/{id} restores mtimes after its first build"
        );
        assert_eq!(
            step["name"].as_str(),
            Some(STEP_NAME),
            "{WORKFLOW}/{id}'s step after the checkout isn't the mtime restore"
        );
        assert_eq!(
            run(step),
            STEP_RUN,
            "{WORKFLOW}/{id}'s mtime restore isn't the script fetched at the job's commit and started fail closed"
        );
        assert!(
            step["if"].is_badvalue() && step["continue-on-error"].is_badvalue(),
            "{WORKFLOW}/{id}'s mtime restore can be skipped or fail unseen"
        );
    }
}

#[test]
fn each_compiling_job_restores_mtimes_right_after_its_checkout() {
    check_step(&read(WORKFLOW));
}

/// The step as the workflow holds it, its comment line with it.
#[cfg(feature = "controls")]
fn step_text() -> String {
    format!(
        "      # Unchanged sources get back the mtimes the last run left them (TASK-M0-89): see the header.\n      - name: {STEP_NAME}\n        timeout-minutes: 5\n        run: |\n{}",
        STEP_RUN
            .lines()
            .map(|l| format!("          {l}\n"))
            .collect::<String>()
    )
}

negative_control!(
    each_compiling_job_restores_mtimes_right_after_its_checkout,
    "the step moved before the checkout",
    expected =
        "win-gpu.yml/win-vulkan doesn't restore mtimes once, in the step right after its checkout",
    {
        let text = edited(&read(WORKFLOW), &step_text(), "");
        check_step(&edited(
            &text,
            "      - name: Clean checkout\n",
            &format!("{}      - name: Clean checkout\n", step_text()),
        ))
    }
);

mod step_missing {
    use super::*;
    negative_control!(
        each_compiling_job_restores_mtimes_right_after_its_checkout,
        "win-self-check without the step",
        expected = "win-gpu.yml/win-self-check doesn't restore mtimes once",
        {
            let text = read(WORKFLOW);
            let job = text.find("  win-self-check:\n").expect("the job");
            let at = job + text[job..].find(&step_text()).expect("its step");
            check_step(&format!(
                "{}{}",
                &text[..at],
                &text[at + step_text().len()..]
            ))
        }
    );
}

mod from_the_checkout {
    use super::*;
    negative_control!(
        each_compiling_job_restores_mtimes_right_after_its_checkout,
        "the script run from the checkout, not fetched at the job's commit",
        expected = "win-gpu.yml/win-vulkan's mtime restore isn't the script fetched",
        check_step(&edited(
            &read(WORKFLOW),
            "-File $restore $env:GITHUB_WORKSPACE",
            "-File .github\\scripts\\mtime_restore.ps1 $env:GITHUB_WORKSPACE"
        ))
    );
}

fn check_unchanged(text: &str) {
    let doc = parse(text);
    for id in COMPILES {
        let steps = steps(&doc["jobs"][id]);
        assert_eq!(
            steps.get(1).map(run),
            Some(GUARD_RUN),
            "{WORKFLOW}/{id}'s guard step isn't the one R-711 set"
        );
        let checkout = steps.iter().find(|s| is_checkout(s)).expect("a checkout");
        assert!(
            checkout["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with(CHECKOUT)),
            "{WORKFLOW}/{id}'s checkout isn't the pinned v7.0.1"
        );
        let with = checkout["with"].as_hash().expect("the checkout's inputs");
        assert!(
            with.len() == 3
                && checkout["with"]["persist-credentials"].as_bool() == Some(false)
                && checkout["with"]["fetch-depth"].as_i64() == Some(1)
                && checkout["with"]["clean"].as_bool() == Some(true),
            "{WORKFLOW}/{id}'s checkout inputs changed: {with:?}"
        );
    }
}

#[test]
fn the_guard_and_the_clean_checkout_are_unchanged() {
    check_unchanged(&read(WORKFLOW));
}

negative_control!(
    the_guard_and_the_clean_checkout_are_unchanged,
    "the checkout no longer cleaning",
    expected = "win-gpu.yml/win-vulkan's checkout inputs changed",
    check_unchanged(&edited(
        &read(WORKFLOW),
        "          clean: true\n",
        "          clean: false\n"
    ))
);

mod guard_changed {
    use super::*;
    negative_control!(
        the_guard_and_the_clean_checkout_are_unchanged,
        "the guard fetched from the checkout's branch",
        expected = "win-gpu.yml/win-vulkan's guard step isn't the one R-711 set",
        check_unchanged(&edited(
            &read(WORKFLOW),
            "$env:GITHUB_SHA/.github/scripts/git_guard.ps1",
            "main/.github/scripts/git_guard.ps1"
        ))
    );
}

/// Every target folder `win-gpu.yml` named before the mtime restore (origin/main before TASK-M0-89), where a dispatch
/// of an older ref, or a revert of the restore, still builds without it.
const OLD_TARGETS: [&str; 2] = [
    "E:\\ci-cache\\target\\win-vulkan",
    "E:\\ci-cache\\target\\win-self-check",
];

/// A Windows path as the file system compares it: case, separators and a trailing separator ignored.
fn same_folder(a: &str, b: &str) -> bool {
    let norm = |p: &str| {
        p.replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase()
    };
    norm(a) == norm(b)
}

fn check_targets(text: &str) {
    let doc = parse(text);
    for id in COMPILES {
        let dir = doc["jobs"][id]["env"]["CARGO_TARGET_DIR"]
            .as_str()
            .unwrap_or_else(|| {
                panic!("{WORKFLOW}/{id} restores mtimes with no CARGO_TARGET_DIR of its own for its record to sit beside")
            });
        for old in OLD_TARGETS {
            assert!(
                !same_folder(dir, old),
                "{WORKFLOW}/{id} builds into {dir}, a target folder a workflow without the mtime restore builds into"
            );
        }
    }
}

#[test]
fn the_restoring_jobs_build_where_no_older_workflow_does() {
    check_targets(&read(WORKFLOW));
}

negative_control!(
    the_restoring_jobs_build_where_no_older_workflow_does,
    "win-vulkan back in the folder older workflows build into",
    expected = "win-gpu.yml/win-vulkan builds into E:\\ci-cache\\target\\win-vulkan, a target folder a workflow without",
    check_targets(&edited(
        &read(WORKFLOW),
        "      CARGO_TARGET_DIR: E:\\ci-cache\\target\\win-vulkan-m89\n",
        "      CARGO_TARGET_DIR: E:\\ci-cache\\target\\win-vulkan\n"
    ))
);

mod old_target_spelled_otherwise {
    use super::*;
    negative_control!(
        the_restoring_jobs_build_where_no_older_workflow_does,
        "win-self-check in the old folder, in other case and with a trailing separator",
        expected = "win-gpu.yml/win-self-check builds into E:/CI-cache/target/Win-Self-Check/, a target folder",
        check_targets(&edited(
            &read(WORKFLOW),
            "      CARGO_TARGET_DIR: E:\\ci-cache\\target\\win-self-check-m89\n",
            "      CARGO_TARGET_DIR: E:/CI-cache/target/Win-Self-Check/\n"
        ))
    );
}

mod no_target {
    use super::*;
    negative_control!(
        the_restoring_jobs_build_where_no_older_workflow_does,
        "win-vulkan with no target folder of its own",
        expected = "win-gpu.yml/win-vulkan restores mtimes with no CARGO_TARGET_DIR of its own",
        check_targets(&edited(
            &read(WORKFLOW),
            "      CARGO_TARGET_DIR: E:\\ci-cache\\target\\win-vulkan-m89\n",
            ""
        ))
    );
}

/// The line holding `what`, which must be the only one.
fn only_line(lines: &[&str], what: &str) -> usize {
    let at: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].contains(what))
        .collect();
    assert_eq!(
        at.len(),
        1,
        "{SCRIPT}: {} lines hold {what}, not one",
        at.len()
    );
    at[0]
}

fn check_script(script: &str) {
    let lines: Vec<&str> = script.lines().collect();
    let set = only_line(&lines, "SetLastWriteTimeUtc(");
    let guard = (0..set)
        .rev()
        .find(|&i| lines[i].trim_start().starts_with("if ("))
        .unwrap_or_else(|| panic!("{SCRIPT}: the mtime is set under no condition"));
    let condition = lines[guard];
    assert!(
        condition.contains("$restore -and ")
            && condition.contains("$old.Blobs.TryGetValue($path, [ref]$was)")
            && condition.contains(" -and $was -ceq $key "),
        "{SCRIPT}: the mtime is set other than on the manifest's blob id equal to the index's: {condition}"
    );
    assert!(
        lines[guard + 1..set].iter().all(|l| l.trim() == "try {"),
        "{SCRIPT}: the mtime is set other than straight under its comparison"
    );
    let key = only_line(&lines, "$key = $index[$path]");
    assert!(key < guard, "{SCRIPT}: the key compared isn't the index's");
    assert!(
        script
            .contains("$indexEntry = '\\A(100644|100755) ([0-9a-f]{40}|[0-9a-f]{64}) 0\\t(.+)\\z'")
            && script.contains("$index[$Matches[3]] = \"$($Matches[1]) $($Matches[2])\""),
        "{SCRIPT}: the index's key isn't its mode and blob id"
    );
    let delete = only_line(&lines, "[System.IO.File]::Delete($manifest)");
    let rename = only_line(&lines, "[System.IO.File]::Move($temporary, $manifest)");
    assert!(
        delete < set && set < rename,
        "{SCRIPT}: the old manifest isn't deleted before a mtime is set and the new one written after"
    );
}

#[test]
fn the_script_restores_only_on_equal_blob_ids() {
    check_script(&read(SCRIPT));
}

negative_control!(
    the_script_restores_only_on_equal_blob_ids,
    "a restore that doesn't compare blob ids",
    expected = "the mtime is set other than on the manifest's blob id equal to the index's",
    check_script(&edited(&read(SCRIPT), " -and $was -ceq $key ", " "))
);

mod deleted_late {
    use super::*;
    negative_control!(
        the_script_restores_only_on_equal_blob_ids,
        "the old manifest deleted only after the restore",
        expected = "the old manifest isn't deleted before a mtime is set",
        {
            let script = edited(
                &read(SCRIPT),
                "    if ([System.IO.File]::Exists($manifest)) { [System.IO.File]::Delete($manifest) }\n",
                "",
            );
            check_script(&edited(
                &script,
                "    $text = New-Object System.Text.StringBuilder\n",
                "    if ([System.IO.File]::Exists($manifest)) { [System.IO.File]::Delete($manifest) }\n    $text = New-Object System.Text.StringBuilder\n",
            ))
        }
    );
}

mod unconditional {
    use super::*;
    negative_control!(
        the_script_restores_only_on_equal_blob_ids,
        "a second mtime set outside the comparison",
        expected = "lines hold SetLastWriteTimeUtc(, not one",
        check_script(&edited(
            &read(SCRIPT),
            "        $fresh++\n",
            "        [System.IO.File]::SetLastWriteTimeUtc($file, [DateTime]::UtcNow)\n        $fresh++\n"
        ))
    );
}
