# TASK-M8-18 — Chart builder window: axis kinds, presets, domain preview, quick render, Apply (03_chartbuilder.png)

- **Milestone:** M8
- **Closes:** REQ-CHART-040, REQ-GUI-108, REQ-GUI-109, REQ-GUI-110, REQ-GUI-111
- **Depends on:** TASK-M8-06, TASK-M4-17, TASK-M5-05
- **Needs (earlier milestones):** REQ-SYS-013, REQ-CHART-039, REQ-CHART-043, REQ-CHART-005, REQ-CHART-021, REQ-GUI-003, REQ-GUI-004, REQ-SYS-027
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The Chart builder window builds a chart from two axes, each of a kind: a latent direction (a normalised mix of the eight z components, normalise showing |q|), a physical quantity (with a range and what is held fixed per the kind-2 residual convention), or a Burrau dimension (Euclid (m, n) → the primitive triple, with the neighbourhood swept). Presets are saved pairs of axes (Save, Duplicate, Delete). The Domain preview shows the admissible region from the chart's `validate(u, v)`, the forbidden region hatched, the view rectangle, the boundary formula and "forbidden in view: N%". A quick render (e.g. 64 × 64 at a short horizon) runs at the view's aspect. The footer checks q₁ · q₂ = 0, lists the hidden directions and the residual, and offers Revert / Apply (Apply emits the chart SetFields).

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "03 Chart builder"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"

## Deliverables
- `crates/gui/src/windows/chart_builder/{axes,presets,domain_preview,quick_render,footer}.rs`.
- `crates/engine/src/chart_preview.rs` — forbidden fraction over a uniform grid via `validate`; the quick render as a prebake-style uniform job (REQ-SYS-027).
- Presets stored in ViewUI-side user storage (not engine state).
- Tests: `preset_crud`, `forbidden_fraction`, `footer_orthogonality`; screenshot cases `03_chartbuilder/{latent,physical,burrau}`, `03_chartbuilder/previews`.

## Acceptance tests
- `cargo xtask screenshot 03_chartbuilder` (each axis kind) and `cargo test -p gui normalise_unit_q` — screenshot of each axis kind against 03_chartbuilder.png; normalise sets |q| = 1 (REQ-CHART-040).
- `cargo test -p gui preset_crud` — save, duplicate, edit and delete a preset; the list reflects each (REQ-GUI-108).
- `cargo test -p engine forbidden_fraction` and `cargo xtask screenshot 03_chartbuilder` — N% equals the fraction of a uniform grid over the view that validate rejects; screenshot against 03_chartbuilder.png (REQ-GUI-109).
- `cargo xtask screenshot 03_chartbuilder` (quick render) — screenshot against 03_chartbuilder.png (REQ-GUI-110).
- `cargo test -p gui footer_orthogonality` — non-orthogonal q₁, q₂ shows the check failing; Revert restores the prior chart; Apply emits the chart SetFields (REQ-GUI-111).

## Notes
- none
