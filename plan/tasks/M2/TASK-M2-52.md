# TASK-M2-52 — Tilt and slice on the six charts that hold a block by a construction: the mass simplex's `ν₀`, a rest start's momentum controls from 0, and the held `L_z` or `K` of the Burrau charts, as physical hidden coordinates (R-633, R-473) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-062, REQ-CHART-065
- **Depends on:** TASK-M2-36
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
R-633 (ruling RQ-603) adds one principle to R-473 J1: where a chart holds a block by latent controls, its hidden coordinates are those controls; where it holds a block by a physical construction, its hidden coordinates are that construction's physical parameters, tilted and sliced by value as R-554 L5's `p_λ`. On TASK-M2-36's tilt path, this task makes the six construction charts tilt and slice, so that T1's "every chart" holds and REQ-CHART-062 and REQ-CHART-065 run over the whole registry:
- **The mass simplex:** its configuration hidden coordinate is `ν₀`, the held Burrau-family shape, a physical value; `d = 0` reproduces the untilted chart exactly (a3).
- **The rest-start charts** (the mass simplex, the Euclid plane, the δm strip): the rest start is `z_mom = 0`, and each of the four momentum controls is a hidden coordinate from 0, the momenta `D_mom` of it under the pixel's masses; every registered momentum link maps 0 to 0 (b1).
- **The Burrau charts with no free control:** the `(ν, K)` chart and `Φ_θ,K` gain the held `L_z`, and `Φ_θ,L_z` the held `K`, as physical hidden coordinates, the momenta §2.2's construction at the tilted value (c2). A cell with no momentum construction at the tilted value (`K < L_z²/2I`) reads as refused, through the tilt's existing ray-degenerate path: no system, area weight 0, drawn as validation layer 2 draws `K < L_z²/2I` (R-633's reading, applied per R-369).

## References
- `decisions.md` § "R-633 — Every central-difference Jacobian of the decode takes the encode contract's step; the steer's `J⁺` drops each singular value below `c·σ₁`, `c` a calibration; a chart that holds a block by a physical construction has that construction's physical parameters as hidden coordinates, tilted and sliced by value, and TASK-M2-36 splits; an unusable copy is a bit test on `state` and `budget_word` *(closes RQ-600, RQ-601, RQ-603 and RQ-604; amends R-473; refines R-550)*"
- `decisions.md` § "R-473 — Tilt and slice on every chart ⚑: the tilt is a rotation in the 3-space of the two screen axes and the hidden target, applied to the chart's own parameters, `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`, `a` and `b` through the chart map exactly and `d` added to the hidden coordinate at the slice centre, untilted bitwise today's; a mass, and every linked coordinate, is tilted in its link's pre-image; the units are queued (RQ-375); the slice moves any hidden coordinate *(extends R-457)*"
- `decisions.md` § "R-554 — The shape sphere's exponential map past `r = π` is the domain's end by design, blank and unwarned; the Lean-each-axis corner with one target is `ray-degenerate`, unfolded, its area weights 0 and its shares undefined; on a tilted view the pole maps apply only where `N_a = 0` and `N_b = 0`, the equator fold and the mirror's pixel fold left to the human's shape sphere ruling; T16 and T16b against a calibrated threshold; the physical `p_λ` components as the shape sphere's tilt targets *(closes RQ-470 to RQ-474; amends R-467, R-473, R-506, R-537 and R-538)*"
- `decisions.md` § "R-477 — The tests of R-469 to R-476, each with an arm that can fail: the same state through two charts, tilt at zero the identity, the two maps equal where they should be and different where not, over the pole, Tip as sheet orthonormal, never leaving the manifold under fuzzed navigation, the mass tilt on the simplex, saved charts round-tripping bitwise, blank never a colour, and exactly the right views re-integrating"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps"
- `docs/design/principia_chart_reference.md` § "2.2 Deterministic momentum construction"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"

## Deliverables
- The six charts' hidden coordinates (`crates/kernel/src/chart/`): the mass simplex's `ν₀` and four momentum controls; the Euclid plane's and the δm strip's four momentum controls; the held `L_z` of `(ν, K)` and `Φ_θ,K`; the held `K` of `Φ_θ,L_z`. Each is offered as a tilt target and a Slice centre slider, a physical value shown as itself, a momentum control through its link.
- The tilt on each, through TASK-M2-36's path: `d` added to the physical value at the slice centre for `ν₀`, `L_z` and `K`, and to the momentum control from 0 for a rest start; untilted, no tilt path and bitwise today's (C2).
- The refused cell: on the three Burrau charts, a cell whose tilted value leaves no momentum construction (`K < L_z²/2I`, §2.2's terminal case) is no system, its area weight 0.
- Slice along each (T6), by value for a physical hidden coordinate.
- The arms of `tilt_every_chart` and `slice_every_chart` for the six charts, and the corner arm (RQ-471, R-554) over the whole registry.

## Acceptance tests
- `cargo test -p kernel tilt_every_chart` — REQ-CHART-062's verify over every chart of `kernel::chart::REGISTRY`: the five latent-control charts' arms as TASK-M2-36 left them, and the construction charts' (R-633): a tilt into each physical hidden coordinate at `d = 0` bit-identical to the untilted chart over a 33 × 33 grid; on the mass simplex a 30° tilt into `ν₀` moves each pixel's `ν` by its `d` and keeps its masses; on a rest-start chart a 30° tilt into a momentum control gives the momenta `D_mom` of that control from 0 under each pixel's masses; on `(ν, K)` and `Φ_θ,K` a tilt into the held `L_z`, and on `Φ_θ,L_z` into the held `K`, gives §2.2's construction at the tilted value, cells with `K < L_z²/2I` refused (no system, area weight 0) and every other cell valid; the Lean-each-axis corner over the whole registry reports `ray-degenerate`; controls: the mass simplex's `z_α` held at the centre's value across the window (the pre-R-633 reading), which fails the `d = 0` arm, a refused cell counted or coloured, and a `d` added to a physical value for a latent target, each fail.
- `cargo test -p kernel slice_every_chart` — REQ-CHART-065's verify over every chart of `kernel::chart::REGISTRY`: a slice edit moves only that latent control of `z₀`, or, for `ν₀`, the held `L_z` or the held `K`, only that held value.

## Notes
- **⚑ Physics review is required:** the hidden coordinates change what a tilt or a slice of these charts decodes to (RQ-603, "physics and definition"). The physics reviewer checks the six charts against R-633 and the refused cells against §2.2's construction.
- **c1, the fallback, is not taken.** R-633 falls back to c1 (no tilt and no slice on the three Burrau charts, queued) only if §2.2's construction cannot take a nonzero held value. It can: §2.2 takes any `L_z` and `K*` with `K* ≥ L_z²/2I` and is terminal below it. If the build finds otherwise, the task stops and queues it, as R-633 says.
- **The refused cell (applied per R-369, flagged in R-633 for the owner's veto).** The corpus's `ray-degenerate` is a chart's `system_image` (chart_decoder_contract Part 5); R-633's "through the tilt's existing ray-degenerate path" is read per cell, the cell taking that path's area weight 0 and being no system, and the chart's `system_image` stays its own.
- **Split from TASK-M2-36 by R-633** (28 Oct 2026), which builds the tilt path and the five latent-control charts; this task depends on it and is numbered next after TASK-M2-51.
