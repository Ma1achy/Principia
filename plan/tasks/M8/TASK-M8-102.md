# TASK-M8-102 — CPU batch: numerical inverses, their branch rule and seam

- **Milestone:** M8
- **Closes:** REQ-CHART-107, REQ-CHART-129, REQ-CHART-130
- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M4-06, TASK-M6-03, TASK-M4-34, TASK-M8-100
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The third lowering: an unbaked or numerical pair fills the integrator's IC buffer from f64 on the CPU, labelled and not live.

## References
- `docs/design/principia_chart_composition.md` § "7.2 CPU batch"
- `docs/design/principia_chart_composition.md` § "7.3 A numerical inverse: branch and seam"
- `docs/design/principia_chart_composition.md` § "7.4 Seams: jitter, spread and refinement"
- `docs/design/principia_dd_refinement_policy.md` § "1. The split rule — no exponent in the split test"
- `docs/design/principia_core_design.md` § "3. Kernel eats chart params, decodes in-kernel"

## Deliverables
- `crates/engine/src/chart_batch.rs`.
- The solver, its rule and seam; the branch in the IC record.
- The `seam_straddled` flag in the resolve pass (R-142), for baked charts and from `branch` in the IC record for numerical ones; `seam-contaminated`; the mask on `Policy::Tolerance`'s `unresolved`; the decision `Seam` and `n_seam`.

## Acceptance tests
- `cargo test -p engine numerical_branch_seam` — the rule, branch and seam; two orders, one bit pattern; `forward(inverse(x)) = x` on every written sample; a fixture of known solvable samples solves, no failures; controls: a rule reading a neighbour, a pair with no rule, mutation: the target `F(z) + x`, a wrong root written (REQ-CHART-107).
- `cargo test -p engine seam_jitter_flagged` — copies at the sampling note's offsets in `(u, v)`; a footprint across a numerical seam, polar's branch line or region D's mask, a baked chart's, flagged, all `E + 1` copies kept, spread and `error_ratio` labelled `seam-contaminated`; controls: a copy dropped at the seam, the flag read from a neighbour (REQ-CHART-129).
- `cargo test -p engine seam_refinement_masked` — a flagged footprint leaves `n_unresolved`, a mask-blanked copy included; a quad of them takes `Seam`, `n_seam` counted, no priority from `coherence`; a non-finite copy still fires; an unflagged neighbour still splits; today's registry bitwise; controls: the mask removed, the mask on an unflagged footprint, `Keep` for `Seam` (REQ-CHART-130).

## Notes
- Shares R-669's Path B batch where that exists.
- Waits on its rulings item: RQ-719. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
