//! The Windows runner's WSL default route (TASK-M0-72; the PC's ask on #390, 14 Nov 2026, and the owner's OK that day
//! to close it). A launcher that names no distro, System32's `bash.exe` or `wsl.exe` without `-d`, goes to the account's
//! default distribution, and so reached `principia-wsl` from a test that started a bare `bash` (run 43593938536). The PC
//! pointed gh-runner's default at a GUID no distro has; `wsl --import` makes the distro it imports the default again.
//!
//! - `wsl.ps1 register`, once the import is verified, points gh-runner's HKCU Lxss `DefaultDistribution` at
//!   `{00000000-0000-0000-0000-0000000000de}`, after printing the value it had, reads it back, and fails the job if it
//!   can't.
//! - `.github/scripts/wsl_route.ps1`, a step of `win-self-check` each night, runs as gh-runner System32's
//!   `bash.exe -c true` and `wsl.exe -e true`, naming no distro: each must fail, every distro's state must read the same
//!   after as before (Stopped staying Stopped), and none may be the default. Its control runs with it on the PC: the
//!   same verdict on four simulated open routes must call each open, or the step fails.
//!
//! Off Windows there is no PowerShell, so, as `wsl.rs` does, these read the scripts and the workflow: the statements
//! the verdict and the register's write are made of, and the order of the check's steps. Each test has its negative
//! controls (R-176).

use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::YamlLoader;

const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const PS: &str = ".github/scripts/wsl.ps1";
const ROUTE: &str = ".github/scripts/wsl_route.ps1";
/// The job that runs the check, and its step's two lines.
const JOB: &str = "win-self-check";
const STEP: &str = "powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\wsl_route.ps1\nexit $LASTEXITCODE\n";

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

/// The text of `verb`'s arm of wsl.ps1's switch, through to the next arm.
fn arm<'a>(script: &'a str, verb: &str) -> &'a str {
    let start = script
        .find(&format!("\n    '{verb}' {{\n"))
        .unwrap_or_else(|| panic!("{PS} has no `{verb}` arm"));
    let rest = &script[start + 1..];
    let end = rest[1..]
        .find("\n    '")
        .or_else(|| rest.find("\n    default {"))
        .map_or(rest.len(), |e| e + 1);
    &rest[..end]
}

/// The text of the function `name`, from `function name` through its closing brace at the start of a line.
fn function<'a>(script: &'a str, name: &str) -> &'a str {
    let start = script
        .find(&format!("\nfunction {name}"))
        .unwrap_or_else(|| panic!("no function {name}"));
    let rest = &script[start + 1..];
    &rest[..rest.find("\n}\n").expect("the function ends") + 2]
}

/// The check's own statements: wsl_route.ps1 from its marker comment on, after its functions.
fn check_body(script: &str) -> &str {
    let start = script
        .find("\n# The check: ")
        .unwrap_or_else(|| panic!("{ROUTE} has no `# The check: ` marker"));
    &script[start..]
}

/// The statements of `body`, trimmed, without blank lines and comments.
fn statements(body: &str) -> Vec<&str> {
    body.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

/// Where `needle` first is in `text`, or a panic naming `what`.
fn at(text: &str, needle: &str, what: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("{what}: no `{needle}`"))
}

// ---- The step: nightly, on the runner, its failure the job's --------------------------------------------------------

/// `win-self-check` runs `wsl_route.ps1` once, in a step of its own after the checkout, fail closed, with no `if` and no
/// `continue-on-error`, so that an open route fails the job.
fn check_step(workflow: &str) {
    let doc = YamlLoader::load_from_str(workflow)
        .expect("the workflow parses")
        .remove(0);
    let steps = doc["jobs"][JOB]["steps"].as_vec().expect("its steps");
    let checkout = steps
        .iter()
        .position(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/checkout@"))
        })
        .expect("a checkout");
    let found: Vec<usize> = steps
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s["run"]
                .as_str()
                .is_some_and(|r| r.contains("wsl_route.ps1"))
        })
        .map(|(i, _)| i)
        .collect();
    let [index] = found[..] else {
        panic!("{JOB} doesn't run wsl_route.ps1 once: {found:?}");
    };
    let step = &steps[index];
    assert!(index > checkout, "{JOB} runs the check before its checkout");
    assert_eq!(
        step["run"].as_str(),
        Some(STEP),
        "{JOB}'s check doesn't pass its exit on"
    );
    assert!(
        step["if"].is_badvalue() && step["continue-on-error"].is_badvalue(),
        "{JOB}'s check can be skipped or tolerated"
    );
}

