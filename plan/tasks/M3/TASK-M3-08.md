# TASK-M3-08 — Heggie's regularised system: its equations, maps and regularised energy

- **Milestone:** M3
- **Closes:** REQ-INT-121
- **Depends on:** TASK-M3-68
- **Needs (earlier milestones):** REQ-DEC-005, REQ-GEN-008
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-4, PIT-4.3, PIT-4.4
- **Size:** ~1,100 lines, the contract's transcription and the tests included

**Split three ways 13 Nov 2026, applied per R-369 (task scoping; the owner's standing rule to split under ~300 mutants
where natural, and before review above ~500).** The built Heggie part counted 1,008 mutants. This task keeps Heggie's
regularised system: the contract's transcription (Part 2b) and the regularised energies (Part 1), the system, its
registration and return, its time-reversal map and its regularised energy in the kernel, and the energies Aarseth–Zare's
march reports. TASK-M3-77 takes the stepper slot, the step control and the direct march, with the kernel tests that need
the march (REQ-INT-122); TASK-M3-78 takes the gates, `radial-collision`, `hamiltonian-fd` and `separation-floor`'s Heggie
arm, with REQ-VAL-045 and REQ-VAL-357. *Was:* title "The Heggie occupant, registered once at t = 0, and the radial
collision gate"; Closes "REQ-VAL-045, REQ-VAL-357"; Size "~900 lines, the tests included".

**Re-scoped 13 Nov 2026, applied per R-369: logH and seam wiring wait on RQ-808 to RQ-810.** This task keeps Heggie:
its equations and step control in integrator contract Part 2b, its regularised energy in Part 1, Heggie × every stepper
as direct marches registered once at `t = 0` (as the reference driver does), the `radial-collision` gate for Heggie × RK4
and Aarseth–Zare, `hamiltonian-fd` for Heggie and Aarseth–Zare, and Heggie's arms of `separation-floor --propose` and
`regularised_step_reversibility`. TASK-M3-75 takes logH, the wiring into `integrate_with` (the seam's `ADVANCE`), the
visible re-registration count (REQ-INT-018, REQ-INT-073), the seam tests and logH's arms, and closes the requirements
whose logH or seam arm it builds. *Was:* title "Heggie and logH occupants, and the visible re-registration count";
Closes "REQ-INT-006, REQ-INT-018, REQ-INT-073, REQ-VAL-045, REQ-INT-084, REQ-VAL-170, REQ-VAL-171, REQ-VAL-190,
REQ-INT-095, REQ-INT-104, REQ-INT-105"; Size "~520 lines".

## Goal
Heggie 1974 global regularisation (three relative vectors, no reference body, never re-registers) is transcribed into
integrator contract Part 2b with citations, its equations, time transformations (Eq. 22 at n = 3/2 the default, Eq. 20
selectable; R-161), maps and step control, and its regularised energy and Aarseth–Zare's are defined in Part 1 (R-566).
The kernel builds Heggie's regularised system to it, `Γ*` and its rates under each time transformation, the registration
from and the return to the Cartesian state, the time-reversal map and the regularised energy, each checked against an
independent anchor; Aarseth–Zare's march reports the energies the radial-collision gate reads (REQ-INT-121).
*Was (13 Nov 2026, split three ways):* "Heggie 1974 global regularisation (three relative vectors, no reference body, never re-registers) is built to integrator
contract Part 2b, its equations, time transformations (Eq. 22 at n = 3/2 the default, Eq. 20 selectable; R-161) and step
control transcribed there with citations, and marched directly: registered once at `t = 0` and carried across every sync
boundary of R-565's grid, with every stepper through its form on Heggie's regularised system (R-561), Heggie × RK4 the
default pair (R-560). Its regularised energy is defined in Part 1 (R-566), and the two-body radial collision gate, kept as
the regularisation test, reads it for Heggie × RK4 and for Aarseth–Zare. Heggie and Aarseth–Zare are finite-difference
checked against their Hamiltonians, and Heggie's floor and time-reversal arms are measured. *Was (13 Nov 2026,
re-scoped):* 'The regularisation slot is complete: Heggie 1974 global regularisation (three relative vectors, no
reference body, never re-registers), stepped with RK4, is the default occupant, Heggie × RK4 (R-560), and logH (time
transformation only) is the no-chart arm. The re-registration count is visible downstream, and integrator contract Part 2b
defines where it lives — a `re_registrations` profile field, a per-trajectory payload count, or both — with any payload
field added to the ledger as a schema change. The two-body radial collision gate is kept as the regularisation test.'"

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
- `decisions.md` § "R-992 — RQ-808, Heggie's and logH's own state: option (a); each regularised occupant that registers once carries its own state across `ADVANCE` calls in the payload, registered once at `t = 0` (Heggie's `Qᵢ`, `Pᵢ` and `h`; logH's `B` and, in TTL mode, `W`), so neither re-registers and its count is 0; a payload schema change *(closes RQ-808)*"
- `decisions.md` § "R-993 — RQ-809, logH's step: option (a); logH's own time modes are the reference's drift–kick–drift leapfrog, `W` advanced at the midpoint velocity in TTL mode, selected as logH × KDK; logH × RK4 is classical RK4 on `(r, v, t, W)`; logH × Euler and × Yoshida wait for TASK-M3-39 *(closes RQ-809; refines R-500)*"
- `decisions.md` § "R-994 — RQ-810, logH's landing: option (a), with RQ-795's answer; logH tests the landing after the step, as Heggie and Aarseth–Zare do, under R-998's rule: no landing tolerance, and the state lands on the boundary with the clock *(closes RQ-810)*"
- `docs/design/principia_dd_simstate_payload.md` § "0. The two buffers (split by access pattern)"

