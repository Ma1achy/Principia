# Gives each tracked file whose content hasn't changed since the job's last run the mtime it had then (TASK-M0-89). The
# guard (`git_guard.ps1`) empties the index, so actions/checkout's `clean: true` deletes and writes every tracked file, and
# every source gets a mtime newer than cargo's fingerprints: the crates rebuild with their content unchanged. This runs
# right after the checkout and before any build. Like the guard, it is fetched at the job's own commit and never run from
# the checkout.
#
#   mtime_restore.ps1 <checkout> <manifest>
#
# The manifest, outside the checkout, beside the job's target folder under E:\ci-cache, one per job, holds each tracked
# file's path, mode and git blob id and the mtime the last run left it with. A file whose mode and blob id, from the index
# the checkout just wrote, equal the manifest's gets the manifest's mtime back; a file whose blob differs, or that is new,
# keeps the checkout's fresh one. A file whose content changed never gets an older mtime.
#
# The manifest must always describe the tree this job's target was last built from, or not exist. So it is read, then
# deleted before anything else changes, and the new one is written last, through a temporary file and a rename. A
# missing, unreadable or malformed manifest restores nothing (a full rebuild, never a stale one); so does any change to a
# `.gitattributes`, since an equal blob is then not equal bytes on disk. Exits 1 only if the old manifest can't be
# deleted, which would leave one that no longer describes the build (or it isn't given its two arguments); any later failure leaves no manifest, restores what
# it had restored (each one an unchanged file) and exits 0 with a warning.
$ErrorActionPreference = 'Stop'

