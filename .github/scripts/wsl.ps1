# The Windows side of the WSL2 distro `principia-wsl` (RQ-751 option (c), built in parallel and never blocking; the
# owner's words of 9 Nov 2026). The self-hosted runner's dispatch-only jobs start it through the record,
# `.github/scripts/win_measure.ps1 run`, as the runner's account, gh-runner. Inside the distro everything runs as `ci`,
# through `scripts/wsl/inside.sh`, from a clone at the job's own commit; interop is off, so nothing inside can reach
# Windows, and Windows reads only what `wsl.exe` relays: stdout, and the exit status.
#
#   wsl.ps1 cache-ready
#                      Fails, naming the folder, unless the image cache E:\runner-tools\wsl-cache\ exists and gh-runner
#                      can write to it. The owner makes the folder; nothing here makes it, or writes in its place.
#   wsl.ps1 cache-lookup
#                      Exits 0 only if the cache holds the image of this checkout's recipe (Get-RecipeKey), whole: its
#                      length and SHA-256 the ones recorded when it was built. Otherwise it exits non-zero, saying why,
#                      and the job downloads this run's image. It writes nothing.
#   wsl.ps1 check-image
#                      Checks that the image a download wrote (WSL_IMAGE) is whole, its length and SHA-256 the ones
#                      built, and nothing more; the registering job runs it after each download, to try again on a bad
#                      one.
#   wsl.ps1 register   Imports the image of this checkout's recipe, as gh-runner, to E:\wsl\principia-wsl\, never C:.
#                      The image is the cache's, E:\runner-tools\wsl-cache\<recipe key>\: found there by the lookup
#                      (WSL_CACHE_HIT is `success`), or else this run's download (WSL_IMAGE: rootfs.tar.gz,
#                      rootfs.sha256, rootfs.bytes and manifest.txt), checked and moved there first. Either way the
#                      cache's copy is the one checked and imported, every time, never trusted by its name. Refuses
#                      unless a manifest hash is pinned in scripts/wsl/manifest.sha256 and the image's manifest has it,
#                      and unless the copy is whole, its length and bytes the ones built. Keeps at most two images in
#                      the cache, this one and the one used before it. Refuses if the distro exists, unless WSL_REPLACE
#                      is `true`. After the import it derives the manifest again inside the distro, from what is really
#                      there, and unregisters it if that differs from the pin, or if deriving it fails or stops short.
#   wsl.ps1 proofs     The six proofs RQ-751 (c) needs before it can replace (b), each reported PASS or FAIL with its
#                      evidence; exits 0 only if all six pass. WSL_PARITY_COMMIT and WSL_HOSTED_PARITY (the hosted
#                      runner's results, this run's) feed proof 6; WSL_IMAGE (this run's rebuild) feeds proof 5.
#   wsl.ps1 cpu        Refuses unless `.github/wsl-cpu-enabled` exists (a reviewed PR adds it once all six proofs have
#                      passed). Then the CPU tests and the job queue's mutants inside, through `scripts/contain.sh` in
#                      its unprivileged mode; their results come out as a tar stream on a second `wsl.exe`'s stdout.
#
# Every verb but check-image and the cache's two, which never start WSL, ends with `wsl.exe --terminate`, which ends
# every process in the distro, whatever was left running.
$ErrorActionPreference = 'Stop'
$verb = $args[0]
$Distro = 'principia-wsl'
$Disk = 'E:\wsl\principia-wsl'
$User = 'ci'
$Repo = '/home/ci/work/principia'
$Inside = "$Repo/scripts/wsl/inside.sh"
$Record = Join-Path $env:RUNNER_TEMP 'win-measure'
# The image cache (the owner, 12 Nov 2026: "Cache the verified image on E:"). The owner makes the folder, writable by
# gh-runner; this script never makes it. Each image is in a folder named by its recipe key, holding the four files the
# build writes; `<key>.partial` is one being filled.
$CacheRoot = 'E:\runner-tools\wsl-cache'
$ImageFiles = @('rootfs.tar.gz', 'rootfs.sha256', 'rootfs.bytes', 'manifest.txt')
# The recipe: what `scripts/wsl/build-image.sh` builds the image from. The script itself, which holds the base's URL and
# SHA-256, the Ubuntu snapshot date, the package list and the cargo tools' versions; every file of the repository it
# reads; and the manifest's pin. The image's bytes differ from build to build (run 43312851701: tar order, times,
# logs), what it is built from doesn't, so the cache is keyed on this.
$RecipeFiles = @('scripts/wsl/build-image.sh', 'scripts/wsl/manifest.sh', 'scripts/wsl/wsl.conf', 'scripts/wsl/manifest.sha256', 'rust-toolchain.toml')
# wsl.exe's own messages in UTF-8, not UTF-16.
$env:WSL_UTF8 = '1'
# What goes to wsl.exe's stdin, in UTF-8 with no byte-order mark. Windows PowerShell's .NET writes a started process's
# stdin through a writer in Console.InputEncoding, and writes that encoding's preamble as the process starts, before a
# byte of ours: every stdin began EF BB BF (run 43418528499: `sh: 1: #!/bin/sh: not found`, the mark before the `#`).
# So the encoding is set to one with no preamble; with no console to set it on the setter fails, and the value it keeps
# is set in its place. A newer .NET takes it per process, as StandardInputEncoding. Invoke-Wsl refuses to send anything
# while a preamble remains.
$NoBom = New-Object System.Text.UTF8Encoding $false
try { [Console]::InputEncoding = $NoBom } catch {
    $field = [Console].GetField('_inputEncoding', [Reflection.BindingFlags]'NonPublic, Static')
    if ($field) { $field.SetValue($null, $NoBom) }
}

