# TASK-M0-55 — The Windows runner's named measurements on `main` (R-859)

- **Milestone:** M0
- **Closes:** REQ-TOOL-224
- **Depends on:** TASK-M0-04
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The owner's ruling of 8 Nov 2026 (R-859) says what R-711's runner, the owner's PC, may run from the anchor move: the
GPU suites nightly and on pushes to `main` (Vulkan now, DX12 from Phase 2), gpu-idle and the weekly proof, each a named
measurement and never a required check. The list is not exhaustive (R-860): `win-gpu.yml`'s nightly Windows build and
tests, `win-tests`, stays a named measurement too. `win-gpu.yml` already runs the Vulkan suites, `win-tests` and the
nightly self-check, and `runner-proof.yml` the weekly proof; gpu-idle so far runs only from a test branch, which goes
after the anchor work.
This task gives gpu-idle its place on `main`, makes the guard test pin the runner's jobs to the ruled list, and shows
each one reporting under its own name.

## References
- `decisions.md` § "R-859 — From the anchor move, the Windows runner may run, as named measurements and never as required checks: the GPU suites (Vulkan now, DX12 from Phase 2) nightly and on pushes to `main`, gpu-idle, the weekly proof and the 7 Nov heavy-work comparison; benchmarks of record stay on the Mac, Windows numbers a separate named series; moving required or heavy CI work to the PC is not approved and returns through `/rulings-report`; the runner's security rules are unchanged *(refines R-186, R-656 and R-711)*"
- `decisions.md` § "R-860 — R-859's list of what the Windows runner may run is not exhaustive: the nightly `win-tests` job and the Windows arms of the path-cover and time-column benchmarks stay as named measurements, never required checks *(closes RQ-742; refines R-859)*"
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"

## Deliverables
- gpu-idle on `main`: a job, `workflow_dispatch` only, whose one step runs the PC's read-only GPU sample script, in
  `runner-proof.yml` or a workflow of its own, keeping every rule the guard test holds for the runner's jobs (the
  repository and actor guard, no credential, pinned actions if any, the self-check first through the PC's hook), and
  pinned to its one step as `check_proof` pins the weekly proof.
- `xtask/tests/ci_self_hosted.rs`: `self_hosted_jobs_are_named_measurements`, the self-hosted jobs across every
  workflow being exactly the ruled list, each by name, with its controls: the Vulkan GPU suites, `win-tests`, the
  nightly self-check, gpu-idle and the weekly proof (R-859, R-860); the module comment and `const JOBS` follow.
  Any qa file that models the same list changes in this task's qa commit (R-290).
- `win-gpu.yml`'s and `runner-proof.yml`'s header comments cite R-859 beside R-711.
- `win-tests` stays in `win-gpu.yml`, a nightly named measurement (R-860).

## Acceptance tests
- `cargo test -p xtask --test ci_self_hosted` — every check, `self_hosted_jobs_are_named_measurements` among them,
  passes on the workflows as committed (REQ-TOOL-224).
- `cargo test -p xtask --features controls --test ci_self_hosted` — the negative controls: an unlisted self-hosted job
  fails, a listed job removed fails, and gpu-idle under a push trigger fails (R-176).
- After the anchor move and the merge: a nightly `win-gpu` run on `main` and one dispatched gpu-idle run on `main`,
  each linked in a comment on the PR, each reporting under its own name and neither in any required check (REQ-TOOL-224).

## Notes
- Starts after the anchor move, the owner's own act; until it the PC's hook refuses `main`'s merges, so nothing here
  can be shown running.
- DX12's job is not in this task: it joins the measurement workflow once the DX12 opt-in lands, in TASK-M0-58 (R-711
  item 2, R-859 "DX12 from Phase 2", R-862).
- Never a required check: no job here may enter `ci-gate`'s `needs`, branch protection or any gate's reading
  (R-711 item 1, R-859). A red job here is not main red.
- gpu-idle is dispatched from the Mac only when the owner says they are signing out of the PC, on an idle runner,
  with nothing alongside it.
