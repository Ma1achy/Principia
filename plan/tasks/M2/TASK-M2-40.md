# TASK-M2-40 — Saved charts in the kernel: the oblique latent chart and the curved parent with its tilt, composed tilts, and the bitwise reopen (R-475) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-073
- **Depends on:** TASK-M2-36
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~300 lines, the tests included

## Goal
A saved chart is a chart: a latent view saved as the oblique chart `(z₀, q₁, q₂)` exactly; a curved one as its parent, slice centre and tilt, its own parameters the window offsets, so a new tilt composes (`R′` first, then `R`). Reopened at zero tilt each reproduces the saved view bitwise (S3, C8).

## References
- `decisions.md` § "R-475 — "Save view as chart", in the Chart section and on View ›, commits the current view as a named custom chart that can be tilted, sliced and saved again, so tilts compose and any plane is reachable: a latent view saves `(z₀, q₁, q₂)` as an oblique chart, a curved one its parent, slice centre and tilt; reopened at zero tilt it reproduces the view bitwise; one undo entry, the picker's "Saved" group, its name editable and its provenance shown; "twelve tilt directions" is the full count"
- `decisions.md` § "R-473 — Tilt and slice on every chart ⚑: the tilt is a rotation in the 3-space of the two screen axes and the hidden target, applied to the chart's own parameters, `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`, `a` and `b` through the chart map exactly and `d` added to the hidden coordinate at the slice centre, untilted bitwise today's; a mass, and every linked coordinate, is tilted in its link's pre-image; the units are queued (RQ-375); the slice moves any hidden coordinate *(extends R-457)*"
- `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, "Tip as sheet", the nav cube's axis-angle map and the default, and "Lean each axis", today's, an option, differing only with one target and both angles set; Tip as sheet carries on over the pole, wrapped only in total tip at 360° and never folded onto its 180° partner; planes are compared up to sign and never written back; the map is recorded everywhere as `tilt_map`; the compass draws the active map, B11 revisited; Lean's Gram factor and warnings; every readout shows each axis's components *(closes RQ-340; amends R-451, R-457 and R-459)*"
- `docs/design/principia_chart_reference.md` § "1.1 Affine slices — the "no tilt" case"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"

## Deliverables
- The saved-chart variant of the chart registry, its params and its serialisation.
- Test `saved_chart`.

## Acceptance tests
- `cargo test -p kernel saved_chart` — REQ-CHART-073's verify (C8, the composition order's control).

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics. It covers the composition of tilts (R-475 L5).
