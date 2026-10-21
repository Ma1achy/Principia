# TASK-M7-49 — Chart position's Picture base, the fade off the slice and the ensemble's nominal copy (R-542 SM8, SM10)

- **Milestone:** M7
- **Closes:** REQ-COL-130, REQ-COL-131, REQ-COL-137, REQ-COL-138
- **Depends on:** TASK-M7-48, TASK-M7-44
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~650 lines, the tests included (R-543's wrap mode and tileable checkerboard added; *was:* "~450 lines")

## Goal
Image mode: a picture laid on the target's square pulled back through the flow, mirror-repeated and dimmed past it by default, the built-ins and an uploaded picture as a render-side texture; R-543's wrap mode past the square (Repeat, Mirror or Clamp) and separate dim setting, Mirror and dim on by default so every saved stain renders as before (REQ-COL-137), and the tileable checkerboard (REQ-COL-138); the opt-in fade off the slice; the ensemble's nominal copy. **SM2 is a hard constraint** (the human's message, R-542): no change to `SimState`, the payload, the `ICDescriptor` or any layout. The field is derived at read in the fragment stage from data it already has (`r, p` at the playhead, the masses, the `state` lane, the WGSL encode). If anything turns out to need a stored field, stop and queue it in `REVIEW_QUEUE.md`; do not add one (REQ-COL-114).

## References
- `decisions.md` § "R-542 — The state map, as `principia_state_map.md` gives it and its prototype's code transcribes it: each sample coloured by where its own system is now on a chosen chart, through one stain node, Chart position, on every chart, the shape sphere included, coloured by a base and layers; Shape n̂ a separate node, unchanged; a system that leaves the chart drawn at the edge it left by, its last position not stored; no sim struct changed; Chart position waits on RQ-489 *(extends R-64)*"
- `docs/design/principia_state_map.md` § "SM8. Image mode"
- `docs/design/principia_state_map.md` § "SM10. Time, ensemble and fade"
- `docs/design/principia_colour_composition.md` § "3. The `ctx` contract"
- `docs/design/principia_state_map.md` § "SM12. Tests (each with the arm that fires)"
- `decisions.md` § "R-543 — Colour maps that tile the infinite plane, as the state map follow-up gives them and its prototype's code transcribes them: a Torus gradient on the plane and the cylinder, periodic in chart units, one-to-one within a period and seamless across periods; a wrap mode past the Picture's square, Repeat, Mirror or Clamp, and a separate dim setting, Mirror and dimmed by default so every saved stain renders as before; a tileable checkerboard; the Flow preset becomes it under Repeat *(amends R-542)*"

## Deliverables
- The Picture base and its bind group, the built-in pictures, the upload path (a `RenderState` field, its tier in R-484's table), the fade (`crates/render/`).
- Tests `chart_position_picture`, `chart_position_fade`.
- The wrap mode and the dim setting (`RenderState` fields beside the picture, R-542 H5) and the "Checkerboard, tileable" built-in, transcribed from SM8's *Transcribed from the prototype (R-543)* paragraph; tests `chart_position_picture_wrap`, `chart_position_tileable_checker`.

## Acceptance tests
- `cargo test -p render chart_position_picture` — REQ-COL-130's verify.
- `cargo test -p render chart_position_fade` — REQ-COL-131's verify.
- `cargo test -p render chart_position_picture_wrap` — REQ-COL-137's verify: Wrap arm (Repeat against Mirror, identical on [0, 1)², differing past the square, and at s = 1 Repeat reading texel 0 and Mirror texel N − 1; R-543 H11) and Wrap defaults bitwise (a stain saved without the wrap and dim fields renders bitwise as R-542's look-up; Gradient 2D, Picture and Palette hex), each with its failing arm (R-543 §5; the Gradient 2D and Palette hex legs' arms R-543 H11's).
- `cargo test -p render chart_position_tileable_checker` — REQ-COL-138's verify: Tileable checker seam, with the old checkerboard as its failing arm (R-543 §5).

## Notes
- **REQ-GUI-340 moved to TASK-M7-51 (gui re-review G6 of PR #270; applied per R-369),** which builds the inspector the controls sit in; this task keeps the render side. It keeps the gui reviewer because the picture's look on the figure (mirror-repeated, dimmed past the square) and the fade are GUI-facing. *Was:* "- **Closes:** REQ-COL-130, REQ-COL-131, REQ-GUI-340", with its controls deliverable and acceptance line.
- **gui review (PR #270 code review, 2; applied per R-369):** SM8 and SM10 are the GUI's in the human's message (board 3's Picture inspector is TASK-M7-51's; PR #270 gui re-review note, applied per R-369; *was:* "the gui reviewer reads board 3 per render GUI spec §G13's R-542 bullet"). *Was:* "- **Reviewers:** code, qa".
- TASK-M7-44 builds R-484's tier table, where the picture takes its tier (R-542 H5; applied per R-369).
- **R-543 (the wrap mode and the tileable checkerboard):** REQ-COL-137 and REQ-COL-138 are the follow-up's §4 items 2 and 3; Wrap defaults bitwise needs the app, so it is this task's (R-543 H8); on the prototype's transcription (`docs/incoming/2026-10-21_state_map_tiling/checks/r543_tiling_check.py`) version 9's look-up at its defaults equals version 8's on every texel. The inspector's controls are TASK-M7-51's (REQ-GUI-340). The tileable checkerboard's 0.8, 0.8 and 0.07 are REQ-COL-129's register entries, proposed here.
- **Applied meanwhile:** RQ-484 (a); RQ-499 (a), the code's fade and no brightness under a Picture.
