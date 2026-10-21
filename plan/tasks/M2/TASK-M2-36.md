# TASK-M2-36 — Tilt and slice on every chart: the rotation R(τ₁, τ₂) on the chart's own parameters, mass tilts in logits, the extension after the tilt (R-473, R-472 X3) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-062, REQ-CHART-063, REQ-CHART-064, REQ-CHART-065, REQ-CHART-066
- **Depends on:** TASK-M2-14, TASK-M2-22, TASK-M2-08, TASK-M2-11, TASK-M2-12, TASK-M2-13, TASK-M2-28
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~700 lines, the tests included

## Goal
Every chart tilts and slices (R-473). The tilt is a rotation `R(τ₁, τ₂)` in the 3-space of the two screen axes and the hidden target, `(a, b, d) = R·(u, v, 0)`: `a`, `b` go through the chart's own map exactly and `d` is added to the hidden coordinate's latent control at the slice centre; with `X ≠ Y` it is the two-target form in `(a, b, d_X, d_Y)`. `R` comes from the active tilt map (R-474: Tip as sheet or Lean each axis), computed on the CPU and sent in the chart's params. A mass is tilted in logits, every linked coordinate in its link's pre-image (T3). Untilted, the chart takes no tilt path and is bitwise today's (C2). Slice moves any hidden coordinate (T6). A tilted chart extends past `[0,1]²` by its axes' types on the tilted `(a, b)` (X3).

