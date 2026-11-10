# TASK-M0-58 — The Windows runner's DX12 run, Phase 2: the GPU suites on DX12 as a named measurement (R-862, R-859)

- **Milestone:** M0
- **Closes:** REQ-TOOL-226
- **Depends on:** TASK-M0-55, TASK-M0-57
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
R-859 lets R-711's runner run "the GPU suites (Vulkan now, DX12 from Phase 2), nightly and on pushes to main", as named
measurements and never as required checks; R-711 item 2 lets DX12 run there "as a measurement job" once its opt-in lands
(TASK-M0-57). This task is Phase 2: the GPU suites on DX12 on the runner, beside the Vulkan run, under a name of its own.

## References
- `decisions.md` § "R-862 — DX12 is an opt-in, off by default: a cargo feature and `PRIN_GPU_BACKEND=dx12`, documented "local testing only, untested in CI", in no required or hosted CI job; native DX12 previews web-v1's behaviour on Windows, where Chrome runs WebGPU on D3D12 *(amends R-169 and R-206)*"
- `decisions.md` § "R-859 — From the anchor move, the Windows runner may run, as named measurements and never as required checks: the GPU suites (Vulkan now, DX12 from Phase 2) nightly and on pushes to `main`, gpu-idle, the weekly proof and the 7 Nov heavy-work comparison; benchmarks of record stay on the Mac, Windows numbers a separate named series; moving required or heavy CI work to the PC is not approved and returns through `/rulings-report`; the runner's security rules are unchanged *(refines R-186, R-656 and R-711)*"
- `decisions.md` § "R-860 — R-859's list of what the Windows runner may run is not exhaustive: the nightly `win-tests` job and the Windows arms of the path-cover and time-column benchmarks stay as named measurements, never required checks *(closes RQ-742; refines R-859)*"
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"

## Deliverables
- A DX12 job in `win-gpu.yml`: the GPU suites built with the `dx12` feature and run with `PRIN_GPU_BACKEND=dx12`,
  nightly and on pushes to `main`, keeping every rule the guard test holds for the runner's jobs (the triggers, the
  repository and actor guard, no credential, pinned actions, the self-check first through the PC's hook), on an idle
  runner with nothing alongside it.
- `xtask/tests/ci_self_hosted.rs`: the DX12 job joins `self_hosted_jobs_are_named_measurements`' list by name, and
  `dx12_in_no_required_or_hosted_job` admits it as the runner's named measurement and nothing else.
- `win-gpu.yml`'s header comment names the DX12 run, citing R-862 and R-859.

## Acceptance tests
- `cargo test -p xtask --test ci_self_hosted` — every check passes with the DX12 job in the list (REQ-TOOL-226).
- `cargo test -p xtask --features controls --test ci_self_hosted` — the negative controls: the DX12 job moved to a
  hosted workflow fails, and the DX12 job without the feature fails (R-176).
- After the merge: one nightly DX12 run on `main`, linked in a comment on the PR, reporting under its own name and in
  no required check (REQ-TOOL-226).

## Notes
- Never a required check: no job here enters `ci-gate`'s `needs`, branch protection or any gate's reading (R-711
  item 1, R-859). A red DX12 run is not main red; a Windows-only failure is handled as R-719 says.
- Starts after the anchor move, as TASK-M0-55 does.
