# TASK-M2-66 — The primitive registry, the refusals, the labels and the bitwise digest, on the CPU (R-916)

- **Milestone:** M2
- **Closes:** REQ-CHART-096, REQ-CHART-097, REQ-CHART-098, REQ-CHART-099, REQ-CHART-127, REQ-CHART-128, REQ-CHART-169
- **Depends on:** TASK-M2-58, TASK-M2-59, TASK-M2-14, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines, the tests included

## Goal
The authoring layer gains the registry on the CPU: every pair is validated against one refusal table, and every chart comes out of `resolve` with its lowering and its labels, held to the digest TASK-M2-61 writes before its re-expression. The f32 band's label (R-876) is its CPU arm. The baked entries' GPU side and the axis cap stay TASK-M4-33's.

## References
- `docs/design/principia_chart_composition.md` § "3. The primitive registry"
- `docs/design/principia_chart_composition.md` § "5.5 Every case where two axes can't be independent"
- `docs/design/principia_chart_composition.md` § "6.1 Refusals: genuine contradictions only"
- `docs/design/principia_chart_composition.md` § "6.3 The Domain preview shows why"
- `docs/design/principia_chart_composition.md` § "6.2 Labels: reduced capability, never a refusal"
- `docs/design/principia_chart_composition.md` § "7. Lowering: three ways a pair runs"
- `docs/design/principia_chart_composition.md` § "1. What exists, and is kept"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `docs/design/principia_chart_composition.md` § "3.3 Every primitive's class"
- `decisions.md` § "R-852 — Every registry primitive is classed as a state function, a conserved quantity (exact or approximate, labelled) or a trajectory functional, never an axis; final-condition charts stay a design item for the rulings report, with the check that conserved axes are `T`-independent *(on RQ-734, which stays open)*"
- `decisions.md` § "R-876 — An f32 chart may refuse a pixel inside the float-wide band at the warp's floor, the refusal labelled as rounding; the edge walk is repeated on the GPU when the shader decodes these charts *(closes RQ-740; refines R-708)*"
- `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"

## Deliverables
- `crates/engine/src/chart_authoring/registry.rs`: the rows, each with its class; `AxisSpec`, `ChartSpec`.
- The round-trip table every later row with an inverse must add to (§ 12).
- `crates/engine/src/chart_authoring/validate.rs`: the new `Refusal` variants; the pair table.
- `resolve`'s lowering and labels; `presets_bitwise` reading TASK-M2-61's `fixtures/chart_composition/digest.json`.
- The f32 band defined in chart_reference § 2.1, its quantity and width, under physics review (R-72; REQ-CHART-169).

## Acceptance tests
- `cargo test -p engine -- size_only_refused excluded_primitives` — I, R and ℓ refused by name; every registered row sampled on decodes, one constant there and not declared size-only failing; control: a row constant on decodes declared free fails; a row with an inverse naming a binary phase fails, a forward-only overlay row naming one passes; a combination of closed parts with no registered inverse, as exp(K) + K, reports none and refuses as an axis with NoJointInverse, and one with a registered closed or numerical inverse reports that class; control: the first reported closed fails (REQ-CHART-096).
- `cargo test -p engine pair_table_complete` — every registered pair's rank on a grid agrees with the table in both directions; controls: a table row removed fails, a rank-2 pair added to the table fails (REQ-CHART-097).
- `cargo test -p engine lowering_labels` — a latent pair lowers uniform, a baked pair to its entry, an unbaked closed pair to CPU batch with its label, a numerical pair to CPU batch with its label; control: an unbaked pair returning a refusal fails (REQ-CHART-098).
- `cargo test -p engine presets_bitwise` — each chart of kernel::chart::REGISTRY, each gallery preset and a saved chart of each kind against fixtures/chart_composition/digest.json, written on the commit before the registry; controls: one mantissa bit flipped in a digest fails, a default field serialised fails (REQ-CHART-099).
- `cargo test -p engine round_trip_complete` — every registered row with an inverse, closed or numerical, has a round-trip case, a read-back of its Holds column and a named mutation control that fires on that row; forward-only rows and combinations classed none need none; controls: a case removed fails, an inverse added with no case fails, a K case with its momentum direction wrong fails (REQ-CHART-127).
- `cargo test -p engine primitive_class` — every row has its class; a row declared conserved and exact holds along a short run within the run's drift, at ε = 0, the softened and Newtonian energies being R-791's and R-819's; a row declared approximate shows the label; a functional is refused as an axis; controls: a state row declared conserved fails, a functional accepted fails (REQ-CHART-128).
- `cargo test -p kernel f32_band_label` — a pixel on the floor that the f32 decode refuses is labelled rounding, and its f64 decode is a system; no f32-accepted pixel fails its f64 check; controls: a band refusal drawn as the edge fails, an accepted non-system fails; plus review: the band's quantity and width in chart_reference § 2.1, approved by the physics reviewer, and, if a number, its proposal with evidence confirmed at the M2 gate (REQ-CHART-169).

## Notes
- Split from TASK-M4-33 by R-916 (9 Nov 2026); REQ-CHART-169 is REQ-CHART-152's CPU arm, split from TASK-M4-06, which keeps the GPU edge walk.
- The digest is written once, on TASK-M2-61's baseline commit; this task is held to it, applied per R-369 (the drafter's recommendation).
- **Build order (R-916):** stage 3, after stage 2 (TASK-M2-65).
- *11 Nov 2026 (the re-plan's § A.1 and the addendum's § A.3, CHANGE; entered by the re-plan's addition, `plan/charts/m2-chart-replan-addition.md` § F.4):* the lowering label becomes the fragment route (R-935, under RQ-765 (a)); each map gains a slope tier (R-926 item 9; TASK-M2-88 builds the slopes) and a scale (R-935, under RQ-762 (a)); each row's edges declare the kinds allowed and a default (R-942, under RQ-775 (a); TASK-M2-101 to TASK-M2-105 build the kinds). The lines above stand as written until this task's file is rewritten under those rulings; where they differ, the rulings bind.
- *12 Nov 2026 (R-926 item 5, superseding R-839; R-935 under RQ-765; the chart audit's F7):* every chart is evaluated on the CPU, so "CPU batch" reads as the CPU evaluator and no view is labelled CPU-only; a route label is the fragment's, "closed form" or "table" (REQ-CHART-098's, per R-935). *Was:* "an unbaked closed pair to CPU batch with its label" (Acceptance, `lowering_labels`), which stands as written.
