# TASK-M3-74 — The FTLE shadow's joint step on the default occupant, Heggie × RK4, at the sample's physical time (REQ-VAL-355)

- **Milestone:** M3
- **Closes:** REQ-VAL-355
- **Depends on:** TASK-M3-08, TASK-M3-72
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines, the tests included

## Goal
R-974 (RQ-793 (a)) gives every occupant a joint step that marches the FTLE shadow's `(x, δ⃗)` together, `x`
bit-identical to the occupant's own step, and names the default regularised occupant among them (integrator contract
Part 2, Part 2b, R-560). TASK-M3-72 builds the joint step for KDK, Yoshida-4, Yoshida-6 and RK4, with the (a1)/(a2)
force difference its measurement chooses, and writes into `principia_dd_integrator.md` § 3.8 how the joint step keeps
the sample and its shadow at the same physical time under any occupant that owns its time mapping (R-72). This task
builds the joint step for Heggie × RK4, the default occupant (`dτ = S^{3/2}/(R₁R₂R₃) dt`, Part 2b, R-161), which
TASK-M3-08 builds, by that definition: its `x` bit-identical to Heggie × RK4's `ADVANCE`, its `δ⃗` through the same
stages, the sample and its shadow at the same physical time at every renormalisation as § 3.8 states.

## References
- `docs/design/principia_dd_integrator.md` § "3.8 Co-computations (tier-gated, ride the forward pass)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-974 — RQ-793, the displacement form of the FTLE shadow: option (a), a joint step on the occupant interface; whether the force difference is formed plainly in f32 (a1) or cancellation-free (a2) is chosen by a measurement whose criterion TASK-M3-72 writes before any run, physics-reviewed *(closes RQ-793)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- `crates/kernel`: the joint step of the FTLE shadow's displacement form for Heggie × RK4, `x` bit-identical to its
  `ADVANCE`, `δ⃗` advanced through the same stages with the force difference TASK-M3-72's measurement chose, the sample
  and its shadow kept at the same physical time as § 3.8 defines it.
- `cargo test -p kernel displacement_shadow_same_time`, with its control (R-176).

## Acceptance tests
- `cargo test -p kernel displacement_shadow_same_time` — on the Heggie × RK4 occupant, at every renormalisation of a
  joint-step run, the shadow's elapsed physical time equals the sample's as the definition states; control: a joint
  step that advances the shadow by the sample's `Δτ` alone, its elapsed `t` drifting from the sample's, fails
  (REQ-VAL-355).
- Review (physics): the joint step keeps the sample and its shadow at the same physical time as § 3.8 states, on
  Heggie × RK4 (REQ-VAL-355).

## Notes
- New 13 Nov 2026, applied per R-369 (task scoping, a two-way door; the orchestrator): split from TASK-M3-72, which
  stopped because the Heggie × RK4 joint step needs TASK-M3-08's occupant and TASK-M3-72 does not depend on it.
  REQ-VAL-355 moves here with its verify; its definition stays TASK-M3-72's, in § 3.8.
