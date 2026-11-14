# TASK-M0-71 — The PC runs the GPU suites only: `win-tests` leaves `win-gpu.yml`, and no test on the runner starts a bare `bash` or `wsl`

- **Milestone:** M0
- **Closes:** REQ-TOOL-244
- **Depends on:** TASK-M0-55
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~450 lines, the guard test and its controls the most of them; ~110 lines removed with `win-tests`

## Goal
The owner's words of 14 Nov 2026: "1. The PC runs the GPU suites only. Tests that fail there for lack of Unix tools
(most of the 382) are excluded on Windows with a stated reason (they run on hosted Linux already), not fixed. Fix only
failures that could mean a wrong result on Windows. ... 5. Windows work gets one agent slot at a time". The same day the
scheduled `win-tests` job (run 43593938536) showed why it is urgent: xtask's tests that run `Command::new("bash")`
found C:\Windows\System32\bash.exe, WSL's launcher, before any bash on PATH, and so ran in the account's default
distro, `principia-wsl`, as `ci`; the CARGO_HOME and HOME the tests set don't cross into WSL, so
`.github/actions/cargo-ci-config/config.sh` appended three copies to `ci`'s own `~/.cargo/config.toml`, invalid TOML,
breaking proofs 5 and 6. Any job on the runner that starts a bare `bash` or `wsl` can change the distro. So `win-tests`
leaves the PC, and a test fails if any test a job on the runner can reach starts a bare `bash` or `wsl`.

## References
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-859 — From the anchor move, the Windows runner may run, as named measurements and never as required checks: the GPU suites (Vulkan now, DX12 from Phase 2) nightly and on pushes to `main`, gpu-idle, the weekly proof and the 7 Nov heavy-work comparison; benchmarks of record stay on the Mac, Windows numbers a separate named series; moving required or heavy CI work to the PC is not approved and returns through `/rulings-report`; the runner's security rules are unchanged *(refines R-186, R-656 and R-711)*"
- `decisions.md` § "R-860 — R-859's list of what the Windows runner may run is not exhaustive: the nightly `win-tests` job and the Windows arms of the path-cover and time-column benchmarks stay as named measurements, never required checks *(closes RQ-742; refines R-859)*"
- `decisions.md` § "R-988 — A job on the Windows PC that stops unclearly is re-run once, automatically, by a hosted job; a hung one ends at a time limit of 1.5 times its measured time; both are logged *(the owner's goal of 12 Nov 2026, item 5; amends R-859's security rules, the owner's job rules of 5 Nov 2026, for this re-run; extends R-714; refines R-970; read with R-711 and R-968)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"

## Deliverables
- `.github/workflows/win-gpu.yml`: the `win-tests` job and its `only` choice removed; the header's account of the
  jobs, of the GPU-only rule and of the concurrency groups kept true. `win-vulkan`, `win-self-check`, `win-contain` and
  the WSL jobs unchanged.
- `.github/win-times.md`: `win-tests`' seven rows leave the table, which names nothing the workflows don't have, with a
  note of where they stand in history.
- `.config/nextest.toml`: the `ci` profile's `cfg(windows)` hang limit, which only `win-tests` used, removed, with its
  test, `xtask/tests/ci_win_test_limit.rs`.
- `xtask/tests/ci_self_hosted.rs`: the runner's job list and nightly-only list without `win-tests`, and its controls
  that edited `win-tests` pointed at `win-self-check`, each still tripping on the same rule.
- `xtask/tests/ci_win_no_wsl.rs`: reads every cargo command of every job of `win-gpu.yml` on the runner, with the
  nextest profile its step names, and finds each test file it can reach (a `--test` target, a profile's
  `binary_id(=…)` list, every test file holding a name filter's text, or every test of a package run whole, and the
  modules each includes); fails if any of them, or any library source of the workspace, holds a `Command::new` of
  `bash`, `wsl`, or System32's copy of either, unless the file is left out on Windows by an inner `#![cfg(unix)]` or
  `#![cfg(not(windows))]` with its reason in the comment above it; fails on a command it can't read. It starts no
  process. With its negative controls (R-176).
- Any test a job on the runner reaches that starts a bare `bash` or `wsl` is left out on Windows with its reason, or
  pointed at Git's bash by its path; no assertion is loosened. (On this tree none is reached once `win-tests` is gone:
  the GPU suites' targets, the name filters' files in `engine`, `validation` and `xtask`, and `ci_self_hosted.rs`
  start no shell, and the libraries' only shell, `sh` in `validation::spawn`'s unit tests, is not WSL's and no filter
  of the runner selects them.)
- `plan/OPERATIONS.md` § "The Windows runner's work (R-859)": the owner's 14 Nov words and what they changed;
  REQ-TOOL-224's statement without `win-tests`.

## Acceptance tests
- `cargo test -p xtask --test ci_win_no_wsl` and `cargo test -p xtask --test ci_win_no_wsl --features controls`:
  `win_runner_tests_start_no_bare_bash_or_wsl`, `the_runner_reaches_the_gpu_suites_and_the_guard` and
  `a_file_excluded_on_windows_with_its_reason_is_allowed` pass on the tree; every control fails as registered: a
  whole-workspace test step put back on the runner, a bare `bash` in a reached test target, System32's `wsl.exe` by
  its path in a GPU target, `wsl` in a new test file a name filter selects, `bash` in a module a reached target
  includes, `bash` in a library source, a Windows exclusion with no reason above it, a test argument the check can't
  read, the guard's step gone, and a whole-package xtask run reaching the job queue's tests (REQ-TOOL-244).
- `cargo test -p xtask --test ci_self_hosted` and `--features controls`: every rule holds on the narrowed workflow,
  and every control fails as registered (REQ-TOOL-244).
- `python3 plan/check_plan.py` passes.

## Notes
- Entered 14 Nov 2026 from the owner's words above (applied per R-369: task scoping, the build of the owner's
  direction as its own task). They narrow R-860, which kept `win-tests` as a named measurement; the orchestrator
  records them in `decisions.md`.
- Not in scope (other tasks): `build-image.sh` making `config.toml` root-owned, `inside.sh parity` failing loudly, the
  re-registration of the distro, and the runner account's default-distro setting on the PC (the owner's call).
- qa's `xtask/tests/qa_TASK-M0-61.rs` has two controls that edit `win-tests`' text (`qa_other_job_cancels` and the
  limits' `c9`); with the job gone their edits find nothing, so they need qa's change, an exception the owner's words
  force (R-369).
- The PC runs in the one Windows slot; nothing on `main` waits for it.
