# TASK-M3-32 — Re-registration: the Heggie default gate, the control, the doubling protocol and the logH falsification

- **Milestone:** M3
- **Closes:** REQ-INT-050, REQ-INT-051, REQ-VAL-052, REQ-VAL-115, REQ-VAL-136
- **Depends on:** TASK-M3-23, TASK-M3-14, TASK-M3-31, TASK-M0-03
- **Needs (earlier milestones):** REQ-VAL-006, REQ-VAL-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4, PIT-4.1, PIT-4.2, PIT-4.3, PIT-4.4, PIT-4.5
- **Size:** ~450 lines

## Goal
The re-registration mechanism's measurements run through the comparison harness: the regularisation-free control (its drift field correlates positively with FTLE, a re-registering occupant's does not); the Heggie-vs-AZ comparison that makes Heggie the default; the doubling protocol (re-registration ×2 at fixed step, eta adjusted so steps p50 stays flat within 6%) measured for the default occupant against the controls, with its acceptance level calibrated; and the logH falsification check under the same RK4 and step control, its verdict written to pitfalls §4.4 (R-74).

## References
- `docs/read_first/principia_01_pitfalls.md` § "4.3 The control that found it"
- `docs/read_first/principia_01_pitfalls.md` § "4.4 The remedy, and its own falsification test"
- `docs/read_first/principia_01_pitfalls.md` § "4.1 The finding"
- `docs/read_first/principia_INDEX.md` § "The evidence base — where settled defaults were measured"
- `docs/design/principia_dd_integrator.md` § "Drill-down — the Integrator (Physics rung)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `docs/read_first/principia_01_pitfalls.md` § "4.2 What it caused"
- `docs/read_first/principia_01_pitfalls.md` § "4. RE-REGISTRATION COUNT — the mechanism that had to be found twice"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-74 — The research phases are settled by the vertical slice *(closes RQ-25)*"
- `docs/read_first/principia_00_philosophy.md` § "7.8 Sequencing — what is next, and why in this order"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"
- `decisions.md` § "R-164 — +0.305 and −0.082 are correlations, not controls"
- `decisions.md` § "R-165 — The 32-case figure is 3915 → 74"
- `decisions.md` § "R-161 ✱ — Heggie's default time transformation is the measured one, Eq. 22 at n = 3/2"
- `decisions.md` § "R-166 — The fixtures"

## Deliverables
- `crates/validation/src/gates/reregistration.rs` — control correlation, Heggie-vs-AZ matrix, doubling protocol, logH-vs-Heggie matrix, each through `compare()`.
- Doc change: `docs/read_first/principia_01_pitfalls.md` §4.4 — the logH verdict (mechanism supported, or recorded wrong).
- Calibration proposal for the default occupant's acceptance level under doubled re-registration.

## Acceptance tests
- `cargo test -p validation unregularised_control` — the occupant registry includes an unregularised (leapfrog) occupant sharing no coordinate machinery with the others; no gate rests on the FTLE–drift Spearman correlations (+0.305 leapfrog, −0.082 AZ, a null against its shifted control), which are prior findings, not controls (R-164) (REQ-INT-050).
- `cargo xtask gate heggie-vs-az` — on the 32-case matrix (`fixtures/case_matrix.toml`), Heggie wins on the recorded cases (31 of 32; err>10, `error_ratio > 10`, 3915 → 74 at prin-rs `8600d45`, the original run at `70cfbc4` giving 3916 → 73, R-165) with AZ winning only on 'far'; the default occupant config names Heggie, and the re-run confirms its default time transformation, Eq. 22 at n = 3/2 (R-161) (REQ-INT-051).
- `cargo xtask gate reregistration-doubling` — double the sync-boundary re-registration count with eta adjusted so steps p50 stays flat within 6%; record the drift-field change in decades against the controls (LC branch 2.5e-6, hysteresis 7.5e-5, AZ ×2 4.4e-1); render config_stability and assert no pale straight-edged wedges (acceptance level: REQ-VAL-136, calibrated) (REQ-VAL-052).
- `cargo xtask gate logh-falsification` — logH and Heggie on the same case matrix with the same RK4 and step control; per-case err and whether logH matches or beats Heggie recorded; the verdict written to pitfalls §4.4 (REQ-VAL-115).
- `cargo xtask gate reregistration-doubling --propose` — the default occupant (Heggie) measured under the doubling protocol, the acceptance level stated relative to the controls; the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-VAL-136).

## Notes
- *Was: "Gap: the 32-case Heggie-vs-AZ matrix, the 'err' metric behind 'err>10', the control's fixture ICs and the named slices ('far', `config_stability`) are prin-rs artefacts not defined in the corpus."* RQ-103 ruled: the matrix and `err>10` (`error_ratio > 10`, prin-rs `stats.rs:71-88`) are `fixtures/case_matrix.toml`, the slices `fixtures/slices.toml` (R-166); there are no control ICs — +0.305 and −0.082 are correlations, recorded as prior findings with no gate (R-164).
- Heggie has no re-registration; the doubling protocol's meaning for the default occupant is itself something the proposal must state.
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-136.
