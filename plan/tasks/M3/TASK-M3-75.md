# TASK-M3-75 — logH, Heggie's and logH's seam, and the visible re-registration count

- **Milestone:** M3
- **Closes:** REQ-INT-006, REQ-INT-018, REQ-INT-073, REQ-INT-084, REQ-VAL-170, REQ-VAL-171, REQ-VAL-190, REQ-INT-095, REQ-INT-104, REQ-INT-105, REQ-INT-118, REQ-PAY-114
- **Depends on:** TASK-M3-78
- **Needs (earlier milestones):** REQ-DEC-005, REQ-GEN-008
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-4, PIT-4.3, PIT-4.4
- **Size:** ~700 lines, the tests included

**Waits on RQ-808, RQ-809 and RQ-810.** Split from TASK-M3-08, 13 Nov 2026, applied per R-369 (task scoping): logH and
the seam wiring wait on RQ-808 to RQ-810; TASK-M3-08 keeps Heggie's direct march and the gates that run it. Nothing here
is built before the three are ruled.

## Goal
The regularisation slot is complete: logH (time transformation only) is the no-chart arm, with its TTL time mode the
reversible occupant (R-162), and Heggie and logH are reached through the occupant seam, `ADVANCE`, wired into
`integrate_with` as RQ-808's ruling says, so the default occupant, Heggie × RK4 (R-560), runs as the kernel runs it. The
re-registration count is visible downstream, and integrator contract Part 2b defines where it lives — a
`re_registrations` profile field, a per-trajectory payload count, or both — with any payload field added to the ledger as
a schema change. Each requirement TASK-M3-08 built a Heggie or Aarseth–Zare arm of is closed here with its logH arm.

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
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"
- `docs/contracts/principia_integrator_contract.md` § "The sync grid is a fixed quantum of physical time (R-565)"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"
- `decisions.md` § "R-604 — A regularised copy's step budget is checked every step: its running total against `max(N_min, rate · t_elapsed)`; `N_min` protects early close encounters and is an M3 calibration *(closes RQ-527; settles R-564's accounting)*"

- `decisions.md` § "R-992 — RQ-808, Heggie's and logH's own state: option (a); each regularised occupant that registers once carries its own state across `ADVANCE` calls in the payload, registered once at `t = 0` (Heggie's `Qᵢ`, `Pᵢ` and `h`; logH's `B` and, in TTL mode, `W`), so neither re-registers and its count is 0; a payload schema change *(closes RQ-808)*"
## Deliverables
- `crates/kernel/src/regularisation/logh.rs` — logH transcribed: its time modes, logH and TTL, as RQ-809's ruling sets
  them, its TTL time mode the reversible occupant (R-162), its profile `run_reversible` true, the only one (R-500); its
  landing step as RQ-810's ruling sets it. From TASK-M3-08 (13 Nov 2026, R-369), its Deliverable's logH half.
