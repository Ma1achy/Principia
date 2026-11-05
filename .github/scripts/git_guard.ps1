# Refuses a job when the checkout kept between runs on the self-hosted runner holds something that would run or redirect
# git (the owner, 5 Nov 2026; R-711 (lands first in its own rulings PR)). It runs before actions/checkout, which runs git
# in that folder. A job can write there, so this file is never run from it: the step fetches this file at the job's own
# commit. It reads .git as files and each config through `git config --file`, takes only the keys and values a clean
# checkout holds, and, once all of that has passed, has git hash every object and write an empty index. The PC's job-started
# hook runs a guard of its own, `E:\runner-tools\checks\git-guard.ps1`; this step is the second layer.
#
#   git_guard.ps1                  Guard $env:GITHUB_WORKSPACE. On passing, empty .git/index. On firing, log to
#                                  $env:WIN_REFUSAL_LOG, empty the checkout (a fresh clone next run) and exit 1.
#   git_guard.ps1 check <folder>   Report on <folder>'s .git and exit 0 or 1, deleting nothing: the controls' way in.
#
# Fails closed: anything it can't read or follow is a refusal. A first run, with no .git, passes, as does the empty folder
# a refusal leaves: a job can empty that folder, not remove it.
$ErrorActionPreference = 'Stop'

# The config keys that can execute or redirect: the owner's list, then the same classes the PC's hook adds (applied per
# R-369). Any key off $allowed refuses, so this list only names a refusal's reason, and a key can't pass by being missing
# here. Matched with -like, so `*` is any text, and without regard to case, as git matches a key's section and name.
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
    'fsck.*'
    # Both are for a git newer than the PC's 2.43, which has neither, so an upgrade can't open them without a word.
    'hook.*'
    'extensions.refStorage'
)
# The only `key=value` pairs a kept checkout may hold, as git prints them, each key once: what actions/checkout v7.0.1
# (3d3c42e5) leaves with this workflow's inputs. Anything else refuses, at the cost of one red job and a fresh clone (the
# owner's order, widened per R-369). A value counts too: `remote.origin.fetch` would otherwise have the action's own fetch
# write a replace ref after this script has finished. Sources are in that action's `src/`.
$allowed = @(
    # git-command-manager.ts:367, `git init`; git-source-provider.ts:130. What git's probes write on this PC's Windows git.
    'core.repositoryformatversion=0'
    'core.filemode=false'
    'core.bare=false'
    'core.logallrefupdates=true'
    'core.symlinks=false'
    'core.ignorecase=true'
    # git-command-manager.ts:406, `git remote add`; git-source-provider.ts:131.
    'remote.origin.url=https://github.com/Ma1achy/Principia'
    'remote.origin.fetch=+refs/heads/*:refs/remotes/origin/*'
    # git-command-manager.ts:521; git-source-provider.ts:137.
    'gc.auto=0'
    # git-command-manager.ts:226, `checkout --force -B main refs/remotes/origin/main` with ref-helper.ts:37's start point,
    # which sets the upstream. A dispatch from another branch leaves other keys, so the next run refuses and clones.
    'branch.main.remote=origin'
    'branch.main.merge=refs/heads/main'
    # git-command-manager.ts:197, `sparse-checkout disable`, in config.worktree: git 2.43's builtin/sparse-checkout.c:374-384.
    'core.sparsecheckout=false'
    'core.sparsecheckoutcone=false'
    'index.sparse=false'
)
# What the action fetches from, url-helper.ts:20, with no `.git`, which must be there as well as allowed. A differing value
# would only have the action delete the folder (git-directory-helper.ts:27-29), but that reads the last of several.
$originUrl = 'https://github.com/Ma1achy/Principia'

# The git the PC's hook uses, and never a name: PATH can differ here, as an earlier step can write to GITHUB_PATH.
$gitExe = 'C:\Program Files\Git\cmd\git.exe'
# git reads each config itself, so a reader here never has to match its grammar: a config git can't read fails, and what
# it reads is what the next git would. `--file` reads that one file and nothing of the checkout, `--no-includes` follows no
# include, and listing keys runs nothing. --null, because a subsection can hold a CR, which would split a key if it were
# read as lines. git 2.43's documentation has each of these flags.
$arguments = 'config --no-includes --file "{0}" --list --null'
# A commit's name proves nothing kept in .git, as git doesn't hash an object when it reads it: fsck does. --git-dir says
# which repo, so no discovery runs. Not --connectivity-only, which leaves blobs unhashed.
$objects = '--git-dir "{0}" fsck --no-dangling'
# An index with no entries, and not none: with no index at all, the action's `checkout --detach` fails and it recreates the
# repo, whatever the run (`git-directory-helper.ts:51-53, 109-113`), which pays for the clone every time.
$emptyIndex = '--git-dir "{0}" read-tree --empty'

# git's standard output. It starts outside the checkout, and the job can't write to $env:SystemRoot. Any failure throws, so
# it refuses.
function Invoke-Git($what) {
    if (-not [System.IO.File]::Exists($gitExe)) { throw "no git at $gitExe" }
    $start = New-Object System.Diagnostics.ProcessStartInfo
    $start.FileName = $gitExe
    $start.Arguments = $what
    $start.WorkingDirectory = $env:SystemRoot
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $run = [System.Diagnostics.Process]::Start($start)
    $errors = $run.StandardError.ReadToEndAsync()
    $out = $run.StandardOutput.ReadToEnd()
    $run.WaitForExit()
    if ($run.ExitCode -ne 0) { throw "git $what failed, status $($run.ExitCode): $($errors.Result)" }
    $out
}

# The file's entries, each `key` then a newline then its value, as --null writes them.
function Read-Config($path) {
    (Invoke-Git ($arguments -f $path)).Split([char]0) | Where-Object { $_ }
}

