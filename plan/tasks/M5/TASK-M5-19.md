# TASK-M5-19 — roundtrip_error and the per-footprint temporal accumulators

- **Milestone:** M5
- **Closes:** REQ-PAY-078, REQ-PAY-079, REQ-PAY-065
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

## Deliverables
- `docs/design/principia_dd_generation_root.md` §3.7: when `roundtrip_error` runs and how it reaches the reduction
  (REQ-PAY-079), with the "Removed lines" note.
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

## Notes
- Open RQs: RQ-72 (from the closed requirements' `rq:`).
- Calibrations (R-71) this task proposes: REQ-PAY-078.
- Definitions (R-72) this task writes: REQ-PAY-079.
- **Waits on RQ-72** (REQ-PAY-065): whether a per-footprint latched bit or count travels in `QuadReduction`, or the
  latch is evaluated on the GPU and only its verdict returns. The storage layout of the latch is this task's R-72
  definition once RQ-72 is ruled.
- REQ-PAY-078 is an R-71 calibration: proposed value and evidence in the PR; the human confirms at the M5 gate.
- The horizon measurement must reuse the original discretisation (pitfalls §3, REQ-TOOL-038).