# The PC's tailnet addresses, which identify the host and are never printed: Tailscale's IPv4 range 100.64.0.0/10 (CGNAT)
# and its IPv6 prefix fd7a:115c:a1e0::/48. Everything this script prints that can hold a host address goes through
# Hide-Tailnet: each proof's line (Report, and so the summary at the end and wsl-proofs.txt), what wsl.exe prints
# (Invoke-Wsl's echo) and the failure messages (Fail). Only the printed text is masked; every proof decides on the real
# addresses. Loopback and WSL's switch's private ranges print as they are: the proofs' evidence rests on them, and they
# identify nothing. `xtask/tests/wsl_mask.rs` runs these two patterns, with these replacements, on made-up addresses.
$TailnetV4 = '\b100\.(6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.[0-9]{1,3}\.[0-9]{1,3}\b'
$TailnetV6 = '(?i)\bfd7a:115c:a1e0(:[0-9a-f]{0,4})*'
function Hide-Tailnet([string]$Text) {
    ($Text -creplace $TailnetV4, '100.x.x.x') -creplace $TailnetV6, 'fd7a:115c:a1e0::x'
}

# Written to the console, as Invoke-Wsl writes: a line left on the output stream is lost with the exit when the caller is
# collecting that output, as `$pinned = Get-Pinned` is.
function Fail($message) {
    [Console]::Out.WriteLine((Hide-Tailnet "::error::wsl.ps1: $message"))
    exit 125
}

# One argument as .NET Framework's command line parses it back; none here may hold a quote.
function Quote([string]$a) {
    if ($a.Contains('"')) { throw "an argument with a quote: $a" }
    if ($a -match '\s' -or $a -eq '') { return '"' + $a + '"' }
    $a
}

function Start-Wsl([string[]]$Arguments) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = 'wsl.exe'
    $psi.Arguments = ($Arguments | ForEach-Object { Quote $_ }) -join ' '
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [Text.Encoding]::UTF8
    if ($psi.PSObject.Properties['StandardInputEncoding']) { $psi.StandardInputEncoding = $NoBom }
    [System.Diagnostics.Process]::Start($psi)
}

# `wsl.exe <arguments>`: its stdout shown line by line as it comes (or, with -OutFile, written there as bytes), its
# stderr after; -Stdin's bytes on its stdin, with nothing before them. Returns the exit status, the stdout lines and the
# stderr.
function Invoke-Wsl([string[]]$Arguments, [byte[]]$Stdin = $null, [string]$OutFile = $null) {
    if ($Stdin -and [Console]::InputEncoding.GetPreamble().Length -ne 0) {
        Fail "stdin's encoding, $([Console]::InputEncoding.WebName), would put a byte-order mark before what is sent"
    }
    $p = Start-Wsl $Arguments
    $err = $p.StandardError.ReadToEndAsync()
    if ($Stdin) { $p.StandardInput.BaseStream.Write($Stdin, 0, $Stdin.Length) }
    $p.StandardInput.Close()
    $lines = New-Object System.Collections.Generic.List[string]
    if ($OutFile) {
        $file = [IO.File]::Create($OutFile)
        try { $p.StandardOutput.BaseStream.CopyTo($file) } finally { $file.Dispose() }
    } else {
        while ($null -ne ($line = $p.StandardOutput.ReadLine())) {
            [Console]::Out.WriteLine((Hide-Tailnet $line))
            $lines.Add($line)
        }
    }
    $p.WaitForExit()
    $e = $err.Result
    if ($e) { [Console]::Out.WriteLine((Hide-Tailnet $e.TrimEnd())) }
    [pscustomobject]@{ Code = $p.ExitCode; Lines = $lines.ToArray(); Err = "$e" }
}

