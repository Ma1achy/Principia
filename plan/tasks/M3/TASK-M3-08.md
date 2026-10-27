# TASK-M3-08 — Heggie and logH occupants, and the visible re-registration count

- **Milestone:** M3
- **Closes:** REQ-INT-006, REQ-INT-018, REQ-INT-073, REQ-VAL-045, REQ-INT-084, REQ-VAL-170, REQ-VAL-171, REQ-VAL-190, REQ-INT-095, REQ-INT-104, REQ-INT-105
- **Depends on:** TASK-M3-07
- **Needs (earlier milestones):** REQ-DEC-005, REQ-GEN-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4, PIT-4.3, PIT-4.4
- **Size:** ~520 lines

## Goal
The regularisation slot is complete: Heggie 1974 global regularisation (three relative vectors, no reference body, never re-registers), stepped with RK4, is the default occupant, Heggie × RK4 (R-560), and logH (time transformation only) is the no-chart arm. The re-registration count is visible downstream, and integrator contract Part 2b defines where it lives — a `re_registrations` profile field, a per-trajectory payload count, or both — with any payload field added to the ledger as a schema change. The two-body radial collision gate is kept as the regularisation test.

## References
- `docs/contracts/principia_canonical_spec.md` § "8. Locked vocabulary"
- `docs/design/principia_dd_predictability_horizon.md` § "The predictability horizon"
- `docs/contracts/principia_integrator_contract.md` § "The profile gains a field"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_validation_orbits.md` § "4. What the suite cannot do"
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `decisions.md` § "R-159 — The prin-rs reference set is imported *(closes RQ-102 and RQ-103, with R-160 to R-167)*"
- `decisions.md` § "R-160 — The integrator equations are transcribed into integrator_contract Part 2b"
- `decisions.md` § "R-161 ✱ — Heggie's default time transformation is the measured one, Eq. 22 at n = 3/2"
- `decisions.md` § "R-162 — The reversible occupant is logH's TTL time mode"
- `decisions.md` § "R-500 — The capability profile's `reversible` is two flags: `step_reversible`, true for KDK, Yoshida-4, Yoshida-6 and the TTL leapfrog, false for Euler, RK4 and Aarseth–Zare + RK4, the regularised occupants measured; and `run_reversible`, logH-TTL only; both diagnostic routes gate on `step_reversible`, and a negative weight does not break a symmetric step *(closes RQ-369; refines R-162; amends R-57 as to integrator contract Part 6's sentence)*"
- `docs/design/principia_dd_validation_orbits.md` § "2. What the suite tests that the current gates do not"
- `decisions.md` § "R-257 — The briefs' unheld obligations: two ported, one superseded, one not standing, the kernel gates ported with values *(closes RQ-157)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

- `docs/contracts/principia_integrator_contract.md` § "COM projection and invariant monitoring"
- `docs/contracts/principia_integrator_contract.md` § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)"
- `decisions.md` § "R-560 — The default occupant is Heggie × RK4, the combination every Heggie result was measured with; REQ-GUI-102's default and the Run window change with it"
- `decisions.md` § "R-561 — Every stepper × regularisation pair is allowed, regularisation off included; a pair measured not to work carries a warning beside the selector, in the Problems panel and in each export's settings, and is still defined exactly so its results reproduce; a table marks each pair measured good, measured bad or not measured"
- `decisions.md` § "R-566 — Under a regularised occupant the energy is named twice: the two-body collision gate (REQ-VAL-045) and any per-step suspect flag read the regularised energy; the Cartesian energy is for diagnostics, read only from runs that did not stop at an event *(refines R-257 as to the radial collision gate's energy)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 6 — Three boundaries stated honestly (deferred work lands here)"
- `decisions.md` § "R-563 — The integration floor follows the occupant: with regularisation off the substep machinery is unchanged; under a regularised occupant running out of the step budget makes the result undetermined, never discarded, and the separation floor and the entry-cap hits are counted, non-terminal signals; REQ-SCHED-063 and REQ-SCHED-077 are re-pointed"
- `decisions.md` § "R-606 — The default occupant is Heggie × RK4 at every tier: tiers trade resolution and samples, never the integrator; the tier column is the per-stepper binding with regularisation `none`; revisited as RQ-529 (c) once more pairs are measured *(closes RQ-529)*"

## Deliverables
- `crates/kernel/src/regularisation/{heggie,logh}.rs` — transcribed physics layers; logH carries its TTL time mode, the reversible occupant (R-162), its profile `run_reversible` true, the only one (R-500); Heggie's time transformation defaults to Eq. 22 at n = 3/2, with Eq. 20 selectable (R-161).
- Doc change: `docs/contracts/principia_integrator_contract.md` § "The equations and the step control (R-160, R-161)" — the Heggie, logH and TTL equations of motion, time transformations and step control, and the predictive step limit, transcribed from `docs/reference/prin-rs/src/integrate/{heggie,logh}/` and `az/driver.rs` with citations (Heggie 1974; Mikkola & Tanikawa 1999; Preto & Tremaine 1999) (REQ-INT-084).
- Default occupant config = Heggie × RK4 (R-560); `none`, AZ, logH selectable, and every stepper under each (R-561). *Was:* "Default occupant config = Heggie".
- Doc change: `docs/contracts/principia_integrator_contract.md` § "The profile gains a field" — where the re-registration count lives (definition, R-72); if a payload field: the ledger row in `crates/ledger`, its generated accessors and the schema-version change.
- `fixtures/gates/radial-collision/` — the two-body radial collision case.
- `cargo xtask gate hamiltonian-fd`: each occupant's analytic derivatives against a finite difference of its Hamiltonian (R-257).

## Acceptance tests
- `cargo xtask gate separation-floor --propose` — REQ-INT-105's verify for Heggie and logH, on their own floored quantities: Heggie's pre-floor `dt/dτ` and `S` under Eq. 20 and Eq. 22, logH's drift denominator in its logH and TTL modes, over the radial collision passage and the close-encounter fixtures, with the floored-step counts (every step on which the floor acts, integrator contract Part 6) and the control (a `TINY` above the smallest pre-floor value floors steps and is reported); with TASK-M3-07's Aarseth–Zare evidence it gives the proposed `TINY` at f32 and f64, which the human confirms at the M3 gate and is recorded in decisions.md (REQ-INT-105; physics review 5606990734 of PR #277, P2, applied per R-369).
- `cargo test -p kernel regularisation_default` — every stepper × regularisation pair, `none` included, runs a step on a fixture state; the default config selects Heggie × RK4, with Eq. 22 at n = 3/2 as Heggie's time transformation and Eq. 20 selectable (R-161); controls: a default config of Heggie × KDK fails, and a config builder that refuses any pair fails (REQ-INT-006; R-560, R-561). *Was:* "each stepper composes with each regularisation; the default config selects Heggie, with Eq. 22 …".
- Part 2b gives the equations, time transformations and step control for Heggie, logH and TTL and the predictive step limit, each cited to its paper and to the prin-rs file it was transcribed from; the physics reviewer approved; the human confirmed it at the M3 gate (REQ-INT-084).
- Review (physics): a consumer can read, per method or per trajectory, how many sync-boundary re-registrations occurred (REQ-INT-018).
- Part 2b states the choice; any payload field appears in the ledger with a schema-version change; physics reviewer approved (REQ-INT-073).
- `cargo xtask gate radial-collision` — the radial collision case passes through d_min ≈ 1e-11 with bounded energy drift (the AZ validation reported 6.2e-15 at d_min = 1.35e-11), run for the default occupant and AZ; the gate's values (dd_validation_orbits §2, R-257): equal masses from rest, third body far away, passing through `d_min < 1e-10` with `|dE/E| < 1e-12`, `E` the regularised energy under a regularised occupant (R-566); controls: the value the gate reads is bitwise the occupant's regularised-energy drift, and the Cartesian read-out of the same run after the passage is printed beside it and differs from it (the design measured it 280× to 7100× worse), so a gate whose reader returns the Cartesian value fails the reader check; and, the design §11's control, over a halving ladder of η the Cartesian read-out after the passage does not fall with η (REQ-VAL-045; physics review 5605806092 of PR #277, F1). *Was:* "control: … is printed and is worse than the regularised one …, so the gate pointed at it fails".
- `cargo xtask gate hamiltonian-fd` — for each occupant, the analytic derivatives match the finite-differenced Hamiltonian within REQ-VAL-171's step and tolerance over random states; a sign-flipped derivative fails (REQ-VAL-170).
- `cargo test -p kernel regularised_step_reversibility` — dd test 3 run on each built regularised occupant (Heggie, logH in each time mode, Aarseth–Zare + RK4) through its time-reversal map (REQ-INT-095), the residual and its norm taken on the full extended state, at REQ-VAL-191's criterion: each residual and verdict printed; fails if one is missing or its profile's `step_reversible` differs from its measured verdict; a verdict other than R-500's listed values (Aarseth–Zare + RK4 false, logH's TTL time mode true) is reported for the review queue; controls: the measurement at the same `dt` on RK4 and on REQ-VAL-032's asymmetric Yoshida-4 reads not reversible, a profile with one regularised occupant's flag flipped fails, and a measurement set with one built occupant left out fails (REQ-VAL-190).
- Doc change: integrator contract Part 2b — each regularised occupant's time-reversal map (Heggie's regularised coordinates and momenta and physical time; logH's time variable and its conjugate; TTL's auxiliary `W`; Aarseth–Zare + RK4's pairs), each an involution on its full state, derived from its equations; physics reviewer approves (REQ-INT-095).
- Doc change: integrator contract Part 1 § "COM projection and invariant monitoring" — each regularised occupant's regularised energy, with the equation it is read from (Heggie's and Aarseth–Zare's from their regularised coordinates and momenta; logH's from its transcribed equations); on the radial collision it stays at round-off through the passage while the Cartesian energy does not; physics reviewer approves (REQ-INT-104, R-566).
- `cargo xtask gate hamiltonian-fd --propose` — the step and tolerance with the measured discrepancy per occupant; the human confirms them at the M3 gate and they are recorded in decisions.md (REQ-VAL-171).

## Notes
- *Was: "Gap: Heggie's and logH's equations of motion and their step control are not in the corpus; REQ-INT-016 says they are transcribed from prin-rs, which is not in this repository."* RQ-102 ruled: R-159 imports the prin-rs reference set (`docs/reference/prin-rs`, reference, not authority); R-160 has this task transcribe the equations into Part 2b (REQ-INT-084), physics-reviewed and confirmed at the M3 gate. `FINDINGS.md:96`'s Eq. 20 statement is a documentation error; the default is Eq. 22 at n = 3/2 (R-161). TTL is built, validated and loses on accuracy in prin-rs, a prior finding (R-162).
- The Heggie-vs-AZ gate (REQ-INT-051) and the logH falsification (REQ-VAL-115) are TASK-M3-32's, after the comparison harness.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-073, REQ-INT-095, REQ-INT-104 (R-566); and, under R-160, REQ-INT-084.
- R-560 to R-566 (21 Oct 2026): the default occupant is Heggie × RK4; every pair runs (TASK-M3-39 measures the table); the radial collision gate reads the regularised energy. The default at every tier is Heggie × RK4 (R-606, RQ-529 (a); *was:* "RQ-529's (a) meanwhile").
- R-257 (closes RQ-157) ported kernel-build §5's gates with their values into dd_validation_orbits §2: the radial collision gate reads `d_min < 1e-10` with `|dE/E| < 1e-12` (REQ-VAL-045); the finite-difference Hamiltonian check is REQ-VAL-170, its step and tolerance REQ-VAL-171 (R-71 calibration).
