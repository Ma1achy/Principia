# TASK-M0-70 — win-self-check's guard firing plants a HEAD, so the guard logs the planted hook's reason

- **Milestone:** M0
- **Closes:** REQ-TOOL-243
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~110 lines, the test and its controls included

## Goal
The owner's Windows goal of 14 Nov 2026: win-self-check green. After TASK-M0-64 merged, win-self-check run 43572471690
failed 1 of 307: `git_guard_script_refuses_what_is_planted` (`xtask/tests/ci_self_hosted.rs`, `cfg(windows)`) at its
firing's log check, "git_guard: the firing logged no reason". The test is wrong, not the guard: the firing's `.git`
holds `config` and a hook but no `HEAD`, which `git_guard.ps1`'s `Find-Planted` has read in every `.git` since
5d242dd09 (5 Nov 2026). The read throws, the script's outer `catch` replaces what it had found with "can't read the
checkout", it refuses, empties the checkout and logs that reason, so the log never holds "active hook". Every earlier
assertion of the firing passed in that run, as a refusal for either reason meets them. The firing plants a HEAD as a
clean checkout holds it; the guard script and every assertion of the test are unchanged (R-711's security rules,
re-proved nightly by win-self-check, R-860).

## References
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-860 — R-859's list of what the Windows runner may run is not exhaustive: the nightly `win-tests` job and the Windows arms of the path-cover and time-column benchmarks stay as named measurements, never required checks *(closes RQ-742; refines R-859)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"

## Deliverables
- `xtask/tests/ci_self_hosted.rs`: the firing's `.git` as one list, `FIRING`: a clean `.git/config`,
  `.git/HEAD` at `ref: refs/heads/main`, and the planted `.git/hooks/pre-commit`; `check_script_refuses`'s firing
  plants it. The firing's log assertion, unchanged in what it checks, prints the log and the script's output when it
  fails.
- A test that runs on every platform, `git_guard_firing_plants_what_the_script_reads`: each file the script reads by
  `Join-Path $git.FullName '<name>'` is in `FIRING`, and `FIRING`'s HEAD matches the script's own HEAD pattern.
- `.github/scripts/git_guard.ps1` unchanged; no assertion of `check_script_refuses` removed or weakened.

## Acceptance tests
- `cargo test -p xtask --test ci_self_hosted git_guard_firing_plants` — the firing plants what the script reads;
  controls: the firing with no HEAD, a HEAD the script refuses, and a script whose reads the test can't find, each
  fail (REQ-TOOL-243).
- `cargo test -p xtask --test ci_self_hosted` and `cargo test -p xtask --features controls --test ci_self_hosted` on
  the Mac pass; `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` pass (REQ-TOOL-243).
- On the PC, win-self-check's `git_guard_script_refuses_what_is_planted` passes on the first run on `main` after
  merge, its control, the script with its lists off, still failing; the run id reported in the PR or a follow-up
  comment, a measurement (R-711) (REQ-TOOL-243).

## Notes
- Entered 14 Nov 2026 (applied per R-369: CI tooling, a test-only fix, two-way).
- `xtask/tests/ci_self_hosted.rs` is tooling-owned: no qa commit has touched it, so the implementer edits it.
- The firing is `cfg(windows)`: the Mac proves the firing plants what the script reads; only the PC proves the guard
  logs the hook, and that `[AppendOnly]::Add`, the PC's writer, writes the test's own log.
