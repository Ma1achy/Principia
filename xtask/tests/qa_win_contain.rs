//! qa's tests for tooling/win-contain, written from the owner's CI shape order of 8 Nov 2026, step 1 ("run tests and
//! mutants inside a Job Object with kill-on-close, a process limit and an 8 GiB per-process memory cap (REQ-VAL-181's
//! equivalent of prlimit). Prove it with the same kill test contain.sh check uses."), and the criterion written before
//! the run: each kill-test scenario contained as `contain.sh check` does, and after the job exits no process from the
//! test remains, listed by the check; plus "nothing runs uncontained where containment is required" and the runner rules.
//!
//! Off Windows there is no PowerShell, so these read `.github/scripts/win_contain.ps1` and `win-gpu.yml`. Each test has
//! its negative controls (R-176).

use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::YamlLoader;

const SCRIPT: &str = ".github/scripts/win_contain.ps1";
const CONTAIN: &str = "scripts/contain.sh";
const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const OPERATIONS: &str = "plan/OPERATIONS.md";
const JOB: &str = "win-contain";
/// The owner's "8 GiB", in bytes.
const EIGHT_GIB: u64 = 8 * 1024 * 1024 * 1024;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the control's target is gone: {from}");
    text.replacen(from, to, 1)
}

/// The text of the PowerShell `switch` arm `'<verb>' {`, up to its closing brace at the same indent.
fn arm<'a>(script: &'a str, verb: &str) -> &'a str {
    let head = format!("\n    '{verb}' {{\n");
    let start = script
        .find(&head)
        .unwrap_or_else(|| panic!("{SCRIPT} has no arm `{verb}`"));
    let end = script[start + 1..]
        .find("\n    }\n")
        .unwrap_or_else(|| panic!("{SCRIPT}'s arm `{verb}` has no end"));
    &script[start..start + 1 + end]
}

/// The body of the PowerShell function `name`, up to its closing brace at column 0.
fn function<'a>(script: &'a str, name: &str) -> &'a str {
    let head = format!("\nfunction {name}");
    let start = script
        .find(&head)
        .unwrap_or_else(|| panic!("{SCRIPT} has no function {name}"));
    let end = script[start + 1..]
        .find("\n}\n")
        .unwrap_or_else(|| panic!("{SCRIPT}'s function {name} has no end"));
    &script[start..start + 1 + end]
}

/// The number assigned to `$<name> = ` at the start of a line.
fn assigned(script: &str, name: &str) -> Option<u64> {
    let prefix = format!("${name} = ");
    script
        .lines()
        .find_map(|l| l.strip_prefix(prefix.as_str()))
        .and_then(|rest| {
            rest.split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|n| n.parse().ok())
        })
}

// ---- The cap is the owner's 8 GiB --------------------------------------------------------------------------------

fn check_cap(script: &str) {
    assert_eq!(
        assigned(script, "MemoryCap"),
        Some(EIGHT_GIB),
        "{SCRIPT}'s per-process cap isn't the owner's 8 GiB"
    );
    // The job is built with that cap, and the C# flag for it is JOB_OBJECT_LIMIT_PROCESS_MEMORY (winnt.h: 0x100).
    assert!(
        script.contains("[PrinContain]::Run([string[]]$rest, [uint32]$limit, [uint64]$MemoryCap)"),
        "{SCRIPT}'s run doesn't build its job with the cap"
    );
    assert!(
        script.contains("public const uint ProcessMemory = 0x100;")
            && script.contains("public const uint KillOnJobClose = 0x2000;")
            && script.contains("public const uint ActiveProcess = 0x8;"),
        "{SCRIPT}'s limit flags aren't winnt.h's JOB_OBJECT_LIMIT_* values"
    );
}

#[test]
fn qa_win_contain_cap_is_eight_gib() {
    check_cap(&read(SCRIPT));
}

negative_control!(
    qa_win_contain_cap_is_eight_gib,
    "a decimal 8 GB cap in place of 8 GiB",
    expected = "isn't the owner's 8 GiB",
    check_cap(&edited(
        &read(SCRIPT),
        "$MemoryCap = 8589934592",
        "$MemoryCap = 8000000000"
    ))
);

