# TASK-M3-29 — Free-fall brake orbits and the reversibility diagnostic

- **Milestone:** M3
- **Closes:** REQ-VAL-042, REQ-VAL-044, REQ-VAL-134, REQ-INT-032
- **Depends on:** TASK-M3-24, TASK-M3-28, TASK-M3-08
- **Needs (earlier milestones):** REQ-ENC-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~400 lines

## Goal
Two or three free-fall (brake) periodic orbits from Hristov et al. 2023 or the Three-body Gallery run as closure tests asserting a return to rest in the same configuration, with the order reported. The reversibility diagnostic exists — replaying the recorded macro-step + substep schedule in reverse, or binding a fixed-step reversible occupant with COM projection disabled — and refuses occupants whose profile has `reversible: false`. The reversibility measure `xi` is defined in dd_validation_orbits (R-72) and the brake-orbit reversibility test runs on the reversible occupant, logH's TTL time mode (R-162).

## References
- `docs/design/principia_dd_validation_orbits.md` § "3. Proposed suite"
- `docs/design/principia_dd_validation_orbits.md` § "5. Immediate action"
- `docs/design/principia_dd_validation_orbits.md` § "1.1 Free-fall periodic orbits — the best fit for this project"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 6 — Three boundaries stated honestly (deferred work lands here)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"
- `decisions.md` § "R-162 — The reversible occupant is logH's TTL time mode"
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"

## Deliverables
- `fixtures/ground_truth/free_fall/` — the chosen orbits with their published ICs and periods.
- `crates/validation/src/reversibility.rs` — the diagnostic (one of the two routes), guarded by the profile flag.
- Doc change: `docs/design/principia_dd_validation_orbits.md` §1.1 — `xi`'s formula and what a reversible occupant should read (REQ-VAL-134).
- `xtask gate reversibility-xi` over the brake orbits.

## Acceptance tests
- `cargo xtask gate free-fall-closure` — each orbit returns to rest in the same configuration after one period (tolerance: REQ-VAL-130, calibrated); order reported (REQ-VAL-042).
- `cargo xtask gate reversibility-xi` — time symmetry via `xi` on the brake orbits, run on logH-TTL (R-162) (REQ-VAL-044).
- The doc gives xi's formula and what a reversible occupant should read; physics reviewer approved (REQ-VAL-134).
- Review (physics): the reversibility diagnostic refuses an occupant with reversible = false; the implementation uses one of the two named routes (REQ-INT-032).

## Notes
- *Was: "Gap: the only `reversible: true` occupant in the Part 2a table is AZ + time-transformed leapfrog (Mikkola–Tanikawa), whose equations the corpus doesn't give and whose build in M3 isn't decided; without it REQ-VAL-044's gate cannot run ('blocked on a reversible occupant')."* RQ-102 ruled: R-162 — a reversible occupant exists: logH's TTL time mode, built in TASK-M3-08; Aarseth–Zare with Mikkola–Tanikawa isn't required.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-VAL-134.
