# The check that no launcher naming no distro reaches one (TASK-M0-72; the PC's ask on #390, 14 Nov 2026, and the
# owner's OK that day to close the route). On the runner's account, System32's `bash.exe` and `wsl.exe` started with no
# `-d` go to the account's default WSL distribution: so a test's bare `bash` reached `principia-wsl` and wrote to it
# (run 43593938536). The PC points gh-runner's default at a GUID no distro has, and `wsl.ps1 register` points it there
# again after each `wsl --import`, which makes the distro it imports the default.
#
# The runner's nightly self-check job runs this, as gh-runner. System32's `bash.exe -c true`, the copy Rust's
# `Command::new("bash")` finds before PATH, and `wsl.exe -e true`, each naming no distro, must fail; every distro's state
# in `wsl --list --verbose` must read the same after them as before, every one Stopped before so that a start shows;
# and none may be the default. It exits 0 only if so, and only once the same verdict has called each of four simulated
# open routes open; a verdict that calls one closed proves nothing, and the check exits 1.
#
# The one script on the runner that starts WSL's launchers naming no distro, on purpose, each start one that must fail
# (`xtask/tests/ci_win_no_wsl.rs` holds every other script to the distro's name). It writes nothing. What it prints is
# each launcher's exit and words, the distros' names and states, and its verdict.
$ErrorActionPreference = 'Stop'
$Distro = 'principia-wsl'
$System = Join-Path $env:SystemRoot 'System32'
$Wsl = Join-Path $System 'wsl.exe'
# wsl.exe's own messages in UTF-8, not UTF-16.
$env:WSL_UTF8 = '1'

# Starts $File $Arguments as gh-runner, naming no distro unless $Arguments does, and waits at most 60 s, its stdin
# closed at once. `Code` is its exit, $null if it never started or hung; `Lines` what it printed, stdout's then
# stderr's; `Note` says which, with those lines.
function Invoke-Program([string]$File, [string]$Arguments) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $File
    $psi.Arguments = $Arguments
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [Text.Encoding]::UTF8
    try { $p = [System.Diagnostics.Process]::Start($psi) } catch {
        return [pscustomobject]@{ Code = $null; Lines = @(); Note = "not started: $_" }
    }
    $p.StandardInput.Close()
    $out = $p.StandardOutput.ReadToEndAsync()
    $err = $p.StandardError.ReadToEndAsync()
    if (-not $p.WaitForExit(60000)) {
        # It may end between the wait and the kill.
        try { $p.Kill() } catch { }
        return [pscustomobject]@{ Code = $null; Lines = @(); Note = 'hung: still running after 60 s' }
    }
    $lines = @(@("$($out.Result)" -split "`r?`n") + @("$($err.Result)" -split "`r?`n") |
            ForEach-Object { ("$_" -replace "`0", '').Trim() } | Where-Object { $_ })
    [pscustomobject]@{ Code = $p.ExitCode; Lines = $lines; Note = "exit $($p.ExitCode), saying '$($lines -join ' | ')'" }
}

# Each of gh-runner's distros, from `wsl --list --verbose`: `Table`, what it printed; `Text`, each name and state,
# sorted; `Defaults`, the one marked `*`, if any; `Idle`, whether every one is Stopped. $null if the states weren't
# read, so that nothing could be told from them: either listing failed, the verbose one has no header row, it shows
# no distro (the PC always has principia-wsl, so none means the reading failed), or a distro `wsl --list --quiet`
# names isn't among its rows. Both listings' exits and output, and which of those causes applied, go to the log
# through Write-Host, never the output this returns, so that a refusal on the PC says why.
function Get-DistroStates {
    $listed = Invoke-Program $Wsl '--list --verbose'
    $quiet = Invoke-Program $Wsl '--list --quiet'
    Write-Host "wsl --list --verbose: $($listed.Note)"
    Write-Host "wsl --list --quiet: $($quiet.Note)"
    if ($listed.Code -ne 0 -or $quiet.Code -ne 0) { Write-Host 'the states are unread: a listing failed, hung or never started (its exit above)'; return $null }
    if (@($listed.Lines | Where-Object { $_ -match '^NAME\s+STATE\s+VERSION$' }).Count -eq 0) { Write-Host 'the states are unread: wsl --list --verbose printed no NAME STATE VERSION header'; return $null }
    $rows = @(foreach ($line in $listed.Lines) {
            if ($line -match '^(\*?)\s*(\S+)\s+(\S+)\s+([0-9]+)$') {
                [pscustomobject]@{ Name = $Matches[2]; State = $Matches[3]; Default = $Matches[1] -eq '*' }
            }
        })
    if ($rows.Count -eq 0) { Write-Host 'the states are unread: wsl --list --verbose shows no distro, and principia-wsl is always registered'; return $null }
    $names = @($quiet.Lines)
    foreach ($name in $names) {
        if (@($rows | ForEach-Object { $_.Name }) -notcontains $name) { Write-Host "the states are unread: wsl --list --verbose doesn't show $name, which wsl --list --quiet names"; return $null }
    }
    [pscustomobject]@{
        Table = $listed.Lines
        Text = (@($rows | Sort-Object Name | ForEach-Object { "$($_.Name)=$($_.State)" }) -join ', ')
        Defaults = @($rows | Where-Object { $_.Default } | ForEach-Object { $_.Name })
        Idle = @($rows | Where-Object { $_.State -cne 'Stopped' }).Count -eq 0
    }
}