mod qa_flag {
    use super::*;
    negative_control!(
        qa_win_contain_cap_is_eight_gib,
        "the process-memory flag given JOB_OBJECT_LIMIT_JOB_MEMORY's value",
        expected = "aren't winnt.h's JOB_OBJECT_LIMIT_* values",
        check_cap(&edited(
            &read(SCRIPT),
            "public const uint ProcessMemory = 0x100;",
            "public const uint ProcessMemory = 0x200;"
        ))
    );
}

// ---- Nothing runs uncontained where containment is required ------------------------------------------------------

fn check_fail_closed(script: &str) {
    let fail = function(script, "Fail");
    assert!(
        fail.contains("exit 125"),
        "{SCRIPT}'s Fail doesn't fail the run (contain.sh's die exits 125)"
    );
    assert_eq!(
        contain_die_code(&read(CONTAIN)),
        125,
        "{CONTAIN}'s die no longer exits 125"
    );
    let run = arm(script, "run");
    let required = run
        .find("if ($env:PRIN_CONTAIN -ne 'required') {")
        .unwrap_or_else(|| panic!("{SCRIPT}'s run never asks whether containment is required"));
    // Past the pass-through, every way to the command goes through the job, and a failure of it is Fail.
    let contained = &run[required..];
    let pass_through_end = contained
        .find("\n        }\n")
        .expect("the pass-through has no end");
    let after = &contained[pass_through_end..];
    assert!(
        !after.contains("& $command"),
        "{SCRIPT}'s run starts its command outside the job where containment is required"
    );
    assert!(
        after.contains("Fail 'containment required, and this is not Windows'")
            && after.contains("Load")
            && after.contains("} catch {\n            Fail "),
        "{SCRIPT}'s run doesn't fail when containment can't be had"
    );
    let load = function(script, "Load");
    assert!(
        load.contains("} catch {\n        Fail "),
        "{SCRIPT} runs on when the Job Object API can't be loaded"
    );
}

/// The exit code of contain.sh's `die`.
fn contain_die_code(contain: &str) -> u64 {
    let at = contain.find("die() {").expect("contain.sh has no die");
    let body = &contain[at..at + contain[at..].find("\n}\n").expect("die has no end")];
    body.lines()
        .find_map(|l| l.trim().strip_prefix("exit "))
        .and_then(|n| n.parse().ok())
        .expect("contain.sh's die has no exit code")
}

#[test]
fn qa_win_contain_fails_closed() {
    check_fail_closed(&read(SCRIPT));
}

negative_control!(
    qa_win_contain_fails_closed,
    "Fail exits 0",
    expected = "Fail doesn't fail the run",
    check_fail_closed(&edited(&read(SCRIPT), "    exit 125\n}", "    exit 0\n}"))
);

mod qa_fallback {
    use super::*;
    negative_control!(
        qa_win_contain_fails_closed,
        "a failed job falls back to running the command plainly",
        expected = "starts its command outside the job",
        check_fail_closed(&edited(
            &read(SCRIPT),
            "            Fail \"containment required, and it failed: $(Reason $_)\"\n",
            "            $command, $commandArgs = $rest\n            & $command @commandArgs\n            exit $LASTEXITCODE\n"
        ))
    );
}

mod qa_load {
    use super::*;
    negative_control!(
        qa_win_contain_fails_closed,
        "an Add-Type failure only warned about",
        expected = "runs on when the Job Object API can't be loaded",
        check_fail_closed(&edited(
            &read(SCRIPT),
            "    } catch {\n        Fail \"containment required, and the Job Object API",
            "    } catch {\n        Write-Warning \"containment required, and the Job Object API"
        ))
    );
}

// ---- The process limit can only be lowered -----------------------------------------------------------------------

