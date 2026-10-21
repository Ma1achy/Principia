# TASK-M7-48 — Chart position's colour slot: the bases (Gradient, Site blend, Palette), the layers, the brightness occupants and the calibrations (R-542 SM6, SM11)

- **Milestone:** M7
- **Closes:** REQ-COL-122, REQ-COL-123, REQ-COL-124, REQ-COL-125, REQ-COL-126, REQ-COL-127, REQ-COL-128, REQ-COL-129
- **Depends on:** TASK-M7-47, TASK-M7-04, TASK-M7-05, TASK-M7-06, TASK-M7-08, TASK-M7-10
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~1,100 lines, the tests included

## Goal
Chart position's colour slot, colour composition §1.5: one base and an ordered list of layers per quotient, the 2D and 3D gradients one-to-one and in gamut, the sphere's Site blend one code path with Shape n̂'s, Shape n̂ bitwise unchanged; the brightness occupants Displacement and Left its starting tile; the calibration values proposed with their evidence. **SM2 is a hard constraint** (the human's message, R-542): no change to `SimState`, the payload, the `ICDescriptor` or any layout. The field is derived at read in the fragment stage from data it already has (`r, p` at the playhead, the masses, the `state` lane, the WGSL encode). If anything turns out to need a stored field, stop and queue it in `REVIEW_QUEUE.md`; do not add one (REQ-COL-114).

## References
- `decisions.md` § "R-542 — The state map, as `principia_state_map.md` gives it and its prototype's code transcribes it: each sample coloured by where its own system is now on a chosen chart, through one stain node, Chart position, on every chart, the shape sphere included, coloured by a base and layers; Shape n̂ a separate node, unchanged; a system that leaves the chart drawn at the edge it left by, its last position not stored; no sim struct changed; Chart position waits on RQ-489 *(extends R-64)*"
- `docs/design/principia_state_map.md` § "SM6. Colour maps: one-to-one on the quotient, so they own lightness"
- `docs/design/principia_state_map.md` § "SM11. Colouring Chart position: a base, then layers"
- `docs/design/principia_state_map.md` § "SM12. Tests (each with the arm that fires)"
- `docs/design/principia_state_map.md` § "SM13. Queue, don't rule"
- `docs/design/principia_colour_composition.md` § "1.5 A point on a chart: Chart position's colour slot (R-542)"
- `docs/design/principia_colour_composition.md` § "2. Site-set kinds"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"

## Deliverables
- The bases, tilings, swatches, layers and brightness occupants (`crates/render/`), transcribed from SM11's *Transcribed from the prototype* paragraph.
- The calibration register entries (REQ-COL-129).
- Tests `chart_position_slot`, `chart_position_injective`, `chart_position_site_blend_shared`, `shape_n_unchanged`, `palette_neighbours_differ`, `whole_periods`, `chart_position_brightness`.

## Acceptance tests
- `cargo test -p render chart_position_slot` — REQ-COL-122's verify.
- `cargo test -p render chart_position_injective` — REQ-COL-123's verify.
- `cargo test -p render chart_position_site_blend_shared` — REQ-COL-124's verify.
- `cargo test -p render shape_n_unchanged` — REQ-COL-125's verify.
- `cargo test -p render palette_neighbours_differ` — REQ-COL-126's verify.
- `cargo test -p render whole_periods` — REQ-COL-127's verify.
- `cargo test -p render chart_position_brightness` — REQ-COL-128's verify.
- Review checklist — REQ-COL-129's verify (the register entries, confirmed at the M7 gate).

## Notes
- **Applied meanwhile:** RQ-487 and RQ-488 (a); RQ-497 (a), Shape n̂ stays `vector`; RQ-498 (a), the code's signed n̂ (R-544: the whole sphere, settled where it turns on the hemisphere), the site-blend test as written (*was:* "the site-blend test on the folded hemisphere", PR #270's physics review, P1); RQ-499 (a), the code's triangle lightness and Fibonacci swatches.
