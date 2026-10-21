# TASK-M8-18 — Chart builder window: axis kinds, presets, domain preview, quick render, Apply (03_chartbuilder.png)

- **Milestone:** M8
- **Closes:** REQ-CHART-040, REQ-GUI-108, REQ-GUI-109, REQ-GUI-110, REQ-GUI-111, REQ-GUI-160
- **Depends on:** TASK-M8-06, TASK-M4-17, TASK-M5-05, TASK-M6-28, TASK-M6-45, TASK-M2-37
- **Needs (earlier milestones):** REQ-SYS-013, REQ-CHART-039, REQ-CHART-043, REQ-CHART-005, REQ-CHART-021, REQ-GUI-003, REQ-GUI-004, REQ-SYS-027
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The Chart builder window builds a chart from two axes, each of a kind: a latent direction (a normalised mix of the eight z components, normalise showing |q|), a physical quantity (with a range and what is held fixed per the kind-2 residual convention), or a Burrau dimension (Euclid (m, n) → the primitive triple, with the neighbourhood swept). Presets are saved pairs of axes (Save, Duplicate, Delete). The Domain preview shows the admissible region from the chart's `validate(u, v)`, the forbidden region hatched, the view rectangle, the boundary formula and "forbidden in view: N%". A quick render (e.g. 64 × 64 at a short horizon) runs at the view's aspect. The footer checks q₁ · q₂ = 0, lists the hidden directions and the residual, and offers Revert / Apply (Apply emits the chart SetFields).

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "03 Chart builder"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"
- `docs/gui/principia_render_gui_spec.md` § "11. Presets = whole graphs"
- `docs/design/principia_memory_tiers.md` § "7. The "are you sure?" safety system (three severities, none blocking)"
- `decisions.md` § "R-466 — Slice centre, Tilt and Navigate each have a "reset" beside the heading that returns the section to the active chart preset's own values, never literal zero; Home does all three in one command, from a button in the canvas's corner, the Home key in both keyboard modes and View › Home; each is one undoable command, dimmed when already home, and drives the compass live *(extends R-409, R-457, R-458 and R-462)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity failures are forbidden *(closes RQ-263)*"

## Deliverables
- `crates/gui/src/windows/chart_builder/{axes,presets,domain_preview,quick_render,footer}.rs`.
- `crates/engine/src/chart_preview.rs` — forbidden fraction over a uniform grid via `validate`; the quick render as a prebake-style uniform job (REQ-SYS-027).
- Presets stored in ViewUI-side user storage (not engine state).
- Tests: `preset_crud`, `forbidden_fraction`, `footer_orthogonality`; screenshot cases `03_chartbuilder/{latent,physical,burrau}`, `03_chartbuilder/previews`.

## Acceptance tests
- `cargo xtask screenshot 03_chartbuilder` (each axis kind) and `cargo test -p gui normalise_unit_q` — screenshot of each axis kind against 03_chartbuilder.png; normalise sets |q| = 1 (REQ-CHART-040).
- `cargo test -p gui preset_crud` — save, duplicate, edit and delete a preset; the list reflects each; a saved preset stores its base point, centre and window, which a duplicate keeps and the section resets and Home of the saved preset read back exactly; control: a saved preset whose resets fall back to another preset's values fails (REQ-GUI-108).
- `cargo test -p engine forbidden_fraction` and `cargo xtask screenshot 03_chartbuilder` — N% equals the fraction of a uniform grid over the view that validate rejects; screenshot against 03_chartbuilder.png (REQ-GUI-109).
- `cargo xtask screenshot 03_chartbuilder` (quick render) — screenshot against 03_chartbuilder.png (REQ-GUI-110).
- `cargo test -p gui footer_orthogonality` — non-orthogonal q₁, q₂ shows the check failing; Revert restores the prior chart; Apply emits the chart SetFields (REQ-GUI-111).
- Definition: the chart-preset, Your-stains and warning-suppression stores written into render_gui_spec §G7 and §11 and approved by the physics reviewer (REQ-GUI-160).

## Notes
- **A saved preset stores its home values (R-466; review round 1 of PR #243, gui review F6).** With its axes, a saved preset stores its base point, centre and window, which the section resets and Home read (REQ-GUI-108, REQ-GUI-224). TASK-M6-45 defines the three in render_gui_spec §G2 and §G7 and builds them on the mock; this task stores them with each saved, duplicated or edited preset.
- Closes, for gaps the corpus leaves open: REQ-GUI-160 (R-72 definition) (classification accepted by R-132).
- R-390: TASK-M6-28 builds the Chart builder's frame on the mock engine, its previews square; this task depends on it and wires it to the real chart system, keeping every requirement it closes.
- R-408 (10 Oct 2026): "forbidden in view" counts a visible pixel only if every axis is inside its primary range, a
  domain-hatched pixel leaves both the count and the total, and only validity failures are forbidden. `forbidden_fraction`
  uses TASK-M8-05's classifier (REQ-CHART-056). With the egui layer shown the view is `[0,1]²`, where nothing changes,
  so REQ-GUI-109's acceptance line stands. A physical-quantity axis is affine past `[0,1]²` (R-408's port, B1).
- R-467 (20 Oct 2026): the Chart builder's Domain preview (R-408, §G7) stands unchanged: it is a builder tool, not a
  presentation view, and keeps its hatch and its "forbidden in view" count. Whether a Chart-builder chart can fail
  inside its own square, and so whether it gets the debug view's in-square warning, is RQ-372, carried by REQ-GUI-228
  in TASK-M8-47, which depends on this task (decisions.md § R-467, F2).
- **R-471 (20 Oct 2026):** a physical-quantity axis of kind 3 (energy, `L_z`) is warped by default, its linear mapping an option (REQ-CHART-067, REQ-CHART-068, TASK-M2-37, a new dependency); the Domain preview's N% is 0 inside the square under the warp. R-475: saved charts share the presets' store (REQ-GUI-108's note). RQ-372 is answered for energy-type axes and stays open, narrowed (TASK-M8-47).
- **R-506 (20 Oct 2026) rules RQ-372:** every physical-quantity axis, the virial ratio and the mass ratio included, is warped to its feasible range by default (the mass ratio through the mass link), linear an option; the warps not written are REQ-CHART-070's (TASK-M2-37).
- R-478 W3, R-480 P3 (20 Oct 2026): the read-only links and the warp previews are TASK-M8-52's, after this task.