- Heggie's and logH's seam `ADVANCE` (each one's `Regularisation` impl), wired into `integrate_with` as RQ-808's ruling
  says, each reporting its march's end through `Regularisation::advance_reaching` (`Reached::NonFinite`,
  `Reached::Exhausted`; the 12 Nov 2026 note of TASK-M3-08, physics review 5879658175 of PR #502, P2).
- Default occupant config = Heggie × RK4 (R-560); `none`, AZ, logH selectable, and every stepper under each (R-561). From
  TASK-M3-08 (13 Nov 2026, R-369). *Was (there):* "Default occupant config = Heggie".
- Doc change: `docs/contracts/principia_integrator_contract.md` § "The equations and the step control (R-160, R-161)" —
  the logH and TTL equations of motion, time transformations and step control, transcribed from
  `docs/reference/prin-rs/src/integrate/logh/` with citations (Mikkola & Tanikawa 1999; Preto & Tremaine 1999), beside
  TASK-M3-08's Heggie transcription (REQ-INT-084).
- Doc change: `docs/contracts/principia_integrator_contract.md` § "The profile gains a field" — where the re-registration
  count lives (definition, R-72); if a payload field: the ledger row in `crates/ledger`, its generated accessors and the
  schema-version change. From TASK-M3-08 (13 Nov 2026, R-369).
- logH's arms of `cargo xtask gate hamiltonian-fd`, `separation-floor --propose` and `regularised_step_reversibility`,
  beside TASK-M3-08's Heggie and Aarseth–Zare arms.

- The carried regularised state (R-992): Heggie's `Qᵢ`, `Pᵢ` and `h` (13 numbers) and its own clock, and logH's `B` and, in TTL mode, `W`, registered once at `t = 0` and passed through the seam beside `(m, r, p)` on every ADVANCE call; doc change: payload §1, the carried state's layout, widths and place (in `SimState` or a buffer beside it), a definition (R-72), physics- and perf-reviewed, with §7's memory figures recomputed and the ledger's schema change (REQ-INT-118, REQ-PAY-114). *Was (13 Nov 2026):* "Heggie's `Qᵢ`, `Pᵢ` and `h` and logH's `B`".

## Acceptance tests
- `cargo test -p kernel regularisation_default` — every stepper × regularisation pair, `none` included, runs a step on a
  fixture state; the default config selects Heggie × RK4, with Eq. 22 at n = 3/2 as Heggie's time transformation and
  Eq. 20 selectable (R-161); controls: a default config of Heggie × KDK fails, and a config builder that refuses any pair
  fails (REQ-INT-006; R-560, R-561). From TASK-M3-08 (13 Nov 2026, R-369); logH's pairs as RQ-809's ruling sets them.
- `cargo test -p kernel heggie_seam` and `cargo test -p kernel logh_seam` — each occupant drives a non-finite and an
  exhausted march through `integrate_with`, as `az_seam_non_finite_fails` and `az_seam_exhausted_stops` do, and its
  re-registrations are as RQ-808's ruling says, counted where REQ-INT-073 puts them (the 12 Nov 2026 note of TASK-M3-08).
- Part 2b gives the equations, time transformations and step control for Heggie, logH and TTL and the predictive step
  limit, each cited to its paper and to the prin-rs file it was transcribed from; the physics reviewer approved; the human
  confirmed it at the M3 gate (REQ-INT-084).
- Review (physics): a consumer can read, per method or per trajectory, how many sync-boundary re-registrations occurred
  (REQ-INT-018).
- Part 2b states the choice; any payload field appears in the ledger with a schema-version change; physics reviewer
  approved (REQ-INT-073).
- `cargo xtask gate separation-floor --propose` — REQ-INT-105's verify for Heggie and logH, on their own floored quantities: Heggie's pre-floor `dt/dτ` and `S` under Eq. 20 and Eq. 22, logH's drift denominator in its logH and TTL modes, over the radial collision passage and the close-encounter fixtures, with the floored-step counts (every step on which the floor acts, integrator contract Part 6) and the control (a `TINY` above the smallest pre-floor value floors steps and is reported); with TASK-M3-68's Aarseth–Zare evidence (*was:* TASK-M3-07's, split 10 Nov 2026, applied per R-369) it gives the proposed `TINY` at f32 and f64, which the human confirms at the M3 gate and is recorded in decisions.md (REQ-INT-105; physics review 5606990734 of PR #277, P2, applied per R-369). From TASK-M3-08 (13 Nov 2026, R-369); TASK-M3-08 built Heggie's arm, and this one adds logH's and closes it.
- `cargo xtask gate hamiltonian-fd` — for each occupant, the analytic derivatives match the finite-differenced Hamiltonian within REQ-VAL-171's step and tolerance over random states; a sign-flipped derivative fails (REQ-VAL-170). From TASK-M3-08 (13 Nov 2026, R-369); TASK-M3-08 built Heggie's and Aarseth–Zare's arms, and this one adds logH's and closes it.
- `cargo test -p kernel regularised_step_reversibility` — dd test 3 run on each built regularised occupant (Heggie, logH in each time mode, Aarseth–Zare + RK4) through its time-reversal map (REQ-INT-095), the residual and its norm taken on the full extended state, at REQ-VAL-191's criterion: each residual and verdict printed; fails if one is missing or its profile's `step_reversible` differs from its measured verdict; a verdict other than R-500's listed values (Aarseth–Zare + RK4 false, logH's TTL time mode true) is reported for the review queue; controls: the measurement at the same `dt` on RK4 and on REQ-VAL-032's asymmetric Yoshida-4 reads not reversible, a profile with one regularised occupant's flag flipped fails, and a measurement set with one built occupant left out fails (REQ-VAL-190). From TASK-M3-08 (13 Nov 2026, R-369); TASK-M3-08 built the Heggie and Aarseth–Zare + RK4 arms, and this one adds logH's time modes and closes it.
- Doc change: integrator contract Part 2b — each regularised occupant's time-reversal map (Heggie's regularised coordinates and momenta and physical time; logH's time variable and its conjugate; TTL's auxiliary `W`; Aarseth–Zare + RK4's pairs), each an involution on its full state, derived from its equations; physics reviewer approves (REQ-INT-095). From TASK-M3-08 (13 Nov 2026, R-369); TASK-M3-08 wrote Heggie's and Aarseth–Zare + RK4's maps.
- Doc change: integrator contract Part 1 § "COM projection and invariant monitoring" — each regularised occupant's regularised energy, with the equation it is read from (Heggie's and Aarseth–Zare's from their regularised coordinates and momenta; logH's from its transcribed equations); on the radial collision it stays at round-off through the passage while the Cartesian energy does not; physics reviewer approves (REQ-INT-104, R-566). From TASK-M3-08 (13 Nov 2026, R-369); TASK-M3-08 wrote Heggie's and Aarseth–Zare's.
- `cargo xtask gate hamiltonian-fd --propose` — the step and tolerance with the measured discrepancy per occupant; the human confirms them at the M3 gate and they are recorded in decisions.md (REQ-VAL-171). From TASK-M3-08 (13 Nov 2026, R-369); TASK-M3-08 proposed for Heggie and Aarseth–Zare, and this one adds logH's.

