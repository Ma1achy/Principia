# TASK-M8-123 — The binary-plus-single scattering chart, with f_col and Heggie & Sweatman 1991 alongside (R-847)

- **Milestone:** M8
- **Closes:** REQ-CHART-144, REQ-VAL-354
- **Depends on:** TASK-M4-37, TASK-M8-111, TASK-M8-102
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The owner's reversal of R-669 (R-847, RQ-726 (b)): binary-plus-single scattering becomes a chart and builder axes like any other, with Heggie & Sweatman 1991 alongside and the f_col diagnostic built with it. Its missing definitions, `b_max`, the pericentre phase, and the pull-in distance from infinity to a finite start with a stated tidal error, are written here under physics review (R-72), never guessed. TR-1, MK-1 and SL-1 score on it in M9 (TASK-M9-23).

## References
- `decisions.md` § "R-847 — R-669 is reversed: fixed-`k` and binary-plus-single (scattering) become charts and builder axes, built when their turn comes, with Heggie & Sweatman 1991 alongside and f_col built with the scattering chart; their targets score in M9; orbital elements are axes at a labelled pair, plus overlays; MK-1 stays, labelled a planar re-run *(closes RQ-723, RQ-725 and RQ-726)*"
- `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data"
- `docs/design/principia_chart_composition.md` § "3.2 The primitives of the full order"

## Deliverables
- The scattering chart: its axes from § 3.2's rows, the hyperbolic-Kepler pull-in with its stated tidal error and recorded distance, the phase at pericentre passage, `b_max`; each definition first, physics-reviewed.
- Heggie & Sweatman 1991's planar set-up as a preset, where the held copy settles it (RQ-729's gap named otherwise).
- The f_col diagnostic (REQ-VAL-354).

## Acceptance tests
- `cargo test -p engine scattering_chart` — REQ-CHART-144's arms and controls.
- `cargo test -p engine f_col` — REQ-VAL-354's arm and control.

## Notes
- Nothing here is built until TASK-M4-37 has carried the rulings into the design (its Depends on).
