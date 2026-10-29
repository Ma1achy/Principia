# TASK-M2-51 — The orbit atlas through encode and decode: E and L per row (OC-2), and every atlas IC in the 8D latent chart (AT-D6)

- **Milestone:** M2
- **Closes:** REQ-VAL-214, REQ-VAL-215, REQ-VAL-216, REQ-VAL-217
- **Depends on:** TASK-M2-20, TASK-M2-26
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~350 lines

## Goal
The replication suite's targets OC-2, AT-D6 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Every Three Body Orbits atlas row's energy and angular momentum, recomputed from its initial conditions through the engine's own encode and decode path, must match the row's catalogued `E_cat` and `L_cat` to their printed precision (OC-2). Every published atlas initial condition must be encoded into the 8-dimensional latent chart, with scaled twins (rows equal up to a rescaling) landing on one point, the yin-yang α and β pairs landing on different points, each slice family's residual off its fitted plane printed, and the canonical coordinates exported as a CSV carrying the atlas's credit line (AT-D6). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M2 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.2 The orbit atlas"
- `docs/design/principia_dd_replication.md` § "4.3 The atlas's slice families"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 7 — Ground-truth ingestion (the validation programme's demand)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 4 — Conditioning: assert in physical units, never in z"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `crates/validation/src/atlas.rs`: the atlas CSV reader (masses as given), the tested frame remap with its round trip, the per-row E and L check, and the 8D encoding with the twin, yin-yang and slice-residual reports.
- `fixtures/atlas/threebodyorbits_ics.csv`, a byte copy of the record's CSV with its sha256 (52a67e5d…) checked in a test, and `fixtures/atlas/CREDITS.md` with the CC BY 4.0 credit line and CITATIONS.md's per-family credits.
- The canonical-coordinates export, `cargo xtask atlas-export`, a CSV that carries the credit line.
- Calibration proposals for OC-2's and AT-D6's tolerances.

## Acceptance tests
- `cargo xtask gate replication --target OC-2` — reference data the atlas CSV (sha256 52a67e5d…), masses as given, not normalised (rule 11); the conversion from the atlas's centre-of-mass frame into the canonical frame is a tested function with a round trip; per row |E − E_cat| and |L − L_cat| within REQ-VAL-215's tolerance; the CSV's own `E_recomp` and `L_recomp` are not the reference; control: a row with one velocity component's sign flipped must fail (REQ-VAL-214).
- `cargo xtask gate replication --target OC-2 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M2 gate and recorded in `decisions.md` (REQ-VAL-215).
- `cargo xtask gate replication --target AT-D6` — per row the encode residual and κ (inverse_encode Part 4); twins agree within REQ-VAL-217's tolerance and yin-yang pairs are separated by more than it; the three slice families of § 4.3 each print their residual off the fitted plane; control: a pair deliberately rescaled non-uniformly must land on different points; the export carries CC BY 4.0 to https://www.threebodyorbits.com and CITATIONS.md's per-family credits (REQ-VAL-216).
- `cargo xtask gate replication --target AT-D6 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M2 gate and recorded in `decisions.md` (REQ-VAL-217).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- Calibrations proposed here (R-71; human confirmation at the M2 gate, then recorded in decisions.md): REQ-VAL-215, REQ-VAL-217.
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`); no task may commit it.
- One of the orbit catalogue's six tests (`docs/incoming/2026-10-26_orbit_catalogues/INSTRUCTION_orbit_catalogues_2026-10-26.md` § "What to do with it (an addition to tier A)"), folded in by R-629.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
