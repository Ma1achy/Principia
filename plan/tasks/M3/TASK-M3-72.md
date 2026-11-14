# TASK-M3-72 — The displacement form of the FTLE shadow, R-974's joint step (REQ-VAL-038), and R-976's calibration re-run

- **Milestone:** M3
- **Closes:** REQ-VAL-038
- **Depends on:** TASK-M3-14
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The f32 Benettin shadow's displacement form, as R-974 (RQ-793 (a)) rules it: a joint step on the occupant interface, `x` bit-identical to `step(x)` and `δ⃗` advanced through the same stages, the force difference formed from pair separations, plainly in f32 (a1) or cancellation-free (a2) as a pre-registered measurement chooses; the shadow's projection, energy and collision test formed from `x + δ⃗` where they need absolute positions, defined here under physics review; built beside TASK-M3-14's absolute form and tested against it on the known-Lyapunov periodic orbits, switching to displacement only if FTLE accuracy meaningfully improves (REQ-VAL-038). The definition is written into dd_integrator § 3.8 (R-72) as the ruling has it. Nothing named `Displacement` stays that is not one: TASK-M3-14's `Displacement` variant, the `shadow-form` gate's column of that name and `shadow_forms_agree` are replaced or removed once the form is built (physics review 5866988394 of PR #493, N6). Under an occupant that owns its time mapping (Heggie × RK4, the default, `dτ = S^{3/2}/(R₁R₂R₃) dt`, Part 2b, R-161; logH alike) one `Δτ` moves the sample and its shadow by different physical `Δt`, so this task also defines, under physics review (R-72; REQ-VAL-355), how the joint step keeps the two at the same physical time; the options for that definition include carrying `δt` as a component of the displacement, resynchronised at each renormalisation, or stepping the shadow's `τ` so that it lands on the sample's `t`. The Heggie × RK4 joint step and the test that the sample and its shadow keep the same physical time on it are TASK-M3-74's, which needs TASK-M3-08's Heggie occupant; the definition is written here, for any occupant that owns its time mapping. It also re-runs REQ-INT-076's pre-registered eight-orbit calibration, unchanged, once this form and R-972's (d) + (a) are built, and proposes `min_regions` ≥ 6 (R-976).

## References
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `docs/design/principia_dd_integrator.md` § "3.8 Co-computations (tier-gated, ride the forward pass)"
- `docs/design/principia_dd_integrator.md` § "3.4 COM projection (per `STEP`; the policy is integrator contract Part 1)"
- `docs/design/principia_dd_integrator.md` § "5. Unit tests"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"
- `decisions.md` § "R-974 — RQ-793, the displacement form of the FTLE shadow: option (a), a joint step on the occupant interface; whether the force difference is formed plainly in f32 (a1) or cancellation-free (a2) is chosen by a measurement whose criterion TASK-M3-72 writes before any run, physics-reviewed *(closes RQ-793)*"
- `decisions.md` § "R-976 — RQ-794, what REQ-INT-076 proposes for the FTLE shadow's ‖δ₀‖ and `n_renorm`: option (c), (3e-3, 300, 0.2) stay provisional and stated as unsupported on close-encounter chaotic orbits wherever the FTLE is shown; the pre-registered eight-orbit calibration is re-run unchanged, then `min_regions` ≥ 6 proposed; the M3 gate confirms no final value until the re-run *(closes RQ-794)*"
- `decisions.md` § "R-1009 — RQ-814, which shadow form the M3 gate's values of REQ-INT-076 are for: option (d), neither at M3; the confirmation of the FTLE shadow's ‖δ₀‖ and `n_renorm` moves to the M4 gate, beside REQ-PAY-085's switch, and the provisional (3e-3, 300, 0.2) stay in force, stated as unsupported, until then *(closes RQ-814; amends R-976)*"
- `decisions.md` § "R-972 — RQ-791, the FTLE when only its shadow reaches a terminal or goes non-finite: (d) + (a); the shadow's detectors monitor only and the shadow marches on through what would be its terminal, and once the shadow's `S` is non-finite `ftle_valid` is false, `ftle` reads NaN and every aggregate drops the sample *(closes RQ-791)*"

