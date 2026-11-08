//! `.github/scripts/win_contain.ps1`, Windows' counterpart of `scripts/contain.sh` (the owner's CI shape order of 8 Nov
//! 2026, step 1: "run tests and mutants inside a Job Object with kill-on-close, a process limit and an 8 GiB per-process
//! memory cap (REQ-VAL-181's equivalent of prlimit). Prove it with the same kill test contain.sh check uses."), and
//! `win-gpu.yml`'s `win-contain`, the job on the self-hosted runner that proves it.
//!
//! Off Windows there is no PowerShell, so, as `ci_self_hosted.rs` does with the .git guard, these read the script and the
//! workflow: the limits it sets and their values, the order in which it starts its command, the scenarios its check runs,
//! and that the job runs only when dispatched for it alone. The proof itself is `win_contain.ps1 check`, run by that job on the runner. Each test has
//! its negative controls (R-176).

use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const SCRIPT: &str = ".github/scripts/win_contain.ps1";
const CONTAIN: &str = "scripts/contain.sh";
const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const JOB: &str = "win-contain";
/// The job's whole condition: the runner's guard, then a dispatch naming this job alone; never a push, the schedule or
/// `all`, until the proof has passed.
const DISPATCH_ONLY: &str = "github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && github.event_name == 'workflow_dispatch' && inputs.only == 'win-contain'";
/// The check's step, started fail closed through the record, as `ci_self_hosted.rs` asks of every script.
const CHECK_CALL: &str = "powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\win_measure.ps1 run contain-check powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\win_contain.ps1 check";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `text` with `from` replaced by `to`, once; the edit's target must be there.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the edit's target is gone: {from}");
    text.replacen(from, to, 1)
}

/// The number after `prefix` on the line that starts with it, after leading spaces.
fn number_after(text: &str, prefix: &str) -> Option<u64> {
    text.lines()
        .find_map(|l| l.trim_start().strip_prefix(prefix))
        .and_then(|rest| {
            rest.split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|n| n.parse().ok())
        })
}

/// The number in contain.sh's `prlimit --as=` probe: REQ-VAL-181's cap as Linux CI applies it.
fn prlimit_cap(contain: &str) -> u64 {
    let at = contain
        .find("prlimit --as=")
        .unwrap_or_else(|| panic!("{CONTAIN} has no `prlimit --as=`"));
    contain[at + "prlimit --as=".len()..]
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{CONTAIN}'s `prlimit --as=` has no number"))
}

// ---- The three limits ---------------------------------------------------------------------------------------------

fn check_limits(script: &str, contain: &str) {
    for line in [
        "public const uint KillOnJobClose = 0x2000;",
        "public const uint ActiveProcess = 0x8;",
        "public const uint ProcessMemory = 0x100;",
        "public const uint Limits = KillOnJobClose | ActiveProcess | ProcessMemory;",
        "info.Basic.LimitFlags = Limits;",
        "info.Basic.ActiveProcessLimit = processLimit;",
        "info.ProcessMemoryLimit = new UIntPtr(memoryCap);",
    ] {
        assert!(
            script.lines().any(|l| l.trim() == line),
            "{SCRIPT} doesn't set the job's limits as the order asks: no `{line}`"
        );
    }
    let cap = number_after(script, "$MemoryCap = ").expect("no $MemoryCap");
    assert_eq!(
        cap,
        prlimit_cap(contain),
        "{SCRIPT}'s memory cap isn't REQ-VAL-181's, as {CONTAIN} applies it"
    );
    let limit = number_after(script, "$ProcessLimit = ").expect("no $ProcessLimit");
    let low = number_after(script, "$CheckProcessLimit = ").expect("no $CheckProcessLimit");
    assert!(
        0 < low && low < limit,
        "{SCRIPT}'s check doesn't test the process limit below the one runs get: {low} against {limit}"
    );
}

#[test]
fn win_contain_sets_the_three_limits() {
    check_limits(&read(SCRIPT), &read(CONTAIN));
}