#[test]
fn wsl_route_check_runs_in_win_self_check() {
    check_step(&read(WORKFLOW));
}

negative_control!(
    wsl_route_check_runs_in_win_self_check,
    "the check's step gone",
    expected = "doesn't run wsl_route.ps1 once",
    check_step(&edited(
        &read(WORKFLOW),
        "wsl_route.ps1\n",
        "wsl.ps1 proofs\n"
    ))
);

mod step_tolerated {
    use super::*;
    negative_control!(
        wsl_route_check_runs_in_win_self_check,
        "an open route tolerated",
        expected = "can be skipped or tolerated",
        check_step(&edited(
            &read(WORKFLOW),
            "      - name: No bash or wsl naming no distro reaches one\n",
            "      - name: No bash or wsl naming no distro reaches one\n        continue-on-error: true\n"
        ))
    );
}

mod step_exit_dropped {
    use super::*;
    negative_control!(
        wsl_route_check_runs_in_win_self_check,
        "the script's exit not passed on",
        expected = "doesn't pass its exit on",
        check_step(&edited(
            &read(WORKFLOW),
            "wsl_route.ps1\n          exit $LASTEXITCODE\n",
            "wsl_route.ps1\n          exit 0\n"
        ))
    );
}

mod step_early {
    use super::*;
    /// The check's step, as `win-self-check` holds it.
    #[cfg(feature = "controls")]
    const THE_STEP: &str = "      - name: No bash or wsl naming no distro reaches one\n        run: |\n          powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\wsl_route.ps1\n          exit $LASTEXITCODE\n";
    negative_control!(
        wsl_route_check_runs_in_win_self_check,
        "the check before the checkout, so it runs a script the checkout hasn't written",
        expected = "runs the check before its checkout",
        check_step(&edited(
            &edited(&read(WORKFLOW), THE_STEP, ""),
            "      - name: Toolchain on PATH\n        run: |\n          Add-Content -LiteralPath $env:GITHUB_PATH -Value 'E:\\runner-tools\\cargo\\bin'\n          exit 0\n      - name: Clean checkout\n        id: checkout\n        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n        with:\n          persist-credentials: false\n          fetch-depth: 1\n          clean: true\n      - name: Record the commit, driver and toolchain\n        run: |\n          powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\win_measure.ps1 start\n          exit $LASTEXITCODE\n      - name: No CRLF",
            &format!("{THE_STEP}      - name: Toolchain on PATH\n        run: |\n          Add-Content -LiteralPath $env:GITHUB_PATH -Value 'E:\\runner-tools\\cargo\\bin'\n          exit 0\n      - name: Clean checkout\n        id: checkout\n        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n        with:\n          persist-credentials: false\n          fetch-depth: 1\n          clean: true\n      - name: Record the commit, driver and toolchain\n        run: |\n          powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\win_measure.ps1 start\n          exit $LASTEXITCODE\n      - name: No CRLF")
        ))
    );
}

// ---- The check: bare launchers, between two readings of every distro's state ----------------------------------------

