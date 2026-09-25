# TASK-M3-12 — The escape detector: closure and E_rel, the window, the escaper and ionisation

- **Milestone:** M3
- **Closes:** REQ-EVT-005, REQ-EVT-006, REQ-EVT-011, REQ-EVT-015, REQ-EVT-016, REQ-EVT-017, REQ-EVT-018
- **Depends on:** TASK-M3-08, TASK-M3-11
- **Needs (earlier milestones):** REQ-SYS-016, REQ-PAY-004
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1.5, PIT-2, PIT-2.1, PIT-2.2
- **Size:** ~450 lines

## Goal
Escape is detected post-step on the projected state iff `|Δn̂|` over the 0.4-time-unit window `< tau` AND `E_rel > 0` — two conditions, no receding test, no `r_esc`, no `k_esc` — with `E_rel = ½|Δv|² − (M_pair + m_b)/d` about the other two's centre of mass (total mass, R-29), the window sampled on the macro-step grid for unregularised occupants and on the sync grid for regularised ones (R-95), the escaper the body with `E_rel > 0` and the largest `d` (R-61). Triple ejection is escape with `detail = 3`, gated on all pairwise energies positive, all separations growing and total `E > 0`, with the escape rule's settling (R-32). The settling test reads the lagged `n̂`, never `closure_min`. The branch inputs `|Δn̂|` and `E_rel` take rule 6's explicit-fma treatment (R-34).

## References
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/design/principia_dd_integrator.md` § "3.6 Detectors (per `STEP`, on the projected state)"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `decisions.md` § "R-29 — The escape criterion's undefined parts *(IE-1, amended)*"
- `decisions.md` § "R-61 — The escaper's separation is its distance to the other two's centre of mass *(amends R-29)*"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/design/principia_dd_simstate_payload.md` § "VERTICAL-SLICE ADDITIONS"
- `decisions.md` § "R-95 — After escape fires *(closes RQ-48, in part)*"
- `decisions.md` § "R-32 — The ionisation gate is pairwise-unbound, separating and total `E > 0`, with settling *(IE-4 (c))*"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `docs/read_first/principia_01_pitfalls.md` § "2.2 The criterion"
- `docs/read_first/principia_01_pitfalls.md` § "2.1 Escape is a limit, not a threshold"
- `docs/read_first/principia_01_pitfalls.md` § "2. THE ESCAPE CRITERION — what replaced it, and why"
- `open-questions.md` § "Open questions"
- `decisions.md` § "R-34 — FMA: explicit fma at every branch input, enumerated *(IE-6)*"

## Deliverables
- `crates/kernel/src/detect/escape.rs` — `e_rel(b)`, `escaper()`, the window test on the lagged register, the ionisation gate.
- Detector signature takes state + params only (no `r_esc`, receding or `k_esc` inputs).
- Doc follow-through: integrator contract Part 7's ionisation line updated to R-32's gate if not already.
- `fixtures/escape/` — synthetic hyperbolic ejection, grazing near-escape, settled bound hierarchy, mid-encounter flicker, unequal-mass E_rel fixture, two-candidate escaper fixture, three-way unbound state.

## Acceptance tests
- `cargo test -p kernel escape_detector` — dd test 5: a synthetic hyperbolic ejection fires once |Δn̂| < tau with E_rel > 0; a grazing near-escape never fires; a settled bound hierarchy (E_rel < 0) never fires; escaper id = largest-d body (REQ-EVT-005).
- `cargo test -p kernel ionisation_gate` — a synthetic three-way unbound E > 0 state fires escape detail = 3; a state with one bound pair does not (REQ-EVT-006).
- Review (physics): the settling test does not read closure_min (REQ-EVT-011).
- `cargo test -p kernel escape_two_conditions` — settled hierarchical escaper fires; bound hierarchy with settled shape but E_rel < 0 does not; mid-encounter flicker with E_rel > 0 but |Δn̂| ≥ tau does not; the detector has no r_esc/receding/k_esc inputs (REQ-EVT-015).
- `cargo test -p kernel e_rel_total_mass` — unequal-mass fixture where the M_pair-only form gives E_rel > 0 and the total-mass form gives E_rel < 0: the detector returns the total-mass value and does not fire (REQ-EVT-016).
- `cargo test -p kernel escape_window_grid` — for KDK/Yoshida the detector compares n̂ at the current macro-step boundary with n̂ held from 0.4 time units earlier on the macro-step grid; for AZ, Heggie and logH the same on the sync grid, for several n_sync values (REQ-EVT-017).
- `cargo test -p kernel escaper_largest_d` — two bodies with E_rel > 0 differing in d: the larger-d body (to the other two's CoM) is reported as escaper and in detail (REQ-EVT-018).

## Notes
- Gap reported: no value for `tau` is given (Part 3: '—', 'to re-measure'); REQ-VAL-051 sets it by measurement in TASK-M3-34, but this detector and everything downstream needs a value first, and no calibration requirement covers it.
- The window of 0.4 time units is provisional (R-29).
