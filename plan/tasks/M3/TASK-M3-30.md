# TASK-M3-30 — Word calculus against the braid classes, and the truncation rate

- **Milestone:** M3
- **Closes:** REQ-VAL-043, REQ-VAL-037, REQ-PAY-083
- **Depends on:** TASK-M3-24, TASK-M3-16
- **Needs (earlier milestones):** REQ-PAY-029
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The free-group word encoder reproduces the published braid class of Šuvakov–Dmitrašinović gallery orbits, and the word truncation rate is measured over the validation fixtures and a representative survey to decide whether 76 symbols suffice, against a calibrated threshold.

## References
- `docs/design/principia_dd_validation_orbits.md` § "3. Proposed suite"
- `docs/design/principia_dd_validation_orbits.md` § "1.4 Šuvakov–Dmitrašinović (13 families, 2013) and Broucke–Hénon–Hadjidemetriou"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"

## Deliverables
- `fixtures/ground_truth/suvakov/` — gallery ICs with their published words/classes.
- `crates/validation/src/gates/word_calculus.rs`; `xtask gate word-truncation` over the reference charts at production horizon.
- Calibration proposal for the truncation-rate threshold.

## Acceptance tests
- `cargo test -p validation word_braid_class` — for each gallery orbit used, the computed word equals the published class (REQ-VAL-043).
- `cargo xtask gate word-truncation` — records the fraction of truncated words over the reference charts at production horizon (REQ-VAL-037).
- `cargo xtask gate word-truncation --propose` — measured truncation rates over the validation fixtures and a representative survey against the proposed threshold; the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-PAY-083).

## Notes
- Depends on the branch-cut convention gap reported for TASK-M3-16: a braid class can only be compared once a↔cut and the sign are fixed.
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-PAY-083.
