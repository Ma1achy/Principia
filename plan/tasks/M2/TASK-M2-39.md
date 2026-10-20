# TASK-M2-39 — The tilt cases that wait: a window pixel whose (a, b) leave the square under Tip as sheet (RQ-402), and a saved curved chart tilted into its own target (RQ-403) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-074, REQ-CHART-075
- **Depends on:** TASK-M2-36, TASK-M2-38, TASK-M2-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines, the tests included

## Goal
Apply RQ-402's ruling on what a window pixel draws when Tip as sheet, with one target and both angles set, takes its chart parameters `(a, b)` outside the chart's square (R-471 H9), and RQ-403's on how a new tilt of a saved curved chart into its own saved target composes (R-475 L5).

## References
- `decisions.md` § "R-471 — Energy-type axes (`E`, `L_z`, `K`, `KE`, `PE`) are warped by default, as chart_reference §2.1 warps them, the warp evaluated per pixel with that pixel's own `U`, `I` and masses, so every pixel inside `[0,1]²` is feasible under every tilt; a linear mapping is an option per axis, its infeasible pixels "not a system" and the navigation stop keeping the centre feasible, recorded in the view state and the sidecar ⚑ *(supersedes R-408's port B1 as the default)*"
- `decisions.md` § "R-475 — "Save view as chart", in the Chart section and on View ›, commits the current view as a named custom chart that can be tilted, sliced and saved again, so tilts compose and any plane is reachable: a latent view saves `(z₀, q₁, q₂)` as an oblique chart, a curved one its parent, slice centre and tilt; reopened at zero tilt it reproduces the view bitwise; one undo entry, the picker's "Saved" group, its name editable and its provenance shown; "twelve tilt directions" is the full count"
- `decisions.md` § "R-507 — T4's units: the screen axis in units of the axis's full range at zoom 1, the hidden coordinate in its slider span, so no unit depends on zoom; the latent chart's tilt stays the decoder's basis edit; the human's half-width proposal is withdrawn *(closes RQ-375; amends R-473)*"
- `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"

## Deliverables
- Per RQ-402's ruling, in the tilt path of `crates/kernel/src/chart/` (TASK-M2-36's).
- Per RQ-403's ruling, in the saved chart's composition (TASK-M2-40's), and the saved target offered again.
- Tests `tip_sheet_in_window`, `saved_chart_same_target`.

## Acceptance tests
- `cargo test -p kernel tip_sheet_in_window` — REQ-CHART-074's verify.
- `cargo test -p kernel saved_chart_same_target` — REQ-CHART-075's verify.

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics.
- **A leaf task waiting for RQ-402 and RQ-403** (`rq:` on each requirement): nothing depends on it.
- **Re-scoped (R-507 U4, applied per R-369).** This task was T4's units, waiting for RQ-375; R-507 rules them and TASK-M2-36 builds them with T2. *Was:* "T4's units for a tilt mixing a chart parameter with a hidden coordinate, as RQ-375 rules (R-473) ⚑", closing REQ-CHART-064, depending on TASK-M2-36.
