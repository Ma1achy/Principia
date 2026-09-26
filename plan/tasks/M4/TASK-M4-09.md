# TASK-M4-09 — Bounded dispatch: chunked marches that return before a watchdog

- **Milestone:** M4
- **Closes:** REQ-PERF-010, REQ-PERF-081, REQ-INT-059
- **Depends on:** TASK-M4-05, TASK-M4-02, TASK-M3-20
- **Needs (earlier milestones):** REQ-SCHED-002, REQ-INT-007, REQ-INT-038, REQ-PERF-003, REQ-TOOL-001, REQ-EVT-010
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
"Bound the dispatch, not the work." A long march is split into dispatch chunks small enough to return before a driver watchdog, carrying state between them, with results identical to one long dispatch; the same chunking mechanism drives CPU-side yielding. The integration loops exit through the `done` flag set by `detect_terminal` on collision or escape, with zero `break` — proven by a 100-macro-step single dispatch matching the CPU's branch words. The chunk bound itself is an R-71 calibration.

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "6.3 Device loss, timeouts, driver resets"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "GPU"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "CPU"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"
- `decisions.md` § "R-103 — Escape ends the production loop; the §2.4 checks run in the harness *(closes RQ-63 and RQ-69)*"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"

## Deliverables
- `crates/engine/src/chunking.rs`: chunk planner (macro-steps per dispatch) used by the GPU march and the CPU rayon path alike.
- `crates/validation/tests/loop_shape.rs`: the 100-macro-step single-dispatch parity test.
- `xtask` bench `dispatch-chunk`: per-chunk duration measurements on the human's own Mac (Metal) via `prin profile`, never on a hosted runner (R-186); the proposed bound with its evidence in the PR.

## Acceptance tests
- `cargo test -p engine chunked_march` — a long march run as multiple bounded dispatches gives results identical to one long dispatch; the CPU path yields through the same mechanism (REQ-PERF-010).
- `cargo test -p kernel discipline_lint` (no `break` in march loops, no `for s in 0..N_sub`) plus `cargo test -p validation hundred_steps_one_dispatch` — a GPU dispatch of 100 macro-steps in one dispatch through SPIR-V → MSL (native `wgpu`) and the same dispatch on lavapipe (R-110) match the CPU branch words (REQ-INT-059).
- `cargo xtask bench dispatch-chunk` — decisions.md records the bound with its evidence: measured per-chunk durations against the target backends' watchdog limits; checked by the perf reviewer and confirmed by the human at the M4 gate (REQ-PERF-081, calibrated).

## Notes
- REQ-PERF-081 is a calibration (R-71): the PR carries the proposed value, its evidence and the reviewer's check, marked pending; the human confirms it at the M4 gate and it is then recorded in decisions.md.
- REQ-PERF-081 is a calibration (R-71): the PR marks the bound pending until the human confirms it at the M4 gate.
- RQ-97 ruled: R-113 — REQ-INT-059 is the native half (Metal and lavapipe); the SPIR-V → WGSL → browser-compiler leg is REQ-VAL-144, closed with REQ-VAL-116 at M8 (TASK-M8-40). This settles the gap.
