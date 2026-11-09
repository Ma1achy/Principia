# TASK-M2-56 — A saved Lean each axis tilt with one target and both angles set, tilted again into its target, as RQ-746 rules ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-149
- **Depends on:** TASK-M2-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines, the tests included

## Goal
R-513 Q1 feeds a new offset on the saved target into the saved `R`'s third input. A saved Lean each axis tilt with one target and both angles set has skewed axes and no third column, so TASK-M2-40's kernel refuses that one composition. Once RQ-746 is ruled, this task builds it as ruled, and the saved target is offered in that case too (R-513 Q3).

## References
- `decisions.md` § "R-513 — A new tilt of a saved curved chart into its own saved target composes the rotations, `(a, b, d) = R·R′·(u, v, 0)`, the saved `R` acting on `(u′, v′, d′)`; the saved target is offered again *(closes RQ-403; amends R-473 and R-475)*"
- `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, "Tip as sheet", the nav cube's axis-angle map and the default, and "Lean each axis", today's, an option, differing only with one target and both angles set; Tip as sheet carries on over the pole, wrapped only in total tip at 360° and never folded onto its 180° partner; planes are compared up to sign and never written back; the map is recorded everywhere as `tilt_map`; the compass draws the active map, B11 revisited; Lean's Gram factor and warnings; every readout shows each axis's components *(closes RQ-340; amends R-451, R-457 and R-459)*"
- `decisions.md` § "R-475 — "Save view as chart", in the Chart section and on View ›, commits the current view as a named custom chart that can be tilted, sliced and saved again, so tilts compose and any plane is reachable: a latent view saves `(z₀, q₁, q₂)` as an oblique chart, a curved one its parent, slice centre and tilt; reopened at zero tilt it reproduces the view bitwise; one undo entry, the picker's "Saved" group, its name editable and its provenance shown; "twelve tilt directions" is the full count"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"

## Deliverables
- `crates/kernel/src/chart/saved.rs` and `crates/kernel/src/chart/tilt.rs`: `Composed::new` admitting the case, composing as RQ-746's ruling says.
- Test `saved_chart_skewed`.

## Acceptance tests
- `cargo test -p kernel saved_chart_skewed` — REQ-CHART-149's verify.

## Notes
- **⚑ Physics review is required:** it builds the composition of tilts (R-513).
- **Waits on RQ-746** (`REVIEW_QUEUE.md`): its options are R-513's `R·R′` with the saved plane's unit normal as the third input, and R-514's frame action. Not started until it is ruled.
- Split from REQ-CHART-077 (physics review 5816819537 of PR #447, F2; R-473 J12's precedent), applied per R-369 (task scoping, open to veto). Until it lands, TASK-M8-48's target dropdowns don't offer the saved target in this one case.
