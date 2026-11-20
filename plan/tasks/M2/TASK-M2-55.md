# TASK-M2-55 — Saved curved charts, the rest: compositions onto three or four hidden coordinates, a saved chart saved again, and the composed chart's area weight and repeat counting (R-475, R-513) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-148
- **Depends on:** TASK-M2-40, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~500 lines, the tests included

## Goal
TASK-M2-40 builds the saved chart and its composition up to two hidden offsets and one saved level, and refuses what lies past them. This task builds the rest that R-475 and R-513 rule: (a) a composition whose offsets land on three or four hidden coordinates, each on its own target (R-513 Q1, "one rule for every new target"; R-475 S5, planes in which an axis mixes several hidden coordinates); (b) a saved curved chart saved again, its definition the chain of saved tilts, `R·R′·R″` (R-475 S1); and (c) the composed chart's area weight and `ray-degenerate` flag (R-474 R7, R-554) and, on the shape sphere, its `system_image`, `pixel` and `counted_total` repeat counting (R-554 L3, R-655), as a single tilted chart has them.

## References
- `decisions.md` § "R-475 — "Save view as chart", in the Chart section and on View ›, commits the current view as a named custom chart that can be tilted, sliced and saved again, so tilts compose and any plane is reachable: a latent view saves `(z₀, q₁, q₂)` as an oblique chart, a curved one its parent, slice centre and tilt; reopened at zero tilt it reproduces the view bitwise; one undo entry, the picker's "Saved" group, its name editable and its provenance shown; "twelve tilt directions" is the full count"
- `decisions.md` § "R-513 — A new tilt of a saved curved chart into its own saved target composes the rotations, `(a, b, d) = R·R′·(u, v, 0)`, the saved `R` acting on `(u′, v′, d′)`; the saved target is offered again *(closes RQ-403; amends R-473 and R-475)*"
- `decisions.md` § "R-473 — Tilt and slice on every chart ⚑: the tilt is a rotation in the 3-space of the two screen axes and the hidden target, applied to the chart's own parameters, `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`, `a` and `b` through the chart map exactly and `d` added to the hidden coordinate at the slice centre, untilted bitwise today's; a mass, and every linked coordinate, is tilted in its link's pre-image; the units are queued (RQ-375); the slice moves any hidden coordinate *(extends R-457)*"
- `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, "Tip as sheet", the nav cube's axis-angle map and the default, and "Lean each axis", today's, an option, differing only with one target and both angles set; Tip as sheet carries on over the pole, wrapped only in total tip at 360° and never folded onto its 180° partner; planes are compared up to sign and never written back; the map is recorded everywhere as `tilt_map`; the compass draws the active map, B11 revisited; Lean's Gram factor and warnings; every readout shows each axis's components *(closes RQ-340; amends R-451, R-457 and R-459)*"
- `decisions.md` § "R-554 — The shape sphere's exponential map past `r = π` is the domain's end by design, blank and unwarned; the Lean-each-axis corner with one target is `ray-degenerate`, unfolded, its area weights 0 and its shares undefined; on a tilted view the pole maps apply only where `N_a = 0` and `N_b = 0`, the equator fold and the mirror's pixel fold left to the human's shape sphere ruling; T16 and T16b against a calibrated threshold; the physical `p_λ` components as the shape sphere's tilt targets *(closes RQ-470 to RQ-474; amends R-467, R-473, R-506, R-537 and R-538)*"
- `decisions.md` § "R-655 — On a tilted shape sphere the spherical map's lattice of repeats is folded: a pixel `(s + k, t + 2j)` with `N_a·k + 2·N_b·j = 0` for integers `(k, j) ≠ 0` is the same system as `(s, t)` and is counted once, at its canonical point, the lattice decided exactly on the stored angles *(closes RQ-631; amends R-554 L3 and R-537 EX7(b))*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"

## Deliverables
- `crates/kernel/src/chart/saved.rs` and `crates/kernel/src/chart/tilt.rs`: a shift carrying up to four hidden offsets (or another carrier the four fit), `Composed::new` admitting every such route; the saved definition of a saved chart, its serialisation and reopening; `weight` and the shape sphere's repeat counting on `Tilted<C, P, Composed<P>>`.
- Test `saved_chart_wide`.

## Acceptance tests
- `cargo test -p kernel saved_chart_wide` — REQ-CHART-148's verify.

## Notes
- **⚑ Physics review is required:** it builds the composition of tilts (R-475 L5, R-513).
- Split from TASK-M2-40 by the orchestrator (code review 5816802860 of PR #447, C1; physics review 5816819537, F3), applied per R-369 (task scoping, open to veto): TASK-M2-40 refuses (a) (`Composed::new` returns `None`) and (b) (`Saved::curved` takes no tilted chart), and gives the composed chart no weight or repeat counting, meanwhile. Nothing ruled is dropped.
- Widening `Shift` changes the qa files of earlier tasks that build one by its fields; such an `M` on a qa file is an exception the ruling forces, named in the PR (R-290, R-369).
- The case RQ-746 asks about (a saved Lean each axis tilt with one target and both angles set, tilted again into its target) is TASK-M2-56's, not this task's.
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Depends on: TASK-M2-40".