## References
- `decisions.md` § "R-473 — Tilt and slice on every chart ⚑: the tilt is a rotation in the 3-space of the two screen axes and the hidden target, applied to the chart's own parameters, `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`, `a` and `b` through the chart map exactly and `d` added to the hidden coordinate at the slice centre, untilted bitwise today's; a mass, and every linked coordinate, is tilted in its link's pre-image; the units are queued (RQ-375); the slice moves any hidden coordinate *(extends R-457)*"
- `decisions.md` § "R-472 — Extension stands: R-407's four types, past a bounded edge a pixel blank; F3 has Extend and Fit, a GUI state field, remembered, named in the sidecar, present mode following it; a tilted, sliced or saved chart extends by its axes' types, nothing written per chart *(amends R-407 and R-468)*"
- `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, "Tip as sheet", the nav cube's axis-angle map and the default, and "Lean each axis", today's, an option, differing only with one target and both angles set; Tip as sheet carries on over the pole, wrapped only in total tip at 360° and never folded onto its 180° partner; planes are compared up to sign and never written back; the map is recorded everywhere as `tilt_map`; the compass draws the active map, B11 revisited; Lean's Gram factor and warnings; every readout shows each axis's components *(closes RQ-340; amends R-451, R-457 and R-459)*"
- `decisions.md` § "R-477 — The tests of R-469 to R-476, each with an arm that can fail: the same state through two charts, tilt at zero the identity, the two maps equal where they should be and different where not, over the pole, Tip as sheet orthonormal, never leaving the manifold under fuzzed navigation, the mass tilt on the simplex, saved charts round-tripping bitwise, blank never a colour, and exactly the right views re-integrating"
- `decisions.md` § "R-457 — The Manifold view's Navigate section loses its grid of eight `z₀` boxes; "Centre z₀" is renamed "Slice centre" and "Slice & tilt" "Tilt"; each tilt angle tilts into a latent dimension the user picks, "tilt τ₁ into [X]" and "tilt τ₂ into [Y]", the defaults today's one shared target; the step slider is removed *(amends render_gui_spec §G2's Navigate and Slice & tilt, R-409 A3's "the slice step" and R-451 B1's single tilt target)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"
- `decisions.md` § "R-507 — T4's units: the screen axis in units of the axis's full range at zoom 1, the hidden coordinate in its slider span, so no unit depends on zoom; the latent chart's tilt stays the decoder's basis edit; the human's half-width proposal is withdrawn *(closes RQ-375; amends R-473)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Directions are axis kinds — the unifying rule"
- `docs/contracts/principia_chart_decoder_contract.md` § "What the GPU knows about all of this: nothing"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"

## Deliverables
- The chart's hidden coordinates per chart (R-473 J1), from the block-touch sets (`crates/kernel/src/chart/`).
- `R` from the stored angles, targets and `tilt_map` (Tip as sheet's Rodrigues rotation, R-474 K1; Lean each axis; the two-target form), shared with the GUI's `View::basis` maths, and the chart params carrying `R`, the hidden indices and scales (R-473 J5).
- The tilt path in each chart's map, skipped exactly at `τ = 0` (J6); mass logits and links' pre-images (J8).
- Slice along any hidden coordinate (T6), through its latent control.
- The extension types acting on the tilted `(a, b)` (R-472 X3a).
- The tilted fold (chart_reference §3.3; applied per R-369): under a nonzero tilt `system_image` folds two pixels only when their full `(a, b, d)` decode to the same system, over the tilted view's pixels, recomputed when `τ₁`, `τ₂` or the targets change; a tilted shape-sphere view, on either map, is generically bijective (REQ-CHART-062).
- Units as R-507 rules them: each screen axis in units of its full range at zoom 1, the hidden coordinate in its slider span, on the curved charts; the decoder's basis edit on the latent chart (REQ-CHART-064, test `tilt_units`). *Was:* "Units: the human's proposal on the curved charts and the decoder's basis edit on the latent chart, as RQ-375 applies meanwhile (R-473 J3); REQ-CHART-064 is TASK-M2-39's."
- Tests `tilt_every_chart`, `tilt_mass_logits`, `slice_every_chart`, `tilted_extension`.

## Acceptance tests
- `cargo test -p kernel tilt_every_chart` — REQ-CHART-062's verify (C2 with its controls), with the tilted fold's arm: the shape sphere on both maps tilted 30° into a momentum control reports bijective and counts every pixel of the tilted view once, recomputed on a change of `τ` or target, the untilted value at `τ = 0`; control: a fold that pairs a tilted pixel with its shape mirror undercounts and fails. *Was:* "REQ-CHART-062's verify (C2 with its controls)."
- `cargo test -p kernel tilt_mass_logits` — REQ-CHART-063's verify (C7).
- `cargo test -p kernel tilt_units` — REQ-CHART-064's verify (R-507).
- `cargo test -p kernel slice_every_chart` — REQ-CHART-065's verify.
- `cargo test -p kernel tilted_extension` — REQ-CHART-066's verify.

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics. It covers T2 on the shape sphere (the pole-crossing extension composed with a tilt), T3 and the anchor (R-473 ⚑).
- **Units (R-507, ruling RQ-375).** On a curved chart `R` acts on the parameter offsets in units of each axis's full range at zoom 1 and the hidden offset in its slider span, scaled back after; the latent chart keeps the basis edit (U1, U2). REQ-CHART-064 moved here from TASK-M2-39 (applied per R-369: T2 is not built without its scale). *Was:* "Built under the meanwhile reading (R-473 J3); the ruling is TASK-M2-39's, a leaf."
- **Under Tip as sheet** with one target and both angles set a window pixel's `(a, b)` can leave the square; the types act on it as X3a says (R-471 H9), and REQ-CHART-066's test covers it; R-512 (ruling RQ-402) makes such a pixel past the edge, blank past a bounded one, which TASK-M2-39 tests. *Was:* "(RQ-402, R-471 H9), and REQ-CHART-066's test covers it; what it should draw is TASK-M2-39's."
- **Re-integration (R-92):** a curved chart's tilt is a chart-param edit, a sim key.
- **The tilted fold (physics review 5593301205 of PR #255, P1; applied per R-369, from chart_decoder_contract Part 5's definition, "how the address map … folds onto distinct systems"; flagged for the human's veto, as it changes area statistics).** R-503's cap and per-pixel fold and R-141's n-to-1 with n = 2 are the untilted chart's values. Under a tilt, a pixel and its shape mirror carry different `d` and are distinct systems, so the fold is taken over the tilted view's pixels and pairs only pixels whose full `(a, b, d)` decode to the same system. The task depends on TASK-M2-28, which builds the exponential map's fold.
