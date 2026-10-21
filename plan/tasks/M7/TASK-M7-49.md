# TASK-M7-49 — Chart position's Picture base, the fade off the slice and the ensemble's nominal copy (R-542 SM8, SM10)

- **Milestone:** M7
- **Closes:** REQ-COL-130, REQ-COL-131
- **Depends on:** TASK-M7-48, TASK-M7-44
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~450 lines, the tests included

## Goal
Image mode: a picture laid on the target's square pulled back through the flow, mirror-repeated and dimmed past it, the built-ins and an uploaded picture as a render-side texture; the opt-in fade off the slice; the ensemble's nominal copy. **SM2 is a hard constraint** (the human's message, R-542): no change to `SimState`, the payload, the `ICDescriptor` or any layout. The field is derived at read in the fragment stage from data it already has (`r, p` at the playhead, the masses, the `state` lane, the WGSL encode). If anything turns out to need a stored field, stop and queue it in `REVIEW_QUEUE.md`; do not add one (REQ-COL-114).

## References
- `decisions.md` § "R-542 — The state map, as `principia_state_map.md` gives it and its prototype's code transcribes it: each sample coloured by where its own system is now on a chosen chart, through one stain node, Chart position, on every chart, the shape sphere included, coloured by a base and layers; Shape n̂ a separate node, unchanged; a system that leaves the chart drawn at the edge it left by, its last position not stored; no sim struct changed; Chart position waits on RQ-489 *(extends R-64)*"
- `docs/design/principia_state_map.md` § "SM8. Image mode"
- `docs/design/principia_state_map.md` § "SM10. Time, ensemble and fade"
- `docs/design/principia_colour_composition.md` § "3. The `ctx` contract"

## Deliverables
- The Picture base and its bind group, the built-in pictures, the upload path (a `RenderState` field, its tier in R-484's table), the fade (`crates/render/`).
- Tests `chart_position_picture`, `chart_position_fade`.

## Acceptance tests
- `cargo test -p render chart_position_picture` — REQ-COL-130's verify.
- `cargo test -p render chart_position_fade` — REQ-COL-131's verify.

## Notes
- **REQ-GUI-340 moved to TASK-M7-51 (gui re-review G6 of PR #270; applied per R-369),** which builds the inspector the controls sit in; this task keeps the render side. It keeps the gui reviewer because the picture's look on the figure (mirror-repeated, dimmed past the square) and the fade are GUI-facing. *Was:* "- **Closes:** REQ-COL-130, REQ-COL-131, REQ-GUI-340", with its controls deliverable and acceptance line.
- **gui review (PR #270 code review, 2; applied per R-369):** SM8 and SM10 are the GUI's in the human's message (board 3's Picture inspector is TASK-M7-51's; PR #270 gui re-review note, applied per R-369; *was:* "the gui reviewer reads board 3 per render GUI spec §G13's R-542 bullet"). *Was:* "- **Reviewers:** code, qa".
- TASK-M7-44 builds R-484's tier table, where the picture takes its tier (R-542 H5; applied per R-369).
- **Ruled (R-558):** RQ-484 (a); RQ-499, the file's words for these two: one fade everywhere, the trace's `max(0.18, e^{−d/1.2})`, and the brightness slot applying under a Picture as under every base. *Was:* "**Applied meanwhile:** RQ-484 (a); RQ-499 (a), the code's fade and no brightness under a Picture."