# The last $Count non-blank lines a failed Invoke-Wsl wrote, stdout's then stderr's (a traceback is stderr's), joined
# for a proof's one line.
function Get-Tail($Result, [int]$Count = 6) {
    $all = @(@($Result.Lines) + @("$($Result.Err)" -split "`r?`n") | ForEach-Object { "$_".Trim() } | Where-Object { $_ })
    ($all | Select-Object -Last $Count) -join ' | '
}

# As ci, in the distro, from /.
function Invoke-Ci([string[]]$Command, [byte[]]$Stdin = $null, [string]$OutFile = $null) {
    Invoke-Wsl (@('-d', $Distro, '-u', $User, '--cd', '/', '--') + $Command) $Stdin $OutFile
}

function Get-Distros {
    $names = @(& wsl.exe --list --quiet 2>$null | ForEach-Object { "$_".Trim([char]0, ' ') } | Where-Object { $_ })
    $names
}

function Stop-Distro {
    & wsl.exe --terminate $Distro 2>&1 | Out-Null
}

function Get-Pinned {
    $pin = (Get-Content -LiteralPath 'scripts\wsl\manifest.sha256' -Raw).Trim()
    if ($pin -notmatch '^[0-9a-f]{64}$') {
        Fail "no manifest hash is pinned in scripts/wsl/manifest.sha256 (it reads '$pin'): nothing pins the image, so it isn't registered. The first hosted build prints the hash; a reviewed PR pins it."
    }
    $pin
}

function Get-Sha256([string]$Path) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

# The image in $Image is whole: its files are there, rootfs.tar.gz is as long as built, and its bytes hash to what was
# built. The length comes first, so a download that stopped short names itself before the hash says only that it
# differs. `check-image` runs it after each download, `register` again before the import.
function Assert-Image([string]$Image) {
    $rootfs = Join-Path $Image 'rootfs.tar.gz'
    foreach ($f in $rootfs, (Join-Path $Image 'manifest.txt'), (Join-Path $Image 'rootfs.sha256'), (Join-Path $Image 'rootfs.bytes')) {
        if (-not (Test-Path -LiteralPath $f)) { Fail "the image has no $f" }
    }
    $length = (Get-Content -LiteralPath (Join-Path $Image 'rootfs.bytes') -Raw).Trim()
    if ($length -notmatch '^[0-9]+$') { Fail "rootfs.bytes holds no length: '$length'" }
    $copiedLength = (Get-Item -LiteralPath $rootfs).Length
    if ($copiedLength -lt [int64]$length) { Fail "rootfs.tar.gz is a short copy, $copiedLength of $length bytes: the download stopped short" }
    $built = (Get-Content -LiteralPath (Join-Path $Image 'rootfs.sha256') -Raw).Trim()
    $copied = Get-Sha256 $rootfs
    if ($copied -ne $built) { Fail "rootfs.tar.gz is $copied here, $built as built: the copy is not the image" }
    "the image is whole: rootfs.tar.gz is $copiedLength bytes, sha256 $copied, as built"
}

function Get-TextSha256([string[]]$Lines) {
    $bytes = [Text.Encoding]::UTF8.GetBytes((($Lines -join "`n") + "`n"))
    $sha = [Security.Cryptography.SHA256]::Create()
    -join ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') })
}

# The recipe's key: the SHA-256 of a first line naming the scheme and one line per recipe file, its path and its
# bytes' SHA-256. The same commit gives the same key on any machine; a change to any recipe file gives another.
function Get-RecipeKey {
    $lines = @('principia-wsl recipe 1') + @(foreach ($f in $RecipeFiles) { "$f $(Get-Sha256 $f)" })
    Get-TextSha256 $lines
}

# The cache's folder is there, and is a folder. The owner makes it and nothing here does, so a missing one fails here,
# named, before anything is written.
function Assert-Cache {
    if (-not (Test-Path -LiteralPath $CacheRoot -PathType Container)) {
        Fail "the image cache $CacheRoot\ doesn't exist: the owner makes it on the PC, writable by $([Environment]::UserName), before a register run. Nothing is written in its place"
    }
}

