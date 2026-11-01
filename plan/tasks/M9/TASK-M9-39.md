# TASK-M9-39 — The softened force law: Plummer softening, built after the core build, and the refusal of ε ≠ 0 lifted

- **Milestone:** M9
- **Closes:** REQ-INT-110, REQ-SYS-094
- **Depends on:** TASK-M3-01, TASK-M3-05, TASK-M3-06
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-9
- **Size:** ~300 lines

## Goal
`epsilon` is carried, tagged and refused (TASK-M3-06); this task gives it its force law (R-654): `V = −Σ mᵢmⱼ/√(r² + ε²)`, the force its negative gradient, with `ε` canonical, a fraction of the initial hyperradius `R` fixed at `t = 0` and never co-moving, recorded in every output header like `r_coll`, and energy and drift defined under the softened potential. Scheduled late, after the core build, at the owner's word; it waits on RQ-656, and starts only once that entry is ruled.

## References
- `decisions.md` § "R-654 — The softened force law is Plummer softening, `V = −Σ mᵢmⱼ/√(r² + ε²)`, with `ε` canonical, a fraction of the initial hyperradius `R` fixed at `t = 0` and never co-moving, recorded in every output header like `r_coll`; energy and drift are defined under the softened potential; its task is scheduled late, after the core build, and runs with `ε ≠ 0` stay refused until it lands *(closes RQ-627; queues RQ-656)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/design/principia_dd_generation_root.md` § "Open"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/kernel/src/physics/pairs.rs`, `force.rs` and `energy.rs` — the softened force and potential, one `r² + ε²` shared by both, for `f32` and `f64`.
- The canonical `ε` fixed at `t = 0` from the initial `R`; no co-moving option exists.
- `crates/engine/src/dispatch/validate.rs` — the refusal of `ε ≠ 0` lifted for finite `ε` in the range this task defines (R-72; physics reviewer approves); NaN, ±∞ and negative `ε` stay refused.
- `ε` in every output header beside `r_coll`.
- What RQ-656's ruling requires of the collision test, the regularised occupants and the Chazy classifier.

## Acceptance tests
- `cargo test -p kernel softened_force` — the force equals the central difference of V at f64, for ε = 0, a small ε and an ε above the pair distance; ε = 0 is bit-identical to the Newtonian force and energy; ε scales with the initial R and does not change as R does along a run; a softened leapfrog's drift under the softened V converges with step size; control: E read with the Newtonian V at ε ≠ 0 fails that drift bound (REQ-INT-110).
- `cargo test -p engine softened_dispatch` — every output header of a run carries ε beside r_coll, equal to the dispatched value's bits; an ε > 0 run is tagged and refused pooling with ε = 0; finite ε at each end of the range dispatches, and just outside it, NaN and ±∞ are refused with the typed reason; control: a header writer dropping ε fails (REQ-SYS-094).

## Notes
- Added by R-654 (30 Oct 2026; the port, 1 Nov 2026). A build task, not a target: it is not scored, its tests block its own merge, and TASK-M9-37, the report, does not wait on it. In M9 because R-650 and `plan/MILESTONES.md` call M0 to M8 the core build and no other milestone follows it (applied per R-369).
- Definitions written here (R-72; physics reviewer approves before merge): the range of `ε`.
- Scheduled at the owner's word; R-415's early-merge test does not bring it forward.
