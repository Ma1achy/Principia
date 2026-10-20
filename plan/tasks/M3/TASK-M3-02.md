# TASK-M3-02 — Stepper occupants and the capability profile

- **Milestone:** M3
- **Closes:** REQ-INT-005, REQ-INT-013, REQ-INT-014, REQ-INT-015, REQ-INT-037, REQ-INT-056, REQ-INT-078, REQ-VAL-032, REQ-INT-090, REQ-VAL-191
- **Depends on:** TASK-M3-01
- **Needs (earlier milestones):** REQ-INT-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The five stepper occupants — Euler (debug only), KDK, Yoshida-4, Yoshida-6 and RK4 — exist as build-time Rust variants implementing `STEP` with exactly dd_integrator §3.2's update rules and coefficients, each advertising its capability profile `{order, force_evals, symplectic, step_reversible, run_reversible, owns_time_mapping}` (the re-registration field is added by TASK-M3-08; R-500 split `reversible` into the two flags, KDK, Yoshida-4 and Yoshida-6 step-reversible and none of them run-reversible). Yoshida-4's and Yoshida-6's coefficients carry their citation (Yoshida 1990; Yoshida-6 its Table 1, solution A) and pass a convergence-order test at orders 4 and 6 that fails on a transcription error before they enter the shared source; the paper is not added (R-57, R-501). *Was:* "Yoshida-6's coefficients are checked against Yoshida (1990) Table 1 solution A and the `w₂ < 0` label fixed". Integrator contract Part 2 gains the definition of how `order` sets the meaning of the `total_substeps_log2` proxy and FTLE confidence (R-72).

## References
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `docs/design/principia_core_design.md` § "4. Integrate and colour are separate passes — and now separate *mechanisms*"
- `docs/contracts/principia_gui_state_contract.md` § "3. The registry is the scanned filesystem — for the *fragment* side; compute occupants are Rust build variants"
- `docs/contracts/principia_gui_state_contract.md` § "5. The stain editor — a free, typed node graph (R-64)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_integrator_contract.md` § "The table gains two rows"
- `docs/design/principia_dd_integrator.md` § "3.2 Occupants"
- `decisions.md` § "R-57 — The Yoshida-6 coefficients are checked against Yoshida (1990), and the `w₂ < 0` label fixed *(TO-1)*"
- `open-questions.md` § "Audit section C — transcription checks"
- `docs/design/principia_dd_integrator.md` § "6. Deferred / flagged"
- `docs/design/principia_dd_integrator.md` § "5. Unit tests"
- `decisions.md` § "R-162 — The reversible occupant is logH's TTL time mode"
- `decisions.md` § "R-168 — REQ-INT-014 covers the built occupants; Aarseth–Zare + TTL is an allowed future occupant *(follows R-162)*"
- `decisions.md` § "R-500 — The capability profile's `reversible` is two flags: `step_reversible`, true for KDK, Yoshida-4, Yoshida-6 and the TTL leapfrog, false for Euler, RK4 and Aarseth–Zare + RK4, the regularised occupants measured; and `run_reversible`, logH-TTL only; both diagnostic routes gate on `step_reversible`, and a negative weight does not break a symmetric step *(closes RQ-369; refines R-162; amends R-57 as to integrator contract Part 6's sentence)*"
- `decisions.md` § "R-501 — Yoshida's coefficients carry their citation, Yoshida (1990), Table 1 for Yoshida-6; the paper is not added; a convergence-order test at orders 4 and 6 fails on a transcription error *(amends R-57)*"

## Deliverables
- `crates/kernel/src/occupant/mod.rs` — the `Stepper` trait (`fn step(state, dt, geom)`), `CapabilityProfile` struct, and the closed, build-time occupant set (an enum of monomorphised types; no runtime compilation or interpretation path).
- `crates/kernel/src/occupant/{euler,kdk,yoshida4,yoshida6,rk4}.rs` — §3.2's rules; Yoshida-4 and -6 as KDK compositions with the printed drift/kick sequences.
- `crates/kernel/src/occupant/table.rs` — the Part 2 / Part 2a profile table, including the two AZ rows' profiles (filled by TASK-M3-07), and the default tier binding per occupant; Euler reachable only from a debug selection.
- Doc change: `docs/contracts/principia_integrator_contract.md` Part 2 — the mapping from profile `order` to the interpretation of `total_substeps_log2` and to FTLE confidence (definition, REQ-INT-078).
- The coefficient blocks in `yoshida4.rs` and `yoshida6.rs` carry the citation dd §3.2 prints (R-501). The doc changes R-57 asked for here were made by R-500's and R-501's port (dd §3.2, §5 test 1a, §6; integrator contract Part 6). *Was:* "Doc change: `docs/design/principia_dd_integrator.md` §3.2 / §6 — the Yoshida-6 transcription check recorded (the digits compared with Yoshida 1990 Table 1 solution A) and the `w₂ < 0` label corrected, per R-57."