# Moves this run's download, $From, into the cache as $Entry. It is checked whole, and its manifest the pin, before
# anything is written; moved into `$Entry.partial`, and only then renamed $Entry, so that a folder named by a key holds a
# whole copy or nothing. `register` checks the cache's copy again before the import.
function Save-Image([string]$From, [string]$Entry, [string]$Pinned) {
    Assert-Cache
    Assert-Image $From
    $hash = Get-Sha256 (Join-Path $From 'manifest.txt')
    if ($hash -ne $Pinned) { Fail "this run's image's manifest is $hash, the pin is ${Pinned}: not caching it" }
    $partial = "$Entry.partial"
    foreach ($old in $partial, $Entry) {
        if (Test-Path -LiteralPath $old) { Remove-Item -LiteralPath $old -Recurse -Force }
    }
    New-Item -ItemType Directory -Path $partial | Out-Null
    foreach ($f in $ImageFiles) { Move-Item -LiteralPath (Join-Path $From $f) -Destination (Join-Path $partial $f) }
    Rename-Item -LiteralPath $partial -NewName (Split-Path -Leaf $Entry)
    "cached this run's image as $Entry"
}

# At most two images in the cache: $Current, marked as just used, and the one used last before it. Only folders named
# as the cache names them, a recipe key in lowercase hex (`-cmatch`: a folder named in capitals is no key's) or one
# being filled, are touched; anything else in the folder is left alone.
function Limit-Cache([string]$Current) {
    Assert-Cache
    (Get-Item -LiteralPath (Join-Path $CacheRoot $Current)).LastWriteTimeUtc = [DateTime]::UtcNow
    $entries = @(Get-ChildItem -LiteralPath $CacheRoot -Directory | Where-Object { $_.Name -cmatch '^[0-9a-f]{64}(\.partial)?$' })
    $previous = @($entries | Where-Object { $_.Name -cmatch '^[0-9a-f]{64}$' -and $_.Name -ne $Current } |
            Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1 | ForEach-Object { $_.Name })
    $keep = @($Current) + $previous
    foreach ($e in $entries) {
        if ($keep -notcontains $e.Name) {
            "removing $($e.Name) from the image cache: it keeps this image and the one before it"
            Remove-Item -LiteralPath $e.FullName -Recurse -Force
        }
    }
    "the image cache keeps $($keep -join ', ')"
}

# The manifest of the registered distro, derived inside it from what is there: the job's `manifest.sh`, on stdin.
function Get-LiveManifest {
    $script = [IO.File]::ReadAllBytes((Resolve-Path 'scripts\wsl\manifest.sh'))
    $r = Invoke-Ci @('sh', '-s') $script
    if ($r.Code -ne 0) { return $null }
    $r.Lines
}

# A fresh clone inside, at the job's commit, verified equal; the native side has checked that commit out already.
function Initialize-Clone {
    $url = "https://github.com/$env:GITHUB_REPOSITORY.git"
    $r = Invoke-Ci @('rm', '-rf', $Repo)
    $r = Invoke-Ci @('mkdir', '-p', '/home/ci/work')
    $r = Invoke-Ci @('git', 'clone', '--quiet', '--filter=blob:none', $url, $Repo)
    if ($r.Code -ne 0) { return "the clone of $url failed: exit $($r.Code)" }
    $r = Invoke-Ci @('git', '-C', $Repo, 'checkout', '--quiet', '--detach', $env:GITHUB_SHA)
    if ($r.Code -ne 0) { return "checkout of $env:GITHUB_SHA failed: exit $($r.Code)" }
    $r = Invoke-Ci @('git', '-C', $Repo, 'rev-parse', 'HEAD')
    $head = ($r.Lines | Select-Object -Last 1)
    if ($head -ne $env:GITHUB_SHA) { return "the clone is at '$head', not $env:GITHUB_SHA" }
    $null
}