- `docs/contracts/principia_integrator_contract.md` § "COM projection and invariant monitoring"
- `docs/contracts/principia_integrator_contract.md` § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)"
- `decisions.md` § "R-560 — The default occupant is Heggie × RK4, the combination every Heggie result was measured with; REQ-GUI-102's default and the Run window change with it"
- `decisions.md` § "R-561 — Every stepper × regularisation pair is allowed, regularisation off included; a pair measured not to work carries a warning beside the selector, in the Problems panel and in each export's settings, and is still defined exactly so its results reproduce; a table marks each pair measured good, measured bad or not measured"
- `decisions.md` § "R-566 — Under a regularised occupant the energy is named twice: the two-body collision gate (REQ-VAL-045) and any per-step suspect flag read the regularised energy; the Cartesian energy is for diagnostics, read only from runs that did not stop at an event *(refines R-257 as to the radial collision gate's energy)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 6 — Three boundaries stated honestly (deferred work lands here)"
- `decisions.md` § "R-563 — The integration floor follows the occupant: with regularisation off the substep machinery is unchanged; under a regularised occupant running out of the step budget makes the result undetermined, never discarded, and the separation floor and the entry-cap hits are counted, non-terminal signals; REQ-SCHED-063 and REQ-SCHED-077 are re-pointed"
- `decisions.md` § "R-606 — The default occupant is Heggie × RK4 at every tier: tiers trade resolution and samples, never the integrator; the tier column is the per-stepper binding with regularisation `none`; revisited as RQ-529 (c) once more pairs are measured *(closes RQ-529)*"
- `docs/contracts/principia_integrator_contract.md` § "Aarseth–Zare: the equations, the reference body, the Levi-Civita inverse and the step control (transcribed)"
- `docs/contracts/principia_integrator_contract.md` § "The sync grid is a fixed quantum of physical time (R-565)"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"
- `decisions.md` § "R-604 — A regularised copy's step budget is checked every step: its running total against `max(N_min, rate · t_elapsed)`; `N_min` protects early close encounters and is an M3 calibration *(closes RQ-527; settles R-564's accounting)*"

## Deliverables
- `crates/kernel/src/regularisation/heggie.rs` — Heggie's regularised system transcribed: the three relative vectors and
  their Levi-Civita regularisation, `Γ*` and its equations of motion under Eq. 20 and under Eq. 22 at n = 3/2, the
  default (R-161), the registration from the Cartesian state (`register`) and the return to it (`HgSystem::image`), the
  regularised energy (`HgSystem::energy`), the time-reversal map (`reversed`) and the regularised system as a `Flow` for
  the steppers' forms. The stepper slot, the step control and the march are TASK-M3-77's.