negative_control!(
    win_contain_sets_the_three_limits,
    "kill-on-close dropped from the job's flags",
    expected = "doesn't set the job's limits as the order asks",
    check_limits(
        &edited(
            &read(SCRIPT),
            "Limits = KillOnJobClose | ActiveProcess | ProcessMemory;",
            "Limits = ActiveProcess | ProcessMemory;"
        ),
        &read(CONTAIN)
    )
);

mod cap {
    use super::*;
    negative_control!(
        win_contain_sets_the_three_limits,
        "a cap one byte off REQ-VAL-181's",
        expected = "memory cap isn't REQ-VAL-181's",
        check_limits(
            &edited(
                &read(SCRIPT),
                "$MemoryCap = 8589934592",
                "$MemoryCap = 8589934593"
            ),
            &read(CONTAIN)
        )
    );
}

mod low_limit {
    use super::*;
    negative_control!(
        win_contain_sets_the_three_limits,
        "the check's low limit at the runs' limit, so scenario 45 never runs",
        expected = "doesn't test the process limit below the one runs get",
        check_limits(
            &edited(
                &read(SCRIPT),
                "$CheckProcessLimit = 6",
                "$CheckProcessLimit = 1024"
            ),
            &read(CONTAIN)
        )
    );
}

// ---- The command is in the job before it runs a line ----------------------------------------------------------------

/// The body of the C# `Run`.
fn run_body(script: &str) -> &str {
    let start = script
        .find("public static int Run(")
        .unwrap_or_else(|| panic!("{SCRIPT} has no Run"));
    let end = script[start..]
        .find("\n    }\n")
        .unwrap_or_else(|| panic!("{SCRIPT}'s Run has no end"));
    &script[start..start + end]
}

fn check_start_order(script: &str) {
    let body = run_body(script);
    // Whether `a` comes before `b` in Run, both being there.
    let before = |a: &str, b: &str| match (body.find(a), body.find(b)) {
        (Some(a), Some(b)) => a < b,
        _ => false,
    };
    assert!(
        body.contains("CreateSuspended | CreateNewProcessGroup | CreateNoWindow"),
        "{SCRIPT} doesn't start its command suspended, in a group and a console of its own"
    );
    assert!(
        before("NewJob(", "Start(")
            && before("Start(", "AssignProcessToJobObject(")
            && before("AssignProcessToJobObject(", "ResumeThread("),
        "{SCRIPT} lets its command run before it is in the job"
    );
    assert!(
        before("WaitForSingleObject(", "TerminateJobObject(")
            && before("TerminateJobObject(", "ActiveProcesses(job)"),
        "{SCRIPT} doesn't end what its command left, and wait for it"
    );
}

#[test]
fn win_contain_starts_its_command_inside_the_job() {
    check_start_order(&read(SCRIPT));
}

negative_control!(
    win_contain_starts_its_command_inside_the_job,
    "the command resumed before it is assigned to the job",
    expected = "lets its command run before it is in the job",
    check_start_order(&{
        let script = read(SCRIPT);
        let body = run_body(&script).to_owned();
        let assign = "        if (!AssignProcessToJobObject(job, pi.Process)) {";
        let resume = "        if (ResumeThread(pi.Thread) == 0xFFFFFFFF) {";
        let swapped = edited(&edited(&body, assign, "@@ASSIGN@@"), resume, "@@RESUME@@")
            .replacen("@@ASSIGN@@", resume, 1)
            .replacen("@@RESUME@@", assign, 1);
        script.replacen(&body, &swapped, 1)
    })
);

mod suspended {
    use super::*;
    negative_control!(
        win_contain_starts_its_command_inside_the_job,
        "the command started running",
        expected = "doesn't start its command suspended",
        check_start_order(&edited(
            &read(SCRIPT),
            "CreateSuspended | CreateNewProcessGroup | CreateNoWindow",
            "CreateNewProcessGroup | CreateNoWindow"
        ))
    );
}

mod left {
    use super::*;
    negative_control!(
        win_contain_starts_its_command_inside_the_job,
        "nothing ends what the command left",
        expected = "doesn't end what its command left",
        check_start_order(&edited(
            &read(SCRIPT),
            "        TerminateJobObject(job, 1);\n",
            ""
        ))
    );
}

