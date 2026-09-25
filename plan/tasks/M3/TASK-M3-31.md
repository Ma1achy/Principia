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
- `decisions.md` § "R-163 — The wedge ablation is re-run on Heggie"
- `decisions.md` § "R-166 — The fixtures"
- `decisions.md` § "R-160 — The integrator equations are transcribed into integrator_contract Part 2b"
- `docs/contracts/principia_integrator_contract.md` § "The equations and the step control (R-160, R-161)"

## Deliverables
- The predictive step limit in the regularised occupants' step control, default on, each ablation arm a build-time switch for the gate only.
- `crates/validation/src/gates/ablation.rs` — the two columns measured separately over a CPU render of `config_stability`.

## Acceptance tests
- `cargo xtask gate step-limit-ablation` — ablation on config_stability (`fixtures/slices.toml`) with prin-rs `wedge_census.rs`'s three switches (none / dtau only / clamp only / limit only / all three), re-run on Heggie with Aarseth–Zare kept for comparison (R-163), wedge density being at least 25% pale pixels in a 9×9 window at 1024²: the shipping config gives non-finite = 0 and wedge density ≈ 0.0001 (the limit-only / all-three rows), and both columns are reported separately (REQ-INT-052).

## Notes
- *Was: "Gap: the predictive step limit is named only in the INDEX as a prin-rs decision — its definition, the 'dtau' and 'clamp' arms, the wedge-density metric and the `config_stability` slice are not in the corpus."* RQ-102 and RQ-103 ruled: the limit, `dτ ≤ f·d_min / (|v_rel|_max·A·B)` with f = 0.02, is in integrator_contract Part 2b (R-160; its transcription is REQ-INT-084, TASK-M3-08); the arms and the density metric are pitfalls §8's, from prin-rs `examples/wedge_census.rs` (R-163); `config_stability` is in `fixtures/slices.toml` (R-166).
