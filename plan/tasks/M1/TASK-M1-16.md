# TASK-M1-16 — Conform ctx.chart.slice_uv to the slice plane (R-394)

- **Milestone:** M1
- **Closes:** REQ-COL-063
- **Depends on:** TASK-M1-07
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~250 lines

## Goal
`ctx.chart.slice_uv` is the sample's position in the slice plane's own frame (R-394): `c + h·(2·ctx.quad.uv − 1)`,
from its quad's centre and half-width, the per-quad frames TASK-M1-07 binds, as deep_zoom §1 writes
`u = c + h·(2t − 1)`. In-plane pan and zoom never change it for a given sample (R-97), and the screen-relative position
stays the separate `ctx.screen.uv`. TASK-M1-06 filled the lane as the grid tiling,
`(vec2<f32>(r.quad_xy) + r.quad_uv) / vec2<f32>(ctx_uniforms.quads)` in `crates/render/src/bind.rs`, which is the view
reading R-394 did not choose; this task conforms it and adds the test RQ-257 found missing, on a non-square, offset
grid. It also carries the M1 gate's check of R-395's first half: the absolute coordinate does not band up to ℓ_switch.

## References
- `docs/design/principia_colour_composition.md` § "3. The `ctx` contract"
- `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision"
- `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"
- `docs/design/principia_memory_tiers.md` § "4. The six quality tiers"
- `decisions.md` § "R-394 — `ctx.chart.slice_uv` is the sample's position in the slice plane, stable under pan and zoom; a screen-relative position is a separate field *(closes RQ-257)*"
- `decisions.md` § "R-395 — REQ-TOOL-019's "no banding" holds with absolute coordinates up to ℓ_switch, checked at the M1 gate, and through the per-quad local coordinates beyond it, at M5 *(closes RQ-242)*"
- `decisions.md` § "R-97 — Quad addresses live in the slice plane *(closes RQ-57)*"
- `decisions.md` § "R-90 — The decoder switchover trigger *(closes RQ-41)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/render/src/bind.rs`: the chart lane's `slice_uv` member becomes
  `quad_frames[r.quad].xy + quad_frames[r.quad].zw * (2.0 * r.quad_uv - vec2<f32>(1.0))`, the same per-quad frames
  `ctx.quad.centre` and `ctx.quad.half_width` read; the grid-tiling expression is gone, and `slice_uv` no longer reads
  `ctx_uniforms.quads`.
- `crates/engine/tests/chart_slice_uv.rs`: `slice_uv` on `flat_at(.., 5, [7, 2])` with unequal columns and rows, its
  pan and zoom invariance, and the two failing variants below.
- `crates/render/tests/uv_absolute_banding.rs`: the depth sweep of the absolute coordinate to ℓ_switch, against the
  banding criterion and bound TASK-M1-07 defines (REQ-TOOL-152, as proposed until the M1 gate).

## Acceptance tests
- `cargo test -p engine chart_slice_uv` — on `flat_at(.., 5, [7, 2])` with a 3 × 2 grid (unequal columns and rows),
  the quad in column `i` and row `j` reads `ctx.chart.slice_uv = ((7 + i + ½)/32, (2 + j + ½)/32)` at its centre and
  `c ± h` at its corners, Y-up; the same quad inside a panned or zoomed grid (another origin or grid that contains its
  cell) reads the same `slice_uv`, bit for bit; a lane with width and height swapped, and the old grid-tiling formula,
  each fail the test (REQ-COL-063).
- `cargo test -p render uv_absolute_no_banding_to_l_switch` — R-395's first half, the M1 gate's check: at N = 8 and
  N = 16 (the named tiers' N), for every depth from 1 to ℓ_switch = 20, near `u ≈ 0.6` (the binade [0.5, 1), where
  f32's ulp is coarsest), the absolute f32 coordinate `c + h·(2t − 1)`'s adjacent deltas pass REQ-TOOL-152's criterion,
  or adjacent samples collapse to one coordinate, where R-90's switchover fires first (N = 16 at ℓ = 20); a non-dyadic
  N fixture (N = 6 at ℓ = 19) is shown exceeding the proposed bound, so the check can fail (REQ-TOOL-019; R-395).
- `cargo test -p engine` and `cargo test -p render` — the existing `ctx` tests stay green: no other lane changes.

## Notes
- R-394 (RQ-257) is the ruling this task applies; it is in M1, after TASK-M1-07, because the per-quad frames it reads
  arrive with TASK-M1-07 (PR #160), which also touches `bind.rs` (applied per R-369).
- On the harness's default grid, the slice's cells at its depth from the origin, the old and new formulas agree, which
  is why qa's square 4 × 4 test at depth 2 (`crates/engine/tests/qa_TASK-M1-06.rs`) could not tell them apart; only a
  non-square or offset grid does.
- The banding sweep is REQ-TOOL-019's, which TASK-M1-07 closes on M1's flat grid. R-395 adds the sweep to its verify
  after TASK-M1-07 was in review, so it is built here (applied per R-369); this task does not close REQ-TOOL-019.
- At a non-dyadic N, at the proposed bound 1/16, the absolute coordinate reads "banded" before ℓ_switch (N = 6 from
  ℓ = 19, N = 12 from ℓ = 18): RQ-258, open, for the human at the M1 gate with REQ-TOOL-152's value. The sweep covers
  N = 8 and N = 16 until it is ruled, and the N = 6 fixture only shows the check able to fail.
- `slice_uv` formed in f32 is an absolute coordinate, so past ℓ_switch it bands like any other (R-395); a fragment there
  works from `ctx.quad.centre` and the offset `h·(2·ctx.quad.uv − 1)`. REQ-TOOL-158 (M5, TASK-M5-04) is that half.
- TASK-M2-25, the first task to place `ctx.chart.z` per pixel, depends on this one.