// ---- The check runs contain.sh check's scenarios --------------------------------------------------------------------

/// The text of the script's `<verb>` arm.
fn arm<'a>(script: &'a str, verb: &str) -> &'a str {
    let head = format!("\n    '{verb}' {{\n");
    let start = script
        .find(&head)
        .unwrap_or_else(|| panic!("{SCRIPT} has no `{verb}`"));
    let end = script[start + 1..]
        .find("\n    }\n")
        .unwrap_or_else(|| panic!("{SCRIPT}'s `{verb}` has no end"));
    &script[start..start + 1 + end]
}

fn check_scenarios(script: &str) {
    let probe = arm(script, "probe");
    for code in [41, 43, 44, 45, 46] {
        assert!(
            probe.contains(&format!("exit {code}")),
            "{SCRIPT}'s probe never fails scenario {code}"
        );
    }
    assert!(
        probe.contains("probe: 42 gap"),
        "{SCRIPT}'s probe doesn't report scenario 42, the gap"
    );
    let check = arm(script, "check");
    for (mode, what) in [
        (
            "$shell + 'run' + $shell + @('probe', $Outer.Id, $ProcessLimit, 1)",
            "run",
        ),
        (
            "$shell + 'run' + $shell + 'run' + $shell + @('probe', $Outer.Id, $ProcessLimit, 2)",
            "run nested in run",
        ),
        (
            "$shell + 'run' + $shell + @('probe', $Outer.Id, $CheckProcessLimit, 1)",
            "run at the low limit",
        ),
        ("$shell + 'run' + $shell + 'hold'", "kill-on-close"),
    ] {
        assert!(
            check.contains(mode),
            "{SCRIPT}'s check doesn't run the mode {what}"
        );
    }
    for (step, what) in [
        (
            "Get-Marked $OuterMark",
            "the control finding the outside process",
        ),
        (
            "Commit([uint64]($MemoryCap + 65536))",
            "the control's commit",
        ),
        ("$leak = Get-Marked $LeakMark", "the leftover listing"),
        (
            "Stop-Process -Id $wrapper.Id -Force\n",
            "the run's process killed",
        ),
    ] {
        assert!(check.contains(step), "{SCRIPT}'s check lacks {what}");
    }
    assert!(
        check.contains("if ($env:PRIN_CONTAIN -ne 'required')")
            && arm(script, "run").contains("if ($env:PRIN_CONTAIN -ne 'required')"),
        "{SCRIPT} doesn't take PRIN_CONTAIN=required as contain.sh does"
    );
}

#[test]
fn win_contain_check_runs_every_scenario() {
    check_scenarios(&read(SCRIPT));
}

negative_control!(
    win_contain_check_runs_every_scenario,
    "the breakaway scenario dropped from the probe",
    expected = "probe never fails scenario 46",
    check_scenarios(&read(SCRIPT).replace("exit 46", "exit 0"))
);

mod nested {
    use super::*;
    negative_control!(
        win_contain_check_runs_every_scenario,
        "the nested mode dropped",
        expected = "doesn't run the mode run nested in run",
        check_scenarios(&edited(
            &read(SCRIPT),
            "$shell + 'run' + $shell + 'run' + $shell + @('probe', $Outer.Id, $ProcessLimit, 2)",
            "$shell + 'run' + $shell + @('probe', $Outer.Id, $ProcessLimit, 2)"
        ))
    );
}

mod close {
    use super::*;
    negative_control!(
        win_contain_check_runs_every_scenario,
        "the run's process never killed",
        expected = "lacks the run's process killed",
        check_scenarios(&edited(
            &read(SCRIPT),
            "Stop-Process -Id $wrapper.Id -Force\n",
            "\n"
        ))
    );
}

// ---- Scenario 45 gives its slots back; 46 counts only the breakaway refusal -------------------------------------------

/// The text of `text` from `from` to the next `to` after it, or None if either is missing.
fn between<'a>(text: &'a str, from: &str, to: &str) -> Option<&'a str> {
    let start = text.find(from)?;
    let end = text[start..].find(to)?;
    Some(&text[start..start + end])
}

