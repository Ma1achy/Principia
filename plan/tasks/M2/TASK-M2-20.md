# TASK-M2-20 — Ground-truth ingestion: residual, κ(z) and the literature imports

- **Milestone:** M2
- **Closes:** REQ-VAL-014, REQ-ENC-025, REQ-VAL-019, REQ-VAL-120
- **Depends on:** TASK-M2-19, TASK-M0-05
- **Needs (earlier milestones):** REQ-VAL-002, REQ-VAL-007
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-5, PIT-3
- **Size:** ~420 lines

## Goal
Encode serves the validation programme's harder demand: foreign physical ICs are canonicalised and encoded, and the result exposes, beside z, the physical-space residual ‖D(z) − C(x)‖_phys (the bug signal) and a conditioning number κ(z), defined in inverse_encode Part 4 by this task. Anosova region-D and Burrau rest starts are imported through this door and their invariants match the papers' stated values after the recorded rescale; the reference values and the match tolerance are proposed with evidence.

## References
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 7 — Ground-truth ingestion (the validation programme's demand)"
- `docs/notes/principia_validation_ground_truth_note.md` § "New demands this places on the inverse-encode contract"
- `docs/notes/principia_validation_ground_truth_note.md` § "Everything is representable in principle — so a failed round-trip is a BUG signal"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 4 — Conditioning: assert in physical units, never in z"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
- `docs/design/principia_dd_encode.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_dd_encode.md` § "1. What it is"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_validation_orbits.md` § "1.2 Figure-eight — the canonical closure test"
- `docs/design/principia_dd_validation_orbits.md` § "1.3 Lagrange and Euler central configurations — the only analytic ones"

## Deliverables
- `crates/kernel/src/encode/ingest.rs` (z, residual, κ) and `crates/validation/src/import.rs` (the literature-IC import, tagged report per IC).
- The inverse_encode_contract Part 4 doc change defining κ(z), with a "Removed lines" note.
- Fixtures `fixtures/gates/literature_import/` (each IC with its paper citation); gate `cargo xtask gate literature-import`.

## Acceptance tests
- `cargo test -p kernel ingest_outputs` — Lagrange, figure-eight and Burrau ICs ingested; each encode result carries z, the physical residual and κ (REQ-VAL-014).
- Doc review (physics): Part 4 gives κ(z) as a formula (building on d logit/ds = 1/(s(1−s))) that bounds the T2 residual; physics reviewer approved (REQ-ENC-025).
- `cargo xtask gate literature-import` — import the literature ICs; E and L_z, multiplied back by the recorded rescale, match the papers' values within REQ-VAL-120's tolerance (REQ-VAL-019).
- Calibration: the proposal cites each paper's stated values, the recorded rescale and the measured import residuals, and states the tolerance; reviewer-checked, confirmed by the human at the M2 gate, recorded in `decisions.md` (REQ-VAL-120).

## Notes
- Gap G15 (‖·‖_phys) applies to the residual.
- The papers' values are not in the corpus; they enter through REQ-VAL-120's proposal, with citations.
