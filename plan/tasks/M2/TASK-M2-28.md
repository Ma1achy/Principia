# TASK-M2-28 — Shape-sphere display: the DoubleCover hemisphere and the equal-area projection

- **Milestone:** M2
- **Closes:** REQ-CHART-002, REQ-RENDER-026, REQ-CHART-047
- **Depends on:** TASK-M2-08, TASK-M2-14
- **Needs (earlier milestones):** REQ-GUI-001, REQ-SYS-009
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The shape-sphere chart shows its redundancy honestly: `system_image = DoubleCover` (2-to-1 over the φ hemispheres) carries its fold, and the chart renders one hemisphere and says so, or both with the redundancy flagged. It defaults to the equirectangular projection and offers an equal-area alternative (Mollweide or Hammer–Aitoff) for quantitative area comparisons, where the Lagrange poles aren't compressed.

## References
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 1 — The physics: what the 8D actually is"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `decisions.md` § "R-104 — The new `system_image` value is `DoubleCover` *(closes RQ-64)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- The hemisphere label / redundancy flag and the projection option on the shape-sphere chart (`crates/kernel/src/chart/shape_sphere.rs`, `crates/gui/` canvas label and selector).
- Screenshot tests under `cargo xtask screenshot` (reference per Gap G5).

## Acceptance tests
- `cargo xtask screenshot shape-sphere` — the shape-sphere chart shows either one labelled hemisphere or both with a redundancy flag; `cargo test -p engine chart_descriptors` reports DoubleCover (REQ-CHART-002).
- `cargo xtask screenshot shape-sphere-projection` — both projections are selectable on the shape-sphere chart (REQ-RENDER-026).
- Definition: the equal-area projection and its keying written into chart_reference §3.3 and approved by the physics reviewer (REQ-CHART-047).

## Notes
- REQ-CHART-002 waits on RQ-71 (DoubleCover vs 2-to-1).
- Gap G5: no artboard in `docs/gui/design/` shows the shape-sphere chart's hemisphere label or a projection selector, and render_gui_spec doesn't place the control; the dev GUI itself lands at M8. The screenshot's reference is unresolved.
- Gap G6: which equal-area projection (Mollweide or Hammer–Aitoff), and whether the projection is a different chart map (sim key, re-integrates) or a display remap.
- Waits on RQ-95 (`REVIEW_QUEUE.md`): M2 requirements that need M3, M4, M5 or an artboard.
- Closes, for gaps the corpus leaves open: REQ-CHART-047 (R-72 definition) (REVIEW_QUEUE RQ-110 lists them for the human).
