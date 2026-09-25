# TASK-M3-12 — The escape detector: closure and E_rel, the window, the escaper and ionisation

- **Milestone:** M3
- **Closes:** REQ-EVT-005, REQ-EVT-006, REQ-EVT-011, REQ-EVT-015, REQ-EVT-016, REQ-EVT-017, REQ-EVT-018, REQ-EVT-024, REQ-EVT-026
- **Depends on:** TASK-M3-08, TASK-M3-11
- **Needs (earlier milestones):** REQ-SYS-016, REQ-PAY-004
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1.5, PIT-2, PIT-2.1, PIT-2.2
- **Size:** ~450 lines

## Goal
Escape is detected post-step on the projected state iff `|Δn̂|` over the 0.4-time-unit window `< tau` AND `E_rel > 0` — two conditions, no receding test, no `r_esc`, no `k_esc` — with `E_rel = ½|Δv|² − (M_pair + m_b)/d` about the other two's centre of mass (total mass, R-29), the window sampled on the macro-step grid for unregularised occupants and on the sync grid for regularised ones (R-95), the escaper the body with `E_rel > 0` and the largest `d` (R-61). Triple ejection is escape with `detail = 3`, gated on all pairwise energies positive, all separations growing and total `E > 0`, with the escape rule's settling (R-32). The settling test reads the lagged `n̂`, never `closure_min`. The branch inputs `|Δn̂|` and `E_rel` take rule 6's explicit-fma treatment (R-34). The provisional `tau` is **1e-3**: prin-rs's `CLOSURE_TAU` (`src/outcome.rs:263` at `8600d45`), inside the recorded range 7.04e-05 … 2.70e-02 (R-173); TASK-M3-34 replaces it with the confirmed value (REQ-EVT-025).

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
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-173 — Tau is split into a provisional and a confirmed value *(closes C2, C3, S3, G6)*"
- `decisions.md` § "R-182 — Escape fixtures are defined; proposed tolerances are provisional in CI *(closes T4, T5)*"

## Deliverables
- `crates/kernel/src/detect/escape.rs` — `e_rel(b)`, `escaper()`, the window test on the lagged register, the ionisation gate.
- Detector signature takes state + params only (no `r_esc`, receding or `k_esc` inputs).
- Doc follow-through: integrator contract Part 7's ionisation line updated to R-32's gate if not already.
- `fixtures/escape/` — the seven fixtures of "Fixture definitions" below, with their parameters and expected outcomes.
- `crates/kernel/src/detect/escape.rs` — `TAU_PROVISIONAL = 1e-3`, commented with its source (prin-rs `CLOSURE_TAU`, R-173) and "provisional until REQ-EVT-025".

