# TASK-M7-34 — Image embedding: the hybrid variant re-measured on real rendered figures, and §5's ~8 % reconciled

- **Milestone:** M7
- **Closes:** REQ-TOOL-151
- **Depends on:** TASK-M7-30, TASK-M7-32, TASK-M7-18, TASK-M7-21
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~250 lines (a measurement harness and a doc reconciliation)

## Goal
Before the M7 gate, re-measure the hybrid embedding at `k = 25` (REQ-TOOL-111) on real rendered figures, not random canvases, and reconcile `principia_dd_image_embedding.md` §5's "nearest rotation is recoverable but leaves ~8% bit error" with the 37.6 % raw low-bit error TASK-M7-30 measured on random canvases where the rotated-back pixel centres fall on the original's pixel corners (R-385). A real rendered figure is one the renderer produces from a computed field through the M7 display chain, coloured by a §7.1 preset and written by the PNG export with the hybrid embedding. The measurements join REQ-TOOL-111's evidence, which the human confirms at the M7 gate.

## References
- `docs/design/principia_dd_image_embedding.md` § "5. Two variants, one choice"
- `docs/design/principia_dd_image_embedding.md` § "7. Measured capacity and behaviour"
- `decisions.md` § "R-385 — Before the M7 gate, the hybrid embedding (`k = 25`) is re-measured on real rendered figures, and the 37.6 % raw low-bit error is reconciled with §5's ~8 %"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/render/tests/embed_hybrid_real_figures.rs` — the measurement harness: at least three real rendered figures of different §7.1 presets, among them a smooth one and one with a fine fractal boundary, at 512² and 1024², each embedded by the hybrid variant at `k = 25` (and at `k = 9`, for comparison) through the PNG export; for each, the raw low-bit error after nearest rotation and nearest rotation back at each whole degree 0–89 and at 7.3° and 34.7°, and the intact records recovered at each of §7's angles (7°, 34°, 45°, 7.3°, 34.7°).
- `docs/design/principia_dd_image_embedding.md` §5 (and §7 where it gives the figure) — the raw error on real figures and the conditions under which ~8 % and ~37.6 % hold, citing R-385, with a "Removed lines" note for any reworded line. No decision changes: the default stays tiled and `k` stays REQ-TOOL-111's proposal.
- The measurements, added to REQ-TOOL-111's proposal for the M7 gate.

## Acceptance tests
- `cargo test -p render embed_hybrid_real_figures` — measures and records the per-angle recovery of the hybrid variant at `k = 25` on the real figures and reports it, with the raw low-bit error table. It asserts that the measurement ran over every listed figure, size and angle and that the report was written; it does not fail on a recovery shortfall. A shortfall goes to the M7 gate for the human, because `k = 25` is the human's calibration (REQ-TOOL-111, R-71), and the reconciliation of 37.6 % with ~8 % goes in the same report (R-385, applied per R-369; REQ-TOOL-151).
- Review (code, qa): §5 states the raw error measured on real figures and the conditions under which ~8 % and ~37.6 % hold, reconciled with TASK-M7-30's random-canvas figures (8.13 % at 34°, 8.53 % at 45°, 4.30 % at 12°; 37.6 % at 7°, 20°, 60°, 83°; 43.3 % and 43.1 % at 7.3° and 34.7°); the measurements are cited in REQ-TOOL-111's proposal (REQ-TOOL-151).

## Notes
- R-385, applied per R-369: the dependencies are what produces a real rendered figure: the hybrid (TASK-M7-30), embedding in PNG export (TASK-M7-32), the §7.1 presets (TASK-M7-18) and the display chain (TASK-M7-21).
- R-385, applied per R-369: the test measures and records the per-angle recovery on the real figures and reports it. It asserts that the measurement ran over every listed figure, size and angle and that the report was written; it does not fail on a recovery shortfall. A shortfall goes to the M7 gate for the human, because `k = 25` is the human's calibration (REQ-TOOL-111, R-71), and the reconciliation of 37.6 % with ~8 % goes in the same report. The task does not change `k`.
