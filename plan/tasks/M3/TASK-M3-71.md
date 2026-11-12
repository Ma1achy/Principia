# TASK-M3-71 — The collision test's floor, recorded with the run (R-954)

- **Milestone:** M3
- **Closes:** REQ-EVT-035
- **Depends on:** TASK-M3-09
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines, the tests included

## Goal
Below the f32 spacing of the positions at the pair's coordinates, `r_coll` means "collision at f32 coincidence" and
`d_min` is resolved only to that spacing; the slider stays valid to 0, and the floor is recorded with the run (R-954,
integrator contract Part 7). The test itself is unchanged (TASK-M3-09). This task writes the definition REQ-EVT-035
asks for into Part 7, under physics review, first: what is recorded (a value, a flag or both), how it is computed from
the run, the floor varying with the pair's coordinates along it, and where it is stored. Then it builds it, so every
output says whether its `r_coll` lay below what the test resolves.

## References
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/design/principia_dd_integrator.md` § "3.6 Detectors (per `STEP`, on the projected state)"
- `decisions.md` § "R-954 — RQ-782, the collision test's floor: option (a), a definition in integrator contract Part 7; below the f32 spacing of the positions at the pair's coordinates `r_coll` means collision at f32 coincidence, `d_min` is resolved only to that spacing, the slider stays valid to 0, and the floor is recorded with the run *(closes RQ-782)*"
- `decisions.md` § "R-547 — Decode and encode numerics: the swap test is the conjugacy; step 0 is skipped within a calibrated `τ_I` and rescales in two steps; a branch input that must agree across backends is built only from operations every IEEE backend rounds alike, with no subnormal intermediate; the native build runs without MIR GVN and CI runs it at the release profile; T1 holds off the mirror band; REQ-SYS-015 is split *(closes RQ-283, RQ-295, RQ-296, RQ-298, RQ-299, RQ-360 and RQ-365; corrects R-34)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-973 — RQ-792, where the collision test's floor is recorded: option (a) in its cautious form, a per-sample exponent byte latched as the largest floor over every near-firing state and the `d_min` state, each output's flag read as `r_coll < floor`; its placement and bit-level definition are TASK-M3-71's, under physics review *(closes RQ-792)*"

## Deliverables
- Integrator contract Part 7: the definition of the floor's record, physics-reviewed, beside R-954's dated bullet
  (R-72; REQ-EVT-035).
- The record, computed as defined and carried on every output beside `r_coll`.
- `cargo test -p engine r_coll_floor_recorded`, with its controls (R-176).

## Acceptance tests
- Review (physics): Part 7 carries the definition of the floor's record (REQ-EVT-035).
- `cargo test -p engine r_coll_floor_recorded` — REQ-EVT-035's verify: a run at the default `r_coll` records that it
  lies above the floor, one below the f32 spacing of its bodies' coordinates records that it lies below, and the record
  is on every output beside `r_coll`; controls: a recorder reading `r_coll` alone, not the coordinates, misreports a
  run whose bodies lie at `|x|` in `[1, 2)` with `r_coll` between the two spacings, and fails; an output that drops the
  record fails.

## Notes
- New (11 Nov 2026), from R-954 (RQ-782 (a)): no task built the `r_coll` slider or a dispatch-side flag, so this one
  carries the clause "the floor is recorded with the run" (applied per R-369: task scoping).
- The test's figures in REQ-EVT-035's verify are examples; the definition fixes them. A value the definition needs and
  the corpus lacks is a calibration requirement (R-71), never a guess.
- *12 Nov 2026 (R-973, RQ-792 (a), cautious form):* the form is ruled, so this task no longer waits: a per-sample byte holding the floor's exponent, latched as the largest floor over every state at which the test was near firing (`d_quant(s) < r_coll + floor(s)`) and over the `d_min` state, each output's flag read as `r_coll < floor` (REQ-EVT-035). Its placement (8 B at f32 under R-397's recheck, or a share of TASK-M3-41's 8 B if it fits there) and its bit-level layout are this task's definition, in Part 7 and the payload, under physics review, with the export decoder and the schema hash; a choice there that no ruling makes goes to REVIEW_QUEUE.md.