# The route is closed only if no probe ran to exit 0 or hung, the distros' states read the same after the probes as
# before, and no distro is the default after them.
function Test-RouteClosed($Probes, $Before, $After) {
    $ran = @($Probes | Where-Object { $_.Code -eq 0 -or $_.Note -like 'hung*' })
    $ran.Count -eq 0 -and $Before.Text -ceq $After.Text -and $After.Defaults.Count -eq 0
}

# The check: every distro Stopped first, then the probes between two readings of their states.
$stopped = Invoke-Program $Wsl "--terminate $Distro"
"wsl --terminate ${Distro}: $($stopped.Note)"
$before = Get-DistroStates
$before.Table
if ($null -eq $before) { '::error::wsl_route.ps1: the distros'' states before the probes could not be read; the cause, and both listings, are printed above'; exit 1 }
if (-not $before.Idle) { "::error::wsl_route.ps1: a distro is running before the probes, so a start by them could not be told: '$($before.Text)'"; exit 1 }
$probes = @(
    [pscustomobject]@{ Name = 'bash -c true'; Run = Invoke-Program (Join-Path $System 'bash.exe') '-c true' }
    [pscustomobject]@{ Name = 'wsl -e true'; Run = Invoke-Program $Wsl '-e true' }
)
$after = Get-DistroStates
$after.Table
$stopped = Invoke-Program $Wsl "--terminate $Distro"
"wsl --terminate ${Distro}: $($stopped.Note)"
if ($null -eq $after) { '::error::wsl_route.ps1: the distros'' states after the probes could not be read; the cause, and both listings, are printed above'; exit 1 }
foreach ($p in $probes) { "probe $($p.Name), as $([Environment]::UserName), naming no distro: $($p.Run.Note)" }
"the distros before the probes: '$($before.Text)'; after: '$($after.Text)'; the default after: '$($after.Defaults -join ', ')'"

# The control: the same verdict on four open routes, simulated, each differing from a closed one in one thing, must
# call each open.
$idle = [pscustomobject]@{ Text = "$Distro=Stopped"; Defaults = @() }
$refused = [pscustomobject]@{ Code = 1; Note = 'exit 1' }
$simulated = [ordered]@{
    'a probe that ran' = Test-RouteClosed @([pscustomobject]@{ Code = 0; Note = 'exit 0' }) $idle $idle
    'a probe that hung' = Test-RouteClosed @([pscustomobject]@{ Code = $null; Note = 'hung: still running after 60 s' }) $idle $idle
    'a distro started' = Test-RouteClosed @($refused) $idle ([pscustomobject]@{ Text = "$Distro=Running"; Defaults = @() })
    'a default distro' = Test-RouteClosed @($refused) $idle ([pscustomobject]@{ Text = "$Distro=Stopped"; Defaults = @($Distro) })
}
$blind = @($simulated.Keys | Where-Object { $simulated[$_] })
if ($blind.Count -ne 0) { "::error::wsl_route.ps1: the verdict calls a simulated open route closed ($($blind -join '; ')), so it proves nothing"; exit 1 }
"the verdict calls each simulated open route open: $($simulated.Keys -join '; ')"

if (-not (Test-RouteClosed @($probes | ForEach-Object { $_.Run }) $before $after)) { '::error::wsl_route.ps1: open: a launcher naming no distro ran, hung or started one, or a distro is the default'; exit 1 }
'wsl_route.ps1: closed: no launcher naming no distro ran, no distro started, and none is the default'
exit 0