/// The check stops the distro and reads every distro's state, failing if they can't be read or one is running; then
/// probes System32's `bash.exe -c true` and `wsl.exe -e true`, naming no distro; reads the states again; prints what
/// each probe and reading gave; and exits 0 only after the verdict on them has said closed.
fn check_probes(script: &str) {
    let body = check_body(script);
    let what = ROUTE;
    assert!(
        script.contains(
            "\n$System = Join-Path $env:SystemRoot 'System32'\n$Wsl = Join-Path $System 'wsl.exe'\n"
        ),
        "the probes' wsl.exe isn't System32's"
    );
    let order = [
        "$stopped = Invoke-Program $Wsl \"--terminate $Distro\"",
        "$before = Get-DistroStates",
        "if ($null -eq $before) { '::error::",
        "'; exit 1 }",
        "if (-not $before.Idle) { \"::error::",
        "\"; exit 1 }",
        "[pscustomobject]@{ Name = 'bash -c true'; Run = Invoke-Program (Join-Path $System 'bash.exe') '-c true' }",
        "[pscustomobject]@{ Name = 'wsl -e true'; Run = Invoke-Program $Wsl '-e true' }",
        "$after = Get-DistroStates",
        "$stopped = Invoke-Program $Wsl \"--terminate $Distro\"",
        "if ($null -eq $after) { '::error::",
        "'; exit 1 }",
        "foreach ($p in $probes) { \"probe $($p.Name), as $([Environment]::UserName), naming no distro: $($p.Run.Note)\" }",
        "if (-not (Test-RouteClosed @($probes | ForEach-Object { $_.Run }) $before $after)) { '::error::",
        "'; exit 1 }",
        "'wsl_route.ps1: closed: ",
        "\nexit 0\n",
    ];
    let mut last = 0;
    for needle in order {
        let found = body[last..].find(needle).unwrap_or_else(|| {
            panic!("{what} doesn't, in order, read, probe, read and judge: `{needle}`")
        });
        last += found + needle.len();
    }
    let probes = &body[at(body, "$probes = @(", what)..at(body, "$after = Get-DistroStates", what)];
    assert!(
        !probes.contains("-d ") && !probes.contains("$Distro"),
        "a probe names a distro, so it tests nothing of the default"
    );
    assert_eq!(
        body.matches("\nexit 0\n").count(),
        1,
        "{what} has a way to pass other than its verdict"
    );
}

#[test]
fn wsl_route_check_probes_bare_launchers_between_readings() {
    check_probes(&read(ROUTE));
}

negative_control!(
    wsl_route_check_probes_bare_launchers_between_readings,
    "a third probe, naming the distro, which tests nothing of the default",
    expected = "a probe names a distro",
    check_probes(&edited(
        &read(ROUTE),
        "\n$after = Get-DistroStates\n",
        "\n$x = Invoke-Program $Wsl '-d principia-wsl -e true'\n$after = Get-DistroStates\n"
    ))
);

mod after_read_first {
    use super::*;
    negative_control!(
        wsl_route_check_probes_bare_launchers_between_readings,
        "the second reading taken before the probes",
        expected = "doesn't, in order, read, probe, read and judge",
        check_probes(&edited(
            &edited(&read(ROUTE), "\n$after = Get-DistroStates\n", "\n"),
            "\n$probes = @(\n",
            "\n$after = Get-DistroStates\n$probes = @(\n"
        ))
    );
}

mod running_before {
    use super::*;
    negative_control!(
        wsl_route_check_probes_bare_launchers_between_readings,
        "a distro running before the probes accepted",
        expected = "`if (-not $before.Idle) { \"::error::`",
        check_probes(&edited(
            &read(ROUTE),
            "if (-not $before.Idle) { \"::error::",
            "if ($false) { \"::error::"
        ))
    );
}

mod one_probe {
    use super::*;
    negative_control!(
        wsl_route_check_probes_bare_launchers_between_readings,
        "the bash probe dropped",
        expected = "`[pscustomobject]@{ Name = 'bash -c true'",
        check_probes(&edited(
            &read(ROUTE),
            "    [pscustomobject]@{ Name = 'bash -c true'; Run = Invoke-Program (Join-Path $System 'bash.exe') '-c true' }\n",
            ""
        ))
    );
}

mod open_passes {
    use super::*;
    negative_control!(
        wsl_route_check_probes_bare_launchers_between_readings,
        "a pass before the probes have run",
        expected = "has a way to pass other than its verdict",
        check_probes(&edited(
            &read(ROUTE),
            "\n$probes = @(\n",
            "\nexit 0\n$probes = @(\n"
        ))
    );
}

mod probe_off_system32 {
    use super::*;
    negative_control!(
        wsl_route_check_probes_bare_launchers_between_readings,
        "the wsl probe's copy found on PATH, which may not be the launcher a test's Command::new finds",
        expected = "the probes' wsl.exe isn't System32's",
        check_probes(&edited(
            &read(ROUTE),
            "$Wsl = Join-Path $System 'wsl.exe'",
            "$Wsl = 'wsl.exe'"
        ))
    );
}

