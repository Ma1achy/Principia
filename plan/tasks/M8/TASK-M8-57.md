# TASK-M8-57 — Deep zoom in repeats and far from the root, and seams between images with a shared edge (R-537 EX8) ⚑

- **Milestone:** M8
- **Closes:** REQ-TOOL-182, REQ-RENDER-100, REQ-TOOL-183, REQ-CHART-087, REQ-CHART-088, REQ-GUI-397
- **Depends on:** TASK-M8-56, TASK-M6-08
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
Deep zoom works in the extension and in repeats (R-537 EX8): in a repeat the zoom is on the canonical quad, its f64 centre, half-width, `x₀` and `J_D` the canonical ones; on an affine extension far from the root the quad's f32 centre is taken relative to the root cell's own origin, held in f64; and two adjacent images meet with no background pixel between them at any zoom.

## References
- `decisions.md` § "R-537 — The chart's extension past `[0,1]²` is reached by pan and zoom with the controls shown, not only under F3: wherever the view lands it renders with no gaps, every interaction works there, a pixel in a repeat resolving to its canonical system, and the quadtree, refinement and deep zoom work in extended and wrapped regions, one system computed once ⚑ *(amends R-406, R-407 and R-472)*"
- `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision"
- `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"
- `decisions.md` § "R-570 — Latent and flat charts have no limit on pan or zoom-out: every latent `z` decodes to a system, so a pan never stops there and R-469's stop never fires for it; zoom-out is limited only by the frame budget and the baseline cover, a frame's cost set by the screen, not by the view's extent; every pixel is a system at any pan or zoom; far out the field goes flat by the decoder's saturation, which is correct and is never clamped, rescaled, wrapped or faded; precision is never a navigation limit *(states what R-407, R-467, R-469 and RQ-409 (a) imply; carries RQ-409 (a)'s "no zoom-out limit" from periodic charts to every chart whose axes are affine)*"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_deep_zoom.md` § "4. Sliding depth bound"
- `docs/design/principia_dd_decoder.md` § "3.7 Energy normalisation (optional; keep-or-drop is decision B9)"
- `decisions.md` § "R-597 — "No blank" on latent and flat charts holds at the default links with `μ_max` inside REQ-GEN-038's range, which is capped at 10; under the edge-reaching simplex link a pixel with `M₀₁ < 10⁻⁵` is genuinely degenerate and draws as not a system, with the reason in the readout *(closes RQ-536; qualifies R-570 item 3; caps REQ-GEN-038's `μ_max` range)*"
- `decisions.md` § "R-598 — With energy normalisation off every latent pixel is a system; with it on, a pixel whose target energy cannot be reached has no system at that energy and draws as not a system, with the reason in the readout, never unnormalised *(closes RQ-552; qualifies R-570 item 3)*"
- `decisions.md` § "R-599 — Collapse is judged per axis: an axis along which the field is saturated and the samples are identical is genuinely flat and does not count; a quad stops only when every axis along which the field varies has collapsed, or at the screen floor; the readout names the saturated axis *(closes RQ-554; refines R-90's terminal stop)*"

## Deliverables
- The canonical quad's deep-zoom uniforms for a repeat, the root-cell anchor for the f32 centre, and the shared-edge rasterisation of image seams, in `crates/kernel/` and `crates/render/`.
- Tests `deep_zoom_extension` and `image_seams`.
- R-570: the far field of a latent or flat chart flat by the decoder's saturation alone, every pixel a system, and the test `far_field_saturates` (`crates/kernel`).
- Proposal: the saturation tolerance, with the measured adjacent-pixel differences at `|z| = 10⁴` and near the origin; the human confirms it at the M8 gate (REQ-CHART-088).
- Proposal: the depth tolerance far from the root, with the measured placement depth at the root and at `u ~ 10³` and the fixture's `|x₀|`; the human confirms it at the M8 gate (REQ-TOOL-183).

## Acceptance tests
- `cargo test -p kernel deep_zoom_extension` — REQ-TOOL-182's verify (R-537 T9, T10).
- `cargo test -p render image_seams` — REQ-RENDER-100's verify (R-537 T11).
- `cargo test -p kernel far_field_saturates` — REQ-CHART-087's verify (R-570), at the default links with `E* = None`: where every coordinate the view varies is saturated (views at `10⁴·d`, `d`'s component on each varied coordinate at least 0.1, asserted for the seed) adjacent pixels agree within REQ-CHART-088's tolerance; at `10⁴·ê_k` adjacent pixels along `ê_k` agree; every pixel a valid system, none tagged; controls: near the origin adjacent pixels differ by more than the tolerance, and at `10⁴·ê_k` the plane's other, unsaturated axis stays structured; (3) at `μ_max = 10`, the cap, no pixel tagged (control: a `μ_max` past the cap whose far-field `M₀₁` falls below 10⁻⁵ tags a far pixel); (4) under the edge-reaching simplex link a far pixel with `M₀₁ < 10⁻⁵` is `DEGENERATE(M01_TINY)` and not a system, the tag carried for the readout; (5) with `Some(E*)` at an unreachable energy a far pixel is `ENERGY_INFEASIBLE` or `ENERGY_UNSCALABLE` and not a system, never decoded unnormalised (R-597, R-598).
- `cargo test -p gui readout_saturated_axis` — REQ-GUI-397's verify (R-599).

## Notes
- **⚑ Physics review is required**: the precision in repeats and far from the root; it does not merge on the code reviewer's word alone.
- **The anchor** is the root cell's own origin, not a re-set plane anchor, which would re-address the cache (R-97; R-537 U12).
- **T10** is measured as placement (REQ-TOOL-158's criterion) on an affine fixture whose decode is not saturated at `u ~ 10³`, the latent chart's α axes being saturated there (REQ-TOOL-182).
- **Closes, for gaps the corpus leaves open:** REQ-TOOL-183 (R-71 calibration).
- *Was:* Closes without REQ-TOOL-183 (physics review 5594430380 of PR #263, P4; applied per R-369).
- **R-570 (21 Oct 2026):** far out the field goes flat, and that is correct; nothing clamps, rescales, wraps or fades it, and precision is never a navigation limit (REQ-CHART-087; physics reviews it with the rest, ⚑). **Closes, for gaps the corpus leaves open:** REQ-CHART-088 (R-71 calibration, the saturation tolerance). **R-597 (25 Oct 2026, RQ-536 (a)):** "no blank" holds at the default links with `μ_max` in REQ-GEN-038's range, capped at 10; under the edge-reaching simplex link a pixel with `M₀₁ < 10⁻⁵` is genuinely degenerate, tagged `DEGENERATE(M01_TINY)` and drawn as not a system with the reason (arms (3) and (4)). **R-598 (RQ-552 (a)):** with η_E on, a pixel whose energy cannot be reached is not a system, with the reason, never unnormalised (arm (5)). **R-599 (RQ-554 (c)):** the hover readout over a band quad names its saturated axis (REQ-GUI-397, gui joining the reviewers, applied per R-369). *Was:* "**RQ-536 waits:** under the edge-reaching simplex link, or `softmax_tanh` with a large `μ_max`, the far field reaches the `M01_TINY` fence; that arm of REQ-CHART-087 waits, and the default-link arms do not."
- **Fix round 1 of PR #279** (physics review 5624655553, P1 to P3; applied per R-369): flatness is per coordinate (chart_reference §5.4), so far_field_saturates judges it only where every coordinate the view varies is saturated, and REQ-CHART-088 is measured on those views; `E* = None` is pinned, η_E on the latent chart being R-598's arm (5) (*was:* "being RQ-552 (its arm of REQ-CHART-087 waits, `rq: RQ-552`)"); the precision warning in the flat far field, where `AT_F32_FLOOR` fires at home zoom, is RQ-553, recorded in deep_zoom §4 and not settled. *Was:* "`cargo test -p kernel far_field_saturates` — REQ-CHART-087's verify (R-570): at `|z| = 10⁴` the states decoded at adjacent pixels agree within REQ-CHART-088's tolerance, every pixel a valid system at the default links; control: near the origin they differ."
- **RQ-553 decided per R-369** (23 Oct 2026, fix round 2 of PR #279, physics review 5624721489, P4): option (a) with the derivative test; the flat far field raises no precision warning, built by TASK-M6-21 (REQ-GUI-012). This task's ladder stops refinement there as before.
