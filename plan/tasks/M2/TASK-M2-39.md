# TASK-M2-39 — Under Tip as sheet, a window pixel whose (a, b) leave the chart's square is past the edge, blank past a bounded one (R-512) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-076
- **Depends on:** TASK-M2-36, TASK-M2-38
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines, the tests included

## Goal
When Tip as sheet, with one target and both angles set, takes a window pixel's own chart parameters `(a, b)` outside the chart's square, the pixel is past the chart's edge (R-512, ruling RQ-402): its axes' types act on `(a, b)` as past the window, so past a bounded edge it is not a system and draws blank, and past the energy warp's continuing `t > 1` side it is that side's system (R-512 F1, R-508). *Was:* "Apply RQ-402's ruling on what a window pixel draws when Tip as sheet, with one target and both angles set, takes its chart parameters `(a, b)` outside the chart's square (R-471 H9), and RQ-403's on how a new tilt of a saved curved chart into its own saved target composes (R-475 L5)."

## References
- `decisions.md` § "R-471 — Energy-type axes (`E`, `L_z`, `K`, `KE`, `PE`) are warped by default, as chart_reference §2.1 warps them, the warp evaluated per pixel with that pixel's own `U`, `I` and masses, so every pixel inside `[0,1]²` is feasible under every tilt; a linear mapping is an option per axis, its infeasible pixels "not a system" and the navigation stop keeping the centre feasible, recorded in the view state and the sidecar ⚑ *(supersedes R-408's port B1 as the default)*"
- `decisions.md` § "R-475 — "Save view as chart", in the Chart section and on View ›, commits the current view as a named custom chart that can be tilted, sliced and saved again, so tilts compose and any plane is reachable: a latent view saves `(z₀, q₁, q₂)` as an oblique chart, a curved one its parent, slice centre and tilt; reopened at zero tilt it reproduces the view bitwise; one undo entry, the picker's "Saved" group, its name editable and its provenance shown; "twelve tilt directions" is the full count"
- `decisions.md` § "R-507 — T4's units: the screen axis in units of the axis's full range at zoom 1, the hidden coordinate in its slider span, so no unit depends on zoom; the latent chart's tilt stays the decoder's basis edit; the human's half-width proposal is withdrawn *(closes RQ-375; amends R-473)*"
- `decisions.md` § "R-512 — Under Tip as sheet a window pixel whose own `(a, b)` leave the chart's square is past the edge, and blank past a bounded one; E3 and C6 read over the chart's square *(closes RQ-402; amends R-471, R-472 and R-477)*"
- `decisions.md` § "R-508 — An energy chart's `t > 1` side continues: past `t = 1` the warp's `K > K_max` is drawn and counted as an affine side, while past `t < 0`, and the `s` axis past either edge, the warp stays bounded *(closes RQ-376; amends R-471)*"
- `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"

## Deliverables
- R-512's past-the-edge rule inside the window, in the tilt path of `crates/kernel/src/chart/` (TASK-M2-36's), the types acting on `(a, b)` exactly as past the window. *Was:* "Per RQ-402's ruling, in the tilt path of `crates/kernel/src/chart/` (TASK-M2-36's)." and "Per RQ-403's ruling, in the saved chart's composition (TASK-M2-40's), and the saved target offered again."; the second moved to TASK-M2-40 (R-513 Q4).
- Test `tip_sheet_in_window`. *Was:* "Tests `tip_sheet_in_window`, `saved_chart_same_target`."

## Acceptance tests
- `cargo test -p kernel tip_sheet_in_window` — REQ-CHART-076's verify.

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics.
- **A leaf task waiting for RQ-402 and RQ-403** (`rq:` on each requirement): nothing depends on it. *Superseded by the R-512 and R-513 note below; kept as history.*
- **Re-scoped (R-507 U4, applied per R-369).** This task was T4's units, waiting for RQ-375; R-507 rules them and TASK-M2-36 builds them with T2. *Was:* "T4's units for a tilt mixing a chart parameter with a hidden coordinate, as RQ-375 rules (R-473) ⚑", closing REQ-CHART-064, depending on TASK-M2-36.
- **R-512 and R-513 (20 Oct 2026) rule RQ-402 and RQ-403 (applied per R-369):** nothing waits. RQ-402's half is R-512's past-the-edge rule, built here; RQ-403's half, REQ-CHART-077, moves to TASK-M2-40, which builds the composition it extends (R-513 Q4), so this task no longer depends on TASK-M2-40. Still a leaf: nothing depends on it.
