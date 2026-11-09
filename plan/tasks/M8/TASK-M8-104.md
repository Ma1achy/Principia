# TASK-M8-104 — 1D charts in the GUI

- **Milestone:** M8
- **Closes:** REQ-GUI-420
- **Depends on:** TASK-M8-18, TASK-M2-66, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
One swept axis and one annotation axis, as the Euclid plane has it, offered as a builder option, drawn as a strip with a line.

## References
- `docs/design/principia_chart_composition.md` § "10.1 1D charts"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"

## Deliverables
- The builder's 1D option; `crates/gui/src/windows/chart_builder/` and the strip's view.

## Acceptance tests
- `cargo xtask screenshot 03_chartbuilder` and `cargo test -p engine one_d_chart` — 1 + 7; a density claim refused; control: the annotation coordinate read (REQ-GUI-420).

## Notes
- **Build order (R-863):** no stage of the owner's order names it, so no stage edge is made; where it goes is RQ-744 (C).
- R-669 stands: this does not build a binary-plus-single chart, and TR-1 stays "not scored: no chart".
- Waits on the owner's ruling of the design doc as a whole, no single item mapping here (the order's A6, "Nothing implemented until ruled"); TASK-M2-58 in Depends on holds it.
- *8 Nov 2026 (R-873):* **Build order:** stage 3 of R-863's order, after stage 2 (TASK-M2-65), placed by R-873 (RQ-744 (A) (a) and (C)); TASK-M2-81 and TASK-M8-108, stage 4, depend on it. The note above that no stage edge is made is overtaken. *Was:* "- **Depends on:** TASK-M2-58, TASK-M8-18, TASK-M4-33"
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Depends on: TASK-M2-58, TASK-M8-18, TASK-M4-33, TASK-M2-65".
