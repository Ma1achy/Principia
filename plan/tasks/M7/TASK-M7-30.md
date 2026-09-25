# TASK-M7-30 — Image embedding: the hybrid variant for arbitrary rotation

- **Milestone:** M7
- **Closes:** REQ-TOOL-064, REQ-TOOL-111
- **Depends on:** TASK-M7-29
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
The hybrid variant — per-bit k-fold redundancy plus tiling — available beside the tiled default, recovering after arbitrary rotation (7°, 34°, 45°, 7.3°, 34.7°) by an angle search with the CRC as oracle; its default k proposed with evidence (R-71).

## References
- `docs/design/principia_dd_image_embedding.md` § "5. Two variants, one choice"
- `docs/design/principia_dd_image_embedding.md` § "7. Measured capacity and behaviour"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/render/src/embed/hybrid.rs` — k-fold bit redundancy, the angle search.
- The R-71 proposal: default k with §5/§7's measurements and its capacity cost.

## Acceptance tests
- `cargo test -p render embed_hybrid_rotation` — the hybrid variant recovers after 7°, 34°, 45°, 7.3°, 34.7° rotation; the tiled variant stays the default (REQ-TOOL-064).
- Review (code, qa): the proposal cites §5/§7's measurements (k = 9 → 5 %, k = 25 → 15 % uniform noise; ~8 % bit error after nearest rotation) and the capacity cost of the chosen k; confirmed by the human at the M7 gate (REQ-TOOL-111).

## Notes
- Calibration (R-71): REQ-TOOL-111.
