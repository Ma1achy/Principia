# TASK-M3-77 — Heggie's direct march, registered once at t = 0, and its step control

- **Milestone:** M3
- **Closes:** REQ-INT-122
- **Depends on:** TASK-M3-08
- **Needs (earlier milestones):** REQ-DEC-005, REQ-GEN-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4, PIT-4.3, PIT-4.4
- **Size:** ~1,600 lines, the tests and the landing's definition included (*was:* ~1,400, before R-1007)

Split from TASK-M3-08, 13 Nov 2026, applied per R-369 (task scoping; the owner's standing rule to split under ~300
mutants where natural): TASK-M3-08 builds Heggie's regularised system, this task its march, TASK-M3-78 its gates.

## Goal
Heggie's direct march is built to integrator contract Part 2b's step control: registered once at `t = 0` and carried
across every sync boundary of R-565's grid, landing on each at `k·Δ_sync`, with every stepper through its form on Heggie's
regularised system (R-561), the explicit finiteness exit and R-604's budget ending the march latched, and Part 6's floor
signal and entry-cap hits counted, landing under R-998's rule as R-1007 applies it, its definition written in Part 2b;
and dd test 3 is measured on Heggie × RK4 and Aarseth–Zare + RK4 (REQ-INT-122).

## References
- `docs/contracts/principia_integrator_contract.md` § "Heggie: the equations, the time transformations, the maps and the step control (transcribed)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `docs/contracts/principia_integrator_contract.md` § "Part 6 — Three boundaries stated honestly (deferred work lands here)"
- `decisions.md` § "R-160 — The integrator equations are transcribed into integrator_contract Part 2b"
- `decisions.md` § "R-161 ✱ — Heggie's default time transformation is the measured one, Eq. 22 at n = 3/2"
- `docs/contracts/principia_integrator_contract.md` § "The sync grid is a fixed quantum of physical time (R-565)"
- `docs/contracts/principia_integrator_contract.md` § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)"
- `docs/contracts/principia_integrator_contract.md` § "The time-reversal maps (REQ-INT-095)"
- `decisions.md` § "R-604 — A regularised copy's step budget is checked every step: its running total against `max(N_min, rate · t_elapsed)`; `N_min` protects early close encounters and is an M3 calibration *(closes RQ-527; settles R-564's accounting)*"
- `decisions.md` § "R-561 — Every stepper × regularisation pair is allowed, regularisation off included; a pair measured not to work carries a warning beside the selector, in the Problems panel and in each export's settings, and is still defined exactly so its results reproduce; a table marks each pair measured good, measured bad or not measured"
- `decisions.md` § "R-500 — The capability profile's `reversible` is two flags: `step_reversible`, true for KDK, Yoshida-4, Yoshida-6 and the TTL leapfrog, false for Euler, RK4 and Aarseth–Zare + RK4, the regularised occupants measured; and `run_reversible`, logH-TTL only; both diagnostic routes gate on `step_reversible`, and a negative weight does not break a symmetric step *(closes RQ-369; refines R-162; amends R-57 as to integrator contract Part 6's sentence)*"
- `decisions.md` § "R-998 — RQ-795, Aarseth–Zare's landing: option (b), no landing tolerance, on the owner's condition that the state lands on the boundary together with the clock, the final step sized or retaken to land, never the clock corrected alone while the state stays overshot; checked by the figure-eight closure's convergence order, not only its closure value *(closes RQ-795)*"
- `decisions.md` § "R-1007 — RQ-812, Heggie's landing: option (a), one landing rule for all three methods; Heggie's occupant lands under R-998's rule, with no landing tolerance, the state landing on the boundary with the clock, the final step sized or retaken, checked by the figure-eight closure's convergence order *(closes RQ-812; extends R-998 and R-994)*"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"

