# TASK-M4-15 — The closure field on the Config α×β z_q = 0 plane: free-fall orbits light up

- **Milestone:** M4
- **Closes:** REQ-VAL-076, REQ-VAL-137, REQ-VAL-074, REQ-PAY-074, REQ-TOOL-044
- **Depends on:** TASK-M4-10, TASK-M4-06, TASK-M3-29
- **Needs (earlier milestones):** REQ-PAY-043, REQ-PAY-056, REQ-VAL-055, REQ-VAL-042, REQ-VAL-126, REQ-CHART-030, REQ-VAL-048, REQ-GUI-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~400 lines

## Goal
The self-validating render of dd_validation_orbits §6.1: the closure field on the Config α×β chart's z_q = 0 plane (free-fall starts at rest) shows low closure where the known free-fall periodic orbits sit. The threshold at which they count as lit is calibrated (R-71). The precision-dependent closure floor (~1e-7 on the f32 GPU path, ~1e-16 on the f64 CPU path) is reported as a saturation limit, not a lost orbit; the places it is reported are defined in the payload doc (R-72). The f32 field finds orbits but never ranks them: any orbit candidate routes through the f64 inspector's closure test.

## References
- `docs/design/principia_dd_validation_orbits.md` § "6.1 Why rendering beats searching"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_validation_orbits.md` § "6.4 Two cautions"

## Deliverables
- `fixtures/golden/closure-freefall/`: the z_q = 0 plane render and the known orbit locations (encoded from REQ-VAL-126's fixture set via Path A).
- `xtask` gate `closure-threshold`: closure read at the known locations against the background; the proposed threshold with its evidence.
- `crates/render` / `crates/engine`: the closure readout labelled with its precision path and floor; the orbit-candidate path requiring the f64 closure test.
- Doc change: `docs/design/principia_dd_simstate_payload.md` (closure) — where the floor is reported (readout, export, legend), with the "Removed lines" note.

## Acceptance tests
- `cargo xtask golden closure-freefall` — render the z_q = 0 plane; the known free-fall orbit locations light up at the threshold REQ-VAL-137 (calibrated) (REQ-VAL-076).
- `cargo xtask gate closure-threshold` — renders the plane, reads closure at the known orbit locations against the background and states the threshold; recorded in decisions.md after the human confirms it at the M4 gate (REQ-VAL-137, calibrated).
- `cargo test -p render closure_floor_label` plus physics review — closure readouts carry the precision path and its floor; a 1e-9 reference orbit reading ~1e-7 on the GPU is labelled saturated (REQ-VAL-074).
- Physics reviewer on the doc change — the payload doc names each place the floor is reported; every closure display or export path shows the floor for its precision (REQ-PAY-074).
- `cargo test -p engine orbit_candidate_requires_f64` plus code review — any orbit-candidate report routes through the f64 inspector closure test; nothing ranks by f32 closure (REQ-TOOL-044).

## Notes
- REQ-VAL-137 is a calibration (R-71): the PR carries the proposed value, its evidence and the reviewer's check, marked pending; the human confirms it at the M4 gate and it is then recorded in decisions.md.
- REQ-PAY-074 is a definition (R-72): the doc change is part of this PR and needs the physics reviewer's approval before merge.
- The threshold is relative to the observed background distribution, never an absolute cutoff by eye (pitfalls §3: closure spans decades).
- Export paths arrive in M7/M8; the definition written here governs them.
