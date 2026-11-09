# TASK-M2-81 — Path re-timing, alignment and flags (the path type is TASK-M2-61's, the line on blocks TASK-M2-65's, export TASK-M8-108's; R-873, R-899, R-916)

- **Milestone:** M2
- **Closes:** REQ-CHART-114, REQ-CHART-116, REQ-CHART-124, REQ-CHART-133
- **Depends on:** TASK-M2-58, TASK-M2-60, TASK-M2-55, TASK-M2-57, TASK-M2-66, TASK-M2-67, TASK-M2-68, TASK-M2-69, TASK-M2-70, TASK-M2-71, TASK-M2-72, TASK-M2-73, TASK-M2-74, TASK-M2-75, TASK-M2-76, TASK-M2-77, TASK-M2-78, TASK-M2-79, TASK-M2-80
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~700 lines

## Goal
Every arm of the path type, with tilt and zoom calling their own code, a frame recorded by value: re-timing, alignment and the flags, on the CPU. Export through CPU batch is TASK-M8-108's (R-916).

## References
- `docs/design/principia_chart_composition.md` § "17.2 The mechanism"
- `docs/design/principia_chart_composition.md` § "17.3 The constraint: bitwise through the mechanism"
- `docs/design/principia_chart_composition.md` § "17.4 `s = 0` and `s = 1`"
- `docs/design/principia_chart_composition.md` § "17.5 A frame: valid, reproducible, recorded"
- `docs/design/principia_chart_composition.md` § "17.6 Per-axis and pair-map morphs"
- `docs/design/principia_chart_composition.md` § "17.7 Plane to plane"
- `docs/design/principia_chart_composition.md` § "17.9 Path shape and timing"
- `docs/design/principia_chart_composition.md` § "17.10 Without a lock: alignment"
- `docs/design/principia_chart_composition.md` § "17.11 Honesty"
- `docs/design/principia_chart_composition.md` § "17.14 Tests"

## Deliverables
- `crates/engine/src/`: the path type, its arms, the record and the sim key's morph arm.
- CPU batch's path lowering: blocks, the great circle, alignment, re-timing, control charts.
- The planner's row and flag.

## Acceptance tests
- Physics reviewer: the metric on the blended state in the chart reference (REQ-CHART-124).
- `cargo test -p engine -- path_gram_flags path_blank_ends path_hold_table path_small_step path_links_cut` — each row and its control, path_small_step's fixture of planes δ = 10⁻¹⁰ apart with `arccos` as its control (REQ-CHART-114).
- `cargo test -p engine path_lower_half_stays` — two views wholly in the open lower hemisphere, whose ends differ at the chart layer, so the arm function returns `Path::Line`, asserted first, keep `n₂ < 0` at every pixel of every frame, ends included; the other arms assert only signed, no mirror taken (R-544, the owner's rider to RQ-736); mutation: `canon::canonicalise` for the signed form; control: a literal `z` blend (REQ-CHART-133).
- `cargo test -p engine path_retime` — spread at most 1.25, pinned ends; control: the table off (REQ-CHART-116).

## Notes
- **Build order (R-863):** it straddles stages 1, 2 and 4: its path type and bitwise tests are stage 1's, now TASK-M2-61's too; its line on blocks is the morph stage 2 bases or one of stage 4's families; its re-timing, alignment and export are stage 4's. No stage edge is made; RQ-744 (B) holds the choice.
- If path_tilt_bitwise or path_zoom_bitwise can't pass through the path type, the generalisation isn't adopted: stop and report (the owner's rule).
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M2-58 in Depends on holds it.
- *8 Nov 2026 (R-873, R-899):* **Split (RQ-744 (B) (a)).** REQ-CHART-113's stage-1 tests move to TASK-M2-61; the line on blocks is built by TASK-M2-65 as the locked path's carrier (the brief's Q9 (a), R-899); this task keeps re-timing, alignment and export through CPU batch, stage 4, and depends on every stage-3 task. *Was (acceptance):* "`cargo test -p engine -- path_tilt_bitwise path_zoom_bitwise path_lock_bitwise path_save_view_bitwise path_keyframe_bitwise path_ends_bitwise path_record_round_trip` — each against the direct operation (REQ-CHART-113)." *Was:* "- **Closes:** REQ-CHART-113, REQ-CHART-114, REQ-CHART-116, REQ-CHART-124, REQ-CHART-133"; "- **Depends on:** TASK-M2-58, TASK-M2-60, TASK-M8-102, TASK-M8-31"
- *9 Nov 2026 (R-916):* moved from M8 to M2 and renumbered: formerly TASK-M8-106; the header follows plan/tasks.yaml. *Was:* "Milestone: M8"; "Depends on: TASK-M2-58, TASK-M2-60, TASK-M8-102, TASK-M8-31, TASK-M8-103, TASK-M8-104, TASK-M2-77, TASK-M8-118, TASK-M2-78, TASK-M2-79, TASK-M8-126, TASK-M7-72"; "Reviewers: code, qa, physics, perf".
- *9 Nov 2026 (R-916):* the export arm, the perf reviewer's, goes to TASK-M8-108 (export through CPU batch, a visual adapter); this task keeps the non-visual path maths, reviewed by code, qa and physics. *Was (title):* "The path type, the line on blocks, and export through CPU batch"; *was (Goal):* "Every arm of the path type, with tilt and zoom calling their own code, a frame recorded by value, and export working from day one for any ends."; *was (Reviewers):* "code, qa, physics, perf".
