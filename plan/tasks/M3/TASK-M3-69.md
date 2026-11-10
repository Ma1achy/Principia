# TASK-M3-69 — The FTLE field's chart independence (R-924)

- **Milestone:** M3
- **Closes:** REQ-CHART-132
- **Depends on:** TASK-M3-14, TASK-M2-71
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines, the test included

## Goal
FTLE and the Lyapunov exponent are computed in physical phase space, never in chart coordinates, so a chart's warp can't change a system's exponent: two charts that decode one state give it one `λ_T`, read on copy 0. The Benettin shadow's `δ₀` (TASK-M3-14; its direction fixed by dd_integrator § 3.8, REQ-INT-077) is a function of the state, never of the chart. Split from TASK-M3-14 by R-924, since the test needs TASK-M2-71's warped chart; it waits for M2's gate as R-918 has it.

## References
- `docs/design/principia_chart_composition.md` § "6.5 Chaos measures live in phase space"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `docs/design/principia_dd_integrator.md` § "3.8 Co-computations (tier-gated, ride the forward pass)"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `decisions.md` § "R-918 — R-917's follow-ups: TASK-M3-14 (the Benettin FTLE shadow) stops and is re-queued after M2's gate; M2's maths that seems to need the integrator is tested against exactly known trajectories through a trajectory interface the real integrator plugs into after M2, flagging only what can't be tested so; CI work outside the order, TASK-M2-38 and TASK-M2-56 finishing, and the gate checklist first are agreed *(no RQ)*"
- `decisions.md` § "R-924 — TASK-M3-14 (the Benettin FTLE shadow) resumes now; its chart-independence arm (REQ-CHART-132), which needs TASK-M2-71's warped chart, moves to a new task that waits on TASK-M2-71 *(amends R-918 in part)*"

## Deliverables
- The `ftle_chart_independent` test in `crates/engine`, with its control (R-176).

## Acceptance tests
- `cargo test -p engine ftle_chart_independent` — a linear chart, a warped one at a pixel where the warp's value is exact, and a third with a different tangent through the same state, a tilted pair, decode one bit pattern and give one lambda_T bit for bit, read on copy 0; control: delta_0's direction taken from a chart tangent fails (REQ-CHART-132).

## Notes
- New with R-924 (10 Nov 2026), applied per R-369: REQ-CHART-132 and its acceptance arm, unchanged, moved here from TASK-M3-14, which resumes now; before R-918 they were TASK-M8-100's. Waits for TASK-M2-71's warps and M2's gate (R-917, R-918).
