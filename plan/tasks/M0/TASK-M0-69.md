# TASK-M0-69 — Proof 2's two `wsl.conf` lines (R-1010): `[gpu] enabled=false` and a `[boot] command=` removing the `WSLInterop` registration, the image test, the re-pin and the re-registration

- **Milestone:** M0
- **Closes:** REQ-TOOL-242
- **Depends on:** TASK-M0-63
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~150 lines, the tests and their controls included

## Goal
The first real run of the WSL distro's proofs (run 43418528499, `wsl-proofs`) found two holes to Windows that WSL
opens on its own, so proof 2 (isolation) fails: `/usr/lib/wsl/drivers`, the driver store on C:, is 9p-mounted by WSL's
GPU share, on by default; and `/proc/sys/fs/binfmt_misc/WSLInterop` is registered at every start whatever
`[interop] enabled` says, and only root can remove it. R-1010 (RQ-815 (a)) lets the distro's `wsl.conf` gain exactly
two things to close them: `[gpu] enabled=false`, and a `[boot] command=` that removes the `WSLInterop` registration
(writing `-1` to `/proc/sys/fs/binfmt_misc/WSLInterop`, which WSL runs as root at the distro's start, without
systemd). Nothing else in the file changes: no `systemd=`, no sudo, no sudoers entry, the owner's other lines as they
gave them. The image test follows the new file, keeping its ban on `systemd=`; the manifest is re-pinned; and after
merge the distro is re-registered with `replace: true` and proof 2 run against RQ-815's criterion, written before any
run (R-649): every line of `inside.sh`'s `isolation()` passes unchanged, with `/usr/lib/wsl/drivers` unmounted and
`WSLInterop` absent.

## References
- `decisions.md` § "R-1010 — RQ-815, proof 2's two holes to Windows: option (a); the WSL distro's `wsl.conf` may gain `[gpu] enabled=false` and a `[boot] command=` that removes the `WSLInterop` registration, with no `systemd=` and no sudo; the image test's exact text and its `[boot]` ban follow the new file, its ban on `systemd=` kept; the manifest is re-pinned and the distro re-registered with `replace: true` *(closes RQ-815; amends R-915)*"
- `decisions.md` § "R-915 — Inside WSL, `contain.sh check` proves an unprivileged mode: user and PID namespaces with no sudo and the real `kill -9 -1`, passing only if a ci process outside the namespace and the runner agent and gh-runner's other Windows processes survive; WSL keeps no sudo, no systemd and the owner's exact `wsl.conf`; the mode is selected explicitly, never a fallback, and fails, never skips, without user namespaces; other machines keep the strict mode; proof 4 reworded *(closes RQ-753; amends R-912)*"
- `decisions.md` § "R-912 — Windows containment, RQ-751: option (b) now, the Job Object proof passing with its two process-visibility gaps accepted and the job-queue mutants staying on GitHub's runners; option (c), CPU tests and mutants in a WSL2 distro under `contain.sh`, built in parallel and never blocking, replaces (b) only once each of its five requirements is proven, the job-queue mutants then free to move to Windows; GPU tests stay native *(closes RQ-751; amends R-905)*"
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"

## Deliverables
- `scripts/wsl/wsl.conf`: the owner's file with the two additions, a `[gpu]` section holding `enabled=false` and a
  `[boot]` section holding one `command=` that writes `-1` to `/proc/sys/fs/binfmt_misc/WSLInterop`; no other line
  added, removed or changed.
- `xtask/tests/wsl.rs`: the owner's text it holds the file to (`OWNERS_CONF`) becomes that text with the two additions,
  byte for byte, its doc comment and the module's ("with no `[boot]` and so no systemd") saying what the file now holds
  and why (R-1010); `systemd=` is refused in `wsl.conf` and in `build-image.sh`; `build-image.sh` still installs the file
  unchanged and writes no `[boot]`, sudo or sudoers entry of its own. Each test with its negative controls (R-176).
- `scripts/wsl/manifest.sha256`: re-pinned to the manifest of the image the new recipe builds, as the pin was set
  first; the recipe key changes with `wsl.conf`, so the hosted `wsl-image` job builds afresh.
- Nothing in `scripts/wsl/inside.sh`'s `isolation()` changes.

## Acceptance tests
- `cargo test -p xtask --test wsl` and `cargo test -p xtask --test wsl --features controls`:
  `wsl_image_is_pinned_and_has_the_owners_config` passes on the new file, and every control fails as registered: a
  `wsl.conf` with `systemd=` anywhere, a `[boot]` section with another command or a second line, either addition
  missing, any other line changed, and `build-image.sh` writing `systemd=`, `[boot]` or a sudo entry (REQ-TOOL-242).
- `cargo test -p xtask --test ci_wsl_cache --test ci_self_hosted`: the recipe key still covers `wsl.conf` and the pin,
  and the cache and registration rules are unchanged (REQ-TOOL-242).
- After merge, on the PC: a `wsl-register` dispatch with `replace: true`
  (`gh workflow run win-gpu.yml --ref main -f only=wsl-register -f replace=true`), then a `wsl-proofs` dispatch; proof
  2's line reported against the criterion above, every `isolation()` check unchanged, with both run ids, in the PR or
  its follow-up comment. A measurement (R-711): its outcome is reported as it is, and nothing in `isolation()` is
  loosened to pass it (REQ-TOOL-242).

## Notes
- Entered 14 Nov 2026 by R-1010 (RQ-815 (a); applied per R-369: task scoping, the build of the ruling as its own task,
  since TASK-M0-63, which left proof 2's lines to RQ-815, has merged).
- If proof 2 then fails on anything else, `WSLInterop-late` among them (`isolation()` checks it too, and run
  43418528499 did not find it), that is reported and queued in `REVIEW_QUEUE.md`, not closed by a further `wsl.conf`
  line: R-1010 allows these two additions and nothing else.
- The PC runs in the one Windows slot; the dispatches wait for it, and nothing on `main` waits for them.
- `plan/OPERATIONS.md` § "The Windows runner's work (R-859)" already reads the amended file (the rulings PR for R-1010).
