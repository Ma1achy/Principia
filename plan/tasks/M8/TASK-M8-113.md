# TASK-M8-113 — Time-axis chart mode on the real integrator: the column march (R-916)

- **Milestone:** M8
- **Closes:** REQ-CHART-178
- **Depends on:** TASK-M2-83, TASK-M3-40, TASK-M3-67, TASK-M8-108
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
Time-axis chart mode on the real integrator: the column march, its column equal to a normal render at the playhead and its extension continuing bitwise, after TASK-M2-83's maths and TASK-M3-67's provider.

## References
- `docs/design/principia_chart_composition.md` § "18.1 Verdict: a chart mode, no fifth axis kind"
- `docs/design/principia_chart_composition.md` § "18.2 Against the core"
- `docs/design/principia_chart_composition.md` § "18.3 Stored rows and dense output: the conflict, plainly"
- `docs/design/principia_chart_composition.md` § "18.4 Interactions"
- `docs/design/principia_chart_composition.md` § "18.6 Tests"

## Deliverables
- `crates/engine/src/chart_authoring/`: `ChartMode::TimeAxis`.
- `crates/kernel/src/driver/`: the column march.
- `docs/contracts/principia_chart_decoder_contract.md`: the mode, on ruling.

## Acceptance tests
- `cargo test -p engine -- time_column_matches_playhead time_extend_continues` — each with its control, the first with unregularised and regularised arms and a column whose later crossing cancels symbols, control: a row reading a prefix of the final word (REQ-CHART-178).

## Notes
- **Build order (R-863):** stage 5, time-axis charts and the symmetry filter. It depends on stage 4's task, TASK-M8-108, and merges only with its tests green.
- RQ-727's ruling amends the caching contract's T row and the temporal note.
- Waits on its rulings item: RQ-727. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); R-917's hold holds it, no longer an edge: R-916 took TASK-M2-58 (was TASK-M4-37), the rulings applied, out of Depends on, and no M3 to M9 task starts until M2's gate passes, which waits on TASK-M2-58 as an M2 task. *Was:* "TASK-M4-37 in Depends on holds it."
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Closes: REQ-CHART-123"; "Depends on: TASK-M4-37, TASK-M4-33, TASK-M8-108".
- *9 Nov 2026 (R-916):* split: the mode's maths through the trajectory interface (REQ-CHART-123, REQ-CHART-174) is TASK-M2-83's; this task keeps the two tests that read the real march (F6). *Was (title):* "Time-axis chart mode"; *was (Acceptance):* the ten-test line, eight of them now at TASK-M2-83.
