# TASK-M0-64 — win-self-check's guard test plants a clean checkout's config as the checkout leaves it, the sparse keys in config.worktree

- **Milestone:** M0
- **Closes:** REQ-TOOL-234
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~120 lines, the test and its controls included

## Goal
The owner's goal of 12 Nov 2026 for the Windows runner, item E: win-self-check green. Its
`git_guard_script_refuses_what_is_planted` (`xtask/tests/ci_self_hosted.rs`, `cfg(windows)`) has failed on the PC on
every nightly since 5d242dd09 (5 Nov 2026). The test is wrong, not the guard: its "whole" case writes every pair of the
guard's `$allowed`, the three sparse-checkout keys included, into `.git/config`, while the kept checkout under it
already holds those three in `.git/config.worktree`, where `sparse-checkout disable` writes them; the guard script
reads both files and rightly refuses "repeated config key". The test plants the whole list as a clean checkout holds
it: the sparse keys in `.git/config.worktree`, the rest in `.git/config`. The guard script and what the test checks are
unchanged (R-711's security rules, re-proved nightly by win-self-check, R-860).

## References
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-860 — R-859's list of what the Windows runner may run is not exhaustive: the nightly `win-tests` job and the Windows arms of the path-cover and time-column benchmarks stay as named measurements, never required checks *(closes RQ-742; refines R-859)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"

## Deliverables
- `xtask/tests/ci_self_hosted.rs`: a helper giving the guard's whole list, or the list with one wrong value, as the two
  files a clean checkout holds, the keys a clean kept checkout holds in `.git/config.worktree` in that file and the
  rest in `.git/config`; `check_script_refuses`'s "whole" case and its wrong-value cases plant through it.
- A test that runs on every platform, `git_guard_whole_config_splits_as_a_clean_checkout`: the planted
  `.git/config.worktree` holds exactly the keys a clean kept checkout's does, and the two files together hold every
  pair of the list once, each read back by the script's own git line.
- `.github/scripts/git_guard.ps1` unchanged; no assertion of `check_script_refuses` removed or weakened.

## Acceptance tests
- `cargo test -p xtask --test ci_self_hosted git_guard_whole_config_splits` — the split holds; controls: the whole list
  planted in `.git/config` alone fails, and the sparse keys planted in both files fails (REQ-TOOL-234).
- `cargo test -p xtask --test ci_self_hosted` and `cargo test -p xtask --features controls --test ci_self_hosted` on
  the Mac pass; `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` pass (REQ-TOOL-234).
- On the PC, win-self-check's `git_guard_script_refuses_what_is_planted` passes on the first run on `main` after
  merge, its control, the script with its lists off, still failing; the run id reported in the PR or a follow-up
  comment, a measurement (R-711) (REQ-TOOL-234).

## Notes
- Entered 13 Nov 2026 (applied per R-369: CI tooling, a test-only fix, two-way).
- `xtask/tests/ci_self_hosted.rs` is tooling-owned: no qa commit has touched it, so the implementer edits it.
- The test is `cfg(windows)`: the Mac proves the split and compiles the helper; only the PC proves the guard passes it.
