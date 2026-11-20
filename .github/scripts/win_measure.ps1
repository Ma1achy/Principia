# The Windows measurement jobs' record (`.github/workflows/win-gpu.yml`; R-649, R-711). Every result carries the label
# below in the job's summary, with its commit and driver version, and its full output is kept in
# $RUNNER_TEMP\win-measure, which the job uploads: item 4 of that ruling lets a recorded run serve Paper 2's non-Metal
# parity evidence.
#
#   win_measure.ps1 start                       Record the identity of the run. Its reads can't fail the job.
#   win_measure.ps1 run <name> <command> <arg>  Run the command, its output shown and kept in <name>.log, and exit with
#                                               its status. With $env:WIN_MEASURE_EXPECT set, a passing command whose
#                                               output doesn't hold that text fails, and the record says why.
#                                               With $env:WIN_MEASURE_REPORT set, in a reporting step's own env alone,
#                                               the result is reported, not asserted (R-694, R-695; TASK-M0-88): its
#                                               pass and fail counts and key lines go to the job's summary under that
#                                               label, its real status to the record, and it exits 0.
#   win_measure.ps1 finish                      Write the job's summary, and copy in what this run wrote to the target
#                                               folder's reports.
#
# PowerShell 5.1, the version the runner's `shell:` starts, whose `Tee-Object` writes UTF-16: hence the writer below.
$ErrorActionPreference = 'Continue'
$verb, $rest = $args
$label = 'possibly faulty hardware (owner-reported)'
$dir = Join-Path $env:RUNNER_TEMP 'win-measure'
$identity = Join-Path $dir 'identity.txt'
$results = Join-Path $dir 'results.tsv'
$started = Join-Path $dir 'started.txt'

# One tool's output for the record; a tool that's missing or fails says so, and the job goes on.
function Read-Tool($name, $tool, $toolArgs) {
    $text = try { (& $tool @toolArgs 2>&1 | ForEach-Object { "$_" }) -join ' | ' } catch { "unavailable: $_" }
    "${name}: $text"
}

switch ($verb) {
    'start' {
        New-Item -ItemType Directory -Force $dir | Out-Null
        Set-Content -LiteralPath $started -Value (Get-Date -Format o)
        $lines = @(
            "label: $label"
            "commit: $env:GITHUB_SHA"
            "ref: $env:GITHUB_REF"
            "event: $env:GITHUB_EVENT_NAME"
            "actor: $env:GITHUB_ACTOR"
            "run: $env:GITHUB_RUN_ID attempt $env:GITHUB_RUN_ATTEMPT"
            "job: $env:GITHUB_JOB"
            "runner: $env:RUNNER_NAME"
            "backend: $env:PRIN_GPU_BACKEND"
            Read-Tool 'driver' 'nvidia-smi' @('--query-gpu=name,driver_version', '--format=csv,noheader')
            Read-Tool 'rustc' 'rustc' @('-vV')
            Read-Tool 'nextest' 'cargo' @('nextest', '--version')
        )
        $lines | Set-Content -LiteralPath $identity
        $lines
        exit 0
    }
    'run' {
        $name, $command, $commandArgs = $rest
        $log = Join-Path $dir "$name.log"
        $writer = New-Object System.IO.StreamWriter($log, $true)
        $writer.AutoFlush = $true
        $clock = [System.Diagnostics.Stopwatch]::StartNew()
        # Cleared, so that a command that never started can't pass on an earlier status, or on none.
        $global:LASTEXITCODE = $null
        & $command @commandArgs 2>&1 | ForEach-Object { $line = "$_"; $writer.WriteLine($line); $line }
        $code = $LASTEXITCODE
        $writer.Dispose()
        if ($null -eq $code) { $code = 1 }
        $note = ''
        if ($env:WIN_MEASURE_EXPECT -and $code -eq 0 -and -not (Select-String -LiteralPath $log -SimpleMatch -Pattern $env:WIN_MEASURE_EXPECT -Quiet)) {
            $code = 1
            $note = "refused: no '$env:WIN_MEASURE_EXPECT' in the output"
            "win_measure: $note"
        }
        $seconds = [int]$clock.Elapsed.TotalSeconds
        if ($env:WIN_MEASURE_REPORT) {
            # Reported, not asserted: the status is recorded and shown, never returned. The key lines: nextest's per-test
            # and summary lines, its timing stripped so the final repeat of a failure counts once, and the golden runner's
            # per-case verdicts.
            $keys = @(Get-Content -LiteralPath $log | ForEach-Object { "$_".Trim() } |
                Where-Object { $_ -match '^(PASS|FAIL|SIGSEGV|SIGABRT|TIMEOUT) \[' -or $_ -match '^Summary \[' -or $_ -match '^xtask golden: .+: (pass|FAIL) on ' } |
                ForEach-Object { $_ -replace '^(PASS|FAIL|SIGSEGV|SIGABRT|TIMEOUT) \[[^\]]*\] \([^)]*\) ', '$1 ' } |
                Select-Object -Unique)
            $passed = @($keys | Where-Object { $_ -match '^PASS ' -or $_ -match ': pass on ' }).Count
            $failed = @($keys | Where-Object { $_ -match '^(FAIL|SIGSEGV|SIGABRT|TIMEOUT) ' -or $_ -match ': FAIL on ' }).Count
            $verdict = "reported, not asserted: exit $code, $passed passed, $failed failed"
            $summary = @(
                "### ${name}: $env:WIN_MEASURE_REPORT"
                ''
                "$verdict. This step does not fail the job."
                ''
                '```'
                $keys
                '```'
            )
            [System.IO.File]::AppendAllText($env:GITHUB_STEP_SUMMARY, ($summary -join "`n") + "`n")
            Add-Content -LiteralPath $results -Value "$name`t$code`t$seconds`t$verdict $note"
            "win_measure: $name $verdict"
            exit 0
        }
        Add-Content -LiteralPath $results -Value "$name`t$code`t$seconds`t$note"
        exit $code
    }
    'finish' {
        $since = [datetime](Get-Content -LiteralPath $started)
        foreach ($report in 'golden', 'gates') {
            $from = Join-Path $env:CARGO_TARGET_DIR $report
            if (Test-Path -LiteralPath $from) {
                $fresh = @(Get-ChildItem -LiteralPath $from -Recurse -File | Where-Object { $_.LastWriteTime -ge $since })
                foreach ($file in $fresh) {
                    $to = Join-Path (Join-Path $dir $report) $file.FullName.Substring($from.Length + 1)
                    New-Item -ItemType Directory -Force (Split-Path $to) | Out-Null
                    Copy-Item -LiteralPath $file.FullName -Destination $to
                }
            }
        }
        $summary = @(
            "## ${env:GITHUB_JOB}: $label"
            ''
            'A measurement, never a required check (R-649). Windows-only results that do not reproduce are hardware-suspect.'
            ''
            '```'
            Get-Content -LiteralPath $identity
            '```'
            ''
            'A suite not listed here did not run.'
            ''
            '| suite | exit | seconds | note |'
            '|---|---|---|---|'
        )
        if (Test-Path -LiteralPath $results) {
            $summary += Get-Content -LiteralPath $results | ForEach-Object { '| ' + ($_ -replace "`t", ' | ') + ' |' }
        }
        [System.IO.File]::AppendAllText($env:GITHUB_STEP_SUMMARY, ($summary -join "`n") + "`n")
        exit 0
    }
    default {
        "win_measure: unknown verb '$verb'"
        exit 2
    }
}
