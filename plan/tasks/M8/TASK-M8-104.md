# TASK-M8-104 — 1D charts in the GUI

- **Milestone:** M8
- **Closes:** REQ-GUI-420
- **Depends on:** TASK-M4-37, TASK-M8-18, TASK-M4-33
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
- R-669 stands: this does not build a binary-plus-single chart, and TR-1 stays "not scored: no chart".
- Waits on the owner's ruling of the design doc as a whole, no single item mapping here (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
