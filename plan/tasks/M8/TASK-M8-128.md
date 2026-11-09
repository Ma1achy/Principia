# TASK-M8-128 — Final-condition charts: a CPU batch chart of state functions at time T, the round-trip miss measured, large misses masked (R-879)

- **Milestone:** M8
- **Closes:** REQ-CHART-153, REQ-CHART-154
- **Depends on:** TASK-M2-58, TASK-M8-102, TASK-M8-108, TASK-M8-118, TASK-M2-78, TASK-M2-79, TASK-M2-69, TASK-M2-73
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~500 lines, the tests included

## Goal
R-879 (RQ-734): final-condition charts, built after the M8 charts, as a CPU batch chart whose axes are state functions at
time `T` (the model's Theorem 11: flip the momenta, integrate forward `T`, flip back), on the default integrator as an
approximate inverse. The round-trip miss is measured and reported per pixel, a pixel past the calibrated threshold
masked; the standard size `I = 1` kept at the chosen state with a per-pixel horizon as a labelled exception for these
charts only; a start found kept in the signed state; conserved axes shown `T`-independent (R-852).

## References
- `decisions.md` § "R-879 — Final-condition charts are built after the M8 charts, as a CPU batch chart on the default integrator, the round-trip miss measured per pixel and large misses masked; the standard size is kept at the chosen state, with a per-pixel horizon as a labelled exception for final-condition charts only; the rejection of "anything measured after `t = 0`" narrows to trajectory functionals and event reads *(closes RQ-734)*"
- `decisions.md` § "R-852 — Every registry primitive is classed as a state function, a conserved quantity (exact or approximate, labelled) or a trajectory functional, never an axis; final-condition charts stay a design item for the rulings report, with the check that conserved axes are `T`-independent *(on RQ-734, which stays open)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_chart_composition.md` § "10.3 Final-condition charts"
- `docs/contracts/principia_integrator_contract.md` § "Part 5 — Units and the horizon (inherited scale gauge)"

## Deliverables
- The final-condition chart as a CPU batch chart in `crates/engine`, its per-pixel round-trip miss computed, reported
  and, past the threshold, masked and labelled; the per-pixel horizon labelled as this chart's exception.
- The threshold proposed with its evidence (REQ-CHART-154).

## Acceptance tests
- `cargo test -p engine final_condition_chart` — on a small grid each pixel's round trip measured and reported, pixels
  past the threshold masked and labelled; a conserved axis (`L_z²E`, the sign of `E`) reads the same at two `T`; the
  per-pixel horizon labelled; controls: a pixel past the threshold drawn unmasked, a single global horizon where the
  rescale differs per pixel, an unsigned (mirrored) start, each failing (REQ-CHART-153).
- Proposal: the round-trip miss threshold with its evidence (the per-pixel miss distribution on a stated grid and `T`, and
  the share masked), checked by the physics reviewer, confirmed by the owner at the M8 gate and recorded in decisions.md;
  never a value chosen in the PR (R-71; REQ-CHART-154).

## Notes
- Added by R-879 (8 Nov 2026). Its Depends on, the M8 charts and TASK-M8-102's CPU batch route, applied per R-369.
- The horizon's exception is written into the integrator contract's Part 5 by R-879; nothing else changes the standard
  size.
- The owner wrote "this chart may use a per-pixel horizon"; the control that a single global horizon fails where the
  rescale differs per pixel is applied per R-369 (P8 of physics review 5820410914 of PR #455), `I = 1` being kept at the
  chosen state.
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Depends on: TASK-M4-37, TASK-M8-102, TASK-M8-108, TASK-M8-118, TASK-M8-122, TASK-M8-123".