## Acceptance tests
- Review (code): the occupant set is closed at build time; no code path compiles or interprets user-authored compute code (REQ-INT-005).
- Review (code, physics): `CapabilityProfile` exists with the listed fields for every composed occupant, and energy-drift interpretation, the reversibility diagnostic and `total_substeps_log2`/FTLE confidence read it — grep shows no occupant-name special cases outside the profile (REQ-INT-013).
- `cargo test -p kernel occupant_profiles` — each built occupant's advertised profile, `step_reversible` and `run_reversible` included, and default tier binding equals the Part 2 / Part 2a tables; controls: a table giving Yoshida-4 `step_reversible` false, or KDK `run_reversible` true, fails; Aarseth–Zare + time-transformed leapfrog is an allowed future occupant in Part 2a, not a v1 requirement, and is not built (R-162, R-168) (REQ-INT-014).
- Review (code): Euler is absent from tier defaults and production occupant lists and present only in the debug selection (REQ-INT-015).
- `cargo test -p kernel occupant_coefficients` — constants equal §3.2 to all printed digits; |w₀ + 2w₁ − 1| ≤ 3·10⁻¹⁵, REQ-INT-090's bound (RQ-405, option (a) applied meanwhile: the printed digits give 0.9999999999999998 in f64); Yoshida-6 w₀ = 1 − 2(w₁+w₂+w₃) (REQ-INT-037). *Was:* "w₀ + 2w₁ = 1" (code review 5587795260 C7, qa review 5587775086 A of PR #258).
- `cargo test -p kernel yoshida6_published` — the shared source's Yoshida-6 constants equal dd §3.2's printed digits, their block carrying the citation (Yoshida 1990, Table 1, solution A), and the Yoshida-4 block carries its citation (Yoshida 1990, the fourth-order closed form); REQ-INT-090's checks pass on them (REQ-INT-056).
- `cargo test -p kernel yoshida_convergence_order` — on the Kepler-embedded fixture, the maximum relative energy error over one orbit, `max_t |E(t) − E₀| / |E₀|`, fitted against `dt` over a log–log ladder: Yoshida-4's fitted order rounds to 4 and Yoshida-6's to 6, and the test prints the ladder, the fit and its detection floor (the smallest relative coefficient change it detects; estimated near 3·10⁻⁷ for Yoshida-4 and 10⁻⁹ for Yoshida-6); controls: Yoshida-4's `w₁` and Yoshida-6's `w₁`, `w₂`, `w₃`, each with its 4th significant digit changed and `w₀` recomputed, give a fitted order that rounds lower and fail (REQ-INT-090).
- `cargo test -p kernel yoshida_order_conditions` — evaluated exactly on the shipped f64 constants, the order conditions of a symmetric composition of a symmetric step, `|Σw − 1|`, `|Σw³| / Σ|w|³` and, for Yoshida-6, the two fifth-order conditions `|Σw⁵| / Σ|w|⁵` and the nested one (the coefficient of `[A₁,[A₁,A₃]]` in the BCH logarithm at `h⁵`, over the same coefficient with `|w|`; dd §5 test 1a), are at most 3·10⁻¹⁵ (Yoshida-4) and 5·10⁻¹⁴ (Yoshida-6); controls: each coefficient (Yoshida-6's `w₀` recomputed) moved by one unit of its 15th (Yoshida-4) or 13th (Yoshida-6) significant digit, in each direction, exceeds the bound and fails; the last printed digits are REQ-INT-037's equality (REQ-INT-090).
- `cargo test -p kernel step_reversibility` — dd test 3: STEP(dt), negate p, STEP(dt), negate p returns the state within REQ-VAL-191's rounding-level criterion for KDK, Yoshida-4 and Yoshida-6, without wrapper projection or substepping; control: Yoshida-4 with its stages made asymmetric (w₁, w₁, w₀) exceeds it and fails (REQ-VAL-032).
- Review (physics): the proposed rounding-level criterion with its evidence (the residuals for KDK, Yoshida-4 and Yoshida-6 at f64 and f32 across the dt used, and the asymmetric control's, scaled by the state norm); the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-VAL-191).
- Physics reviewer approves the Part 2 definition of what `order` means for the complexity proxy and FTLE confidence (REQ-INT-078).

## Notes
- R-57 is applied here: Yoshida-6 must not enter the shared source before the coefficient check passes; R-501 makes that check the citation and the convergence-order test.
- RQ-369 ruled: R-500 (option (c)) — the flag is split; nothing is left for this task to define. The rounding-level criterion is REQ-VAL-191, a calibration this task proposes; TASK-M3-08 measures the regularised occupants against it (REQ-VAL-190). The dt ladder is this task's, named in its PR (applied per R-369, R-501). *As first ported:* "The test 3 rounding-level criterion this task uses is the one TASK-M3-08 measures the regularised occupants against (REQ-VAL-190). The dt ladder and fit of the order test are this task's".
- RQ-405 (open): Yoshida-4's printed digits; this task builds them as printed, and REQ-INT-037 stays held (`rq: RQ-405`) when this task merges.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-078.