- `cargo test -p kernel regularised_state_carried` — REQ-INT-118's verify: Heggie × RK4 and logH in each time mode marched across `k` ADVANCE calls with the carried state equal one direct march registered once at `t = 0`, bitwise, with 0 re-registrations, and live march, catch-up and re-boot after eviction agree bitwise; controls: a seam rebuilding the regularised state from `(m, r, p)` at each call (Heggie's clock included), a carried state not restored on re-boot, and a re-boot that restores Heggie's `Q`, `P` and `h` but not its own clock, fail; the last control first asserts its precondition, at least one landing on its fixture whose landed clock differs from the boundary bitwise, shown before the assertion, planting one if the fixture has none or reporting the control void with the reason (REQ-INT-118). *Was (13 Nov 2026):* "controls: a seam rebuilding the regularised state from `(m, r, p)` at each call, and a carried state not restored on re-boot, fail (REQ-INT-118)."
- Review checklist — payload §1 states the carried state's layout, widths and place, §7's figures recomputed, the ledger's schema change recorded; physics and perf reviewers approve (REQ-PAY-114).

## Notes
- **Waits on RQ-808** (Heggie and logH cannot keep their regularised state across `ADVANCE` calls: the seam's wiring,
  the re-registration count and where it lives), **RQ-809** (logH's pairs with Euler, KDK and the Yoshidas, and which
  step logH's own time modes take, which sets TTL's reversibility flags) and **RQ-810** (logH's landing secant's
  precision-dependent test). Split from TASK-M3-08, 13 Nov 2026, applied per R-369 (task scoping, a two-way door, by the
  orchestrator): TASK-M3-08 had stopped on them before any code (its PR #511, the record).
- The Notes of TASK-M3-08 that bear on logH, the seam and the count stand here as there: R-159 to R-162 (the prin-rs
  reference set, the transcription, Eq. 22's default, TTL a prior finding); R-560 to R-566 and R-606 (the default occupant
  at every tier, every pair runs, the radial collision gate's regularised energy); R-257 (the finite-difference check,
  REQ-VAL-170 and REQ-VAL-171); and the 12 Nov 2026 note on `advance_reaching` (physics review 5879658175 of PR #502, P2).
- Definitions closed here (R-72; physics reviewer approves before merge): REQ-INT-073, REQ-INT-095, REQ-INT-104 (R-566);
  and, under R-160, REQ-INT-084.
- *13 Nov 2026, applied per R-369 (task scoping):* REQ-INT-118 and REQ-PAY-114, the carried state (R-992, the owner's
  answer on RQ-808), move here from TASK-M3-08 with its Deliverable and its two Acceptance lines, word for word, as
  #515's note on TASK-M3-08 says; perf joins the reviewers for the payload schema change. The R-992, R-993 and R-994
  notes stay on TASK-M3-08, and RQ-808 to RQ-810, which this task waited on, are ruled by them. *Was:* Closes without REQ-INT-118 and REQ-PAY-114; Reviewers "code, qa, physics".
- *13 Nov 2026, applied per R-369 (TASK-M3-08 split three ways):* depends on TASK-M3-78, and through it on TASK-M3-77
  and TASK-M3-08, since Heggie's march and gates, which this task wires and extends, are theirs. *Was:* "Depends on:
  TASK-M3-08".