// ---- The verdict, and the control that it calls an open route open --------------------------------------------------

/// The verdict: closed only if no probe ran to exit 0 or hung, the states read the same after as before, and no distro
/// is the default after; a probe that never started or hung has no exit code, and a hung one's note says so; a reading
/// that misses a distro is none, and one with any distro not Stopped isn't idle.
fn check_verdict(script: &str) {
    assert_eq!(
        statements(function(script, "Test-RouteClosed")),
        [
            "function Test-RouteClosed($Probes, $Before, $After) {",
            "$ran = @($Probes | Where-Object { $_.Code -eq 0 -or $_.Note -like 'hung*' })",
            "$ran.Count -eq 0 -and $Before.Text -ceq $After.Text -and $After.Defaults.Count -eq 0",
            "}",
        ],
        "the verdict can call an open route closed"
    );
    let probe = function(script, "Invoke-Program");
    assert!(
        probe.contains("return [pscustomobject]@{ Code = $null; Lines = @(); Note = \"not started: $_\" }")
            && probe.contains("if (-not $p.WaitForExit(60000)) {\n        # It may end between the wait and the kill.\n        try { $p.Kill() } catch { }\n        return [pscustomobject]@{ Code = $null; Lines = @(); Note = 'hung: still running after 60 s' }")
            && probe.contains("[pscustomobject]@{ Code = $p.ExitCode; Lines = $lines; Note = "),
        "a probe's result doesn't say whether it ran, failed or hung"
    );
    let states = function(script, "Get-DistroStates");
    // A refusal: a line opening with `condition` that ends by returning nothing.
    let refuses = |condition: &str| {
        states
            .lines()
            .map(str::trim)
            .any(|l| l.starts_with(condition) && l.ends_with("; return $null }"))
    };
    assert!(
        refuses("if (@($rows | ForEach-Object { $_.Name }) -notcontains $name) { ")
            && refuses("if ($listed.Code -ne 0 -or $quiet.Code -ne 0) { ")
            && refuses("if (@($listed.Lines | Where-Object { $_ -match '^NAME\\s+STATE\\s+VERSION$' }).Count -eq 0) { ")
            && refuses("if ($rows.Count -eq 0) { ")
            && states.contains("$names = @($quiet.Lines)")
            && states.contains(
                "Defaults = @($rows | Where-Object { $_.Default } | ForEach-Object { $_.Name })"
            )
            && states
                .contains("Idle = @($rows | Where-Object { $_.State -cne 'Stopped' }).Count -eq 0")
            && states.contains("Default = $Matches[1] -eq '*'"),
        "a reading of the distros' states can miss a start or a default"
    );
    // Each refusal names its cause on the log, after both listings' exits and output, and returns nothing else; and
    // the check's two refusals point at it.
    assert!(
        states.contains("    Write-Host \"wsl --list --verbose: $($listed.Note)\"\n    Write-Host \"wsl --list --quiet: $($quiet.Note)\"\n    if ($listed.Code")
            && states.matches("return $null").count() == 4
            && states.matches("{ Write-Host '").count() + states.matches("{ Write-Host \"").count() == 4
            && states
                .lines()
                .filter(|l| l.contains("return $null"))
                .all(|l| l.contains("Write-Host 'the states are unread: ") || l.contains("Write-Host \"the states are unread: ")),
        "a reading refused leaves no cause in the log"
    );
    let body = check_body(script);
    for when in ["before", "after"] {
        assert!(
            body.contains(&format!("the distros'' states {when} the probes could not be read; the cause, and both listings, are printed above'; exit 1 }}")),
            "the refusal {when} the probes doesn't point at its cause"
        );
    }
}

#[test]
fn wsl_route_verdict_calls_an_open_route_open() {
    check_verdict(&read(ROUTE));
}

negative_control!(
    wsl_route_verdict_calls_an_open_route_open,
    "a distro started by a probe not counted",
    expected = "the verdict can call an open route closed",
    check_verdict(&edited(
        &read(ROUTE),
        " -and $Before.Text -ceq $After.Text",
        ""
    ))
);

