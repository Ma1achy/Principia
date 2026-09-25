# TASK-M7-26 — The style stage: figure-only styles, plain for scientific checks

- **Milestone:** M7
- **Closes:** REQ-COL-044, REQ-COL-027
- **Depends on:** TASK-M7-25
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The optional style stage at the head of the display chain, applied to the figure only (chrome and legend unstyled), offering in v1 plain, watercolour & pencil and the seven print presets, as the poster's implementation (`workbench/principia_poster_both_sides.html` and its press module) defines them (R-130): watercolour & pencil is the poster's painted treatment; print is the paper.design halftone presets with the poster's patches (per-plate slip, cell ceiling); the golden-image and validation harnesses force style = plain.

## References
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"
- `decisions.md` § "R-67 — The display chain *(closes RQ-23)*"

- `decisions.md` § "R-130 ✱ — The styles are the poster's *(closes RQ-107)*"
## Deliverables
- `crates/render/shaders/wgsl/compositor/style_*.wgsl` and `crates/render/src/display/style.rs` — plain, the poster's painted treatment (watercolour & pencil), and the seven print presets ported from the poster's press module with its per-plate slip and cell ceiling (R-130).
- The style selector in the Display window.
- `xtask golden` and the validation harness forcing `style = plain`.

## Acceptance tests
- `cargo xtask screenshot 04_windows` (one scenario per v1 preset: plain, watercolour & pencil, the seven print presets) — each preset against 04_windows.png's Display window; chrome and legend are unstyled (REQ-COL-044).
- Review (code, qa): the golden-image and validation harnesses force style = plain (REQ-COL-027).

## Notes
- RQ-107 ruled: R-130 — the styles are defined by the poster's implementation; render_gui_spec's "print · Poster78, more" is read as the seven print presets. This settles the blocking gap. The human may veto (design ruling).