# The entry of $parent called $name, or nothing. A listing that fails throws, where Test-Path would only say no.
function Get-Entry($parent, $name) {
    if ($parent) { @(Get-ChildItem -LiteralPath $parent.FullName -Force | Where-Object { $_.Name -eq $name }) | Select-Object -First 1 }
}

# What .git holds that it must not; empty if nothing.
function Find-Planted($git) {
    if ($git.Attributes -band [System.IO.FileAttributes]::ReparsePoint) { return ".git is a link" }
    if (-not $git.PSIsContainer) { return ".git is a file" }
    $found = @()
    # Each sends git elsewhere for its config, hooks or objects, or puts other content under a commit's name.
    foreach ($redirect in 'commondir', 'gitdir') {
        if (Get-Entry $git $redirect) { $found += "redirect: $redirect" }
    }
    # Refs kept here would hide a replace ref from the checks below.
    if (Get-Entry $git 'reftable') { $found += '.git/reftable' }
    $hooks = Get-Entry $git 'hooks'
    if ($hooks) {
        foreach ($hook in @(Get-ChildItem -LiteralPath $hooks.FullName -Force)) {
            if ($hook.Name -notlike '*.sample') { $found += "active hook: $($hook.Name)" }
        }
    }
    $info = Get-Entry $git 'info'
    if (Get-Entry $info 'attributes') { $found += 'info/attributes' }
    if (Get-Entry $info 'grafts') { $found += 'info/grafts' }
    if (Get-Entry (Get-Entry (Get-Entry $git 'objects') 'info') 'alternates') { $found += 'objects/info/alternates' }
    $replace = Get-Entry (Get-Entry $git 'refs') 'replace'
    if ($replace -and @(Get-ChildItem -LiteralPath $replace.FullName -Force -Recurse).Count) { $found += 'refs/replace/' }
    $packed = Get-Entry $git 'packed-refs'
    if ($packed -and [System.IO.File]::ReadAllText($packed.FullName).IndexOf('refs/replace/', [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
        $found += 'refs/replace/ in packed-refs'
    }
    # `config` must exist: a .git without one is no checkout this script can vouch for. `config.worktree` needn't, and
    # isn't refused for existing, as the action's `sparse-checkout disable` writes one: it gets the same key check.
    $configs = @(Get-Item -LiteralPath (Join-Path $git.FullName 'config') -Force)
    $worktree = Get-Entry $git 'config.worktree'
    if ($worktree) { $configs += $worktree }
    # What the action leaves: on `main`, or detached for a ref that names no branch (ref-helper.ts:30-47).
    $head = [System.IO.File]::ReadAllText((Join-Path $git.FullName 'HEAD'))
    if ($head -cnotmatch '\A(ref: refs/heads/main|[0-9a-f]{40})\n?\z') { $found += "HEAD: $head" }
    $allowedKeys = @($allowed | ForEach-Object { $_.Split('=')[0] })
    $held = @()
    $pairs = @()
    foreach ($config in $configs) {
        foreach ($entry in @(Read-Config $config.FullName)) {
            $key, $value = $entry -split "`n", 2
            $held += $key
            $pairs += "$key=$value"
            if (@($keys | Where-Object { $key -like $_ }).Count) {
                $found += "config key: $key"
            } elseif ($allowedKeys -cnotcontains $key) {
                $found += "config key off the allowlist: $key"
            } elseif ($allowed -cnotcontains "$key=$value") {
                $found += "config value off the allowlist: $key=$value"
            }
        }
    }
    # Each holds one value in a clean checkout, and git and the action can each read a different one of several.
    foreach ($repeat in @($held | Group-Object | Where-Object { $_.Count -gt 1 })) {
        $found += "repeated config key: $($repeat.Name)"
    }
    if ($pairs -cnotcontains "remote.origin.url=$originUrl") { $found += "remote.origin.url isn't this repo" }
    # Last, as it reads the kept repo: only what has passed all of the above gets as far as being opened by git.
    if (-not $found.Count) {
        $null = Invoke-Git ($objects -f $git.FullName)
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
    # The index is the record of what's on disk, so a job can forge it: a gitlink keeps a planted folder from `clean`, and a
    # stat entry keeps an edited file from `reset`. With no entries `clean` clears the tree and `reset` writes every tracked
    # file; `.git` is kept, so the fetch's saving stays. It comes after every check, as it writes to the kept repo, and a
    # failure throws, so it refuses.
    if ($git.Count -and -not $planted.Count -and -not $check) {
        $null = Invoke-Git ($emptyIndex -f $git[0].FullName)
    }
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

# Each thing in the checkout, `.git` with the rest, and not the folder itself, which the runner holds. The names here are
# planted, so no shell reads them, and only .NET's calls take them, as data. A link goes as a link: nothing goes inside it.
function Remove-Entry($item) {
    try {
        $link = $item.Attributes -band [System.IO.FileAttributes]::ReparsePoint
        if (-not $link) { $item.Attributes = [System.IO.FileAttributes]::Normal }
        if ($item.PSIsContainer) {
            if (-not $link) {
                foreach ($inner in @(Get-ChildItem -LiteralPath $item.FullName -Force)) { Remove-Entry $inner }
            }
            [System.IO.Directory]::Delete($item.FullName, $false)
        } else {
            [System.IO.File]::Delete($item.FullName)
        }
    } catch {
        "git guard: could not delete $($item.Name): $_"
    }
}
foreach ($item in @(Get-ChildItem -LiteralPath $root -Force)) { Remove-Entry $item }
$left = @(Get-ChildItem -LiteralPath $root -Force)
if ($left.Count) { "git guard: could not empty the checkout: $($left.Count) left" }
exit 1
