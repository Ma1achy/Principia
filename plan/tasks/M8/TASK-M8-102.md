# TASK-M8-102 — CPU batch: numerical inverses, their branch rule and seam

- **Milestone:** M8
- **Closes:** REQ-CHART-107
- **Depends on:** TASK-M4-33, TASK-M4-06
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The third lowering: an unbaked or numerical pair fills the integrator's IC buffer from f64 on the CPU, labelled and not live.

## References
- `docs/design/principia_chart_composition.md` § "7.2 CPU batch"
- `docs/design/principia_chart_composition.md` § "7.3 A numerical inverse: branch and seam"
- `docs/design/principia_core_design.md` § "3. Kernel eats chart params, decodes in-kernel"

## Deliverables
- `crates/engine/src/chart_batch.rs`.
- The solver, its rule and seam; the branch in the IC record.

## Acceptance tests
- `cargo test -p engine numerical_branch_seam` — the rule, branch and seam; two orders, one bit pattern; round trip on every sample; controls: a rule reading a neighbour, a pair with no rule (REQ-CHART-107).

## Notes
- Waits on RQ-719. Shares R-669's Path B batch where that exists.
