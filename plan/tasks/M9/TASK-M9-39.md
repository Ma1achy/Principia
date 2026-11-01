# TASK-M9-39 — The softened force law: Plummer softening, built after the core build, and the refusal of ε ≠ 0 lifted

- **Milestone:** M9
- **Closes:** REQ-INT-110, REQ-SYS-094, REQ-SYS-095
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
- `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time, pre-integration)"
- `docs/design/principia_dd_decoder.md` § "3.7 Energy normalisation (optional; keep-or-drop is decision B9)"
- `docs/design/principia_chart_reference.md` § "0.6 Energy normalisation — optional, and forbidden on some charts"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/kernel/src/physics/pairs.rs`, `force.rs` and `energy.rs` — the softened force and potential, one `r² + ε²` shared by both, for `f32` and `f64`.
- The canonical `ε`, the sim-key value, read as given: `R₀ = √(I₀/M) = 1` in the gauge, so it is the fraction and the length, and it is never recomputed from the state; no co-moving option exists.
- `crates/engine/src/dispatch/validate.rs` — the refusal of `ε ≠ 0` lifted for finite `ε` in the range REQ-SYS-095 calibrates; NaN, ±∞ and negative `ε` stay refused.
- `ε` in every output header beside `r_coll`.
- The range of `ε`'s calibration proposal, its ends with their evidence (R-71).
- What RQ-656's ruling requires of the collision test, the regularised occupants, the Chazy classifier, `E_rel` and the ionisation gate, the decode's `V₀` with `virial_ratio` and the collision pole (REQ-VAL-027, REQ-COL-006, the fragment decode included), `η_E`'s `U` and its outcomes, and the substep law.

## Acceptance tests
- `cargo test -p kernel softened_force` — the force equals the central difference of V at f64, for ε = 0, a small ε and an ε above the pair distance; ε = 0 is bit-identical to the Newtonian force and energy; ε is bitwise the sim-key value at every step and on every sample, along a run in which R changes; a softened leapfrog's drift under the softened V converges with step size; at ε ≠ 0 the decode stage's E₀ agrees with the kernel's E₀ under the softened V, the descriptor's V₀ being in the form RQ-656's ruling gives; controls: E read with the Newtonian V at ε ≠ 0 fails that drift bound, an E₀ read with the Newtonian V at ε ≠ 0 fails that agreement, and an ε recomputed from the state or from R(t) fails the bitwise check (REQ-INT-110).
- `cargo test -p engine softened_dispatch` — every output header of a run carries ε beside r_coll, equal to the dispatched value's bits; an ε > 0 run is tagged and refused pooling with ε = 0; finite ε at each end of the range dispatches, and just outside it, NaN and ±∞ are refused with the typed reason; control: a header writer dropping ε fails (REQ-SYS-094).
- Review checklist (physics reviewer) — the range of `ε`: the proposal measures what sets each end and states it, with its evidence; reviewer-checked, confirmed by the human and recorded in `decisions.md` (REQ-SYS-095).

## Notes
- Added by R-654 (30 Oct 2026; the port, 1 Nov 2026). A build task, not a target: it is not scored, its tests block its own merge, and TASK-M9-37, the report, does not wait on it. In M9 because R-650 and `plan/MILESTONES.md` call M0 to M8 the core build and no other milestone follows it (applied per R-369).
- Calibrations proposed here (R-71; human confirmation with the M9 report if this task has landed by then, otherwise in the next batch after it lands, then recorded in decisions.md): REQ-SYS-095, the range of `ε`. *Was:* "Definitions written here (R-72; physics reviewer approves before merge): the range of `ε`." (code review 5730587922 of PR #375, C4; applied per R-369).
- Scheduled at the owner's word; R-415's early-merge test does not bring it forward.