## Acceptance tests
- `cargo test -p kernel escape_detector` — dd test 5: a synthetic hyperbolic ejection fires once |Δn̂| < tau with E_rel > 0; a grazing near-escape never fires; a settled bound hierarchy (E_rel < 0) never fires; escaper id = largest-d body (REQ-EVT-005).
- `cargo test -p kernel ionisation_gate` — a synthetic three-way unbound E > 0 state fires escape detail = 3; a state with one bound pair does not (REQ-EVT-006).
- Review (physics): the settling test does not read closure_min (REQ-EVT-011).
- `cargo test -p kernel escape_two_conditions` — settled hierarchical escaper fires; bound hierarchy with settled shape but E_rel < 0 does not; mid-encounter flicker with E_rel > 0 but |Δn̂| ≥ tau does not; the detector has no r_esc/receding/k_esc inputs (REQ-EVT-015).
- `cargo test -p kernel e_rel_total_mass` — unequal-mass fixture where the M_pair-only form gives E_rel > 0 and the total-mass form gives E_rel < 0: the detector returns the total-mass value and does not fire (REQ-EVT-016).
- `cargo test -p kernel escape_window_grid` — for KDK/Yoshida the detector compares n̂ at the current macro-step boundary with n̂ held from 0.4 time units earlier on the macro-step grid; for AZ, Heggie and logH the same on the sync grid, for several n_sync values (REQ-EVT-017).
- `cargo test -p kernel escaper_largest_d` — two bodies with E_rel > 0 differing in d: the larger-d body (to the other two's CoM) is reported as escaper and in detail (REQ-EVT-018).
- `cargo test -p kernel escape_tau_provisional` — the detector's tau is 1e-3 and lies inside 7.04e-05 … 2.70e-02; the constant's comment cites its source (REQ-EVT-024).
- Review checklist (physics §9): the fixture definitions below give every parameter and expected outcome, and each fixture's control is discriminating (REQ-EVT-026).


## Fixture definitions (R-182; an R-72 definition, physics-reviewed)
Units G = 1. Each fixture is an initial state in the centre-of-mass frame (positions r_i, velocities v_i, masses m_i), marched with the default occupant (Heggie) to t_end = 20 unless stated; the window is 0.4 time units (R-29) and tau is the provisional 1e-3 (REQ-EVT-024). "Binary(a)" is bodies 0 and 1 on a circular orbit of separation a about their own centre of mass, in the plane, relative speed √((m₀+m₁)/a). The third body's d and E_rel are measured to the pair's centre of mass (R-29, R-61); a stated speed is relative to that centre of mass, and the state is then shifted into the system's centre-of-mass frame.
1. `hyperbolic_ejection` — m = (1, 1, 1); Binary(1); body 2 at d = 20, moving radially away at speed √1.3 (E_rel = +0.5). Expected: escape fires once |Δn̂| < tau; escaper 2; `detail` names body 2.
2. `grazing_near_escape` — as 1, but body 2's speed √0.28 (E_rel = −0.01). Expected: never fires.
3. `settled_bound_hierarchy` — m = (1, 1, 1); Binary(1); body 2 on a circular outer orbit at d = 10 (speed √(3/10)). Expected: |Δn̂| settles below tau, E_rel < 0; never fires.
4. `mid_encounter_flicker` — m = (1, 1, 1); Binary(1); body 2 at d = 1.5 moving through the binary's centre of mass with E_rel = +0.2 (speed √4.4); t_end = 1. Expected: |Δn̂| ≥ tau throughout the window; does not fire.
5. `unequal_mass_e_rel` — m = (1, 1, 2); Binary(1); body 2 at d = 10 moving radially away at speed √0.6 (½v² = 0.3: the M_pair-only form 0.3 − 0.2 > 0; the total-mass form 0.3 − 0.4 < 0). Expected: the detector returns E_rel = −0.1 and does not fire.
6. `two_candidate_escaper` — m = (1, 1, 1); body 0 at the origin; bodies 1 and 2 on opposite sides moving radially away, at d = 15 and d = 25 from the other two's centre of mass, each with E_rel = +0.3 at t = 0. Expected: escaper is the larger-d body (2), in `escaper` and in `detail`.
7. `three_way_unbound` — m = (1, 1, 1); an equilateral triangle of side 10 about the centre of mass, each body moving radially outward at speed 1 (every pairwise energy ½·½·3 − 1/10 > 0; total E = 1.2 > 0). Expected: escape fires with `detail = 3` after the settling test. Its control, `one_bound_pair`: Binary(1) with body 2 at d = 20 moving away at speed √2.3 (E_rel = +1.0; total E = −0.5 + (2/3)(1.0) ≈ +0.17 > 0, one bound pair): `detail` ≠ 3.
The fixture files are `fixtures/escape/<name>.toml`, each holding these parameters and the expected outcome.

## Notes
- The window of 0.4 time units is provisional (R-29).
- R-173: this task states the provisional tau (REQ-EVT-024); the confirmed tau, inside REQ-VAL-051's re-measured gap, is REQ-EVT-025, closed by TASK-M3-34.
- Closes, for gaps the corpus leaves open: REQ-EVT-026 (R-72 definition: the fixtures, R-182).