if ($args.Count -ne 2 -or -not $args[0] -or -not $args[1]) { 'usage: mtime_restore.ps1 <checkout> <manifest>'; exit 1 }
$root = ([string]$args[0]).TrimEnd('\')
$manifest = [string]$args[1]
# The git the guard uses, and never a name: PATH can differ here, as an earlier step can write to GITHUB_PATH.
$gitExe = 'C:\Program Files\Git\cmd\git.exe'
$header = 'principia-mtimes 1'
# One tracked regular file in the index: mode, blob id, stage 0, path. Links, submodules and conflicts stay fresh.
$indexEntry = '\A(100644|100755) ([0-9a-f]{40}|[0-9a-f]{64}) 0\t(.+)\z'
# One file in the manifest: mode and blob id, its mtime in UTC file-time ticks, its path.
$manifestEntry = '\A(100644|100755) ([0-9a-f]{40}|[0-9a-f]{64})\t([0-9]{1,19})\t(.+)\z'

# git's standard output, as UTF-8. Any failure throws.
function Invoke-Git($what) {
    if (-not [System.IO.File]::Exists($gitExe)) { throw "no git at $gitExe" }
    $start = New-Object System.Diagnostics.ProcessStartInfo
    $start.FileName = $gitExe
    $start.Arguments = $what
    $start.WorkingDirectory = $env:SystemRoot
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.StandardOutputEncoding = New-Object System.Text.UTF8Encoding $false
    $run = [System.Diagnostics.Process]::Start($start)
    $errors = $run.StandardError.ReadToEndAsync()
    $out = $run.StandardOutput.ReadToEnd()
    $run.WaitForExit()
    if ($run.ExitCode -ne 0) { throw "git $what failed, status $($run.ExitCode): $($errors.Result)" }
    $out
}

function New-Map {
    New-Object 'System.Collections.Generic.Dictionary[string,string]' ([System.StringComparer]::Ordinal)
}

# The index: each tracked regular file's path to its `mode blob`. Throws if it lists none.
function Read-Index {
    $index = New-Map
    foreach ($entry in (Invoke-Git ('-C "{0}" ls-files -s -z' -f $root)).Split([char]0)) {
        if ($entry -cmatch $indexEntry) { $index[$Matches[3]] = "$($Matches[1]) $($Matches[2])" }
    }
    if (-not $index.Count) { throw 'the index lists no file' }
    , $index
}

# The manifest: each path to its `mode blob` and to its ticks. Throws on anything but a whole manifest: its header, one
# well-formed line per path, each path once, and the count it ends with.
function Read-Manifest {
    $lines = [System.IO.File]::ReadAllText($manifest, [System.Text.Encoding]::UTF8).Split("`n")
    if ($lines.Count -lt 3 -or $lines[0] -cne $header -or $lines[$lines.Count - 1] -cne '') { throw 'no header or no last newline' }
    $count = $lines.Count - 3
    if ($lines[$lines.Count - 2] -cne "end $count") { throw 'not whole' }
    $blobs = New-Map
    $ticks = New-Map
    for ($at = 1; $at -le $count; $at++) {
        if (-not ($lines[$at] -cmatch $manifestEntry)) { throw "line $($at + 1) is malformed" }
        $path = $Matches[4]
        if ($blobs.ContainsKey($path)) { throw "$path is listed twice" }
        $null = [DateTime]::FromFileTimeUtc([long]$Matches[3])
        $blobs[$path] = "$($Matches[1]) $($Matches[2])"
        $ticks[$path] = $Matches[3]
    }
    @{ Blobs = $blobs; Ticks = $ticks }
}

# Whether every `.gitattributes` in $from is in $to with the same mode and blob.
function Test-Covered($from, $to) {
    foreach ($path in $from.Keys) {
        if (-not ($path -cmatch '(\A|/)\.gitattributes\z')) { continue }
        $other = $null
        if (-not $to.TryGetValue($path, [ref]$other) -or $other -cne $from[$path]) { return $false }
    }
    $true
}

$old = $null
$why = 'no manifest'
if ([System.IO.File]::Exists($manifest)) {
    try { $old = Read-Manifest } catch { $why = "the manifest is unusable: $_" }
}
try {
    if ([System.IO.File]::Exists($manifest)) { [System.IO.File]::Delete($manifest) }
} catch {
    "::error::could not delete the manifest $manifest, which no longer describes the build: $_"
    exit 1
}

try {
    $index = Read-Index
    $restore = $false
    if ($old) {
        $restore = (Test-Covered $index $old.Blobs) -and (Test-Covered $old.Blobs $index)
        if (-not $restore) { $why = 'a .gitattributes changed' }
    }
    $restored = 0
    $fresh = 0
    foreach ($path in $index.Keys) {
        $file = [System.IO.Path]::Combine($root, $path)
        $key = $index[$path]
        $was = $null
        # The one place a mtime is set: only on the same mode and blob id as the manifest's.
        if ($restore -and $old.Blobs.TryGetValue($path, [ref]$was) -and $was -ceq $key -and [System.IO.File]::Exists($file)) {
            try {
                [System.IO.File]::SetLastWriteTimeUtc($file, [DateTime]::FromFileTimeUtc([long]$old.Ticks[$path]))
                $restored++
                continue
            } catch {
                "could not restore $path, kept fresh: $_"
            }
        }
        $fresh++
    }
    if ($restore) {
        "mtimes: restored $restored, kept fresh $fresh"
    } else {
        "mtimes: restored 0, kept fresh $fresh ($why)"
    }

    $text = New-Object System.Text.StringBuilder
    $null = $text.Append("$header`n")
    $written = 0
    foreach ($path in $index.Keys) {
        $file = [System.IO.Path]::Combine($root, $path)
        if ($path.Contains("`n") -or $path.Contains("`r") -or -not [System.IO.File]::Exists($file)) { continue }
        $ticks = [System.IO.File]::GetLastWriteTimeUtc($file).ToFileTimeUtc()
        $null = $text.Append("$($index[$path])`t$ticks`t$path`n")
        $written++
    }
    $null = $text.Append("end $written`n")
    $null = [System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($manifest))
    $temporary = "$manifest.new"
    [System.IO.File]::WriteAllText($temporary, $text.ToString(), (New-Object System.Text.UTF8Encoding $false))
    [System.IO.File]::Move($temporary, $manifest)
    "mtimes: wrote $written files to $manifest"
} catch {
    "::warning::mtimes: no manifest written, so the next run restores nothing: $_"
}
exit 0