- `crates/kernel/src/regularisation/az.rs` — Aarseth–Zare's regularised energy read from its regularised state
  (`AzSystem::energy`), and its march reporting the `E` of its first registration and the regularised energy where it
  ended (`AzAdvanced::registered`, `AzAdvanced::regularised`; R-566).
- Doc change: integrator contract Part 2b — Heggie's equations of motion, time transformations and step control, and its
  predictive step limit, transcribed from `docs/reference/prin-rs/src/integrate/heggie/` with citations (Heggie 1974),
  and Heggie's and Aarseth–Zare + RK4's time-reversal maps, each an involution on its full state; Part 1 — Heggie's and
  Aarseth–Zare's regularised energies, each with the equation it is read from; physics-reviewed (REQ-INT-084's,
  REQ-INT-095's and REQ-INT-104's Heggie and Aarseth–Zare parts; TASK-M3-75 adds logH's and closes them).
- *Was (13 Nov 2026, split three ways), the Deliverables before the split; the march's are TASK-M3-77's and the gates'
  TASK-M3-78's, each restated there:*
  - `crates/kernel/src/regularisation/heggie.rs` — Heggie transcribed: the three relative vectors and their Levi-Civita
    regularisation, `Γ*` and its equations of motion under Eq. 20 and under Eq. 22 at n = 3/2, the default (R-161), the
    maps to and from the Cartesian state, the regularised energy, and the step control (the per-step interval rule, the
    predictive limit on the relative velocities, the landing clamp and secant, the explicit finiteness exit and R-604's
    budget), with Part 6's floor signal and entry-cap hits counted and each step's physical increment fed to `dt_max`; the
    march registered once at `t = 0` and carried across every boundary of R-565's grid; every stepper fills it through its
    form on Heggie's system (R-561), each pair's profile its form's. Heggie's seam `ADVANCE` (its `Regularisation` impl)
    and its wiring into `integrate_with` are TASK-M3-75's (RQ-808). *Was (13 Nov 2026, re-scoped):*
    "`crates/kernel/src/regularisation/{heggie,logh}.rs` — transcribed physics layers; logH carries its TTL time mode, the
    reversible occupant (R-162), its profile `run_reversible` true, the only one (R-500); Heggie's time transformation
    defaults to Eq. 22 at n = 3/2, with Eq. 20 selectable (R-161)."
  - Doc change: `docs/contracts/principia_integrator_contract.md` Part 2b — Heggie's equations of motion, time
    transformations and step control, and its predictive step limit, transcribed from `docs/reference/prin-rs/src/integrate/heggie/`
    with citations (Heggie 1974), physics-reviewed; REQ-INT-084's Heggie part, which TASK-M3-75 completes with logH and TTL
    and closes. *Was (13 Nov 2026, re-scoped):* "Doc change: `docs/contracts/principia_integrator_contract.md` § "The
    equations and the step control (R-160, R-161)" — the Heggie, logH and TTL equations of motion, time transformations and
    step control, and the predictive step limit, transcribed from `docs/reference/prin-rs/src/integrate/{heggie,logh}/` and
    `az/driver.rs` with citations (Heggie 1974; Mikkola & Tanikawa 1999; Preto & Tremaine 1999) (REQ-INT-084)."
  - Doc change: integrator contract Part 1 § "COM projection and invariant monitoring" — Heggie's and Aarseth–Zare's
    regularised energy, each with the equation it is read from (REQ-INT-104's Heggie and Aarseth–Zare parts; logH's is
    TASK-M3-75's, which closes it); and Part 2b — Heggie's and Aarseth–Zare + RK4's time-reversal maps, each an involution on
    its full state, derived from its equations (REQ-INT-095's Heggie and Aarseth–Zare parts; logH's is TASK-M3-75's, which
    closes it). Aarseth–Zare's march reports its regularised energy beside its state, so the gate reads the occupant's own
    value.
  - `fixtures/gates/radial-collision/` — the two-body radial collision case.
  - `cargo xtask gate hamiltonian-fd`: Heggie's (under Eq. 20 and Eq. 22) and Aarseth–Zare's analytic derivatives against
    a finite difference of their Hamiltonians (R-257), with `--propose` for REQ-VAL-171 on those occupants; logH's arm, and
    the closing of REQ-VAL-170 and REQ-VAL-171, are TASK-M3-75's. *Was (13 Nov 2026, re-scoped):* "`cargo xtask gate hamiltonian-fd`: each occupant's analytic derivatives against a finite difference of its Hamiltonian (R-257)."
  - *Was (13 Nov 2026, moved to TASK-M3-75):* "Default occupant config = Heggie × RK4 (R-560); `none`, AZ, logH selectable,
    and every stepper under each (R-561). *Was:* "Default occupant config = Heggie"."
  - *Was (13 Nov 2026, moved to TASK-M3-75):* "Doc change: `docs/contracts/principia_integrator_contract.md` § "The profile
    gains a field" — where the re-registration count lives (definition, R-72); if a payload field: the ledger row in
    `crates/ledger`, its generated accessors and the schema-version change."

  - *Was (13 Nov 2026, moved to TASK-M3-75 with REQ-INT-118 and REQ-PAY-114, as #515's note below says):* "The carried regularised state (R-992): Heggie's `Qᵢ`, `Pᵢ` and `h` (13 numbers) and its own clock, and logH's `B` and, in TTL mode, `W`, registered once at `t = 0` and passed through the seam beside `(m, r, p)` on every ADVANCE call; doc change: payload §1, the carried state's layout, widths and place (in `SimState` or a buffer beside it), a definition (R-72), physics- and perf-reviewed, with §7's memory figures recomputed and the ledger's schema change (REQ-INT-118, REQ-PAY-114). *Was (13 Nov 2026):* 'Heggie's `Qᵢ`, `Pᵢ` and `h` and logH's `B`'."

## Acceptance tests
- `cargo test -p kernel --test heggie_system` — Heggie's regularised system against independent anchors: the Cartesian →
  regularised → Cartesian round trip at unequal masses, the registered `h` and the regularised energy against the
  Cartesian energy, and Eq. 7's relative velocities against the image's (control: a quarter-turned `Q₀`); `Γ* = (E − h)·
  R₀R₁R₂` with `E` the Cartesian energy of the image (control: two reduced masses swapped); the rates under Eq. 20 and
  Eq. 22 against a central difference of the Hamiltonian at step `1e-6`, within `1e-8` (control: a sign flipped);
  `dt/dτ` and the floors at `TINY`; the time-reversal map an involution negating `P` and `t`; the flow's sums; the default
  Eq. 22 (R-161); each with its control (REQ-INT-121).
- `cargo test -p kernel --test heggie_system az_energy_reported` — Aarseth–Zare's march reports its first registration's
  `E` bitwise and the regularised energy where it ended; control: the last registration's `E` read for the first's.
- Doc review (physics): Part 2b's Heggie transcription and the two time-reversal maps, and Part 1's regularised energies.
- *Was (13 Nov 2026, split three ways), the Acceptance tests before the split; TASK-M3-77's and TASK-M3-78's restate those
  they take, and the transcription's own checks that need no march are the first line above:*
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
  - `cargo test -p kernel heggie_pairs` — Heggie × every stepper (Euler, KDK, Yoshida-4, Yoshida-6, RK4) under Eq. 20 and
    Eq. 22 marches a fixture state across a sync boundary as a direct march, its state finite and its profile its form's;
    Heggie's time transformation defaults to Eq. 22, Eq. 20 selectable (R-161); controls: a march whose stepper takes no step
    fails, and a default of Eq. 20 fails (REQ-INT-006's Heggie part, which TASK-M3-75 closes with the seam's pairs).
  - `cargo test -p kernel heggie_march` — the transcription's own checks: the Cartesian → regularised → Cartesian round trip
    at unequal masses, the regularised energy against the Cartesian energy at registration, `Γ* = (E_reg − h)·R₁R₂R₃`, the
    march registered once (its frozen `h` unchanged across boundaries), landing on each boundary at `k·Δ_sync`, the
    explicit finiteness exit and R-604's budget ending the march latched (`End::NonFinite`, `End::Exhausted`), the floor
    signal on `dt/dτ < TINY`, `S < TINY` or `Rᵢ < TINY`, and the entry-cap hit on `c₀ > c₁`, each with its control.
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
  - `cargo test -p kernel regularised_step_reversibility` — dd test 3 run on Heggie × RK4 (Eq. 20 and Eq. 22) and
    Aarseth–Zare + RK4 through each one's time-reversal map, the residual and its norm taken on the full extended state, at
    REQ-VAL-191's criterion: each residual and verdict printed; fails if one is missing or its profile's `step_reversible`
    differs from its measured verdict; a verdict other than R-500's listed value (Aarseth–Zare + RK4 false) is reported for
    the review queue; controls: the measurement at the same `dt` on RK4 and on REQ-VAL-032's asymmetric Yoshida-4 reads not
    reversible, a profile with one regularised occupant's flag flipped fails, and a measurement set with one built occupant
    left out fails (REQ-VAL-190's Heggie and Aarseth–Zare arms; TASK-M3-75 adds logH's time modes and closes it).
  - Doc change: integrator contract Part 2b — Heggie's equations, time transformations and step control, and its
    predictive step limit, each cited to Heggie 1974 and to the prin-rs file it was transcribed from; Heggie's and
    Aarseth–Zare + RK4's time-reversal maps, each an involution on its full state, derived from its equations; Part 1 —
    Heggie's and Aarseth–Zare's regularised energy with the equation each is read from, on the radial collision at
    round-off through the passage while the Cartesian energy is not; the physics reviewer approves.
  - *Was (13 Nov 2026, re-scoped), the lines the Heggie and Aarseth–Zare arms above replace, their logH arms moved to TASK-M3-75:*
    - "`cargo xtask gate separation-floor --propose` — REQ-INT-105's verify for Heggie and logH, on their own floored quantities: Heggie's pre-floor `dt/dτ` and `S` under Eq. 20 and Eq. 22, logH's drift denominator in its logH and TTL modes, over the radial collision passage and the close-encounter fixtures, with the floored-step counts (every step on which the floor acts, integrator contract Part 6) and the control (a `TINY` above the smallest pre-floor value floors steps and is reported); with TASK-M3-68's Aarseth–Zare evidence (*was:* TASK-M3-07's, split 10 Nov 2026, applied per R-369) it gives the proposed `TINY` at f32 and f64, which the human confirms at the M3 gate and is recorded in decisions.md (REQ-INT-105; physics review 5606990734 of PR #277, P2, applied per R-369)."
    - "`cargo xtask gate hamiltonian-fd` — for each occupant, the analytic derivatives match the finite-differenced Hamiltonian within REQ-VAL-171's step and tolerance over random states; a sign-flipped derivative fails (REQ-VAL-170)."
    - "`cargo test -p kernel regularised_step_reversibility` — dd test 3 run on each built regularised occupant (Heggie, logH in each time mode, Aarseth–Zare + RK4) through its time-reversal map (REQ-INT-095), the residual and its norm taken on the full extended state, at REQ-VAL-191's criterion: each residual and verdict printed; fails if one is missing or its profile's `step_reversible` differs from its measured verdict; a verdict other than R-500's listed values (Aarseth–Zare + RK4 false, logH's TTL time mode true) is reported for the review queue; controls: the measurement at the same `dt` on RK4 and on REQ-VAL-032's asymmetric Yoshida-4 reads not reversible, a profile with one regularised occupant's flag flipped fails, and a measurement set with one built occupant left out fails (REQ-VAL-190)."
    - "Doc change: integrator contract Part 2b — each regularised occupant's time-reversal map (Heggie's regularised coordinates and momenta and physical time; logH's time variable and its conjugate; TTL's auxiliary `W`; Aarseth–Zare + RK4's pairs), each an involution on its full state, derived from its equations; physics reviewer approves (REQ-INT-095)."
    - "Doc change: integrator contract Part 1 § "COM projection and invariant monitoring" — each regularised occupant's regularised energy, with the equation it is read from (Heggie's and Aarseth–Zare's from their regularised coordinates and momenta; logH's from its transcribed equations); on the radial collision it stays at round-off through the passage while the Cartesian energy does not; physics reviewer approves (REQ-INT-104, R-566)."
    - "`cargo xtask gate hamiltonian-fd --propose` — the step and tolerance with the measured discrepancy per occupant; the human confirms them at the M3 gate and they are recorded in decisions.md (REQ-VAL-171)."
  - *Was (13 Nov 2026, moved to TASK-M3-75):* "`cargo test -p kernel regularisation_default` — every stepper ×
    regularisation pair, `none` included, runs a step on a fixture state; the default config selects Heggie × RK4, with Eq. 22
    at n = 3/2 as Heggie's time transformation and Eq. 20 selectable (R-161); controls: a default config of Heggie × KDK
    fails, and a config builder that refuses any pair fails (REQ-INT-006; R-560, R-561)."
  - *Was (13 Nov 2026, moved to TASK-M3-75):* "Part 2b gives the equations, time transformations and step control for
    Heggie, logH and TTL and the predictive step limit, each cited to its paper and to the prin-rs file it was transcribed
    from; the physics reviewer approved; the human confirmed it at the M3 gate (REQ-INT-084)."; "Review (physics): a
    consumer can read, per method or per trajectory, how many sync-boundary re-registrations occurred (REQ-INT-018)."; "Part
    2b states the choice; any payload field appears in the ledger with a schema-version change; physics reviewer approved
    (REQ-INT-073)."; and the logH arms of `hamiltonian-fd`, `separation-floor --propose` and
    `regularised_step_reversibility`, with the logH parts of the two Doc changes (REQ-INT-095, REQ-INT-104).
  - *Was (13 Nov 2026, moved to TASK-M3-75 with REQ-INT-118 and REQ-PAY-114):* the `regularised_state_carried` test and
    the payload §1 review checklist (REQ-INT-118, REQ-PAY-114), word for word in TASK-M3-75.

## Notes
- *Was: "Gap: Heggie's and logH's equations of motion and their step control are not in the corpus; REQ-INT-016 says they are transcribed from prin-rs, which is not in this repository."* RQ-102 ruled: R-159 imports the prin-rs reference set (`docs/reference/prin-rs`, reference, not authority); R-160 has this task transcribe the equations into Part 2b (REQ-INT-084), physics-reviewed and confirmed at the M3 gate. `FINDINGS.md:96`'s Eq. 20 statement is a documentation error; the default is Eq. 22 at n = 3/2 (R-161). TTL is built, validated and loses on accuracy in prin-rs, a prior finding (R-162).
- The Heggie-vs-AZ gate (REQ-INT-051) and the logH falsification (REQ-VAL-115) are TASK-M3-32's, after the comparison harness.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-073, REQ-INT-095, REQ-INT-104 (R-566); and, under R-160, REQ-INT-084.
- R-560 to R-566 (21 Oct 2026): the default occupant is Heggie × RK4; every pair runs (TASK-M3-39 measures the table); the radial collision gate reads the regularised energy. The default at every tier is Heggie × RK4 (R-606, RQ-529 (a); *was:* "RQ-529's (a) meanwhile").
- R-257 (closes RQ-157) ported kernel-build §5's gates with their values into dd_validation_orbits §2: the radial collision gate reads `d_min < 1e-10` with `|dE/E| < 1e-12` (REQ-VAL-045); the finite-difference Hamiltonian check is REQ-VAL-170, its step and tolerance REQ-VAL-171 (R-71 calibration).
- Depends on TASK-M3-68, not TASK-M3-07 (applied per R-369, 10 Nov 2026: TASK-M3-07 was split, and Aarseth–Zare is TASK-M3-68): its radial-collision gate, `regularised_step_reversibility` and the two Doc changes read Aarseth–Zare, and its `separation-floor` proposal takes Aarseth–Zare's evidence. *Was:* "TASK-M3-07" in Depends on.
- **12 Nov 2026 (PR #502; physics review 5879658175, P2):** Heggie and logH report their own march's end through `Regularisation::advance_reaching` (`crates/kernel/src/regularisation/mod.rs`), `Reached::NonFinite` at the explicit finiteness exit and `Reached::Exhausted` on R-604's step budget, as Aarseth–Zare does, rather than inherit the trait's default. The default reports only `Target` or `Short` and so suits only a march that cannot fail: a non-finite or budget-exhausted march that took it would count as completed and be run on, against R-604's "never run on" and Part 2a's "a terminal event … still latches at the step it fires on". Each occupant's test drives a non-finite and an exhausted march through `integrate_with`, as `az_seam_non_finite_fails` and `az_seam_exhausted_stops` do.
- *13 Nov 2026, applied per R-369 (task scoping, the orchestrator):* TASK-M3-08 is split. The ADVANCE seam passes
  `(m, r, p)` once per macro-step, so Heggie and logH could not keep their regularised state across calls (RQ-808);
  logH's pairs and its own step are unstated (RQ-809); and logH's landing secant tests a precision-dependent literal
  (RQ-810). None of the three touches Heggie's direct march, registered once at `t = 0` as the reference driver does, so
  this task builds it and the gates that run it; TASK-M3-75 waits on the three and takes the rest. The 12 Nov note below
  (`advance_reaching` and the `integrate_with` tests) is TASK-M3-75's; here the direct march ends `NonFinite` and
  `Exhausted` as Aarseth–Zare's does, tested directly (`heggie_march`).
- *13 Nov 2026, applied per R-369:* Aarseth–Zare's arms of `regularised_step_reversibility` and of the two Doc changes
  (its time-reversal map, its regularised energy) are built here with Heggie's: the original task named them, the split
  left them with neither part, and the reversibility test's own rule ("fails if one is missing") counts every built
  regularised occupant.
- **13 Nov 2026 (R-992, R-993, R-994; RQ-808 to RQ-810):** the owner answered the three questions this task stopped on. Heggie and logH carry their own state across ADVANCE calls, registered once (R-992), so REQ-INT-118 and REQ-PAY-114 are added here; logH's own time modes are the reference's drift–kick–drift leapfrog and classical RK4 (R-993); logH tests its landing after the step under R-998's rule (R-994). The split of this task (the report's D-22, the logH-and-hookup task) is not on `main`; when it lands it moves REQ-INT-118, REQ-PAY-114 and the carried state to the new task (applied per R-369: task scoping).
- **13 Nov 2026 (physics review 5886173032 of #515, F5; code review 5886308127, C6):** Heggie's carried state includes its own clock, as R-992's option (a) reads ("13 numbers and the clock"), so the carried-state Deliverable and `regularised_state_carried` name it, with the clock re-boot control and its precondition (REQ-INT-118, REQ-PAY-114).
- **13 Nov 2026 (RQ-812; code review 5886308127 of #515, C7; applied per R-369: task scoping):** this task is not held whole. Only Heggie's landing, the landing branch of its step control, waits on RQ-812 (Heggie's landing, open; REQ-INT-084 names it). The reference's `LAND_EPS_REL` (`heggie/driver.rs:328`, `:345`, `:385`, `:390`) is never put into a branch (rule 1). Everything else in the task proceeds.
- *13 Nov 2026, applied per R-369 (task scoping, the split landing after #515):* REQ-INT-118 and REQ-PAY-114, the
  carried-state Deliverable and its two Acceptance lines move to TASK-M3-75, as the R-992 note above says; perf stays a
  reviewer here, as #515's plan names it.
- *13 Nov 2026, applied per R-369 (the orchestrator, following #502's precedent for Aarseth–Zare before RQ-795 was
  ruled):* Heggie's landing tolerance is a caller-supplied setting (`HgParams::land_eps_rel`), with no value in the kernel
  and no production configuration; its tests and gates run at the reference's values (`1e-5` at f32, `1e-14` at f64) and
  say so, and each site is marked RQ-812. The final landing follows RQ-812's ruling, through a task.
- *13 Nov 2026, applied per R-369 (the orchestrator; task scoping):* the three-way split above. The Doc changes stay
  here whole, the step control's transcription included, since TASK-M3-77 builds to them; the radial collision's
  measured numbers in Part 1 are TASK-M3-78's. `heggie::register` returns a struct, not a tuple, and so do the
  validation FD helpers, to drop combinatorial mutants (changes no result); `az::register` keeps its tuple, since
  `qa_TASK-M3-68.rs` destructures it and the implementer does not edit qa's files (R-290).
