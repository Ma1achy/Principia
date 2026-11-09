# TASK-M8-103 — The Chart builder's composition panels

- **Milestone:** M8
- **Closes:** REQ-GUI-419
- **Depends on:** TASK-M2-58, TASK-M8-18, TASK-M2-68, TASK-M2-69, TASK-M2-71, TASK-M2-73, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
§ G7's axes become primitive, warp, extension and what is held; a chart gains its pair map and constraints; the Domain preview shows why.

## References
- `docs/design/principia_chart_composition.md` § "2. The two layers"
- `docs/design/principia_chart_composition.md` § "6.3 The Domain preview shows why"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"

## Deliverables
- `crates/gui/src/windows/chart_builder/`: the panels; the mock's first.

## Acceptance tests
- `cargo xtask screenshot 03_chartbuilder` and `cargo test -p gui composition_panels` — a refused pair shows its reason, a labelled pair runs, a saved composed chart reopens bitwise; control: a refused pair offered as Apply (REQ-GUI-419).

## Notes
- **Build order (R-863):** no stage of the owner's order names it, so no stage edge is made; where it goes is RQ-744 (C).
- What to try: on `cargo run -p gui --features mock`, build a pair the table refuses, and read the Domain preview.
- Waits on the owner's ruling of the design doc as a whole, no single item mapping here (the order's A6, "Nothing implemented until ruled"); TASK-M2-58 in Depends on holds it.
- *8 Nov 2026 (R-873):* **Build order:** stage 3 of R-863's order, after stage 2 (TASK-M2-65), placed by R-873 (RQ-744 (A) (a) and (C)); TASK-M2-81 and TASK-M8-108, stage 4, depend on it. The note above that no stage edge is made is overtaken. *Was:* "- **Depends on:** TASK-M4-37, TASK-M8-18, TASK-M4-34, TASK-M8-118, TASK-M8-100, TASK-M8-102"
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Depends on: TASK-M4-37, TASK-M8-18, TASK-M4-34, TASK-M8-118, TASK-M8-100, TASK-M8-102, TASK-M8-117".
