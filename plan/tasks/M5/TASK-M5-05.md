# TASK-M5-05 — Chart-domain quad skip and the label of skipped pixels

- **Milestone:** M5
- **Closes:** REQ-CHART-039, REQ-CHART-041, REQ-SCHED-083
- **Depends on:** TASK-M5-03
- **Needs (earlier milestones):** REQ-CHART-028, REQ-CHART-043, REQ-CHART-006, REQ-CHART-010, REQ-ENC-013
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~260 lines

## Goal
The scheduler asks each chart's `validate(u, v)` (the same function lookup/lock and the legend call) whether a
quad lies in the chart's feasible region, and skips quads by a rule written into scheduler_contract (wholly outside,
wholly inside, partly inside — REQ-SCHED-083). What a skipped quad's pixels carry is defined in inverse_encode_contract's
chart-aware validation so that no pixel is unlabelled (REQ-CHART-041). The GPU never sees an invalid IC.

## References
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch"
- `docs/design/principia_chart_reference.md` § "0.7 Degeneracy — every pixel gets a label"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `decisions.md` § "R-26 — Every chart declares its domain function *(CD-6)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `docs/contracts/principia_scheduler_contract.md` Part 6: the quad-skip rule (REQ-SCHED-083).
- `docs/contracts/principia_inverse_encode_contract.md` "Chart-aware validation": the label a skipped quad's pixels carry
  (REQ-CHART-041). Both with the "Removed lines" note.
- `crates/engine/src/quadtree/domain.rs`: quad classification against `validate`.
- Tests `crates/engine/tests/domain_skip.rs` on the (L_z, E) chart.

## Acceptance tests
- `cargo test -p engine domain_skip` — quads above the (L_z, E) parabola are skipped by the scheduler; lookup and legend call validate (REQ-CHART-039).
- Review checklist (physics) — the doc states the label a skipped quad's pixels carry, so that no pixel is unlabelled (chart_reference §0.7); physics reviewer approved; the doc change is in this PR and the physics reviewer approves it before merge (REQ-CHART-041).
- Review checklist (physics) — the doc states the rule for quads wholly outside, wholly inside and partly inside the chart's domain; the doc change is in this PR and the physics reviewer approves it before merge (REQ-SCHED-083).

## Notes
- Definitions (R-72) this task writes: REQ-CHART-041, REQ-SCHED-083.
- Both definitions are R-72 doc changes; the physics reviewer approves them before merge.
