# TASK-M3-33 — The validation harness's post-escape march and pitfalls §2.4's checks

- **Milestone:** M3
- **Closes:** REQ-VAL-054, REQ-VAL-123, REQ-VAL-124, REQ-EVT-013
- **Depends on:** TASK-M3-24, TASK-M3-18, TASK-M0-03
- **Needs (earlier milestones):** REQ-VAL-005, REQ-VAL-007
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1.5, PIT-2.4, PIT-3
- **Size:** ~450 lines

## Goal
R-103 is built: `crates/validation` marches past escape on its own copy of the state — the production payload never sees it — and runs pitfalls §2.4's checks on the corrected detector: check 1 recorded as passed from prin-rs; the 0-of-895 style re-bind test (+1, +2, +3, +4 and +8 sync boundaries past firing, original discretisation); check 2, independent ground truth (separation growing without bound); check 3 on the deep-interior chart. Each check's pass threshold, horizon and fixture are calibrated (R-71).

## References
- `decisions.md` § "R-31 — Escape doesn't terminate until pitfalls §2.4's three checks pass *(IE-3)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/read_first/principia_01_pitfalls.md` § "2.4 Should it terminate?"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-95 — After escape fires *(closes RQ-48, in part)*"
- `decisions.md` § "R-103 — Escape ends the production loop; the §2.4 checks run in the harness *(closes RQ-63 and RQ-69)*"
- `docs/read_first/principia_01_pitfalls.md` § "1.5 The root cause underneath the root cause"
- `decisions.md` § "R-166 — The fixtures"

## Deliverables
- `crates/validation/src/escape_checks/march.rs` — the harness march: clones the state at the firing step, integrates with the same discretisation (TOOL-038's guard), never writes the payload buffer.
- `crates/validation/src/escape_checks/{rebind,check2,check3}.rs` and `xtask gate escape-checks`.
- The unit fixture of a close encounter where E_rel goes briefly positive while receding and closure has not settled.
- Calibration proposals for check 2 and check 3 (threshold, horizon, fixture set; the deep-interior chart region).

## Acceptance tests
- `cargo xtask gate escape-checks` — recorded results of check 2 and check 3 on the corrected detector (thresholds: REQ-VAL-123, REQ-VAL-124, calibrated), check 1 recorded as passed from prin-rs; the harness's §2.4 march leaves the payload buffer byte-identical (REQ-VAL-054).
- `cargo xtask gate escape-checks --propose check2` — the separation-growth test, its horizon and the fixture set, with the check's result on the corrected detector; the human confirms at the M3 gate and it is recorded in decisions.md (REQ-VAL-123).
- `cargo xtask gate escape-checks --propose check3` — the deep-interior chart region, the horizon and the pass threshold, with the check's result on the corrected detector; the human confirms at the M3 gate and it is recorded in decisions.md (REQ-VAL-124).
- `cargo xtask gate escape-rebind` — every trajectory that fires escape on a fixture slice, integrated +1, +2, +3, +4 and +8 sync boundaries past firing with the original discretisation: none re-bind; and `cargo test -p kernel escape_not_transient` — the close-encounter fixture (E_rel briefly positive, closure unsettled) does not fire (REQ-EVT-013).

## Notes
- This task also carries the harness byte-identity arm that REQ-EVT-010 and REQ-PAY-058 (TASK-M3-17) name.
- A discriminator must not come from a quantity the early stop alters (pitfalls §3; REQ-VAL-005).
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-123, REQ-VAL-124.
- RQ-103 ruled: R-166 — the named slices, `deep interior` among them, are in `fixtures/slices.toml`.
