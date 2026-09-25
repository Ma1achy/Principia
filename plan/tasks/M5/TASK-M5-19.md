# TASK-M5-19 — roundtrip_error and the per-footprint temporal accumulators

- **Milestone:** M5
- **Closes:** REQ-PAY-078, REQ-PAY-079, REQ-PAY-065, REQ-REF-045
- **Depends on:** TASK-M5-08, TASK-M5-18, TASK-M3-23
- **Needs (earlier milestones):** REQ-PAY-057, REQ-SCHED-004, REQ-TOOL-038
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-10
- **Size:** ~380 lines

## Goal
`roundtrip_error` is computed and reaches the reduction: when it runs (which quads, at which point of the march)
is written into §3.7 (REQ-PAY-079), and its time-reversal horizon is proposed as a calibration with its measured cost
(REQ-PAY-078). The temporal accumulators are fixed-size and updated in place on the GPU each step: the latching running
max divergence per footprint, held with the resident quad (not a `QuadReduction` member) and evicted with it, feeding
"unresolved" under the same eps; the running mean divergence and the write-once first-divergence time as diagnostics
that the split ignores; no divergence-trend member.

## References
- `docs/design/principia_dd_generation_root.md` § "Ensemble spread — the two bounded contributors"
- `docs/contracts/principia_scheduler_contract.md` § "Part 8 — Continuous refinement & the live-to-live handoff"
- `docs/design/principia_dd_generation_root.md` § "Temporal accumulators (scheduler Part 8)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-91 — The temporal accumulators feed "unresolved" *(closes RQ-42)*"
- `decisions.md` § "R-99 — The latch is per footprint and lives with the resident quad *(closes RQ-59)*"
- `decisions.md` § "R-142 — The latch is evaluated on the GPU; only its verdict returns *(closes RQ-72)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `docs/design/principia_dd_generation_root.md` §3.7: when `roundtrip_error` runs and how it reaches the reduction
  (REQ-PAY-079), with the "Removed lines" note.
- `docs/design/principia_dd_generation_root.md` § "Temporal accumulators (scheduler Part 8)": the per-footprint latch's storage layout with the resident quad and its drop on evict and merge (REQ-REF-045, moved here from TASK-M6-03 by R-113).
- `crates/kernel/src/reduce/{roundtrip.rs, temporal.rs}`; `crates/engine/src/cache/latch.rs` (per-footprint latch storage
  with the resident quad).
- Tests `crates/kernel/tests/temporal_accumulators.rs`.
- The calibration proposal: `roundtrip_error` against horizon on the measured phase-error cases (RC §7.19f) and the
  per-quad cost at the chosen horizon (`cargo xtask bench roundtrip-horizon`).

## Acceptance tests
- Review checklist (physics) — the task shows roundtrip_error against horizon on measured phase-error cases (RC §7.19f) and the per-quad cost at the chosen horizon; the proposed value is marked pending and the human confirms it at the M5 gate, then it is recorded in `decisions.md` (REQ-PAY-078).
- `cargo xtask bench roundtrip-horizon` — produces the evidence for the proposal: the task shows roundtrip_error against horizon on measured phase-error cases (RC §7.19f) and the per-quad cost at the chosen horizon (REQ-PAY-078).
- Review checklist (physics) — the ledger row states when the time-reversal round trip is computed and how it reaches the reduction; the doc change is in this PR and the physics reviewer approves it before merge (REQ-PAY-079).
- `cargo test -p kernel temporal_accumulators` — accumulator size is constant in t; QuadReduction has no running_max_divergence or divergence_trend member; running max is monotone per footprint; first-divergence time never changes after first write; a footprint whose running max once exceeded eps still reads unresolved after its spread falls back below eps; the split's output ignores the diagnostics (REQ-PAY-065).
- Review checklist (physics) — the section gives the latch's layout per footprint with the resident quad, and shows it is dropped on evict and merge; the doc change is in this PR and the physics reviewer approves it before merge (REQ-REF-045).

## Notes
- Calibrations (R-71) this task proposes: REQ-PAY-078.
- Definitions (R-72) this task writes: REQ-PAY-079, REQ-REF-045.
- RQ-72 ruled: R-142 — the latch is evaluated on the GPU in the resolve pass, its state in GPU-resident per-quad
  memory; `QuadReduction` carries only the verdict, the count of unresolved footprints, latched ones included. The
  latch's storage layout in that memory is this task's R-72 definition (REQ-REF-045).
- REQ-PAY-078 is an R-71 calibration: proposed value and evidence in the PR; the human confirms at the M5 gate.
- The horizon measurement must reuse the original discretisation (pitfalls §3, REQ-TOOL-038).
- RQ-100 ruled: R-113 — REQ-REF-045 (the latch's layout) moves to M5 and is written here, where the latch is built.
