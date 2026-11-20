# TASK-M8-47 — A Chart-builder chart that fails validity inside [0,1]²: its drawing, and its console warning only for warped axes (R-467, R-506)

- **Milestone:** M8
- **Closes:** REQ-GUI-228
- **Depends on:** TASK-M8-18, TASK-M8-46
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~120 lines, the test included

## Goal
A Chart-builder chart whose physical-quantity axis's range leaves the slice's reach inside `[0,1]²` is drawn as every chart is (the canvas background in a presentation view, the hatch in the debug view), and its console warning, or the builder's feasibility guarantee that removes the case, is built as RQ-372 rules (R-467 F2).

## References
- `decisions.md` § "R-467 — Forbidden pixels and non-finite values are debug-only: a presentation view draws a pixel that is not part of the chart as the canvas background, with no hatch, key or share, and every failure and non-finite value `#141418`; one switchable debug view, View › Debug view, shows the hatch, `debug_invalid` and a NaN / Inf style, each with its key and count *(amends R-446, and R-407 clause 2 as it applies to display; closes RQ-319)*"
- `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity failures are forbidden *(closes RQ-263)*"
- `decisions.md` § "R-26 — Every chart declares its domain function *(CD-6)*"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `decisions.md` § "R-506 — Every physical-quantity axis follows R-471: warped to its feasible range by default, linear as an option with its non-systems blank; the decoder-bug warning only for a warped axis; the mass ratio warps through the mass link *(closes RQ-372; amends R-471 and R-467; supersedes R-408's port B1 as the default for every physical-quantity axis)*"

## Deliverables
- What RQ-372's ruling asks for, in the Chart builder's Apply path or the console warning (REQ-GUI-228).
- Test `builder_chart_in_square`.

## Acceptance tests
- `cargo test -p gui builder_chart_in_square` — as REQ-GUI-228's verify (REQ-GUI-228).

## Notes
- **Waits for RQ-372** (`rq: RQ-372` on REQ-GUI-228). A leaf task: nothing depends on it.
- **R-471 H5 (20 Oct 2026):** RQ-372 is answered for energy-type axes and stays open, narrowed to the virial ratio and the mass ratio; this task waits for those alone.
- **R-506 (20 Oct 2026) rules RQ-372:** every physical-quantity axis is warped by default, linear an option; the warning is logged only when the chart's physical-quantity axes are all warped (W1). This task no longer waits; REQ-GUI-228 is restated. The two notes above are kept as history.
