# TASK-M6-09 — The per-quad measure weight |det J_D| and the quantitative-claims path

- **Milestone:** M6
- **Closes:** REQ-VAL-094, REQ-VAL-084, REQ-VAL-086
- **Depends on:** TASK-M6-07, TASK-M5-14
- **Needs (earlier milestones):** REQ-SCHED-031, REQ-CHART-032
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~250 lines

## Goal
Quantitative claims have one honest path: statistics come from the uniform-grid measurement dispatch (`FULL_RETENTION`) or from a known measure weighted by `|det J_D|` — taken from the same `J_D` the linear decode uses, computed once — and are repeated under a link swap and a tilt; quadtree leaf counts and refinement density never feed a statistic.

## References
- `docs/design/principia_deep_zoom.md` § "Couplings to the contract"
- `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 2 — Refinement density is not probability density (enforced here, because this is where it would break)"
- `docs/design/principia_deep_zoom.md` § "3. Three-layer quadtree"

## Deliverables
- `crates/engine/src/deep/measure.rs`: `measure_weight(quad) = |det J_D|` read from TASK-M6-07's per-quad cache (no second Jacobian).
- `crates/validation/src/measure.rs`: the statistics estimator API that accepts only a uniform grid or `|det J_D|`-weighted samples, with a link-swap and tilt repeat helper.
- An `xtask` lint that no statistic module imports the quadtree's leaf iterator.

## Acceptance tests
- `cargo test -p engine measure_weight_det_jd` — measure weight equals |det J_D| for the quad; computed once (REQ-VAL-094).
- Review checklist (physics) — every quantitative figure's pipeline uses a uniform grid with Jacobian correction and is repeated under a link swap and a tilt (REQ-VAL-084).
- Review checklist (physics, code) + the leaf-count lint in `cargo xtask plan-check` — every quantitative statistic in the code base is traced to the uniform-grid measurement path or a |det J_D|-weighted estimator; no statistic reads quadtree leaf counts (REQ-VAL-086).

## Notes
- None.