fn check_lower_only(script: &str) {
    let get = function(script, "Get-ProcessLimit");
    assert!(
        get.contains("$n -gt $ProcessLimit") && get.contains("$n -lt 1") && get.contains("Fail "),
        "{SCRIPT} lets PRIN_CONTAIN_PROCESSES raise or drop the process limit"
    );
    // The limit the job gets is that function's, read back by NewJob before the command starts.
    assert!(
        arm(script, "run").contains("$limit = Get-ProcessLimit"),
        "{SCRIPT}'s run doesn't take its limit from Get-ProcessLimit"
    );
    assert!(
        script.contains("back.Basic.ActiveProcessLimit != processLimit")
            && script.contains("back.ProcessMemoryLimit.ToUInt64() != memoryCap")
            && script.contains("(back.Basic.LimitFlags & Limits) != Limits"),
        "{SCRIPT} uses a job without reading back all three limits"
    );
}

#[test]
fn qa_win_contain_process_limit_only_lowered() {
    check_lower_only(&read(SCRIPT));
}

negative_control!(
    qa_win_contain_process_limit_only_lowered,
    "PRIN_CONTAIN_PROCESSES may raise the limit",
    expected = "lets PRIN_CONTAIN_PROCESSES raise or drop the process limit",
    check_lower_only(&edited(&read(SCRIPT), " -or $n -gt $ProcessLimit", ""))
);

mod qa_readback {
    use super::*;
    negative_control!(
        qa_win_contain_process_limit_only_lowered,
        "the cap not read back",
        expected = "without reading back all three limits",
        check_lower_only(&edited(
            &read(SCRIPT),
            "\n            || back.ProcessMemoryLimit.ToUInt64() != memoryCap",
            ""
        ))
    );
}

// ---- One for one with contain.sh check's probe -------------------------------------------------------------------

/// The exit codes contain.sh's probe fails with.
fn contain_probe_codes(contain: &str) -> Vec<u64> {
    let at = contain
        .find("local probe=")
        .expect("contain.sh has no probe");
    let body = &contain[at..at + contain[at..].find("exit 0'").expect("probe has no end")];
    let mut codes: Vec<u64> = body
        .match_indices("exit ")
        .filter_map(|(i, _)| {
            body[i + 5..]
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|n| n.parse().ok())
        })
        .collect();
    codes.sort_unstable();
    codes.dedup();
    codes
}

fn check_one_for_one(script: &str, contain: &str) {
    let codes = contain_probe_codes(contain);
    assert!(
        codes.len() >= 4,
        "{CONTAIN}'s probe has fewer scenarios than expected: {codes:?}"
    );
    let probe = arm(script, "probe");
    for code in codes {
        assert!(
            probe.contains(&format!("exit {code}"))
                || probe.contains(&format!("probe: {code} gap")),
            "{SCRIPT}'s probe has no counterpart of {CONTAIN}'s scenario {code}"
        );
    }
    // The cap's scenario fails both ways: under it refused, or over it granted.
    assert!(
        probe.contains("if ($granted -ne 0 -or $refused -eq 0) { exit 44 }"),
        "{SCRIPT}'s scenario 44 doesn't hold the cap at its boundary both ways"
    );
    // 41 compares every limit with what the check asked for.
    assert!(
        probe.contains("$flags -ne [PrinContain]::Limits")
            && probe.contains("$l.Basic.ActiveProcessLimit -ne [uint32]$limit")
            && probe.contains("$l.ProcessMemoryLimit.ToUInt64() -ne [uint64]$MemoryCap"),
        "{SCRIPT}'s scenario 41 doesn't compare every limit"
    );
}

#[test]
fn qa_win_contain_probe_matches_contain_sh() {
    check_one_for_one(&read(SCRIPT), &read(CONTAIN));
}

negative_control!(
    qa_win_contain_probe_matches_contain_sh,
    "contain.sh gains a scenario the Windows probe lacks",
    expected = "has no counterpart of scripts/contain.sh's scenario 47",
    check_one_for_one(
        &read(SCRIPT),
        &edited(
            &read(CONTAIN),
            "    kill -9 -1 2> /dev/null\n",
            "    true || exit 47\n    kill -9 -1 2> /dev/null\n"
        )
    )
);

