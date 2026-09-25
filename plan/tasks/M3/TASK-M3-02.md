# TASK-M3-02 — Stepper occupants and the capability profile

- **Milestone:** M3
- **Closes:** REQ-INT-005, REQ-INT-013, REQ-INT-014, REQ-INT-015, REQ-INT-037, REQ-INT-056, REQ-INT-078, REQ-VAL-032
- **Depends on:** TASK-M3-01
- **Needs (earlier milestones):** REQ-INT-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The five stepper occupants — Euler (debug only), KDK, Yoshida-4, Yoshida-6 and RK4 — exist as build-time Rust variants implementing `STEP` with exactly dd_integrator §3.2's update rules and coefficients, each advertising its capability profile `{order, force_evals, symplectic, reversible, owns_time_mapping}` (the re-registration field is added by TASK-M3-08). Yoshida-6's coefficients are checked against Yoshida (1990) Table 1 solution A and the `w₂ < 0` label fixed before they enter the shared source (R-57). Integrator contract Part 2 gains the definition of how `order` sets the meaning of the `total_substeps_log2` proxy and FTLE confidence (R-72).

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

## Deliverables
- `crates/kernel/src/occupant/mod.rs` — the `Stepper` trait (`fn step(state, dt, geom)`), `CapabilityProfile` struct, and the closed, build-time occupant set (an enum of monomorphised types; no runtime compilation or interpretation path).
- `crates/kernel/src/occupant/{euler,kdk,yoshida4,yoshida6,rk4}.rs` — §3.2's rules; Yoshida-4 and -6 as KDK compositions with the printed drift/kick sequences.
- `crates/kernel/src/occupant/table.rs` — the Part 2 / Part 2a profile table, including the two AZ rows' profiles (filled by TASK-M3-07), and the default tier binding per occupant; Euler reachable only from a debug selection.
- Doc change: `docs/contracts/principia_integrator_contract.md` Part 2 — the mapping from profile `order` to the interpretation of `total_substeps_log2` and to FTLE confidence (definition, REQ-INT-078).
- Doc change: `docs/design/principia_dd_integrator.md` §3.2 / §6 — the Yoshida-6 transcription check recorded (the digits compared with Yoshida 1990 Table 1 solution A) and the `w₂ < 0` label corrected, per R-57.

## Acceptance tests
- Review (code): the occupant set is closed at build time; no code path compiles or interprets user-authored compute code (REQ-INT-005).
- Review (code, physics): `CapabilityProfile` exists with the listed fields for every composed occupant, and energy-drift interpretation, the reversibility diagnostic and `total_substeps_log2`/FTLE confidence read it — grep shows no occupant-name special cases outside the profile (REQ-INT-013).
- `cargo test -p kernel occupant_profiles` — each built occupant's advertised profile and default tier binding equals the Part 2 / Part 2a tables; Aarseth–Zare + time-transformed leapfrog is an allowed future occupant in Part 2a, not a v1 requirement, and is not built (R-162, R-168) (REQ-INT-014).
- Review (code): Euler is absent from tier defaults and production occupant lists and present only in the debug selection (REQ-INT-015).
- `cargo test -p kernel occupant_coefficients` — constants equal §3.2 to all printed digits; w₀ + 2w₁ = 1; Yoshida-6 w₀ = 1 − 2(w₁+w₂+w₃) (REQ-INT-037).
- `cargo test -p kernel yoshida6_published` — Yoshida-6 coefficients equal the published digits of Yoshida (1990) Table 1 solution A, and the composition shows a 6th-order convergence slope on a Kepler fixture (REQ-INT-056).
- `cargo test -p kernel step_reversibility` — dd test 3: STEP(dt), negate p, STEP(dt), negate p returns the state to rounding level for KDK, Yoshida-4 and Yoshida-6, without wrapper projection or substepping (REQ-VAL-032).
- Physics reviewer approves the Part 2 definition of what `order` means for the complexity proxy and FTLE confidence (REQ-INT-078).

## Notes
- R-57 is applied here: Yoshida-6 must not enter the shared source before the coefficient check passes.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-078.
