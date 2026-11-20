# TASK-M6-09 — The per-quad measure weight |det J_D| and the quantitative-claims path

- **Milestone:** M6
- **Closes:** REQ-VAL-094, REQ-VAL-084, REQ-VAL-086
- **Depends on:** TASK-M6-07, TASK-M5-14
- **Needs (earlier milestones):** REQ-SCHED-031, REQ-CHART-032
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~250 lines

## Goal
Quantitative claims have one honest path: statistics come from the uniform-grid measurement dispatch (`FULL_RETENTION`) or from a known measure weighted per sample by the Gram area factor `√det(J_DᵀJ_D)` of the sample's own exact (tier 1) or implicit (tier 2) slope, and on a tier-3 map by a central difference at R-633's step per sample, labelled "measure by central difference" and admitted per map only by the Richardson check written before any run, a map that fails giving unweighted chart-area fractions on the uniform grid, labelled (R-996) — and are repeated under a link swap and a tilt; quadtree leaf counts and refinement density never feed a statistic.

## References
- `decisions.md` § "R-996 — RQ-800, the measure weight: option (a), per pixel from each pixel's own exact slope; on a chart with no exact slopes, an estimated slope per pixel, labelled, admitted per chart only by the two-step check written before any run, else that chart gives unweighted chart-area fractions *(closes RQ-800; extends R-935)*"
- `docs/design/principia_deep_zoom.md` § "Couplings to the contract"
- `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 2 — Refinement density is not probability density (enforced here, because this is where it would break)"
- `docs/design/principia_deep_zoom.md` § "3. Three-layer quadtree"

## Deliverables
- `crates/engine/src/deep/measure.rs`: `measure_weight(sample)`, the Gram area factor of the sample's own slope (TASK-M2-88's tier 1 or tier 2); on a tier-3 map the central difference at R-633's step per sample, and the per-map Richardson check with its positive control (R-996). *Was:* "`measure_weight(quad) = |det J_D|` read from TASK-M6-07's per-quad cache (no second Jacobian)."
- `crates/validation/src/measure.rs`: the statistics estimator API that accepts only a uniform grid or `|det J_D|`-weighted samples, with a link-swap and tilt repeat helper.
- An `xtask` lint that no statistic module imports the quadtree's leaf iterator.

## Acceptance tests
- `cargo test -p engine measure_weight_per_sample` — REQ-VAL-094's verify: on tier-1 and tier-2 maps each sample's weight is its own slope's Gram area factor, and against a per-quad weight on shallow quads the spread of their ratio is reported; on a tier-3 map the Richardson check `|W(h) − W(h/2)| ≤ B(h) + B(h/2)`, `B` stated per map before the run, admits or refuses the map, a refused map giving unweighted chart-area fractions, labelled; control: an error of `100·(B(h) + B(h/2))` planted in one map's evaluation fails the check (REQ-VAL-094). *Was:* "`cargo test -p engine measure_weight_det_jd` — measure weight equals |det J_D| for the quad; computed once (REQ-VAL-094)."
- Review checklist (physics) — every quantitative figure's pipeline uses a uniform grid with Jacobian correction and is repeated under a link swap and a tilt (REQ-VAL-084).
- Review checklist (physics, code) + the leaf-count lint in `cargo xtask plan-check` — every quantitative statistic in the code base is traced to the uniform-grid measurement path or a |det J_D|-weighted estimator; no statistic reads quadtree leaf counts (REQ-VAL-086).

## Notes
- *13 Nov 2026 (R-996, RQ-800 (a)):* the measure weight is per sample, one definition everywhere; the Goal, the measure deliverable and its acceptance line are rewritten from the answer, each with its *Was*. The title's "per-quad measure weight |det J_D|" reads as the per-sample weight. The Goal's *Was:* "or from a known measure weighted by `|det J_D|` — taken from the same `J_D` the linear decode uses, computed once —".