mod qa_cap_one_way {
    use super::*;
    negative_control!(
        qa_win_contain_probe_matches_contain_sh,
        "a granted over-cap commit passing 44",
        expected = "doesn't hold the cap at its boundary both ways",
        check_one_for_one(
            &edited(
                &read(SCRIPT),
                " -or $refused -eq 0) { exit 44 }",
                ") { exit 44 }"
            ),
            &read(CONTAIN)
        )
    );
}

mod qa_limits_41 {
    use super::*;
    negative_control!(
        qa_win_contain_probe_matches_contain_sh,
        "41 not comparing the memory cap",
        expected = "scenario 41 doesn't compare every limit",
        check_one_for_one(
            &edited(
                &read(SCRIPT),
                "\n                $l.ProcessMemoryLimit.ToUInt64() -ne [uint64]$MemoryCap -or",
                "\n               "
            ),
            &read(CONTAIN)
        )
    );
}

// ---- Nothing left: every process the test starts is one the check lists ------------------------------------------

fn check_all_marked(script: &str) {
    for verb in ["probe", "hold"] {
        let body = arm(script, verb);
        let spawns = body.matches("[PrinContain]::Spawn(").count();
        let marked = body.matches("Ping-Line $LeakMark)").count();
        assert!(spawns > 0, "{SCRIPT}'s {verb} starts nothing");
        assert_eq!(
            spawns, marked,
            "{SCRIPT}'s {verb} starts a process the leftover lister can't find"
        );
    }
    let check = arm(script, "check");
    // After each mode and after the kill, a marked process left fails the check.
    assert!(
        check.contains("$leak = Get-Marked $LeakMark\n            if ($leak) {")
            && check
                .contains("Fail \"check: $($leak.Count) process(es) outlived ``$($mode.Name)``\""),
        "{SCRIPT}'s check doesn't fail on a process left after a mode"
    );
    assert!(
        check.contains("Fail \"check: $($leak.Count) process(es) outlived the job's last handle\""),
        "{SCRIPT}'s check doesn't fail on a process left after kill-on-close"
    );
    // The lister would see a leftover: a marked process already running fails the check before it starts.
    assert!(
        check.contains("if (Get-Marked $LeakMark) { Fail "),
        "{SCRIPT}'s check doesn't start from no marked process"
    );
    // The check's pass is its last word: one exit 0 past the control, after kill-on-close.
    let ok = check.rfind("exit 0").expect("the check never passes");
    let kill = check
        .find("kill-on-close holds")
        .expect("the check never reports kill-on-close");
    assert!(
        check.matches("exit 0").count() == 2 && kill < ok,
        "{SCRIPT}'s check can pass before kill-on-close is proved"
    );
}

#[test]
fn qa_win_contain_leftovers_are_listed() {
    check_all_marked(&read(SCRIPT));
}

negative_control!(
    qa_win_contain_leftovers_are_listed,
    "the probe's detached child unmarked",
    expected = "probe starts a process the leftover lister can't find",
    check_all_marked(&edited(
        &read(SCRIPT),
        "$left = [PrinContain]::Spawn((Ping-Line $LeakMark), 0x08000000)",
        "$left = [PrinContain]::Spawn((Ping-Line '127.0.0.9'), 0x08000000)"
    ))
);

mod qa_mode_leak {
    use super::*;
    negative_control!(
        qa_win_contain_leftovers_are_listed,
        "a leftover after a mode only reported",
        expected = "doesn't fail on a process left after a mode",
        check_all_marked(&edited(
            &read(SCRIPT),
            "Fail \"check: $($leak.Count) process(es) outlived ``$($mode.Name)``\"",
            "\"check: $($leak.Count) process(es) outlived ``$($mode.Name)``\""
        ))
    );
}

