# TASK-M8-113 — Time-axis chart mode

- **Milestone:** M8
- **Closes:** REQ-CHART-123
- **Depends on:** TASK-M4-33
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The mode, column marching and row reads, on CPU batch first.

## References
- `docs/design/principia_chart_composition.md` § "18.1 Verdict: a chart mode, no fifth axis kind"
- `docs/design/principia_chart_composition.md` § "18.2 Against the core"
- `docs/design/principia_chart_composition.md` § "18.4 Interactions"
- `docs/design/principia_chart_composition.md` § "18.6 Tests"

## Deliverables
- `crates/engine/src/chart_authoring/`: `ChartMode::TimeAxis`.
- `crates/kernel/src/driver/`: the column march.
- `docs/contracts/principia_chart_decoder_contract.md`: the mode, on ruling.

## Acceptance tests
- `cargo test -p engine time_column_matches_playhead time_rows_on_boundaries time_zoom_floor time_extend_continues time_terminal_holds time_jitter_ic_only time_two_axes_refused time_window_frozen time_morph_rules time_anchor` — each and its control (REQ-CHART-123).

## Notes
- Waits on RQ-727, whose ruling amends the caching contract's T row and the temporal note.
