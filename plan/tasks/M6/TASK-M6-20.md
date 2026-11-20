# TASK-M6-20 — Refinement reporting: per-chart reports, weighted stop breakdowns, the binding axis, dp/u

- **Milestone:** M6
- **Closes:** REQ-TOOL-055, REQ-TOOL-056, REQ-TOOL-057, REQ-VAL-091, REQ-REF-039
- **Depends on:** TASK-M6-01, TASK-M6-04, TASK-M6-15, TASK-M5-28
- **Needs (earlier milestones):** REQ-TOOL-050, REQ-TOOL-051, REQ-TOOL-054, REQ-PERF-021, REQ-TOOL-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-7, PIT-6
- **Size:** ~380 lines

## Goal
The refinement reports the corpus requires: per chart the leaf count, `alpha` distribution and tree shape (leaf counts compared within a chart only); stop-reason breakdowns weighted by the subtree each stop forecloses, never a bare leaf count; the binding axis (converged / budget-bound / cap-bound) per frame, with a cap-bound image saying so; `dp/u` beside `tol/u` in every refinement benchmark; and the anisotropic-split cost count.

## References
- `docs/design/principia_chart_reference.md` § "5.3 What to report per chart"
- `docs/design/principia_dd_refinement_policy.md` § "6. Decision variants, and the second budget line"
- `docs/read_first/principia_01_pitfalls.md` § "7. A STOP-REASON BREAKDOWN NEEDS WEIGHTING BY WHAT IT FORECLOSES"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "ALWAYS REPORT WHICH AXIS BOUND"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "v0.5 means the NUMBERS are guesses; the SHAPE is not"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "4. Deriving the tier instead of choosing it"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "Collect everything relevant, in one file"
- `docs/design/principia_memory_tiers.md` § "Principia — quality tiers & memory model (auto-mode reference)"
- `docs/design/principia_memory_tiers.md` § "4.1 The same six tiers on the three axes"
- `docs/design/principia_dd_refinement_policy.md` § "5. WHERE THE SAVING GENERALISES — and where it does not"
- `docs/read_first/principia_01_pitfalls.md` § "6. READ `dp/u` BEFORE `tol/u`"
- `docs/read_first/principia_00_philosophy.md` § "8.1 An 8D BVH over the manifold, instead of a per-slice quadtree"

## Deliverables
- `crates/engine/src/telemetry/refine_report.rs`: the per-chart report, foreclosed-weight stop breakdown, binding axis in the frame record.
- `xtask` bench report template carrying `dp/u` next to `tol/u`.
- `cargo xtask gate anisotropic-splits` on the standard charts.

## Acceptance tests
- Review checklist (physics) — per-chart report contains the three quantities; cross-chart comparison views use the alpha distribution, not leaf counts (REQ-TOOL-055).
- `cargo test -p engine stop_breakdown_weighted` — near-field fixture: the 4 level-2 floor leaves report ≈ the whole frame's foreclosed weight (REQ-TOOL-056).
- `cargo test -p engine binding_axis_reported` — force each binding case; the frame record and UI report it (REQ-TOOL-057).
- Review checklist (physics) — every refinement benchmark report carries dp/u next to tol/u (REQ-VAL-091).
- `cargo xtask gate anisotropic-splits` — At the refine milestone, report the count/fraction of isotropic splits whose four children all immediately keep, on the standard charts (REQ-REF-039).

## Notes
- Pitfall regressions: PIT-7 (weight by foreclosure), PIT-6 (read dp/u first). The "exact optimum" for `dp/u` needs the dynamic-programming reference from prin-rs; its port is part of this task's bench harness.