mod qa_early_pass {
    use super::*;
    negative_control!(
        qa_win_contain_leftovers_are_listed,
        "the check passing before the kill-on-close scenario",
        expected = "can pass before kill-on-close is proved",
        check_all_marked(&edited(
            &read(SCRIPT),
            "        # Kill-on-close: this script's stand-in",
            "        exit 0\n        # Kill-on-close: this script's stand-in"
        ))
    );
}

// ---- The job: the runner's, dispatched alone ---------------------------------------------------------------------

fn check_job(workflow: &str, operations: &str) {
    let doc = YamlLoader::load_from_str(workflow)
        .unwrap_or_else(|e| panic!("{WORKFLOW}: {e}"))
        .remove(0);
    let job = &doc["jobs"][JOB];
    assert!(!job.is_badvalue(), "{WORKFLOW} has no job {JOB}");
    let labels: Vec<&str> = job["runs-on"]
        .as_vec()
        .map(|v| v.iter().filter_map(|y| y.as_str()).collect())
        .unwrap_or_default();
    assert!(
        labels.contains(&"self-hosted") && labels.contains(&"windows"),
        "{WORKFLOW}/{JOB} isn't on the self-hosted Windows runner, so ci_self_hosted.rs's rules don't reach it"
    );
    // A conjunction, every term of which must hold: one of them the dispatch, one naming this job alone.
    let cond = job["if"].as_str().expect("the job has no `if`");
    assert!(
        !cond.contains("||") && !cond.contains('!'),
        "{WORKFLOW}/{JOB}'s condition can hold on other than its own dispatch: {cond}"
    );
    let terms: Vec<&str> = cond.split("&&").map(str::trim).collect();
    for needed in [
        "github.event_name == 'workflow_dispatch'",
        "inputs.only == 'win-contain'",
        "github.repository == 'Ma1achy/Principia'",
        "github.actor == 'Ma1achy'",
    ] {
        assert!(
            terms.contains(&needed),
            "{WORKFLOW}/{JOB}'s condition lacks `{needed}`: {cond}"
        );
    }
    assert!(
        job["timeout-minutes"].as_i64().is_some_and(|m| m > 0),
        "{WORKFLOW}/{JOB} has no time limit"
    );
    assert!(
        operations.contains("gh workflow run win-gpu.yml --ref main -f only=win-contain"),
        "{OPERATIONS} doesn't say how to trigger the proof"
    );
}

#[test]
fn qa_win_contain_job_dispatch_alone() {
    check_job(&read(WORKFLOW), &read(OPERATIONS));
}

negative_control!(
    qa_win_contain_job_dispatch_alone,
    "the job also run nightly",
    expected = "can hold on other than its own dispatch",
    check_job(
        &edited(
            &read(WORKFLOW),
            "github.event_name == 'workflow_dispatch' && inputs.only == 'win-contain'",
            "(github.event_name == 'schedule' || github.event_name == 'workflow_dispatch') && inputs.only == 'win-contain'"
        ),
        &read(OPERATIONS)
    )
);

mod qa_not_push {
    use super::*;
    negative_control!(
        qa_win_contain_job_dispatch_alone,
        "the job run on anything but a push",
        expected = "can hold on other than its own dispatch",
        check_job(
            &edited(
                &read(WORKFLOW),
                "github.event_name == 'workflow_dispatch' && inputs.only == 'win-contain'",
                "github.event_name != 'push' && inputs.only == 'win-contain'"
            ),
            &read(OPERATIONS)
        )
    );
}

mod qa_hosted {
    use super::*;
    negative_control!(
        qa_win_contain_job_dispatch_alone,
        "the job moved to a hosted runner",
        expected = "isn't on the self-hosted Windows runner",
        check_job(
            &edited(
                &read(WORKFLOW),
                "  win-contain:\n    if: github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && github.event_name == 'workflow_dispatch' && inputs.only == 'win-contain'\n    runs-on: [self-hosted, windows, nvidia]",
                "  win-contain:\n    if: github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && github.event_name == 'workflow_dispatch' && inputs.only == 'win-contain'\n    runs-on: [windows-2025]"
            ),
            &read(OPERATIONS)
        )
    );
}