mod hung_not_counted {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "a hung probe not counted as one that ran",
        expected = "the verdict can call an open route closed",
        check_verdict(&edited(
            &read(ROUTE),
            "$_.Code -eq 0 -or $_.Note -like 'hung*'",
            "$_.Code -eq 0"
        ))
    );
}

mod default_not_counted {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "a default distro not counted",
        expected = "the verdict can call an open route closed",
        check_verdict(&edited(
            &read(ROUTE),
            " -and $After.Defaults.Count -eq 0",
            ""
        ))
    );
}

mod hung_renamed {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "a hung probe's note that the verdict no longer matches",
        expected = "a probe's result doesn't say whether it ran, failed or hung",
        check_verdict(&edited(
            &read(ROUTE),
            "Note = 'hung: still running after 60 s' }\n    }",
            "Note = 'timed out after 60 s' }\n    }"
        ))
    );
}

mod idle_running {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "a reading that calls a running distro idle",
        expected = "a reading of the distros' states can miss a start or a default",
        check_verdict(&edited(
            &read(ROUTE),
            "$_.State -cne 'Stopped'",
            "$_.State -ceq 'Running!'"
        ))
    );
}

mod unlisted_passes {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "a reading that leaves out a distro the quiet list names",
        expected = "a reading of the distros' states can miss a start or a default",
        check_verdict(&edited(
            &read(ROUTE),
            "which wsl --list --quiet names\"; return $null }",
            "which wsl --list --quiet names\"; continue }"
        ))
    );
}

mod listing_exit_unread {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "a failed listing read as no distros, so empty states before and after compare equal",
        expected = "a reading of the distros' states can miss a start or a default",
        check_verdict(&edited(
            &read(ROUTE),
            "    if ($listed.Code -ne 0 -or $quiet.Code -ne 0) { ",
            "    if ($false) { "
        ))
    );
}

mod no_header_unread {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "a listing with no header row taken as read",
        expected = "a reading of the distros' states can miss a start or a default",
        check_verdict(&edited(
            &read(ROUTE),
            "printed no NAME STATE VERSION header'; return $null }",
            "printed no NAME STATE VERSION header' }"
        ))
    );
}

mod no_distro_unread {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "a listing of no distro taken as read",
        expected = "a reading of the distros' states can miss a start or a default",
        check_verdict(&edited(
            &read(ROUTE),
            "    if ($rows.Count -eq 0) { ",
            "    if ($false) { "
        ))
    );
}

mod cause_unprinted {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "a refusal that returns without naming its cause",
        expected = "a reading refused leaves no cause in the log",
        check_verdict(&edited(
            &read(ROUTE),
            "{ Write-Host 'the states are unread: wsl --list --verbose shows no distro, and principia-wsl is always registered'; return $null }",
            "{ $why = 'no distro'; return $null }"
        ))
    );
}

mod listings_unprinted {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "the listings' exits and output kept from the log",
        expected = "a reading refused leaves no cause in the log",
        check_verdict(&edited(
            &read(ROUTE),
            "    Write-Host \"wsl --list --quiet: $($quiet.Note)\"\n",
            ""
        ))
    );
}

mod refusal_blames_one_cause {
    use super::*;
    negative_control!(
        wsl_route_verdict_calls_an_open_route_open,
        "the refusal after the probes naming one fixed cause",
        expected = "the refusal after the probes doesn't point at its cause",
        check_verdict(&edited(
            &read(ROUTE),
            "the distros'' states after the probes could not be read; the cause, and both listings, are printed above'",
            "wsl --list --verbose shows not every distro wsl --list --quiet names after the probes'"
        ))
    );
}

