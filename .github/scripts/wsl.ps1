# The Windows side of the WSL2 distro `principia-wsl` (RQ-751 option (c), built in parallel and never blocking; the
# owner's words of 9 Nov 2026). The self-hosted runner's dispatch-only jobs start it through the record,
# `.github/scripts/win_measure.ps1 run`, as the runner's account, gh-runner. Inside the distro everything runs as `ci`,
# through `scripts/wsl/inside.sh`, from a clone at the job's own commit; interop is off, so nothing inside can reach
# Windows, and Windows reads only what `wsl.exe` relays: stdout, and the exit status.
#
#   wsl.ps1 register   Imports the image the same run built (WSL_IMAGE: rootfs.tar.gz, rootfs.sha256, manifest.txt),
#                      as gh-runner, to E:\wsl\principia-wsl\, never C:. Refuses unless a manifest hash is pinned in
#                      scripts/wsl/manifest.sha256 and the image's manifest has it, and unless the copy's bytes are the
#                      ones built. Refuses if the distro exists, unless WSL_REPLACE is `true`. After the import it
#                      derives the manifest again inside the distro, from what is really there, and unregisters it if
#                      that differs from the pin.
#   wsl.ps1 proofs     The six proofs RQ-751 (c) needs before it can replace (b), each reported PASS or FAIL with its
#                      evidence; exits 0 only if all six pass. WSL_PARITY_COMMIT and WSL_HOSTED_PARITY (the hosted
#                      runner's results, this run's) feed proof 6; WSL_IMAGE (this run's rebuild) feeds proof 5.
#   wsl.ps1 cpu        Refuses unless `.github/wsl-cpu-enabled` exists (a reviewed PR adds it once all six proofs have
#                      passed). Then the CPU tests and the job queue's mutants inside, through `scripts/contain.sh` in
#                      its unprivileged mode; their results come out as a tar stream on a second `wsl.exe`'s stdout.
#
# Every verb ends with `wsl.exe --terminate`, which ends every process in the distro, whatever was left running.
$ErrorActionPreference = 'Stop'
$verb = $args[0]
$Distro = 'principia-wsl'
$Disk = 'E:\wsl\principia-wsl'
$User = 'ci'
$Repo = '/home/ci/work/principia'
$Inside = "$Repo/scripts/wsl/inside.sh"
$Record = Join-Path $env:RUNNER_TEMP 'win-measure'
# wsl.exe's own messages in UTF-8, not UTF-16.
$env:WSL_UTF8 = '1'

function Fail($message) {
    "::error::wsl.ps1: $message"
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
    [System.Diagnostics.Process]::Start($psi)
}

# `wsl.exe <arguments>`: its stdout shown line by line as it comes (or, with -OutFile, written there as bytes), its
# stderr after; -Stdin's bytes on its stdin. Returns the exit status and the stdout lines.
function Invoke-Wsl([string[]]$Arguments, [byte[]]$Stdin = $null, [string]$OutFile = $null) {
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
            [Console]::Out.WriteLine($line)
            $lines.Add($line)
        }
    }
    $p.WaitForExit()
    $e = $err.Result
    if ($e) { [Console]::Out.WriteLine($e.TrimEnd()) }
    [pscustomobject]@{ Code = $p.ExitCode; Lines = $lines.ToArray() }
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

function Get-TextSha256([string[]]$Lines) {
    $bytes = [Text.Encoding]::UTF8.GetBytes((($Lines -join "`n") + "`n"))
    $sha = [Security.Cryptography.SHA256]::Create()
    -join ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') })
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
function Report([int]$Proof, [bool]$Passed, [string]$Evidence) {
    $line = "proof ${Proof}: $(if ($Passed) { 'PASS' } else { 'FAIL' }): $Evidence"
    $line
    $script:results.Add($line)
}

switch ($verb) {
    'register' {
        $pinned = Get-Pinned
        $image = $env:WSL_IMAGE
        $rootfs = Join-Path $image 'rootfs.tar.gz'
        $manifest = Join-Path $image 'manifest.txt'
        foreach ($f in $rootfs, $manifest, (Join-Path $image 'rootfs.sha256')) {
            if (-not (Test-Path -LiteralPath $f)) { Fail "the image has no $f" }
        }
        $built = (Get-Content -LiteralPath (Join-Path $image 'rootfs.sha256') -Raw).Trim()
        $copied = Get-Sha256 $rootfs
        if ($copied -ne $built) { Fail "rootfs.tar.gz is $copied here, $built as built: the copy is not the image" }
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
        $live = Get-LiveManifest
        $liveHash = if ($live) { Get-TextSha256 $live } else { 'none' }
        Stop-Distro
        if ($liveHash -ne $pinned) {
            & wsl.exe --unregister $Distro | Out-Null
            Fail "inside the imported distro the manifest derives to $liveHash, not the pinned ${pinned}: unregistered it"
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
            if ($cloned) { "the clone failed: $cloned" } else { "cloned inside at $env:GITHUB_SHA, verified" }

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
                $accepted = @($rest -split "`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ -like 'accepted *' -and $_ -ne 'accepted 127.0.0.1' })

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
                $ok = $up -and $selfIn -and $address -and -not $inbound -and -not $accepted -and $reachable.Count -gt 0 -and -not $outbound -and $gatewayListens -and -not $viaSwitch
                Report 3 $ok "listener inside on ${address}:$port up: $up, reached from inside: $selfIn; reached from Windows at: '$($inbound -join ', ')', accepted from outside: '$($accepted -join ', ')'; the host's LAN addresses '$($lan -join ', ')', listening on: '$($reachable -join ', ')', reached from inside: '$($outbound -join ', ')'; the gateway inside '$gateway', the host listening there on ${outPort}: $gatewayListens, on 445: $smbListens; the switch's host addresses '$($hostSwitch -join ', ')', reached from inside on $outPort or 445: '$($viaSwitch -join ', ')'"
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
                Report 6 ($parity.Code -eq 0 -and $compare.Code -eq 0) "at $commit, cargo nextest run -p kernel --profile ci: $(($compare.Lines | Where-Object { $_ -like 'parity:*' }) -join ' / ')"
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
