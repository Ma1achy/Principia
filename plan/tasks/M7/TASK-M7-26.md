# TASK-M7-26 — The style stage: figure-only styles, plain for scientific checks

- **Milestone:** M7
- **Closes:** REQ-COL-044, REQ-COL-027
- **Depends on:** TASK-M7-25
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The optional style stage at the head of the display chain, applied to the figure only (chrome and legend unstyled), offering at least plain, watercolour & pencil and print · Poster78, with paper grain and press misregistration; the golden-image and validation harnesses force style = plain.

## References
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"
- `decisions.md` § "R-67 — The display chain *(closes RQ-23)*"

## Deliverables
- `crates/render/shaders/wgsl/compositor/style_*.wgsl` and `crates/render/src/display/style.rs`.
- The style selector in the Display window.
- `xtask golden` and the validation harness forcing `style = plain`.

## Acceptance tests
- `cargo xtask screenshot 04_windows` (one scenario per style preset) — each preset against 04_windows.png's Display window; chrome and legend are unstyled (REQ-COL-044).
- Review (code, qa): the golden-image and validation harnesses force style = plain (REQ-COL-027).

## Notes
- Gap (blocking): the corpus names the styles but defines none — what watercolour & pencil and Poster78 compute, and the paper-grain and misregistration parameters, are not given. The task cannot be built without a ruling or a definition requirement.
- Waits on RQ-107 (`REVIEW_QUEUE.md`): The style presets are named, not defined.