## Deliverables
- `crates/kernel/src/regularisation/heggie.rs` — the stepper slot under Heggie (`HgStepper`, every stepper through its
  form, each pair's profile its form's), the step control (the per-step interval rule, the predictive limit on Eq. 7's
  relative velocities, the landing clamp and secant, the explicit finiteness exit and R-604's budget, Part 6's floor
  signal and entry-cap hits counted, each step's physical increment fed to `dt_max`) and the march, registered once at
  `t = 0` and carried across every boundary of R-565's grid. The landing follows R-998's rule as R-1007 applies it: no
  landing tolerance in any branch (Part 2c rule 1), the state landing on the boundary together with the clock, the final
  step sized or retaken, never the clock set to the boundary alone while the state stays overshot, and no constant but an
  integer iteration cap in a branch. Heggie's seam `ADVANCE` is TASK-M3-75's. *Was (13 Nov 2026, until R-1007):* "The
  landing tolerance is a caller-supplied setting with no value in the kernel until RQ-812 is ruled (applied per R-369, as
  #502 did for Aarseth–Zare before RQ-795), each site marked RQ-812."
- Doc change, `docs/contracts/principia_integrator_contract.md` § "Heggie: the equations, the time transformations, the
  maps and the step control (transcribed)": how Heggie's last step is sized or retaken, a definition (R-72) within
  R-998's limits, physics-reviewed (R-1007).
- `crates/kernel/tests/heggie.rs` — the tests below.

## Acceptance tests
- `cargo test -p kernel heggie_pairs` — Heggie × every stepper (Euler, KDK, Yoshida-4, Yoshida-6, RK4) under Eq. 20 and
  Eq. 22 marches a fixture state across a sync boundary as a direct march, its state finite and its profile its form's;
  Heggie's time transformation defaults to Eq. 22, Eq. 20 selectable (R-161); controls: a march whose stepper takes no step
  fails, and a default of Eq. 20 fails (REQ-INT-122; REQ-INT-006's Heggie part, which TASK-M3-75 closes with the seam's pairs).
- `cargo test -p kernel heggie_march` — the transcription's own checks: the Cartesian → regularised → Cartesian round trip
  at unequal masses, the regularised energy against the Cartesian energy at registration, `Γ* = (E_reg − h)·R₁R₂R₃`, the
  march registered once (its frozen `h` unchanged across boundaries), landing on each boundary at `k·Δ_sync`, the
  explicit finiteness exit and R-604's budget ending the march latched (`End::NonFinite`, `End::Exhausted`), the floor
  signal on `dt/dτ < TINY`, `S < TINY` or `Rᵢ < TINY`, and the entry-cap hit on `c₀ > c₁`, each with its control (REQ-INT-122).
- `cargo test -p kernel heggie_landing` — Heggie × RK4's figure-eight closure and its convergence order across η in
  [0.02, 0.001] under the landing above (R-1007; REQ-INT-084's Heggie part, which TASK-M3-75 closes): the criterion is
  written before any run and states its round-off floor beforehand, the order is fitted only over step sizes above that
  floor, and an order still short of 4 there, by a threshold the criterion states beforehand, is read as a landing
  defect, not round-off; physics approves the criterion before it runs (applied per R-369, physics review 5890410670 of
  #520, F1); controls: a march that lands the clock alone, its state left overshot, fails, and a stored state whose
  clock differs from the boundary it reports fails (REQ-INT-122).
- `cargo test -p kernel regularised_step_reversibility` — dd test 3 run on Heggie × RK4 (Eq. 20 and Eq. 22) and
  Aarseth–Zare + RK4 through each one's time-reversal map, the residual and its norm taken on the full extended state, at
  REQ-VAL-191's criterion: each residual and verdict printed; fails if one is missing or its profile's `step_reversible`
  differs from its measured verdict; a verdict other than R-500's listed value (Aarseth–Zare + RK4 false) is reported for
  the review queue; controls: the measurement at the same `dt` on RK4 and on REQ-VAL-032's asymmetric Yoshida-4 reads not
  reversible, a profile with one regularised occupant's flag flipped fails, and a measurement set with one built occupant
  left out fails (REQ-VAL-190's Heggie and Aarseth–Zare arms; TASK-M3-75 adds logH's time modes and closes it).

## Notes
- Split from TASK-M3-08 (13 Nov 2026, applied per R-369): these Deliverables and Acceptance tests are TASK-M3-08's
  before the split, restated; `heggie_march`'s checks that need no march (the round trip, `Γ*`, the rates) stay with
  TASK-M3-08's `heggie_system`.
- *13 Nov 2026 (R-1007, RQ-812 (a); applied per R-369, a mechanical consequence of the ruling and the split):* R-1007's
  "this task's step-control part" (TASK-M3-08's note) is this task's: the landing above, its Part 2b definition and the
  convergence-order check. *Was:* "RQ-812 (Heggie's landing, open): the final landing follows its ruling, through a
  task."
- *13 Nov 2026 (code review 5895563626 C2 of #522, applied per R-369, scope):* this task owes the R-604 budget helper
  for any regularised occupant (`over_allowance(steps, t, rate, n_min)` in `az.rs`, `over_budget` routed through it
  bit-identically), added with Heggie's march as its first caller; TASK-M3-08 dropped it.
