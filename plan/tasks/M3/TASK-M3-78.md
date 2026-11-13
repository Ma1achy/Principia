# TASK-M3-78 — Heggie's gates: the radial collision, the Hamiltonian check and the separation floor

- **Milestone:** M3
- **Closes:** REQ-VAL-045, REQ-VAL-357
- **Depends on:** TASK-M3-77
- **Needs (earlier milestones):** REQ-DEC-005, REQ-GEN-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4, PIT-4.3, PIT-4.4
- **Size:** ~1,500 lines, the fixtures and tests included

Split from TASK-M3-08, 13 Nov 2026, applied per R-369 (task scoping; the owner's standing rule to split under ~300
mutants where natural): TASK-M3-08 builds Heggie's regularised system, TASK-M3-77 its march, this task its gates.

## Goal
The two-body radial collision gate, kept as the regularisation test, reads the regularised energy (R-566) for Heggie × RK4
under Eq. 22, marched directly and registered once at `t = 0`, and for Aarseth–Zare (REQ-VAL-045), its region minimum a
calibration (REQ-VAL-357); Heggie's and Aarseth–Zare's rates are checked against a finite difference of their
Hamiltonians (`hamiltonian-fd`, REQ-VAL-170's arms, with REQ-VAL-171's proposal); and `separation-floor --propose` gains
Heggie's arm (REQ-INT-105's).

## References
- `docs/contracts/principia_integrator_contract.md` § "Heggie: the equations, the time transformations, the maps and the step control (transcribed)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `docs/contracts/principia_integrator_contract.md` § "Part 6 — Three boundaries stated honestly (deferred work lands here)"
- `decisions.md` § "R-160 — The integrator equations are transcribed into integrator_contract Part 2b"
- `decisions.md` § "R-161 ✱ — Heggie's default time transformation is the measured one, Eq. 22 at n = 3/2"
- `docs/design/principia_dd_validation_orbits.md` § "2. What the suite tests that the current gates do not"
- `decisions.md` § "R-257 — The briefs' unheld obligations: two ported, one superseded, one not standing, the kernel gates ported with values *(closes RQ-157)*"
- `decisions.md` § "R-566 — Under a regularised occupant the energy is named twice: the two-body collision gate (REQ-VAL-045) and any per-step suspect flag read the regularised energy; the Cartesian energy is for diagnostics, read only from runs that did not stop at an event *(refines R-257 as to the radial collision gate's energy)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_integrator_contract.md` § "COM projection and invariant monitoring"

## Deliverables
- `crates/validation/src/heggie.rs` — the `radial-collision` gate (REQ-VAL-045), `hamiltonian-fd` with `--propose`
  (REQ-VAL-170's and REQ-VAL-171's Heggie and Aarseth–Zare arms; TASK-M3-75 adds logH's and closes them) and
  `separation-floor`'s Heggie arm (REQ-INT-105's; TASK-M3-75 closes it), registered in the gate registry; `separation-floor`'s
  control dispatches its Heggie inputs.
- `fixtures/gates/radial-collision/` — the two-body radial collision case; `fixtures/gates/hamiltonian-fd/`; Heggie's two
  `separation-floor` inputs.
- Doc change: integrator contract Part 1 — the radial collision's measured regularised and Cartesian read-outs under each
  occupant, and which of the gate's controls bind under each.
- `crates/validation/tests/heggie_gates.rs` — the gates' verdicts, controls, refusals, flags and runner.

## Acceptance tests
- `cargo xtask gate radial-collision` — the radial collision case passes through d_min ≈ 1e-11 with bounded energy drift
  (the AZ validation reported 6.2e-15 at d_min = 1.35e-11), run for the default occupant, Heggie × RK4 under Eq. 22 as a
  direct march registered once at `t = 0`, and for AZ; the gate's values (dd_validation_orbits §2, R-257): equal masses
  from rest, third body far away, passing through `d_min < 1e-10` with `|dE/E| < 1e-12`, `E` the regularised energy under a
  regularised occupant (R-566); controls: the value the gate reads is bitwise the occupant's regularised-energy drift, and
  the Cartesian read-out of the same run after the passage is printed beside it and differs from it (the design measured
  it 280× to 7100× worse), so a gate whose reader returns the Cartesian value fails the reader check; and, the design §11's
  control, over a halving ladder of η the Cartesian read-out after the passage does not fall with η (REQ-VAL-045; physics
  review 5605806092 of PR #277, F1). *Was:* "control: … is printed and is worse than the regularised one …, so the gate
  pointed at it fails". *Was (13 Nov 2026, re-scoped):* "run for the default occupant and AZ", the default occupant then
  reached through the seam.
- `cargo xtask gate radial-collision` prints the regions it ran, the passage at each rung of the halving η ladder, and
  their count against REQ-VAL-357's minimum, proposed at 4 with that evidence in the PR (REQ-VAL-357, added 13 Nov 2026,
  applied per R-369 and R-71: the gate framework names a calibration requirement for every gate's region minimum).
- `cargo xtask gate hamiltonian-fd` — for Heggie (Eq. 20 and Eq. 22) and Aarseth–Zare, the analytic derivatives match the
  finite-differenced Hamiltonian within REQ-VAL-171's step and tolerance over random states; a sign-flipped derivative
  fails (REQ-VAL-170's Heggie and Aarseth–Zare arms; TASK-M3-75 adds logH's and closes it).
- `cargo xtask gate hamiltonian-fd --propose` — the step and tolerance with the measured discrepancy for Heggie and
  Aarseth–Zare (REQ-VAL-171's arms; TASK-M3-75 adds logH's and closes it, the human confirming at the M3 gate).
- `cargo xtask gate separation-floor --propose` — REQ-INT-105's Heggie arm: Heggie's pre-floor `dt/dτ` and `S` under
  Eq. 20 and Eq. 22, over the radial collision passage and the close-encounter fixtures, with the floored-step counts
  (every step on which the floor acts, integrator contract Part 6) and the control (a `TINY` above the smallest pre-floor
  value floors steps and is reported), beside TASK-M3-68's Aarseth–Zare evidence; logH's arm and the closing of
  REQ-INT-105 are TASK-M3-75's.

## Notes
- Split from TASK-M3-08 (13 Nov 2026, applied per R-369): these Deliverables and Acceptance tests are TASK-M3-08's
  before the split, restated. Two decisions there go to the physics reviewer: under Aarseth–Zare the "differs" control
  and the reader check do not bind, its two read-outs coinciding bitwise (possibly one-way), and prin-rs's 280× to
  7100× is not reproduced by the direct march (3.7×), recorded, not a criterion.
