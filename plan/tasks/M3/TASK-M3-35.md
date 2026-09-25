# TASK-M3-35 — The patchwork regression and the stop_on_escape images

- **Milestone:** M3
- **Closes:** REQ-EVT-012, REQ-EVT-014, REQ-EVT-023, REQ-VAL-050
- **Depends on:** TASK-M3-33, TASK-M3-22
- **Needs (earlier milestones):** REQ-VAL-003, REQ-VAL-012, REQ-TOOL-014
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1, PIT-1.3, PIT-1.4, PIT-1.6, PIT-1.7
- **Size:** ~400 lines

## Goal
Pitfalls §1 stays fixed: the controlled experiment renders the config slice once integrating every pixel to t_max and once with the shipping termination behaviour, one variable changed, and the images show the same continuous ribbons with no domes, tents, wedges or seams; the t_end histogram of escape-terminated pixels is not ~32 strata. The stop_on_escape on/off comparison — the 'off' image coming from the validation harness's march (RQ-78) — gives near-identical images. Both golden tolerances are calibrated, and the artefact-triage checklist carries §1.7's signatures.

## References
- `docs/read_first/principia_01_pitfalls.md` § "1. THE PATCHWORK — the longest-running one, and the most instructive"
- `docs/read_first/principia_01_pitfalls.md` § "1.4 The actual bug"
- `docs/read_first/principia_01_pitfalls.md` § "1.3 What settled it — a controlled experiment, not an argument"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/read_first/principia_01_pitfalls.md` § "1.6 The fix — see §2 for the criterion"
- `decisions.md` § "R-95 — After escape fires *(closes RQ-48, in part)*"
- `docs/read_first/principia_01_pitfalls.md` § "1.7 What to recognise next time"
- `docs/read_first/principia_01_pitfalls.md` § "1.1 What was observed"
- `decisions.md` § "R-103 — Escape ends the production loop; the §2.4 checks run in the harness *(closes RQ-63 and RQ-69)*"
- `decisions.md` § "R-148 — The validation harness renders the "off" image *(closes RQ-78)*"

## Deliverables
- `xtask golden patchwork` and `xtask golden stop-on-escape` over CPU payloads of the config slice, the 'integrate everything' and 'off' images produced by `crates/validation`'s harness march.
- Seam-structure check on the difference image; t_end histogram.
- `crates/validation/TRIAGE.md` — the artefact triage checklist with the three §1.7 signatures and the t_end-level-set and independent-occupant checks.
- Calibration proposal for both golden tolerances.

## Acceptance tests
- `cargo xtask golden patchwork` — every pixel to t_max vs the shipping termination, one variable changed: the same continuous ribbons, no domes, tents, wedges or clean seams (per-pixel diff within the golden tolerance, REQ-EVT-023 calibrated); t_end of escape-terminated pixels is not ~32 discrete strata (REQ-EVT-012).
- `cargo xtask golden stop-on-escape` — the same slice with stop_on_escape on (production) and off (the harness march): per-pixel colour difference within the golden tolerance (REQ-EVT-023 calibrated) and no seam structure in the difference image (REQ-EVT-014).
- `cargo xtask golden stop-on-escape --propose` — per-pixel differences on the config slice measured for both experiments, the tolerance stated with a seam-structure check; the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-EVT-023).
- Review (qa, physics): the artefact triage checklist includes the three §1.7 signatures; for any flagged image t_end is rendered to test whether the boundaries are its level sets, and the independent occupant is run on the same ICs to compare tame vs wild regions (REQ-VAL-050).

## Notes
- RQ-78 ruled: R-148 — the "off" image is rendered by the validation harness, whose own march continues past escape, and the regression (REQ-EVT-014, tolerance REQ-EVT-023) stands there. This task was written for that reading.
- Gap: 'the config slice' / `config_stability` are not defined as chart + parameters in the corpus.
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-EVT-023.
- Waits on RQ-103 (`REVIEW_QUEUE.md`): The prin-rs fixtures and slices the M3 re-runs need.
