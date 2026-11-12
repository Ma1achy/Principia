# TASK-M4-13 — FTLE on the GPU: the f32 shadow form and the horizon map

- **Milestone:** M4
- **Closes:** REQ-PAY-085, REQ-TOOL-043, REQ-TOOL-126, REQ-GUI-437
- **Depends on:** TASK-M4-06, TASK-M4-04, TASK-M3-36, TASK-M3-72, TASK-M3-73
- **Needs (earlier milestones):** REQ-INT-043, REQ-INT-076, REQ-INT-077, REQ-VAL-038, REQ-PAY-021, REQ-PAY-032, REQ-VAL-035, REQ-RENDER-013, REQ-TOOL-009, REQ-GUI-438
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~350 lines

## Goal
The Benettin FTLE as the GPU computes it. The f32 shadow is run in both forms — absolute and displacement — against known-Lyapunov periodic orbits, and the FTLE-accuracy improvement that would justify switching to displacement is proposed with its evidence (R-71). The instrument declares its predictability horizon: a horizon map `t_max(IC) = ln(1/eps)/ftle(IC)` computed from the Benettin FTLE, never the ensemble spread, with the finite-time-λ caveat carried wherever the figure appears.

## References
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_predictability_horizon.md` § "4.2 The horizon is a field, not a constant"
- `docs/design/principia_dd_predictability_horizon.md` § "4.3 It bounds what the renderer may claim"
- `docs/design/principia_dd_predictability_horizon.md` § "5. Theoretical framing, and its use in the papers"
- `docs/read_first/principia_00_philosophy.md` § "4.1 The instrument must say where it stops knowing"
- `docs/design/principia_dd_predictability_horizon.md` § "4.1 The two kernels have different horizons"
- `docs/design/principia_dd_predictability_horizon.md` § "1. The result"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-976 — RQ-794, what REQ-INT-076 proposes for the FTLE shadow's ‖δ₀‖ and `n_renorm`: option (c), (3e-3, 300, 0.2) stay provisional and stated as unsupported on close-encounter chaotic orbits wherever the FTLE is shown; the pre-registered eight-orbit calibration is re-run unchanged, then `min_regions` ≥ 6 proposed; the M3 gate confirms no final value until the re-run *(closes RQ-794)*"

## Deliverables
- `crates/kernel`: the displacement-form shadow as a build-time alternative for the measurement (the shipped form unchanged until the ruling).
- `xtask` gate `ftle-shadow-form`: both forms against the known-Lyapunov orbits of REQ-VAL-038's fixture; the proposed improvement threshold with its evidence.
- `crates/render`: the horizon-map view as a derived read-side quantity from `ftle` (NaN when `has_ftle` is false), with the caveat text in its legend.

## Acceptance tests
- `cargo xtask gate ftle-shadow-form` — FTLE of both shadow forms against known-Lyapunov periodic orbits, and the improvement the switch requires, proposed with evidence, checked by the physics reviewer and confirmed by the human at the M4 gate (REQ-PAY-085, calibrated).
- `cargo test -p render horizon_map` — the horizon map equals `ln(1/eps)/ftle` per sample; no code path feeds it ensemble spread; the finite-time-λ caveat travels with the view (REQ-TOOL-043).
- Definition: the horizon map's eps written into dd_predictability_horizon §4.2 and approved by the physics reviewer (REQ-TOOL-126).
- `cargo test -p render ftle_provisional_stated` — while REQ-INT-076's values are unconfirmed, the horizon map's view and its legend state the calibration (3e-3, 300, 0.2) as provisional and unsupported on close-encounter chaotic orbits, from REQ-GUI-438's source; controls: the view or legend without it, and the statement kept once the values are confirmed, each fail (REQ-GUI-437). *Was:* "every view of `ftle`, the horizon map's legend and the export header state the calibration (3e-3, 300, 0.2) as provisional and unsupported on close-encounter chaotic orbits, from one source; controls: a view without it".

## Notes
- REQ-PAY-085 is a calibration (R-71): the PR carries the proposed value, its evidence and the reviewer's check, marked pending; the human confirms it at the M4 gate and it is then recorded in decisions.md.
- Gap: which `eps` the horizon map uses is not stated by the corpus.
- Closes, for gaps the corpus leaves open: REQ-TOOL-126 (R-72 definition) (classification accepted by R-132).
- Applied per R-369 (12 Nov 2026), a consequence of REQ-VAL-038's move from TASK-M3-14 to TASK-M3-72: TASK-M3-72 joins Depends on, since this task needs REQ-VAL-038 and builds its displacement-form shadow to the definition RQ-793 rules there. *Was:* "Depends on: TASK-M4-06, TASK-M4-04, TASK-M3-36".
- *12 Nov 2026 (R-976, RQ-794 (c); applied per R-369: task scoping):* Closes gains REQ-GUI-437, the provisional status of the FTLE shadow's calibration stated wherever the FTLE is shown; this is the first task that puts the FTLE in a view and a legend, and later views read the same statement. *Was:* "Closes: REQ-PAY-085, REQ-TOOL-043, REQ-TOOL-126".
- *12 Nov 2026, later (R-976; code review 5875707244 of PR #503, F4; applied per R-369: task scoping):* REQ-GUI-437 is narrowed to the horizon map and its legend, the other surfaces each closed by the task that builds them, and the one source is REQ-GUI-438's (TASK-M3-73), which joins Depends on; the `ftle_provisional_stated` line now names the horizon map's view and legend. *Was:* "Depends on: TASK-M4-06, TASK-M4-04, TASK-M3-36, TASK-M3-72"; "Needs (earlier milestones): REQ-INT-043, REQ-INT-076, REQ-INT-077, REQ-VAL-038, REQ-PAY-021, REQ-PAY-032, REQ-VAL-035, REQ-RENDER-013, REQ-TOOL-009".