# gh-runner's Windows processes, this script's own tree excepted: what `kill -9 -1` inside must leave alone.
function Get-RunnerProcesses {
    $all = @(Get-CimInstance Win32_Process)
    $ours = @{ $PID = $true }
    do {
        $grew = $false
        foreach ($p in $all) {
            if ($ours.ContainsKey([int]$p.ParentProcessId) -and -not $ours.ContainsKey([int]$p.ProcessId)) {
                $ours[[int]$p.ProcessId] = $true
                $grew = $true
            }
        }
    } while ($grew)
    $me = [Environment]::UserName
    foreach ($p in $all) {
        if ($ours.ContainsKey([int]$p.ProcessId)) { continue }
        $owner = try { Invoke-CimMethod -InputObject $p -MethodName GetOwner } catch { $null }
        if ($owner -and $owner.User -eq $me) {
            [pscustomobject]@{ Key = "$($p.ProcessId)@$($p.CreationDate.ToString('o'))"; Name = $p.Name; Id = $p.ProcessId }
        }
    }
}

# Whether a TCP connection from here to host:port opens within 3 s.
function Test-Tcp([string]$HostName, [int]$Port) {
    $c = New-Object System.Net.Sockets.TcpClient
    try { $c.ConnectAsync($HostName, $Port).Wait(3000) -and $c.Connected } catch { $false } finally { $c.Dispose() }
}

$results = New-Object System.Collections.Generic.List[string]
# A proof's line, masked (Hide-Tailnet) before it is printed or kept: the summary and wsl-proofs.txt are these lines.
function Report([int]$Proof, [bool]$Passed, [string]$Evidence) {
    $line = Hide-Tailnet "proof ${Proof}: $(if ($Passed) { 'PASS' } else { 'FAIL' }): $Evidence"
    $line
    $script:results.Add($line)
}

