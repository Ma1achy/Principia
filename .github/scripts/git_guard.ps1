# Refuses a job when the checkout kept between runs on the self-hosted runner holds something that would run or redirect
# git (the owner, 5 Nov 2026; R-711 (lands first in its own rulings PR)). It runs before actions/checkout, which runs git
# in that folder. A job can write there, so this file is never run from it: the step fetches this file at the job's own
# commit. It reads .git as files, never through git, so reading runs nothing and follows no include. The PC's job-started
# hook runs a guard of its own, `E:\runner-tools\checks\git-guard.ps1`, on the same list; this step is the second layer.
#
#   git_guard.ps1                  Guard $env:GITHUB_WORKSPACE. On firing, log to $env:WIN_REFUSAL_LOG, empty the checkout
#                                  (a fresh clone next run) and exit 1.
#   git_guard.ps1 check <folder>   Report on <folder>'s .git and exit 0 or 1, deleting nothing: the controls' way in.
#
# Fails closed: anything it can't read or follow is a refusal. A first run, with no .git, passes, as does the empty folder
# a refusal leaves: a job can empty that folder, not remove it.
$ErrorActionPreference = 'Stop'

# The config keys that can execute or redirect: the owner's list, then the same classes the PC's hook adds (applied per
# R-369). Matched with -like, so `*` is any text, and without regard to case, as git matches a key's section and name.
# `xtask/tests/ci_self_hosted.rs` reads this list.
$keys = @(
    'core.hooksPath'
    'core.fsmonitor'
    'core.sshCommand'
    'core.editor'
    'core.pager'
    'filter.*.clean'
    'filter.*.smudge'
    'filter.*.process'
    'diff.*.textconv'
    'credential.helper'
    'include.path'
    'includeIf.*'
    'url.*.insteadOf'
    'http.proxy'
    'gpg.program'
    'gpg.ssh.program'
    'core.gitProxy'
    'core.askPass'
    'core.attributesFile'
    'core.worktree'
    'core.excludesFile'
    'diff.external'
    'diff.*.command'
    'merge.*.driver'
    'alias.*'
    'pager.*'
    'sequence.editor'
    'gpg.*.program'
    'remote.*.uploadpack'
    'remote.*.receivepack'
    'remote.*.proxy'
    'remote.*.vcs'
    'credential.*.helper'
    'http.*.proxy'
    'url.*.pushInsteadOf'
)
$header = '^\[\s*([^\s"\]]+)(?:\s+"((?:[^"\\]|\\.)*)")?\s*\](.*)$'
$entry = '^([A-Za-z][A-Za-z0-9-]*)\s*(=.*)?$'

# A config file's full keys, one for each entry. A line this can't read as git would throws, so it refuses.
function Read-Keys($path) {
    $section = ''
    foreach ($raw in [System.IO.File]::ReadAllLines($path)) {
        $line = $raw.Trim()
        if ($line -match $header) {
            $name, $sub, $rest = $Matches[1], $Matches[2], $Matches[3]
            $section = if ($sub) { "$name.$sub" } else { $name }
            $line = $rest.Trim()
        }
        if ($line -eq '' -or $line[0] -eq '#' -or $line[0] -eq ';') { continue }
        if (($section -eq '') -or ($line -notmatch $entry)) { throw "a line git's config can't hold, in ${path}: $raw" }
        "$section.$($Matches[1])"
    }
}

# What .git holds that it must not; empty if nothing.
function Find-Planted($git) {
    if ($git.Attributes -band [System.IO.FileAttributes]::ReparsePoint) { return ".git is a link" }
    if (-not $git.PSIsContainer) { return ".git is a file" }
    $held = @(Get-ChildItem -LiteralPath $git.FullName -Force)
    $found = @()
    foreach ($hooks in @($held | Where-Object { $_.Name -eq 'hooks' })) {
        foreach ($hook in @(Get-ChildItem -LiteralPath $hooks.FullName -Force)) {
            if ($hook.Name -notlike '*.sample') { $found += "active hook: $($hook.Name)" }
        }
    }
    foreach ($info in @($held | Where-Object { $_.Name -eq 'info' })) {
        if (@(Get-ChildItem -LiteralPath $info.FullName -Force | Where-Object { $_.Name -eq 'attributes' }).Count) {
            $found += 'info/attributes'
        }
    }
    # `config` must exist: a .git without one is no checkout this script can vouch for. `config.worktree` needn't, and
    # isn't refused for existing, as actions/checkout writes one for sparse-checkout settings: it gets the same key check.
    $configs = @($held | Where-Object { $_.Name -eq 'config.worktree' }) + @(Get-Item -LiteralPath (Join-Path $git.FullName 'config') -Force)
    foreach ($config in $configs) {
        foreach ($key in @(Read-Keys $config.FullName)) {
            foreach ($listed in $keys) {
                if ($key -like $listed) { $found += "config key: $key" }
            }
        }
    }
    $found
}

$check = $args[0] -eq 'check'
$root = if ($check) { $args[1] } else { $env:GITHUB_WORKSPACE }
try {
    $held = try {
        @(Get-ChildItem -LiteralPath $root -Force)
    } catch [System.Management.Automation.ItemNotFoundException] {
        @()
    }
    $git = @($held | Where-Object { $_.Name -eq '.git' })
    $planted = if ($git.Count) { @(Find-Planted $git[0]) } else { @() }
} catch {
    $planted = @("can't read the checkout: $_")
}
if (-not $planted.Count) {
    "git guard: nothing planted in $root"
    exit 0
}
$planted | ForEach-Object { "git guard: $_" }
if ($check) { exit 1 }

# Past here the job fails whatever else fails. The log is append-only to this account: Add-Content and FileStream are
# refused, so a line goes through the PC's own writer, which returns 0 on success.
$ErrorActionPreference = 'Continue'
try {
    . E:\runner-tools\checks\append-only.ps1
    $line = "$(Get-Date -Format o) run $env:GITHUB_RUN_ID job $env:GITHUB_JOB commit ${env:GITHUB_SHA}: $($planted -join '; ')"
    $status = [AppendOnly]::Add($env:WIN_REFUSAL_LOG, [System.Text.Encoding]::UTF8.GetBytes("$line`r`n"))
    if ($status -ne 0) { "git guard: could not write the refusal log: status $status" }
} catch {
    "git guard: could not write the refusal log: $_"
}

# Each thing in the checkout, `.git` with the rest, and not the folder itself, which the runner holds. rmdir and del,
# because they remove a link and not what it points to, and they remove git's read-only files.
foreach ($item in @(Get-ChildItem -LiteralPath $root -Force)) {
    if ($item.PSIsContainer) {
        cmd /c rmdir /s /q "$($item.FullName)"
    } else {
        cmd /c del /f /q /a "$($item.FullName)"
    }
}
$left = @(Get-ChildItem -LiteralPath $root -Force)
if ($left.Count) { "git guard: could not empty the checkout: $($left.Count) left" }
exit 1