fn check_slots(script: &str) {
    let probe = arm(script, "probe");
    // Scenario 45, from its base count to the console read that starts 43.
    let s45 = between(
        probe,
        "$base = [PrinContain]::ActiveProcesses",
        "probe: 43:",
    )
    .unwrap_or("");
    assert!(
        s45.contains("if ($child -gt 0) { $started += $child }")
            && s45.contains("foreach ($c in $started) { [PrinContain]::Kill($c) | Out-Null }"),
        "{SCRIPT}'s scenario 45 doesn't end the children it started"
    );
    assert!(
        s45.contains(
            "while (($active = [PrinContain]::ActiveProcesses([IntPtr]::Zero)) -gt $base)"
        ),
        "{SCRIPT}'s scenario 45 doesn't wait for its slots to come back"
    );
    assert_eq!(
        number_after(script, "$BreakawayRefused = "),
        Some(5),
        "{SCRIPT} doesn't take ERROR_ACCESS_DENIED as the breakaway refusal"
    );
    // Scenario 46's breakaway start, from its slot check to the child it leaves behind.
    let s46 = between(probe, "probe: 46: no slot free", "$left = ").unwrap_or("");
    assert!(
        probe.contains("if ($active -ge [int]$limit) { \"probe: 46: no slot free"),
        "{SCRIPT}'s scenario 46 doesn't make sure a slot is free"
    );
    assert!(
        s46.contains("if ($broke -ne -$BreakawayRefused) {"),
        "{SCRIPT}'s scenario 46 accepts any refusal of the breakaway start"
    );
}

#[test]
fn win_contain_low_limit_frees_its_slots() {
    check_slots(&read(SCRIPT));
}

negative_control!(
    win_contain_low_limit_frees_its_slots,
    "scenario 45's children left holding the job's slots",
    expected = "doesn't end the children it started",
    check_slots(&edited(
        &read(SCRIPT),
        "foreach ($c in $started) { [PrinContain]::Kill($c) | Out-Null }",
        ""
    ))
);

mod slot_wait {
    use super::*;
    negative_control!(
        win_contain_low_limit_frees_its_slots,
        "scenario 45 not waiting for its slots",
        expected = "doesn't wait for its slots to come back",
        check_slots(&edited(&read(SCRIPT), ")) -gt $base)", ")) -gt $base + 1)"))
    );
}

mod any_refusal {
    use super::*;
    negative_control!(
        win_contain_low_limit_frees_its_slots,
        "scenario 46 accepting the process limit's refusal",
        expected = "accepts any refusal of the breakaway start",
        check_slots(&edited(
            &read(SCRIPT),
            "if ($broke -ne -$BreakawayRefused) {",
            "if ($false) {"
        ))
    );
}

mod refusal_code {
    use super::*;
    negative_control!(
        win_contain_low_limit_frees_its_slots,
        "the process limit's error taken as the breakaway refusal",
        expected = "doesn't take ERROR_ACCESS_DENIED",
        check_slots(&edited(
            &read(SCRIPT),
            "$BreakawayRefused = 5",
            "$BreakawayRefused = 1816"
        ))
    );
}

mod no_free_slot {
    use super::*;
    negative_control!(
        win_contain_low_limit_frees_its_slots,
        "scenario 46 run with the job full",
        expected = "doesn't make sure a slot is free",
        check_slots(&edited(
            &read(SCRIPT),
            "if ($active -ge [int]$limit) { \"probe: 46: no slot free",
            "if ($false) { \"probe: 46: no slot free"
        ))
    );
}

/// Run's failure paths after the start close the process and thread handles; NUL's handle is checked and closed.
fn check_handles(script: &str) {
    let body = run_body(script);
    for what in ["AssignProcessToJobObject", "ResumeThread"] {
        let path = between(
            body,
            &format!("Exception e = Error(\"{what}\");"),
            "throw e;",
        )
        .unwrap_or("");
        assert!(
            path.contains("CloseHandle(pi.Thread);") && path.contains("CloseHandle(pi.Process);"),
            "{SCRIPT}'s Run leaves the handles open when {what} fails"
        );
    }
    assert!(
        script.contains("if (h == new IntPtr(-1)) throw Error(\"CreateFile NUL\");")
            && script.contains("if (nul != IntPtr.Zero) CloseHandle(nul);"),
        "{SCRIPT} doesn't check and close NUL's handle"
    );
}