/// The control, on the PC: the verdict on four simulated open routes, a probe that ran, one that hung, a distro
/// started and a default distro, each differing from a closed route in one thing, must call each open, before the
/// real verdict is reached; if it calls one closed the step fails.
fn check_control(script: &str) {
    let body = check_body(script);
    let what = "the check's control";
    for case in [
        "'a probe that ran' = Test-RouteClosed @([pscustomobject]@{ Code = 0; Note = 'exit 0' }) $idle $idle",
        "'a probe that hung' = Test-RouteClosed @([pscustomobject]@{ Code = $null; Note = 'hung: still running after 60 s' }) $idle $idle",
        "'a distro started' = Test-RouteClosed @($refused) $idle ([pscustomobject]@{ Text = \"$Distro=Running\"; Defaults = @() })",
        "'a default distro' = Test-RouteClosed @($refused) $idle ([pscustomobject]@{ Text = \"$Distro=Stopped\"; Defaults = @($Distro) })",
    ] {
        assert!(body.contains(case), "{what} doesn't simulate {case}");
    }
    assert!(
        body.contains("$idle = [pscustomobject]@{ Text = \"$Distro=Stopped\"; Defaults = @() }")
            && body.contains("$refused = [pscustomobject]@{ Code = 1; Note = 'exit 1' }"),
        "{what}'s cases differ from a closed route in more than one thing"
    );
    let blind = at(body, "if ($blind.Count -ne 0) { \"::error::", what);
    assert!(
        body.contains("$blind = @($simulated.Keys | Where-Object { $simulated[$_] })")
            && body[blind..]
                .lines()
                .next()
                .is_some_and(|l| l.trim_end().ends_with("; exit 1 }")),
        "{what} doesn't fail the step when the verdict calls a simulated open route closed"
    );
    assert!(
        blind < at(body, "if (-not (Test-RouteClosed @($probes", what),
        "{what} runs after the real verdict, which can then pass without it"
    );
}

#[test]
fn wsl_route_check_runs_its_control_on_the_pc() {
    check_control(&read(ROUTE));
}

negative_control!(
    wsl_route_check_runs_its_control_on_the_pc,
    "the default-distro case dropped",
    expected = "doesn't simulate 'a default distro'",
    check_control(&edited(
        &read(ROUTE),
        "    'a default distro' = ",
        "    'a default' = "
    ))
);

mod control_tolerated {
    use super::*;
    negative_control!(
        wsl_route_check_runs_its_control_on_the_pc,
        "a blind verdict reported and passed",
        expected = "doesn't fail the step when the verdict calls a simulated open route closed",
        check_control(&edited(
            &read(ROUTE),
            "so it proves nothing\"; exit 1 }",
            "so it proves nothing\" }"
        ))
    );
}

mod control_late {
    use super::*;
    negative_control!(
        wsl_route_check_runs_its_control_on_the_pc,
        "the control after the real verdict",
        expected = "runs after the real verdict",
        check_control(&edited(
            &edited(
                &read(ROUTE),
                "\nif (-not (Test-RouteClosed @($probes",
                "\nif ($false -and (Test-RouteClosed @($probes"
            ),
            "\n$idle = [pscustomobject]",
            "\nif (-not (Test-RouteClosed @($probes | ForEach-Object { $_.Run }) $before $after)) { exit 1 }\n$idle = [pscustomobject]"
        ))
    );
}

mod control_two_changes {
    use super::*;
    negative_control!(
        wsl_route_check_runs_its_control_on_the_pc,
        "the base route of the cases not a closed one",
        expected = "differ from a closed route in more than one thing",
        check_control(&edited(
            &read(ROUTE),
            "$refused = [pscustomobject]@{ Code = 1; Note = 'exit 1' }",
            "$refused = [pscustomobject]@{ Code = 0; Note = 'exit 0' }"
        ))
    );
}

// ---- register: the default pointed at no distro after every import --------------------------------------------------

