# TASK-M0-63 — The WSL proofs' scripts: proof 6's stdin, byte-order mark, hosted lavapipe and failure line; proof 3's relayed connections

- **Milestone:** M0
- **Closes:** REQ-TOOL-234
- **Depends on:** TASK-M0-04
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~600 lines, the tests included

## Goal
Run 43418528499 was the first to reach `wsl-proofs`, the six proofs R-912's option (c) needs before the WSL distro can
replace (b). Proofs 1, 4 and 5 passed; 2, 3 and 6 failed. This task fixes what in the repository's scripts made proof 6
fail and what proof 3 could not show, so that the next `wsl-proofs` dispatch reports both as they really ran:

- proof 6's comparison reopened `/dev/stdin`, which `wsl.exe -u ci` refuses (EACCES): it reads the stdin it already has;
- every stdin `wsl.ps1` sent began with a UTF-8 byte-order mark (`sh: 1: <mark>#!/bin/sh: not found`), which Windows
  PowerShell's .NET writes ahead of a started process's stdin: `wsl.ps1` sets an encoding with no mark, and
  `parity.py` drops one where it arrives;
- the hosted `wsl-parity` job installed no lavapipe, so 13 of kernel's GPU harness tests failed there and passed in the
  distro: it installs what `ci.yml`'s test jobs install, through the Mesa cache's restore, and its "CPU only" comment is
  corrected;
- a failed proof 6 printed only `parity:*` lines, so a traceback left an empty line: it carries each failed side's last
  lines, stderr included;
- proof 3 discarded `accepted 127.0.0.1`, so a connection relayed in through WSL's localhost forwarding could not be
  shown: those beyond the one from inside are counted, reported and fail the proof, its pass condition no looser.

Proof 2's two fixes, `[gpu] enabled=false` and a `[boot] command=` removing the `WSLInterop` registration, change
`scripts/wsl/wsl.conf`, which R-915 keeps "exactly as I gave it": they wait on RQ-815 and are not built here. Turning
off localhost forwarding itself is a `.wslconfig` setting on the PC, not the repository's.

## References
- `decisions.md` § "R-912 — Windows containment, RQ-751: option (b) now, the Job Object proof passing with its two process-visibility gaps accepted and the job-queue mutants staying on GitHub's runners; option (c), CPU tests and mutants in a WSL2 distro under `contain.sh`, built in parallel and never blocking, replaces (b) only once each of its five requirements is proven, the job-queue mutants then free to move to Windows; GPU tests stay native *(closes RQ-751; amends R-905)*"
- `decisions.md` § "R-915 — Inside WSL, `contain.sh check` proves an unprivileged mode: user and PID namespaces with no sudo and the real `kill -9 -1`, passing only if a ci process outside the namespace and the runner agent and gh-runner's other Windows processes survive; WSL keeps no sudo, no systemd and the owner's exact `wsl.conf`; the mode is selected explicitly, never a fallback, and fails, never skips, without user namespaces; other machines keep the strict mode; proof 4 reworded *(closes RQ-753; amends R-912)*"
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-1010 — RQ-815, proof 2's two holes to Windows: option (a); the WSL distro's `wsl.conf` may gain `[gpu] enabled=false` and a `[boot] command=` that removes the `WSLInterop` registration, with no `systemd=` and no sudo; the image test's exact text and its `[boot]` ban follow the new file, its ban on `systemd=` kept; the manifest is re-pinned and the distro re-registered with `replace: true` *(closes RQ-815; amends R-915)*"

## Deliverables
- `scripts/wsl/parity.py`: `compare -` reads the open stdin; every input is read as `utf-8-sig`, a leading mark
  dropped, summaries and nextest's JSON alike.
- `scripts/wsl/inside.sh`: the `compare` arm passes `-`; proof 6's comment no longer says "CPU only".
- `.github/scripts/wsl.ps1`: `[Console]::InputEncoding` set to UTF-8 with no preamble before any process starts (the
  cached value set in its place where there is no console), `StandardInputEncoding` likewise on a newer .NET, and
  `Invoke-Wsl` refusing to send stdin while a preamble remains; `Invoke-Wsl` keeping the stderr it shows; proof 6's
  line carrying each failed side's last lines; proof 3 counting connections relayed through localhost.
- `.github/workflows/win-gpu.yml`: `wsl-parity` restores the Mesa packages and installs `mesa-vulkan-drivers` as
  `ci.yml`'s test jobs do, before proof 6's command; its comment corrected.
- `xtask/tests/wsl.rs`: the tests below, each with its negative controls. `scripts/wsl/wsl.conf` and the manifest's
  pin are unchanged, as are the recipe key and the image.

## Acceptance tests
- `cargo test -p xtask --test wsl` and `cargo test -p xtask --test wsl --features controls`:
  `wsl_parity_reads_its_stdin_past_a_byte_order_mark`, `wsl_stdin_carries_no_byte_order_mark`,
  `wsl_parity_runs_on_the_hosted_jobs_lavapipe`, `wsl_proof_six_says_how_a_side_failed` and
  `wsl_proof_three_tries_the_host_over_the_switch` pass, and every control fails as registered: `inside.sh compare`
  given `/dev/stdin`, `parity.py` reading the mark as text, the encoding left with its mark or unguarded,
  `wsl-parity` with no lavapipe, installing it late or other packages, a failed side's tail or stderr left out, a
  relayed connection passing or left out of the evidence (REQ-TOOL-234).
- `cargo test -p xtask --test ci_mesa_debs --test ci_pr_tier --test ci_self_hosted --test ci_wsl_cache`: the new apt
  step follows the cache's restore and keeps the bounded apt calls and the 8-minute limit (REQ-TOOL-234).
- After merge, a `wsl-proofs` dispatch: proof 6's line names both sides' results, and any failure its cause; the run id
  in the PR or its follow-up comment (REQ-TOOL-234).

## Notes
- Entered 13 Nov 2026 (applied per R-369: CI tooling within the runner rules, the owner's goal of 12 Nov 2026 for the
  Windows runner, item 3; task scoping, re-scoped to leave out what RQ-815 holds).
- No `wsl-register` is needed after merge: the recipe (`build-image.sh`, `manifest.sh`, `wsl.conf`, the pin,
  `rust-toolchain.toml`) is unchanged.
- *14 Nov 2026 (R-1010, RQ-815 (a)):* proof 2's two `wsl.conf` lines are allowed; TASK-M0-69 builds them, with their tests, the re-pin and the re-registration (REQ-TOOL-242). This task, merged, is unchanged.
