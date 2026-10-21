# TASK-M7-48 — Chart position's colour slot: the bases (Gradient, Site blend, Palette), the layers, the brightness occupants and the calibrations (R-542 SM6, SM11)

- **Milestone:** M7
- **Closes:** REQ-COL-122, REQ-COL-123, REQ-COL-124, REQ-COL-125, REQ-COL-126, REQ-COL-127, REQ-COL-128, REQ-COL-129, REQ-COL-136
- **Depends on:** TASK-M7-47, TASK-M7-04, TASK-M7-05, TASK-M7-06, TASK-M7-08, TASK-M7-10
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~1,250 lines, the tests included (R-543's Torus gradient added; *was:* "~1,100 lines")

## Goal
Chart position's colour slot, colour composition §1.5: one base and an ordered list of layers per quotient, the 2D and 3D gradients one-to-one and in gamut, the sphere's Site blend one code path with Shape n̂'s, Shape n̂ bitwise unchanged; the brightness occupants Displacement and Left its starting tile; the calibration values proposed with their evidence; and R-543's Torus gradient on the plane and the cylinder, periodic in chart units, one-to-one within a period and seamless across periods (REQ-COL-136). **SM2 is a hard constraint** (the human's message, R-542): no change to `SimState`, the payload, the `ICDescriptor` or any layout. The field is derived at read in the fragment stage from data it already has (`r, p` at the playhead, the masses, the `state` lane, the WGSL encode). If anything turns out to need a stored field, stop and queue it in `REVIEW_QUEUE.md`; do not add one (REQ-COL-114).

## References
- `decisions.md` § "R-542 — The state map, as `principia_state_map.md` gives it and its prototype's code transcribes it: each sample coloured by where its own system is now on a chosen chart, through one stain node, Chart position, on every chart, the shape sphere included, coloured by a base and layers; Shape n̂ a separate node, unchanged; a system that leaves the chart drawn at the edge it left by, its last position not stored; no sim struct changed; Chart position waits on RQ-489 *(extends R-64)*"
- `docs/design/principia_state_map.md` § "SM6. Colour maps: one-to-one on the quotient, so they own lightness"
- `docs/design/principia_state_map.md` § "SM11. Colouring Chart position: a base, then layers"
- `docs/design/principia_state_map.md` § "SM12. Tests (each with the arm that fires)"
- `docs/design/principia_state_map.md` § "SM13. Queue, don't rule"
- `docs/design/principia_colour_composition.md` § "1.5 A point on a chart: Chart position's colour slot (R-542)"
- `docs/design/principia_colour_composition.md` § "2. Site-set kinds"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `decisions.md` § "R-543 — Colour maps that tile the infinite plane, as the state map follow-up gives them and its prototype's code transcribes them: a Torus gradient on the plane and the cylinder, periodic in chart units, one-to-one within a period and seamless across periods; a wrap mode past the Picture's square, Repeat, Mirror or Clamp, and a separate dim setting, Mirror and dimmed by default so every saved stain renders as before; a tileable checkerboard; the Flow preset becomes it under Repeat *(amends R-542)*"

## Deliverables
- The bases, tilings, swatches, layers and brightness occupants (`crates/render/`), transcribed from SM11's *Transcribed from the prototype* paragraph.
- The calibration register entries (REQ-COL-129).
- Tests `chart_position_slot`, `chart_position_injective`, `chart_position_site_blend_shared`, `shape_n_unchanged`, `palette_neighbours_differ`, `whole_periods`, `chart_position_brightness`.
- The Torus gradient (R-543), transcribed from SM6's *Transcribed from the prototype (R-543)* paragraph; test `chart_position_torus`; the Torus's register entries in REQ-COL-129.

## Acceptance tests
- `cargo test -p render chart_position_slot` — REQ-COL-122's verify.
- `cargo test -p render chart_position_injective` — REQ-COL-123's verify.
- `cargo test -p render chart_position_site_blend_shared` — REQ-COL-124's verify.
- `cargo test -p render shape_n_unchanged` — REQ-COL-125's verify.
- `cargo test -p render palette_neighbours_differ` — REQ-COL-126's verify.
- `cargo test -p render whole_periods` — REQ-COL-127's verify.
- `cargo test -p render chart_position_brightness` — REQ-COL-128's verify.
- Review checklist — REQ-COL-129's verify (the register entries, confirmed at the M7 gate).
- `cargo test -p render chart_position_torus` — REQ-COL-136's verify: Torus seam (to 1e-12, well past the square), Torus one-to-one (min OKLab separation > 0 on a 200² grid, and the signed radial chroma (a, b)·(cos θ, sin θ) > 0 at every grid point, read from the map's output) and Torus gamut (0 of 240², no sample changed by a clamp), each with its failing arms (R-543 §5).

## Notes
- **R-543 (the Torus gradient):** REQ-COL-136 is the follow-up's §4 item 1, its verify §5's three Torus rows; `docs/incoming/2026-10-21_state_map_tiling/checks/r543_tiling_check.py` is the transcription they were run on and shows each arm firing, a reference for the Rust test, not a substitute. Its constants and default Repeats join REQ-COL-129 (R-543 H7). Applied meanwhile: RQ-506 (a), the code's period counts; RQ-507 (a), Left its starting tile on the Palette's tile.
- **Applied meanwhile:** RQ-487 and RQ-488 (a); RQ-497 (a), Shape n̂ stays `vector`; RQ-498 (a), the code's signed n̂, the site-blend test as written (*was:* "the site-blend test on the folded hemisphere", PR #270's physics review, P1); RQ-499 (a), the code's triangle lightness and Fibonacci swatches.