## Deliverables
- `crates/kernel`: the displacement-form shadow, as R-974 rules it, a joint step for each of KDK, Yoshida-4, Yoshida-6 and RK4, in both (a1) and (a2) far enough to be measured, beside the absolute form, which stays the shipped form until the measurement and REQ-PAY-085's calibration say otherwise.
- Doc change: `docs/design/principia_dd_integrator.md` § 3.8 — the displacement form's definition, per R-974 (R-72), with the shadow's projection, energy and collision test from `x + δ⃗`; integrator contract Part 2 and Part 2a, the joint step.
- This file: the criterion that chooses (a1) or (a2), written before any run (R-649) and physics-reviewed.
- Doc change: `docs/design/principia_dd_integrator.md` § 3.8 — how the joint step keeps the sample and its shadow at the same physical time under any occupant that owns its time mapping (R-72; REQ-VAL-355's definition, which TASK-M3-74 builds on and closes), physics-reviewed: carry `δt` in the displacement and resynchronise it at each renormalisation, or step the shadow's `τ` so that it lands on the sample's `t`, or another the definition states.
- `crates/validation/src/measure/benettin.rs` and `fixtures/gates/shadow-form/` — both forms against the known exponent on the periodic-orbit set, at f32 and f64, with a control that the displacement form differs bitwise from the absolute form on some orbit.
- TASK-M3-14's `Displacement` variant, the `shadow-form` gate's column of that name and `shadow_forms_agree` replaced by the real form or removed (physics N6).

## Acceptance tests
- `cargo xtask gate shadow-form` — FTLE error vs the known exponent for both shadow forms on the periodic-orbit set; the switch to displacement is made only if accuracy meaningfully improves, the result recorded (REQ-VAL-038). The measurement is the one RQ-793 names, its criterion written before the run (R-649): the signed gap `λ_T(f32) − λ_T(f64)` on the shadow-form set at ‖δ₀‖ = 1e-5, judged by its magnitude, against the absolute form's +0.023, +0.055, −0.014 and +0.204; whether Burrau at f32 stays finite at ‖δ₀‖ ≤ 1e-5; and a control that fails when the displacement form is bit-identical to the absolute form on every orbit.
- § 3.8 states the displacement form as R-974 rules it; physics reviewer approved (REQ-VAL-038).
- The (a1)/(a2) measurement, by the criterion this file states before any run (R-649), its numbers and the form it chooses recorded; physics reviewer approved (REQ-VAL-038, R-974).
- `cargo xtask gate benettin-calibration` — R-976's re-run: the pre-registered eight-orbit calibration, its grid, meter, selection and tolerance rules unchanged, run after this form and R-972's (d) + (a) are built, proposing `min_regions` ≥ 6 with its evidence; the human confirms REQ-INT-076's values at the M4 gate, beside REQ-PAY-085 (R-1009) (REQ-INT-076, closed by TASK-M3-14). *Was:* "the human confirms REQ-INT-076's values at the M3 gate".
- Review (physics): § 3.8 states how the joint step keeps the sample and its shadow at the same physical time under any occupant that owns its time mapping (REQ-VAL-355's definition; TASK-M3-74 closes REQ-VAL-355).

## The (a1)/(a2) criterion (R-974; written 13 Nov 2026, before any run, R-649)
Committed before the joint step is built or run; nothing in it changes after the results (R-649). Physics review checks
it (R-974).

- **The forms.** The absolute form (the payload's, TASK-M3-14's); (a1), the joint step with the force difference
  `f(rᵢⱼ + δᵢⱼ) − f(rᵢⱼ)` formed plainly at the kernel's precision; (a2), the joint step with the same difference formed
  cancellation-free.
- **The runs.** Yoshida-4 (the `shadow-form` gate's occupant), `n_renorm` = 300 (REQ-INT-076's provisional value), the
  default sim key at each orbit's horizon, no collision radius; each form at f32 and at f64.
- **The measure.** On each orbit of the shadow-form set (`validation::measure::benettin::periodic_set`: Lagrange at
  equal masses, at (0.5, 0.3, 0.2) and at (0.98, 0.01, 0.01), and the figure-eight, to `T` = 30), at ‖δ₀‖ = 1e-5, the
  signed gap `g = λ_T(f32) − λ_T(f64)`, both runs in the same form, so the gap is what f32 costs that form. A gap that is
  not finite counts as infinite. A form's worst gap `W` is the largest `|g|` over the four orbits.
- **Burrau.** Whether the form's f32 `λ_T` on Burrau (`validation::measure::benettin::burrau`, to its default horizon)
  is finite at both ‖δ₀‖ = 1e-5 and ‖δ₀‖ = 1e-6.
- **The choice.**
  1. Where exactly one of (a1) and (a2) stays finite on Burrau at both values, that one is chosen.
  2. Otherwise (a2) is chosen where `W(a1) − W(a2) ≥ 1/T`, `T` = 30: one neper of growth over the horizon, the
     resolution of an exponent read over `T` (the scale dd test 10's change is read on, `validation::measure::benettin`'s
     `Test10::change`). A smaller gain is below what a `λ_T` read over `T` resolves.
  3. Otherwise (a1) is chosen: the plain form, which R-974 names first, with fewer operations per stage. Both worst gaps
     infinite fall here.
- **Recorded beside the choice, deciding nothing.** Each form's signed gaps on the four orbits, against the absolute
  form's +0.023, +0.055, −0.014 and +0.204 (TASK-M3-14's figures, re-measured in the same run); each f32 form's error
  against the known exponent; the same at ‖δ₀‖ = 3e-3; and Burrau's f32 `λ_T` in each form. Whether the chosen form's
  gain over the absolute form is meaningful enough to switch the payload stays REQ-PAY-085's calibration, at M4
  (TASK-M4-13).
- **The result (13 Nov 2026, `cargo xtask gate shadow-form`, run after the criterion's commit; R-649).** At ‖δ₀‖ = 1e-5
  the signed gaps on (Lagrange equal masses, (0.5, 0.3, 0.2), (0.98, 0.01, 0.01), figure-eight) are: absolute +0.02256,
  +0.05513, −0.01379, +0.20376 (TASK-M3-14's +0.023, +0.055, −0.014, +0.204, re-measured); (a1) +0.00017, +0.00003,
  −0.00011, −0.00017, `W(a1)` = 0.00017; (a2) +0.00011, −0.00010, +0.00000, −0.00000, `W(a2)` = 0.00011. Burrau's f32
  `λ_T` at ‖δ₀‖ = 1e-5 and 1e-6: absolute NaN, NaN; (a1) 0.52176, 0.52885; (a2) 0.52169, 0.52729, both finite. Rule 1
  does not apply; `W(a1) − W(a2)` = +0.00006 < 1/T = 0.03333, so rule 2 does not; **(a1) is chosen** (rule 3), and
  `Displacement` names it. At ‖δ₀‖ = 3e-3 the worst gaps are 0.00169, 0.00010 and 0.00010.

## R-976's re-run (13 Nov 2026, `cargo xtask gate benettin-calibration`; R-649, R-71)
The pre-registered eight-orbit calibration, its grid, meter, selection and tolerance rules unchanged, run in both
shadow forms. A measurement: what the owner confirms at the M3 gate is a calibration value (R-71), not this task's
choice.

- **In force, provisional (R-976):** ‖δ₀‖ = 3e-3, `n_renorm` = 300, tolerance 0.2, unchanged by this task. The re-run
  measures them at worst change 0.4699 in the displacement form (a1), with a bias of +8.32 at f32 and +8.17 at f64 on
  (21, 20, 29) at rest, so ‖δ₀‖ = 3e-3 lies outside the linear regime there; in the absolute form their worst change is
  not finite.
- **The rule's pick, each form's proposal from its evidence:** in the displacement form (a1), ‖δ₀‖ = 1e-4,
  `n_renorm` = 1, worst change 0.0764, tolerance 0.2, 0 of 42 candidates non-finite; in the absolute form, ‖δ₀‖ = 3e-3,
  `n_renorm` = 1, worst change 1.35, tolerance 3, 39 of 42 non-finite (RQ-794 found this form's pick tests nothing).
- **`min_regions` ≥ 6, proposed (R-976), with its evidence:** the report's regions are its 6 chaotic orbits, the
  Pythagorean triangles at rest. Leaving each out moves the displacement form's pick only without (21, 20, 29), to
  (3e-4, 1), and the absolute form's only without (9, 40, 41), to (1e-3, 1000): each pick rests on all 6 regions,
  one of them deciding it.
- **Which form the M3-gate values are for** depends on REQ-PAY-085's switch, an M4 calibration (TASK-M4-13): RQ-814,
  for the owner. Nothing in this task waits on it. *14 Nov 2026:* ruled by R-1009 (RQ-814 (d)): neither at M3; REQ-INT-076
  is confirmed at the M4 gate, beside REQ-PAY-085.

## Notes
- New (12 Nov 2026), applied per R-369 (task re-scoping, physics's recommendation in review 5866988394 of PR #493, § (4)): REQ-VAL-038 and its `shadow-form` acceptance arm moved here from TASK-M3-14, whose other deliverables do not depend on the form. The arm is TASK-M3-14's, unchanged, with the measurement and control RQ-793 names added.
- Waits on RQ-793 (what the displacement form is; a one-way door). Whether a gain is "meaningful" stays REQ-PAY-085's calibration, at M4 (TASK-M4-13, which depends on this task).
- RQ-794 (what REQ-INT-076 proposes for ‖δ₀‖ and `n_renorm`) waits on RQ-793 as well; where its re-run lands is that ruling's.
- *12 Nov 2026 (PR #497's code review 5867138483, notes 1 and 2):* the measurement is written as the signed gap, its figures being signed, and R-649 joins References. *Was:* "`|λ_T(f32) − λ_T(f64)|` on the shadow-form set at ‖δ₀‖ = 1e-5".
- *12 Nov 2026 (R-974, RQ-793 (a); R-976, RQ-794 (c)):* this task no longer waits on RQ-793. The (a1)/(a2) choice is made by a measurement whose criterion this file states before any run, physics-reviewed; both forms are built far enough to measure. REQ-INT-076's re-run lands here (applied per R-369: task scoping), run after the displacement form and R-972's (d) + (a) (TASK-M3-14) are built, since a re-run before them measures nothing new; REQ-INT-076 stays TASK-M3-14's, whose provisional proposal stands until the M3 gate confirms the re-run's. *Was (title):* "The displacement form of the FTLE shadow (REQ-VAL-038, waits on RQ-793)"; *was (Size):* "~400 lines, open until RQ-793's ruling fixes the form"; *was (Goal, the clause):* "as RQ-793's ruling defines it (what is stepped, how the force difference is formed, and what the shadow's own projection, monitor and detector see),"; *was (Goal, the last sentence):* "This task waits on RQ-793; it is not dispatched before the ruling." The two notes above on RQ-793 and RQ-794 are settled by these rulings.
- *12 Nov 2026, later (R-974, physics review 5875695467 of PR #503, F1):* a recorded consequence of R-974: under an occupant that owns its time mapping the same `Δτ` moves the sample and its shadow by different `Δt`, and `δ⃗` gains a component along the flow; Closes gains REQ-VAL-355, its definition and its acceptance arm. *Was:* "Closes: REQ-VAL-038".
- *Re-scoped 13 Nov 2026, applied per R-369 (task scoping, a two-way door; the orchestrator): Heggie × RK4 needs
  TASK-M3-08.* The joint step on the default occupant, Heggie × RK4, needs TASK-M3-08's Heggie occupant, which this
  task does not depend on. It moves to TASK-M3-74 (after TASK-M3-08 and this task), with `displacement_shadow_same_time`
  and its control; REQ-VAL-355 moves with its verify and is closed there. This task keeps the joint step for KDK,
  Yoshida-4, Yoshida-6 and RK4 in (a1) and (a2), the criterion and the measurement, the `shadow-form` gate and its
  control, the N6 cleanup, both § 3.8 doc changes, the same-time definition written for any occupant that owns its time
  mapping, and R-976's re-run. *Was (Closes):* "REQ-VAL-038, REQ-VAL-355". *Was (Deliverables, the first):* "a joint
  step for each of KDK, Yoshida-4, Yoshida-6, RK4 and the default regularised occupant, in both (a1) and (a2)". *Was
  (Deliverables, the same-time doc change):* "under an occupant that owns its time mapping (R-72; REQ-VAL-355)". *Was
  (Acceptance, the review arm):* "under an occupant that owns its time mapping (REQ-VAL-355)." *Was (Acceptance, the
  last arm):* "`cargo test -p kernel displacement_shadow_same_time` — on the Heggie × RK4 occupant, at every
  renormalisation of a joint-step run, the shadow's elapsed physical time equals the sample's as the definition states;
  control: a joint step that advances the shadow by the sample's `Δτ` alone, its elapsed `t` drifting from the sample's,
  fails (REQ-VAL-355)."
- *14 Nov 2026 (R-1009, RQ-814 (d)):* neither of the re-run's two picks is confirmed at the M3 gate: REQ-INT-076 moves to M4 and is confirmed at the M4 gate, beside REQ-PAY-085 (TASK-M4-13), whose measurement says which form ships and so which pick applies; (3e-3, 300, 0.2) stay in force, provisional and stated as unsupported, until then. The 12 Nov note's "until the M3 gate confirms the re-run's" is its record.
