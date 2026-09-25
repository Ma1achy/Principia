# TASK-M2-14 — Axis metadata, per-chart descriptors and the well-posedness validator

- **Milestone:** M2
- **Closes:** REQ-CHART-004, REQ-CHART-006, REQ-CHART-011, REQ-CHART-012, REQ-CHART-014, REQ-CHART-035, REQ-SYS-013
- **Depends on:** TASK-M2-08, TASK-M2-11, TASK-M2-12, TASK-M2-13
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~460 lines

## Goal
The CPU authoring layer exists: every axis is one of the closed set of four kinds (raw latent, derived-in-block, cross-block invariant, coupled curve) with its metadata — block-touch set, annotation-only, invariant (sector, dependency set, `conserved_along_flow`: true for E and L_z, false for K), residual convention, curve (embed and tangent) — lowered by monomorphisation, never interpreted. Every chart declares `system_image` (bijective, n-to-1 with its fold, DoubleCover, ray-degenerate), `has_feasibility_boundary`, `coupling` and annotation. The well-posedness validator confirms all 8 DOF are determined exactly once (annotation axes count none), refuses under- and double-determined blocks and invariant axes not downstream of their dependencies, and runs before resolution so resolve only ever sees a validated chart. A physical-quantity axis makes a chart nonlinear.

## References
- `docs/contracts/principia_chart_decoder_contract.md` § "The four axis kinds — a closed set"
- `docs/contracts/principia_chart_decoder_contract.md` § "Design axioms (the six that must survive contact with a code agent)"
- `docs/contracts/principia_canonical_spec.md` § "4. Charts & navigation *(authoritative: `chart_decoder_contract`)*"
- `docs/design/principia_core_design.md` § "2. Authoring layer lowers to specialised kernels — by monomorphisation"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `decisions.md` § "R-27 — Both Burrau charts are kept, each labelled with its quotient *(CD-7)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-104 — The new `system_image` value is `DoubleCover` *(closes RQ-64)*"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "03 Chart builder"
- `docs/contracts/principia_lowering_contract.md` § "Part 5 — The resolution function (the "switch", concretely)"
- `decisions.md` § "R-26 — Every chart declares its domain function *(CD-6)*"
- `decisions.md` § "R-141 — The shape sphere is 2-to-1 over its φ hemispheres *(closes RQ-71, corrects R-104)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-117 — The lowering appendix's shape-sphere row uses (θ, φ) *(closes RQ-85)*"
## Deliverables
- `crates/engine/src/chart_authoring/` (axis kinds, axis metadata, per-chart descriptors, the validator).
- A `ValidatedChart` type only the validator constructs, taken by the resolve entry.
- Descriptors for every chart of TASK-M2-05 to TASK-M2-13; tests `crates/engine/tests/well_posedness.rs`.

## Acceptance tests
- Review (physics): axes are the closed four-kind enum, authored CPU-side; no runtime axis-kind dispatch in the kernel; each chart is a type parameter of the kernel (REQ-CHART-004).
- `cargo test -p engine validator_invariant_downstream` — the validator refuses an invariant axis whose dependency set is not upstream; (L_z, E) solves jointly (REQ-CHART-006).
- `cargo test -p engine well_posedness` — fixtures: 2 swept + 6 frozen accepted; 1 swept + annotation + 7 frozen accepted; a double-written block and an under-determined block each refused (REQ-CHART-011).
- Review (physics): the axis descriptor struct has every field; E and L_z set `conserved_along_flow`, K does not (REQ-CHART-012).
- `cargo test -p engine chart_descriptors` — each registered chart returns the descriptors; the shape sphere is n-to-1 with n = 2 (R-141); the full-range Burrau (Euclid) chart's value follows RQ-127's ruling (REQ-CHART-014; the int (m, n) lattice's "bijective" is REQ-CHART-037's, M4).
- `cargo test -p engine physical_axis_nonlinear` — adding a physical-quantity axis sets the chart kind to nonlinear and the decode path calls Φ (REQ-CHART-035).
- `cargo test -p engine validate_before_resolve` — resolve on an invalid chart is unreachable: validation rejects it first (resolve takes only a `ValidatedChart`) (REQ-SYS-013).

## Notes
- RQ-71 ruled: R-141 — the shape sphere is n-to-1 (n = 2) and `DoubleCover` is retired. REQ-CHART-014 waits on **RQ-127** for the full-range Burrau chart, which R-27 made the two-labelled-systems case.
- RQ-85 ruled: R-117 — the shape sphere's descriptors are those of chart_reference §3's (θ, φ) chart; the lowering appendix row is conformed to it.
- RQ-95 ruled: R-113 — REQ-CHART-014's lattice clause ("the int (m, n) lattice is bijective") moves into REQ-CHART-037's verify (TASK-M4-06).
