# TASK-M2-37 — Energy-type axes warped by default, per pixel, with a linear option per axis; PE's warp defined (R-471) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-067, REQ-CHART-068, REQ-CHART-069, REQ-CHART-070
- **Depends on:** TASK-M2-36, TASK-M2-12, TASK-M2-11, TASK-M2-14
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~500 lines, the tests included

## Goal
Axes of kind 3 (`E`, `L_z`, `K`, `KE`, `PE`) are warped by default wherever they appear, each alone taking its own line of chart_reference §2.1's warp, so every pixel of `[0,1]²` is feasible (E1, H1). The warp is evaluated per pixel with that pixel's own `U`, `I` and masses after the tilt's `d`, so tilted energy charts stay feasible (E3, T5). A linear mapping is an option per axis, a sim key: affine, mapped onto the warp's range at the centre, its infeasible pixels not a system, unwarned (E2, H4, H5). `PE`'s warp is written as a §2.1 paragraph under physics review, or no chart offers it (H2). R-506 widens it to every physical-quantity axis: the virial ratio's warp and the mass ratio's range in the logits are written in §2.1 under physics review too, or no chart offers them (REQ-CHART-070, W2), and R-508 continues the warp's `t` axis past `t = 1`, bounded below `t = 0` and past `s`'s edges (REQ-CHART-067).

## References
- `decisions.md` § "R-471 — Energy-type axes (`E`, `L_z`, `K`, `KE`, `PE`) are warped by default, as chart_reference §2.1 warps them, the warp evaluated per pixel with that pixel's own `U`, `I` and masses, so every pixel inside `[0,1]²` is feasible under every tilt; a linear mapping is an option per axis, its infeasible pixels "not a system" and the navigation stop keeping the centre feasible, recorded in the view state and the sidecar ⚑ *(supersedes R-408's port B1 as the default)*"
- `decisions.md` § "R-473 — Tilt and slice on every chart ⚑: the tilt is a rotation in the 3-space of the two screen axes and the hidden target, applied to the chart's own parameters, `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`, `a` and `b` through the chart map exactly and `d` added to the hidden coordinate at the slice centre, untilted bitwise today's; a mass, and every linked coordinate, is tilted in its link's pre-image; the units are queued (RQ-375); the slice moves any hidden coordinate *(extends R-457)*"
- `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity failures are forbidden *(closes RQ-263)*"
- `decisions.md` § "R-470 — A pixel that is not a system draws blank, the view's background, in every view, the outcome view included, never a palette colour; the hatch survives only as the optional Overlays › Chart › forbidden region, off by default, over blank pixels only; NaN, ±∞, `sim_failed`, `decode_failed` and the classification behind the hatch show only in the debug views; R-408's statistics are unchanged *(amends R-407, R-446 and R-467; closes RQ-374)*"
- `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid"
- `docs/contracts/principia_chart_decoder_contract.md` § "The four axis kinds — a closed set"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `decisions.md` § "R-506 — Every physical-quantity axis follows R-471: warped to its feasible range by default, linear as an option with its non-systems blank; the decoder-bug warning only for a warped axis; the mass ratio warps through the mass link *(closes RQ-372; amends R-471 and R-467; supersedes R-408's port B1 as the default for every physical-quantity axis)*"
- `decisions.md` § "R-508 — An energy chart's `t > 1` side continues: past `t = 1` the warp's `K > K_max` is drawn and counted as an affine side, while past `t < 0`, and the `s` axis past either edge, the warp stays bounded *(closes RQ-376; amends R-471)*"

## Deliverables
- The per-axis warp in `crates/kernel/src/chart/invariant.rs`, `mixed.rs` and the Burrau family's `K` axes, evaluated per pixel.
- The `warped | linear` chart parameter and the linear mapping (H4).
- The `PE` paragraph in chart_reference §2.1 (REQ-CHART-070, R-72), or its refusal by the well-posedness validator.
- Tests `energy_axes_warped`, `energy_axis_linear_option`, `energy_warp_per_pixel`, `pe_axis`.

## Acceptance tests
- `cargo test -p kernel energy_axes_warped` — REQ-CHART-067's verify.
- `cargo test -p kernel energy_axis_linear_option` — REQ-CHART-068's verify.
- `cargo test -p kernel energy_warp_per_pixel` — REQ-CHART-069's verify.
- `cargo test -p kernel pe_axis` and the physics review of the §2.1 paragraph — REQ-CHART-070's verify.

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics. It covers E3 on a tilted energy chart, the linear option's edge and B1's retirement (R-471 ⚑).
- **The t > 1 side** stays bounded (RQ-376, nothing waits). *Superseded by R-508, the next note; kept as history (code review 5587720942 F7, physics review 5587614130 item 3, of PR #255).*
- **R-508 (20 Oct 2026) rules RQ-376:** the warp's `t > 1` side continues, affine and counted; `t < 0` and the `s` axis stay bounded (REQ-CHART-067). *Was:* "The t > 1 side stays bounded (RQ-376, nothing waits)."
- **R-506 (20 Oct 2026) rules RQ-372:** every physical-quantity axis is warped by default, linear an option; REQ-CHART-070's definition now covers the virial ratio's warp and a mass-ratio range in the logits, written here under physics review (W2), and REQ-CHART-068's linear option covers them.
- **Physics review 5586890917 of PR #255:** REQ-CHART-068's centre arm replaced (F2; the centre agrees only at γ_K = 1); REQ-CHART-069's every-pixel arm narrowed to the pixels whose `(a, b)` lie in the square (F1; R-512, ruling RQ-402, reads E3 so). *Was:* "(F1, RQ-402)."