/// `register` closes the default route once, after the imported distro is verified and before its success line; the
/// close prints the value it had, refuses if a distro has the GUID, writes the GUID, reads it back and fails unless it
/// is the GUID; the key and the GUID are gh-runner's Lxss and `{…00de}`.
fn check_register(script: &str) {
    let register = arm(script, "register");
    let what = "register";
    let import = at(
        register,
        "& wsl.exe --import $Distro $Disk $rootfs --version 2",
        what,
    );
    let unverified = at(register, "\n        if (-not $verified) {\n", what);
    let success = at(
        register,
        "\"registered ${Distro}: its manifest, derived inside it, is the pinned $pinned\"",
        what,
    );
    let close: Vec<usize> = register
        .match_indices("        Close-DefaultRoute\n")
        .map(|(i, _)| i)
        .collect();
    assert!(
        close.len() == 1 && import < unverified && unverified < close[0] && close[0] < success,
        "register doesn't close the default route once, after verifying the import and before it reports success"
    );
    assert!(
        script.contains("\n$Lxss = 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss'\n")
            && script.contains("\n$NoDistro = '{00000000-0000-0000-0000-0000000000de}'\n"),
        "the default is set somewhere other than gh-runner's Lxss key, or to another GUID"
    );
    assert_eq!(
        statements(function(script, "Close-DefaultRoute")),
        [
            "function Close-DefaultRoute {",
            "$previous = (Get-ItemProperty -LiteralPath $Lxss).DefaultDistribution",
            "\"gh-runner's default WSL distribution was $(if ($previous) { $previous } else { 'unset' })\"",
            "$registered = @(Get-ChildItem -LiteralPath $Lxss | ForEach-Object { $_.PSChildName })",
            "if ($registered -contains $NoDistro) { Fail \"a distro is registered as ${NoDistro}: gh-runner's default can't be pointed at no distro\" }",
            "try {",
            "Set-ItemProperty -LiteralPath $Lxss -Name DefaultDistribution -Value $NoDistro",
            "$now = (Get-ItemProperty -LiteralPath $Lxss).DefaultDistribution",
            "} catch {",
            "Fail \"setting gh-runner's default WSL distribution to $NoDistro failed, so the default route is open: $_\"",
            "}",
            "if ($now -cne $NoDistro) { Fail \"gh-runner's default WSL distribution reads back '$now', not ${NoDistro}: the default route is open\" }",
            "\"gh-runner's default WSL distribution is now $NoDistro, which no distro has\"",
            "}",
        ],
        "the close doesn't print the old default, write {{…00de}}, read it back and fail unless it holds"
    );
}

#[test]
fn wsl_register_closes_the_default_route() {
    check_register(&read(PS));
}

negative_control!(
    wsl_register_closes_the_default_route,
    "register leaves the default the import set",
    expected = "register doesn't close the default route once",
    check_register(&edited(&read(PS), "        Close-DefaultRoute\n", ""))
);

mod closed_before_import {
    use super::*;
    negative_control!(
        wsl_register_closes_the_default_route,
        "the close before the import, which then opens the route again",
        expected = "register doesn't close the default route once",
        check_register(&edited(
            &edited(&read(PS), "        Close-DefaultRoute\n", ""),
            "        New-Item -ItemType Directory -Force -Path $Disk | Out-Null\n",
            "        Close-DefaultRoute\n        New-Item -ItemType Directory -Force -Path $Disk | Out-Null\n"
        ))
    );
}

mod no_read_back {
    use super::*;
    negative_control!(
        wsl_register_closes_the_default_route,
        "the write trusted without reading it back",
        expected = "read it back and fail unless it holds",
        check_register(&edited(
            &read(PS),
            "    if ($now -cne $NoDistro) { Fail",
            "    if ($false) { Fail"
        ))
    );
}

mod other_guid {
    use super::*;
    negative_control!(
        wsl_register_closes_the_default_route,
        "the default pointed at another GUID",
        expected = "or to another GUID",
        check_register(&edited(
            &read(PS),
            "$NoDistro = '{00000000-0000-0000-0000-0000000000de}'",
            "$NoDistro = '{00000000-0000-0000-0000-0000000000df}'"
        ))
    );
}

mod old_value_unprinted {
    use super::*;
    negative_control!(
        wsl_register_closes_the_default_route,
        "the old default not printed to the log",
        expected = "the close doesn't print the old default",
        check_register(&edited(
            &read(PS),
            "    \"gh-runner's default WSL distribution was $(if ($previous) { $previous } else { 'unset' })\"\n",
            ""
        ))
    );
}

mod write_failure_tolerated {
    use super::*;
    negative_control!(
        wsl_register_closes_the_default_route,
        "a failed write carried on past",
        expected = "the close doesn't print the old default",
        check_register(&edited(
            &read(PS),
            "        Fail \"setting gh-runner's default WSL distribution to $NoDistro failed, so the default route is open: $_\"\n",
            "        \"setting it failed: $_\"\n"
        ))
    );
}
