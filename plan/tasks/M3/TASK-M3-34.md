# TASK-M3-34 — Escape re-validation: precision, recall and the tau gap

- **Milestone:** M3
- **Closes:** REQ-VAL-040, REQ-VAL-051, REQ-EVT-019
- **Depends on:** TASK-M3-33
- **Needs (earlier milestones):** REQ-VAL-004, REQ-VAL-006, REQ-VAL-135
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2.2, PIT-2.3, PIT-3
- **Size:** ~350 lines

## Goal
The escape criterion is re-validated with R-29's `E_rel` against check 2's independent ground truth, with the legacy t = 30 set kept as a comparison: precision, recall and the tau gap are recorded for both. `tau` is set inside the measured gap between escapers' and bound trajectories' `|Δn̂|`, not tuned, with outcomes unchanged across the middle of the gap. A variant firing earlier with any false positives fails (late rather than wrong).

## References
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/design/principia_dd_integrator.md` § "3.6 Detectors (per `STEP`, on the projected state)"
- `decisions.md` § "R-29 — The escape criterion's undefined parts *(IE-1, amended)*"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"
- `docs/read_first/principia_01_pitfalls.md` § "2.2 The criterion"
- `open-questions.md` § "Open questions"
- `decisions.md` § "R-95 — After escape fires *(closes RQ-48, in part)*"
- `docs/read_first/principia_01_pitfalls.md` § "2.4 Should it terminate?"
- `docs/read_first/principia_01_pitfalls.md` § "2.3 The 96.3% recall is the right failure direction"

## Deliverables
- `crates/validation/src/gates/escape_revalidation.rs` — precision/recall against both ground truths; `|Δn̂|` distributions for t = 25–30; the gap ratio.
- The chosen tau with its gap evidence, written where integrator contract Part 3 and pitfalls §2.2 state 'to re-measure'.

## Acceptance tests
- `cargo xtask gate escape-revalidation` — re-run against check 2's ground truth and the legacy t = 30 set; precision, recall (previously 100% / 96.3%) and the tau gap (previously 383×) recorded for both (REQ-VAL-040).
- `cargo xtask gate tau-gap` — |Δn̂| distributions of escapers and bound trajectories across t = 25–30; the gap ratio recorded (claimed 383×, 7.04e-05 vs 2.70e-02; prin-rs at best 6.8×); the chosen tau lies within the gap and outcomes are unchanged across the middle of the gap (REQ-VAL-051).
- `cargo xtask gate escape-precision` — on the re-validation set, escape fires whose ground truth is bound are counted; gated at the measured precision (pre-R-29: 100.0%); a criterion variant that fires earlier with any false positives fails (REQ-EVT-019).

## Notes
- Gap reported: the legacy t = 30 set (config chart, 'unbound and receding at t = 30') is a prin-rs dataset not in the repository; its regeneration recipe is not given.