#[test]
fn win_contain_closes_its_handles() {
    check_handles(&read(SCRIPT));
}

negative_control!(
    win_contain_closes_its_handles,
    "the handles left open when ResumeThread fails",
    expected = "leaves the handles open when ResumeThread fails",
    check_handles(&edited(
        &read(SCRIPT),
        "Exception e = Error(\"ResumeThread\");\n            CloseHandle(pi.Thread);\n            CloseHandle(pi.Process);\n",
        "Exception e = Error(\"ResumeThread\");\n"
    ))
);

mod nul {
    use super::*;
    negative_control!(
        win_contain_closes_its_handles,
        "NUL's handle never closed",
        expected = "doesn't check and close NUL's handle",
        check_handles(&edited(
            &read(SCRIPT),
            "if (nul != IntPtr.Zero) CloseHandle(nul);",
            ""
        ))
    );
}

// ---- The job: dispatch only, running the check ----------------------------------------------------------------------

fn check_workflow(text: &str) {
    let doc = YamlLoader::load_from_str(text)
        .unwrap_or_else(|e| panic!("{WORKFLOW}: {e}"))
        .remove(0);
    let job = &doc["jobs"][JOB];
    assert!(!job.is_badvalue(), "{WORKFLOW} has no job {JOB}");
    assert_eq!(
        job["if"].as_str(),
        Some(DISPATCH_ONLY),
        "{WORKFLOW}/{JOB} runs on more than its own dispatch before its proof has passed"
    );
    let options: Vec<&str> = doc["on"]["workflow_dispatch"]["inputs"]["only"]["options"]
        .as_vec()
        .map(|o| o.iter().filter_map(Yaml::as_str).collect())
        .unwrap_or_default();
    assert!(
        options.contains(&JOB),
        "{WORKFLOW}'s dispatch can't name {JOB}"
    );
    let steps = job["steps"].as_vec().cloned().unwrap_or_default();
    let check = steps
        .iter()
        .find(|s| {
            s["run"]
                .as_str()
                .is_some_and(|r| r.contains("win_contain.ps1"))
        })
        .unwrap_or_else(|| panic!("{WORKFLOW}/{JOB} doesn't run the check"));
    assert_eq!(
        check["run"].as_str().and_then(|r| r.lines().next()),
        Some(CHECK_CALL),
        "{WORKFLOW}/{JOB} doesn't run the check through the record"
    );
    assert_eq!(
        check["env"]["PRIN_CONTAIN"].as_str(),
        Some("required"),
        "{WORKFLOW}/{JOB}'s check doesn't require containment, so it would pass having proved nothing"
    );
}

#[test]
fn win_contain_job_is_dispatch_only_and_runs_the_check() {
    check_workflow(&read(WORKFLOW));
}

negative_control!(
    win_contain_job_is_dispatch_only_and_runs_the_check,
    "the job let run on a push and the schedule before the proof",
    expected = "runs on more than its own dispatch",
    check_workflow(&edited(
        &read(WORKFLOW),
        " && github.event_name == 'workflow_dispatch' && inputs.only == 'win-contain'",
        ""
    ))
);

mod all {
    use super::*;
    negative_control!(
        win_contain_job_is_dispatch_only_and_runs_the_check,
        "the job let run on a dispatch of `all`",
        expected = "runs on more than its own dispatch",
        check_workflow(&edited(
            &read(WORKFLOW),
            "&& inputs.only == 'win-contain'",
            "&& (inputs.only == 'all' || inputs.only == 'win-contain')"
        ))
    );
}

mod not_required {
    use super::*;
    negative_control!(
        win_contain_job_is_dispatch_only_and_runs_the_check,
        "the check run without PRIN_CONTAIN, which passes saying containment isn't required",
        expected = "doesn't require containment",
        check_workflow(&edited(
            &read(WORKFLOW),
            "          PRIN_CONTAIN: required\n",
            "          PRIN_CONTAIN: optional\n"
        ))
    );
}
