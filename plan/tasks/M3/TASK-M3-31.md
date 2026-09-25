# TASK-M3-31 — The predictive step limit and the two-artefact ablation

- **Milestone:** M3
- **Closes:** REQ-INT-052
- **Depends on:** TASK-M3-08, TASK-M3-19
- **Needs (earlier milestones):** REQ-VAL-009, REQ-VAL-003
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4.2, PIT-8
- **Size:** ~300 lines

## Goal
The predictive step limit is on by default in the regularised occupants' step control, and the ablation of pitfalls §8 is a standing gate on `config_stability`: five arms (none / dtau only / clamp only / limit only / all three), with the non-finite count and the wedge density reported as separate columns, the shipping config giving non-finite = 0 and wedge density ≈ 0.0001.

## References
- `docs/read_first/principia_01_pitfalls.md` § "8. TWO ARTEFACTS ARE NOT ONE DEFECT"
- `docs/read_first/principia_INDEX.md` § "The evidence base — where settled defaults were measured"

## Deliverables
- The predictive step limit in the regularised occupants' step control, default on, each ablation arm a build-time switch for the gate only.
- `crates/validation/src/gates/ablation.rs` — the two columns measured separately over a CPU render of `config_stability`.

## Acceptance tests
- `cargo xtask gate step-limit-ablation` — ablation on config_stability (none / dtau only / clamp only / limit only / all three): the shipping config gives non-finite = 0 and wedge density ≈ 0.0001 (the limit-only / all-three rows), and both columns are reported separately (REQ-INT-052).

## Notes
- Gap reported: the predictive step limit is named only in the INDEX as a prin-rs decision — its definition, the 'dtau' and 'clamp' arms, the wedge-density metric and the `config_stability` slice are not in the corpus.