switch ($verb) {
    'cache-ready' {
        Assert-Cache
        $probe = Join-Path $CacheRoot ".write-probe-$PID"
        try {
            [IO.File]::WriteAllText($probe, '')
            Remove-Item -LiteralPath $probe -Force
        } catch {
            Fail "the image cache $CacheRoot\ isn't writable by $([Environment]::UserName): $_"
        }
        "the image cache $CacheRoot\ is there and writable; this run's recipe key is $(Get-RecipeKey)"
        exit 0
    }
    'cache-lookup' {
        Assert-Cache
        $key = Get-RecipeKey
        $entry = Join-Path $CacheRoot $key
        if (-not (Test-Path -LiteralPath $entry -PathType Container)) {
            "cache miss: no image of recipe $key in $CacheRoot\, so this run's is downloaded"
            exit 1
        }
        Assert-Image $entry
        "cache hit: the image of recipe $key in $entry is whole"
        exit 0
    }
    'check-image' {
        Assert-Image $env:WSL_IMAGE
        exit 0
    }
    'register' {
        $pinned = Get-Pinned
        $key = Get-RecipeKey
        $image = Join-Path $CacheRoot $key
        if ($env:WSL_CACHE_HIT -eq 'success') {
            "cache hit: registering the cached image of recipe $key, checked again here"
        } else {
            "cache miss: caching this run's image as recipe $key"
            Save-Image $env:WSL_IMAGE $image $pinned
        }
        # The sweep runs before the check, so that a check that fails still leaves at most two images; it keeps this
        # one, and the import below still waits on the check.
        Limit-Cache $key
        $rootfs = Join-Path $image 'rootfs.tar.gz'
        $manifest = Join-Path $image 'manifest.txt'
        Assert-Image $image
        $hash = Get-Sha256 $manifest
        if ($hash -ne $pinned) { Fail "the image's manifest is $hash, the pin is ${pinned}: refusing it" }
        "the image's manifest matches the pin: $hash"
        if ((Get-Distros) -contains $Distro) {
            if ($env:WSL_REPLACE -ne 'true') { Fail "$Distro is registered already; dispatch with replace: true to replace it" }
            "replacing ${Distro}: unregistering it"
            & wsl.exe --unregister $Distro
            if ($LASTEXITCODE -ne 0) { Fail "wsl --unregister ${Distro}: exit $LASTEXITCODE" }
        }
        New-Item -ItemType Directory -Force -Path $Disk | Out-Null
        "wsl --import $Distro $Disk (WSL 2), as $([Environment]::UserName)"
        & wsl.exe --import $Distro $Disk $rootfs --version 2
        if ($LASTEXITCODE -ne 0) { Fail "wsl --import: exit $LASTEXITCODE" }
        # From here the distro is registered but unverified. Unless the manifest derived inside it is the pin, the
        # `finally` stops and unregisters it, whatever ended the `try`: a mismatch, or a terminating error (a throw, or a
        # cmdlet or native stderr under $ErrorActionPreference 'Stop'), which `catch` turns into the reason. Nothing in
        # the `try` calls `Fail` or `exit`; the one `Fail` comes after the `finally` has run.
        $verified = $false
        $why = 'verifying the imported distro stopped before it finished'
        try {
            $live = Get-LiveManifest
            $liveHash = if ($live) { Get-TextSha256 $live } else { 'none' }
            Stop-Distro
            if ($liveHash -eq $pinned) {
                $verified = $true
            } else {
                $why = "inside the imported distro the manifest derives to $liveHash, not the pinned $pinned"
            }
        } catch {
            $why = "verifying the imported distro failed: $_"
        } finally {
            if (-not $verified) {
                try { Stop-Distro } catch { }
                $unregistered = try { & wsl.exe --unregister $Distro | Out-Null; $LASTEXITCODE -eq 0 } catch { $false }
            }
        }
        if (-not $verified) {
            if ($unregistered) { Fail "${why}: unregistered it" }
            Fail "${why}; wsl --unregister $Distro failed too: unregister it by hand before registering again"
        }
        "registered ${Distro}: its manifest, derived inside it, is the pinned $pinned"
        exit 0
    }
    'proofs' {
        if ((Get-Distros) -notcontains $Distro) { Fail "$Distro isn't registered: register it first" }
        $winUser = [Environment]::UserName
        try {
            # Proof 1: non-interactive, as gh-runner, never the owner; the boot time and the console's user are kept
            # for the reboot and logged-out runs, which are this job run again.
            $session = (Get-Process -Id $PID).SessionId
            $os = Get-CimInstance Win32_OperatingSystem
            $console = (Get-CimInstance Win32_ComputerSystem).UserName
            $asCi = Invoke-Ci @('whoami')
            $default = Invoke-Wsl @('-d', $Distro, '--cd', '/', '--', 'whoami')
            $inCi = ($asCi.Lines | Select-Object -Last 1)
            $inDefault = ($default.Lines | Select-Object -Last 1)
            $ok = $winUser -eq 'gh-runner' -and $session -eq 0 -and $inCi -eq $User -and $inDefault -eq $User
            Report 1 $ok "Windows user $winUser, session $session (0: no interactive session), booted $($os.LastBootUpTime.ToString('o')), console user '$console'; inside: $inCi as -u ci, $inDefault by default"

            $cloned = Initialize-Clone
            if ($cloned) { Hide-Tailnet "the clone failed: $cloned" } else { "cloned inside at $env:GITHUB_SHA, verified" }

            # Proof 2: no drive and no Windows program reachable from inside.
            if ($cloned) { Report 2 $false "no clone: $cloned" } else {
                $r = Invoke-Ci @('bash', $Inside, 'isolation')
                Report 2 ($r.Code -eq 0) "inside.sh isolation: exit $($r.Code), $(@($r.Lines | Where-Object { $_ -like 'FAIL:*' }).Count) failed check(s)"
            }

            # Proof 3: nothing gets in to a listener inside; nothing inside gets out to the LAN, or to the host over WSL's
            # virtual switch: the distro's default gateway (in NAT mode, the host's side of the switch) and the host's
            # vEthernet and WSL addresses, each on a port the host listens on and on SMB's 445.
            if ($cloned) { Report 3 $false "no clone: $cloned" } else {
                $port = 47913
                $listener = Start-Wsl @('-d', $Distro, '-u', $User, '--cd', '/', '--', 'bash', $Inside, 'listen', "$port", '30')
                $listener.StandardInput.Close()
                $ready = $listener.StandardOutput.ReadLineAsync()
                $up = $ready.Wait(20000) -and $ready.Result -eq 'listening'
                $address = ((Invoke-Ci @('bash', $Inside, 'address')).Lines | Select-Object -Last 1)
                $selfIn = (Invoke-Ci @('bash', $Inside, 'connect', '127.0.0.1', "$port")).Code -eq 0
                $inbound = @(foreach ($target in @($address, '127.0.0.1', 'localhost')) { if ($target -and (Test-Tcp $target $port)) { $target } })
                $listener.WaitForExit(40000) | Out-Null
                if (-not $listener.HasExited) { $listener.Kill() }
                # Its output after `listening`, once that line was read; if it never came, the proof fails on $up.
                $rest = if ($ready.IsCompleted) { $listener.StandardOutput.ReadToEnd() } else { '' }
                $accepts = @($rest -split "`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ -like 'accepted *' })
                $accepted = @($accepts | Where-Object { $_ -ne 'accepted 127.0.0.1' })
                # From 127.0.0.1: the connection from inside, if it was made, and any that WSL's localhost forwarding
                # relayed from Windows, which arrive from 127.0.0.1 too: those beyond the one from inside got in.
                $relayed = [Math]::Max(0, @($accepts | Where-Object { $_ -eq 'accepted 127.0.0.1' }).Count - $(if ($selfIn) { 1 } else { 0 }))

                $lan = @(Get-NetIPAddress -AddressFamily IPv4 | Where-Object {
                        $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' -and
                        $_.InterfaceAlias -notlike 'vEthernet*' -and $_.InterfaceAlias -notlike '*WSL*' } |
                    ForEach-Object { $_.IPAddress })
                $gateway = ((Invoke-Ci @('bash', $Inside, 'gateway')).Lines | Select-Object -Last 1)
                if ($gateway -notmatch '^\d{1,3}(\.\d{1,3}){3}$') { $gateway = '' }
                $hostSwitch = @(@($gateway) + @(Get-NetIPAddress -AddressFamily IPv4 | Where-Object {
                            $_.InterfaceAlias -like 'vEthernet*' -or $_.InterfaceAlias -like '*WSL*' } |
                        ForEach-Object { $_.IPAddress }) | Where-Object { $_ } | Select-Object -Unique)
                $outPort = 47914
                $server = New-Object System.Net.Sockets.TcpListener([Net.IPAddress]::Any, $outPort)
                $server.Start()
                try {
                    $reachable = @($lan | Where-Object { Test-Tcp $_ $outPort })
                    $outbound = @(foreach ($ip in $reachable) { if ((Invoke-Ci @('bash', $Inside, 'connect', $ip, "$outPort")).Code -eq 0) { $ip } })
                    # The gateway must be the host's own, listening on the port, or its refusal shows nothing.
                    $gatewayListens = $gateway -and (Test-Tcp $gateway $outPort)
                    $smbListens = $gateway -and (Test-Tcp $gateway 445)
                    $viaSwitch = @(foreach ($ip in $hostSwitch) {
                            foreach ($p in @($outPort, 445)) { if ((Invoke-Ci @('bash', $Inside, 'connect', $ip, "$p")).Code -eq 0) { "${ip}:$p" } } })
                } finally { $server.Stop() }
                $ok = $up -and $selfIn -and $address -and -not $inbound -and -not $accepted -and $relayed -eq 0 -and $reachable.Count -gt 0 -and -not $outbound -and $gatewayListens -and -not $viaSwitch
                Report 3 $ok "listener inside on ${address}:$port up: $up, reached from inside: $selfIn; reached from Windows at: '$($inbound -join ', ')', accepted from outside: '$($accepted -join ', ')', relayed through localhost: $relayed; the host's LAN addresses '$($lan -join ', ')', listening on: '$($reachable -join ', ')', reached from inside: '$($outbound -join ', ')'; the gateway inside '$gateway', the host listening there on ${outPort}: $gatewayListens, on 445: $smbListens; the switch's host addresses '$($hostSwitch -join ', ')', reached from inside on $outPort or 445: '$($viaSwitch -join ', ')'"
            }

            # Proof 4: contain.sh check, unprivileged, with its real `kill -9 -1`; a ci process outside its namespace
            # (another session's, beside the check's own) and every gh-runner process on Windows must survive it.
            if ($cloned) { Report 4 $false "no clone: $cloned" } else {
                $mark = '600.4217'
                $sentinel = Start-Wsl @('-d', $Distro, '-u', $User, '--cd', '/', '--', 'sleep', $mark)
                $sentinel.StandardInput.Close()
                $winSentinel = Start-Process -FilePath 'powershell.exe' -ArgumentList '-NoProfile', '-NonInteractive', '-Command', 'Start-Sleep -Seconds 600' -WindowStyle Hidden -PassThru
                Start-Sleep -Seconds 2
                $seen = (Invoke-Ci @('pgrep', '-x', '-f', "sleep $mark")).Code -eq 0
                $before = @(Get-RunnerProcesses)
                $agent = @($before | Where-Object { $_.Name -in 'Runner.Listener.exe', 'Runner.Worker.exe' })
                $check = Invoke-Ci @('bash', $Inside, 'check')
                $after = @(Get-RunnerProcesses)
                $lost = @($before | Where-Object { $after.Key -notcontains $_.Key })
                $ciAlive = (Invoke-Ci @('pgrep', '-x', '-f', "sleep $mark")).Code -eq 0
                $winAlive = -not $winSentinel.HasExited
                if (-not $sentinel.HasExited) { $sentinel.Kill() }
                if ($winAlive) { Stop-Process -Id $winSentinel.Id -Force }
                $ok = $check.Code -eq 0 -and $seen -and $ciAlive -and $agent.Count -ge 2 -and -not $lost -and $winAlive
                Report 4 $ok "contain.sh check (unprivileged): exit $($check.Code); the ci process outside: seen before $seen, alive after $ciAlive; gh-runner's Windows processes: $($before.Count) before, the runner agent's $($agent.Count) ($(($agent | ForEach-Object { $_.Name }) -join ', ')), lost: '$(($lost | ForEach-Object { "$($_.Name) $($_.Id)" }) -join ', ')'; a Windows process started for it alive after: $winAlive"
            }

            # Proof 5: the manifest after a rebuild (this run's hosted build) and the registered distro's, derived inside
            # it, each equal to the pin.
            $pin = (Get-Content -LiteralPath 'scripts\wsl\manifest.sha256' -Raw).Trim()
            $rebuilt = Join-Path $env:WSL_IMAGE 'manifest.txt'
            $rebuiltHash = if (Test-Path -LiteralPath $rebuilt) { Get-Sha256 $rebuilt } else { 'none' }
            $live = Get-LiveManifest
            $liveHash = if ($live) { Get-TextSha256 $live } else { 'none' }
            Report 5 ($pin -match '^[0-9a-f]{64}$' -and $rebuiltHash -eq $pin -and $liveHash -eq $pin) "pinned '$pin'; this run's rebuild $rebuiltHash; the registered distro's, derived inside, $liveHash"

            # Proof 6: one pinned commit and one CPU command, in WSL and on the hosted runner, test by test.
            if ($cloned) { Report 6 $false "no clone: $cloned" } else {
                $commit = $env:WSL_PARITY_COMMIT
                $parity = Invoke-Ci @('bash', $Inside, 'parity', $commit)
                if (Test-Path -LiteralPath "$env:WSL_HOSTED_PARITY") {
                    $compare = Invoke-Ci @('bash', $Inside, 'compare') ([IO.File]::ReadAllBytes($env:WSL_HOSTED_PARITY))
                } else {
                    $compare = [pscustomobject]@{ Code = 1; Lines = @("parity: the hosted runner's results are missing: '$env:WSL_HOSTED_PARITY'") }
                }
                $evidence = "at $commit, cargo nextest run -p kernel --profile ci: $(($compare.Lines | Where-Object { $_ -like 'parity:*' }) -join ' / ')"
                if ($parity.Code -ne 0) { $evidence += "; inside.sh parity exited $($parity.Code), ending: $(Get-Tail $parity)" }
                if ($compare.Code -ne 0) { $evidence += "; inside.sh compare exited $($compare.Code), ending: $(Get-Tail $compare)" }
                Report 6 ($parity.Code -eq 0 -and $compare.Code -eq 0) $evidence
            }
        } finally {
            Stop-Distro
            New-Item -ItemType Directory -Force -Path $Record | Out-Null
            $results | Set-Content -LiteralPath (Join-Path $Record 'wsl-proofs.txt')
        }
        ''
        $results
        $failed = @($results | Where-Object { $_ -notlike 'proof *: PASS:*' }).Count
        if ($results.Count -ne 6 -or $failed) { "wsl.ps1 proofs: $failed of 6 failed"; exit 1 }
        'wsl.ps1 proofs: all six passed'
        exit 0
    }
    'cpu' {
        if (-not (Test-Path -LiteralPath '.github\wsl-cpu-enabled')) {
            Fail '.github/wsl-cpu-enabled is absent: CPU tests and mutants in WSL are off until all six proofs have passed and a reviewed PR adds it'
        }
        if ((Get-Distros) -notcontains $Distro) { Fail "$Distro isn't registered" }
        try {
            $cloned = Initialize-Clone
            if ($cloned) { Fail $cloned }
            $run = Invoke-Ci @('bash', $Inside, 'cpu')
            New-Item -ItemType Directory -Force -Path $Record | Out-Null
            $tar = Invoke-Ci @('bash', $Inside, 'results') $null (Join-Path $Record 'wsl-cpu-results.tar')
            "the results' tar stream: exit $($tar.Code)"
        } finally {
            Stop-Distro
        }
        exit $(if ($run.Code -ne 0) { $run.Code } else { $tar.Code })
    }
    default {
        "wsl.ps1: unknown verb '$verb'"
        exit 2
    }
}
